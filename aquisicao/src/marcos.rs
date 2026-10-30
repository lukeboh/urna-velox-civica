//! **Marcos e curiosidades da noite** (etapa 5): o que só a thread do motor sabe com exatidão — **qual seção** entrou
//! em cada lote e a conta do "matematicamente" — vira a mensagem `marco` da difusão (`docs/arquitetura/PROTOCOLO_WS.md` §4.5) e uma
//! linha do `<saida>/marcos.csv` (para a conferência e para a retomada: depois de um reinício o histórico é relido daqui).
//!
//! Classes:
//! * `primeira_secao` — a primeira seção totalizada do país (`br`), de cada UF e de cada circunscrição municipal
//!   publicada: UF, município, zona, seção, horário (do BU) e `seq`. É a primeira linha `aplicado` do `secoes.log` da
//!   abrangência (as duas saem do mesmo laço, na mesma ordem);
//! * `secao_decisiva` — a seção do BU que fez a circunscrição virar "matematicamente eleito / 2º turno". O motor avalia
//!   **por lote**: registra-se a **última seção aplicada no lote que pertence à circunscrição** e `lote_bus` (quantos
//!   BUs o lote teve). A 1× os lotes têm 1–3 BUs; a 60× podem ter dezenas, e qualquer seção da circunscrição no mesmo
//!   lote poderia ser "a decisiva" (ambiguidade documentada);
//! * `faltam_para_mat` — quantos eleitores ainda precisam ser apurados até o "matematicamente" ficar garantido
//!   ([`urna_velox::matematica::faltam_para_mat`]); mandado quando muda, no máximo 1×/s por circunscrição, e **0 no
//!   `seq` exato do evento** (no próprio lote). No `uv-servico` a conta é feita na thread `uv-faltam` ([`ContaFaltam`],
//!   desde a 0.8.10, B-009): a verificação de 1 s da thread do motor só tira uma fotografia dos votos das
//!   circunscrições que mudaram e recolhe as respostas da rodada anterior. Antes, a conta rodava na própria verificação
//!   com orçamento de 0,5 ms por rodada e, com os BUs chegando depressa, a circunscrição da tela ficava minutos sem
//!   valor novo;
//! * `bu_marco` — o BU nº 10 mil, 50 mil, 100 mil, 250 mil… da noite (`seq` exato);
//! * `virada` — troca do 1º colocado de uma circunscrição (verificação de 1 s, depois de 1 % do eleitorado apurado);
//! * `pct_secoes` — 25/50/75/90/99/100 % das seções do Brasil e de cada UF (conferido a cada 5 s);
//! * `ultima_secao` — a última seção aplicada quando a apuração encerra.
//!
//! Nada aqui expõe pessoa: só seção, zona e município.

use crate::servico::ChaveUf;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::io::Write as _;
use std::path::Path;
use std::time::{Duration, Instant};
use urna_velox::acompanhamento::Acompanhamento;
use urna_velox::matematica::{faltam_para_mat, MatCirc};
use urna_velox::oficial::Abrangencia;

/// `seq` dos marcos de contagem (`bu_marco`).
pub const MARCOS_BU: [u64; 7] = [10_000, 50_000, 100_000, 250_000, 500_000, 750_000, 1_000_000];
pub const CAB_MARCOS: &str = "seq,horario,classe,abrangencia,cargo,uf,municipio,zona,secao,estado,valor,lote_bus";

/// Uma seção que mudou o estado num lote (aplicada, substituída ou anulada), com a hora do site do BU e o `seq` depois dela.
#[derive(Clone, Debug)]
pub struct SecaoAplicada { pub k: ChaveUf, pub hora: i64, pub seq: u64 }

/// "aaaa-mm-dd hh:mm:ss" (como o `horario` dos eventos).
pub fn horario(t: i64) -> String {
    let (a, m, d, h, mi, s) = urna_velox::metricas::civil(t);
    format!("{a:04}-{m:02}-{d:02} {h:02}:{mi:02}:{s:02}")
}

