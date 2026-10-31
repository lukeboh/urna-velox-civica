use super::*;
/// A fila de um ciclo segue o recebimento no TSE (das métricas) e mistura as UFs; sem métrica, a disponibilização
/// do `cs.json`; empate pela UF e pela seção.
#[test]
fn fila_na_ordem_de_recebimento() {
    let sec = |uf: &[u8; 2], se: u16, disp: &str| Secao { uf: *uf, municipio: 1, zona: 1, secao: se, disponivel: disp.into() };
    let t = |s: &str| urna_velox::metricas::segundos(s).unwrap();
    let (ac, sp, df) = (sec(b"ac", 1, "04/10/2026 17:30:00"), sec(b"sp", 2, "04/10/2026 17:20:00"), sec(b"df", 3, "04/10/2026 17:25:00"));
    // recebimento: SP 17:05, AC 17:10; o DF não está nas métricas e vai pela disponibilização (17:25)
    let ordem = HashMap::from([((*b"sp", sp.chave()), t("04/10/2026 17:05:00")), ((*b"ac", ac.chave()), t("04/10/2026 17:10:00"))]);
    let mut v: Vec<_> = [&ac, &df, &sp].iter().map(|s| (chave_ordem(&ordem, s), s.uf)).collect();
    v.sort_unstable_by_key(|x| x.0);
    assert_eq!(v.iter().map(|x| x.1).collect::<Vec<_>>(), vec![*b"sp", *b"ac", *b"df"]);
    // sem métricas: disponibilização (SP 17:20, DF 17:25, AC 17:30), não a ordem alfabética das UFs
    let vazio = HashMap::new();
    let mut w: Vec<_> = [&ac, &df, &sp].iter().map(|s| (chave_ordem(&vazio, s), s.uf)).collect();
    w.sort_unstable_by_key(|x| x.0);
    assert_eq!(w.iter().map(|x| x.1).collect::<Vec<_>>(), vec![*b"sp", *b"df", *b"ac"]);
    // mesmo horário: pela UF
    let (a2, b2) = (sec(b"ba", 5, "04/10/2026 18:00:00"), sec(b"am", 9, "04/10/2026 18:00:00"));
    assert!(chave_ordem(&vazio, &b2) < chave_ordem(&vazio, &a2));
}
/// B-056 (25/10/2026, 2º turno, pleito 3221): às 06:07 UTC o `aux.json` de Wellington (ZZ 30805/0001/1088) já dizia
/// BU "Recebido" às 01:51:38 e o BU estava no ar, mas o `zz-p003221-cs.json` ainda era o de 09/10 (nenhuma das
/// 2.715 seções com disponibilização). O exterior tem de ser sondado já, sem esperar o `cs.json` nem um primeiro BU.
#[test]
fn exterior_sondado_antes_do_cs_json() {
    let t = |s: &str| urna_velox::metricas::segundos(s).unwrap();
    assert!(domingo(t("25/10/2026 03:07:00")) && domingo(t("04/10/2026 00:00:00")) && domingo(t("27/10/2024 23:59:59")));
    assert!(!domingo(t("24/10/2026 23:59:59")) && !domingo(t("26/10/2026 00:00:00")));
    // ZZ do 3221: 1.351 seções esperadas (2.715 − 1.364 agregadas), nada apurado, cs.json lido sem disponibilização
    assert_eq!(quando_sondar(true, true, 1351, 0, Some(0), false, true), Sonda::Antecipada, "B-056: o exterior não é sondado antes do cs.json");
    // achados os primeiros BUs pela sondagem, segue antecipada enquanto o cs.json não listar nada
    assert_eq!(quando_sondar(true, true, 1351, 5, Some(0), true, true), Sonda::Antecipada);
    // fora do domingo, sem o cs.json lido ou sem as agregadas (nsp) no cs.json: nada antecipado
    assert_eq!(quando_sondar(true, true, 1351, 0, Some(0), false, false), Sonda::Nao);
    assert_eq!(quando_sondar(true, true, 1351, 0, None, false, true), Sonda::Nao);
    assert_eq!(quando_sondar(true, false, 1351, 0, Some(0), false, true), Sonda::Nao);
    // UF do Brasil: nunca antecipada
    assert_eq!(quando_sondar(false, true, 38738, 0, Some(0), false, true), Sonda::Nao);
    // com o cs.json listando seções: no domingo o exterior segue antecipado (B-065); fora dele, a regra de 16/10
    assert_eq!(quando_sondar(true, true, 1351, 5, Some(5), true, true), Sonda::Antecipada);
    assert_eq!(quando_sondar(true, true, 1351, 5, Some(5), true, false), Sonda::Nao);
    assert_eq!(quando_sondar(true, true, 1351, 1310, Some(1310), true, false), Sonda::SeResolvidas);
    assert_eq!(quando_sondar(false, true, 38738, 38700, Some(38700), true, true), Sonda::SeResolvidas);
    assert_eq!(quando_sondar(false, true, 38738, 38738, Some(38738), true, true), Sonda::Nao);
}
/// B-058 (25/10/2026): depois de um reinício, o `ultimo_bu` do estado e dos resultados ficava vazio até o próximo BU
/// (às 17h57, com o TSE parado desde 17:11:45, o aviso M-019 não tinha o horário). Na retomada, o horário vem do
/// diário: a maior disponibilização das seções com BU (não instaladas e seções sem BU não contam).
#[test]
fn ultimo_bu_vem_do_diario_na_retomada() {
    let t = |s: &str| urna_velox::metricas::segundos(s).unwrap();
    let k = |s: u16| (*b"sp", tse::chave(71072, 1, s));
    let bu = Situ::Bu { imp: 0, sha: [0; 32], anulada: false, pos: 0, tam: 0 };
    let sit: HashMap<ChaveUf, Situ> = [(k(1), bu.clone()), (k(2), bu.clone()), (k(3), Situ::NaoInstalada), (k(5), bu)].into_iter().collect();
    let disp: HashMap<ChaveUf, i64> = [(k(1), t("25/10/2026 15:47:02")), (k(2), t("25/10/2026 17:11:45")), (k(3), t("25/10/2026 17:30:00")),
        (k(4), t("25/10/2026 17:40:00")), (k(5), 0)].into_iter().collect();
    assert_eq!(ultimo_bu_do_diario(&sit, &disp), Some(t("25/10/2026 17:11:45")), "B-058: o último BU do diário na retomada");
    assert_eq!(ultimo_bu_do_diario(&HashMap::new(), &disp), None, "sem BU no diário: nada");
}
/// B-065 (25/10/2026, 16h40): o `cs.json` do ZZ passou a listar seções às 14h e parou em 708 às 15h46; o `aux.json` e
/// o BU de Londres (29971/0001/0985) estavam no ar desde 15:47:02 e o concorrente tinha 776 × 708. Com o `cs.json`
/// listando alguma seção a sondagem antecipada do exterior acabava (só voltava faltando 5 % ou 50): o exterior
/// ficava preso ao ritmo do `cs.json`. No domingo o exterior segue sondado enquanto faltar seção.
#[test]
fn exterior_sondado_com_o_cs_json_atrasado() {
    assert_eq!(quando_sondar(true, true, 1351, 708, Some(708), true, true), Sonda::Antecipada, "B-065: o exterior segue sondado com o cs.json listando seções");
    assert_eq!(quando_sondar(true, true, 1351, 1310, Some(1310), true, true), Sonda::Antecipada);
    // tudo apurado, fora do domingo ou sem nsp: como antes
    assert_eq!(quando_sondar(true, true, 1351, 1351, Some(1351), true, true), Sonda::Nao);
    assert_eq!(quando_sondar(true, true, 1351, 708, Some(708), true, false), Sonda::Nao);
    assert_eq!(quando_sondar(true, false, 1351, 708, Some(708), true, true), Sonda::Nao);
    // Brasil: nada muda
    assert_eq!(quando_sondar(false, true, 38738, 700, Some(700), true, true), Sonda::Nao);
}
/// B-057 (25/10/2026): o exterior era sondado a 2 req/s e cada seção a cada 300 s (uma volta em ~11 min), na ordem
/// do catálogo. Agora: o calendário do `cs.json` do turno anterior diz quando cada seção costuma chegar; as devidas
/// agora vão primeiro e a cada 20 s, com reforço de 8 req/s, e um 429 acaba com o reforço.
#[test]
fn exterior_sondado_pelo_calendario() {
    let t = |s: &str| urna_velox::metricas::segundos(s).unwrap();
    // Wellington (30805/0001/1088) em 04/10: 01:49:24; a seção 1089 é agregada (sem data) e fica fora
    let cs = r#"{"dg":"05/10/2026","hg":"02:59:38","abr":[{"cd":"zz","mu":[{"cd":"30805","nm":"WELLINGTON","zon":[{"cd":"0001","sec":[{"ns":"1088","da":"04/10/2026","ha":"01:49:24"},{"ns":"1089","nsp":"1088"}]}]}]}]}"#;
    let cal = calendario_de_cs(cs);
    let esperado = 3600 + 49 * 60 + 24;
    assert_eq!(cal.len(), 1);
    assert_eq!(cal.get(&crate::tse::chave(30805, 1, 1088)), Some(&esperado), "B-057: calendário do turno anterior");
    assert!(calendario_de_cs("não é json").is_empty());
    let e = Some(esperado);
    // devida: de 30 min antes a 2 h depois do horário esperado (B-059)
    assert_eq!(faixa_calendario(e, t("25/10/2026 01:44:30")), (0, SONDA_REPETIR_DEVIDA_S), "B-057: seção devida agora");
    assert_eq!(faixa_calendario(e, t("25/10/2026 04:53:00")), (1, SONDA_REPETIR_ATRASADA_S));
    assert_eq!(faixa_calendario(e, t("25/10/2026 03:49:24")), (0, SONDA_REPETIR_DEVIDA_S));
    assert_eq!(faixa_calendario(e, t("25/10/2026 03:49:25")), (1, SONDA_REPETIR_ATRASADA_S));
    // atrasada: até 12 h; depois disso (ou ainda longe do horário) é futura, no ritmo de antes
    assert_eq!(faixa_calendario(e, t("25/10/2026 13:00:00")), (1, SONDA_REPETIR_ATRASADA_S));
    assert_eq!(faixa_calendario(e, t("25/10/2026 14:00:00")), (2, SONDA_REPETIR_S));
    assert_eq!(faixa_calendario(e, t("25/10/2026 01:18:00")), (2, SONDA_REPETIR_S));
    assert_eq!(faixa_calendario(None, t("25/10/2026 01:50:00")), (2, SONDA_REPETIR_S));
    // virada da meia-noite: esperado 23:50, agora 00:10 do dia seguinte
    assert_eq!(faixa_calendario(Some(23 * 3600 + 50 * 60), t("26/10/2026 00:10:00")), (0, SONDA_REPETIR_DEVIDA_S));
    // ritmo: reforço só na antecipada, sem 429 e com a fila do Brasil quase vazia
    assert_eq!(sondas_por_rodada(true, false, 0), SONDA_POR_RODADA_EXTERIOR, "B-057: reforço do exterior");
    assert_eq!(sondas_por_rodada(true, false, SONDA_FILA_OCIOSA - 1), SONDA_POR_RODADA_EXTERIOR);
    assert_eq!(sondas_por_rodada(true, false, SONDA_FILA_OCIOSA), SONDA_POR_RODADA);
    assert_eq!(sondas_por_rodada(true, true, 0), SONDA_POR_RODADA, "B-057: depois de um 429 o reforço acaba");
    assert_eq!(sondas_por_rodada(false, false, 0), SONDA_POR_RODADA);
}
/// B-059 (25/10/2026, 05h46): o concorrente tinha 13 seções de Nagoya e nós 3. Os BUs de Nagoya começaram a chegar
/// ~20 min antes do horário do 1º turno, fora da janela "devida" do calendário (B-057), e ficaram a 300 s por seção.
/// Vale para qualquer município: o que já tem BU é sondado inteiro a cada 20 s, e a janela abre 30 min antes.
#[test]
fn exterior_municipio_quente_sondado_a_cada_20s() {
    let t = |s: &str| urna_velox::metricas::segundos(s).unwrap();
    let e = Some(6 * 3600); // seção esperada às 06:00:00 pelo calendário do turno anterior
    // município ainda sem BU e longe do horário (1 h antes): ritmo de antes
    assert_eq!(faixa_secao(e, t("25/10/2026 05:00:00"), false), (2, SONDA_REPETIR_S));
    // município com BU: todas as seções a cada 20 s, qualquer que seja o horário esperado (ou sem calendário)
    assert_eq!(faixa_secao(e, t("25/10/2026 05:00:00"), true), (0, SONDA_REPETIR_DEVIDA_S), "B-059: município com BU não é sondado a cada 20 s");
    assert_eq!(faixa_secao(None, t("25/10/2026 05:00:00"), true), (0, SONDA_REPETIR_DEVIDA_S));
    assert_eq!(faixa_secao(e, t("25/10/2026 20:00:00"), true), (0, SONDA_REPETIR_DEVIDA_S));
    // chegou 14 min antes do calendário (Nagoya): dentro da janela de 30 min
    assert_eq!(faixa_secao(e, t("25/10/2026 05:46:00"), false), (0, SONDA_REPETIR_DEVIDA_S), "B-059: janela de 30 min antes do horário");
    assert_eq!(faixa_secao(e, t("25/10/2026 05:30:00"), false), (0, SONDA_REPETIR_DEVIDA_S));
    assert_eq!(faixa_secao(e, t("25/10/2026 05:29:59"), false), (2, SONDA_REPETIR_S));
    // depois do horário: como no B-057
    assert_eq!(faixa_secao(e, t("25/10/2026 08:00:01"), false), (1, SONDA_REPETIR_ATRASADA_S));
}
/// B-060 (25/10/2026, 13h05): os nós tinham 199 seções cada e `sondadas` parado em 1.170 de 1.351: ~180 seções do
/// exterior nunca foram consultadas. A faixa quente (140 seções a cada 20 s) enchia o orçamento da rodada e metade das
/// seleções eram do outro nó (descartadas sem requisição). Agora só as seções do nó disputam a rodada e uma cota dela
/// vai para as sondadas há mais tempo, qualquer que seja a faixa.
#[test]
fn sondagem_cobre_todas_as_secoes_do_no() {
    let ku = |i: u64| -> ChaveUf { (*b"zz", i) };
    // a seção está devida: nunca pedida, ou há `repetir` s
    assert!(devida(None, 0, 300) && !devida(Some(0), 299, 300) && devida(Some(0), 300, 300));
    // 1) a partição: só as seções do nó disputam o orçamento
    let lista: Vec<Candidata> = (0..20).map(|i| (0u8, i64::MIN, ku(i))).collect();
    let propria = so_do_no(lista, |k| k.1 % 2 == 0);
    assert_eq!(propria.len(), 10, "B-060: a rodada devia disputar só as seções deste nó");
    assert!(propria.iter().all(|c| c.2 .1 % 2 == 0));
    // 2) a cota: com a faixa quente sozinha enchendo a rodada, as nunca sondadas ainda entram
    let mut l: Vec<Candidata> = (0..50).map(|i| (0u8, 1000 + i as i64, ku(i))).collect();
    l.extend((100..110).map(|i| (4u8, i64::MIN, ku(i))));
    let esc = escolher_sondas(l.clone(), 12, 3);
    assert_eq!(esc.len(), 12);
    assert_eq!(esc.iter().filter(|k| k.1 >= 100).count(), 3, "B-060: a cota é das sondadas há mais tempo");
    // as demais vão por prioridade (as mais antigas da faixa quente primeiro)
    assert!(esc.iter().filter(|k| k.1 < 100).all(|k| k.1 < 9), "B-060: o resto por prioridade");
    // sem reforço (rodada comum, cota 0): só por prioridade, como antes
    assert!(escolher_sondas(l, SONDA_POR_RODADA, 0).iter().all(|k| k.1 < 50));
    // 3) simulação de 7 min, uma rodada por segundo, com os números de produção: 2.000 seções, metade de cada nó; as
    //    400 primeiras (200 deste nó) são quentes (a cada 20 s) e as outras, frias (a cada 300 s). Quente 200/20 = 10 req/s
    //    + frio 800/300 = 2,7 req/s > 12: sem a cota, as frias passam fome
    let mut sondadas: HashMap<ChaveUf, i64> = HashMap::new();
    let mut vezes: HashMap<u64, u32> = HashMap::new();
    let mut maior_intervalo_quente = 0;
    for agora in 0..420i64 {
        let mut lista: Vec<Candidata> = vec![];
        for i in 0..2000u64 {
            let (prio, rep) = if i < 400 { (0u8, SONDA_REPETIR_DEVIDA_S) } else { (4u8, SONDA_REPETIR_S) };
            let ult = sondadas.get(&ku(i)).copied();
            if devida(ult, agora, rep) { lista.push((prio, ult.unwrap_or(i64::MIN), ku(i))); }
        }
        let esc = escolher_sondas(so_do_no(lista, |k| k.1 % 2 == 0), SONDA_POR_RODADA_EXTERIOR, 3);
        assert!(esc.len() <= SONDA_POR_RODADA_EXTERIOR);
        for k in esc {
            assert!(k.1 % 2 == 0, "B-060: pediu seção do outro nó");
            if k.1 < 400 && agora > 100 { if let Some(&t) = sondadas.get(&k) { maior_intervalo_quente = maior_intervalo_quente.max(agora - t); } }
            sondadas.insert(k, agora);
            *vezes.entry(k.1).or_default() += 1;
        }
    }
    let frias_sem_consulta = (400..2000u64).filter(|i| i % 2 == 0 && !vezes.contains_key(i)).count();
    assert_eq!(frias_sem_consulta, 0, "B-060: seções frias deste nó nunca consultadas em 7 min");
    assert!(maior_intervalo_quente <= 60, "B-060: a faixa quente passou de 60 s entre consultas ({maior_intervalo_quente} s)");
}
#[test]
fn painel_desenha() {
    assert_eq!((mil(0), mil(999), mil(1000), mil(1234567)), ("0".into(), "999".into(), "1.000".into(), "1.234.567".into()));
    let p = Painel { prefixo: "br-c0001".into(), nomes: HashMap::from([((1, 22), "FULANO".into()), ((1, 13), "BELTRANO".into())]),
        arquivo: None, ultimo: None, t0: Instant::now(), terminal: true };
    let v = json!({"eleicao": 6257, "turno": 1, "cargo": 1, "abr": "br", "seq": 480125, "ultimo_bu": "2026-10-04T21:35:22-03:00",
        "encerrada": false, "secoes": {"total": 517179, "apuradas": 480125}, "eleitorado": 158745502, "comparecimento": 120000000,
        "validos": 114000000, "vb": 2000000, "vn": 3000000, "vnt": 5000, "estado": "matematicamente_2t",
        "cand": [{"n": 22, "v": 54871226, "pct": 46.99, "st": "", "mat": {"estado": "2t", "seq": 480125, "em": "2026-10-04T21:35:22-03:00"}},
                 {"n": 13, "v": 52828648, "pct": 45.24, "st": "", "mat": null}, {"n": 99, "v": 0, "pct": 0.0, "st": "", "mat": null}]});
    let t = p.desenhar(&v, true);
    assert!(t.starts_with("PRESIDENTE · BR · eleição 6257 (1º turno)"), "{t}");
    assert!(t.contains("seções 480.125/517.179 (92.84 %)") && t.contains("nulos 3.005.000") && t.contains("estado: MATEMATICAMENTE_2T"), "{t}");
    assert!(t.contains(" 22 FULANO") && t.contains("54.871.226  46.99 %") && t.contains("← matematicamente 2t (seq 480.125, 21:35:22)"), "{t}");
    assert!(t.contains(" 99 ?"), "candidato sem nome: {t}");
    let r = p.desenhar(&v, false);
    assert!(r.contains("PRESIDENTE BR seq 480125") && r.contains("22 FULANO 46.99% · 13 BELTRANO 45.24%"), "{r}");
}

