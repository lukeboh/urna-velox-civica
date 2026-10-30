//! Candidatos: o que a situação dos candidatos precisa e **não** está nos BUs — destinação dos votos de cada
//! candidato, data de nascimento (desempate), eleitorado da circunscrição (regra dos 200 mil e eleitorado ainda
//! não apurado), turno, número de vagas e se a totalização foi finalizada.
//!
//! O motor não sabe de onde esses dados vêm. Nesta etapa o carregador é [`Candidatos::dos_oficiais`] (arquivos
//! oficiais de divulgação já baixados); na etapa 2 virão dos dados publicados antes da apuração.
//!
//! **Circunscrição** de cada cargo: Brasil (Presidente), UF (Governador, Senador e demais cargos gerais) ou
//! município (Prefeito, Vereador, consultas — [`cargo_municipal`]).

use crate::catalogo::cargo_municipal;
use crate::oficial::{Abrangencia, Oficial};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Estruturas para serialização/deserialização do formato `candidatos.json` (docs/arquitetura/FORMATO_CANDIDATOS.md).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CandidatoJson {
    pub numero: u32,
    #[serde(default)]
    pub nome: String,
    #[serde(default)]
    pub nome_urna: String,
    #[serde(default)]
    pub destinacao: String,
    pub nascimento: Option<String>,
    /// sigla do partido (`par[].sg` do `-u.json`; desde a `0.4.1`, para o site; o motor ignora)
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub partido: String,
    /// vice e suplentes (`cand[].vs[]`; desde a `0.4.1`, para o site; o motor ignora)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub vices: Vec<ViceJson>,
}

/// Vice ou suplente de um candidato (`vs[]` do `-u.json`): `tp` = "v" (vice), "1"/"2" (suplentes), como no arquivo.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ViceJson {
    #[serde(default)]
    pub tp: String,
    #[serde(default)]
    pub nome: String,
    #[serde(default)]
    pub nome_urna: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub partido: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CircunscricaoJson {
    pub abr: String,
    #[serde(default)]
    pub eleitorado: u64,
    #[serde(default)]
    pub candidatos: Vec<CandidatoJson>,
}

/// Uma abrangência publicada do cargo (a circunscrição ou uma menor, ex.: Presidente por UF).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AbrangenciaJson {
    pub abr: String,
    #[serde(default)]
    pub eleitorado: u64,
    /// seções a totalizar (`s.ts` do `-u.json` pré-apuração: BUs + não instaladas; sem as agregadas); 0 = desconhecido.
    /// O `uv-servico` encerra a apuração quando todas as abrangências chegam a esse número
    #[serde(default, skip_serializing_if = "eh_zero")]
    pub secoes: u64,
}

fn eh_zero(x: &u64) -> bool { *x == 0 }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CargoJson {
    pub cargo: u8,
    #[serde(default = "default_um")]
    pub vagas: u8,
    #[serde(default)]
    pub consulta: bool,
    /// só as circunscrições (onde se decide quem é eleito): Brasil, UF ou município
    #[serde(default)]
    pub circunscricoes: Vec<CircunscricaoJson>,
    /// todas as abrangências publicadas do cargo, com o eleitorado (vazio = só as circunscrições)
    #[serde(default)]
    pub abrangencias: Vec<AbrangenciaJson>,
}

