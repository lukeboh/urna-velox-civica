use super::*;

const ARQ: &str = "{\"abr\":\"br\",\"cand\":[{\"mat\":{\"estado\":\"2t\",\"seq\":7},\"n\":13}],\"secoes\":{\"apuradas\":40,\"total\":50},\"seq\":99}\n";

#[test]
fn topicos_dos_arquivos() {
    assert_eq!(topico_de("br-c0001-e006257.json"), Some("br-c0001"));
    assert_eq!(topico_de("ba34134-c0011-e000620.json"), Some("ba34134-c0011"));
    assert_eq!(topico_de("saude.json"), None);
    assert_eq!(topico_de("../x-e1.json"), None);
}

#[test]
fn quadro_traz_o_arquivo_byte_a_byte() {
    let q = Quadro::novo("br-c0001".into(), "br-c0001-e006257.json".into(), Bytes::from_static(ARQ.as_bytes()), 3, Some(1_000));
    let mo = q.m().unwrap();
    assert_eq!((mo.seq, mo.secoes, q.ver), (99, 40, 3), "seq do topo, não o do matematicamente");
    assert_eq!(&mo.res[..], ARQ.as_bytes());
    assert!(std::ptr::eq(q.m().unwrap(), mo), "montada uma vez só");
    let m: &str = mo.msg.as_str();
    assert!(m.starts_with("{\"tipo\":\"atualizacao\",\"topico\":\"br-c0001\",\"seq\":99,\"secoes_apuradas\":40,\"ver\":3,\"t_bu\":1000,"), "{m}");
    let i = m.find("\"resultado\":").unwrap() + "\"resultado\":".len();
    assert_eq!(&m.as_bytes()[i..m.len() - 1], ARQ.as_bytes());
    let v: Value = serde_json::from_str(m).unwrap();
    assert_eq!(v["resultado"]["seq"], 99);
    let ins = q.instantaneo().unwrap();
    let vi: Value = serde_json::from_str(ins.as_str()).unwrap();
    assert_eq!((vi["tipo"].as_str(), vi["seq"].as_u64()), (Some("instantaneo"), Some(99)));
    assert!(ins.as_str().ends_with(&format!("{ARQ}}}")));
    assert!(Quadro::novo("x".into(), "x".into(), Bytes::from_static(b"{\"sem\":1}"), 1, None).m().is_none());
}

