//! **Trava "no máximo X à frente do TSE"** (M-020, pedido do Luciano em 25/10/2026, 18h03): se o TSE soltar os BUs de
//! uma vez, o Urna Velox ficaria muito à frente do site oficial. Com a trava ligada, o `uv-servico` só **aplica** BU
//! enquanto a % de seções totalizadas do Urna Velox (Presidente, Brasil) estiver abaixo da % do TSE no
//! `br-c0001-e<eleição>-u.json` mais `TRAVA_PP` pontos percentuais. A coleta continua igual (os BUs são baixados,
//! validados e gravados no pacote); os que passam do teto ficam **retidos**, na ordem de chegada, e entram quando o TSE
//! anda. Sem o arquivo do TSE (ainda não lido, fora do ar), vale o último valor lido (0 % antes do primeiro): a trava
//! nunca solta por falta de dado.
//!
//! O limite vem da chave `TRAVA_PP` do `no.env` (editável no painel dos nós), relida a cada poucos segundos: muda sem
//! reiniciar o serviço. De 0 (nunca à frente do TSE) a 100 (sem limite); vazio ou ausente = desligada (1.0.7, pedido
//! do Luciano às 18h26). Para mudar: `deploy/oci/trava.sh <0-100|desligar>` ou o painel dos nós. Rede (combinado com o Luciano em 25/10/2026): com a trava, o
//! `br-c0001` do TSE é buscado sempre (uma vez por `--tse-intervalo`, com `If-None-Match`), não só com a comparação na
//! tela de algum visitante.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Tópico do TSE que mede a trava.
pub const TOPICO: &str = "br-c0001";

/// Estado da trava, compartilhado entre o motor, a leitura do `no.env` e a saúde.
pub struct Trava {
    /// pontos percentuais à frente (f64 em bits; 0 = nunca à frente, 100 = sem limite, negativo = desligada)
    pp: AtomicU64,
    /// % de seções totalizadas no TSE (f64 em bits; o último valor lido)
    tse_pct: AtomicU64,
    /// BUs retidos agora e soltos desde a partida
    pub retidos: AtomicU64,
    pub soltos: AtomicU64,
    lido: Mutex<Option<(PathBuf, Instant)>>,
}

/// -1.0 em bits: desligada até ler o `no.env`
const DESLIGADA: u64 = 0xBFF0_0000_0000_0000;

pub static TRAVA: Trava = Trava { pp: AtomicU64::new(DESLIGADA), tse_pct: AtomicU64::new(0), retidos: AtomicU64::new(0), soltos: AtomicU64::new(0), lido: Mutex::new(None) };

impl Trava {
    /// Pontos percentuais à frente permitidos, de 0 (nunca à frente do TSE) a 100 (sem limite); negativo = desligada.
    pub fn pp(&self) -> f64 { f64::from_bits(self.pp.load(Relaxed)) }
    pub fn definir_pp(&self, v: f64) { self.pp.store(if v.is_finite() && v >= 0.0 { v.min(100.0) } else { -1.0 }.to_bits(), Relaxed); }
    pub fn ligada(&self) -> bool { (0.0..100.0).contains(&self.pp()) }
    pub fn tse_pct(&self) -> f64 { f64::from_bits(self.tse_pct.load(Relaxed)) }
    /// Novo valor lido do TSE (só valores válidos; nunca volta a "sem dado").
    pub fn definir_tse(&self, total: u64, totalizadas: u64) {
        if total > 0 { self.tse_pct.store((totalizadas.min(total) as f64 * 100.0 / total as f64).to_bits(), Relaxed); }
    }
    /// Relê o `TRAVA_PP` do `no.env` no máximo a cada 5 s.
    pub fn reler(&self, no_env: &Path) {
        {
            let mut l = self.lido.lock().unwrap();
            if l.as_ref().is_some_and(|(p, t)| p == no_env && t.elapsed() < Duration::from_secs(5)) { return; }
            *l = Some((no_env.to_path_buf(), Instant::now()));
        }
        let Ok(txt) = std::fs::read_to_string(no_env) else { return };
        // ausente, vazio ou ilegível: desligada
        let v = crate::painel::config_no::ler(&txt).get("TRAVA_PP").and_then(|x| x.replace(',', ".").parse::<f64>().ok()).unwrap_or(-1.0);
        self.definir_pp(v);
    }
}

/// Quantos BUs ainda podem ser aplicados agora: o teto é `(tse_pct + pp)` % das `total` seções, menos as `apuradas`.
/// `pp` de 0 (nunca à frente do TSE) a 100 (sem limite); negativo = trava desligada.
pub fn aplicaveis(pp: f64, tse_pct: f64, total: u64, apuradas: u64) -> u64 {
    if !(0.0..100.0).contains(&pp) { return u64::MAX; }
    let teto = (((tse_pct.max(0.0) + pp) / 100.0) * total as f64).floor();
    if teto >= total as f64 { return u64::MAX; }
    (teto as u64).saturating_sub(apuradas)
}

/// A trava vale neste serviço? Só contra o site do TSE (não contra o simulador nem no replay) e com a comparação
/// ligada (é ela que busca o `br-c0001`; sem ela a % do TSE ficaria em 0 e a trava seguraria tudo).
pub fn vale(base: &str, controle_replay: bool, com_comparacao: bool) -> bool {
    base.contains("tse.jus.br") && !controle_replay && com_comparacao
}

/// Fila dos BUs retidos pela trava: na ordem de chegada, uma entrada por seção (a mesma seção de novo, ex.: sondada
/// outra vez, fica com a primeira).
pub struct Fila<K, T> {
    itens: std::collections::VecDeque<(K, T)>,
    chaves: std::collections::HashSet<K>,
}

impl<K: std::hash::Hash + Eq + Copy, T> Default for Fila<K, T> {
    fn default() -> Self { Fila { itens: Default::default(), chaves: Default::default() } }
}

impl<K: std::hash::Hash + Eq + Copy, T> Fila<K, T> {
    pub fn len(&self) -> usize { self.itens.len() }
    pub fn is_empty(&self) -> bool { self.itens.is_empty() }
    pub fn contem(&self, k: &K) -> bool { self.chaves.contains(k) }

    /// Passa um lote pela trava. `limite`: `None` = trava desligada (solta tudo o que estava retido, na ordem, e depois
    /// o lote); `Some(n)` = o lote entra na fila e saem os `n` primeiros (na ordem de chegada).
    pub fn passar(&mut self, lote: Vec<(K, T)>, limite: Option<u64>) -> Vec<T> {
        if limite.is_none() && self.itens.is_empty() { return lote.into_iter().map(|x| x.1).collect(); }
        for (k, t) in lote { if self.chaves.insert(k) { self.itens.push_back((k, t)); } }
        let n = limite.map_or(self.itens.len(), |n| n.min(self.itens.len() as u64) as usize);
        self.itens.drain(..n).map(|(k, t)| { self.chaves.remove(&k); t }).collect()
    }
}

#[cfg(test)]
mod testes;
