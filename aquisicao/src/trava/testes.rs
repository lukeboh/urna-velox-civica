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
