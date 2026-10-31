use super::*;
/// Maior peso numa janela da regra para requisições nos instantes `ts` (s reais) com os pesos `pesos`.
fn pico(regra: RegraLimite, ts: &[f64], pesos: &[f64]) -> f64 {
    let mut j = JanelaIp::default();
    ts.iter().zip(pesos).map(|(&t, &p)| j.somar(regra, t, p)).fold(0.0, f64::max)
}
fn uniforme(rps: f64, de: f64, ate: f64) -> Vec<f64> { (0..((ate - de) * rps) as usize).map(|i| de + i as f64 / rps).collect() }
/// B-067: as regras do limite por IP. A saída espaçada a 95 req/s (o `Limitador` do coletor) nunca passa de 100
/// em nenhuma regra; a mesma média, chegando amontoada na virada de um segundo, passa na janela deslizante e não
/// nos segundos fechados; os 404 com peso maior passam o limite abaixo de 100 req/s; o bloco de 10 s absorve um
/// segundo de 110.
#[test]
fn regras_do_limite_por_ip() {
    let um = |n: usize| vec![1.0; n];
    let (seg, desl, dez) = (RegraLimite::Fixa(1.0), RegraLimite::Deslizante(1.0), RegraLimite::Fixa(10.0));
    // 95 req/s espaçadas por 10 s
    let t = uniforme(95.0, 0.0, 10.0);
    for r in [seg, desl, dez, RegraLimite::Deslizante(10.0)] {
        let pk = pico(r, &t, &um(t.len()));
        assert!(!excede(r, pk, 100.0), "{r:?}: pico {pk} com 95 req/s espaçadas");
    }
    // 60 req em 0,90–0,99 s e 60 em 1,00–1,09 s: 60 por segundo fechado, 120 em 1 s deslizante
    let mut t: Vec<f64> = (0..60).map(|i| 0.90 + i as f64 * 0.0015).collect();
    t.extend((0..60).map(|i| 1.00 + i as f64 * 0.0015));
    assert_eq!(pico(seg, &t, &um(120)), 60.0);
    assert!(!excede(seg, 60.0, 100.0));
    assert_eq!(pico(desl, &t, &um(120)), 120.0);
    assert!(excede(desl, 120.0, 100.0), "amontoado na virada do segundo: 429 na deslizante");
    // 80 req/s, 30 % delas 404 com peso 2: 56 + 24 × 2 = 104 por segundo
    let t = uniforme(80.0, 0.0, 5.0);
    let pesos: Vec<f64> = (0..t.len()).map(|i| if i % 10 < 3 { 2.0 } else { 1.0 }).collect();
    let pk = pico(seg, &t, &pesos);
    assert!(excede(seg, pk, 100.0) && !excede(seg, pico(seg, &t, &um(t.len())), 100.0), "404 com peso 2: {pk}");
    // um segundo com 110 e depois nada: 429 em segundos fechados, não no bloco de 10 s (limite 1000)
    let t = uniforme(110.0, 0.0, 1.0);
    assert!(excede(seg, pico(seg, &t, &um(110)), 100.0));
    assert!(!excede(dez, pico(dez, &t, &um(110)), 100.0));
    // a janela deslizante esquece o que saiu dela
    let mut j = JanelaIp::default();
    for &x in &uniforme(100.0, 0.0, 1.0) { j.somar(desl, x, 1.0); }
    assert_eq!(j.somar(desl, 5.0, 1.0), 1.0);
}
#[test]
fn regra_do_limite_de_texto() {
    assert_eq!(RegraLimite::de_texto("segundo").unwrap(), RegraLimite::Fixa(1.0));
    assert_eq!(RegraLimite::de_texto("fixa:10").unwrap(), RegraLimite::Fixa(10.0));
    assert_eq!(RegraLimite::de_texto("deslizante").unwrap(), RegraLimite::Deslizante(1.0));
    assert_eq!(RegraLimite::de_texto("deslizante:0.5").unwrap(), RegraLimite::Deslizante(0.5));
    assert!(RegraLimite::de_texto("deslizante:0").is_err());
    assert!(RegraLimite::de_texto("balde").is_err());
    assert_eq!(RegraLimite::default().texto(), "fixa:1");
}
#[test]
fn textos() {
    let d = segundos("27/10/2024 00:00:00").unwrap();
    assert_eq!(hora("17:05:10", d + 3600), Some(d + 17 * 3600 + 5 * 60 + 10));
    assert_eq!(hora("28/10/2024 01:00:00", d), Some(d + 86400 + 3600));
    assert_eq!(secao_de_texto("ma:7315/23/354"), Some((*b"ma", tse::chave(7315, 23, 354))));
    assert_eq!(secao_de_texto("ma:7315/23"), None);
    assert_ne!(hash_reenvio("abc", 1), hash_reenvio("abc", 2));
    assert_eq!(tse::sha256_do_hash_dir(&hash_reenvio("abc", 1)) == [0; 32], false);
}
/// Os BUs publicados antes das 17h (exterior, 1º turno de 2026) entram juntos às 17h; os outros não mudam.
#[test]
fn piso_das_17h() {
    let h = |x: &str| segundos(&format!("04/10/2026 {x}"));
    let piso = h("17:00:00");
    // Wellington: publicada 01:49:24, "recebida" 17:08:44 (o lote do aux.json)
    assert_eq!(com_piso(h("17:08:44"), h("01:49:24"), piso), piso, "recebimento agrupado às 17:08");
    assert_eq!(com_piso(h("01:49:24"), h("01:49:24"), piso), piso, "pela disponibilização");
    // Budapeste, só com o horário escolhido (13:44)
    assert_eq!(com_piso(h("13:44:27"), None, piso), piso);
    // Brasil e Miami: depois das 17h, como estão
    assert_eq!(com_piso(h("17:00:31"), h("17:01:02"), piso), h("17:00:31"));
    assert_eq!(com_piso(h("20:57:03"), h("20:57:04"), piso), h("20:57:03"));
    // sem horário continua sem; sem piso, nada muda
    assert_eq!(com_piso(None, h("01:49:24"), piso), None);
    assert_eq!(com_piso(h("17:08:44"), h("01:49:24"), None), h("17:08:44"));
}
#[test]
fn relogio_para_no_alvo() {
    let mut r = Relogio { base_sim: 1000.0, base_real: Instant::now(), acelerar: 100_000.0, alvo: Some((1500.0, 2.0)), limite: f64::INFINITY };
    std::thread::sleep(Duration::from_millis(30));
    // a 100 000× passaria de 1500 em 5 ms: para no alvo e segue a 2×
    let (t, chegou) = r.ler();
    assert!(chegou && (1500.0..1501.0).contains(&t), "{t}");
    assert_eq!((r.acelerar, r.alvo), (2.0, None));
    r.posicionar(1200.0);
    assert!((1200.0..1200.5).contains(&r.agora()));
    // o relógio para no limite
    r.limite = 1300.0;
    r.mudar(100_000.0);
    std::thread::sleep(Duration::from_millis(10));
    assert_eq!((r.agora(), r.acelerar), (1300.0, 0.0));
    r.limite = f64::INFINITY;
    r.posicionar(1200.0);
    let j: serde_json::Value = serde_json::from_str(&json_controle(2.0, 1200.0, 1000, 2000, Some((1800.0, 10.0)))).unwrap();
    assert_eq!((j["t"].as_i64(), j["fim"].as_i64(), j["alvo"]["t"].as_i64(), j["alvo"]["depois"].as_f64()), (Some(1200), Some(2000), Some(1800), Some(10.0)));
}
#[test]
fn relogio_muda_sem_salto() {
    let mut r = Relogio { base_sim: 1000.0, base_real: Instant::now(), acelerar: 10.0, alvo: None, limite: f64::INFINITY };
    std::thread::sleep(Duration::from_millis(20));
    let a = r.agora();
    assert!(a > 1000.1);
    r.mudar(1000.0);
    let b = r.agora();
    assert!(b >= a && b - a < 1.0, "{a} → {b}");
    std::thread::sleep(Duration::from_millis(20));
    assert!(r.agora() - b > 10.0);
    r.mudar(0.0);
    let c = r.agora();
    std::thread::sleep(Duration::from_millis(10));
    assert_eq!(r.agora(), c);
    r.mudar(1.0);
    assert!(r.agora() >= c);
}
