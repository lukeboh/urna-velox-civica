//! Conferências completas da etapa 0 com os dados locais (fora do repositório). Rodam só com `--ignored`:
//!
//! ```text
//! UV_DADOS=/home/dev/uvreal cargo test --release -p urna-velox --test integracao -- --ignored --test-threads 1
//! ```
//! Layout de `UV_DADOS`: `dados/*.uvpk`, `oficial/`, `metricas_*.csv` (2026, 1º turno); `2024-1t/` e `2024-2t/`
//! com `dados/`, `oficial/` e as métricas da coleta (`metricas_aux.csv`, `metricas_2t.csv`).

mod comum;
use comum::{
    conferir_pasta, conferir_pasta_com_candidatos, divergencias_por_destinacao, conferir_publicacoes_com_candidatos,
    conferir_publicacoes_pasta, totalizar, totalizar_com_candidatos, uv_dados,
};
use std::path::PathBuf;

fn metricas_2026() -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(uv_dados()).unwrap().flatten().map(|e| e.path())
        .filter(|p| { let n = p.file_name().unwrap().to_string_lossy().into_owned(); n.starts_with("metricas_") && n.ends_with(".csv") && !n.contains(".inicios") && !n.contains(".taxa") })
        .collect();
    v.sort();
    v
}

/// 1º turno de 2026: Presidente (Brasil, 27 UFs e exterior), Governador e Senador (27 UFs) — 83 abrangências.
#[test]
#[ignore]
fn primeiro_turno_2026_83_abrangencias() {
    let u = uv_dados();
    let c = totalizar(&u.join("dados"), &metricas_2026(), &[(5, 2)], 28);
    let r = conferir_pasta(&c, &u.join("oficial"));
    eprintln!("2026: {}", r.resumo());
    assert!(r.ok(), "{}\n{}", r.resumo(), r.detalhes(20));
    assert_eq!((r.arquivos, r.por_cargo.get(&1).copied(), r.por_cargo.get(&3).copied(), r.por_cargo.get(&5).copied()), (83, Some((29, 0)), Some((27, 0)), Some((27, 0))));
    // 794 candidatos aptos (como em comparar_oficial.py) + 38 "Anulado sub judice" (Não eleito)
    assert_eq!(r.situacoes.values().map(|x| x.0).sum::<usize>(), 832);
    let r = conferir_publicacoes_pasta(&c, &u.join("oficial"));
    eprintln!("2026, publicações JSON: {}", r.resumo());
    assert!(r.ok(), "{}\n{}", r.resumo(), r.detalhes(20));
    assert_eq!(r.situacoes.values().map(|x| x.0).sum::<usize>(), 832);
}

/// 1º turno de 2024: Prefeito nos 5.569 municípios e as 5 consultas populares.
#[test]
#[ignore]
fn primeiro_turno_2024_5569_municipios() {
    let u = uv_dados().join("2024-1t");
    let c = totalizar(&u.join("dados"), &[u.join("metricas_aux.csv")], &[], 28);
    let r = conferir_pasta(&c, &u.join("oficial"));
    eprintln!("2024, 1º turno: {}", r.resumo());
    assert!(r.ok(), "{}\n{}", r.resumo(), r.detalhes(20));
    assert_eq!((r.arquivos, r.por_cargo.get(&11).copied()), (5574, Some((5569, 0))));
    assert_eq!(r.nao_finalizados.len(), 9);
    // 15.032 candidatos a Prefeito (municípios com a totalização finalizada) + 10 opções das consultas
    assert_eq!((r.situacoes[&11].0, r.situacoes.values().map(|x| x.0).sum::<usize>()), (15032, 15042));
    let r = conferir_publicacoes_pasta(&c, &u.join("oficial"));
    eprintln!("2024, 1º turno, publicações JSON: {}", r.resumo());
    assert!(r.ok(), "{}\n{}", r.resumo(), r.detalhes(20));
    assert_eq!(r.situacoes.values().map(|x| x.0).sum::<usize>(), 15042);
}

