//! Apoio aos testes de integração: fixtures versionadas (`tests/fixtures`) e dados locais completos (`UV_DADOS`).
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering::Relaxed;
use urna_velox::candidatos::Candidatos;
use urna_velox::catalogo::Catalogo;
use urna_velox::conferencia::{conferir, Relatorio};
use urna_velox::estado::{totalizar_lote, Estado};
use urna_velox::metricas::Metricas;
use urna_velox::oficial::Oficial;
use urna_velox::pacote::{EmMemoria, Escritor, Registro};

pub fn fixtures() -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("fixtures") }

/// Pasta dos dados completos (`UV_DADOS`, ex.: /home/dev/uvreal); os testes `#[ignore]` falham sem ela.
pub fn uv_dados() -> PathBuf {
    PathBuf::from(std::env::var("UV_DADOS").expect("defina UV_DADOS (ex.: /home/dev/uvreal) para os testes de integração completos"))
}

/// Um conjunto de dados carregado e totalizado.
pub struct Conjunto {
    pub mem: EmMemoria,
    pub met: Metricas,
    pub cat: Catalogo,
    pub est: Estado,
}

/// Carrega os pacotes de `dados`, monta o catálogo pelos próprios BUs, aplica as seções anuladas das métricas e
/// totaliza com `threads` threads. Confere que a totalização não deu nenhum alarme.
pub fn totalizar(dados: &Path, metricas: &[PathBuf], vagas: &[(u8, u8)], threads: usize) -> Conjunto {
    let mem = EmMemoria::carregar(dados, None).expect("pacotes");
    assert!(!mem.itens.is_empty(), "nenhum BU em {}", dados.display());
    let ps: Vec<&Path> = metricas.iter().map(|p| p.as_path()).collect();
    let met = Metricas::ler(&ps).expect("métricas");
    let (mut cat, ilegiveis) = Catalogo::dos_pacotes(&mem, vagas);
    assert_eq!(ilegiveis, 0, "BUs ilegíveis ao montar o catálogo");
    cat.usar_anuladas(&met);
    let (est, _) = totalizar_lote(&cat, &mem, threads);
    let distintos = mem.itens.iter().map(|x| (x.0.uf, x.0.chave())).collect::<std::collections::HashSet<_>>().len();
    let obtido = [est.bus.load(Relaxed), est.cargos_invalidos.load(Relaxed), est.desconhecidos.load(Relaxed), est.duplicados.load(Relaxed),
        est.fora_catalogo.load(Relaxed), est.divergentes.load(Relaxed), est.erros.load(Relaxed)];
    let esperado = [distintos as u64, 0, 0, (mem.itens.len() - distintos) as u64, 0, 0, 0];
    assert_eq!(obtido, esperado, "[BUs, cargos inválidos, desconhecidos, duplicados, fora do catálogo, divergentes, erros]");
    Conjunto { mem, met, cat, est }
}

/// Totaliza usando diretamente um `candidatos.json` pré-eleição para construir o catálogo e os candidatos.
pub fn totalizar_com_candidatos(dados: &Path, metricas: &[PathBuf], candidatos_json: &Path, threads: usize) -> (Conjunto, Candidatos) {
    let mem = EmMemoria::carregar(dados, None).expect("pacotes");
    assert!(!mem.itens.is_empty(), "nenhum BU em {}", dados.display());
    let ps: Vec<&Path> = metricas.iter().map(|p| p.as_path()).collect();
    let met = Metricas::ler(&ps).expect("métricas");
    let (cad, arq) = Candidatos::ler(candidatos_json).expect("ler candidatos.json");
    let mut cat = Catalogo::de_arquivo_candidatos(&arq).expect("catálogo do candidatos.json");
    cat.usar_anuladas(&met);
    let (est, _) = totalizar_lote(&cat, &mem, threads);
    let distintos = mem.itens.iter().map(|x| (x.0.uf, x.0.chave())).collect::<std::collections::HashSet<_>>().len();
    let obtido = [est.bus.load(Relaxed), est.cargos_invalidos.load(Relaxed), est.duplicados.load(Relaxed),
        est.fora_catalogo.load(Relaxed), est.divergentes.load(Relaxed), est.erros.load(Relaxed)];
    let esperado = [distintos as u64, 0, (mem.itens.len() - distintos) as u64, 0, 0, 0];
    assert_eq!(obtido, esperado, "[BUs, cargos inválidos, duplicados, fora do catálogo, divergentes, erros]");
    (Conjunto { mem, met, cat, est }, cad)
}