fn default_um() -> u8 { 1 }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EleicaoJson {
    pub codigo: u32,
    #[serde(default = "default_um")]
    pub turno: u8,
    #[serde(default)]
    pub cargos: Vec<CargoJson>,
    /// municípios da eleição (`<uf><município:05>`, do `cm.json`): onde os cargos municipais e as consultas têm região
    #[serde(default)]
    pub municipios: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MunicipioJson {
    pub codigo: u32,
    pub uf: String,
    /// nome do município (`abr[].mu[].nm` do `cm.json`; desde a `0.4.1`, para o site; o motor ignora)
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub nome: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecaoJson {
    pub municipio: u32,
    pub zona: u16,
    pub secao: u16,
    /// seção **agregada** (`nsp` no `cs.json`: juntada a uma seção principal, sem urna nem BU próprio — os votos vêm no
    /// BU da principal). O TSE não a conta no total de seções (`s.ts`); desde a `0.4.9` o total publicado também não.
    /// Fica no catálogo (se uma agregada vier com BU, ele entra e a seção passa a contar)
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub agregada: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ArquivoCandidatos {
    #[serde(default = "default_um_u32")]
    pub versao: u32,
    pub pleito: u32,
    #[serde(default)]
    pub ufs: Vec<String>,
    #[serde(default)]
    pub municipios: Vec<MunicipioJson>,
    #[serde(default)]
    pub secoes: Vec<SecaoJson>,
    #[serde(default)]
    pub eleicoes: Vec<EleicaoJson>,
    /// a destinação dos candidatos foi presumida "Válido" (arquivos sem `dvt`, ou `--ignorar-destinacao`)
    #[serde(default)]
    pub destinacao_presumida: bool,
    /// fontes que faltaram ao montar o arquivo (ex.: `cs.json` de uma UF ainda não publicado); vazio = completo
    #[serde(default)]
    pub incompleto: Vec<String>,
    /// nome de cada UF (`abr[].ds` do `cm.json`: "ac" → "ACRE", "zz" → "EXTERIOR"; desde a `0.4.1`, para o site)
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub nomes_ufs: std::collections::BTreeMap<String, String>,
}

fn default_um_u32() -> u32 { 1 }

/// Eleitorado acima do qual o Prefeito precisa de maioria absoluta no 1º turno (CF, art. 29, II).
pub const LIMITE_2T_PREFEITO: u64 = 200_000;

/// Denominador da maioria absoluta.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Base {
    /// votos dos candidatos aptos (votos válidos) — Presidente e Governador
    Aptos,
    /// votação nominal inteira, inclusive a de candidatos anulados depois da eleição — Prefeito (oficiais de 2024)
    Nominal,
}

/// Sistema de apuração da circunscrição.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sistema {
    /// 1º turno com maioria absoluta: elege quem passa da metade da `base`; senão os 2 primeiros aptos vão ao 2º turno
    MaioriaAbsoluta { base: Base },
    /// os `vagas` aptos mais votados são eleitos (Senador, Prefeito até 200 mil eleitores, 2º turnos, consultas)
    MaioriaSimples { vagas: u8 },
}

impl Sistema {
    /// Sistema e se vale o art. 224 do Código Eleitoral (ninguém eleito quando o mais votado foi anulado ou os
    /// anulados passam de 50 % da votação nominal): em **todo cargo majoritário**, nos dois turnos — observado nos
    /// oficiais de Prefeito de 2024 e no suplementar de Governador de RR de 2026 (B-070, `docs/itens/B-070.md`;
    /// antes da 1.0.35 valia só para Prefeito). Consultas populares não têm candidato: sem art. 224.
    pub fn para(cargo: u8, turno: u8, eleitorado: u64, vagas: u8, consulta: bool) -> (Sistema, bool) {
        if consulta { return (Sistema::MaioriaSimples { vagas: 1 }, false); }
        let art224 = matches!(cargo, 1 | 3 | 5 | 11);
        if turno >= 2 { return (Sistema::MaioriaSimples { vagas: 1 }, art224); }
        match cargo {
            1 | 3 => (Sistema::MaioriaAbsoluta { base: Base::Aptos }, art224),
            11 if eleitorado > LIMITE_2T_PREFEITO => (Sistema::MaioriaAbsoluta { base: Base::Nominal }, true),
            11 => (Sistema::MaioriaSimples { vagas: 1 }, true),
            _ => (Sistema::MaioriaSimples { vagas: vagas.max(1) }, art224),
        }
    }
}

/// Destinação dos votos de um candidato (`dvt` na divulgação).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Destinacao {
    Valido,
    ValidoSubJudice,
    Anulado,
    AnuladoSubJudice,
    /// valor não previsto, guardado como veio (não é apto)
    Outra(String),
}