/// A seção pertence à circunscrição/abrangência?
pub fn dentro(abr: &Abrangencia, k: &ChaveUf) -> bool {
    match abr {
        Abrangencia::Brasil => true,
        Abrangencia::Uf(u) => *u == k.0,
        Abrangencia::Municipio(u, mu) => *u == k.0 && (k.1 >> 32) as u32 == *mu,
    }
}

fn uf_txt(k: &ChaveUf) -> String { String::from_utf8_lossy(&k.0).into_owned() }

/// Fotografia de uma circunscrição em disputa para a conta do `faltam_para_mat` (índice em `Acompanhamento::circs`).
struct PedidoFaltam { i: usize, seq: u64, em: i64, votos: Vec<u64>, restante: u64 }
/// Resultado da conta de um [`PedidoFaltam`].
struct RespostaFaltam { i: usize, seq: u64, em: i64, restante: u64, valor: Option<u64> }

/// Thread `uv-faltam` (B-009): faz a conta do `faltam_para_mat` (uma busca binária sobre o restante, dezenas de
/// avaliações do "matematicamente" por circunscrição) fora da thread do motor, com prioridade baixa. Recebe lotes de
/// fotografias e, se chegarem lotes enquanto conta, fica só com a fotografia mais nova de cada circunscrição.
pub struct ContaFaltam {
    tx: crossbeam_channel::Sender<Vec<PedidoFaltam>>,
    rx: crossbeam_channel::Receiver<RespostaFaltam>,
}

impl ContaFaltam {
    /// Abre a thread com as circunscrições (na ordem de `Acompanhamento::circs`); ela termina quando o `ContaFaltam` sai.
    pub fn iniciar(circs: Vec<urna_velox::candidatos::Circunscricao>) -> std::io::Result<ContaFaltam> {
        let (tx, rx_ped) = crossbeam_channel::unbounded::<Vec<PedidoFaltam>>();
        let (tx_resp, rx) = crossbeam_channel::unbounded::<RespostaFaltam>();
        std::thread::Builder::new().name("uv-faltam".into()).spawn(move || {
            crate::difusao::prioridade_baixa();
            while let Ok(lote) = rx_ped.recv() {
                let mut ped: std::collections::BTreeMap<usize, PedidoFaltam> = lote.into_iter().map(|p| (p.i, p)).collect();
                while let Ok(l) = rx_ped.try_recv() { ped.extend(l.into_iter().map(|p| (p.i, p))); }
                for p in ped.into_values() {
                    let Some(c) = circs.get(p.i) else { continue };
                    let valor = faltam_para_mat(c, &p.votos, p.restante);
                    if tx_resp.send(RespostaFaltam { i: p.i, seq: p.seq, em: p.em, restante: p.restante, valor }).is_err() { return; }
                }
            }
        })?;
        Ok(ContaFaltam { tx, rx })
    }
}

pub struct Marcos {
    arq: Option<std::fs::File>,
    /// município → nome (do `candidatos.json`, para o marco já sair legível)
    nomes: HashMap<u32, String>,
    /// municípios que são circunscrição (Prefeito, consultas municipais)
    munis_circ: HashSet<([u8; 2], u32)>,
    /// abrangências que já têm a primeira seção
    primeira: HashSet<String>,
    /// (abrangência, cargo, estado) com seção decisiva registrada
    decisivas: HashSet<(String, u8, String)>,
    bu_feitos: HashSet<u64>,
    /// a última seção aplicada (para `ultima_secao`)
    pub ultima: Option<SecaoAplicada>,
    ultima_enviada: Option<u64>,
    /// por circunscrição (índice em `Acompanhamento::circs`): `seq` da última conta e o último valor mandado
    faltam: Vec<(Option<u64>, Option<Option<u64>>)>,
    prox_faltam: usize,
    /// conta em paralelo (`uv-servico`; `None` = na própria verificação, com orçamento) e, por circunscrição, o `seq` do
    /// último valor mandado (uma resposta com fotografia mais velha que ele é descartada)
    conta: Option<ContaFaltam>,
    ult_seq: Vec<u64>,
    /// marcos relidos do `marcos.csv` na partida (histórico depois de um reinício)
    pub historico: Vec<Value>,
    /// líder (número) de cada circunscrição na verificação anterior; viradas por (abrangência, cargo)
    lideres: Vec<Option<u32>>,
    n_viradas: HashMap<(String, u8), u64>,
    /// (abrangência, %) já registrados em `pct_secoes`
    pct_feitos: HashSet<(String, u64)>,
}