/// Confere o estado com todos os `-u.json` de `oficial`: votos e, com os candidatos montados pelos mesmos arquivos,
/// a situação de cada candidato calculada pelo motor.
pub fn conferir_pasta(c: &Conjunto, oficial: &Path) -> Relatorio {
    let ofs = Oficial::ler_pasta(oficial).expect("arquivos oficiais");
    let cad = Candidatos::dos_oficiais(&ofs);
    conferir(&c.cat, &c.est, &c.mem, &ofs, &c.met, Some(&cad))
}

pub fn conferir_pasta_com_candidatos(c: &Conjunto, oficial: &Path, cad: &Candidatos) -> Relatorio {
    let ofs = Oficial::ler_pasta(oficial).expect("arquivos oficiais");
    conferir(&c.cat, &c.est, &c.mem, &ofs, &c.met, Some(cad))
}

/// Gera a publicação encerrada (JSON de docs/FORMATO_RESULTADO.md) de cada arquivo oficial e confere com ele.
pub fn conferir_publicacoes_pasta(c: &Conjunto, oficial: &Path) -> Relatorio {
    let ofs = Oficial::ler_pasta(oficial).expect("arquivos oficiais");
    let cad = Candidatos::dos_oficiais(&ofs);
    let p = urna_velox::resultado::Publicador::novo(&c.cat, Some(&cad), Some(&c.met));
    urna_velox::conferencia::conferir_publicacoes(&p, &c.est, &ofs, urna_velox::resultado::Momento { encerrada: true, seq: c.est.bus.load(Relaxed), ultimo_bu: None })
}

pub fn conferir_publicacoes_com_candidatos(c: &Conjunto, oficial: &Path, cad: &Candidatos) -> Relatorio {
    let ofs = Oficial::ler_pasta(oficial).expect("arquivos oficiais");
    let p = urna_velox::resultado::Publicador::novo(&c.cat, Some(cad), Some(&c.met));
    urna_velox::conferencia::conferir_publicacoes(&p, &c.est, &ofs, urna_velox::resultado::Momento { encerrada: true, seq: c.est.bus.load(Relaxed), ultimo_bu: None })
}

/// Divergências esperadas com a destinação presumida "Válido" (candidatos.json pré-eleição montado sem `dvt`):
/// só de situação, e só em arquivos cujo oficial tem candidato com destinação não apta (cassado, indeferido…).
/// Os votos têm de ser idênticos. Devolve (arquivos, situações) divergentes.
pub fn divergencias_por_destinacao(r: &urna_velox::conferencia::Relatorio, oficial: &Path) -> (usize, usize) {
    let ofs = Oficial::ler_pasta(oficial).expect("arquivos oficiais");
    let por_nome: std::collections::HashMap<String, &Oficial> = ofs.iter()
        .map(|o| (o.arquivo.file_name().unwrap().to_string_lossy().into_owned(), o)).collect();
    let mut sit = 0;
    for d in &r.divergentes {
        let of = por_nome[&d.arquivo];
        assert!(of.cand.iter().any(|c| !urna_velox::candidatos::Destinacao::de_texto(&c.dvt).apto()),
            "{}: divergência sem nenhum candidato de destinação não apta no oficial: {:?}", d.arquivo, d.difs);
        for x in &d.difs {
            assert!(x.starts_with("situação"), "{}: divergência que não é de situação: {x}", d.arquivo);
            sit += 1;
        }
    }
    (r.divergentes.len(), sit)
}

/// Grava BUs soltos (`o<pleito><uf><município><zona><seção>-bu.dat`) num pacote temporário e o carrega.
/// Cada chamada tem o seu diretório (processo + contador): os testes de um mesmo arquivo rodam em threads do mesmo
/// processo e, com o mesmo rótulo, apagavam e regravavam o pacote uns dos outros (falhas intermitentes em `retirada`).
pub fn pacote_de_soltos(rotulo: &str, arqs: &[PathBuf]) -> (PathBuf, EmMemoria) {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("uv-teste-{}-{n}-{rotulo}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let mut e = Escritor::novo(&dir).unwrap();
    for p in arqs {
        let n = p.file_name().unwrap().to_str().unwrap();
        let r = Registro { pleito: n[1..6].parse().unwrap(), uf: n.as_bytes()[6..8].try_into().unwrap(), municipio: n[8..13].parse().unwrap(),
            zona: n[13..17].parse().unwrap(), secao: n[17..21].parse().unwrap(), situacao: 1, origem: 2, sha256_vsc: [0; 32] };
        e.gravar(&r, &std::fs::read(p).unwrap()).unwrap();
    }
    e.fechar().unwrap();
    let mem = EmMemoria::carregar(&dir, None).unwrap();
    (dir, mem)
}
