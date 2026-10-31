use super::*;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};

/// "TSE" mínimo em 127.0.0.1: responde pelo nome do arquivo (último trecho do caminho); 404 se não tiver.
fn servidor(arqs: Arc<Mutex<HashMap<String, String>>>) -> String {
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let end = format!("http://{}", l.local_addr().unwrap());
    std::thread::spawn(move || for c in l.incoming() {
        let Ok(mut c) = c else { continue };
        let (mut buf, mut b) = (vec![], [0u8; 4096]);
        while !buf.windows(4).any(|w| w == b"\r\n\r\n") {
            match c.read(&mut b) { Ok(0) | Err(_) => break, Ok(n) => buf.extend_from_slice(&b[..n]) }
        }
        let req = String::from_utf8_lossy(&buf);
        let caminho = req.split_whitespace().nth(1).unwrap_or("").split('?').next().unwrap_or("");
        let nome = caminho.rsplit('/').next().unwrap_or("").to_string();
        let resp = match arqs.lock().unwrap().get(&nome).cloned() {
            Some(x) => format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{x}", x.len()),
            None => "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_string(),
        };
        let _ = c.write_all(resp.as_bytes());
    });
    end
}

/// `-u.json` pré-apuração de Presidente (2º turno) com o eleitorado `te` e as seções `ts`.
fn u_json(te: u64, ts: u64) -> String {
    format!(r#"{{"ele":"6258","t":"2","tf":"n","dg":"13/10/2026","hg":"10:56:48","carg":[{{"cd":"1","nv":"1","agr":[{{"par":[{{"sg":"X","cand":[{{"n":"13","nmu":"A","dt":"01/01/1950"}},{{"n":"22","nmu":"B","dt":"01/01/1960"}}]}}]}}]}}],"e":{{"te":"{te}"}},"s":{{"ts":"{ts}"}},"v":{{}}}}"#)
}

/// Eleitorado de uma abrangência de Presidente no `candidatos.json` gravado (circunscrição `br` ou abrangência).
fn eleitorado(saida: &Path, abr: &str) -> (u64, u64) {
    let v: Value = serde_json::from_slice(&std::fs::read(saida.join("candidatos.json")).unwrap()).unwrap();
    let c = &v["eleicoes"][0]["cargos"][0];
    let circ = c["circunscricoes"].as_array().unwrap().iter().find(|x| x["abr"] == "br").map(|x| x["eleitorado"].as_u64().unwrap()).unwrap_or(0);
    let a = c["abrangencias"].as_array().unwrap().iter().find(|x| x["abr"] == abr).map(|x| x["eleitorado"].as_u64().unwrap()).unwrap_or(0);
    (circ, a)
}

/// B-055: o TSE publica o eleitorado do 2º turno aos poucos (no 3221, o `te` do TO ficou 0 até 15/10 e o do `br`
/// só então chegou a 158.745.502). `preparar --atualizar` tem de baixar de novo os `-u.json`, não usar o cache.
#[test]
fn atualizar_baixa_de_novo_os_u_json() {
    let arqs = Arc::new(Mutex::new(HashMap::new()));
    {
        let mut a = arqs.lock().unwrap();
        a.insert("ele-c.json".into(), r#"{"pl":[{"cd":"3221","c":"ele2026","dt":"25/10/2026","e":[{"cd":"6258","nm":"Presidente","t":"2","tp":"1","abr":[{"cd":"BR","cp":[{"cd":"1","tp":"1"}]}]}]}]}"#.to_string());
        a.insert("mun-e006258-cm.json".into(), r#"{"abr":[{"cd":"TO","ds":"TOCANTINS","mu":[{"cd":"73440","nm":"PALMAS"}]},{"cd":"SE","ds":"SERGIPE","mu":[{"cd":"31054","nm":"ARACAJU"}]}]}"#.to_string());
        a.insert("to-p003221-cs.json".into(), r#"{"abr":[{"cd":"TO","mu":[{"cd":"73440","nm":"PALMAS","zon":[{"cd":"29","sec":[{"ns":"1"}]}]}]}]}"#.to_string());
        a.insert("se-p003221-cs.json".into(), r#"{"abr":[{"cd":"SE","mu":[{"cd":"31054","nm":"ARACAJU","zon":[{"cd":"1","sec":[{"ns":"1"}]}]}]}]}"#.to_string());
        // como no TSE em 13/10: TO ainda sem eleitorado; o Brasil só com SE
        a.insert("br-c0001-e006258-u.json".into(), u_json(1_000, 1));
        a.insert("se-c0001-e006258-u.json".into(), u_json(1_000, 1));
        a.insert("to-c0001-e006258-u.json".into(), u_json(0, 0));
    }
    let base = servidor(arqs.clone());
    let rt = tokio::runtime::Runtime::new().unwrap();
    let alvo = Alvo::novo(&base, "oficial", "ele2026", 3221);
    let d = std::env::temp_dir().join(format!("uv-preparo-b055-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    let op = |atualizar| Opcoes { fontes: d.join("fontes"), saida: d.clone(), offline: false, atualizar, ignorar_destinacao: false, cargos: vec![] };
    assert!(preparar(&rt, &alvo, &op(false)).unwrap().is_empty());
    assert_eq!(eleitorado(&d, "to"), (1_000, 0));
    // como no TSE em 15/10: o TO entrou e o Brasil passou a somar as duas UFs
    {
        let mut a = arqs.lock().unwrap();
        a.insert("br-c0001-e006258-u.json".into(), u_json(1_700, 2));
        a.insert("to-c0001-e006258-u.json".into(), u_json(700, 1));
    }
    assert!(preparar(&rt, &alvo, &op(true)).unwrap().is_empty());
    assert_eq!(eleitorado(&d, "to"), (1_700, 700), "preparar --atualizar ficou com o eleitorado do cache (B-055)");
    std::fs::remove_dir_all(&d).unwrap();
}
