//! Replay da noite da eleição: injeta cada BU no horário em que o TSE o recebeu (campo dr/hr do
//! aux.json, guardado nas métricas da coleta), com aceleração opcional, totaliza em fluxo e publica
//! continuamente os resultados no formato do motor (`docs/arquitetura/FORMATO_RESULTADO.md`): Brasil (Presidente), UF
//! (cargos gerais) e, nos cargos municipais (Prefeito, consultas), município — só as abrangências que
//! receberam BU desde a publicação anterior —, com o "matematicamente eleito / 2º turno" recalculado a cada
//! ciclo quando há candidatos (`--oficial`). No fim, uma publicação encerrada de tudo, com a situação final.
//! Mede, para cada BU, o tempo entre a chegada e a primeira publicação que o inclui.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use urna_velox::acompanhamento::Acompanhamento;
use urna_velox::candidatos::Candidatos;
use urna_velox::catalogo::Catalogo;
use urna_velox::estado::{Agregador, Estado};
use urna_velox::metricas::Metricas;
use urna_velox::oficial::Abrangencia;
use urna_velox::pacote::EmMemoria;
use urna_velox::resultado::{gravar_atomico, Momento, Publicador};

pub use urna_velox::metricas::{hhmm, segundos};

/// Lê os CSVs de métricas da coleta (`t_s,uf,municipio,zona,secao,...,recebido_tse,...`) → (uf, chave) → segundos.
pub fn ler_chegadas(arqs: &[PathBuf]) -> anyhow::Result<HashMap<([u8; 2], u64), i64>> {
    let ps: Vec<&std::path::Path> = arqs.iter().map(|p| p.as_path()).collect();
    Ok(Metricas::ler(&ps)?.recebido)
}

/// `corte`: horário (segundos, ver `segundos`) a partir do qual as chegadas são antecipadas para o próprio corte.
/// No 1º turno de 2026, 2.922 seções têm no aux.json um BU reenviado dias depois (situação "Recebido");
/// sem corte, o replay esperaria até 07/10.
/// `cargos`: cargos publicados a cada ciclo (ex.: 1 Presidente, 3 Governador, 5 Senador).
/// `candidatos`: destinação, nascimento e eleitorado (arquivos oficiais); sem eles, a publicação não tem situação
/// nem "matematicamente". `metricas`: seções não instaladas (contadas como totalizadas).
pub struct Config {
    pub acelerar: f64,
    pub saida: PathBuf,
    pub gravar: bool,
    pub corte: Option<i64>,
    pub cargos: Vec<u8>,
    pub candidatos: Option<Candidatos>,
    pub metricas: Option<Metricas>,
}