impl Destinacao {
    /// Valores vistos em 2024 e 2026: "Válido", "Anulado", "Anulado sub judice".
    pub fn de_texto(s: &str) -> Destinacao {
        let s = s.trim();
        let sj = s.to_lowercase().contains("sub judice");
        if s.starts_with("Válido") { if sj { Destinacao::ValidoSubJudice } else { Destinacao::Valido } }
        else if s.starts_with("Anulado") { if sj { Destinacao::AnuladoSubJudice } else { Destinacao::Anulado } }
        else { Destinacao::Outra(s.to_string()) }
    }
    /// Candidato apto: os votos dele são válidos (destinação começa com "Válido").
    pub fn apto(&self) -> bool { matches!(self, Destinacao::Valido | Destinacao::ValidoSubJudice) }
    pub fn texto(&self) -> &str {
        match self {
            Destinacao::Valido => "Válido", Destinacao::ValidoSubJudice => "Válido sub judice",
            Destinacao::Anulado => "Anulado", Destinacao::AnuladoSubJudice => "Anulado sub judice", Destinacao::Outra(s) => s,
        }
    }
}

/// "dd/mm/aaaa" → aaaammdd (ordem cronológica)
pub fn data(s: &str) -> Option<u32> {
    let mut p = s.trim().split('/').map(|x| x.parse::<u32>().ok());
    let (d, m, a) = (p.next()??, p.next()??, p.next()??);
    ((1..=31).contains(&d) && (1..=12).contains(&m)).then_some(a * 10000 + m * 100 + d)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidato {
    /// número na urna (ou da opção, nas consultas)
    pub numero: u32,
    pub destinacao: Destinacao,
    /// data de nascimento, aaaammdd (desempate: o mais idoso fica à frente)
    pub nascimento: Option<u32>,
}

/// Uma circunscrição de um cargo: onde se decide quem é eleito.
#[derive(Clone, Debug)]
pub struct Circunscricao {
    pub abr: Abrangencia,
    pub eleitorado: u64,
    pub candidatos: Vec<Candidato>,
    /// totalização finalizada; se não, a situação fica pendente (como na divulgação, `tf` = n)
    pub finalizada: bool,
    pub sistema: Sistema,
    pub art224: bool,
}

#[derive(Clone, Debug)]
pub struct CargoCad {
    pub cargo: u8,
    pub vagas: u8,
    pub consulta: bool,
    pub circunscricoes: Vec<Circunscricao>,
}

#[derive(Clone, Debug)]
pub struct EleicaoCad {
    pub codigo: u32,
    pub turno: u8,
    pub cargos: Vec<CargoCad>,
}

#[derive(Clone, Debug, Default)]
pub struct Candidatos {
    pub eleicoes: Vec<EleicaoCad>,
    /// (cargo, circunscrição) → (eleição, cargo, circunscrição) em `eleicoes`
    indice: HashMap<(u8, Abrangencia), (usize, usize, usize)>,
    /// eleitorado de cada abrangência publicada (inclusive as menores que a circunscrição, ex.: Presidente por UF)
    pub eleitorado_abr: HashMap<(u8, Abrangencia), u64>,
    /// abrangências publicadas de cada cargo (as dos arquivos de divulgação), em ordem
    pub abrangencias: HashMap<u8, Vec<Abrangencia>>,
    /// seções a totalizar em cada abrangência publicada (`s.ts` pré-apuração), quando conhecidas
    pub secoes_abr: HashMap<(u8, Abrangencia), u64>,
    /// seções agregadas (chave de [`crate::chave`]): fora do total de seções publicado ([`SecaoJson::agregada`])
    pub agregadas: std::collections::HashSet<u64>,
}

/// Circunscrição do cargo que contém a abrangência `abr` (a de um arquivo de resultado).
pub fn circunscricao_de(cargo: u8, abr: &Abrangencia) -> Abrangencia {
    match abr {
        _ if cargo == 1 => Abrangencia::Brasil,
        Abrangencia::Municipio(u, m) if cargo_municipal(cargo) => Abrangencia::Municipio(*u, *m),
        Abrangencia::Municipio(u, _) | Abrangencia::Uf(u) => Abrangencia::Uf(*u),
        Abrangencia::Brasil => Abrangencia::Brasil,
    }
}

impl Candidatos {
    /// Acrescenta uma circunscrição (substitui se já existir).
    #[allow(clippy::too_many_arguments)]
    pub fn acrescentar(&mut self, eleicao: u32, turno: u8, cargo: u8, vagas: u8, consulta: bool, c: Circunscricao) {
        let ei = match self.eleicoes.iter().position(|e| e.codigo == eleicao) {
            Some(i) => i,
            None => { self.eleicoes.push(EleicaoCad { codigo: eleicao, turno, cargos: vec![] }); self.eleicoes.len() - 1 }
        };
        let e = &mut self.eleicoes[ei];
        let ci = match e.cargos.iter().position(|x| x.cargo == cargo) {
            Some(i) => i,
            None => { e.cargos.push(CargoCad { cargo, vagas, consulta, circunscricoes: vec![] }); e.cargos.len() - 1 }
        };
        let cc = &mut e.cargos[ci];
        let key = (cargo, c.abr.clone());
        match self.indice.get(&key) {
            Some(&(a, b, x)) if a == ei && b == ci => cc.circunscricoes[x] = c,
            _ => { cc.circunscricoes.push(c); self.indice.insert(key, (ei, ci, cc.circunscricoes.len() - 1)); }
        }
    }

    pub fn circunscricao(&self, cargo: u8, abr: &Abrangencia) -> Option<&Circunscricao> {
        let &(e, c, x) = self.indice.get(&(cargo, abr.clone()))?;
        Some(&self.eleicoes[e].cargos[c].circunscricoes[x])
    }

    pub fn n_circunscricoes(&self) -> usize { self.indice.len() }

    /// (eleição, turno) do cargo.
    pub fn eleicao_do_cargo(&self, cargo: u8) -> Option<(u32, u8)> {
        self.eleicoes.iter().find(|e| e.cargos.iter().any(|c| c.cargo == cargo)).map(|e| (e.codigo, e.turno))
    }

    /// (eleição, turno) de um arquivo do cargo na abrangência `abr` — o mesmo cargo pode estar em mais de uma
    /// eleição do pleito (ex.: Prefeito em várias eleições suplementares); sem a circunscrição, a do cargo.
    pub fn eleicao_de(&self, cargo: u8, abr: &Abrangencia) -> Option<(u32, u8)> {
        match self.indice.get(&(cargo, circunscricao_de(cargo, abr))) {
            Some(&(e, _, _)) => Some((self.eleicoes[e].codigo, self.eleicoes[e].turno)),
            None => self.eleicao_do_cargo(cargo),
        }
    }

    /// Cargos dos candidatos, em ordem.
    pub fn cargos(&self) -> Vec<u8> {
        let mut v: Vec<u8> = self.eleicoes.iter().flat_map(|e| e.cargos.iter().map(|c| c.cargo)).collect();
        v.sort_unstable();
        v.dedup();
        v
    }

    /// Monta os candidatos com os arquivos oficiais de divulgação: de cada cargo, só os arquivos da própria
    /// circunscrição (Presidente: `br`; Governador e Senador: UF; Prefeito e consultas: município).
    pub fn dos_oficiais(ofs: &[Oficial]) -> Candidatos {
        let mut cad = Candidatos::default();
        for o in ofs {
            let Some(abr) = &o.abr else { continue };
            cad.eleitorado_abr.insert((o.cargo, abr.clone()), o.eleitorado);
            if o.secoes > 0 { cad.secoes_abr.insert((o.cargo, abr.clone()), o.secoes); }
            cad.abrangencias.entry(o.cargo).or_default().push(abr.clone());
            if circunscricao_de(o.cargo, abr) != *abr { continue; }
            let (sistema, art224) = Sistema::para(o.cargo, o.turno, o.eleitorado, o.vagas, o.consulta);
            let candidatos = o.cand.iter().map(|c| Candidato { numero: c.numero, destinacao: Destinacao::de_texto(&c.dvt), nascimento: data(&c.nascimento) }).collect();
            cad.acrescentar(o.eleicao, o.turno, o.cargo, o.vagas, o.consulta,
                Circunscricao { abr: abr.clone(), eleitorado: o.eleitorado, candidatos, finalizada: o.finalizada, sistema, art224 });
        }
        for v in cad.abrangencias.values_mut() { v.sort(); v.dedup(); }
        cad
    }

    /// [`Candidatos::dos_oficiais`] com todos os `-u.json` de uma pasta.
    pub fn dos_arquivos_oficiais(pasta: &Path) -> anyhow::Result<Candidatos> { Ok(Candidatos::dos_oficiais(&Oficial::ler_pasta(pasta)?)) }

    /// Carrega `candidatos.json` e devolve tanto a estrutura `ArquivoCandidatos` quanto o `Candidatos` construído.
    pub fn ler(caminho: &Path) -> anyhow::Result<(Candidatos, ArquivoCandidatos)> {
        let txt = std::fs::read_to_string(caminho)?;
        let arq: ArquivoCandidatos = serde_json::from_str(&txt)?;
        let cad = Candidatos::do_arquivo(&arq);
        Ok((cad, arq))
    }

    /// Constrói `Candidatos` a partir do `ArquivoCandidatos` deserializado.
    pub fn do_arquivo(arq: &ArquivoCandidatos) -> Candidatos {
        let mut cad = Candidatos::default();
        cad.agregadas = arq.secoes.iter().filter(|s| s.agregada).map(|s| crate::chave(s.municipio, s.zona, s.secao)).collect();
        for ele in &arq.eleicoes {
            for carg in &ele.cargos {
                for a in &carg.abrangencias {
                    let Some(abr) = Abrangencia::de_codigo(&a.abr) else { continue };
                    cad.eleitorado_abr.insert((carg.cargo, abr.clone()), a.eleitorado);
                    if a.secoes > 0 { cad.secoes_abr.insert((carg.cargo, abr.clone()), a.secoes); }
                    cad.abrangencias.entry(carg.cargo).or_default().push(abr);
                }
                for circ in &carg.circunscricoes {
                    let Some(abr) = Abrangencia::de_codigo(&circ.abr) else { continue };
                    // só a própria circunscrição decide quem é eleito (ex.: Presidente só no Brasil)
                    if circunscricao_de(carg.cargo, &abr) != abr { continue; }
                    cad.eleitorado_abr.insert((carg.cargo, abr.clone()), circ.eleitorado);
                    if carg.abrangencias.is_empty() { cad.abrangencias.entry(carg.cargo).or_default().push(abr.clone()); }
                    let (sistema, art224) = Sistema::para(carg.cargo, ele.turno, circ.eleitorado, carg.vagas, carg.consulta);
                    let candidatos: Vec<Candidato> = circ.candidatos.iter().map(|c| Candidato {
                        numero: c.numero,
                        destinacao: Destinacao::de_texto(&c.destinacao),
                        nascimento: c.nascimento.as_deref().and_then(data),
                    }).collect();
                    cad.acrescentar(ele.codigo, ele.turno, carg.cargo, carg.vagas, carg.consulta,
                        Circunscricao {
                            abr,
                            eleitorado: circ.eleitorado,
                            candidatos,
                            // a totalização ainda vai acontecer: a situação sai quando a publicação for encerrada
                            finalizada: true,
                            sistema,
                            art224,
                        }
                    );
                }
            }
        }
        for v in cad.abrangencias.values_mut() { v.sort(); v.dedup(); }
        cad
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    #[test]
    fn sistemas() {
        // art. 224 em todo cargo majoritário (B-070): Presidente, Governador, Senador e Prefeito, nos dois turnos
        assert_eq!(Sistema::para(1, 1, 0, 1, false), (Sistema::MaioriaAbsoluta { base: Base::Aptos }, true));
        assert_eq!(Sistema::para(3, 1, 0, 1, false), (Sistema::MaioriaAbsoluta { base: Base::Aptos }, true));
        assert_eq!(Sistema::para(1, 2, 0, 1, false), (Sistema::MaioriaSimples { vagas: 1 }, true));
        assert_eq!(Sistema::para(3, 2, 0, 1, false), (Sistema::MaioriaSimples { vagas: 1 }, true));
        assert_eq!(Sistema::para(5, 1, 0, 2, false), (Sistema::MaioriaSimples { vagas: 2 }, true));
        assert_eq!(Sistema::para(25, 1, 0, 1, false), (Sistema::MaioriaSimples { vagas: 1 }, false), "Conselheiro Distrital: fora");
        assert_eq!(Sistema::para(11, 1, 200_000, 1, false), (Sistema::MaioriaSimples { vagas: 1 }, true));
        assert_eq!(Sistema::para(11, 1, 200_001, 1, false), (Sistema::MaioriaAbsoluta { base: Base::Nominal }, true));
        assert_eq!(Sistema::para(11, 2, 900_000, 1, false), (Sistema::MaioriaSimples { vagas: 1 }, true));
        assert_eq!(Sistema::para(55, 1, 9_000, 1, true), (Sistema::MaioriaSimples { vagas: 1 }, false));
    }
    #[test]
    fn destinacoes_e_datas() {
        assert!(Destinacao::de_texto("Válido").apto());
        assert_eq!(Destinacao::de_texto("Anulado sub judice"), Destinacao::AnuladoSubJudice);
        assert!(!Destinacao::de_texto("Anulado").apto() && !Destinacao::de_texto("Cassado").apto());
        assert_eq!(data("26/07/1971"), Some(19710726));
        assert_eq!(data(""), None);
        assert!(data("01/01/1950") < data("31/12/1951"));
    }
    #[test]
    fn circunscricoes() {
        let mu = Abrangencia::Municipio(*b"ac", 1031);
        assert_eq!(circunscricao_de(1, &Abrangencia::Uf(*b"zz")), Abrangencia::Brasil);
        assert_eq!(circunscricao_de(3, &Abrangencia::Uf(*b"df")), Abrangencia::Uf(*b"df"));
        assert_eq!(circunscricao_de(5, &mu), Abrangencia::Uf(*b"ac"));
        assert_eq!(circunscricao_de(11, &mu), mu);
        assert_eq!(circunscricao_de(55, &mu), mu);
    }
    #[test]
    fn test_leitura_candidatos_json() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/candidatos_exemplo.json");
        let (cad, arq) = Candidatos::ler(&p).expect("ler candidatos_exemplo.json");
        assert_eq!(arq.pleito, 3221);
        assert_eq!(arq.eleicoes.len(), 2);
        assert_eq!(cad.eleicoes.len(), 2);
        assert_eq!(cad.cargos(), vec![1, 11]);
        assert!(arq.destinacao_presumida && arq.incompleto.is_empty());
        // Presidente: uma circunscrição (Brasil); "df" é só abrangência publicada, com o próprio eleitorado
        let (br, df, sp) = (Abrangencia::Brasil, Abrangencia::Uf(*b"df"), Abrangencia::Uf(*b"sp"));
        assert_eq!(cad.n_circunscricoes(), 2);
        assert!(cad.circunscricao(1, &br).is_some() && cad.circunscricao(1, &df).is_none());
        let mut a1 = cad.abrangencias[&1].clone();
        a1.sort();
        let mut esperado = vec![br.clone(), df.clone(), sp.clone()];
        esperado.sort();
        assert_eq!(a1, esperado);
        assert_eq!(cad.eleitorado_abr[&(1, df.clone())], 2_200_000);
        assert_eq!(cad.eleitorado_abr[&(1, br.clone())], 155_823_368);
        assert_eq!(cad.eleicao_de(11, &Abrangencia::Municipio(*b"sp", 67458)), Some((6284, 1)));
        assert_eq!(cad.eleicao_de(1, &df), Some((6258, 2)));
        let cat = crate::catalogo::Catalogo::de_arquivo_candidatos(&arq).expect("catálogo");
        assert_eq!(cat.pleito, 3221);
        assert_eq!(cat.cargos, vec![1, 11]);
        assert_eq!(cat.secoes.len(), 2);
        // Prefeito só no município da eleição suplementar
        let ci = cat.ci(11).unwrap();
        let nc = cat.nc();
        let tem = |m: u32| cat.base[cat.muni(m).unwrap() as usize * nc + ci] != crate::catalogo::AUSENTE;
        assert!(tem(67458) && !tem(97012));
        // UF inválida: erro, não pânico
        let mut ruim = arq.clone();
        ruim.municipios[0].uf = "d".into();
        assert!(crate::catalogo::Catalogo::de_arquivo_candidatos(&ruim).is_err());
    }

    #[test]
    fn agregadas_fora_do_total_de_secoes() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/candidatos_exemplo.json");
        let (_, mut arq) = Candidatos::ler(&p).expect("ler candidatos_exemplo.json");
        assert!(!serde_json::to_string(&arq).unwrap().contains("agregada"), "campo omitido quando falso");
        arq.secoes[0].agregada = true;
        let txt = serde_json::to_string(&arq).unwrap();
        assert!(txt.contains("\"agregada\":true"));
        let arq: ArquivoCandidatos = serde_json::from_str(&txt).unwrap();
        let cad = Candidatos::do_arquivo(&arq);
        assert_eq!(cad.agregadas.len(), 1);
        let cat = crate::catalogo::Catalogo::de_arquivo_candidatos(&arq).expect("catálogo");
        assert_eq!(cat.secoes.len(), 2, "a agregada continua no catálogo");
        let est = crate::estado::Estado::novo(&cat);
        let br = Abrangencia::Brasil;
        assert_eq!(crate::resultado::Publicador::novo(&cat, None, None).secoes(&est, &br), (2, 0));
        assert_eq!(crate::resultado::Publicador::novo(&cat, Some(&cad), None).secoes(&est, &br), (1, 0));
        // B-051: agregada descoberta na apuração sai do total, uma vez só; a do candidatos.json já estava fora
        let mut p = crate::resultado::Publicador::novo(&cat, Some(&cad), None);
        let s = &arq.secoes;
        let uf: [u8; 2] = arq.municipios.iter().find(|m| m.codigo == s[1].municipio).unwrap().uf.as_bytes().try_into().unwrap();
        let (k0, k1) = (crate::chave(s[0].municipio, s[0].zona, s[0].secao), crate::chave(s[1].municipio, s[1].zona, s[1].secao));
        assert_eq!(p.agregada(uf, k0), None, "já fora do total pelo candidatos.json");
        assert_eq!(p.agregada(*b"xx", k1), None, "de outra UF");
        assert_eq!(p.agregada(uf, crate::chave(99_999, 1, 1)), None, "fora do catálogo");
        assert!(p.agregada(uf, k1).is_some());
        assert_eq!(p.secoes(&est, &br), (0, 0));
        assert_eq!(p.agregada(uf, k1), None, "não desconta duas vezes");
        assert_eq!(p.secoes(&est, &br), (0, 0));
    }
}
