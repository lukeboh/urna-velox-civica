//! `uv-motor previsao`: backtest da previsão (etapa 8, `docs/HANDOFF_0.8.0.md`) numa noite reproduzida em ordem de
//! chegada dos BUs. A cada `intervalo_s` do relógio da noite, a previsão roda só com o que se sabia naquele instante
//! (e só para as abrangências que mudaram, como no `uv-servico`); no fim, compara com o que aconteceu: calibração da
//! faixa P10–P90, erro do P50 por faixa de apuração e custo da rodada. Grava também os gabaritos da noite.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write as _;
use std::path::PathBuf;
use std::time::Instant;
use urna_velox::acompanhamento::Acompanhamento;
use urna_velox::candidatos::Candidatos;
use urna_velox::catalogo::Catalogo;
use urna_velox::estado::{Agregador, Estado};
use urna_velox::metricas::Metricas;
use urna_velox::noite::ordem_de_chegada;
use urna_velox::oficial::{Abrangencia, Oficial};
use urna_velox::pacote::EmMemoria;
use urna_velox::previsao::{classe, grade, mediana_curvas, percentil, tau_da_serie, Curva, Gabaritos, Parametros, Previsao, Previsor};
use std::sync::atomic::Ordering::Relaxed;

pub struct Entrada {
    pub dados: PathBuf,
    pub candidatos: Option<PathBuf>,
    pub oficial: Option<PathBuf>,
    pub chegadas: String,
    pub anuladas: Option<String>,
    pub vagas: Vec<(u8, u8)>,
}

pub struct Opcoes {
    pub nome: String,
    pub gabaritos: String,
    pub gerar: Option<PathBuf>,
    pub classes: String,
    pub intervalo_s: i64,
    pub cargos: Option<Vec<u8>>,
    pub par: Parametros,
    pub threads: usize,
    pub sem_previsao: bool,
    pub todos_topicos: bool,
    pub saida: PathBuf,
}

/// Abrangência usada para gerar os gabaritos: (município, região de algum cargo) e a série (t, aptos, seções).
struct UnG {
    classe: &'static str,
    abr: Abrangencia,
    munis: Vec<(usize, usize)>,
    pts: Vec<(i64, u64, u64)>,
}

struct Linha {
    unidade: usize,
    p: Previsao,
}

fn hm(t: i64) -> String { let (_, _, _, h, mi, s) = urna_velox::metricas::civil(t); format!("{h:02}:{mi:02}:{s:02}") }

