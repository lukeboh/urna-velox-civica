use super::*;

fn tmp(nome: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("uv-serie-{nome}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}
fn pub_(seq: u64, ap: u64) -> String { format!("{{\"v\":1,\"abr\":\"br\",\"seq\":{seq},\"secoes\":{{\"total\":10,\"apuradas\":{ap}}}}}") }
fn h(hms: &str) -> i64 { urna_velox::metricas::segundos(&format!("25/10/2026 {hms}")).unwrap() }

/// Uma versão por minuto (a última do minuto), voltar no tempo descarta o que vinha depois, a última versão é a
/// publicação final byte a byte, e um acervo cortado no meio (queda) é lido até a última linha inteira.
#[test]
fn serie_por_minuto_com_volta_e_queda() {
    let s = tmp("min");
    let pasta = s.join(PASTA);
    {
        let mut a = Acervo::abrir(&pasta).unwrap();
        a.versao(None, 1, "br-c0001-e006258.json", &pub_(0, 0)).unwrap();
        a.versao(Some(h("17:00:10")), 2, "br-c0001-e006258.json", &pub_(1, 1)).unwrap();
        a.versao(Some(h("17:00:50")), 3, "br-c0001-e006258.json", &pub_(2, 2)).unwrap();
        a.secoes(3, &[(7u64 << 32 | 1 << 16 | 5, h("17:00:50"))]).unwrap();
        a.versao(Some(h("17:01:05")), 4, "br-c0001-e006258.json", &pub_(3, 3)).unwrap();
        a.versao(Some(h("17:03:00")), 5, "br-c0001-e006258.json", &pub_(9, 9)).unwrap();
        // replay reposicionado: volta para 17:01 e segue por outro caminho
        a.versao(Some(h("17:01:30")), 6, "br-c0001-e006258.json", &pub_(4, 4)).unwrap();
        a.versao(Some(h("17:02:00")), 7, "br-c0001-e006258.json", &pub_(5, 5)).unwrap();
        a.versao(Some(h("17:02:00")), 8, "sp-c0001-e006258.json", &pub_(5, 2)).unwrap();
    }
    // segunda partida, que cai no meio de uma linha
    {
        let mut a = Acervo::abrir(&pasta).unwrap();
        a.versao(Some(h("17:04:00")), 9, "br-c0001-e006258.json", &pub_(6, 6)).unwrap();
        a.descarregar().unwrap();
        let c = a.caminho.clone();
        a.fechar().unwrap();
        let mut b = std::fs::read(&c).unwrap();
        // acrescenta um gzip cortado (o que sobra de uma queda no meio da escrita)
        let mut g = GzEncoder::new(Vec::new(), Compression::new(6));
        g.write_all(b"{\"t\":1,\"s\":1,\"a\":\"br-c0001-e006258.json\",\"j\":{\"cor").unwrap();
        g.flush().unwrap();
        b.extend(g.get_ref());
        std::fs::write(&c, b).unwrap();
    }
    let pd = s.join("publicacao");
    std::fs::create_dir_all(&pd).unwrap();
    std::fs::write(pd.join("br-c0001-e006258.json"), pub_(6, 6)).unwrap();
    std::fs::write(pd.join("sp-c0001-e006258.json"), pub_(5, 2)).unwrap();
    std::fs::write(s.join("eventos.csv"), "seq,horario,abrangencia,cargo,candidato,estado\n# retomada\n4,2026-10-25 17:01:30,br,1,13,matematicamente_eleito\n").unwrap();
    let o = Opcoes { saida: s.clone(), acervo: None, candidatos: None, destino: s.join("consulta"), passo_s: 60, tse: None, oficial: None, em_curso: false };
    let (r, ok) = montar(&o).unwrap();
    assert!(ok, "{r}");
    assert_eq!(r["conferencia"]["identicos"], 2);
    assert_eq!(r["acervo"]["arquivos"], 2);
    assert_eq!(r["acervo"]["truncados"].as_array().unwrap().len(), 1, "a linha cortada é ignorada");
    let t: Value = serde_json::from_str(&std::fs::read_to_string(s.join("consulta/t/br-c0001.json")).unwrap()).unwrap();
    let seqs: Vec<(Value, u64)> = t["versoes"].as_array().unwrap().iter().map(|v| (v[0].clone(), v[1]["seq"].as_u64().unwrap())).collect();
    assert_eq!(seqs, vec![(Value::Null, 0), (json!(h("17:00:50")), 2), (json!(h("17:01:30")), 4), (json!(h("17:02:00")), 5), (json!(h("17:04:00")), 6)],
        "última de cada minuto; o 17:03 da primeira passada saiu com a volta");
    let i: Value = serde_json::from_str(&std::fs::read_to_string(s.join("consulta/indice.json")).unwrap()).unwrap();
    assert_eq!(i["instantes"], json!([h("17:00:00"), h("17:01:00"), h("17:02:00"), h("17:04:00")]));
    assert_eq!(i["topicos"]["sp-c0001"]["versoes"], 1);
    let e: Value = serde_json::from_str(&std::fs::read_to_string(s.join("consulta/eventos.json")).unwrap()).unwrap();
    assert_eq!(e["linhas"][0][6], json!(h("17:01:30")), "eventos com a hora da noite, sem o comentário");
    assert!(s.join("consulta/t/br-c0001.json.gz").exists());
    // publicação final diferente da última versão: a conferência recusa e nada é gravado
    std::fs::write(pd.join("br-c0001-e006258.json"), pub_(7, 7)).unwrap();
    let o2 = Opcoes { destino: s.join("consulta2"), ..o };
    let (r, ok) = montar(&o2).unwrap();
    assert!(!ok);
    assert_eq!(r["conferencia"]["divergentes"], json!(["br-c0001-e006258.json"]));
    assert!(!s.join("consulta2").exists());
    // pleito em curso: a mesma divergência (a publicação andou) não impede a série; marcador e índice em curso
    let o3 = Opcoes { destino: s.join("consulta3"), em_curso: true, ..o2 };
    let (r, ok) = montar(&o3).unwrap();
    assert!(ok, "{r}");
    assert!(s.join("consulta3/EM_CURSO").exists());
    let i3: Value = serde_json::from_str(&std::fs::read_to_string(s.join("consulta3/indice.json")).unwrap()).unwrap();
    assert_eq!(i3["em_curso"], true);
    // refeita: a pasta é trocada inteira (nada da anterior sobra, nenhuma pasta .nova/.velha)
    std::fs::write(s.join("consulta3/sobra.json"), "{}").unwrap();
    assert!(montar(&o3).unwrap().1);
    assert!(!s.join("consulta3/sobra.json").exists());
    assert!(!s.join("consulta3.nova").exists() && !s.join("consulta3.velha").exists());
    let _ = std::fs::remove_dir_all(&s);
}

/// O mapa: o minuto em que cada seção acendeu, no índice do `GET /api/v1/mapa/secoes`, com a menor hora vista.
#[test]
fn mapa_da_serie() {
    let s = tmp("mapa");
    let cp = Path::new(env!("CARGO_MANIFEST_DIR")).join("../motor/tests/fixtures/candidatos_exemplo.json");
    let (_, arq) = urna_velox::candidatos::Candidatos::ler(&cp).unwrap();
    let cad = urna_velox::candidatos::Candidatos::do_arquivo(&arq);
    let cat = urna_velox::catalogo::Catalogo::de_arquivo_candidatos(&arq).unwrap();
    let mut ch = crate::mapa::chaves_do_catalogo(&cat, &cad);
    ch.sort_by_key(|x| x.1);
    let k = ch[0].1;
    {
        let mut a = Acervo::abrir(&s.join(PASTA)).unwrap();
        a.versao(Some(h("17:00:00")), 1, "br-c0001-e000001.json", &pub_(1, 1)).unwrap();
        a.secoes(1, &[(k, h("17:05:10")), (999, h("17:06:00"))]).unwrap();
        a.secoes(2, &[(k, h("17:02:10"))]).unwrap();
    }
    std::fs::create_dir_all(s.join("publicacao")).unwrap();
    std::fs::write(s.join("publicacao/br-c0001-e000001.json"), pub_(1, 1)).unwrap();
    let o = Opcoes { saida: s.clone(), acervo: None, candidatos: Some(cp), destino: s.join("c"), passo_s: 60, tse: None, oficial: None, em_curso: false };
    let (r, ok) = montar(&o).unwrap();
    assert!(ok, "{r}");
    assert_eq!(r["mapa"]["acesas"], 1);
    assert_eq!(r["mapa"]["fora"], 1);
    let m: Value = serde_json::from_str(&std::fs::read_to_string(s.join("c/mapa.json")).unwrap()).unwrap();
    use base64::Engine;
    let b = base64::engine::general_purpose::STANDARD.decode(m["m"].as_str().unwrap()).unwrap();
    assert_eq!(m["inicio"], json!(h("17:00:00")));
    assert_eq!(u16::from_le_bytes([b[0], b[1]]), 3, "17:02 = 3º minuto desde o início (a menor hora vista)");
    let lista: Value = serde_json::from_str(&std::fs::read_to_string(s.join("c/mapa-secoes.json")).unwrap()).unwrap();
    assert_eq!(lista["n"], m["total"]);
    let _ = std::fs::remove_dir_all(&s);
}

/// Lado do TSE pelas parciais: a última versão de cada minuto (pela hora da geração), só dos tópicos publicados,
/// sem as pastas de zona; o JSON vem compacto.
#[test]
fn tse_das_parciais_por_minuto() {
    let s = tmp("parciais");
    let lote = |dir: &str, linhas: &[(&str, &str, &str)]| {
        std::fs::create_dir_all(s.join(dir)).unwrap();
        let mut g = GzEncoder::new(Vec::new(), Compression::new(6));
        for (a, fim, c) in linhas { writeln!(g, "{}", json!({"arquivo": a, "fim": fim, "conteudo": c})).unwrap(); }
        std::fs::write(s.join(dir).join("lote-000001-000009.jsonl.gz"), g.finish().unwrap()).unwrap();
    };
    lote("u-br-uf", &[("br-c0001-e006258-u.json", "2026-10-25T17:00:05.1", "\n{ \"hg\" : \"17:00:05\" }"),
        ("br-c0001-e006258-u.json", "2026-10-25T17:00:40.0", "{\"hg\":\"17:00:40\"}"),
        ("br-c0001-e006258-u.json", "2026-10-25T17:02:00.0", "{\"hg\":\"17:02:00\"}"),
        ("sp-c0005-e006259-u.json", "2026-10-25T17:02:00.0", "{\"hg\":\"x\"}")]);
    lote("u-zona", &[("br-c0001-e006258-u.json", "2026-10-25T17:05:00.0", "{\"hg\":\"zona\"}")]);
    let nomes: HashMap<String, String> = [("br-c0001-e006258-u.json".to_string(), "br-c0001".to_string())].into();
    assert_eq!(tse_das_parciais(&s, &nomes, 60, h("17:01:00")).unwrap()["br-c0001"].len(), 1, "só até o limite");
    let r = tse_das_parciais(&s, &nomes, 60, i64::MAX).unwrap();
    assert_eq!(r.len(), 1, "só os tópicos publicados");
    assert_eq!(r["br-c0001"], vec![(h("17:00:40"), "{\"hg\":\"17:00:40\"}".to_string()), (h("17:02:00"), "{\"hg\":\"17:02:00\"}".to_string())]);
    let _ = std::fs::remove_dir_all(&s);
}