/// % das seções apuradas que viram marco (`pct_secoes`).
pub const PCT_MARCOS: [u64; 6] = [25, 50, 75, 90, 99, 100];

impl Marcos {
    /// Abre (ou cria) `<saida>/marcos.csv` e relê o que já estava lá (retomada). `saida = None`: sem arquivo (testes).
    pub fn abrir(saida: Option<&Path>, nomes: HashMap<u32, String>, munis_circ: HashSet<([u8; 2], u32)>, n_circs: usize) -> anyhow::Result<Marcos> {
        let mut m = Marcos { arq: None, nomes, munis_circ, primeira: HashSet::new(), decisivas: HashSet::new(), bu_feitos: HashSet::new(), ultima: None,
            ultima_enviada: None, faltam: vec![(None, None); n_circs], prox_faltam: 0, conta: None, ult_seq: vec![0; n_circs], historico: vec![], lideres: vec![None; n_circs],
            n_viradas: HashMap::new(), pct_feitos: HashSet::new() };
        let Some(dir) = saida else { return Ok(m) };
        let p = dir.join("marcos.csv");
        let novo = !p.exists();
        if !novo {
            for l in std::fs::read_to_string(&p)?.lines() {
                let f: Vec<&str> = l.split(',').collect();
                if f.len() != 12 || f[0] == "seq" { continue; }
                let Ok(seq) = f[0].parse::<u64>() else { continue };
                let classe = f[2];
                // `faltam_para_mat` não volta como histórico: é recalculado na primeira verificação
                if classe == "faltam_para_mat" { continue; }
                let num = |s: &str| s.parse::<u64>().ok();
                let mut v = json!({"classe": classe, "seq": seq, "horario": f[1], "abrangencia": f[3], "cargo": num(f[4])});
                if !f[5].is_empty() {
                    let mu = num(f[6]).unwrap_or(0) as u32;
                    v["uf"] = json!(f[5]);
                    v["municipio"] = json!(mu);
                    v["municipio_nome"] = json!(m.nomes.get(&mu).cloned().unwrap_or_default());
                    v["zona"] = json!(num(f[7]));
                    v["secao"] = json!(num(f[8]));
                }
                if !f[9].is_empty() { v["estado"] = json!(f[9]); }
                if let Some(x) = num(f[10]) { v["valor"] = json!(x); }
                if let Some(x) = num(f[11]) { v["lote_bus"] = json!(x); }
                match classe {
                    "primeira_secao" => { m.primeira.insert(f[3].to_string()); }
                    "secao_decisiva" => { m.decisivas.insert((f[3].to_string(), num(f[4]).unwrap_or(0) as u8, f[9].to_string())); }
                    "bu_marco" => { m.bu_feitos.insert(seq); }
                    "ultima_secao" => { m.ultima_enviada = Some(seq); }
                    "pct_secoes" => { m.pct_feitos.insert((f[3].to_string(), num(f[10]).unwrap_or(0))); }
                    "virada" => {
                        let e = m.n_viradas.entry((f[3].to_string(), num(f[4]).unwrap_or(0) as u8)).or_default();
                        *e = (*e).max(num(f[10]).unwrap_or(0));
                        if let Some((a, b)) = f[9].split_once(':') { v["candidato"] = json!(num(a)); v["anterior"] = json!(num(b)); }
                    }
                    _ => {}
                }
                m.historico.push(v);
            }
        }
        let mut a = std::fs::OpenOptions::new().create(true).append(true).open(&p)?;
        if novo { writeln!(a, "{CAB_MARCOS}")?; }
        m.arq = Some(a);
        Ok(m)
    }

