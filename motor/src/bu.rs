//! Leitor BER do Boletim de Urna **real** do TSE (especificação `bu.asn1`, pacote
//! "formato-arquivos-de-bu-rdv-e-assinatura-digital", eleições 2026).
//!
//! O arquivo `…-bu.dat` é um `EntidadeEnvelopeGenerico` cujo campo `conteudo`
//! (OCTET STRING) contém um `EntidadeBoletimUrna`. O módulo usa `IMPLICIT TAGS`,
//! então os campos sem tag de contexto mantêm a tag universal; os `SEQUENCE`
//! irmãos (0x30) são distinguidos pela posição.
//!
//! Diferente do leitor do benchmark sintético, este lê bytes vindos da rede:
//! **todo acesso é verificado** e um BU malformado gera `Err`, nunca pânico.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErroBU {
    Truncado(usize),
    TagInesperada { pos: usize, esperada: u8, achada: u8 },
    SemConteudo,
    SemIdentificacao,
}
impl fmt::Display for ErroBU {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{self:?}") }
}
impl std::error::Error for ErroBU {}

type R<T> = Result<T, ErroBU>;

/// Lê um TLV em `b[p..lim]`: devolve (tag, início do valor, fim do valor).
/// Suporta tag de um byte (todas as do bu.asn1) e comprimento curto ou longo (até 4 bytes).
#[inline(always)]
pub fn tlv(b: &[u8], p: usize, lim: usize) -> R<(u8, usize, usize)> {
    if p + 2 > lim { return Err(ErroBU::Truncado(p)); }
    let tag = b[p];
    let l0 = b[p + 1];
    let (vs, len) = if l0 < 0x80 {
        (p + 2, l0 as usize)
    } else {
        let n = (l0 & 0x7f) as usize;
        if n == 0 || n > 4 || p + 2 + n > lim { return Err(ErroBU::Truncado(p)); }
        let mut l = 0usize;
        for k in 0..n { l = (l << 8) | b[p + 2 + k] as usize; }
        (p + 2 + n, l)
    };
    let ve = vs.checked_add(len).ok_or(ErroBU::Truncado(p))?;
    if ve > lim { return Err(ErroBU::Truncado(p)); }
    Ok((tag, vs, ve))
}

/// Inteiro sem sinal BER (os campos do BU são todos não negativos e cabem em u32).
#[inline(always)]
pub fn uint(b: &[u8], s: usize, e: usize) -> u32 {
    let mut v = 0u32;
    for &x in &b[s..e] { v = (v << 8) | x as u32; }
    v
}

/// Recebe os dados de um BU à medida que são lidos, sem alocação.
pub trait Sink {
    fn secao(&mut self, _pleito: u32, _municipio: u32, _zona: u16, _secao: u16) {}
    fn comparecimento(&mut self, _qtd: u32) {}
    fn eleicao(&mut self, _id: u32, _aptos: u32) {}
    /// `cargo`: código do cargo constitucional (1..13) ou cargo/consulta livre (25..99).
    fn cargo(&mut self, cargo: u8, comparecimento: u32);
    /// `tipo`: 1 nominal, 2 branco, 3 nulo, 4 legenda, 5 cargo sem candidato.
    fn voto(&mut self, tipo: u8, qtd: u32, partido: u16, codigo: u32);
    fn fim_cargo(&mut self);
}

/// Localiza o `EntidadeBoletimUrna` dentro do envelope. Devolve (início, fim) em `b`.
pub fn conteudo_envelope(b: &[u8]) -> R<(usize, usize)> {
    let (t, s, e) = tlv(b, 0, b.len())?;
    if t != 0x30 { return Err(ErroBU::TagInesperada { pos: 0, esperada: 0x30, achada: t }); }
    let mut p = s;
    while p < e {
        let (t, vs, ve) = tlv(b, p, e)?;
        // cabecalho(0x30) fase(0x0A) [urna(0x30)] identificacao(0xA0|0xA1) tipoEnvelope(0x0A)
        // [seguranca(0x30): conteúdo cifrado — não ocorre nos BUs publicados] conteudo(0x04)
        if t == 0x04 { return Ok((vs, ve)); }
        p = ve;
    }
    Err(ErroBU::SemConteudo)
}

