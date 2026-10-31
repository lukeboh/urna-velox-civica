use super::*;

const ELE_C: &[u8] = include_bytes!("../../tests/fixtures/ele-c-2026-10-29.json");

fn t(s: &str) -> i64 { urna_velox::metricas::segundos(s).unwrap() }
fn achar(l: &[Pleito], c: u32) -> &Pleito { l.iter().find(|p| p.codigo == c).unwrap() }

#[test]
fn classificacao_pela_data() {
    let d = "25/10/2026";
    assert_eq!(classificar(d, t("24/10/2026 23:59:59"), 0), Some(Situacao::Agendado));
    assert_eq!(classificar(d, t("25/10/2026 00:00:00"), 0), Some(Situacao::EmCurso));
    assert_eq!(classificar(d, t("26/10/2026 07:59:59"), 0), Some(Situacao::EmCurso));
    assert_eq!(classificar(d, t("26/10/2026 08:00:00"), 0), Some(Situacao::Encerrado));
    assert_eq!(classificar(d, t("25/10/2026 12:00:00"), 6), Some(Situacao::Encerrado), "fim da apuração configurável");
    assert_eq!(classificar("xx", 0, 0), None);
}

#[test]
fn ele_c_real_de_29_10_2026() {
    let l = descobrir(ELE_C, t("29/10/2026 12:00:00"), &Opcoes::default()).unwrap();
    assert_eq!(l.len(), 43, "pleitos no ele-c.json de 29/10/2026");
    assert!(l.iter().all(|p| p.situacao == Situacao::Encerrado), "nenhum pleito futuro publicado em 29/10/2026");
    assert_eq!(l[0].codigo, 3221, "o mais recente primeiro");
    let p = achar(&l, 3221);
    assert_eq!((p.nome.as_str(), p.titulo.as_str(), p.turno, p.ano), ("2º turno de 2026", "Eleições Gerais 2026 · 2º turno", 2, 2026));
    assert_eq!(p.data_iso, "2026-10-25");
    assert_eq!(p.complementares, 10, "suplementares e consultas do mesmo dia");
    let p = achar(&l, 3220);
    assert_eq!(p.nome, "1º turno de 2026");
    let cargos: Vec<u8> = p.eleicoes.iter().flat_map(|e| e.cargos.iter().map(|c| c.codigo)).collect();
    assert_eq!(cargos, vec![1, 3, 5, 25], "proporcionais (6, 7, 8) de fora");
    let p = achar(&l, 452);
    assert_eq!((p.nome.as_str(), p.titulo.as_str()), ("1º turno de 2024", "Eleições Municipais 2024 · 1º turno"));
    assert_eq!(achar(&l, 453).nome, "2º turno de 2024");
    let p = achar(&l, 3080);
    assert_eq!((p.nome.as_str(), p.titulo.as_str(), p.ano), ("Suplementar de Paranhos (MS)", "Eleição suplementar · Paranhos (MS)", 2025));
    assert_eq!(p.eleicoes[0].municipios, vec![91847]);
    assert_eq!(p.eleicoes[0].cargos[0].nome, "Prefeito");
    assert_eq!(achar(&l, 3040).nome, "Suplementares de Neves Paulista (SP) e Eldorado (SP)");
    let p = achar(&l, 3237);
    assert_eq!((p.nome.as_str(), p.ufs.clone()), ("Suplementar de Roraima", vec!["rr".to_string()]), "suplementar estadual: o nome já é a UF");
    assert_eq!(p.eleicoes[0].cargos[0].codigo, 3);
    assert_eq!(achar(&l, 3235).nome, "Suplementar de Amparo do Serra (MG)", "preposição em minúscula");
    assert_eq!(achar(&l, 3211).nome, "Suplementar de Cruzeiro do Iguaçu (PR)");
}

#[test]
fn agendado_e_em_curso() {
    // o mesmo arquivo visto em 01/10/2026 (antes do 1º turno) e na noite do 2º turno
    let l = descobrir(ELE_C, t("01/10/2026 10:00:00"), &Opcoes::default()).unwrap();
    assert_eq!(achar(&l, 3220).situacao, Situacao::Agendado);
    assert_eq!(achar(&l, 3221).situacao, Situacao::Agendado);
    assert_eq!(achar(&l, 452).situacao, Situacao::Encerrado);
    let l = descobrir(ELE_C, t("25/10/2026 19:00:00"), &Opcoes::default()).unwrap();
    assert_eq!(achar(&l, 3221).situacao, Situacao::EmCurso);
    assert_eq!(achar(&l, 3220).situacao, Situacao::Encerrado);
}

#[test]
fn acervo_marca_o_replay_e_json() {
    let dir = std::env::temp_dir().join(format!("uv-pleitos-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("453")).unwrap();
    std::fs::write(dir.join("453/.completo"), "").unwrap();
    std::fs::write(dir.join("453/acervo.json"), r#"{"pleito":453,"secoes":97392,"tempo_cadencia_s":7200,"ciclo":"ele2024"}"#).unwrap();
    std::fs::create_dir_all(dir.join("452")).unwrap(); // sem .completo: não serve
    let agora = t("29/10/2026 12:00:00");
    let l = descobrir(ELE_C, agora, &Opcoes { acervo: Some(dir.clone()), ..Default::default() }).unwrap();
    assert!(achar(&l, 453).replay);
    assert!(!achar(&l, 452).replay);
    assert_eq!(achar(&l, 453).acervo, Some(serde_json::json!({"secoes": 97392, "tempo_cadencia_s": 7200})));
    assert_eq!(achar(&l, 452).acervo, None);
    let j = json(&l, agora, ELE_C);
    assert_eq!(j["contagem"]["encerrado"], 43);
    assert_eq!(j["contagem"]["replay"], 1);
    assert_eq!(j["ele_c"]["dg"], "18/10/2026");
    assert_eq!(j["pleitos"][0]["situacao"], "encerrado");
    assert_eq!(j["pleitos"][0]["eleicoes"][0]["tipo"], "suplementar");
    let arq = dir.join("saida/pleitos.json");
    gravar_atomico(&arq, serde_json::to_string(&j).unwrap().as_bytes()).unwrap();
    assert!(arq.is_file());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn ele_c_invalido() {
    assert!(descobrir(b"{}", 0, &Opcoes::default()).is_err());
    assert!(descobrir(b"nada", 0, &Opcoes::default()).is_err());
}