    fn gravar(&mut self, v: &Value) {
        let Some(a) = self.arq.as_mut() else { return };
        let s = |k: &str| match &v[k] { Value::Null => String::new(), Value::String(x) => x.replace([',', '\n'], " "), x => x.to_string() };
        let valor = if v["classe"] == "faltam_para_mat" { s("faltam") } else { s("valor") };
        let _ = writeln!(a, "{},{},{},{},{},{},{},{},{},{},{valor},{}", s("seq"), s("horario"), s("classe"), s("abrangencia"), s("cargo"), s("uf"),
            s("municipio"), s("zona"), s("secao"), s("estado"), s("lote_bus"));
    }

    /// Marco de uma seção (`primeira_secao`, `secao_decisiva`, `bu_marco`, `ultima_secao`).
    fn de_secao(&self, classe: &str, abr: &str, cargo: Option<u8>, s: &SecaoAplicada) -> Value {
        let mu = (s.k.1 >> 32) as u32;
        json!({"classe": classe, "seq": s.seq, "horario": horario(s.hora), "abrangencia": abr, "cargo": cargo, "uf": uf_txt(&s.k), "municipio": mu,
            "municipio_nome": self.nomes.get(&mu).cloned().unwrap_or_default(), "zona": (s.k.1 >> 16) as u16, "secao": s.k.1 as u16})
    }

    /// Uma seção recebeu o primeiro BU (ação `aplicado`): primeira seção do país/UF/circunscrição municipal e marcos de
    /// contagem. Devolve os marcos novos (já gravados no `marcos.csv`).
    pub fn aplicado(&mut self, s: &SecaoAplicada) -> Vec<Value> {
        let mut out = vec![];
        let uf = uf_txt(&s.k);
        let mu = (s.k.1 >> 32) as u32;
        let mut abrs = vec!["br".to_string(), uf.clone()];
        if self.munis_circ.contains(&(s.k.0, mu)) { abrs.push(format!("{uf}{mu:05}")); }
        for a in abrs {
            if self.primeira.insert(a.clone()) { out.push(self.de_secao("primeira_secao", &a, None, s)); }
        }
        if MARCOS_BU.contains(&s.seq) && self.bu_feitos.insert(s.seq) {
            let mut v = self.de_secao("bu_marco", "br", None, s);
            v["valor"] = json!(s.seq);
            out.push(v);
        }
        self.ultima = Some(s.clone());
        for v in &out { self.gravar(v); }
        out
    }

    /// Depois da reavaliação de um lote: para cada evento novo de circunscrição (a partir de `n0` em `ac.eventos`), a
    /// seção decisiva (a última seção do lote dentro da circunscrição) e `faltam = 0` no `seq` do evento; numa revogação,
    /// a circunscrição volta para a conta periódica.
    pub fn decisivas(&mut self, ac: &Acompanhamento, n0: usize, lote: &[SecaoAplicada]) -> Vec<Value> {
        let mut out = vec![];
        for e in &ac.eventos[n0.min(ac.eventos.len())..] {
            if e.numero.is_some() { continue; }
            let c = &ac.circs[e.circ];
            let abr = c.abr().codigo();
            if e.estado.starts_with("revogado:") {
                self.faltam[e.circ].0 = None;
                continue;
            }
            if let Some(s) = lote.iter().rev().find(|s| dentro(c.abr(), &s.k)) {
                if self.decisivas.insert((abr.clone(), c.cargo, e.estado.clone())) {
                    let mut v = self.de_secao("secao_decisiva", &abr, Some(c.cargo), s);
                    v["seq"] = json!(e.seq);
                    v["horario"] = json!(horario(e.em));
                    v["estado"] = json!(e.estado);
                    v["lote_bus"] = json!(lote.len());
                    out.push(v);
                }
            }
            let v = json!({"classe": "faltam_para_mat", "seq": e.seq, "horario": horario(e.em), "abrangencia": abr, "cargo": c.cargo,
                "faltam": 0, "restante": c.restante, "estado": e.estado});
            self.faltam[e.circ] = (Some(e.seq), Some(Some(0)));
            if let Some(u) = self.ult_seq.get_mut(e.circ) { *u = (*u).max(e.seq); }
            out.push(v);
        }
        for v in &out { self.gravar(v); }
        out
    }

