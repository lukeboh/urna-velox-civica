use super::*;
#[test]
fn sistemas() {
    // art. 224 em todo cargo majoritário (B-070): Presidente, Governador, Senador e Prefeito, nos dois turnos
    assert_eq!(Sistema::para(1, 1, 0, 1, false), (Sistema::MaioriaAbsoluta { base: Base::Aptos }, true));
    assert_eq!(Sistema::para(3, 1, 0, 1, false), (Sistema::MaioriaAbsoluta { base: Base::Aptos }, true));
    assert_eq!(Sistema::para(1, 2, 0, 1, false), (Sistema::MaioriaSimples { vagas: 1 }, true));
    assert_eq!(Sistema::para(3, 2, 0, 1, false), (Sistema::MaioriaSimples { vagas: 1 }, true));
    assert_eq!(Sistema::para(5, 1, 0, 2, false), (Sistema::MaioriaSimples { vagas: 2 }, true));
    assert_eq!(Sistema::para(25, 1, 0, 1, false), (Sistema::MaioriaSimples { vagas: 1 }, false), "Conselheiro Distrital: fora");
    assert_eq!(Sistema::para(11, 1, 200_000, 1, false), (Sistema::MaioriaSimples { vagas: 1 }, true));
    assert_eq!(Sistema::para(11, 1, 200_001, 1, false), (Sistema::MaioriaAbsoluta { base: Base::Nominal }, true));
    assert_eq!(Sistema::para(11, 2, 900_000, 1, false), (Sistema::MaioriaSimples { vagas: 1 }, true));
    assert_eq!(Sistema::para(55, 1, 9_000, 1, true), (Sistema::MaioriaSimples { vagas: 1 }, false));
}
#[test]
fn destinacoes_e_datas() {
    assert!(Destinacao::de_texto("Válido").apto());
    assert_eq!(Destinacao::de_texto("Anulado sub judice"), Destinacao::AnuladoSubJudice);
    assert!(!Destinacao::de_texto("Anulado").apto() && !Destinacao::de_texto("Cassado").apto());
    assert_eq!(data("26/07/1971"), Some(19710726));
    assert_eq!(data(""), None);
    assert!(data("01/01/1950") < data("31/12/1951"));
}
#[test]
fn circunscricoes() {
    let mu = Abrangencia::Municipio(*b"ac", 1031);
    assert_eq!(circunscricao_de(1, &Abrangencia::Uf(*b"zz")), Abrangencia::Brasil);
    assert_eq!(circunscricao_de(3, &Abrangencia::Uf(*b"df")), Abrangencia::Uf(*b"df"));
    assert_eq!(circunscricao_de(5, &mu), Abrangencia::Uf(*b"ac"));
    assert_eq!(circunscricao_de(11, &mu), mu);
    assert_eq!(circunscricao_de(55, &mu), mu);
}
#[test]
fn test_leitura_candidatos_json() {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/candidatos_exemplo.json");
    let (cad, arq) = Candidatos::ler(&p).expect("ler candidatos_exemplo.json");
    assert_eq!(arq.pleito, 3221);
    assert_eq!(arq.eleicoes.len(), 2);
    assert_eq!(cad.eleicoes.len(), 2);
    assert_eq!(cad.cargos(), vec![1, 11]);
    assert!(arq.destinacao_presumida && arq.incompleto.is_empty());
    // Presidente: uma circunscrição (Brasil); "df" é só abrangência publicada, com o próprio eleitorado
    let (br, df, sp) = (Abrangencia::Brasil, Abrangencia::Uf(*b"df"), Abrangencia::Uf(*b"sp"));
    assert_eq!(cad.n_circunscricoes(), 2);
    assert!(cad.circunscricao(1, &br).is_some() && cad.circunscricao(1, &df).is_none());
    let mut a1 = cad.abrangencias[&1].clone();
    a1.sort();
    let mut esperado = vec![br.clone(), df.clone(), sp.clone()];
    esperado.sort();
    assert_eq!(a1, esperado);
    assert_eq!(cad.eleitorado_abr[&(1, df.clone())], 2_200_000);
    assert_eq!(cad.eleitorado_abr[&(1, br.clone())], 155_823_368);
    assert_eq!(cad.eleicao_de(11, &Abrangencia::Municipio(*b"sp", 67458)), Some((6284, 1)));
    assert_eq!(cad.eleicao_de(1, &df), Some((6258, 2)));
    let cat = crate::catalogo::Catalogo::de_arquivo_candidatos(&arq).expect("catálogo");
    assert_eq!(cat.pleito, 3221);
    assert_eq!(cat.cargos, vec![1, 11]);
    assert_eq!(cat.secoes.len(), 2);
    // Prefeito só no município da eleição suplementar
    let ci = cat.ci(11).unwrap();
    let nc = cat.nc();
    let tem = |m: u32| cat.base[cat.muni(m).unwrap() as usize * nc + ci] != crate::catalogo::AUSENTE;
    assert!(tem(67458) && !tem(97012));
    // UF inválida: erro, não pânico
    let mut ruim = arq.clone();
    ruim.municipios[0].uf = "d".into();
    assert!(crate::catalogo::Catalogo::de_arquivo_candidatos(&ruim).is_err());
}

#[test]
fn agregadas_fora_do_total_de_secoes() {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/candidatos_exemplo.json");
    let (_, mut arq) = Candidatos::ler(&p).expect("ler candidatos_exemplo.json");
    assert!(!serde_json::to_string(&arq).unwrap().contains("agregada"), "campo omitido quando falso");
    arq.secoes[0].agregada = true;
    let txt = serde_json::to_string(&arq).unwrap();
    assert!(txt.contains("\"agregada\":true"));
    let arq: ArquivoCandidatos = serde_json::from_str(&txt).unwrap();
    let cad = Candidatos::do_arquivo(&arq);
    assert_eq!(cad.agregadas.len(), 1);
    let cat = crate::catalogo::Catalogo::de_arquivo_candidatos(&arq).expect("catálogo");
    assert_eq!(cat.secoes.len(), 2, "a agregada continua no catálogo");
    let est = crate::estado::Estado::novo(&cat);
    let br = Abrangencia::Brasil;
    assert_eq!(crate::resultado::Publicador::novo(&cat, None, None).secoes(&est, &br), (2, 0));
    assert_eq!(crate::resultado::Publicador::novo(&cat, Some(&cad), None).secoes(&est, &br), (1, 0));
    // B-051: agregada descoberta na apuração sai do total, uma vez só; a do candidatos.json já estava fora
    let mut p = crate::resultado::Publicador::novo(&cat, Some(&cad), None);
    let s = &arq.secoes;
    let uf: [u8; 2] = arq.municipios.iter().find(|m| m.codigo == s[1].municipio).unwrap().uf.as_bytes().try_into().unwrap();
    let (k0, k1) = (crate::chave(s[0].municipio, s[0].zona, s[0].secao), crate::chave(s[1].municipio, s[1].zona, s[1].secao));
    assert_eq!(p.agregada(uf, k0), None, "já fora do total pelo candidatos.json");
    assert_eq!(p.agregada(*b"xx", k1), None, "de outra UF");
    assert_eq!(p.agregada(uf, crate::chave(99_999, 1, 1)), None, "fora do catálogo");
    assert!(p.agregada(uf, k1).is_some());
    assert_eq!(p.secoes(&est, &br), (0, 0));
    assert_eq!(p.agregada(uf, k1), None, "não desconta duas vezes");
    assert_eq!(p.secoes(&est, &br), (0, 0));
}
