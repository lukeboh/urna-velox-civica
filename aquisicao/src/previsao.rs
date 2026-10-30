//! Previsão do "matematicamente" e do fim da apuração no `uv-servico` (etapa 8, `docs/handoffs/HANDOFF_0.8.0.md`; o cálculo é o
//! do motor, `urna_velox::previsao`).
//!
//! Roda numa thread própria (`uv-previsao`), **fora da thread do motor**: lê o estado (contadores atômicos) e os
//! candidatos, e recebe do motor só os instantes em que cada circunscrição ficou matematicamente decidida. A cada
//! `--previsao-intervalo` segundos do relógio do serviço (o do site; no replay, o simulado), registra o avanço de cada
//! tópico publicado e prevê só os que mudaram; as previsões vão para a difusão (mensagem `previsao`,
//! `docs/arquitetura/PROTOCOLO_WS.md` §4.7) e para `<saida>/previsoes.csv`. Sempre **estimativa**, com faixa P10–P90.

use crate::relogio::Ancora;
use crate::servico::{Amostra, Comum};
use serde_json::{json, Value};
use std::io::Write as _;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use urna_velox::candidatos::Candidatos;
use urna_velox::estado::Estado;
use urna_velox::oficial::Abrangencia;
use urna_velox::previsao::{Faixa, Previsor};

#[derive(Clone, Debug)]
pub struct Opcoes {
    /// segundos (relógio do serviço) entre duas rodadas; 0 = previsão desligada
    pub intervalo_s: i64,
    pub threads: usize,
}

impl Default for Opcoes {
    fn default() -> Self { Opcoes { intervalo_s: 30, threads: 4 } }
}

/// Uma decisão do motor: (cargo, circunscrição, horário do BU que decidiu).
pub type Definicao = (u8, Abrangencia, i64);

/// "2026-10-25 19:42:10" (como o `horario` dos eventos).
pub fn hora(t: i64) -> String {
    let (a, m, d, h, mi, s) = urna_velox::metricas::civil(t);
    format!("{a:04}-{m:02}-{d:02} {h:02}:{mi:02}:{s:02}")
}

fn r(x: f64, casas: i32) -> f64 { let k = 10f64.powi(casas); (x * k).round() / k }


/// Histórico das faixas do "matematicamente" da noite, no máximo um ponto por minuto: `[instante, p10, p50, p90]` em
/// segundos civis de Brasília (a escala do `GET /api/v1/curva`).
/// M-003: com a âncora, um ponto por minuto **da noite**, em segundos (com fração) do relógio do sistema.
fn historico(h: &[(i64, Faixa)], anc: Option<Ancora>) -> Value {
    let mut out: Vec<(i64, [i64; 4])> = vec![];
    for (t, f) in h {
        let p = [*t, f.p10, f.p50, f.p90];
        match out.last_mut() { Some(u) if t / 60 == u.0 / 60 => *u = (*t, p), _ => out.push((*t, p)) }
    }
    match anc {
        None => json!(out.iter().map(|x| x.1).collect::<Vec<_>>()),
        Some(a) => json!(out.iter().map(|x| x.1.map(|t| a.sistema_ms(t as f64) as f64 / 1000.0)).collect::<Vec<_>>()),
    }
}

