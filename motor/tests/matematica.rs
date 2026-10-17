//! Teste de propriedade do "matematicamente": reproduzindo a noite na ordem real de chegada dos BUs, quem fica
//! matematicamente eleito (ou no 2º turno) é o eleito (ou o candidato no 2º turno) oficial, e nenhuma declaração
//! é revogada depois. Fixtures (sempre) e dados completos (`UV_DADOS`, `--ignored`).

mod comum;
use comum::{fixtures, uv_dados};
use std::path::{Path, PathBuf};
use urna_velox::acompanhamento::Acompanhamento;
use urna_velox::candidatos::{circunscricao_de, Candidatos};
use urna_velox::catalogo::Catalogo;
use urna_velox::matematica::{Mat, MatCirc};
use urna_velox::metricas::Metricas;
use urna_velox::noite::reproduzir;
use urna_velox::oficial::Oficial;
use urna_velox::pacote::EmMemoria;

struct Placar { circs: usize, declaradas: usize, problemas: Vec<String>, us_por_bu: f64 }

fn conferir(ac: &Acompanhamento, ofs: &[Oficial]) -> (usize, usize, Vec<String>) {
    let (mut circs, mut cands, mut prob) = (0, 0, vec![]);
    for c in &ac.circs {
        if c.revogacoes > 0 { prob.push(format!("{} cargo {}: {} revogações", c.abr().codigo(), c.cargo, c.revogacoes)); }
        // o arquivo oficial da própria circunscrição
        let Some(of) = ofs.iter().find(|o| o.cargo == c.cargo && o.abr.as_ref() == Some(c.abr()) && circunscricao_de(o.cargo, c.abr()) == *c.abr()) else { continue };
        if !of.finalizada { continue; }
        let st = |n: u32| of.cand.iter().find(|x| x.numero == n).map_or("", |x| x.st.as_str());
        for (i, m) in c.mat.iter().enumerate() {
            let n = c.circ.candidatos[i].numero;
            let esperado = match m { Mat::Eleito => "Eleito", Mat::SegundoTurno => "2º turno", Mat::Nao => continue };
            cands += 1;
            if st(n) != esperado { prob.push(format!("{} cargo {}: candidato {n} matematicamente '{esperado}', oficial '{}'", c.abr().codigo(), c.cargo, st(n))); }
        }
        match c.mat_circ {
            MatCirc::Eleito => {
                circs += 1;
                let of_eleitos = of.cand.iter().filter(|x| x.st == "Eleito").count();
                let mat_eleitos = c.mat.iter().filter(|&&m| m == Mat::Eleito).count();
                if of_eleitos != mat_eleitos { prob.push(format!("{} cargo {}: {mat_eleitos} matematicamente eleitos, {of_eleitos} oficiais", c.abr().codigo(), c.cargo)); }
            }
            MatCirc::SegundoTurno => {
                circs += 1;
                if !of.cand.iter().any(|x| x.st == "2º turno") { prob.push(format!("{} cargo {}: matematicamente 2º turno sem 2º turno oficial", c.abr().codigo(), c.cargo)); }
            }
            MatCirc::Apurando => {}
        }
    }
    (circs, cands, prob)
}

fn placar(dados: &Path, oficial: &Path, metricas: &[PathBuf], vagas: &[(u8, u8)]) -> Placar {
    placar_com(dados, oficial, metricas, vagas, None)
}

/// Com `candidatos`: catálogo e candidatos do `candidatos.json` pré-eleição (os oficiais só para conferir).
fn placar_com(dados: &Path, oficial: &Path, metricas: &[PathBuf], vagas: &[(u8, u8)], candidatos: Option<&Path>) -> Placar {
    let mem = EmMemoria::carregar(dados, None).unwrap();
    let ps: Vec<&Path> = metricas.iter().map(|p| p.as_path()).collect();
    let met = Metricas::ler(&ps).unwrap();
    let ofs = Oficial::ler_pasta(oficial).unwrap();
    let (mut cat, cad) = match candidatos {
        Some(p) => {
            assert!(p.exists(), "{} não existe: gere com `uv-coletor preparar`", p.display());
            let (cad, arq) = Candidatos::ler(p).unwrap();
            (Catalogo::de_arquivo_candidatos(&arq).unwrap(), cad)
        }
        None => (Catalogo::dos_pacotes(&mem, vagas).0, Candidatos::dos_oficiais(&ofs)),
    };
    cat.usar_anuladas(&met);
    let (_est, ac, md) = reproduzir(&cat, &cad, &mem, &met.recebido, None);
    let (declaradas, candidatos, problemas) = conferir(&ac, &ofs);
    eprintln!("{}: {} BUs ({} sem horário), {} lotes, {} avaliações, {:.2} µs por BU; {} circunscrições, {declaradas} decididas \
               matematicamente, {candidatos} candidatos declarados, {} problemas", dados.display(), md.bus, md.sem_horario, md.lotes,
              md.avaliacoes, md.us_por_bu(), ac.circs.len(), problemas.len());
    Placar { circs: ac.circs.len(), declaradas, problemas, us_por_bu: md.us_por_bu() }
}

