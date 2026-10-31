use super::*;

#[test]
fn acesso_interno_so_fora_da_borda() {
    // borda: igual a `limites`, com o teto de 100 req/s
    assert_eq!(limites_acesso("https://resultados.tse.jus.br", 500.0, 1.0, Acesso::Borda).unwrap(), (TETO_RPS, TETO_RPS, PAUSA_429_TSE_S));
    assert_eq!(limites_acesso("https://espelho.exemplo.org", 95.0, 1.0, Acesso::Borda).unwrap(), limites("https://espelho.exemplo.org", 95.0, 1.0).unwrap());
    // interno com teto: o pedido vale até o teto; sem teto: sem limite
    let i300 = Acesso::de_args("interno", Some(300.0)).unwrap();
    assert_eq!(limites_acesso("https://espelho.exemplo.org", 250.0, 1.0, i300).unwrap(), (300.0, 250.0, PAUSA_429_TSE_S));
    assert_eq!(limites_acesso("https://espelho.exemplo.org", 900.0, 1.0, i300).unwrap().1, 300.0);
    let livre = Acesso::de_args("interno", Some(0.0)).unwrap();
    assert_eq!(livre, Acesso::Interno { teto: None });
    assert_eq!(limites_acesso("http://10.1.2.3:8000", 0.0, 1.0, livre).unwrap().0, SEM_LIMITE_RPS);
    // nunca na borda pública nem no simulador local
    for b in ["https://resultados.tse.jus.br", "https://CDN.tse.jus.br:443/x", "http://127.0.0.1:18080"] {
        assert!(limites_acesso(b, 95.0, 1.0, livre).is_err(), "{b}");
    }
    assert!(Acesso::de_args("tanto-faz", None).is_err());
    assert!(livre.validar(&["https://espelho.exemplo.org", "https://cdn.tse.jus.br"]).is_err(), "a base dos arquivos também conta");
    assert_eq!(host("https://Resultados.TSE.jus.br:443/oficial"), "resultados.tse.jus.br");
}

#[test]
fn rede_interna_passa_de_100() {
    let cfg = ConfigRede { teto: 400.0, rps: 400.0, ..ConfigRede::simples(8, 400.0, 1, 5, 0) };
    let r = Rede::para_acesso("https://espelho.exemplo.org", &cfg, Acesso::Interno { teto: Some(400.0) }).unwrap();
    assert_eq!(r.teto(), 400.0);
    let r = Rede::para_acesso("https://resultados.tse.jus.br", &cfg, Acesso::Borda).unwrap();
    assert_eq!(r.teto(), TETO_RPS, "borda: continua 100");
    assert!(Rede::para_acesso("https://resultados.tse.jus.br", &cfg, Acesso::Interno { teto: None }).is_err());
}
#[tokio::test]
async fn nunca_passa_do_teto() {
    let l = Limitador::com(5000.0, false, 0.0); // pedido acima do teto é reduzido a TETO_RPS
    let t0 = Instant::now();
    for _ in 0..51 { l.esperar().await; }
    // 51 requisições a no máximo 100/s ocupam pelo menos 0,5 s
    assert!(t0.elapsed() >= Duration::from_millis(495), "{:?}", t0.elapsed());
    let a = Limitador::com(1000.0, true, 500.0);
    assert!(a.rps_atual() <= TETO_RPS);
}

/// Só 127.0.0.1/localhost/::1 são servidores locais; nada que só pareça local.
#[test]
fn servidor_local() {
    for b in ["http://127.0.0.1:8080", "http://localhost", "http://LOCALHOST:1/x", "http://[::1]:9000/oficial", "127.0.0.1:5"] { assert!(base_local(b), "{b}"); }
    for b in ["https://resultados.tse.jus.br", "http://127.0.0.1.nip.io", "http://localhost@tse.jus.br", "http://localhost.tse.jus.br:80",
        "http://10.0.0.1", "http://[::2]:80", "http://0.0.0.0"] { assert!(!base_local(b), "{b}"); }
}

/// Contra o TSE: teto 100, req/s ≤ 100, pausa ≥ 600 s, aceleração recusada — qualquer que seja o pedido.
/// Contra um servidor local: teto e pausa escalados pela aceleração.
#[test]
fn limites_nao_afrouxam_contra_o_tse() {
    let tse = "https://resultados.tse.jus.br";
    assert_eq!(limites(tse, 95.0, 1.0).unwrap(), (100.0, 95.0, 600.0));
    assert_eq!(limites(tse, 5000.0, 1.0).unwrap(), (100.0, 100.0, 600.0));
    assert_eq!(limites(tse, 0.0, 0.5).unwrap(), (100.0, 95.0, 600.0));
    assert!(limites(tse, 95.0, 10.0).is_err());
    assert!(limites("http://127.0.0.1.nip.io:80", 95.0, 10.0).is_err());
    assert_eq!(limites("http://127.0.0.1:1", 95.0, 10.0).unwrap(), (1000.0, 950.0, 60.0));
    // sem o teto do TSE contra um servidor local (simulações e ensaios)
    assert_eq!(limites("http://127.0.0.1:1", 5000.0, 1.0).unwrap(), (5000.0, 5000.0, 600.0));
    // Rede::para contra o TSE ignora teto e pausa afrouxados na configuração
    let mut c = ConfigRede::simples(4, 5000.0, 1, 5, 0);
    c.teto = 5000.0;
    c.pausa_429_s = 1.0;
    let r = Rede::para(tse, &c).unwrap();
    assert_eq!(r.teto(), TETO_RPS);
    assert_eq!(r.cfg.pausa_429_s, PAUSA_429_TSE_S);
    assert!(r.restrita_a.is_none());
    // contra um servidor local com teto acima de 100, a rede só aceita URLs desse servidor
    let l = Rede::para("http://127.0.0.1:1", &c).unwrap();
    assert_eq!(l.teto(), 5000.0);
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    assert!(rt.block_on(l.get("https://resultados.tse.jus.br/oficial/ele2026/x", None, None)).is_err());
}