#[test]
fn amostra_janela_e_total() {
    let mut a = Amostra::default();
    for i in 0..100_000 { a.por(if i < 60_000 { 0.5 } else { 2.0 }); }
    // a janela (últimas 50 mil) só vê 2,0; o total tem 60 % de 0,5
    assert_eq!(a.pct(50.0), 2.0);
    assert!((a.pct_total(50.0) - 0.505).abs() < 1e-9 && (a.pct_total(99.0) - 2.005).abs() < 1e-9, "{} {}", a.pct_total(50.0), a.pct_total(99.0));
    a.por(250.7);
    assert_eq!(a.pct_total(100.0), 250.5);
    assert_eq!(a.max_total, 250.7);
}

#[test]
fn diario_ida_e_volta() {
    let p = std::env::temp_dir().join(format!("uv-diario-{}.log", std::process::id()));
    let k = (*b"am", tse::chave(2550, 1, 550));
    let k2 = (*b"zz", tse::chave(29629, 1, 3014));
    let sha = [7u8; 32];
    let mut s = String::from(CAB_DIARIO);
    s.push('\n');
    s += &linha_diario("aplicado", k, "aa", Some(&sha), "Totalizado", false, "27/10/2024 17:27:08", "27/10/2024 17:27:08", "am.uvpk", 60, 947, 1);
    s += &linha_diario("anulado", k, "aa", Some(&sha), "Excluído", true, "27/10/2024 18:00:00", "", "am.uvpk", 60, 947, 2);
    s += &linha_diario("nao_instalada", k2, "", None, "Nao instalada", false, "", "", "", 0, 0, 2);
    s += &linha_diario("pendente", (*b"am", 9), "", None, "aux 404", false, "", "", "", 0, 0, 2);
    s += "1,aplicado,am,2550,1"; // linha cortada por uma queda
    std::fs::write(&p, s).unwrap();
    let (sit, disp, n) = ler_diario(&p).unwrap();
    let _ = std::fs::remove_file(&p);
    assert_eq!(n, 4);
    match &sit[&k] { Situ::Bu { anulada, pos, tam, imp, sha: s } => assert_eq!((*anulada, *pos, *tam, *imp, *s), (true, 60, 947, impressao("aa"), sha)), x => panic!("{x:?}") }
    assert!(matches!(sit[&k2], Situ::NaoInstalada));
    assert_eq!(disp.get(&k).copied(), Some(t_disp("27/10/2024 18:00:00")));
    assert!(t_disp("27/10/2024 18:00:00") > 0 && t_disp("") == -1);
    assert_ne!(impressao("aa"), impressao("ab"));
    assert!(!disp.contains_key(&(*b"am", 9)));
}