/// 2º turno de 2024: Prefeito nos 51 municípios.
#[test]
#[ignore]
fn segundo_turno_2024_51_municipios() {
    let u = uv_dados().join("2024-2t");
    let c = totalizar(&u.join("dados"), &[u.join("metricas_2t.csv")], &[], 28);
    let r = conferir_pasta(&c, &u.join("oficial"));
    eprintln!("2024, 2º turno: {}", r.resumo());
    assert!(r.ok(), "{}\n{}", r.resumo(), r.detalhes(20));
    assert_eq!((r.arquivos, r.por_cargo.get(&11).copied()), (51, Some((51, 0))));
    assert_eq!(r.situacoes[&11], (102, 0));
    let r = conferir_publicacoes_pasta(&c, &u.join("oficial"));
    eprintln!("2024, 2º turno, publicações JSON: {}", r.resumo());
    assert!(r.ok(), "{}\n{}", r.resumo(), r.detalhes(20));
    assert_eq!(r.situacoes[&11], (102, 0));
}

/// 1º turno de 2026 com catálogo e candidatos montados via candidatos.json pré-eleição.
#[test]
#[ignore]
fn primeiro_turno_2026_pre_eleicao() {
    let u = uv_dados();
    let cand_json = u.join("preparo_2026_1t").join("candidatos.json");
    assert!(cand_json.exists(), "{} não existe: gere com `uv-coletor preparar` (docs/FORMATO_CANDIDATOS.md)", cand_json.display());
    let (c, cad) = totalizar_com_candidatos(&u.join("dados"), &metricas_2026(), &cand_json, 28);
    let r = conferir_pasta_com_candidatos(&c, &u.join("oficial"), &cad);
    eprintln!("2026 pré-eleição: {}", r.resumo());
    assert!(r.ok(), "{}\n{}", r.resumo(), r.detalhes(20));
    assert_eq!((r.arquivos, r.por_cargo.get(&1).copied(), r.por_cargo.get(&3).copied(), r.por_cargo.get(&5).copied()), (83, Some((29, 0)), Some((27, 0)), Some((27, 0))));
    assert_eq!(r.situacoes.values().map(|x| x.0).sum::<usize>(), 832);
    let r = conferir_publicacoes_com_candidatos(&c, &u.join("oficial"), &cad);
    eprintln!("2026 pré-eleição, publicações JSON: {}", r.resumo());
    assert!(r.ok(), "{}\n{}", r.resumo(), r.detalhes(20));
    assert_eq!(r.situacoes.values().map(|x| x.0).sum::<usize>(), 832);
}

/// 1º turno de 2024 com catálogo e candidatos montados via candidatos.json pré-eleição.
#[test]
#[ignore]
fn primeiro_turno_2024_pre_eleicao() {
    let u = uv_dados().join("2024-1t");
    let cand_json = u.join("preparo").join("candidatos.json");
    assert!(cand_json.exists(), "{} não existe: gere com `uv-coletor preparar` (docs/FORMATO_CANDIDATOS.md)", cand_json.display());
    let (c, cad) = totalizar_com_candidatos(&u.join("dados"), &[u.join("metricas_aux.csv")], &cand_json, 28);
    // destinação presumida "Válido": votos idênticos; a situação diverge só onde o oficial tem candidato não apto
    let r = conferir_pasta_com_candidatos(&c, &u.join("oficial"), &cad);
    eprintln!("2024 1º turno pré-eleição: {}", r.resumo());
    eprintln!("{}", r.detalhes(100));
    assert_eq!((r.arquivos, r.por_cargo[&11].0 + r.por_cargo[&11].1), (5574, 5569));
    assert_eq!(divergencias_por_destinacao(&r, &u.join("oficial")), (ARQ_DESTINACAO_2024, SIT_DESTINACAO_2024));
    assert_eq!(r.situacoes.values().map(|x| x.0 + x.1).sum::<usize>(), 15042);
    let r = conferir_publicacoes_com_candidatos(&c, &u.join("oficial"), &cad);
    eprintln!("2024 1º turno pré-eleição, publicações JSON: {}", r.resumo());
    assert_eq!(divergencias_por_destinacao(&r, &u.join("oficial")), (ARQ_DESTINACAO_2024, SIT_DESTINACAO_2024));
    assert_eq!(r.situacoes.values().map(|x| x.0 + x.1).sum::<usize>(), 15042);
}

