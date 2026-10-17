//! Acompanhamento das circunscrições durante a totalização: a cada lote de BUs, atualiza os votos e o eleitorado
//! ainda não apurado só das regiões (município × cargo) que receberam BU — por diferença em relação à leitura
//! anterior, então o custo é proporcional aos BUs do lote, não ao tamanho da circunscrição — e reavalia o estado
//! "matemático" ([`avaliar`]) só das circunscrições que mudaram, e registra a
//! **sequência** (nº de BUs aplicados) e o **horário** do BU que disparou cada mudança — os eventos que a etapa 4
//! publica por WebSocket. Um "matematicamente" que deixasse de valer seria uma **revogação** (contada; não deve
//! acontecer).

use crate::candidatos::{Candidatos, Circunscricao};
use crate::catalogo::{Catalogo, AUSENTE};
use crate::estado::Estado;
use crate::matematica::{avaliar, Mat, MatCirc};
use crate::oficial::Abrangencia;
use std::collections::HashMap;
use std::sync::atomic::Ordering::Relaxed;

/// Uma mudança de estado.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Evento {
    /// índice em `Acompanhamento::circs`
    pub circ: usize,
    /// candidato (`None` = a circunscrição)
    pub numero: Option<u32>,
    /// "eleito", "2t" (candidato); "matematicamente_eleito", "matematicamente_2t" (circunscrição); ou o mesmo
    /// prefixado por "revogado:"
    pub estado: String,
    pub seq: u64,
    /// horário do BU que disparou (segundos, ver `metricas::segundos`)
    pub em: i64,
    /// por que uma declaração foi revogada (BU retirado, substituído ou seção anulada depois de totalizada; ver
    /// [`Acompanhamento::bu_alterado`]); vazio nos demais eventos
    pub motivo: String,
}

/// Uma região do estado (município × cargo) que compõe a circunscrição.
struct Regiao {
    base: usize,
    /// índice da região em `Estado::aptos`
    k: usize,
    /// candidato local (posição na lista da região) → índice em `circ.candidatos` (u32::MAX = fora do catálogo)
    mapa: Vec<u32>,
    /// última leitura dos contadores (votos de cada candidato local e aptos), para somar só a diferença
    lido: Vec<u32>,
    lido_aptos: u32,
    sujo: bool,
}

pub struct CircAcomp<'a> {
    pub cargo: u8,
    pub circ: &'a Circunscricao,
    regioes: Vec<Regiao>,
    regioes_sujas: Vec<u32>,
    /// votos de cada candidato da lista (ordem de `circ.candidatos`)
    pub votos: Vec<u64>,
    /// eleitores aptos das seções já totalizadas e eleitorado restante (`R`)
    pub apurado: u64,
    pub restante: u64,
    pub mat: Vec<Mat>,
    pub mat_circ: MatCirc,
    /// (seq, horário) em que cada candidato ficou matematicamente eleito / no 2º turno
    pub mat_em: Vec<Option<(u64, i64)>>,
    pub circ_em: Option<(u64, i64)>,
    /// (seq, horário) da última reavaliação = do último BU recebido pela circunscrição
    pub ultimo: Option<(u64, i64)>,
    pub revogacoes: u32,
}

impl CircAcomp<'_> {
    pub fn abr(&self) -> &Abrangencia { &self.circ.abr }
}

pub struct Acompanhamento<'a> {
    pub circs: Vec<CircAcomp<'a>>,
    /// município (índice no catálogo) → (circunscrição, região) que ele compõe
    por_muni: Vec<Vec<(u32, u32)>>,
    sujo: Vec<bool>,
    sujos: Vec<u32>,
    pub eventos: Vec<Evento>,
    pub avaliacoes: u64,
    indice: HashMap<(u8, Abrangencia), usize>,
    /// motivo de alteração (retirada/substituição/anulação) pendente, por circunscrição, até a próxima avaliação
    motivos: Vec<String>,
}

