//! Situação dos candidatos × divulgação oficial, só com os `-u.json` (os votos do próprio arquivo): para cada arquivo
//! de `fixtures/oficial-majoritarios/`, o motor calcula a situação de cada candidato com os votos publicados (`vap`)
//! e a destinação (`dvt`) e confere com o `st` do TSE.
//!
//! B-070 (`docs/BUGS.md`): no suplementar de Governador de RR (pleito 3237, eleição 6278, 21/06/2026) o mais votado
//! (ARTHUR HENRIQUE, 60,87 %) está "Anulado sub judice"; o TSE publica todos "Não eleito" (`esae = s`, `mnae`: maior
//! votação nominal anulada e anulados acima de 50 % da votação nominal — Código Eleitoral, art. 224), e o motor dava
//! SOLDADO SAMPAIO "Eleito". Os outros arquivos são controles de 2026 com candidatos anulados que não mudam o resultado
//! (Senador do AC e de SE, Governador do RJ e do MA).

use std::path::PathBuf;
use urna_velox::candidatos::Candidatos;
use urna_velox::oficial::Oficial;
use urna_velox::situacao::{situacao_final, Situacao};

fn oficiais() -> Vec<(String, Oficial)> {
    let d = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/oficial-majoritarios");
    let mut v: Vec<(String, Oficial)> = std::fs::read_dir(&d).unwrap().flatten()
        .map(|e| { let n = e.file_name().to_string_lossy().into_owned(); (n, Oficial::ler(&e.path()).unwrap()) }).collect();
    v.sort_by(|a, b| a.0.cmp(&b.0));
    v
}

/// (arquivo, número, situação do motor, situação oficial) de cada candidato que diverge.
fn divergencias(nome: &str, of: &Oficial) -> Vec<(String, u32, &'static str, String)> {
    let cad = Candidatos::dos_oficiais(std::slice::from_ref(of));
    let abr = of.abr.clone().expect("abrangência");
    let circ = cad.circunscricao(of.cargo, &abr).expect("circunscrição");
    let votos: Vec<u64> = circ.candidatos.iter().map(|c| of.cand.iter().find(|x| x.numero == c.numero).map(|x| x.vap).unwrap_or(0)).collect();
    let st = situacao_final(circ, &votos);
    let mut r = vec![];
    for (i, c) in circ.candidatos.iter().enumerate() {
        let oficial = of.cand.iter().find(|x| x.numero == c.numero).map(|x| x.st.clone()).unwrap_or_default();
        if st[i].texto() != oficial { r.push((nome.to_string(), c.numero, st[i].texto(), oficial)); }
    }
    r
}

#[test]
fn situacao_dos_majoritarios_igual_a_oficial() {
    let ofs = oficiais();
    assert_eq!(ofs.len(), 5, "arquivos da fixture");
    let mut todas = vec![];
    for (n, of) in &ofs { todas.extend(divergencias(n, of)); }
    assert!(todas.is_empty(), "situações diferentes do oficial: {todas:?}");
}

#[test]
fn b070_governador_suplementar_de_rr_sem_eleito() {
    let (n, of) = oficiais().into_iter().find(|x| x.0 == "rr-c0003-e006278-u.json").unwrap();
    assert!(divergencias(&n, &of).is_empty(), "{:?}", divergencias(&n, &of));
    let cad = Candidatos::dos_oficiais(std::slice::from_ref(&of));
    let circ = cad.circunscricao(3, of.abr.as_ref().unwrap()).unwrap();
    let votos: Vec<u64> = circ.candidatos.iter().map(|c| of.cand.iter().find(|x| x.numero == c.numero).unwrap().vap).collect();
    assert!(situacao_final(circ, &votos).iter().all(|s| *s == Situacao::NaoEleito));
}

/// O mesmo, para todos os `-u.json` de uma pasta (`UV_OFICIAIS`): ex. os 139 de Presidente, Governador e Senador dos
/// dois turnos de 2026, ou os de Prefeito de 2024. `UV_OFICIAIS=<pasta> cargo test -p urna-velox --test
/// situacao_oficial -- --ignored` (várias pastas: separadas por `:`). Sem `UV_OFICIAIS` (B-072: a conferência do
/// CLAUDE.md, só com `UV_DADOS`), as pastas de oficiais de `UV_DADOS`: `oficial/` (2026, 1º turno), `2024-1t/oficial/`
/// e `2024-2t/oficial/`. Arquivos com a totalização não finalizada (`tf = n`) ou sem circunscrição própria (UF de
/// Presidente) ficam de fora.
#[test]
#[ignore]
fn situacao_igual_a_oficial_em_uma_pasta() {
    let pastas: Vec<PathBuf> = match std::env::var("UV_OFICIAIS") {
        Ok(v) => v.split(':').filter(|s| !s.is_empty()).map(PathBuf::from).collect(),
        Err(_) => {
            let u = PathBuf::from(std::env::var("UV_DADOS").expect("defina UV_OFICIAIS (pastas com -u.json) ou UV_DADOS (ex.: /home/dev/uvreal)"));
            ["oficial", "2024-1t/oficial", "2024-2t/oficial"].iter().map(|p| u.join(p)).collect()
        }
    };
    for d in &pastas { assert!(d.is_dir(), "pasta de oficiais inexistente: {}", d.display()); }
    let (mut n, mut todas) = (0, vec![]);
    for e in pastas.iter().flat_map(|d| std::fs::read_dir(d).unwrap().flatten()) {
        let nome = e.file_name().to_string_lossy().into_owned();
        if !nome.ends_with("-u.json") { continue; }
        let Ok(of) = Oficial::ler(&e.path()) else { continue };
        let Some(abr) = of.abr.clone() else { continue };
        if of.consulta || !of.finalizada || urna_velox::candidatos::circunscricao_de(of.cargo, &abr) != abr { continue; }
        n += 1;
        todas.extend(divergencias(&nome, &of));
    }
    eprintln!("{n} arquivos conferidos, {} situações divergentes", todas.len());
    assert!(n > 0);
    assert!(todas.is_empty(), "situações diferentes do oficial: {todas:?}");
}
