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