    /// Conta periódica (verificação de 1 s): `faltam_para_mat` das circunscrições ainda em disputa que receberam BU desde
    /// a última conta, em rodízio, até `orcamento`. Manda só o que mudou. `seq`/`horario` = os da última reavaliação da
    /// circunscrição (os votos usados na conta são os desse instante); antes do primeiro BU, `seq_atual`/`agora`.
    pub fn faltam(&mut self, ac: &Acompanhamento, seq_atual: u64, agora: i64, orcamento: Duration) -> Vec<Value> {
        if self.conta.is_some() { return self.faltam_paralelo(ac, seq_atual, agora); }
        let t0 = Instant::now();
        let n = ac.circs.len();
        let mut out = vec![];
        for passo in 0..n {
            if passo > 0 && t0.elapsed() >= orcamento { break; }
            let i = (self.prox_faltam + passo) % n;
            let c = &ac.circs[i];
            let (seq, em) = c.ultimo.unwrap_or((seq_atual, agora));
            let (feito, ant) = self.faltam[i];
            if feito == Some(seq) { continue; }
            let (valor, estado) = if c.mat_circ == MatCirc::Apurando { (faltam_para_mat(c.circ, &c.votos, c.restante), "apurando") } else { (Some(0), c.mat_circ.texto()) };
            self.faltam[i] = (Some(seq), Some(valor));
            self.prox_faltam = (i + 1) % n;
            if ant == Some(valor) { continue; }
            out.push(json!({"classe": "faltam_para_mat", "seq": seq, "horario": horario(em), "abrangencia": c.abr().codigo(), "cargo": c.cargo,
                "faltam": valor, "restante": c.restante, "estado": if valor.is_none() { "indefinido" } else { estado }}));
        }
        for v in &out { self.gravar(v); }
        out
    }

    /// Liga a conta do `faltam_para_mat` na thread `uv-faltam` (B-009): daqui em diante [`Marcos::faltam`] não faz conta
    /// nenhuma na thread de quem chama.
    pub fn contar_em_paralelo(&mut self, ac: &Acompanhamento) -> std::io::Result<()> {
        if self.ult_seq.len() != ac.circs.len() { self.ult_seq = vec![0; ac.circs.len()]; }
        self.conta = Some(ContaFaltam::iniciar(ac.circs.iter().map(|c| c.circ.clone()).collect())?);
        Ok(())
    }

