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
mod testes {
    use super::*;

    #[test]
    fn fila_retem_na_ordem_e_solta_ate_o_limite() {
        let lote = |v: &[u32]| v.iter().map(|&k| (k, k * 10)).collect::<Vec<_>>();
        let mut f: Fila<u32, u32> = Fila::default();
        // desligada e vazia: passa tudo, na ordem
        assert_eq!(f.passar(lote(&[1, 2, 3]), None), vec![10, 20, 30]);
        assert!(f.is_empty());
        // ligada, cabem 2 de 5: saem os 2 primeiros, 3 retidos
        assert_eq!(f.passar(lote(&[4, 5, 6, 7, 8]), Some(2)), vec![40, 50]);
        assert_eq!(f.len(), 3);
        assert!(f.contem(&6) && f.contem(&8) && !f.contem(&4));
        // a mesma seção retida chegando de novo (sondada outra vez): fica a primeira
        assert_eq!(f.passar(lote(&[6]), Some(0)), Vec::<u32>::new());
        assert_eq!(f.len(), 3);
        // limite 0 (no teto): nada sai, o lote novo entra no fim
        assert_eq!(f.passar(lote(&[9]), Some(0)), Vec::<u32>::new());
        assert_eq!(f.len(), 4);
        // o TSE andou: cabe 1 (sem lote novo, a cada segundo) — sai o mais antigo
        assert_eq!(f.passar(vec![], Some(1)), vec![60]);
        assert!(!f.contem(&6));
        // limite maior que a fila: sai tudo
        assert_eq!(f.passar(vec![], Some(u64::MAX)), vec![70, 80, 90]);
        assert!(f.is_empty() && !f.contem(&9));
        // retidos e a trava desligada no meio: solta os retidos primeiro, depois o lote
        assert_eq!(f.passar(lote(&[11, 12]), Some(0)), Vec::<u32>::new());
        assert_eq!(f.passar(lote(&[13]), None), vec![110, 120, 130]);
        assert!(f.is_empty());
        // seção que saiu pode entrar de novo (BU substituto)
        assert_eq!(f.passar(lote(&[11]), Some(0)), Vec::<u32>::new());
        assert!(f.contem(&11));
    }

    #[test]
    fn trava_e_fila_juntas_no_cenario_do_tse_soltando_tudo() {
        // 1000 seções; TSE em 20 %, nós em 20 %; chegam 500 BUs de uma vez com a trava em 10 p.p.
        let total = 1000u64;
        let mut ap = 200u64;
        let mut f: Fila<u32, ()> = Fila::default();
        let n = aplicaveis(10.0, 20.0, total, ap);
        let sai = f.passar((0..500).map(|k| (k, ())).collect(), Some(n));
        ap += sai.len() as u64;
        assert_eq!((sai.len(), ap, f.len()), (100, 300, 400), "para em TSE + 10 p.p.");
        // o TSE vai a 45 %: solta até 55 %
        let sai = f.passar(vec![], Some(aplicaveis(10.0, 45.0, total, ap)));
        ap += sai.len() as u64;
        assert_eq!((ap, f.len()), (550, 150));
        // 0 = nunca à frente: com o TSE em 50 %, nada sai
        assert!(f.passar(vec![], Some(aplicaveis(0.0, 50.0, total, ap))).is_empty());
        // trava em 100 (sem limite) ou desligada: sai o resto
        assert_eq!(aplicaveis(100.0, 50.0, total, ap), u64::MAX);
        assert_eq!(f.passar(vec![], None).len(), 150);
    }

    #[test]
    fn so_vale_contra_o_tse_com_a_comparacao() {
        assert!(vale("https://resultados.tse.jus.br/oficial", false, true));
        assert!(!vale("https://resultados.tse.jus.br/oficial", true, true), "replay");
        assert!(!vale("https://resultados.tse.jus.br/oficial", false, false), "sem a comparação, sem a % do TSE");
        assert!(!vale("http://127.0.0.1:18081", false, true), "simulador");
    }

    #[test]
    fn teto_em_pontos_percentuais() {
        // desligada (negativo) ou 100: sem limite
        assert_eq!(aplicaveis(-1.0, 0.0, 499_248, 0), u64::MAX);
        assert_eq!(aplicaveis(100.0, 0.0, 499_248, 0), u64::MAX);
        // 0: nunca à frente do TSE (TSE em 30 %: até 300 de 1000)
        assert_eq!(aplicaveis(0.0, 30.0, 1000, 290), 10);
        assert_eq!(aplicaveis(0.0, 30.0, 1000, 300), 0);
        assert_eq!(aplicaveis(0.0, 0.0, 1000, 0), 0);
        // TSE em 0 % (antes do primeiro arquivo): até 10 % das seções
        assert_eq!(aplicaveis(10.0, 0.0, 1000, 0), 100);
        // TSE em 30 %, nós com 380 de 1000: até 400 → mais 20
        assert_eq!(aplicaveis(10.0, 30.0, 1000, 380), 20);
        // já no teto ou acima: nada
        assert_eq!(aplicaveis(10.0, 30.0, 1000, 400), 0);
        assert_eq!(aplicaveis(10.0, 30.0, 1000, 450), 0);
        // TSE perto do fim: o teto passa de 100 % e a trava solta tudo
        assert_eq!(aplicaveis(10.0, 95.0, 1000, 990), u64::MAX);
        assert_eq!(aplicaveis(10.0, 100.0, 1000, 1000), u64::MAX);
    }

    #[test]
    fn le_o_no_env_e_mantem_o_ultimo_tse() {
        assert_eq!(f64::from_bits(DESLIGADA), -1.0);
        assert!(!TRAVA.ligada() || TRAVA.pp() >= 0.0);
        let t = Trava { pp: AtomicU64::new(DESLIGADA), tse_pct: AtomicU64::new(0), retidos: AtomicU64::new(0), soltos: AtomicU64::new(0), lido: Mutex::new(None) };
        let d = std::env::temp_dir().join(format!("uv-trava-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let p = d.join("no.env");
        std::fs::write(&p, "PLEITO=3221\nTRAVA_PP=10\n").unwrap();
        t.reler(&p);
        assert_eq!(t.pp(), 10.0);
        assert!(t.ligada());
        t.definir_tse(499_248, 49_925);
        assert!((t.tse_pct() - 10.0).abs() < 0.01);
        t.definir_tse(0, 0);
        assert!((t.tse_pct() - 10.0).abs() < 0.01, "sem dado válido, fica o último");
        std::fs::write(&p, "PLEITO=3221\nTRAVA_PP=0\n").unwrap();
        *t.lido.lock().unwrap() = None;
        t.reler(&p);
        assert!(t.ligada() && t.pp() == 0.0, "0 = nunca à frente (ligada)");
        std::fs::write(&p, "PLEITO=3221\n").unwrap();
        *t.lido.lock().unwrap() = None;
        t.reler(&p);
        assert!(!t.ligada(), "sem TRAVA_PP: desligada");
        std::fs::write(&p, "TRAVA_PP=100\n").unwrap();
        *t.lido.lock().unwrap() = None;
        t.reler(&p);
        assert!(!t.ligada(), "100 = sem limite");
        let _ = std::fs::remove_dir_all(&d);
    }
}