/// B-005: 1º turno de 2024 com o `candidatos.json` das simulações do ciclo diário (`preparo-simulacao/`), que leva a
/// destinação final (`dvt`) dos `-u.json` oficiais — a eleição já acabou, não há o que presumir. Exige a conferência
/// idêntica nos 5.574 arquivos, sem nenhuma situação divergente (com a destinação presumida "Válido" eram 33).
#[test]
#[ignore]
fn primeiro_turno_2024_simulacao_destinacao_final() {
    let u = uv_dados().join("2024-1t");
    let cand_json = u.join("preparo-simulacao").join("candidatos.json");
    assert!(cand_json.exists(), "{} não existe: gere com deploy/oci/candidatos_simulacao.sh", cand_json.display());
    let (c, cad) = totalizar_com_candidatos(&u.join("dados"), &[u.join("metricas_aux.csv")], &cand_json, 28);
    let r = conferir_pasta_com_candidatos(&c, &u.join("oficial"), &cad);
    eprintln!("2024 1º turno, simulação: {}", r.resumo());
    eprintln!("{}", r.detalhes(10));
    let div: usize = r.situacoes.values().map(|x| x.1).sum();
    assert_eq!((r.arquivos, div), (5574, 0), "situações divergentes: {div}");
    assert!(r.ok(), "{}\n{}", r.resumo(), r.detalhes(20));
    assert_eq!(r.situacoes.values().map(|x| x.0).sum::<usize>(), 15042);
    let r = conferir_publicacoes_com_candidatos(&c, &u.join("oficial"), &cad);
    eprintln!("2024 1º turno, simulação, publicações JSON: {}", r.resumo());
    assert!(r.ok(), "{}\n{}", r.resumo(), r.detalhes(20));
    assert_eq!(r.situacoes.values().map(|x| x.0).sum::<usize>(), 15042);
}

/// Arquivos e situações de 2024 (1º turno) que divergem com a destinação presumida "Válido": municípios com
/// candidato cassado ou indeferido depois de preparados os candidatos (lista impressa pelo teste).
const ARQ_DESTINACAO_2024: usize = 32;
const SIT_DESTINACAO_2024: usize = 33;

/// 2º turno de 2024 com catálogo e candidatos montados via candidatos.json pré-eleição.
#[test]
#[ignore]
fn segundo_turno_2024_pre_eleicao() {
    let u = uv_dados().join("2024-2t");
    let cand_json = u.join("preparo").join("candidatos.json");
    assert!(cand_json.exists(), "{} não existe: gere com `uv-coletor preparar` (docs/FORMATO_CANDIDATOS.md)", cand_json.display());
    let (c, cad) = totalizar_com_candidatos(&u.join("dados"), &[u.join("metricas_2t.csv")], &cand_json, 28);
    let r = conferir_pasta_com_candidatos(&c, &u.join("oficial"), &cad);
    eprintln!("2024 2º turno pré-eleição: {}", r.resumo());
    assert!(r.ok(), "{}\n{}", r.resumo(), r.detalhes(20));
    assert_eq!((r.arquivos, r.por_cargo.get(&11).copied()), (51, Some((51, 0))));
    assert_eq!(r.situacoes[&11], (102, 0));
    let r = conferir_publicacoes_com_candidatos(&c, &u.join("oficial"), &cad);
    eprintln!("2024 2º turno pré-eleição, publicações JSON: {}", r.resumo());
    assert!(r.ok(), "{}\n{}", r.resumo(), r.detalhes(20));
    assert_eq!(r.situacoes[&11], (102, 0));
}