pub fn rodar(e: Entrada, o: Opcoes) -> anyhow::Result<()> {
    std::fs::create_dir_all(&o.saida)?;
    let t0 = Instant::now();
    let mem = EmMemoria::carregar(&e.dados, None)?;
    let (cad_prep, arq_prep) = match &e.candidatos { Some(p) => { let (c, a) = Candidatos::ler(p)?; (Some(c), Some(a)) } None => (None, None) };
    let mut cat = match &arq_prep { Some(arq) => Catalogo::de_arquivo_candidatos(arq)?, None => Catalogo::dos_pacotes(&mem, &e.vagas).0 };
    if let Some(a) = &e.anuladas { cat.usar_anuladas(&Metricas::ler_lista(a)?); }
    let ofs = match &e.oficial { Some(p) => Oficial::ler_pasta(p)?, None => vec![] };
    let cad = match cad_prep { Some(c) => c, None if e.oficial.is_some() => Candidatos::dos_oficiais(&ofs), None => anyhow::bail!("informe --candidatos ou --oficial") };
    let met = Metricas::ler_lista(&e.chegadas)?;
    let mut excl: HashSet<u64> = cad.agregadas.clone();
    excl.extend(met.nao_instaladas.iter().map(|(_, k)| *k));
    if let Some(a) = &e.anuladas { excl.extend(Metricas::ler_lista(a)?.nao_instaladas.iter().map(|(_, k)| *k)); }
    let (ev, sem_horario) = ordem_de_chegada(&mem, &met.recebido);
    anyhow::ensure!(!ev.is_empty(), "nenhum BU");
    eprintln!("{}: {} BUs ({sem_horario} sem horário), {} circunscrições, {} seções fora (agregadas/não instaladas); carga em {:.1} s",
        o.nome, ev.len(), cad.n_circunscricoes(), excl.len(), t0.elapsed().as_secs_f64());

    let est = Estado::novo(&cat);
    let mut ac = Acompanhamento::novo(&cat, &cad, o.cargos.as_deref());
    let mut topicos: Vec<(u8, Abrangencia)> = ac.circs.iter().map(|c| (c.cargo, c.abr().clone())).collect();
    if o.todos_topicos {
        // como no serviço: também as abrangências menores (UFs e municípios) de cada cargo, só com o fim da apuração
        let cargos: std::collections::BTreeSet<u8> = topicos.iter().map(|x| x.0).collect();
        let nc = cat.nc();
        for &c in &cargos {
            let Some(ci) = cat.ci(c) else { continue };
            for u in &cat.ufs { topicos.push((c, Abrangencia::Uf(*u))); }
            for m in 0..cat.muni_cod.len() { if cat.base[m * nc + ci] != u32::MAX { topicos.push((c, Abrangencia::Municipio(cat.ufs[cat.muni_uf[m] as usize], cat.muni_cod[m]))); } }
        }
        topicos.sort_by(|a, b| (a.0, a.1.codigo()).cmp(&(b.0, b.1.codigo())));
        topicos.dedup();
    }
    let gab = if o.gabaritos.is_empty() { Gabaritos::embutidos() } else {
        Gabaritos::juntar(o.gabaritos.split(',').filter(|s| !s.is_empty()).map(|p| Gabaritos::ler(std::path::Path::new(p))).collect::<anyhow::Result<Vec<_>>>()?)
    };
    let n_curvas = gab.curvas.iter().filter(|c| c.noite != o.nome).count();
    let mut prev = Previsor::novo(&cat, &cad, &topicos, &excl, gab, Some(&o.nome), o.par.clone());
    prev.threads = o.threads.max(1);
    eprintln!("{} unidades de previsão, {n_curvas} curvas de gabarito de outras noites", prev.unidades.len());

    // unidades dos gabaritos: Brasil, UFs e municípios (região de qualquer cargo, para os aptos)
    let nc = cat.nc();
    let regiao = |m: usize| (0..nc).map(|ci| m * nc + ci).find(|&k| cat.base[k] != u32::MAX);
    let mut eleit_muni: HashMap<u32, u64> = HashMap::new();
    for ((_, abr), &x) in &cad.eleitorado_abr { if let Abrangencia::Municipio(_, mu) = abr { let y = eleit_muni.entry(*mu).or_default(); *y = (*y).max(x); } }
    let classes: Vec<&str> = o.classes.split(',').map(|s| s.trim()).collect();
    let mut ung: Vec<UnG> = vec![];
    if o.gerar.is_some() {
        let todos: Vec<(usize, usize)> = (0..cat.muni_cod.len()).filter_map(|m| regiao(m).map(|k| (m, k))).collect();
        if classes.contains(&"br") { ung.push(UnG { classe: "br", abr: Abrangencia::Brasil, munis: todos.clone(), pts: vec![] }); }
        if classes.contains(&"uf") {
            for (ui, u) in cat.ufs.iter().enumerate() {
                let ms: Vec<(usize, usize)> = todos.iter().copied().filter(|&(m, _)| cat.muni_uf[m] as usize == ui).collect();
                if !ms.is_empty() { ung.push(UnG { classe: "uf", abr: Abrangencia::Uf(*u), munis: ms, pts: vec![] }); }
            }
        }
        if classes.contains(&"mun") {
            for &(m, k) in &todos { ung.push(UnG { classe: "", abr: Abrangencia::Municipio(cat.ufs[cat.muni_uf[m] as usize], cat.muni_cod[m]), munis: vec![(m, k)], pts: vec![] }); }
        }
    }

    let mut linhas: Vec<Linha> = vec![];
    let mut custos: Vec<(f64, usize)> = vec![];
    let mut rodada_completa: Option<(f64, usize, i64)> = None;
    let mut ultimo_estado: Vec<(u64, u64, bool)> = vec![(0, 0, false); prev.unidades.len()];
    let secoes_tot: u64 = prev.unidades.iter().map(|u| u.secoes).max().unwrap_or(1);
    let iv = o.intervalo_s.max(1);
    let mut tick = (ev[0].0 / iv + 1) * iv;
    let t_rep = Instant::now();
    let mut ag = Agregador::novo(&cat, &est);
    let mut k = 0;
    let fim_noite = ev.last().unwrap().0;
    let mut frac_real: Vec<Option<f64>> = vec![None; prev.unidades.len()];
    let mut rodada = |tick: i64, prev: &mut Previsor, ac: &Acompanhamento, est: &Estado, ung: &mut Vec<UnG>, linhas: &mut Vec<Linha>| {
        prev.registrar(est, tick);
        prev.definir_pelo(ac);
        for (i, u) in prev.unidades.iter().enumerate() {
            if let (None, Some(ca)) = (frac_real[i], ac.circ_de(u.cargo, &u.abr)) {
                if ca.mat_circ != urna_velox::matematica::MatCirc::Apurando { frac_real[i] = Some(ca.apurado as f64 / ca.circ.eleitorado.max(1) as f64); }
            }
        }
        for g in ung.iter_mut() {
            let a: u64 = g.munis.iter().map(|&(_, k)| est.aptos[k].load(Relaxed) as u64).sum();
            let s: u64 = g.munis.iter().map(|&(m, _)| est.secoes[m].load(Relaxed) as u64).sum();
            if g.pts.last().is_none_or(|x| x.1 != a || x.2 != s) { g.pts.push((tick, a, s)); }
        }
        if o.sem_previsao { return; }
        // só as unidades que mudaram (ou que acabaram de se definir), como no serviço
        let quais: Vec<usize> = prev.unidades.iter().enumerate().filter_map(|(i, u)| {
            let p = u.serie.ultimo()?;
            let def = u.definida.is_some();
            let antes = ultimo_estado[i];
            ultimo_estado[i] = (p.aptos, p.secoes, def);
            (p.aptos != antes.0 || p.secoes != antes.1 || def != antes.2).then_some(i)
        }).collect();
        if quais.is_empty() && !prev.rodada_pendente() { return; }
        let t = Instant::now();
        let feitos = prev.prever(&cad, est, tick, Some(&quais));
        custos.push((t.elapsed().as_secs_f64() * 1e3, quais.len()));
        for i in feitos {
            if let Some(p) = &prev.unidades[i].ultima { if p.situacao != "insuficiente" { linhas.push(Linha { unidade: i, p: p.clone() }); } }
        }
        // custo de uma rodada com todas as unidades, uma vez, quando a maior abrangência passa da metade das seções
        if rodada_completa.is_none() && prev.unidades.iter().map(|u| u.serie.ultimo().map_or(0, |p| p.secoes)).max().unwrap_or(0) * 2 >= secoes_tot {
            let todas: Vec<usize> = (0..prev.unidades.len()).collect();
            let t = Instant::now();
            prev.prever(&cad, est, tick, Some(&todas));
            rodada_completa = Some((t.elapsed().as_secs_f64() * 1e3, todas.len(), tick));
        }
    };
    while k < ev.len() {
        let t = ev[k].0;
        // salto longo sem BU: as rodadas do intervalo não mudariam nada
        if tick + 3600 < t { rodada(tick, &mut prev, &ac, &est, &mut ung, &mut linhas); tick = (t / iv) * iv; }
        while tick <= t { rodada(tick, &mut prev, &ac, &est, &mut ung, &mut linhas); tick += iv; }
        while k < ev.len() && ev[k].0 == t {
            let (r, _, _, _) = &mem.itens[ev[k].1];
            est.aplicar(&cat, r.chave(), mem.bu(ev[k].1), &mut ag);
            if let Some(m) = cat.muni(r.municipio) { ac.bu_aplicado(m as usize); }
            k += 1;
        }
        ac.atualizar(&est, k as u64, t);
    }
    for _ in 0..3 { rodada(tick, &mut prev, &ac, &est, &mut ung, &mut linhas); tick += iv; }
    drop(rodada);
    eprintln!("replay em {:.1} s ({} rodadas com previsão, {} linhas)", t_rep.elapsed().as_secs_f64(), custos.len(), linhas.len());

    // ---- gabaritos da noite
    if let Some(arq) = &o.gerar {
        let g = grade();
        let inicio = ev[0].0;
        let mut por_classe: BTreeMap<&str, (Vec<Vec<f64>>, Vec<Vec<f64>>)> = BTreeMap::new();
        for u in &ung {
            let Some(&(_, af, sf)) = u.pts.last() else { continue };
            if sf < 3 || af == 0 { continue; }
            let cl = if u.classe.is_empty() { classe(&u.abr, eleit_muni.get(&match u.abr { Abrangencia::Municipio(_, m) => m, _ => 0 }).copied().unwrap_or(af)) } else { u.classe };
            let pa: Vec<(i64, f64)> = std::iter::once((inicio, 0.0)).chain(u.pts.iter().map(|p| (p.0, p.1 as f64 / af as f64))).collect();
            let ps: Vec<(i64, f64)> = std::iter::once((inicio, 0.0)).chain(u.pts.iter().map(|p| (p.0, p.2 as f64 / sf as f64))).collect();
            let (Some(ca), Some(cs)) = (tau_da_serie(&pa, inicio, &g), tau_da_serie(&ps, inicio, &g)) else { continue };
            let x = por_classe.entry(cl).or_default();
            x.0.push(ca);
            x.1.push(cs);
        }
        let curvas: Vec<Curva> = por_classe.into_iter().map(|(cl, (a, s))| Curva { noite: o.nome.clone(), classe: cl.to_string(), unidades: a.len() as u32,
            aptos: mediana_curvas(&a).iter().map(|x| x.round()).collect(), secoes: mediana_curvas(&s).iter().map(|x| x.round()).collect() }).collect();
        for c in &curvas { eprintln!("gabarito {} {}: {} abrangências; 50 % em {:.0} min, 99 % em {:.0} min, 100 % em {:.0} min (seções)", c.noite, c.classe, c.unidades,
            c.secoes[50] / 60.0, c.secoes[g.iter().position(|&x| (x - 0.99).abs() < 1e-9).unwrap()] / 60.0, c.secoes[g.len() - 1] / 60.0); }
        std::fs::write(arq, serde_json::to_string(&Gabaritos { grade: g, curvas })?)?;
    }
    if o.sem_previsao { return Ok(()); }

    // ---- avaliação
    let reais: Vec<(Option<i64>, Option<i64>, Option<i64>)> = prev.unidades.iter().map(|u| (u.definida, u.chegou[0], u.ultimo_aumento)).collect();
    let mut csv = String::from("noite,instante,hora,abrangencia,cargo,classe,apurado,secoes,aptos_min,situacao,mat_tipo,mat_p10,mat_p50,mat_p90,prob_antes_fim,fim99_p10,fim99_p50,fim99_p90,fim100_p10,fim100_p50,fim100_p90,real_mat,real_fim99,real_fim100,mat_apurado_p50,real_apurado_mat\n");
    let op = |x: Option<i64>| x.map(|v| v.to_string()).unwrap_or_default();
    for l in &linhas {
        let u = &prev.unidades[l.unidade];
        let p = &l.p;
        let (m, f9, f1) = (p.mat.as_ref(), p.fim99.as_ref(), p.fim100.as_ref());
        let r = reais[l.unidade];
        let _ = writeln!(csv, "{},{},{},{},{},{},{:.5},{:.5},{:.0},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}", o.nome, p.instante, hm(p.instante), u.abr.codigo(), u.cargo, u.classe,
            p.apurado, p.secoes, p.aptos_min, p.situacao, m.map_or("", |x| x.tipo), op(m.map(|x| x.faixa.p10)), op(m.map(|x| x.faixa.p50)), op(m.map(|x| x.faixa.p90)),
            m.map(|x| format!("{:.3}", x.prob_antes_fim)).unwrap_or_default(), op(f9.map(|x| x.p10)), op(f9.map(|x| x.p50)), op(f9.map(|x| x.p90)),
            op(f1.map(|x| x.p10)), op(f1.map(|x| x.p50)), op(f1.map(|x| x.p90)), op(r.0), op(r.1), op(r.2),
            m.map(|x| format!("{:.4}", x.apurado_p50)).unwrap_or_default(), frac_real[l.unidade].map(|x| format!("{x:.4}")).unwrap_or_default());
    }
    std::fs::write(o.saida.join("previsoes.csv"), csv)?;

    // calibração e erro por cargo e alvo
    let alvos = ["mat", "fim99", "fim100"];
    let mut res = serde_json::Map::new();
    let mut md = format!("### {}\n\n| cargo | alvo | circ. | previsões (≥ 20 %) | dentro P10–P90 | média por circ. | erro P50 mediano 20 % / 50 % / 80 % (min) | n |\n|---|---|---|---|---|---|---|---|\n", o.nome);
    let cargos: Vec<u8> = { let mut v: Vec<u8> = prev.unidades.iter().map(|u| u.cargo).collect(); v.sort(); v.dedup(); v };
    for &cargo in &cargos {
        for (ai, alvo) in alvos.iter().enumerate() {
            let faixa = |p: &Previsao| match ai { 0 => p.mat.as_ref().map(|m| m.faixa), 1 => p.fim99, _ => p.fim100 };
            let real = |i: usize| match ai { 0 => reais[i].0, 1 => reais[i].1, _ => reais[i].2 };
            let (mut n, mut dentro) = (0usize, 0usize);
            let mut por_circ: HashMap<usize, (usize, usize)> = HashMap::new();
            let mut erros: [Vec<f64>; 3] = [vec![], vec![], vec![]];
            let mut vistos: [HashSet<usize>; 3] = [HashSet::new(), HashSet::new(), HashSet::new()];
            for l in linhas.iter().filter(|l| prev.unidades[l.unidade].cargo == cargo) {
                let (Some(f), Some(r)) = (faixa(&l.p), real(l.unidade)) else { continue };
                if l.p.instante >= r { continue; }
                for (j, lim) in [0.2, 0.5, 0.8].iter().enumerate() {
                    if l.p.apurado >= *lim && vistos[j].insert(l.unidade) { erros[j].push((f.p50 - r).abs() as f64 / 60.0); }
                }
                if l.p.apurado < 0.2 { continue; }
                n += 1;
                let d = f.contem(r) as usize;
                dentro += d;
                let x = por_circ.entry(l.unidade).or_default();
                x.0 += 1;
                x.1 += d;
            }
            if n == 0 { continue; }
            let media_circ = por_circ.values().map(|(a, b)| *b as f64 / *a as f64).sum::<f64>() / por_circ.len() as f64;
            let med: Vec<String> = erros.iter_mut().map(|v| if v.is_empty() { "—".into() } else { format!("{:.1}", percentil(v, 0.5)) }).collect();
            let ns: Vec<usize> = erros.iter().map(|v| v.len()).collect();
            let _ = writeln!(md, "| {} | {} | {} | {} | {:.1} % | {:.1} % | {} | {} |", urna_velox::catalogo::nome_cargo(cargo), alvo, por_circ.len(), n,
                100.0 * dentro as f64 / n as f64, 100.0 * media_circ, med.join(" / "), ns.iter().map(|x| x.to_string()).collect::<Vec<_>>().join("/"));
            res.insert(format!("c{cargo}-{alvo}"), serde_json::json!({ "circunscricoes": por_circ.len(), "previsoes": n, "dentro": dentro as f64 / n as f64,
                "dentro_media_circ": media_circ, "erro_p50_min": { "20": med[0], "50": med[1], "80": med[2] }, "n_erro": ns }));
        }
    }
    let mut cs: Vec<f64> = custos.iter().map(|c| c.0).collect();
    let custo = serde_json::json!({ "rodadas": cs.len(), "ms_p50": percentil(&mut cs, 0.5), "ms_p95": percentil(&mut cs, 0.95), "ms_max": percentil(&mut cs, 1.0),
        "max_unidades": custos.iter().map(|c| c.1).max().unwrap_or(0), "threads": o.threads,
        "rodada_completa": rodada_completa.map(|(ms, n, t)| serde_json::json!({ "ms": ms, "unidades": n, "instante": hm(t) })) });
    let _ = writeln!(md, "\nCusto: {} rodadas, p50 {:.1} ms, p95 {:.1} ms, máx. {:.1} ms; rodada completa: {}\n", cs.len(), custo["ms_p50"].as_f64().unwrap_or(0.0),
        custo["ms_p95"].as_f64().unwrap_or(0.0), custo["ms_max"].as_f64().unwrap_or(0.0),
        rodada_completa.map(|(ms, n, t)| format!("{n} unidades em {ms:.0} ms às {}", hm(t))).unwrap_or("—".into()));
    let mut par_txt: Vec<String> = prev.paradas.iter().map(|(a, b)| format!("{}–{}", hm(*a), hm(*b))).collect();
    if let Some(p) = prev.parada { par_txt.push(format!("{}–(fim)", hm(p.desde))); }
    let _ = writeln!(md, "Paradas da chegada dos BUs detectadas (previsão suspensa e adiada): {}\n", if par_txt.is_empty() { "nenhuma".into() } else { par_txt.join(", ") });
    res.insert("paradas".into(), serde_json::json!(par_txt));
    // a noite das circunscrições que mais interessam: quando saiu, e a primeira previsão que acertou para valer
    let _ = writeln!(md, "| abrangência | cargo | saiu às | última faixa | desde |\n|---|---|---|---|---|");
    let mut destaque: Vec<usize> = (0..prev.unidades.len()).filter(|&i| prev.unidades[i].definida.is_some() && !prev.unidades[i].historico.is_empty()).collect();
    destaque.sort_by_key(|&i| std::cmp::Reverse(prev.unidades[i].eleitorado));
    for &i in destaque.iter().take(12) {
        let u = &prev.unidades[i];
        if let Some(c) = prev.confirmacao(i) {
            let _ = writeln!(md, "| {} | {} | {} | {}–{}{} | {} |", u.abr.codigo(), u.cargo, hm(c.saiu), hm(c.faixa.p10), hm(c.faixa.p90), if c.dentro { "" } else { " (fora)" },
                c.desde.map(hm).unwrap_or("—".into()));
        }
    }
    res.insert("custo".into(), custo);
    res.insert("fim_noite".into(), hm(fim_noite).into());
    std::fs::write(o.saida.join("resumo.json"), serde_json::to_string_pretty(&serde_json::Value::Object(res))?)?;
    std::fs::write(o.saida.join("resumo.md"), &md)?;
    print!("{md}");
    Ok(())
}
