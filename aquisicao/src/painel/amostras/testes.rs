use super::*;

fn saude(aplicadas: u64, baixados: u64, recebidos: u64) -> Value {
    json!({"versao": "0.8.33", "estado": "coletando", "secoes": {"aplicadas": aplicadas, "pendentes": 3}, "fila": 7,
        "rede": {"bu_baixados": baixados, "req_s_atual": 94.5, "teto_req_s": 95, "http429": 0},
        "recursos": {"rss_mb": 410, "cpu_pct_atual": 55.5},
        "federacao": {"recebidos": recebidos, "publicados": 10, "sincronia": {"lotes_copiados": 2, "servidas": 4, "buscas_erro": 0}},
        "difusao": {"clientes": 12, "mensagens_s": 3.5, "bytes_s": 2048},
        "sonda": {"atraso_s": 0.0, "atrasada": false, "ultima": {"id": 4, "ms": {"motor": 0.4, "gravacao": 1.25}}},
        "alertas": [{"tipo": "a"}, {"tipo": "b"}]})
}

fn col(a: &Amostrador, c: &str) -> Vec<Value> { a.colunas(0)[c].as_array().unwrap().clone() }

#[test]
fn taxas_entre_amostras() {
    let mut a = Amostrador::novo(Path::new("/nao/existe"), 10);
    let sis = json!({"cpu_pct": 20.0, "memoria_mb": {"usada": 1000, "total": 24000}, "carga": [1.5], "disco_livre_gb": 80.0});
    for (t, s) in [(1000, saude(100, 90, 0)), (6000, saude(600, 590, 250)), (11000, saude(50, 40, 300))] {
        let l = a.registrar(t, Some(&s), &sis, Some(1.0));
        a.serie.push_back((t, l));
    }
    assert_eq!(col(&a, "bus_s"), vec![Value::Null, json!(100.0), Value::Null], "5 s, +500; depois o contador voltou (reinício)");
    assert_eq!(col(&a, "baixados_s"), vec![Value::Null, json!(100.0), Value::Null]);
    assert_eq!(col(&a, "fed_recebidos_s"), vec![Value::Null, json!(50.0), json!(10.0)]);
    assert_eq!(col(&a, "req_s")[1], json!(94.5));
    assert_eq!(col(&a, "ws_kb_s")[0], json!(2.0));
    assert_eq!(col(&a, "sis_mem_total_mb")[0], json!(24000.0));
    assert_eq!(a.colunas(6000)["t"], json!([11000]), "só depois de `desde`");
    assert_eq!(col(&a, "sonda_ms"), vec![json!(1.25); 3]);
    assert_eq!(col(&a, "sonda_atraso_s")[0], json!(0.0));
}

#[test]
fn sem_saude_ainda_amostra_o_sistema() {
    let mut a = Amostrador::novo(Path::new("/nao/existe"), 3);
    for t in 0..5u64 { a.amostrar(t * 1000); }
    assert_eq!(a.serie.len(), 3, "capacidade");
    assert!(a.saude_atual.is_none());
    let r = a.resumo();
    assert!(r["servico"]["versao"].is_null());
    assert!(r["sistema"]["memoria_mb"]["total"].as_u64().unwrap_or(0) > 0 || !Path::new("/proc/meminfo").exists());
}

#[test]
fn verificador_externo_no_resumo() {
    let d = std::env::temp_dir().join(format!("uv-painel-sonda-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let mut a = Amostrador::novo(&d, 3);
    a.amostrar(0);
    assert!(a.resumo()["sonda_verificador"].is_null(), "sem uv-sonda");
    std::fs::write(d.join(ARQ_VERIFICADOR), r#"{"ok": true, "alvos": {"ws://x/ws": {"situacao": "ok"}}}"#).unwrap();
    a.amostrar(1000);
    let r = a.resumo();
    assert_eq!(r["sonda_verificador"]["ok"], true);
    assert!(r["sonda_verificador"]["idade_arquivo_s"].as_f64().unwrap() < 60.0);
    let depois = std::time::SystemTime::now() + std::time::Duration::from_secs(90);
    assert!(ler_verificador(&d.join(ARQ_VERIFICADOR), depois).unwrap()["idade_arquivo_s"].as_f64().unwrap() >= 89.0);
    std::fs::write(d.join(ARQ_VERIFICADOR), "não é JSON").unwrap();
    a.amostrar(2000);
    assert!(a.resumo()["sonda_verificador"].is_null());
    // o bloco `sonda` da saúde vai para o resumo
    a.saude_atual = Some(saude(1, 1, 1));
    assert_eq!(a.resumo()["sonda"]["ultima"]["id"], 4);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn resumo_com_alertas_mais_recentes_primeiro() {
    let mut a = Amostrador::novo(Path::new("/nao/existe"), 3);
    a.saude_atual = Some(saude(1, 1, 1));
    assert_eq!(a.resumo()["alertas"], json!([{"tipo": "b"}, {"tipo": "a"}]));
    assert_eq!(a.resumo()["servico"]["versao"], "0.8.33");
}