#[test]
fn fixtures_2024() {
    // 2024-1t: 10 circunscrições (8 Prefeito + 2 consultas); ficam sem decisão SP 62081 (ninguém eleito, art. 224)
    // e SE 31313 (totalização não finalizada, não conferida)
    for (nome, n, decididas) in [("2024-1t", 10, 8), ("2024-2t", 1, 1)] {
        let d = fixtures().join(nome);
        let p = placar(&d.join("dados"), &d.join("oficial"), &[d.join("metricas.csv")], &[]);
        assert!(p.problemas.is_empty(), "{nome}: {:?}", p.problemas);
        assert_eq!((p.circs, p.declaradas), (n, decididas), "{nome}");
    }
}

#[test]
#[ignore]
fn segundo_turno_2024() {
    let u = uv_dados().join("2024-2t");
    let p = placar(&u.join("dados"), &u.join("oficial"), &[u.join("metricas_2t.csv")], &[]);
    assert!(p.problemas.is_empty(), "{:?}", &p.problemas[..p.problemas.len().min(20)]);
    assert_eq!(p.circs, 51);
    assert!(p.us_por_bu < 10.0);
}

#[test]
#[ignore]
fn primeiro_turno_2024() {
    let u = uv_dados().join("2024-1t");
    let p = placar(&u.join("dados"), &u.join("oficial"), &[u.join("metricas_aux.csv")], &[]);
    assert!(p.problemas.is_empty(), "{:?}", &p.problemas[..p.problemas.len().min(20)]);
    assert_eq!(p.circs, 5574);
    assert!(p.us_por_bu < 10.0);
}

#[test]
#[ignore]
fn primeiro_turno_2026() {
    let u = uv_dados();
    let mut met: Vec<PathBuf> = std::fs::read_dir(&u).unwrap().flatten().map(|e| e.path())
        .filter(|p| { let n = p.file_name().unwrap().to_string_lossy().into_owned(); n.starts_with("metricas_") && n.ends_with(".csv") && !n.contains(".inicios") && !n.contains(".taxa") })
        .collect();
    met.sort();
    let p = placar(&u.join("dados"), &u.join("oficial"), &met, &[(5, 2)]);
    assert!(p.problemas.is_empty(), "{:?}", &p.problemas[..p.problemas.len().min(20)]);
    // Presidente (Brasil) + Governador e Senador (27 UFs)
    assert_eq!(p.circs, 55);
    assert!(p.us_por_bu < 10.0);
}

fn metricas_2026() -> Vec<PathBuf> {
    let mut met: Vec<PathBuf> = std::fs::read_dir(uv_dados()).unwrap().flatten().map(|e| e.path())
        .filter(|p| { let n = p.file_name().unwrap().to_string_lossy().into_owned(); n.starts_with("metricas_") && n.ends_with(".csv") && !n.contains(".inicios") && !n.contains(".taxa") })
        .collect();
    met.sort();
    met
}

/// O mesmo, com o `candidatos.json` pré-eleição (só a circunscrição de cada cargo: Presidente uma vez, no Brasil).
#[test]
#[ignore]
fn primeiro_turno_2026_candidatos() {
    let u = uv_dados();
    let p = placar_com(&u.join("dados"), &u.join("oficial"), &metricas_2026(), &[], Some(&u.join("preparo_2026_1t/candidatos.json")));
    assert!(p.problemas.is_empty(), "{:?}", &p.problemas[..p.problemas.len().min(20)]);
    assert_eq!((p.circs, p.declaradas), (55, 55));
}

#[test]
#[ignore]
fn segundo_turno_2024_candidatos() {
    let u = uv_dados().join("2024-2t");
    let p = placar_com(&u.join("dados"), &u.join("oficial"), &[u.join("metricas_2t.csv")], &[], Some(&u.join("preparo/candidatos.json")));
    assert!(p.problemas.is_empty(), "{:?}", &p.problemas[..p.problemas.len().min(20)]);
    assert_eq!(p.circs, 51);
}

/// 1º turno de 2024 com a destinação presumida "Válido": os únicos problemas admitidos são declarações diferentes
/// do oficial em municípios cujo oficial tem candidato de destinação não apta (cassado ou indeferido depois);
/// revogação nenhuma.
#[test]
#[ignore]
fn primeiro_turno_2024_candidatos() {
    let u = uv_dados().join("2024-1t");
    let p = placar_com(&u.join("dados"), &u.join("oficial"), &[u.join("metricas_aux.csv")], &[], Some(&u.join("preparo/candidatos.json")));
    assert_eq!(p.circs, 5574);
    let ofs = Oficial::ler_pasta(&u.join("oficial")).unwrap();
    let nao_aptos: std::collections::HashSet<String> = ofs.iter()
        .filter(|o| o.cand.iter().any(|c| !urna_velox::candidatos::Destinacao::de_texto(&c.dvt).apto()))
        .filter_map(|o| o.abr.as_ref().map(|a| a.codigo())).collect();
    for x in &p.problemas {
        assert!(!x.contains("revogaç"), "{x}");
        let abr = x.split(' ').next().unwrap();
        assert!(nao_aptos.contains(abr), "problema fora de município com candidato não apto: {x}");
    }
    eprintln!("{} problemas, todos em municípios com candidato não apto: {:?}", p.problemas.len(), p.problemas);
}