impl<'a> Acompanhamento<'a> {
    /// Todas as circunscrições dos candidatos cujo cargo está no catálogo (ou só as de `cargos`).
    pub fn novo(cat: &Catalogo, cad: &'a Candidatos, cargos: Option<&[u8]>) -> Acompanhamento<'a> {
        let nc = cat.nc();
        let mut circs = vec![];
        let mut por_muni: Vec<Vec<(u32, u32)>> = vec![vec![]; cat.muni_cod.len()];
        for e in &cad.eleicoes {
            for cc in &e.cargos {
                if cargos.is_some_and(|s| !s.contains(&cc.cargo)) { continue; }
                let Some(ci) = cat.ci(cc.cargo) else { continue };
                for circ in &cc.circunscricoes {
                    let idx: HashMap<u32, u32> = circ.candidatos.iter().enumerate().map(|(i, c)| (c.numero, i as u32)).collect();
                    let munis: Vec<usize> = match &circ.abr {
                        Abrangencia::Brasil => (0..cat.muni_cod.len()).collect(),
                        Abrangencia::Uf(u) => match cat.ufs.iter().position(|x| x == u) {
                            Some(ui) => (0..cat.muni_cod.len()).filter(|&m| cat.muni_uf[m] as usize == ui).collect(),
                            None => vec![],
                        },
                        Abrangencia::Municipio(u, mu) => cat.muni(*mu).map(|m| m as usize).filter(|&m| cat.ufs[cat.muni_uf[m] as usize] == *u).into_iter().collect(),
                    };
                    let id = circs.len() as u32;
                    let mut regioes = vec![];
                    for m in munis {
                        let k = m * nc + ci;
                        if cat.base[k] == AUSENTE { continue; }
                        let mapa: Vec<u32> = cat.candidatos(m, ci).iter().map(|c| idx.get(c).copied().unwrap_or(u32::MAX)).collect();
                        por_muni[m].push((id, regioes.len() as u32));
                        regioes.push(Regiao { base: cat.base[k] as usize, k, lido: vec![0; mapa.len()], mapa, lido_aptos: 0, sujo: false });
                    }
                    let n = circ.candidatos.len();
                    circs.push(CircAcomp { cargo: cc.cargo, circ, regioes, regioes_sujas: vec![], votos: vec![0; n], apurado: 0, restante: circ.eleitorado,
                        mat: vec![Mat::Nao; n], mat_circ: MatCirc::Apurando, mat_em: vec![None; n], circ_em: None, ultimo: None, revogacoes: 0 });
                }
            }
        }
        let nci = circs.len();
        let indice = circs.iter().enumerate().map(|(i, c)| ((c.cargo, c.circ.abr.clone()), i)).collect();
        Acompanhamento { circs, por_muni, sujo: vec![false; nci], sujos: vec![], eventos: vec![], avaliacoes: 0, indice, motivos: vec![String::new(); nci] }
    }

    /// Um BU do município `m` foi retirado ou trocado (reenvio com conteúdo diferente, anulação depois de
    /// totalizada): marca as circunscrições para reavaliação e guarda o motivo, que acompanha qualquer revogação
    /// de "matematicamente" que resultar disso.
    pub fn bu_alterado(&mut self, m: usize, motivo: &str) {
        for &(c, _) in &self.por_muni[m] {
            let x = &mut self.motivos[c as usize];
            if x.len() > 400 { continue; }
            if !x.is_empty() { x.push_str("; "); }
            x.push_str(motivo);
        }
        self.bu_aplicado(m);
    }

    /// Um BU do município `m` (índice no catálogo) foi aplicado ao estado.
    #[inline]
    pub fn bu_aplicado(&mut self, m: usize) {
        for &(c, r) in &self.por_muni[m] {
            let ca = &mut self.circs[c as usize];
            let reg = &mut ca.regioes[r as usize];
            if !reg.sujo { reg.sujo = true; ca.regioes_sujas.push(r); }
            if !self.sujo[c as usize] { self.sujo[c as usize] = true; self.sujos.push(c); }
        }
    }

    /// Marca todas as regiões e circunscrições para releitura e reavaliação.
    pub fn tudo_sujo(&mut self) {
        for m in 0..self.por_muni.len() { self.bu_aplicado(m); }
        for c in 0..self.circs.len() { if !self.sujo[c] { self.sujo[c] = true; self.sujos.push(c as u32); } }
    }

    /// Reavalia as circunscrições que receberam BU desde a última chamada. Devolve quantos eventos novos houve.
    pub fn atualizar(&mut self, est: &Estado, seq: u64, em: i64) -> usize {
        let antes = self.eventos.len();
        let sujos = std::mem::take(&mut self.sujos);
        for &ci in &sujos {
            let ci = ci as usize;
            self.sujo[ci] = false;
            self.avaliacoes += 1;
            let c = &mut self.circs[ci];
            let motivo = std::mem::take(&mut self.motivos[ci]);
            // soma a diferença (com sinal: um BU retirado diminui os contadores) desde a última leitura de cada
            // região que recebeu BU
            for &ri in &c.regioes_sujas {
                let r = &mut c.regioes[ri as usize];
                r.sujo = false;
                let a = est.aptos[r.k].load(Relaxed);
                c.apurado = (c.apurado as i64 + (a as i64 - r.lido_aptos as i64)) as u64;
                r.lido_aptos = a;
                for (j, &i) in r.mapa.iter().enumerate() {
                    let v = est.tally[r.base + j].load(Relaxed);
                    if i != u32::MAX { c.votos[i as usize] = (c.votos[i as usize] as i64 + (v as i64 - r.lido[j] as i64)) as u64; }
                    r.lido[j] = v;
                }
            }
            c.regioes_sujas.clear();
            c.restante = c.circ.eleitorado.saturating_sub(c.apurado);
            c.ultimo = Some((seq, em));
            let a = avaliar(c.circ, &c.votos, c.restante);
            for (i, &m) in a.cand.iter().enumerate() {
                if m == c.mat[i] { continue; }
                let numero = Some(c.circ.candidatos[i].numero);
                if c.mat[i] != Mat::Nao {
                    c.revogacoes += 1;
                    self.eventos.push(Evento { circ: ci, numero, estado: format!("revogado:{}", c.mat[i].texto()), seq, em, motivo: motivo.clone() });
                }
                if m != Mat::Nao {
                    c.mat_em[i] = Some((seq, em));
                    self.eventos.push(Evento { circ: ci, numero, estado: m.texto().into(), seq, em, motivo: String::new() });
                } else {
                    c.mat_em[i] = None;
                }
                c.mat[i] = m;
            }
            if a.circ != c.mat_circ {
                if c.mat_circ != MatCirc::Apurando {
                    c.revogacoes += 1;
                    self.eventos.push(Evento { circ: ci, numero: None, estado: format!("revogado:{}", c.mat_circ.texto()), seq, em, motivo: motivo.clone() });
                }
                if a.circ != MatCirc::Apurando {
                    c.circ_em = Some((seq, em));
                    self.eventos.push(Evento { circ: ci, numero: None, estado: a.circ.texto().into(), seq, em, motivo: String::new() });
                } else {
                    c.circ_em = None;
                }
                c.mat_circ = a.circ;
            }
        }
        self.sujos = sujos;
        self.sujos.clear();
        self.eventos.len() - antes
    }

    /// A circunscrição `abr` (a própria, não uma abrangência menor) do cargo.
    pub fn circ_de(&self, cargo: u8, abr: &Abrangencia) -> Option<&CircAcomp<'a>> { self.indice.get(&(cargo, abr.clone())).map(|&i| &self.circs[i]) }

    pub fn revogacoes(&self) -> u32 { self.circs.iter().map(|c| c.revogacoes).sum() }
}