    /// [`Marcos::faltam`] com a conta em paralelo: recolhe as respostas da rodada anterior (descarta as de fotografia
    /// mais velha que o último valor mandado — o `faltam = 0` do evento, por exemplo) e manda fotografar as
    /// circunscrições em disputa que receberam BU desde a última rodada. Circunscrição já decidida não vai para a conta.
    fn faltam_paralelo(&mut self, ac: &Acompanhamento, seq_atual: u64, agora: i64) -> Vec<Value> {
        let mut out = vec![];
        let resp: Vec<RespostaFaltam> = self.conta.as_ref().map(|c| c.rx.try_iter().collect()).unwrap_or_default();
        for r in resp {
            let Some(c) = ac.circs.get(r.i) else { continue };
            if r.seq < self.ult_seq[r.i] || c.mat_circ != MatCirc::Apurando { continue; }
            self.ult_seq[r.i] = r.seq;
            if self.faltam[r.i].1 == Some(r.valor) { continue; }
            self.faltam[r.i].1 = Some(r.valor);
            out.push(json!({"classe": "faltam_para_mat", "seq": r.seq, "horario": horario(r.em), "abrangencia": c.abr().codigo(), "cargo": c.cargo,
                "faltam": r.valor, "restante": r.restante, "estado": if r.valor.is_none() { "indefinido" } else { "apurando" }}));
        }
        let mut ped = vec![];
        for (i, c) in ac.circs.iter().enumerate() {
            let (seq, em) = c.ultimo.unwrap_or((seq_atual, agora));
            if self.faltam[i].0 == Some(seq) { continue; }
            self.faltam[i].0 = Some(seq);
            if c.mat_circ == MatCirc::Apurando {
                ped.push(PedidoFaltam { i, seq, em, votos: c.votos.clone(), restante: c.restante });
                continue;
            }
            // decidida (o 0 do evento normalmente já saiu em `decisivas`)
            if self.faltam[i].1 == Some(Some(0)) { continue; }
            self.faltam[i].1 = Some(Some(0));
            self.ult_seq[i] = self.ult_seq[i].max(seq);
            out.push(json!({"classe": "faltam_para_mat", "seq": seq, "horario": horario(em), "abrangencia": c.abr().codigo(), "cargo": c.cargo,
                "faltam": 0, "restante": c.restante, "estado": c.mat_circ.texto()}));
        }
        if !ped.is_empty() {
            if let Some(c) = &self.conta { let _ = c.tx.send(ped); }
        }
        for v in &out { self.gravar(v); }
        out
    }

    /// Viradas de liderança (verificação de 1 s): o 1º colocado de cada circunscrição mudou desde a verificação anterior.
    /// Só conta depois de 1 % do eleitorado apurado (antes disso as primeiras seções trocam o líder a toda hora). Uma
    /// virada que vai e volta dentro do mesmo segundo não aparece.
    pub fn viradas(&mut self, ac: &Acompanhamento, seq_atual: u64, agora: i64) -> Vec<Value> {
        if self.lideres.len() != ac.circs.len() { self.lideres = vec![None; ac.circs.len()]; }
        let mut out = vec![];
        for (i, c) in ac.circs.iter().enumerate() {
            if c.apurado == 0 { continue; }
            // líder: mais votos; empate fica com o anterior
            let Some((j, &v)) = c.votos.iter().enumerate().max_by_key(|(j, &v)| (v, std::cmp::Reverse(*j))) else { continue };
            if v == 0 { continue; }
            let n = c.circ.candidatos[j].numero;
            let ant = self.lideres[i];
            if ant.is_some_and(|a| c.votos.get(c.circ.candidatos.iter().position(|k| k.numero == a).unwrap_or(usize::MAX)).copied() == Some(v)) { continue; }
            self.lideres[i] = Some(n);
            let Some(a) = ant.filter(|&a| a != n) else { continue };
            if c.apurado * 100 < c.circ.eleitorado { continue; }
            let abr = c.abr().codigo();
            let k = self.n_viradas.entry((abr.clone(), c.cargo)).or_default();
            *k += 1;
            let (seq, em) = c.ultimo.unwrap_or((seq_atual, agora));
            out.push(json!({"classe": "virada", "seq": seq, "horario": horario(em), "abrangencia": abr, "cargo": c.cargo, "estado": format!("{n}:{a}"),
                "candidato": n, "anterior": a, "valor": *k}));
        }
        for v in &out { self.gravar(v); }
        out
    }

    /// A abrangência já passou por todos os marcos de % (não precisa mais ser contada).
    pub fn pct_completo(&self, abr: &str) -> bool { self.pct_feitos.contains(&(abr.to_string(), 100)) }

