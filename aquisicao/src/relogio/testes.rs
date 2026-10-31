use super::*;

#[test]
fn iso_ida_e_volta() {
    let t = urna_velox::metricas::segundos("18/10/2026 20:41:03").unwrap() * 1000 + 512;
    assert_eq!(iso_ms(t), "2026-10-18T20:41:03.512-03:00");
    assert_eq!(ms_iso(&iso_ms(t)), Some(t));
    assert_eq!(ms_iso("2026-10-18T20:41:03-03:00"), Some(t - 512));
    assert_eq!(ms_iso("2026-10-18 20:41:03"), Some(t - 512));
    assert_eq!(ms_iso("—"), None);
}

#[test]
fn ancora_converte_nos_dois_sentidos() {
    // simulado começa hoje às 07:00:00 (sistema) reproduzindo a noite de 06/10/2024 a partir das 17:00:00, a 500×
    let s0 = urna_velox::metricas::segundos("18/10/2026 07:00:00").unwrap() * 1000;
    let n0 = urna_velox::metricas::segundos("06/10/2024 17:00:00").unwrap();
    let r = Relogio::novo(500.0);
    r.ver_site_em(0, s0 - 5000);
    assert!(r.ancora().is_none(), "sem horário do site, sem âncora");
    r.ver_site_em(n0, s0);
    r.ver_site_em(n0 + 999, s0 + 1);
    let a = r.ancora().unwrap();
    assert_eq!(a, Ancora { noite0: n0 as f64, sis0_ms: s0, acelerar: 500.0 });
    // 10 h da noite do TSE (17:00 → 03:00) = 72 s do sistema
    assert_eq!(a.sistema(n0 + 36_000), s0 / 1000 + 72);
    assert_eq!(a.noite(s0 + 72_000), (n0 + 36_000) as f64);
    // a velocidade cai para 100× aos 72 s: o instante da noite continua o mesmo e o resto anda 5× mais devagar
    r.mudar_aceleracao_em(100.0, s0 + 72_000);
    let b = r.ancora().unwrap();
    assert_eq!(b.noite(s0 + 72_000), (n0 + 36_000) as f64);
    assert_eq!(b.noite(s0 + 73_000), (n0 + 36_100) as f64);
    // na noite real, a âncora só desconta a diferença de relógio
    let real = Relogio::novo(1.0);
    real.ver_site_em(n0, n0 * 1000 + 2_000);
    assert_eq!(real.ancora().unwrap().sistema(n0 + 600), n0 + 602);
    r.reiniciar();
    assert!(r.ancora().is_none());
}
