use super::*;

fn k(mu: u64, z: u64, s: u64) -> u64 { mu << 32 | z << 16 | s }

#[test]
fn faixas() {
    let t = urna_velox::metricas::segundos("25/10/2026 00:00:00").unwrap();
    assert_eq!(faixa(t), 1);
    assert_eq!(faixa(t + 359), 1);
    assert_eq!(faixa(t + 360), 2);
    assert_eq!(faixa(t + 17 * 3600), 171);
    assert_eq!(faixa(t + 86_399), 240);
}

#[test]
fn lista_indices_e_mensagens() {
    let m = Mapa::novo(vec![(*b"sp", k(71072, 1, 20)), (*b"ac", k(1120, 9, 5)), (*b"sp", k(71072, 1, 10)), (*b"sp", k(71072, 2, 7)), (*b"zz", k(29629, 1, 3014)),
        (*b"sp", k(71072, 1, 10))]);
    assert_eq!(m.total(), 5, "duplicada sai");
    let j: serde_json::Value = serde_json::from_slice(&m.secoes).unwrap();
    assert_eq!(j["n"], 5);
    assert_eq!(j["ufs"], serde_json::json!(["ac", "sp", "zz"]));
    // ordem da chave: 1120 (ac), 29629 (zz), 71072 (sp): zona 1 com 10 e 20 (diferença 10), zona 2 com 7
    assert_eq!(j["m"], serde_json::json!([1120, 0, 1, 9, 1, 5, 29629, 2, 1, 1, 1, 3014, 71072, 1, 2, 1, 2, 10, 10, 2, 1, 7]));
    let t = urna_velox::metricas::segundos("25/10/2026 17:03:00").unwrap();
    assert_eq!(m.aplicar(&[(k(71072, 1, 20), t), (k(99999, 1, 1), t), (k(1120, 9, 5), t + 3600)]), 2);
    assert_eq!(m.aplicar(&[(k(71072, 1, 20), t + 7200)]), 0, "a seção já acesa não muda");
    assert_eq!(m.contagem(), (2, 1));
    assert_eq!(*m.n.borrow(), 2);
    let (n, v) = m.mensagens(0);
    assert_eq!((n, v.len()), (2, 1));
    let msg: serde_json::Value = serde_json::from_str(&v[0]).unwrap();
    assert_eq!(msg["tipo"], "mapa");
    assert_eq!((msg["de"].as_u64(), msg["n"].as_u64()), (Some(0), Some(2)));
    assert_eq!(msg["s"], serde_json::json!([3, 171, 0, 181]));
    assert!(m.mensagens(2).1.is_empty());
    let e: serde_json::Value = serde_json::from_slice(&m.estado()).unwrap();
    assert_eq!((e["n"].as_u64(), e["total"].as_u64(), e["fora"].as_u64()), (Some(2), Some(5), Some(1)));
    let h = base64::engine::general_purpose::STANDARD.decode(e["h"].as_str().unwrap()).unwrap();
    assert_eq!(h, vec![181, 0, 0, 171, 0]);
}

/// B-064: o total de seções do mapa é o mesmo da página (`secoes.total` do resultado): sem as agregadas do
/// `candidatos.json` e sem as que se revelam agregadas na noite (`cs.json`/sondagem, B-051).
#[test]
fn b064_total_do_mapa_igual_ao_da_pagina() {
    use urna_velox::candidatos::{ArquivoCandidatos, Candidatos};
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../motor/tests/fixtures/candidatos_exemplo.json");
    let (_, mut arq) = Candidatos::ler(&p).expect("ler candidatos_exemplo.json");
    arq.secoes[0].agregada = true;
    let arq: ArquivoCandidatos = serde_json::from_str(&serde_json::to_string(&arq).unwrap()).unwrap();
    let cad = Candidatos::do_arquivo(&arq);
    let cat = urna_velox::catalogo::Catalogo::de_arquivo_candidatos(&arq).expect("catálogo");
    let est = urna_velox::estado::Estado::novo(&cat);
    let br = urna_velox::oficial::Abrangencia::Brasil;
    let mut pubr = urna_velox::resultado::Publicador::novo(&cat, Some(&cad), None);
    let m = Mapa::novo(chaves_do_catalogo(&cat, &cad));
    let total = |m: &Mapa| serde_json::from_slice::<serde_json::Value>(&m.estado()).unwrap()["total"].as_u64().unwrap();
    let lista: serde_json::Value = serde_json::from_slice(&m.secoes).unwrap();
    assert_eq!(pubr.secoes(&est, &br).0, 1, "a página conta 1 seção (a outra é agregada)");
    assert_eq!(lista["n"].as_u64(), Some(1), "lista do mapa sem a agregada do candidatos.json");
    assert_eq!(total(&m), 1, "total do mapa sem a agregada do candidatos.json");
    // agregada descoberta na noite: sai do total da página e do mapa
    let s = &arq.secoes;
    let uf: [u8; 2] = arq.municipios.iter().find(|x| x.codigo == s[1].municipio).unwrap().uf.as_bytes().try_into().unwrap();
    let k1 = urna_velox::chave(s[1].municipio, s[1].zona, s[1].secao);
    assert!(pubr.agregada(uf, k1).is_some());
    assert!(m.retirar(k1), "a seção sai do mapa");
    assert!(!m.retirar(k1), "uma vez só");
    std::thread::sleep(std::time::Duration::from_millis(FOTO_MS + 50));   // a fotografia é refeita no máximo a cada FOTO_MS
    assert_eq!(total(&m), pubr.secoes(&est, &br).0, "total do mapa = total da página depois da agregada da noite");
    let e: serde_json::Value = serde_json::from_slice(&m.estado()).unwrap();
    assert_eq!(e["ret"], serde_json::json!([0]), "a fotografia diz qual índice saiu");
    // a retirada que vem com BU volta ao total (a página também a conta: total nunca abaixo das apuradas)
    let t = urna_velox::metricas::segundos("25/10/2026 17:03:00").unwrap();
    assert_eq!(m.aplicar(&[(k1, t)]), 1);
    std::thread::sleep(std::time::Duration::from_millis(FOTO_MS + 50));
    assert_eq!(total(&m), 1);
    let msg: serde_json::Value = serde_json::from_str(&m.mensagens(0).1[0]).unwrap();
    assert_eq!(msg["total"].as_u64(), Some(1), "a mensagem mapa leva o total");
}

#[test]
fn historico_em_varias_mensagens() {
    let chaves: Vec<([u8; 2], u64)> = (0..(MAX_POR_MSG as u64 * 2 + 5)).map(|i| (*b"mg", k(41238, 1 + i / 60000, 1 + i % 60000))).collect();
    let m = Mapa::novo(chaves.clone());
    let t = urna_velox::metricas::segundos("25/10/2026 18:00:00").unwrap();
    m.aplicar(&chaves.iter().map(|x| (x.1, t)).collect::<Vec<_>>());
    let (n, v) = m.mensagens(3);
    assert_eq!(n, chaves.len());
    assert_eq!(v.len(), 3);
    let ult: serde_json::Value = serde_json::from_str(v.last().unwrap()).unwrap();
    assert_eq!(ult["n"].as_u64(), Some(n as u64));
}