    /// Marcos de % das seções apuradas (25, 50, 75, 90, 99, 100) do Brasil e de cada UF, da verificação de 1 s:
    /// `abrs` = (abrangência, seções totais, apuradas) — as mesmas contas da publicação (`secoes`).
    pub fn pct_secoes(&mut self, abrs: &[(String, u64, u64)], seq: u64, agora: i64) -> Vec<Value> {
        let mut out = vec![];
        for (abr, tot, ap) in abrs {
            if *tot == 0 { continue; }
            for p in PCT_MARCOS {
                if ap * 100 < tot * p || !self.pct_feitos.insert((abr.clone(), p)) { continue; }
                out.push(json!({"classe": "pct_secoes", "seq": seq, "horario": horario(agora), "abrangencia": abr, "cargo": null, "valor": p,
                    "secoes": ap, "total": tot}));
            }
        }
        for v in &out { self.gravar(v); }
        out
    }

    /// Apuração encerrada: a última seção aplicada (uma vez por seção).
    pub fn encerrada(&mut self) -> Vec<Value> {
        let Some(s) = self.ultima.clone() else { return vec![] };
        if self.ultima_enviada == Some(s.seq) { return vec![]; }
        self.ultima_enviada = Some(s.seq);
        let v = self.de_secao("ultima_secao", "br", None, &s);
        self.gravar(&v);
        vec![v]
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn k(uf: &[u8; 2], mu: u32, zn: u16, se: u16) -> ChaveUf { (*uf, urna_velox::chave(mu, zn, se)) }

    #[test]
    fn primeira_secao_e_contagem() {
        let munis: HashSet<([u8; 2], u32)> = [(*b"sp", 71072)].into_iter().collect();
        let mut m = Marcos::abrir(None, HashMap::from([(71072, "SÃO PAULO".to_string())]), munis, 0).unwrap();
        let v = m.aplicado(&SecaoAplicada { k: k(b"sp", 71072, 1, 10), hora: 0, seq: 1 });
        let abrs: Vec<&str> = v.iter().map(|x| x["abrangencia"].as_str().unwrap()).collect();
        assert_eq!(abrs, ["br", "sp", "sp71072"]);
        assert_eq!((v[2]["zona"].as_u64(), v[2]["secao"].as_u64(), v[2]["municipio_nome"].as_str()), (Some(1), Some(10), Some("SÃO PAULO")));
        assert!(m.aplicado(&SecaoAplicada { k: k(b"sp", 71072, 2, 1), hora: 0, seq: 2 }).is_empty(), "só a primeira");
        let v = m.aplicado(&SecaoAplicada { k: k(b"rj", 60011, 2, 1), hora: 0, seq: 10_000 });
        assert_eq!(v.iter().map(|x| x["classe"].as_str().unwrap()).collect::<Vec<_>>(), ["primeira_secao", "bu_marco"]);
        assert_eq!(m.encerrada().len(), 1);
        assert!(m.encerrada().is_empty(), "uma vez por seção");
        assert!(dentro(&Abrangencia::Brasil, &k(b"zz", 1, 1, 1)) && dentro(&Abrangencia::Uf(*b"rj"), &k(b"rj", 1, 1, 1)));
        assert!(!dentro(&Abrangencia::Municipio(*b"sp", 71072), &k(b"sp", 71073, 1, 1)));
    }

    #[test]
    fn csv_ida_e_volta() {
        let d = std::env::temp_dir().join(format!("uv-marcos-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        {
            let mut m = Marcos::abrir(Some(&d), HashMap::new(), HashSet::new(), 0).unwrap();
            m.aplicado(&SecaoAplicada { k: k(b"am", 2550, 1, 550), hora: 1_000, seq: 1 });
        }
        let mut m = Marcos::abrir(Some(&d), HashMap::from([(2550, "MANAUS".to_string())]), HashSet::new(), 0).unwrap();
        assert_eq!(m.historico.len(), 2);
        assert_eq!((m.historico[1]["abrangencia"].as_str(), m.historico[1]["municipio_nome"].as_str(), m.historico[1]["secao"].as_u64()), (Some("am"), Some("MANAUS"), Some(550)));
        assert!(m.aplicado(&SecaoAplicada { k: k(b"am", 2550, 1, 551), hora: 1_001, seq: 2 }).is_empty(), "retomada: a primeira seção não se repete");
        let _ = std::fs::remove_dir_all(&d);
    }
}