/// A mensagem (sem `tipo`/`t_envio`) da última previsão da unidade `i`, com o histórico (a difusão o tira das
/// atualizações e o deixa só na mensagem da inscrição).
///
/// M-003: com a âncora (`anc`), a conta é feita na escala da noite pelo relógio do sistema e todos os horários saem
/// **no relógio do sistema** (`relogio: "sistema"`): no simulado a 500×, "matematicamente entre 07:04 e 07:05" de hoje.
/// Sem âncora (antes do primeiro horário do site), no relógio do site, como antes.
pub fn mensagem(prev: &Previsor, i: usize, anc: Option<Ancora>) -> Option<Value> {
    let u = &prev.unidades[i];
    let p = u.ultima.as_ref()?;
    let c = prev.confirmacao(i);
    let s = |t: i64| anc.map_or(t, |a| a.sistema(t));
    let h = |t: i64| hora(s(t));
    let faixa = |f: &Faixa| json!({"p10": h(f.p10), "p50": h(f.p50), "p90": h(f.p90), "incerta": f.incerta, "eta": f.eta.map(h)});
    Some(json!({
        "topico": format!("{}-c{:04}", u.abr.codigo(), u.cargo), "abrangencia": u.abr.codigo(), "cargo": u.cargo, "circunscricao": u.circ,
        "estimativa": true, "relogio": if anc.is_some() { "sistema" } else { "site" }, "instante": h(p.instante), "situacao": p.situacao,
        "parada": p.parada.map(|d| json!({"desde": h(d), "minutos": (s(p.instante) - s(d)).max(0) / 60})),
        "base": {"apurado": r(p.apurado, 4), "secoes": r(p.secoes, 4), "aptos_min": p.aptos_min.round()},
        "mat": p.mat.as_ref().map(|m| { let mut v = faixa(&m.faixa); v["tipo"] = json!(m.tipo); v["prob_antes_fim"] = json!(r(m.prob_antes_fim, 3)); v["apurado_p50"] = json!(r(m.apurado_p50, 4)); v }),
        "fim99": p.fim99.as_ref().map(faixa), "fim100": p.fim100.as_ref().map(faixa),
        "chegou": {"fim99": u.chegou[0].map(h), "fim100": u.chegou[1].map(h)},
        "definida": u.definida.map(h),
        "confirmacao": c.map(|c| json!({"saiu": h(c.saiu), "dentro": c.dentro, "faixa": {"p10": h(c.faixa.p10), "p90": h(c.faixa.p90)}, "desde": c.desde.map(h)})),
        "historico": historico(&u.historico, anc),
    }))
}

/// Relógio da previsão (B-013/B-016): o do site (`rel`, o maior `Date` visto); mas, se ele está parado há mais de
/// `RELOGIO_PARADO` e os BUs continuam entrando (`seq` mudou há menos de `BUS_ENTRANDO`), ele anda sozinho desde a
/// última mudança, na velocidade do site (`escala` = 1/aceleração). No modo rápido do ciclo o relógio do simulador
/// chega ao fim da noite antes de o serviço aplicar todos os BUs (nos nós ARM, minutos antes): sem isto a previsão
/// parava na metade da apuração e o gráfico do cartão não andava mais. Na noite real o relógio do site anda sempre.
pub fn relogio(rel: i64, rel_parado: Duration, sem_bu: Duration, escala: f64) -> i64 {
    if rel <= 0 || rel_parado < RELOGIO_PARADO || sem_bu > BUS_ENTRANDO || !(escala > 0.0) { return rel; }
    rel + (rel_parado.as_secs_f64() / escala) as i64
}
const RELOGIO_PARADO: Duration = Duration::from_secs(3);
const BUS_ENTRANDO: Duration = Duration::from_secs(5);

const CAB_CSV: &str = "instante,topico,situacao,apurado,secoes,aptos_min,mat_tipo,mat_p10,mat_p50,mat_p90,prob_antes_fim,fim99_p10,fim99_p50,fim99_p90,fim100_p10,fim100_p50,fim100_p90,definida";