/// Percorre um BU real (`…-bu.dat`) e entrega seção, cargos e votos ao `sink`.
pub fn ler_bu<S: Sink>(arquivo: &[u8], sink: &mut S) -> R<()> {
    let (cs, ce) = conteudo_envelope(arquivo)?;
    let b = &arquivo[..ce];
    let (t, s, e) = tlv(b, cs, ce)?;
    if t != 0x30 { return Err(ErroBU::TagInesperada { pos: cs, esperada: 0x30, achada: t }); }
    let mut p = s;
    let mut n_seq = 0; // posição entre os filhos SEQUENCE (0x30)
    let mut pleito = 0u32;
    let mut achou_id = false;
    while p < e {
        let (t, vs, ve) = tlv(b, p, e)?;
        match t {
            0x30 => {
                match n_seq {
                    0 => pleito = ler_cabecalho(b, vs, ve)?,
                    2 => { ler_identificacao(b, vs, ve, pleito, sink)?; achou_id = true; }
                    3 => {
                        if !achou_id { return Err(ErroBU::SemIdentificacao); }
                        ler_resultados(b, vs, ve, sink)?;
                    }
                    _ => {} // 1 = urna, 4 = historicoCodigosCarga, 5 = historicoVotoImpresso
                }
                n_seq += 1;
            }
            0x02 => sink.comparecimento(uint(b, vs, ve)), // qtdEleitoresCompareceram
            _ => {}
        }
        p = ve;
    }
    if !achou_id { return Err(ErroBU::SemIdentificacao); }
    Ok(())
}

/// CabecalhoEntidade ::= SEQUENCE { dataGeracao GeneralString, idEleitoral CHOICE {[1]|[2]|[3] INTEGER} }
fn ler_cabecalho(b: &[u8], s: usize, e: usize) -> R<u32> {
    let mut p = s;
    while p < e {
        let (t, vs, ve) = tlv(b, p, e)?;
        if (0x81..=0x83).contains(&t) { return Ok(uint(b, vs, ve)); }
        p = ve;
    }
    Ok(0)
}

/// IdentificacaoSecaoEleitoral ::= SEQUENCE { municipioZona SEQUENCE {municipio, zona}, local, secao }
fn ler_identificacao<S: Sink>(b: &[u8], s: usize, e: usize, pleito: u32, sink: &mut S) -> R<()> {
    let (t, ms, me) = tlv(b, s, e)?;
    if t != 0x30 { return Err(ErroBU::TagInesperada { pos: s, esperada: 0x30, achada: t }); }
    let (_, a, z) = tlv(b, ms, me)?;
    let municipio = uint(b, a, z);
    let (_, a, z) = tlv(b, z, me)?;
    let zona = uint(b, a, z) as u16;
    let (_, _, z) = tlv(b, me, e)?; // local
    let (_, a, z2) = tlv(b, z, e)?;
    let secao = uint(b, a, z2) as u16;
    sink.secao(pleito, municipio, zona, secao);
    Ok(())
}

/// SEQUENCE OF ResultadoVotacaoPorEleicao
fn ler_resultados<S: Sink>(b: &[u8], s: usize, e: usize, sink: &mut S) -> R<()> {
    let mut q = s;
    while q < e {
        let (_, a, z) = tlv(b, q, e)?; // ResultadoVotacaoPorEleicao
        // idEleicao, qtdEleitoresAptos, qtdEleitoresAptosSecao, qtdEleitoresAptosTTE (INTEGER),
        // resultadosVotacao (SEQUENCE OF), ultimoHash (OCTET STRING), assinatura (OCTET STRING)
        let mut r = a;
        let mut ints = [0u32; 4];
        let mut ni = 0;
        while r < z {
            let (t, a3, z3) = tlv(b, r, z)?;
            match t {
                0x02 if ni < 4 => {
                    ints[ni] = uint(b, a3, z3);
                    ni += 1;
                    if ni == 2 { sink.eleicao(ints[0], ints[1]); }
                }
                0x30 => ler_resultados_votacao(b, a3, z3, sink)?,
                _ => {}
            }
            r = z3;
        }
        q = z;
    }
    Ok(())
}