/// Thread de difusão: versões sobem, só o último valor fica, eventos e estado.
#[test]
fn curva_um_ponto_por_passo() {
    let (h, _rx) = Hub::novo(Opcoes::padrao("127.0.0.1:0"));
    let t: Arc<str> = "br-c0001".into();
    let pub_ = |hms: &str, ap: u64, a: f64| format!(r#"{{"ultimo_bu":"2026-10-04T{hms}-03:00","secoes":{{"total":100,"apuradas":{ap}}},"cand":[{{"n":22,"pct":{a}}},{{"n":13,"pct":null}}]}}"#);
    for (hms, ap, a) in [("18:00:00", 1, 50.0), ("18:00:20", 2, 51.0), ("18:00:50", 3, 52.0), ("18:01:10", 4, 53.0), ("18:01:30", 5, 54.0), ("17:00:00", 9, 9.0)] {
        h.amostrar_curva(&t, pub_(hms, ap, a).as_bytes());
    }
    let m = h.curvas.lock().unwrap();
    let v = &m[&t];
    // 18:00:00 fixo; 18:00:20 e 18:00:50 trocam o provisório; 18:01:10 (70 s depois do 1º) fixa um ponto novo;
    // 18:01:30 troca o provisório; 17:00 (antes do último) é ignorado
    assert_eq!(v.iter().map(|p| p.ap).collect::<Vec<_>>(), vec![1, 4, 5]);
    assert_eq!(v[2].cand, vec![(22, 54.0)]);
    assert_eq!(segundos_iso("2026-10-04T19:54:46-03:00"), urna_velox::metricas::segundos("04/10/2026 19:54:46"));
}

/// B-082: depois de um reinício (retomada), uma publicação sem `totalizado_em` (hora da noite, `ultimo_bu`) entrava
/// na curva e as seguintes, com `totalizado_em` (relógio do sistema, M-003), continuavam nela: o eixo misturava os
/// dois relógios (19:12 da noite e 10:50 de agora) e a curva ficava espremida na beirada. Uma curva tem um relógio
/// só: o do sistema vale mais (a noite que veio antes sai); depois dele, a hora da noite é ignorada.
#[test]
fn curva_b082_um_relogio_so() {
    let (h, _rx) = Hub::novo(Opcoes::padrao("127.0.0.1:0"));
    let t: Arc<str> = "br-c0001".into();
    let noite = |ap: u64| format!(r#"{{"ultimo_bu":"2026-10-04T19:12:09-03:00","secoes":{{"total":100,"apuradas":{ap}}},"cand":[{{"n":22,"pct":50.0}}]}}"#);
    let sist = |hms: &str, ap: u64| format!(r#"{{"ultimo_bu":"2026-10-04T19:13:00-03:00","totalizado_em":"2026-10-30T{hms}.000-03:00","secoes":{{"total":100,"apuradas":{ap}}},"cand":[{{"n":22,"pct":50.0}}]}}"#);
    h.amostrar_curva(&t, noite(10).as_bytes());
    h.amostrar_curva(&t, sist("09:54:12", 11).as_bytes());
    h.amostrar_curva(&t, sist("09:55:20", 12).as_bytes());
    h.amostrar_curva(&t, noite(13).as_bytes());
    h.amostrar_curva_superada("br-c0001-e006257.json", noite(14).as_bytes());
    let m = h.curvas.lock().unwrap();
    let v = &m[&t];
    let dia = |p: &PontoCurva| (p.t / 86400.0).floor() as i64;
    assert!(v.iter().all(|p| dia(p) == dia(&v[0])), "relógios misturados na curva: {:?}", v.iter().map(|p| (p.t, p.ap)).collect::<Vec<_>>());
    assert_eq!(v.iter().map(|p| p.ap).collect::<Vec<_>>(), vec![11, 12]);
}

/// Rajada (retotalização do replay): as versões superadas fecham os pontos; a curva fica igual à de publicações
/// uma a uma.
#[test]
fn curva_com_versoes_superadas() {
    let pub_ = |s: u32| format!(r#"{{"v":1,"seq":{s},"ultimo_bu":"2026-10-04T18:{:02}:{:02}-03:00","secoes":{{"total":100,"apuradas":{s}}},"cand":[{{"n":22,"pct":50.0}}]}}"#, s / 6, (s % 6) * 10);
    let (a, _ra) = Hub::novo(Opcoes::padrao("127.0.0.1:0"));
    let (b, _rb) = Hub::novo(Opcoes::padrao("127.0.0.1:0"));
    let t: Arc<str> = "br-c0001".into();
    for s in 0..=30 {
        a.amostrar_curva(&t, pub_(s).as_bytes());
        if s < 30 { b.amostrar_curva_superada("br-c0001-e006257.json", pub_(s).as_bytes()); } else { b.amostrar_curva(&t, pub_(s).as_bytes()); }
    }
    let ap = |h: &Hub| h.curvas.lock().unwrap()[&t].iter().map(|p| p.ap).collect::<Vec<_>>();
    assert_eq!(ap(&b), ap(&a));
    assert_eq!(ap(&a).len(), 6, "{:?}", ap(&a));
    // sem `ultimo_bu` (null): nada
    b.amostrar_curva_superada("sp-c0003-e006257.json", br#"{"ultimo_bu":null}"#);
    assert!(!b.curvas.lock().unwrap().contains_key("sp-c0003"));
}

#[test]
fn difusao_guarda_o_ultimo_valor() {
    let (h, rx) = Hub::novo(Opcoes::padrao("127.0.0.1:0"));
    let hd = { let h = h.clone(); std::thread::spawn(move || difundir(h, rx)) };
    let arq = |seq: u64| Bytes::from(ARQ.replace("\"seq\":99", &format!("\"seq\":{seq}")));
    h.entregar(Entrada::Arquivos(vec![("br-c0001-e006257.json".into(), arq(10), None)]));
    h.entregar(Entrada::Arquivos(vec![("br-c0001-e006257.json".into(), arq(20), Some(agora_us()))]));
    h.entregar(Entrada::Eventos(vec![EventoBruto { classe: "matematicamente", seq: 20, horario: "2026-10-04 21:00:00".into(), sistema: None, abrangencia: Some("br".into()),
        cargo: Some(1), candidato: None, estado: "matematicamente_2t".into(), motivo: String::new() }]));
    h.entregar(Entrada::Estado(json!({"estado": "acompanhando", "encerrada": false, "pausa_429": {"ativa": false}, "conferencias": 0, "alertas": {}})));
    let t = Instant::now();
    while h.estado.borrow().is_none() && t.elapsed() < Duration::from_secs(5) { std::thread::sleep(Duration::from_millis(5)); }
    let q = h.topicos.read().unwrap()["br-c0001"].borrow().clone();
    // as duas entregas podem ser juntadas pela thread (só a mais nova vira mensagem): versão 1 ou 2
    let seq = q.m().unwrap().seq;
    assert!(seq == 20 && (1..=2).contains(&q.ver), "{seq} {}", q.ver);
    let e = h.eventos.read().unwrap()[0].clone();
    assert_eq!((e.n, e.topico.as_deref()), (0, Some("br-c0001")));
    assert_eq!(e.valor["candidato"], Value::Null);
    let est: Value = serde_json::from_slice(&h.estado.borrow().as_ref().unwrap().corpo).unwrap();
    assert_eq!(est["clientes"], 0);
    // a thread termina quando o canal fecha (o Hub guarda o lado de envio): fica solta até o fim do teste
    drop(hd);
}

type Ws = tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;
use tokio_tungstenite::tungstenite::Message as M;

/// Próxima mensagem de texto do tipo pedido (pula `estado` e outras), com o texto bruto.
async fn prox(ws: &mut Ws, tipo: &str) -> (Value, String) {
    loop {
        let m = tokio::time::timeout(Duration::from_secs(5), ws.next()).await.unwrap_or_else(|_| panic!("sem mensagem {tipo}")).expect("fechada").expect("erro");
        if let M::Text(t) = m {
            let v: Value = serde_json::from_str(t.as_str()).unwrap();
            if v["tipo"] == tipo { return (v, t.as_str().to_string()); }
        }
    }
}
/// Lê até o fechamento: o código e o motivo do close.
async fn ate_fechar(ws: &mut Ws) -> String {
    loop {
        match tokio::time::timeout(Duration::from_secs(5), ws.next()).await.expect("não fechou") {
            Some(Ok(M::Close(c))) => return c.map(|c| format!("{} {}", u16::from(c.code), c.reason)).unwrap_or_default(),
            Some(Ok(_)) => continue,
            Some(Err(e)) => return format!("erro {e}"),
            None => return "fim".into(),
        }
    }
}
fn resultado(bruto: &str) -> &str { let i = bruto.find("\"resultado\":").unwrap() + 12; &bruto[i..bruto.len() - 1] }

/// Estáticos da página em gzip (M-017: o `mapa.json` com os locais de votação tem ~3 MB; nem o Caddy nem o
/// serviço comprimiam): com `Accept-Encoding: gzip` vem `Content-Encoding: gzip` e o corpo descomprime no original;
/// sem ele, o corpo de sempre; fonte (`woff2`, já comprimida) nunca; `Vary: Accept-Encoding` nos dois casos.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn estaticos_em_gzip() {
    use std::io::Read as _;
    let (h, rx) = Hub::novo(Opcoes::padrao("127.0.0.1:0"));
    { let h = h.clone(); std::thread::spawn(move || difundir(h, rx)); }
    let (atx, arx) = tokio::sync::oneshot::channel();
    let srv = tokio::spawn(servir(h.clone(), move |a| { let _ = atx.send(a); }));
    let end = arx.await.unwrap().to_string();
    let c = reqwest::Client::builder().no_gzip().build().unwrap();
    for caminho in ["/static/mapa.json", "/static/app.js"] {
        let orig = crate::pagina::estatico(caminho).unwrap().corpo;
        let r = c.get(format!("http://{end}{caminho}")).header("Accept-Encoding", "gzip, deflate, br").send().await.unwrap();
        assert_eq!(r.status().as_u16(), 200);
        assert_eq!(r.headers().get("content-encoding").map(|v| v.to_str().unwrap()), Some("gzip"), "{caminho} sem gzip");
        assert_eq!(r.headers().get("vary").map(|v| v.to_str().unwrap()), Some("Accept-Encoding"), "{caminho}: vary");
        let z = r.bytes().await.unwrap();
        assert!(z.len() * 2 < orig.len(), "{caminho}: {} → {} bytes", orig.len(), z.len());
        let mut d = vec![];
        flate2::read::GzDecoder::new(&z[..]).read_to_end(&mut d).unwrap();
        assert!(d == orig, "{caminho}: o gzip descomprime no original");
        let r = c.get(format!("http://{end}{caminho}")).send().await.unwrap();
        assert!(r.headers().get("content-encoding").is_none(), "{caminho}: sem Accept-Encoding, sem gzip");
        assert_eq!(r.headers().get("vary").map(|v| v.to_str().unwrap()), Some("Accept-Encoding"));
        assert!(r.bytes().await.unwrap() == orig);
        let r = c.get(format!("http://{end}{caminho}")).header("Accept-Encoding", "gzip;q=0, identity").send().await.unwrap();
        assert!(r.headers().get("content-encoding").is_none(), "{caminho}: gzip;q=0 recusa o gzip");
    }
    let r = c.get(format!("http://{end}/static/archivo.woff2")).header("Accept-Encoding", "gzip").send().await.unwrap();
    assert!(r.headers().get("content-encoding").is_none(), "woff2 já é comprimido");
    h.desligar();
    let _ = tokio::time::timeout(Duration::from_secs(5), srv).await;
}

/// Mapa das seções (`crate::mapa`, protocolo §4.8) num servidor de verdade: lista com ETag/304, fotografia, `op: mapa`
/// com o histórico a partir de `desde`, atualização ao vivo, outra época (reinício) e servidor sem mapa.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mapa_das_secoes_ponta_a_ponta() {
    use base64::Engine as _;
    let k = |mu: u64, z: u64, s: u64| mu << 32 | z << 16 | s;
    let (h, rx) = Hub::novo(Opcoes::padrao("127.0.0.1:0"));
    { let h = h.clone(); std::thread::spawn(move || difundir(h, rx)); }
    h.definir_mapa(crate::mapa::Mapa::novo(vec![(*b"sp", k(71072, 1, 10)), (*b"sp", k(71072, 1, 11)), (*b"rj", k(60011, 4, 2)), (*b"zz", k(29629, 1, 3014))]));
    let t17 = urna_velox::metricas::segundos("25/10/2026 17:01:00").unwrap();
    h.entregar(Entrada::Secoes(vec![(k(29629, 1, 3014), t17 - 12 * 3600), (k(71072, 1, 11), t17)]));
    let t = Instant::now();
    while h.mapa().unwrap().contagem().0 < 2 { assert!(t.elapsed() < Duration::from_secs(5)); tokio::time::sleep(Duration::from_millis(5)).await; }
    let (atx, arx) = tokio::sync::oneshot::channel();
    let srv = tokio::spawn(servir(h.clone(), move |a| { let _ = atx.send(a); }));
    let end = arx.await.unwrap().to_string();
    let c = reqwest::Client::new();
    let r = c.get(format!("http://{end}/api/v1/mapa/secoes")).send().await.unwrap();
    let etag = r.headers()["etag"].to_str().unwrap().to_string();
    let lista: Value = serde_json::from_str(&r.text().await.unwrap()).unwrap();
    assert_eq!(lista["n"], 4);
    assert_eq!(lista["m"], json!([29629, 2, 1, 1, 1, 3014, 60011, 0, 1, 4, 1, 2, 71072, 1, 1, 1, 2, 10, 1]));
    assert_eq!(c.get(format!("http://{end}/api/v1/mapa/secoes")).header("If-None-Match", &etag).send().await.unwrap().status().as_u16(), 304);
    let e: Value = serde_json::from_str(&c.get(format!("http://{end}/api/v1/mapa/estado")).send().await.unwrap().text().await.unwrap()).unwrap();
    assert_eq!((e["n"].as_u64(), e["total"].as_u64()), (Some(2), Some(4)));
    let hs = base64::engine::general_purpose::STANDARD.decode(e["h"].as_str().unwrap()).unwrap();
    assert_eq!(hs, vec![crate::mapa::faixa(t17 - 12 * 3600), 0, 0, 171]);
    let epoca = e["epoca"].as_u64().unwrap();
    // WebSocket: o que acendeu depois de `desde` e, ao vivo, o resto
    let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://{end}/ws")).await.unwrap();
    ws.send(M::text(format!(r#"{{"op":"mapa","desde":1,"epoca":{epoca}}}"#))).await.unwrap();
    let (v, _) = prox(&mut ws, "mapa").await;
    assert_eq!((v["de"].as_u64(), v["n"].as_u64()), (Some(1), Some(2)));
    assert_eq!(v["s"], json!([3, 171]));
    h.entregar(Entrada::Secoes(vec![(k(60011, 4, 2), t17 + 600), (k(71072, 1, 11), t17 + 900), (k(1, 1, 1), t17)]));
    let (v, _) = prox(&mut ws, "mapa").await;
    assert_eq!((v["de"].as_u64(), v["n"].as_u64()), (Some(2), Some(3)));
    assert_eq!(v["s"], json!([1, 172]));
    assert_eq!(h.mapa().unwrap().contagem(), (3, 1), "a seção fora do catálogo é contada à parte");
    // outra época: o cliente é avisado para buscar a fotografia de novo
    ws.send(M::text(r#"{"op":"mapa","desde":3,"epoca":1}"#)).await.unwrap();
    let (v, _) = prox(&mut ws, "mapa").await;
    assert_eq!(v["reiniciar"], true);
    // sem mapa definido: erro e 503
    let (h2, rx2) = Hub::novo(Opcoes::padrao("127.0.0.1:0"));
    { let h2 = h2.clone(); std::thread::spawn(move || difundir(h2, rx2)); }
    let (atx2, arx2) = tokio::sync::oneshot::channel();
    let srv2 = tokio::spawn(servir(h2.clone(), move |a| { let _ = atx2.send(a); }));
    let end2 = arx2.await.unwrap().to_string();
    assert_eq!(c.get(format!("http://{end2}/api/v1/mapa/estado")).send().await.unwrap().status().as_u16(), 503);
    let (mut ws2, _) = tokio_tungstenite::connect_async(format!("ws://{end2}/ws")).await.unwrap();
    ws2.send(M::text(r#"{"op":"mapa","desde":0}"#)).await.unwrap();
    assert_eq!(prox(&mut ws2, "erro").await.0["motivo"], "mapa_indisponivel");
    h.desligar();
    h2.desligar();
    let _ = tokio::time::timeout(Duration::from_secs(5), srv).await;
    let _ = tokio::time::timeout(Duration::from_secs(5), srv2).await;
}

/// Multipleito (1.1): `GET /consulta/*` serve as séries de consulta (gzip pré-comprimido, cache) e só nomes simples.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn consulta_estatica_com_gzip() {
    use std::io::Write as _;
    let dir = std::env::temp_dir().join(format!("uv-difusao-consulta-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("3237/serie/t")).unwrap();
    std::fs::write(dir.join("indice.json"), "{\"pleitos\":[{\"pleito\":3237}]}").unwrap();
    std::fs::write(dir.join("3237/serie/t/rr-c0003.json"), "{\"v\":1}").unwrap();
    let mut g = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    g.write_all(b"{\"v\":1}").unwrap();
    std::fs::write(dir.join("3237/serie/t/rr-c0003.json.gz"), g.finish().unwrap()).unwrap();
    let mut op = Opcoes::padrao("127.0.0.1:0");
    op.consulta = Some(dir.clone());
    let (h, rx) = Hub::novo(op);
    { let h = h.clone(); std::thread::spawn(move || difundir(h, rx)); }
    let (atx, arx) = tokio::sync::oneshot::channel();
    let srv = tokio::spawn(servir(h.clone(), move |a| { let _ = atx.send(a); }));
    let end = arx.await.unwrap().to_string();
    let c = reqwest::Client::builder().no_gzip().build().unwrap();
    let r = c.get(format!("http://{end}/consulta/indice.json")).send().await.unwrap();
    assert_eq!(r.status().as_u16(), 200);
    assert_eq!(r.headers()["cache-control"], "public, max-age=300");
    let r = c.get(format!("http://{end}/consulta/3237/serie/t/rr-c0003.json")).header("Accept-Encoding", "gzip").send().await.unwrap();
    assert_eq!(r.headers().get("content-encoding").map(|v| v.to_str().unwrap()), Some("gzip"));
    let r = c.get(format!("http://{end}/consulta/3237/serie/t/rr-c0003.json")).send().await.unwrap();
    assert_eq!(r.headers()["cache-control"], "public, max-age=3600");
    assert_eq!(r.text().await.unwrap(), "{\"v\":1}");
    std::fs::write(dir.join("3237/serie/EM_CURSO"), "").unwrap();
    let r = c.get(format!("http://{end}/consulta/3237/serie/t/rr-c0003.json")).send().await.unwrap();
    assert_eq!(r.headers()["cache-control"], "public, max-age=30", "série em curso: cache curto");
    for ruim in ["/consulta/../segredo.json", "/consulta/3237/serie/t/rr-c0003.json.gz", "/consulta/.x.json", "/consulta/x%2F..%2Fy.json"] {
        let r = c.get(format!("http://{end}{ruim}")).send().await.unwrap();
        assert_eq!(r.status().as_u16(), 404, "{ruim}");
    }
    srv.abort();
    let _ = std::fs::remove_dir_all(&dir);
}

/// Multipleito (1.1): `GET /api/v1/pleitos` serve o `pleitos.json` (ETag/304), relê quando o arquivo muda e dá 404
/// sem ele ou com um arquivo que não é a lista.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rest_pleitos_relido_quando_muda() {
    let dir = std::env::temp_dir().join(format!("uv-difusao-pleitos-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let arq = dir.join("pleitos.json");
    let mut op = Opcoes::padrao("127.0.0.1:0");
    op.pleitos = Some(arq.clone());
    let (h, _rx) = Hub::novo(op);
    let (atx, arx) = tokio::sync::oneshot::channel();
    let srv = tokio::spawn(servir(h.clone(), move |a| { let _ = atx.send(a); }));
    let end = arx.await.unwrap().to_string();
    let c = reqwest::Client::new();
    let url = format!("http://{end}/api/v1/pleitos");
    assert_eq!(c.get(&url).send().await.unwrap().status().as_u16(), 404, "sem o arquivo");
    std::fs::write(&arq, "{\"nada\":1}").unwrap();
    assert_eq!(c.get(&url).send().await.unwrap().status().as_u16(), 404, "não é a lista");
    let um = r#"{"versao":1,"pleitos":[{"codigo":3221}]}"#;
    std::fs::write(&arq, um).unwrap();
    let r = c.get(&url).send().await.unwrap();
    assert_eq!(r.status().as_u16(), 200);
    let etag = r.headers()["etag"].to_str().unwrap().to_string();
    assert_eq!(r.text().await.unwrap(), um);
    let r = c.get(&url).header("If-None-Match", &etag).send().await.unwrap();
    assert_eq!(r.status().as_u16(), 304);
    // o uv-coletor pleitos regrava o arquivo (rename): a resposta muda sem reiniciar o serviço
    tokio::time::sleep(Duration::from_millis(20)).await;
    let dois = r#"{"versao":1,"pleitos":[{"codigo":3221},{"codigo":3250}]}"#;
    crate::pleitos::gravar_atomico(&arq, dois.as_bytes()).unwrap();
    let r = c.get(&url).header("If-None-Match", &etag).send().await.unwrap();
    assert_eq!(r.status().as_u16(), 200);
    assert_eq!(r.text().await.unwrap(), dois);
    h.desligar();
    let _ = srv.await;
    let _ = std::fs::remove_dir_all(&dir);
}

/// Protocolo de ponta a ponta num servidor de verdade (porta livre): REST com ETag/304, inscrição → instantâneo,
/// atualização byte a byte, seq que só sobe, eventos, erros e limites (tópicos, taxa, clientes) e o desligamento.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn protocolo_ponta_a_ponta() {
    let mut op = Opcoes::padrao("127.0.0.1:0");
    (op.max_topicos, op.max_clientes, op.msgs_s, op.rajada, op.excessos) = (2, 3, 20.0, 20.0, 10);
    let (h, rx) = Hub::novo(op);
    { let h = h.clone(); std::thread::spawn(move || difundir(h, rx)); }
    let arq = |seq: u64| ARQ.replace("\"seq\":99", &format!("\"seq\":{seq}"));
    h.entregar(Entrada::Arquivos(["br-c0001-e006257.json", "sp-c0001-e006257.json", "rj-c0001-e006257.json"].iter().map(|n| (n.to_string(), Bytes::from(arq(10)), None)).collect()));
    let t = Instant::now();
    while h.n_topicos() < 3 { assert!(t.elapsed() < Duration::from_secs(5)); tokio::time::sleep(Duration::from_millis(5)).await; }
    let (atx, arx) = tokio::sync::oneshot::channel();
    let srv = tokio::spawn(servir(h.clone(), move |a| { let _ = atx.send(a); }));
    let end = arx.await.unwrap().to_string();
    // REST
    let c = reqwest::Client::new();
    let get = |p: &str| c.get(format!("http://{end}{p}")).send();
    assert_eq!(get("/api/v1/estado").await.unwrap().status().as_u16(), 503, "estado antes do primeiro cálculo");
    let tops: Value = serde_json::from_str(&get("/api/v1/topicos").await.unwrap().text().await.unwrap()).unwrap();
    assert_eq!(tops.as_array().unwrap().len(), 3);
    let r = get("/api/v1/resultado/br-c0001").await.unwrap();
    assert_eq!(r.headers()["etag"], "\"10\"");
    assert_eq!(r.headers()["access-control-allow-origin"], "*");
    assert_eq!(r.text().await.unwrap(), arq(10), "mesmos bytes do arquivo");
    let r = c.get(format!("http://{end}/api/v1/resultado/br-c0001")).header("If-None-Match", "\"10\"").send().await.unwrap();
    assert_eq!(r.status().as_u16(), 304);
    for p in ["/api/v1/resultado/xx-c0001", "/api/v1/resultado/..%2Fsaude.json", "/saude.json", "/api/v1/resultado/br-c0001-e006257.json"] {
        assert_eq!(get(p).await.unwrap().status().as_u16(), 404, "{p}");
    }
    assert_eq!(c.post(format!("http://{end}/api/v1/resultado/br-c0001")).send().await.unwrap().status().as_u16(), 405);
    // WebSocket: inscrição, instantâneo e atualização com os bytes do arquivo
    let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://{end}/ws")).await.unwrap();
    ws.send(M::text(r#"{"op":"inscrever","topicos":["br-c0001"]}"#)).await.unwrap();
    let (v, b) = prox(&mut ws, "instantaneo").await;
    assert_eq!((v["topico"].as_str(), v["seq"].as_u64(), v["secoes_apuradas"].as_u64()), (Some("br-c0001"), Some(10), Some(40)));
    assert_eq!(resultado(&b), arq(10));
    h.entregar(Entrada::Arquivos(vec![("br-c0001-e006257.json".into(), Bytes::from(arq(20)), Some(agora_us()))]));
    let (v, b) = prox(&mut ws, "atualizacao").await;
    assert_eq!(v["seq"].as_u64(), Some(20));
    assert!(v["t_bu"].as_u64().unwrap() <= v["t_envio"].as_u64().unwrap());
    assert_eq!(resultado(&b), arq(20));
    // limite de tópicos (2), tópico desconhecido, JSON inválido, op desconhecida, binário
    ws.send(M::text(r#"{"op":"inscrever","topicos":["sp-c0001","rj-c0001"]}"#)).await.unwrap();
    let (v, _) = prox(&mut ws, "instantaneo").await;
    assert_eq!(v["topico"], "sp-c0001");
    assert_eq!(prox(&mut ws, "erro").await.0["motivo"], "limite_topicos");
    ws.send(M::text(r#"{"op":"cancelar","topicos":["sp-c0001"]}"#)).await.unwrap();
    ws.send(M::text(r#"{"op":"inscrever","topicos":["../../etc/passwd"]}"#)).await.unwrap();
    assert_eq!(prox(&mut ws, "erro").await.0["motivo"], "topico_desconhecido");
    ws.send(M::text("isto não é json")).await.unwrap();
    assert_eq!(prox(&mut ws, "erro").await.0["motivo"], "json_invalido");
    ws.send(M::text(r#"{"op":"apagar"}"#)).await.unwrap();
    assert_eq!(prox(&mut ws, "erro").await.0["motivo"], "op_desconhecida");
    ws.send(M::binary(vec![1u8])).await.unwrap();
    assert_eq!(prox(&mut ws, "erro").await.0["motivo"], "binario");
    // evento da circunscrição inscrita; estado
    h.entregar(Entrada::Eventos(vec![EventoBruto { classe: "matematicamente", seq: 20, horario: "2026-10-04 21:00:00".into(), sistema: None, abrangencia: Some("br".into()),
        cargo: Some(1), candidato: Some(13), estado: "2t".into(), motivo: String::new() }]));
    let (v, _) = prox(&mut ws, "evento").await;
    assert_eq!((v["estado"].as_str(), v["candidato"].as_u64(), v["topico"].as_str()), (Some("2t"), Some(13), Some("br-c0001")));
    h.entregar(Entrada::Estado(json!({"estado": "acompanhando", "encerrada": false, "pausa_429": {"ativa": false}, "conferencias": 0, "alertas": {"http429": 1}})));
    assert_eq!(prox(&mut ws, "estado").await.0["alertas"]["http429"], 1);
    assert_eq!(get("/api/v1/estado").await.unwrap().status().as_u16(), 200);
    // taxa: rajada de 20, a 10ª mensagem acima do limite derruba (1008 taxa)
    let (mut w2, _) = tokio_tungstenite::connect_async(format!("ws://{end}/ws")).await.unwrap();
    for _ in 0..80 { if w2.send(M::text(r#"{"op":"cancelar","topicos":[]}"#)).await.is_err() { break; } }
    assert_eq!(prox(&mut w2, "erro").await.0["motivo"], "taxa");
    assert_eq!(ate_fechar(&mut w2).await, "1008 taxa");
    // mensagem maior que 4 KB: fechamento 1009
    let (mut w3, _) = tokio_tungstenite::connect_async(format!("ws://{end}/ws")).await.unwrap();
    let _ = w3.send(M::text(format!("{{\"op\":\"{}\"}}", "x".repeat(10_000)))).await;
    assert!(ate_fechar(&mut w3).await.starts_with("1009"), "mensagem grande");
    // máximo de clientes (3): a 4ª conexão simultânea recebe 503 (as duas derrubadas acima já saíram da conta)
    let t = Instant::now();
    while h.med.clientes.load(Relaxed) != 1 { assert!(t.elapsed() < Duration::from_secs(5)); tokio::time::sleep(Duration::from_millis(5)).await; }
    let mut abertas = vec![];
    for _ in 0..2 { abertas.push(tokio_tungstenite::connect_async(format!("ws://{end}/ws")).await.unwrap().0); }
    let t = Instant::now();
    while h.med.clientes.load(Relaxed) < 3 { assert!(t.elapsed() < Duration::from_secs(5)); tokio::time::sleep(Duration::from_millis(5)).await; }
    let e = tokio_tungstenite::connect_async(format!("ws://{end}/ws")).await.err().expect("a 4ª conexão devia ser recusada");
    assert!(e.to_string().contains("503"), "{e}");
    // ninguém afetado: a primeira conexão continua recebendo, com seq que só sobe
    h.entregar(Entrada::Arquivos(vec![("br-c0001-e006257.json".into(), Bytes::from(arq(30)), Some(agora_us()))]));
    assert_eq!(prox(&mut ws, "atualizacao").await.0["seq"], 30);
    // desligamento: aviso e close 1001 para todos
    h.desligar();
    assert_eq!(prox(&mut ws, "aviso").await.0["motivo"], "desligamento");
    assert_eq!(ate_fechar(&mut ws).await, "1001 desligamento");
    for mut w in abertas { assert_eq!(ate_fechar(&mut w).await, "1001 desligamento"); }
    tokio::time::timeout(Duration::from_secs(5), srv).await.expect("servir não terminou").unwrap().unwrap();
    let d = h.med.desconexoes.lock().unwrap().clone();
    assert_eq!((d.get("taxa"), d.get("mensagem_grande"), d.get("desligamento")), (Some(&1), Some(&1), Some(&3)), "{d:?}");
    assert_eq!(h.med.recusadas.load(Relaxed), 1);
}
/// Sonda que passou pelo motor e pela gravação até 0,8 ms antes de `agora` (injetada 1 ms antes).
fn corpo_sonda(id: u64, local: bool, agora: u64) -> Value {
    let t_inj = agora - 1_000;
    let mut s = crate::sonda::Sonda::nova(id, 1, t_inj);
    if !local { s = crate::sonda::Sonda::remota("no-b", id, 1, t_inj, t_inj + 10); }
    s.t_motor = Some(t_inj + 100);
    s.t_gravacao = Some(t_inj + 200);
    s.json()
}

/// Sondas na difusão: carimbo `t_difusao`, guarda só as últimas [`SONDAS_GUARDADAS`], contador por sonda.
#[test]
fn sondas_guardadas_e_rest() {
    let (h, _rx) = Hub::novo(Opcoes::padrao("127.0.0.1:0"));
    assert_eq!(h.sondas_json()["ultimas"], json!([]));
    h.guardar_sondas(vec![]);
    assert_eq!(*h.n_sondas.borrow(), 0, "lista vazia não acorda ninguém");
    let t = agora_us();
    h.guardar_sondas((1..=SONDAS_GUARDADAS as u64 + 5).map(|i| corpo_sonda(i, true, t)).collect());
    assert_eq!(*h.n_sondas.borrow(), SONDAS_GUARDADAS as u64 + 5);
    let j = h.sondas_json();
    let u = j["ultimas"].as_array().unwrap();
    assert_eq!(u.len(), SONDAS_GUARDADAS);
    assert_eq!((u[0]["id"].as_u64(), u.last().unwrap()["id"].as_u64()), (Some(6), Some(SONDAS_GUARDADAS as u64 + 5)));
    assert!(u[0]["t_difusao"].as_u64().unwrap() >= t);
    assert_eq!(j["enviadas"], 0);
    let m: Value = serde_json::from_str(msg_sonda(&u[0], true).as_str()).unwrap();
    assert_eq!((m["tipo"].as_str(), m["instantaneo"].as_bool()), (Some("sonda"), Some(true)));
    assert!(m["t_envio"].as_u64().unwrap() >= m["t_difusao"].as_u64().unwrap());
    assert_eq!(h.saude_json()["sondas_enviadas"], 0);
    assert_eq!(h.saude_json()["latencias_ms"]["sonda_envio"]["n"], 0);
}

/// `op: sonda` num servidor de verdade: só quem pede recebe; a inscrição traz a última (`instantaneo`); as novas
/// chegam na ordem; desligar para; as dos pares chegam marcadas e não entram na latência deste serviço; REST.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sonda_pelo_websocket() {
    let (h, rx) = Hub::novo(Opcoes::padrao("127.0.0.1:0"));
    { let h = h.clone(); std::thread::spawn(move || difundir(h, rx)); }
    h.entregar(Entrada::Arquivos(vec![("br-c0001-e006257.json".into(), Bytes::from_static(ARQ.as_bytes()), None)]));
    let t = Instant::now();
    while h.n_topicos() < 1 { assert!(t.elapsed() < Duration::from_secs(5)); tokio::time::sleep(Duration::from_millis(5)).await; }
    let (atx, arx) = tokio::sync::oneshot::channel();
    let srv = tokio::spawn(servir(h.clone(), move |a| { let _ = atx.send(a); }));
    let end = arx.await.unwrap().to_string();
    // sem nenhuma sonda ainda: ligar não manda nada
    let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://{end}/ws")).await.unwrap();
    let (mut outro, _) = tokio_tungstenite::connect_async(format!("ws://{end}/ws")).await.unwrap();
    outro.send(M::text(r#"{"op":"inscrever","topicos":["br-c0001"]}"#)).await.unwrap();
    prox(&mut outro, "instantaneo").await;
    ws.send(M::text(r#"{"op":"sonda"}"#)).await.unwrap();
    // ida e volta (um erro) para o `op: sonda` já estar valendo antes da entrega
    ws.send(M::text(r#"{"op":"apagar"}"#)).await.unwrap();
    prox(&mut ws, "erro").await;
    h.entregar(Entrada::Sonda(vec![corpo_sonda(1, true, agora_us())]));
    let (v, _) = prox(&mut ws, "sonda").await;
    assert_eq!((v["id"].as_u64(), v["instantaneo"].as_bool(), v["local"].as_bool()), (Some(1), Some(false), Some(true)));
    let ts: Vec<u64> = ["t_injecao", "t_motor", "t_gravacao", "t_difusao", "t_envio"].iter().map(|k| v[*k].as_u64().unwrap()).collect();
    assert!(ts.windows(2).all(|w| w[0] <= w[1]), "carimbos em ordem: {ts:?}");
    // duas de uma vez (e uma do par): todas, na ordem
    h.entregar(Entrada::Sonda(vec![corpo_sonda(2, true, agora_us()), corpo_sonda(9, false, agora_us()), corpo_sonda(3, true, agora_us())]));
    let ids: Vec<(u64, bool)> = { let mut v = vec![]; for _ in 0..3 { let (m, _) = prox(&mut ws, "sonda").await; v.push((m["id"].as_u64().unwrap(), m["local"].as_bool().unwrap())); } v };
    assert_eq!(ids, vec![(2, true), (9, false), (3, true)]);
    // nova conexão: a inscrição traz a última guardada com instantaneo = true
    let (mut w2, _) = tokio_tungstenite::connect_async(format!("ws://{end}/ws")).await.unwrap();
    w2.send(M::text(r#"{"op":"sonda"}"#)).await.unwrap();
    let (v, _) = prox(&mut w2, "sonda").await;
    assert_eq!((v["id"].as_u64(), v["instantaneo"].as_bool()), (Some(3), Some(true)));
    // desligar: a próxima não chega a ws, chega a w2
    ws.send(M::text(r#"{"op":"sonda","ligar":false}"#)).await.unwrap();
    // garante que o desligar foi processado antes da próxima entrega (ida e volta por um erro)
    ws.send(M::text(r#"{"op":"apagar"}"#)).await.unwrap();
    prox(&mut ws, "erro").await;
    h.entregar(Entrada::Sonda(vec![corpo_sonda(4, true, agora_us())]));
    assert_eq!(prox(&mut w2, "sonda").await.0["id"], 4);
    h.entregar(Entrada::Arquivos(vec![("br-c0001-e006257.json".into(), Bytes::from(ARQ.replace("\"seq\":99", "\"seq\":100")), None)]));
    // ws está inscrita em nada: a próxima mensagem de ws não pode ser sonda; outro (só resultado) nunca vê sonda
    h.entregar(Entrada::Estado(json!({"estado": "acompanhando", "encerrada": false, "pausa_429": {"ativa": false}, "conferencias": 0, "alertas": {}})));
    loop {
        let m = tokio::time::timeout(Duration::from_secs(5), ws.next()).await.unwrap().unwrap().unwrap();
        if let M::Text(t) = m { let v: Value = serde_json::from_str(t.as_str()).unwrap(); assert_ne!(v["tipo"], "sonda"); if v["tipo"] == "estado" { break; } }
    }
    loop {
        let m = tokio::time::timeout(Duration::from_secs(5), outro.next()).await.unwrap().unwrap().unwrap();
        if let M::Text(t) = m { let v: Value = serde_json::from_str(t.as_str()).unwrap(); assert_ne!(v["tipo"], "sonda", "a página não pede sonda"); if v["tipo"] == "estado" { break; } }
    }
    // medidas: enviadas = 4 (ws) + 1 (w2, a 4; a instantânea não conta); latência só das locais (3 + 1)
    assert_eq!(h.med.sondas_enviadas.load(Relaxed), 5);
    assert_eq!(h.med.lat_sonda.lock().unwrap().n, 4);
    // REST
    let r = reqwest::get(format!("http://{end}/api/v1/sonda")).await.unwrap();
    assert_eq!(r.headers()["cache-control"], "no-cache");
    assert_eq!(r.headers()["access-control-allow-origin"], "*");
    let j: Value = serde_json::from_str(&r.text().await.unwrap()).unwrap();
    assert_eq!(j["ultimas"].as_array().unwrap().iter().map(|x| x["id"].as_u64().unwrap()).collect::<Vec<_>>(), vec![1, 2, 9, 3, 4]);
    assert_eq!(j["enviadas"], 5);
    h.desligar();
    tokio::time::timeout(Duration::from_secs(5), srv).await.expect("servir não terminou").unwrap().unwrap();
}
