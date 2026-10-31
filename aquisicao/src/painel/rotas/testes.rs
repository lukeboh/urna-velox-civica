use super::*;

#[test]
fn bases_candidatas() {
    assert_eq!(candidatas("https://resultados.tse.jus.br/oficial/app/index.html#/eleicao", "oficial").unwrap(),
        (vec!["https://resultados.tse.jus.br".to_string()], "oficial".to_string()));
    assert_eq!(candidatas("https://novo.exemplo.org/apuracao/simulado/ele2026/", "oficial").unwrap(),
        (vec!["https://novo.exemplo.org/apuracao".to_string()], "simulado".to_string()));
    assert_eq!(candidatas("https://x.exemplo.org/a/b/index.html?q=1", "oficial").unwrap().0,
        vec!["https://x.exemplo.org/a/b", "https://x.exemplo.org/a", "https://x.exemplo.org"]);
    assert_eq!(candidatas("https://x.exemplo.org", "oficial").unwrap().0, vec!["https://x.exemplo.org"]);
    assert!(candidatas("ftp://x", "oficial").is_err());
    assert!(candidatas("https://x/$(id)/", "oficial").is_err());
    assert!(candidatas("https:///caminho", "oficial").is_err());
}

#[test]
fn estado_e_exemplos() {
    let d = std::env::temp_dir().join(format!("uv-rotas-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let ne = d.join("no.env");
    std::fs::write(&ne, "BASE=https://resultados.tse.jus.br\nPLEITO=3221\n").unwrap();
    let e = estado(&ne, &Value::Null);
    assert_eq!(e["padrao"], true);
    assert_eq!(e["exemplos"]["cs"], "https://resultados.tse.jus.br/oficial/ele2026/arquivo-urna/3221/config/sp/sp-p003221-cs.json");
    std::fs::write(arquivo(&ne), r#"{"base_arquivos":"https://espelho.exemplo.org"}"#).unwrap();
    let e = estado(&ne, &Value::Null);
    assert_eq!(e["padrao"], false);
    assert!(e["exemplos"]["arquivo"].as_str().unwrap().starts_with("https://espelho.exemplo.org/oficial/"));
    std::fs::write(arquivo(&ne), "lixo").unwrap();
    assert!(estado(&ne, &Value::Null)["erro"].is_string());
    let _ = std::fs::remove_dir_all(d);
}