/// SEQUENCE OF ResultadoVotacao { tipoCargo ENUM, qtdComparecimento INTEGER, totaisVotosCargo SEQUENCE OF }
fn ler_resultados_votacao<S: Sink>(b: &[u8], s: usize, e: usize, sink: &mut S) -> R<()> {
    let mut rv = s;
    while rv < e {
        let (_, a4, z4) = tlv(b, rv, e)?;
        let mut comp = 0;
        let mut x = a4;
        while x < z4 {
            let (t5, a5, z5) = tlv(b, x, z4)?;
            match t5 {
                0x02 => comp = uint(b, a5, z5),
                0x30 => ler_totais_cargo(b, a5, z5, comp, sink)?,
                _ => {}
            }
            x = z5;
        }
        rv = z4;
    }
    Ok(())
}

/// SEQUENCE OF TotalVotosCargo { codigoCargo CHOICE {[1] ENUM | [2] INTEGER}, ordemImpressao, votosVotaveis SEQUENCE OF }
fn ler_totais_cargo<S: Sink>(b: &[u8], s: usize, e: usize, comp: u32, sink: &mut S) -> R<()> {
    let mut tc = s;
    while tc < e {
        let (_, a6, z6) = tlv(b, tc, e)?;
        let mut y = a6;
        while y < z6 {
            let (t7, a7, z7) = tlv(b, y, z6)?;
            match t7 {
                0x81 | 0x82 => sink.cargo(uint(b, a7, z7) as u8, comp),
                0x30 => {
                    // SEQUENCE OF TotalVotosVotavel { [1] tipoVoto, [2] quantidadeVotos, [3] IdentificacaoVotavel OPTIONAL, ordemGeracaoHash, hash }
                    let mut vv = a7;
                    while vv < z7 {
                        let (_, a8, z8) = tlv(b, vv, z7)?;
                        let (mut tp, mut qt, mut pa, mut co) = (0u8, 0u32, 0u16, 0u32);
                        let mut w = a8;
                        while w < z8 {
                            let (t9, a9, z9) = tlv(b, w, z8)?;
                            match t9 {
                                0x81 => tp = uint(b, a9, z9) as u8,
                                0x82 => qt = uint(b, a9, z9),
                                0xA3 => {
                                    let (_, pa0, pa1) = tlv(b, a9, z9)?;
                                    pa = uint(b, pa0, pa1) as u16;
                                    let (_, co0, co1) = tlv(b, pa1, z9)?;
                                    co = uint(b, co0, co1);
                                }
                                _ => {}
                            }
                            w = z9;
                        }
                        sink.voto(tp, qt, pa, co);
                        vv = z8;
                    }
                    sink.fim_cargo();
                }
                _ => {}
            }
            y = z7;
        }
        tc = z6;
    }
    Ok(())
}

/// Sink de depuração: imprime o BU em texto (subcomando `inspecionar`).
pub struct Texto(pub String);
impl Sink for Texto {
    fn secao(&mut self, pl: u32, mu: u32, zn: u16, se: u16) { self.0 += &format!("pleito {pl} municipio {mu:05} zona {zn:04} secao {se:04}\n"); }
    fn comparecimento(&mut self, q: u32) { self.0 += &format!("comparecimento {q}\n"); }
    fn eleicao(&mut self, id: u32, aptos: u32) { self.0 += &format!("eleicao {id} aptos {aptos}\n"); }
    fn cargo(&mut self, c: u8, comp: u32) { self.0 += &format!("  cargo {c} comparecimento {comp}:"); }
    fn voto(&mut self, t: u8, q: u32, p: u16, c: u32) { self.0 += &format!(" [{t} {c}/{p}={q}]"); }
    fn fim_cargo(&mut self) { self.0 += "\n"; }
}

#[cfg(test)]
mod testes;