/// Laço da thread `uv-previsao`, até `parar` (ou o serviço parar).
#[allow(clippy::too_many_arguments)]
pub fn rodar(com: &Comum, cad: &Candidatos, est: &Estado, mut prev: Previsor, rx: crossbeam_channel::Receiver<Definicao>, op: Opcoes,
             parar: &AtomicBool, saude: &Mutex<Value>, saida: &Path) {
    let iv = op.intervalo_s.max(1);
    prev.threads = op.threads.max(1);
    let p_csv = saida.join("previsoes.csv");
    let novo = !p_csv.exists();
    let mut csv = std::fs::OpenOptions::new().create(true).append(true).open(&p_csv).ok();
    if novo { if let Some(f) = csv.as_mut() { let _ = writeln!(f, "{CAB_CSV}"); } }
    let mut ultimo: Vec<(u64, u64, bool)> = vec![(0, 0, false); prev.unidades.len()];
    let mut prox = 0i64;
    let (mut rodadas, mut previstas, mut mensagens) = (0u64, 0u64, 0u64);
    let mut custo = Amostra::com_janela(2048);
    let (mut seq_ant, mut seq_mudou) = (u64::MAX, Instant::now());
    let (mut rel_ant, mut rel_mudou) = (i64::MIN, Instant::now());
    let mut agora_ant = 0i64;
    while !parar.load(Relaxed) && !com.parar.load(Relaxed) {
        while let Ok((c, a, em)) = rx.try_recv() { prev.definir(c, &a, em); }
        let seq = est.bus.load(Relaxed);
        if seq != seq_ant { seq_ant = seq; seq_mudou = Instant::now(); }
        let rel = com.relogio_site.load(Relaxed);
        if rel != rel_ant { rel_ant = rel; rel_mudou = Instant::now(); }
        // M-003: o relógio do sistema, na escala da noite (âncora); antes dela, o do site (B-013)
        let agora = com.relogio.noite_agora()
            .unwrap_or_else(|| relogio(rel, rel_mudou.elapsed(), seq_mudou.elapsed(), f64::from_bits(com.escala.load(Relaxed)))).max(agora_ant);
        agora_ant = agora;
        if agora <= 0 || agora < prox { std::thread::sleep(Duration::from_millis(100)); continue; }
        prox = (agora / iv + 1) * iv;
        let t0 = Instant::now();
        prev.registrar(est, agora);
        let quais: Vec<usize> = prev.unidades.iter().enumerate().filter_map(|(i, u)| {
            let p = u.serie.ultimo()?;
            let def = u.definida.is_some();
            let antes = std::mem::replace(&mut ultimo[i], (p.aptos, p.secoes, def));
            (p.aptos != antes.0 || p.secoes != antes.1 || def != antes.2).then_some(i)
        }).collect();
        if quais.is_empty() && !prev.rodada_pendente() { continue; }
        let feitos = prev.prever(cad, est, agora, Some(&quais));
        let mut msgs = vec![];
        let mut linhas = String::new();
        let anc = com.relogio.ancora();
        for i in feitos {
            let u = &prev.unidades[i];
            let Some(p) = &u.ultima else { continue };
            if p.situacao == "insuficiente" { continue; }
            let Some(m) = mensagem(&prev, i, anc) else { continue };
            let f = |x: Option<&Faixa>| x.map(|f| format!("{},{},{}", hora(f.p10), hora(f.p50), hora(f.p90))).unwrap_or_else(|| ",,".into());
            use std::fmt::Write as _;
            let _ = writeln!(linhas, "{},{},{},{:.4},{:.4},{:.0},{},{},{},{},{},{}", hora(agora), m["topico"].as_str().unwrap_or(""), p.situacao, p.apurado, p.secoes, p.aptos_min,
                p.mat.as_ref().map_or("", |x| x.tipo), f(p.mat.as_ref().map(|x| &x.faixa)), p.mat.as_ref().map(|x| format!("{:.3}", x.prob_antes_fim)).unwrap_or_default(),
                f(p.fim99.as_ref()), f(p.fim100.as_ref()), u.definida.map(hora).unwrap_or_default());
            msgs.push(m);
        }
        rodadas += 1;
        previstas += quais.len() as u64;
        mensagens += msgs.len() as u64;
        if let Some(f) = csv.as_mut() { let _ = f.write_all(linhas.as_bytes()); }
        if !msgs.is_empty() { if let Some(h) = &com.difusao { h.entregar(crate::difusao::Entrada::Previsoes(msgs)); } }
        custo.por(t0.elapsed().as_secs_f64() * 1e3);
        *saude.lock().unwrap() = json!({"intervalo_s": iv, "threads": prev.threads, "topicos": prev.unidades.len(), "rodadas": rodadas, "previstas": previstas,
            "mensagens": mensagens, "ultima": hora(agora), "ultima_unidades": quais.len(), "custo_ms": custo.json(),
            "definidas": prev.unidades.iter().filter(|u| u.definida.is_some()).count()});
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn relogio_anda_quando_o_site_para_e_os_bus_entram() {
        let s = Duration::from_secs;
        // relógio do site andando: é ele
        assert_eq!(relogio(1000, s(1), s(0), 1.0 / 500.0), 1000);
        // parado há 10 s com BUs entrando, a 500×: anda 5.000 s da noite
        assert_eq!(relogio(1000, s(10), s(1), 1.0 / 500.0), 6000);
        // parado e sem BU novo (apuração acabou ou parada de verdade): fica
        assert_eq!(relogio(1000, s(10), s(30), 1.0 / 500.0), 1000);
        // antes do primeiro Date
        assert_eq!(relogio(0, s(10), s(1), 1.0), 0);
    }

    /// B-052: a previsão sempre dá um horário, mesmo quando a faixa P10–P90 é larga demais ("ainda incerto"). Cenário do
    /// modo rápido do ciclo: o relógio da previsão anda a 500× (âncora) e os BUs entram no ritmo do serviço, devagar e
    /// constante na escala da noite — a faixa passa do limite de 3 h da noite (22 s do sistema) e, antes da correção, a
    /// mensagem só dizia `incerta`, sem horário nenhum. Governador de uma UF com 4 municípios × 100 seções de 1.000
    /// eleitores, 1 seção a cada 250 s da noite; com metade apurada, o ritmo atual dá o fim em 200 × 250 s = 50.000 s.
    #[test]
    fn previsao_sempre_com_horario() {
        use std::collections::{BTreeMap, BTreeSet, HashSet};
        use urna_velox::candidatos::{Base, Candidato, Circunscricao, Destinacao, Sistema};
        use urna_velox::catalogo::Catalogo;
        use urna_velox::previsao::{Gabaritos, Parametros};
        let (uf, cargo, nums) = (*b"ac", 3u8, [11u32, 22, 33]);
        let munis: Vec<(u32, u8)> = (1..=4).map(|m| (m, 0)).collect();
        let secoes: Vec<u64> = (1..=4u32).flat_map(|m| (1..=100u16).map(move |s| urna_velox::chave(m, 1, s))).collect();
        let cands = BTreeMap::from([((0u8, cargo), nums.iter().copied().collect::<BTreeSet<u32>>())]);
        let cat = Catalogo::montar(9999, vec![uf], munis, cands, BTreeMap::new(), &[(cargo, 1)], secoes);
        let mut cad = Candidatos::default();
        cad.acrescentar(9999, 1, cargo, 1, false, Circunscricao { abr: Abrangencia::Uf(uf), eleitorado: 400_000, finalizada: false,
            sistema: Sistema::MaioriaAbsoluta { base: Base::Aptos }, art224: false,
            candidatos: nums.iter().map(|&n| Candidato { numero: n, destinacao: Destinacao::Valido, nascimento: None }).collect() });
        let est = Estado::novo(&cat);
        let mut prev = Previsor::novo(&cat, &cad, &[(cargo, Abrangencia::Uf(uf))], &HashSet::new(), Gabaritos::embutidos(), None, Parametros::default());
        let ci = cat.ci(cargo).unwrap();
        let t0 = urna_velox::metricas::segundos("04/10/2026 17:10:00").unwrap();
        let passo = 250i64;
        let mut ultima = None;
        for i in 1..=200u32 {
            // seção i no município (i % 4): 1.000 aptos, 360/320/120 votos
            let m = (i % 4) as usize;
            let k = m * cat.nc() + ci;
            est.aptos[k].fetch_add(1000, Relaxed);
            est.secoes[m].fetch_add(1, Relaxed);
            let base = cat.base[k] as usize;
            for (j, v) in [360u32, 320, 120].into_iter().enumerate() { est.tally[base + j].fetch_add(v, Relaxed); }
            est.bus.fetch_add(1, Relaxed);
            let agora = t0 + i as i64 * passo;
            prev.registrar(&est, agora);
            if i % 20 == 0 { prev.prever(&cad, &est, agora, None); ultima = mensagem(&prev, 0, None); }
        }
        let m = ultima.expect("nenhuma previsão");
        let agora = t0 + 200 * passo;
        eprintln!("previsao_sempre_com_horario: mat {} · fim99 {} · fim100 {}", m["mat"], m["fim99"], m["fim100"]);
        let seg = |v: &Value| v.as_str().and_then(crate::relogio::ms_iso).map(|ms| ms / 1000);
        for nome in ["mat", "fim99", "fim100"] {
            let f = &m[nome];
            assert!(f.is_object(), "{nome}: sem previsão");
            // horário que a página mostra: o P50 quando a faixa é estreita; o do ritmo atual (`eta`) quando é larga
            let t = if f["incerta"] == true { seg(&f["eta"]) } else { seg(&f["p50"]) };
            assert!(t.is_some(), "{nome}: \"em horário ainda incerto\" sem horário nenhum: {f}");
        }
        // o horário pelo ritmo atual: as 200 seções que faltam a 250 s cada (±10 %)
        let fim = seg(&m["fim100"]["eta"]).expect("fim100 sem eta") - agora;
        assert!((45_000..=55_000).contains(&fim), "fim100 pelo ritmo atual daqui a {fim} s (esperado ~50.000)");
        let f99 = seg(&m["fim99"]["eta"]).expect("fim99 sem eta") - agora;
        assert!(f99 < fim && f99 > 40_000, "fim99 pelo ritmo atual daqui a {f99} s");
    }
}