pub fn executar(cat: Arc<Catalogo>, mem: &EmMemoria, chegadas: &HashMap<([u8; 2], u64), i64>, cfg: &Config) -> anyhow::Result<String> {
    // eventos (horário de recebimento no TSE, índice do BU), em ordem de chegada
    let mut ev: Vec<(i64, usize)> = vec![];
    let mut sem_horario = 0usize;
    let mut cortados = 0usize;
    for (i, (r, _, _, _)) in mem.itens.iter().enumerate() {
        match chegadas.get(&(r.uf, r.chave())) {
            Some(&t) => {
                let t = match cfg.corte { Some(c) if t > c => { cortados += 1; c } _ => t };
                ev.push((t, i))
            }
            None => sem_horario += 1,
        }
    }
    ev.sort();
    anyhow::ensure!(!ev.is_empty(), "nenhum BU com horário de recebimento");
    let n = ev.len();
    let t0 = ev[0].0;
    let s = cfg.acelerar;
    let previsto = |t: i64| if s > 0.0 { Duration::from_secs_f64((t - t0) as f64 / s) } else { Duration::ZERO };
    let pub_dir = cfg.saida.join("publicacao");
    std::fs::create_dir_all(&pub_dir)?;

    let est = Estado::novo(&cat);
    let aplicados = AtomicUsize::new(0);
    let ultimo_t = AtomicI64::new(t0);
    let fim = AtomicBool::new(false);
    let sujo: Vec<AtomicBool> = (0..cat.muni_cod.len()).map(|_| AtomicBool::new(false)).collect();
    let w0 = Instant::now();
    let cargos: Vec<u8> = cfg.cargos.iter().copied().filter(|&c| cat.ci(c).is_some()).collect();
    let vazio = Candidatos::default();
    let cad = cfg.candidatos.as_ref().unwrap_or(&vazio);

    let (marcas, mut ciclos, final_ms, n_pub, eventos) = std::thread::scope(|sc| -> anyhow::Result<_> {
        // publicador: sempre que há BU novo aplicado, reavalia as circunscrições e grava as abrangências tocadas
        let h = sc.spawn(|| -> anyhow::Result<(Vec<(usize, Duration)>, Vec<f64>, f64, usize, usize)> {
            let publicador = Publicador::novo(&cat, cfg.candidatos.as_ref(), cfg.metricas.as_ref());
            let por_cargo: Vec<(u8, Vec<Abrangencia>)> = cargos.iter().map(|&c| (c, publicador.abrangencias(c, false))).collect();
            let mut ac = Acompanhamento::novo(&cat, cad, Some(&cargos));
            let mut marcas: Vec<(usize, Duration)> = vec![]; // (BUs incluídos, instante da publicação)
            let mut ciclos = vec![];
            let mut ultimo = 0usize;
            let mut n_pub = 0usize;
            let mut munis_sujos = vec![false; cat.muni_cod.len()];
            loop {
                let terminou = fim.load(Ordering::Acquire);
                let w = aplicados.load(Ordering::Acquire);
                if w == ultimo {
                    if terminou { break; }
                    std::thread::sleep(Duration::from_micros(200));
                    continue;
                }
                let tc = Instant::now();
                let t = ultimo_t.load(Ordering::Acquire);
                let mut ufs_sujas = vec![false; cat.ufs.len()];
                for m in 0..cat.muni_cod.len() {
                    if sujo[m].swap(false, Ordering::AcqRel) { munis_sujos[m] = true; ufs_sujas[cat.muni_uf[m] as usize] = true; ac.bu_aplicado(m); }
                }
                ac.atualizar(&est, w as u64, t);
                if cfg.gravar {
                    let mo = Momento { encerrada: false, seq: w as u64, ultimo_bu: Some(t), totalizado_ms: None };
                    for (cargo, abrs) in &por_cargo {
                        for abr in abrs {
                            let tocada = match abr {
                                Abrangencia::Brasil => true,
                                Abrangencia::Uf(u) => cat.ufs.iter().position(|x| x == u).is_some_and(|i| ufs_sujas[i]),
                                Abrangencia::Municipio(_, mu) => cat.muni(*mu).is_some_and(|m| munis_sujos[m as usize]),
                            };
                            if !tocada { continue; }
                            if let Some(j) = publicador.json(&est, Some(&ac), abr, *cargo, mo) { gravar_atomico(&pub_dir, &publicador.nome_arquivo(abr, *cargo), &j)?; n_pub += 1; }
                        }
                    }
                }
                munis_sujos.iter_mut().for_each(|x| *x = false);
                ciclos.push(tc.elapsed().as_secs_f64() * 1e3);
                marcas.push((w, w0.elapsed()));
                ultimo = w;
            }
            // publicação encerrada: tudo, com a situação final
            let tf = Instant::now();
            if cfg.gravar {
                let mo = Momento { encerrada: true, seq: ultimo as u64, ultimo_bu: Some(ultimo_t.load(Ordering::Acquire)), totalizado_ms: None };
                n_pub += publicador.gravar(&est, Some(&ac), mo, &cargos, false, &pub_dir)?;
            }
            // eventos do "matematicamente" (sequência, horário, abrangência, cargo, candidato, estado)
            let mut csv = String::from("seq,horario,circunscricao,cargo,candidato,estado\n");
            for e in &ac.eventos {
                let c = &ac.circs[e.circ];
                let _ = writeln!(csv, "{},{},{},{},{},{}", e.seq, urna_velox::metricas::iso_brasilia(e.em), c.abr().codigo(), c.cargo, e.numero.map(|x| x.to_string()).unwrap_or_default(), e.estado);
            }
            std::fs::write(cfg.saida.join("replay_eventos.csv"), csv)?;
            anyhow::ensure!(ac.revogacoes() == 0, "{} revogações de \"matematicamente\"", ac.revogacoes());
            Ok((marcas, ciclos, tf.elapsed().as_secs_f64() * 1e3, n_pub, ac.eventos.len()))
        });

        // injetor: aplica cada BU no seu horário (acelerado); aplicação numa thread só (~2–3 µs por BU)
        let mut ag = Agregador::novo(&cat, &est);
        let mut atraso_injecao_max = Duration::ZERO;
        for (k, &(t, i)) in ev.iter().enumerate() {
            let alvo = previsto(t);
            let agora = w0.elapsed();
            if alvo > agora { std::thread::sleep(alvo - agora); } else { atraso_injecao_max = atraso_injecao_max.max(agora - alvo); }
            let (r, _, _, _) = &mem.itens[i];
            est.aplicar(&cat, r.chave(), mem.bu(i), &mut ag);
            if let Some(m) = cat.muni(r.municipio) { sujo[m as usize].store(true, Ordering::Release); }
            ultimo_t.store(t, Ordering::Release);
            aplicados.store(k + 1, Ordering::Release);
        }
        let fim_injecao = w0.elapsed();
        fim.store(true, Ordering::Release);
        let (marcas, ciclos, final_ms, n_pub, eventos) = h.join().map_err(|_| anyhow::anyhow!("publicador falhou"))??;
        Ok((marcas, ciclos, final_ms, n_pub, (eventos, fim_injecao, atraso_injecao_max)))
    })?;
    let (n_eventos, fim_injecao, atraso_injecao_max) = eventos;

    // latência por BU = primeira publicação que o inclui − horário (acelerado) de chegada
    let mut lat = Vec::with_capacity(n);
    let mut pubs_por_min: HashMap<i64, u64> = HashMap::new();
    let mut j = 0;
    for (k, &(t, _)) in ev.iter().enumerate() {
        while marcas[j].0 <= k { j += 1; }
        let tp = marcas[j].1;
        lat.push((tp.saturating_sub(previsto(t))).as_secs_f64() * 1e3);
        // minuto (no relógio da noite) em que o BU apareceu publicado
        let t_noite = t0 + if s > 0.0 { (tp.as_secs_f64() * s) as i64 } else { t - t0 };
        *pubs_por_min.entry(t_noite.div_euclid(60)).or_default() += 1;
    }
    let mut rec_por_min: HashMap<i64, u64> = HashMap::new();
    for &(t, _) in &ev { *rec_por_min.entry(t.div_euclid(60)).or_default() += 1; }

    let mut ls = lat.clone();
    ls.sort_by(|a, b| a.partial_cmp(b).unwrap());
    ciclos.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let p = |v: &[f64], q: f64| crate::coletor::percentil(v, q);
    let media = ls.iter().sum::<f64>() / ls.len() as f64;
    let pico = rec_por_min.values().max().copied().unwrap_or(0);

    // série por minuto
    let (mi, mf) = (*rec_por_min.keys().min().unwrap(), *rec_por_min.keys().chain(pubs_por_min.keys()).max().unwrap());
    let mut csv = String::from("minuto;recebimento;divulgacao\n");
    for m in mi..=mf {
        let _ = writeln!(csv, "{};{};{}", hhmm(m * 60), rec_por_min.get(&m).unwrap_or(&0), pubs_por_min.get(&m).unwrap_or(&0));
    }
    std::fs::write(cfg.saida.join("replay_minuto.csv"), csv)?;
    let mut lcsv = String::from("latencia_ms\n");
    for x in &lat { let _ = writeln!(lcsv, "{x:.3}"); }
    std::fs::write(cfg.saida.join("replay_latencias.csv"), lcsv)?;
    let uf = est.consolidar_uf(&cat);
    std::fs::write(cfg.saida.join("totais_uf.csv"), urna_velox::estado::csv_totais_uf(&cat, &uf))?;
    if cargos.iter().any(|&c| cat.municipal[c as usize]) {
        std::fs::write(cfg.saida.join("totais_municipio.csv"), urna_velox::estado::csv_totais_municipio(&cat, &est, None))?;
    }

    let resumo = format!(concat!("{{\"bus\":{},\"sem_horario\":{},\"apos_corte\":{},\"acelerar\":{},\"primeiro\":\"{}\",\"ultimo\":\"{}\",\"pico_bus_min\":{},",
        "\"duracao_s\":{:.3},\"atraso_injecao_max_ms\":{:.3},\"publicacoes\":{},\"arquivos\":{},\"eventos_matematicamente\":{},\"publicacao_final_ms\":{:.1},",
        "\"ciclo_pub_ms\":{{\"p50\":{:.3},\"p99\":{:.3},\"max\":{:.3}}},",
        "\"latencia_ms\":{{\"media\":{:.3},\"p50\":{:.3},\"p90\":{:.3},\"p95\":{:.3},\"p99\":{:.3},\"max\":{:.3}}},",
        "\"conferencia\":{{\"aplicados\":{},\"duplicados\":{},\"desconhecidos\":{},\"cargos_invalidos\":{},\"erros\":{}}}}}\n"),
        n, sem_horario, cortados, s, hhmm(t0), hhmm(ev[n - 1].0), pico, fim_injecao.as_secs_f64(), atraso_injecao_max.as_secs_f64() * 1e3, marcas.len(), n_pub, n_eventos, final_ms,
        p(&ciclos, 50.0), p(&ciclos, 99.0), ciclos.last().copied().unwrap_or(0.0),
        media, p(&ls, 50.0), p(&ls, 90.0), p(&ls, 95.0), p(&ls, 99.0), ls.last().copied().unwrap_or(0.0),
        est.bus.load(Ordering::Relaxed), est.duplicados.load(Ordering::Relaxed), est.desconhecidos.load(Ordering::Relaxed),
        est.cargos_invalidos.load(Ordering::Relaxed), est.erros.load(Ordering::Relaxed));
    std::fs::write(cfg.saida.join("replay_resumo.json"), &resumo)?;
    Ok(resumo)
}
