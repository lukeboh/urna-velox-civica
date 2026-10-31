use super::*;

fn federado_teste(d: &std::path::Path) -> Federado {
    let fed_json = d.join("federacao.json");
    std::fs::write(&fed_json, r#"{"versao":1,"nos":[{"id":"no-a","orgao":"A","ip_saida":"127.0.0.2"}]}"#).unwrap();
    Federado::novo(&Opcoes {
        federacao: fed_json,
        no: "no-a".into(),
        armazem: d.join("armazem"),
        eco_ip: None,
        escuta: None,
    }, "http://127.0.0.1", 1.0).unwrap()
}

#[test]
fn lote_ida_e_volta() {
    let c1 = Cabecalho { tipo: "bu".into(), uf: "am".into(), m: 2550, z: 1, s: 550, hash: "abc".into(), sha: "00".into(), situacao: "Totalizado".into(),
        anulada: false, disp: "27/10/2024 17:01:02".into(), recebido: "x".into(), hora_site: 9 };
    let mut c2 = c1.clone();
    c2.tipo = "nao_instalada".into();
    let regs = vec![(c1, Bytes::from_static(b"\x30\x80bu")), (c2, Bytes::new())];
    let b = codificar(&regs);
    assert_eq!(decodificar(&b).unwrap(), regs);
    assert!(decodificar(&b[..b.len() - 1]).is_err());
    assert!(decodificar(b"outra coisa").is_err());
    assert_eq!(k_de(&regs[0].0), Some((*b"am", tse::chave(2550, 1, 550))));
}

/// Sonda pela federação: vai no lote do nó sem contar como resultado publicado e volta, no par, como sonda remota
/// com a origem, o id, a sessão e o instante da injeção.
#[test]
fn sonda_no_lote_e_no_par() {
    let tmp = std::env::temp_dir().join(format!("uv-fed-sonda-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    let f = federado_teste(&tmp);
    let s = crate::sonda::Sonda::nova(7, 1_792_000_000_000_000, 1_792_000_000_123_456);
    f.publicar_sonda(&s);
    assert_eq!((f.cont.publicados.load(Relaxed), f.cont.sondas_publicadas.load(Relaxed)), (0, 1), "sonda não é resultado");
    gravar_lote(&f).unwrap();
    let l = decodificar(&match f.armazem.ler(&chave_lote(&f.id, 1), None).unwrap() { Lido::Novo(o) => o.bytes, _ => panic!() }).unwrap();
    assert_eq!(l.len(), 1);
    let (c, b) = &l[0];
    assert_eq!((c.tipo.as_str(), c.uf.as_str(), b.len()), ("sonda", "", 0));
    assert_eq!(k_de(c), None, "sem seção: um nó antigo a rejeita sem aplicar nada");
    let r = sonda_do_par("no-a", c, 1_792_000_000_200_000).unwrap();
    assert_eq!(r, crate::sonda::Sonda::remota("no-a", 7, 1_792_000_000_000_000, 1_792_000_000_123_456, 1_792_000_000_200_000));
    assert_eq!(r.json()["ms"]["federacao"], 76.544);
    // ilegíveis
    let mut c2 = c.clone();
    c2.hash = "x".into();
    assert_eq!(sonda_do_par("no-a", &c2, 0), None);
    let mut c3 = c.clone();
    c3.hora_site = -1;
    assert_eq!(sonda_do_par("no-a", &c3, 0), None);
    let mut c4 = c.clone();
    c4.recebido = String::new();
    assert_eq!(sonda_do_par("no-a", &c4, 0).unwrap().sessao, 0, "sem sessão: 0");
    let j = f.saude_json();
    assert_eq!((j["sondas_publicadas"].as_u64(), j["sondas_recebidas"].as_u64(), j["publicados"].as_u64()), (Some(1), Some(0), Some(0)));
    // a sonda não entra nos contadores persistidos (retomada)
    assert!(f.cont.instantaneo().get("sondas_publicadas").is_none());
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn gravar_lote_fraciona_em_tamanho_maximo() {
    let tmp = std::env::temp_dir().join(format!("uv-fed-lote-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    let f = federado_teste(&tmp);

    // Enfileira 1.200 registros (mais que 2x LOTE_MAX_REGISTROS = 500)
    for i in 0..1200u16 {
        let c = Cabecalho {
            tipo: "nao_instalada".into(),
            uf: "am".into(),
            m: 2550,
            z: 1,
            s: i,
            hash: String::new(),
            sha: String::new(),
            situacao: String::new(),
            anulada: false,
            disp: "04/10/2026 17:00:00".into(),
            recebido: String::new(),
            hora_site: 0,
        };
        f.guardar(c, Bytes::new());
    }
    assert_eq!(f.saida.lock().unwrap().len(), 1200);

    gravar_lote(&f).unwrap();
    assert_eq!(f.saida.lock().unwrap().len(), 0);
    assert_eq!(f.seq.load(Relaxed), 3); // 500 + 500 + 200 = 3 lotes
    assert_eq!(f.cont.lotes_gravados.load(Relaxed), 3);

    // Confere os tamanhos de cada lote no armazém
    let l1 = decodificar(&match f.armazem.ler(&chave_lote(&f.id, 1), None).unwrap() { Lido::Novo(o) => o.bytes, _ => panic!() }).unwrap();
    let l2 = decodificar(&match f.armazem.ler(&chave_lote(&f.id, 2), None).unwrap() { Lido::Novo(o) => o.bytes, _ => panic!() }).unwrap();
    let l3 = decodificar(&match f.armazem.ler(&chave_lote(&f.id, 3), None).unwrap() { Lido::Novo(o) => o.bytes, _ => panic!() }).unwrap();
    assert_eq!(l1.len(), 500);
    assert_eq!(l2.len(), 500);
    assert_eq!(l3.len(), 200);

    let _ = std::fs::remove_dir_all(&tmp);
}

/// B-006: os contadores da federação sobrevivem à retomada (mesmo armazém, `--continuar`): depois de reiniciar,
/// `publicados`, `lotes_gravados` e os demais seguem de onde estavam (antes recomeçavam do zero: o nó B mostrou
/// 91 mil publicados ao fim tendo coletado ~240 mil).
#[test]
fn contadores_sobrevivem_a_retomada() {
    let tmp = std::env::temp_dir().join(format!("uv-fed-contadores-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    let f = federado_teste(&tmp);
    for i in 0..700u16 { f.guardar(Federado::cab("nao_instalada", (*b"am", tse::chave(2550, 1, i))), Bytes::new()); }
    f.cont.recebidos.fetch_add(42, Relaxed);
    f.cont.ja_aplicados.fetch_add(7, Relaxed);
    gravar_lote(&f).unwrap();
    assert_eq!((f.cont.publicados.load(Relaxed), f.cont.lotes_gravados.load(Relaxed)), (700, 2));
    drop(f);

    // retomada: novo processo, mesmo armazém
    let g = federado_teste(&tmp);
    assert_eq!(g.seq.load(Relaxed), 2);
    assert_eq!((g.cont.publicados.load(Relaxed), g.cont.lotes_gravados.load(Relaxed)), (700, 2), "publicados/lotes recomeçaram");
    assert_eq!((g.cont.recebidos.load(Relaxed), g.cont.ja_aplicados.load(Relaxed)), (42, 7));
    // e continuam acumulando
    g.guardar(Federado::cab("nao_instalada", (*b"am", tse::chave(2550, 2, 1))), Bytes::new());
    gravar_lote(&g).unwrap();
    assert_eq!((g.cont.publicados.load(Relaxed), g.cont.lotes_gravados.load(Relaxed), g.seq.load(Relaxed)), (701, 3, 3));
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn republicar_ignora_diario_desalinhado_ou_antigo() {
    let tmp = std::env::temp_dir().join(format!("uv-fed-desalinhado-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    let f = federado_teste(&tmp);

    // Cria o lote 1 no armazém
    f.guardar(Cabecalho {
        tipo: "nao_instalada".into(),
        uf: "am".into(),
        m: 2550,
        z: 1,
        s: 1,
        hash: String::new(),
        sha: String::new(),
        situacao: String::new(),
        anulada: false,
        disp: String::new(),
        recebido: String::new(),
        hora_site: 0,
    }, Bytes::new());
    gravar_lote(&f).unwrap();
    assert_eq!(f.seq.load(Relaxed), 1);

    // Pega o mtime do lote 1
    let lote_ms = match f.armazem.ler(&chave_lote(&f.id, 1), None).unwrap() {
        Lido::Novo(o) => o.versao.modificado.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis() as u64,
        _ => panic!(),
    };

    // Cria um secoes.log com timestamp 2 horas após o lote 1 (caso do nó A que causou OOM)
    let log_p = tmp.join("secoes.log");
    let ts_futuro = lote_ms + 7_200_000;
    let mut log_conteudo = String::from(crate::servico::CAB_DIARIO);
    log_conteudo.push('\n');
    for i in 0..10_000u32 {
        log_conteudo.push_str(&format!("{ts_futuro},nao_instalada,am,2550,1,{i},,,Nao instalada,0,04/10/2026 17:00:00,,,0,0,1\n"));
    }
    std::fs::write(&log_p, log_conteudo).unwrap();

    // republicar deve reconhecer o desalinhamento e não carregar nada na memória
    let r = republicar(&f, &tmp, &tmp).unwrap();
    assert_eq!(r.registros, 0, "não deve republicar se o diário for de outra execução");
    assert!(r.desalinhado.is_some(), "o desalinhamento tem de voltar para virar alerta");
    assert_eq!(f.saida.lock().unwrap().len(), 0);
    assert_eq!(f.seq.load(Relaxed), 1);

    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn republicar_recupera_apenas_ultimos_bus_do_corte() {
    let r = cenario_corte("poucas", 5);
    assert_eq!(r.registros, 5, "deve republicar exatamente as 5 seções posteriores ao corte");
    assert_eq!(r.lotes, 1);
    assert!(r.desalinhado.is_none());
}

/// Queda com muito por publicar (ex.: storage fora do ar por minutos antes do `kill -9`): tudo o que passou do corte
/// é republicado, sem teto, mas em lotes de no máximo [`LOTE_MAX_REGISTROS`].
#[test]
fn republicar_sem_teto_em_lotes_limitados() {
    let r = cenario_corte("muitas", 2_345);
    assert_eq!(r.registros, 2_345);
    assert_eq!(r.lotes, 5); // 500 × 4 + 345
    assert!(r.desalinhado.is_none());
}

/// Um lote no storage, um diário com 50 linhas antes do corte (lote − 60 s) e `novas` depois dele (lote − 10 s);
/// republica e confere que a memória terminou vazia e que cada lote gravado tem no máximo [`LOTE_MAX_REGISTROS`].
fn cenario_corte(nome: &str, novas: u32) -> Republicacao {
    let tmp = std::env::temp_dir().join(format!("uv-fed-corte-{nome}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    let f = federado_teste(&tmp);
    f.guardar(Federado::cab("nao_instalada", (*b"am", tse::chave(2550, 1, 1))), Bytes::new());
    gravar_lote(&f).unwrap();
    let lote_ms = match f.armazem.ler(&chave_lote(&f.id, 1), None).unwrap() {
        Lido::Novo(o) => o.versao.modificado.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis() as u64,
        _ => panic!(),
    };
    let mut log = String::from(crate::servico::CAB_DIARIO);
    log.push('\n');
    for i in 0..50u64 {
        log.push_str(&format!("{},nao_instalada,am,2550,1,{i},,,Nao instalada,0,04/10/2026 17:00:00,,,0,0,1\n", lote_ms - 60_000 + i));
    }
    for i in 0..novas as u64 {
        log.push_str(&format!("{},nao_instalada,am,2550,2,{i},,,Nao instalada,0,04/10/2026 17:00:00,,,0,0,1\n", lote_ms - 10_000 + i % 1_000));
    }
    std::fs::write(tmp.join("secoes.log"), log).unwrap();
    let r = republicar(&f, &tmp, &tmp).unwrap();
    assert_eq!(f.saida.lock().unwrap().len(), 0, "nada fica em memória depois da republicação");
    for s in 2..=f.seq.load(Relaxed) {
        let o = match f.armazem.ler(&chave_lote(&f.id, s), None).unwrap() { Lido::Novo(o) => o, _ => panic!() };
        assert!(decodificar(&o.bytes).unwrap().len() <= LOTE_MAX_REGISTROS);
    }
    let _ = std::fs::remove_dir_all(&tmp);
    r
}