/// 2º turno de 2026 (pleito 3221), antes da apuração: o `candidatos.json` gerado por `uv-coletor preparar` tem todos
/// os municípios dos `cm.json` (28 UFs, inclusive SE e TO), Presidente com uma circunscrição (Brasil), as consultas
/// do PR e do MT por UF, e a publicação a frio tem exatamente as abrangências dos `-u.json` pré-apuração.
#[test]
#[ignore]
fn segundo_turno_2026_candidatos_pre_apuracao() {
    use urna_velox::oficial::Abrangencia;
    let u = uv_dados().join("2026-2t").join("preparo");
    let p = u.join("candidatos.json");
    assert!(p.exists(), "{} não existe: gere com `uv-coletor preparar --pleito 3221`", p.display());
    let (cad, arq) = urna_velox::candidatos::Candidatos::ler(&p).unwrap();
    let cat = urna_velox::catalogo::Catalogo::de_arquivo_candidatos(&arq).unwrap();
    // só cs.json podem faltar (SE e TO ainda não publicados em 12/10)
    assert!(arq.incompleto.iter().all(|x| x.contains("-cs.json")), "{:?}", arq.incompleto);
    assert_eq!(arq.ufs.len(), 28);
    for e in &arq.eleicoes {
        assert!(!e.municipios.is_empty(), "eleição {} sem municípios", e.codigo);
        for m in &e.municipios {
            let Some(Abrangencia::Municipio(uf, mu)) = Abrangencia::de_codigo(m) else { panic!("{m}") };
            let i = cat.muni(mu).unwrap_or_else(|| panic!("município {m} da eleição {} fora do catálogo", e.codigo));
            assert_eq!(cat.ufs[cat.muni_uf[i as usize] as usize], uf);
        }
    }
    assert!(cad.circunscricao(1, &Abrangencia::Brasil).is_some());
    assert_eq!(arq.eleicoes.iter().flat_map(|e| &e.cargos).filter(|c| c.cargo == 1).map(|c| c.circunscricoes.len()).sum::<usize>(), 1);
    for (cargo, uf) in [(25u8, *b"pr"), (26, *b"mt"), (27, *b"mt")] {
        assert_eq!(cad.abrangencias[&cargo], vec![Abrangencia::Uf(uf)], "cargo {cargo}");
        assert!(cad.circunscricao(cargo, &Abrangencia::Uf(uf)).is_some());
    }
    // publicação a frio (sem BU): os nomes dos -u.json pré-apuração, sem o "-u"; estado "apurando" e nenhum st
    // o `preparar` guarda os -u.json em `fontes/u/` (e ainda aceita os antigos na raiz de `fontes/`)
    let mut esperado: Vec<String> = [u.join("fontes"), u.join("fontes").join("u")].iter().filter_map(|d| std::fs::read_dir(d).ok())
        .flat_map(|r| r.flatten()).map(|e| e.file_name().to_string_lossy().into_owned()).filter(|n| n.ends_with("-u.json"))
        .map(|n| n.replace("-u.json", ".json")).collect();
    esperado.sort();
    esperado.dedup();
    assert!(!esperado.is_empty(), "nenhum -u.json pré-apuração em {}", u.join("fontes").display());
    let est = urna_velox::estado::Estado::novo(&cat);
    let pubr = urna_velox::resultado::Publicador::novo(&cat, Some(&cad), None);
    let mo = urna_velox::resultado::Momento { encerrada: true, seq: 0, ultimo_bu: None, totalizado_ms: None };
    let mut nomes = vec![];
    for cargo in pubr.cargos() {
        for abr in pubr.abrangencias(cargo, false) {
            let v = pubr.valor(&est, None, &abr, cargo, mo).unwrap_or_else(|| panic!("sem publicação: cargo {cargo} {}", abr.codigo()));
            assert_eq!(v["estado"], "apurando");
            assert!(v["cand"].as_array().unwrap().iter().all(|c| c["st"] == ""), "{v}");
            nomes.push(pubr.nome_arquivo(&abr, cargo));
        }
    }
    nomes.sort();
    assert_eq!(nomes, esperado);
}
