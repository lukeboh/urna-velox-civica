//! `uv-coletor preparar`: monta o `candidatos.json` de um pleito (docs/arquitetura/FORMATO_CANDIDATOS.md) só com arquivos
//! publicados **antes da apuração**, baixando-os com cache local:
//!
//! * `ele-c.json` — eleições do pleito, turno, cargos (majoritários e consultas; proporcionais ficam de fora) e
//!   abrangências de cada eleição;
//! * `mun-e<eleição>-cm.json` — municípios (e UFs) de cada eleição;
//! * `<uf>-p<pleito>-cs.json` — seções de cada UF que tem eleição;
//! * `<abr>-c<cargo>-e<eleição>-u.json` — candidatos (número, nome, nascimento, destinação quando houver) e
//!   eleitorado de cada abrangência publicada (Presidente: Brasil, UFs e exterior; cargos estaduais: UFs; Prefeito e
//!   consultas: município ou UF, conforme o `ele-c.json`).
//!
//! Só a abrangência que é a própria circunscrição do cargo vira circunscrição; as demais entram em `abrangencias`
//! (eleitorado e nome do arquivo a publicar). Arquivo ausente (HTTP 404) de uma UF ou abrangência que tem eleição é
//! registrado em `incompleto` e o comando termina com código 2: na noite, BU de seção fora do catálogo não é totalizado.
//!
//! Rede: no máximo 5 req/s, para no primeiro HTTP 429; o que está no cache não é baixado de novo, salvo
//! com `--atualizar`, que baixa de novo todas as fontes (`ele-c.json`, `cm.json`, `-u.json` e `cs.json` mudam até a
//! eleição: no 3221 o TSE publicou o eleitorado do TO só em 15/10; B-055).

use crate::coletor::{ConfigRede, Rede};
use crate::tse::{self, Alvo};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use urna_velox::candidatos::{circunscricao_de, AbrangenciaJson, ArquivoCandidatos, CandidatoJson, CargoJson, CircunscricaoJson, EleicaoJson, MunicipioJson, SecaoJson, ViceJson};
use urna_velox::catalogo::cargo_municipal;
use urna_velox::oficial::{Abrangencia, Oficial};

pub struct Opcoes {
    pub fontes: PathBuf,
    pub saida: PathBuf,
    /// sem rede: só o cache
    pub offline: bool,
    /// baixa de novo `ele-c.json` e `cs.json`
    pub atualizar: bool,
    /// ignora o `dvt` dos arquivos (simula a pré-apuração com arquivos já apurados): toda destinação = "Válido"
    pub ignorar_destinacao: bool,
    /// só estes cargos (vazio = todos os majoritários e consultas)
    pub cargos: Vec<u8>,
}

struct Baixador<'a> {
    rt: &'a tokio::runtime::Runtime,
    rede: Option<Rede>,
    pub baixados: usize,
    pub do_cache: usize,
}

impl Baixador<'_> {
    /// Conteúdo de `nome` (procura em `<fontes>/<sub>/<nome>` e `<fontes>/<nome>`); baixa de `url` se faltar ou se `forcar`.
    /// `Ok(None)` = HTTP 404 (ou ausente no modo offline).
    fn obter(&mut self, fontes: &Path, sub: &str, nome: &str, url: &str, forcar: bool) -> anyhow::Result<Option<Vec<u8>>> {
        let destino = fontes.join(sub).join(nome);
        if !forcar {
            for p in [destino.clone(), fontes.join(nome)] {
                if p.is_file() { self.do_cache += 1; return Ok(Some(std::fs::read(&p)?)); }
            }
        }
        let Some(rede) = &self.rede else {
            if destino.is_file() { self.do_cache += 1; return Ok(Some(std::fs::read(&destino)?)); }
            return Ok(None);
        };
        let r = self.rt.block_on(rede.get(url, None, None))?;
        self.baixados += 1;
        match r.status {
            200 => {
                std::fs::create_dir_all(destino.parent().unwrap())?;
                std::fs::write(&destino, &r.corpo)?;
                Ok(Some(r.corpo.to_vec()))
            }
            404 | 403 => Ok(None),
            s => anyhow::bail!("{url}: HTTP {s}"),
        }
    }
}

fn txt(v: &Value, k: &str) -> String { v.get(k).and_then(Value::as_str).unwrap_or("").to_string() }

/// Do `-u.json`: número → (nome de urna, sigla do partido, vices/suplentes), de `carg[].agr[].par[].cand[]` (para o
/// site; o motor não usa). Consultas não têm partido: o mapa fica vazio e o nome de urna é o nome da opção.
fn extras_candidatos(v: &Value) -> std::collections::HashMap<u32, (String, String, Vec<ViceJson>)> {
    let mut m = std::collections::HashMap::new();
    for c in v["carg"].as_array().into_iter().flatten() {
        for a in c["agr"].as_array().into_iter().flatten() {
            for p in a["par"].as_array().into_iter().flatten() {
                let sg = txt(p, "sg");
                for k in p["cand"].as_array().into_iter().flatten() {
                    let Ok(n) = txt(k, "n").parse::<u32>() else { continue };
                    let vs = k["vs"].as_array().into_iter().flatten().map(|x| ViceJson { tp: txt(x, "tp"), nome: txt(x, "nm"), nome_urna: txt(x, "nmu"), partido: txt(x, "sgp") }).collect();
                    m.insert(n, (txt(k, "nmu"), sg.clone(), vs));
                }
            }
        }
    }
    m
}

/// Monta e grava o `candidatos.json`. Devolve as pendências (vazio = completo).
pub fn preparar(rt: &tokio::runtime::Runtime, alvo: &Alvo, op: &Opcoes) -> anyhow::Result<Vec<String>> {
    let t0 = std::time::Instant::now();
    std::fs::create_dir_all(&op.fontes)?;
    std::fs::create_dir_all(&op.saida)?;
    let rede = if op.offline { None } else {
        Some(Rede::nova(&ConfigRede::simples(1, 5.0, 2, 30, 1))?)
    };
    let mut bx = Baixador { rt, rede, baixados: 0, do_cache: 0 };
    let base = format!("{}/{}/{}", alvo.base, alvo.ambiente, alvo.ciclo);
    let mut incompleto = vec![];
    // candidatos sem `dvt` no arquivo (pré-apuração): destinação presumida "Válido"
    let mut sem_dvt = 0usize;

    // 1. ele-c.json
    let ele_c = bx.obter(&op.fontes, "", "ele-c.json", &alvo.url_ele_c(), op.atualizar)?.ok_or_else(|| anyhow::anyhow!("ele-c.json indisponível"))?;
    let ele_c: Value = serde_json::from_slice(&ele_c)?;
    let pl = ele_c["pl"].as_array().into_iter().flatten().find(|p| txt(p, "cd") == alvo.pleito.to_string())
        .ok_or_else(|| anyhow::anyhow!("pleito {} não está no ele-c.json", alvo.pleito))?;

    // código → (UF, nome do município); UF → nome (do cm.json: nomes para o site, sem requisição a mais)
    let mut municipios: BTreeMap<u32, (String, String)> = BTreeMap::new();
    let mut nomes_ufs: BTreeMap<String, String> = BTreeMap::new();
    let mut ufs: BTreeSet<String> = BTreeSet::new();
    let mut eleicoes = vec![];
    for el in pl["e"].as_array().into_iter().flatten() {
        let codigo: u32 = txt(el, "cd").parse().unwrap_or(0);
        let turno: u8 = txt(el, "t").parse().unwrap_or(1);
        // cargos majoritários (tp 1) e consultas (tp 3) e as abrangências de cada um no ele-c.json
        let mut cargos_abr: BTreeMap<u8, (bool, Vec<(String, Vec<u32>)>)> = BTreeMap::new();
        for a in el["abr"].as_array().into_iter().flatten() {
            let mus: Vec<u32> = a["mu"].as_array().into_iter().flatten().filter_map(|m| txt(m, "cd").parse().ok()).collect();
            for c in a["cp"].as_array().into_iter().flatten() {
                let tp = txt(c, "tp");
                if tp == "2" { continue; } // proporcional: fora do escopo (3.0)
                let Ok(cargo) = txt(c, "cd").parse::<u8>() else { continue };
                if !op.cargos.is_empty() && !op.cargos.contains(&cargo) { continue; }
                let e = cargos_abr.entry(cargo).or_insert((tp == "3", vec![]));
                e.1.push((txt(a, "cd").to_lowercase(), mus.clone()));
            }
        }
        if cargos_abr.is_empty() { continue; }
        // municípios da eleição (cm.json)
        let nome_cm = format!("mun-e{codigo:06}-cm.json");
        let cm = bx.obter(&op.fontes, "cm", &nome_cm, &format!("{base}/{codigo}/config/{nome_cm}"), op.atualizar)?;
        let Some(cm) = cm else { incompleto.push(format!("{nome_cm} ausente")); continue };
        let cm: Value = serde_json::from_slice(&cm)?;
        let mut munis_el: Vec<(String, u32)> = vec![];
        for a in cm["abr"].as_array().into_iter().flatten() {
            let uf = txt(a, "cd").to_lowercase();
            let ds = txt(a, "ds");
            if !ds.is_empty() { nomes_ufs.entry(uf.clone()).or_insert(ds); }
            for m in a["mu"].as_array().into_iter().flatten() {
                if let Ok(mu) = txt(m, "cd").parse::<u32>() {
                    munis_el.push((uf.clone(), mu));
                    let e = municipios.entry(mu).or_insert_with(|| (uf.clone(), String::new()));
                    if e.1.is_empty() { e.1 = txt(m, "nm"); }
                }
            }
            ufs.insert(uf);
        }
        let ufs_el: BTreeSet<String> = munis_el.iter().map(|x| x.0.clone()).collect();
        // arquivos -u.json de cada cargo
        let mut cargos = vec![];
        for (cargo, (consulta, abrs)) in cargos_abr {
            // abrangências publicadas
            let mut lista: Vec<Abrangencia> = vec![];
            for (cd, mus) in &abrs {
                let uf_alvo = |u: &str| cd == "br" || cd == u;
                if cargo_municipal(cargo) && (!mus.is_empty() || cargo < 25) {
                    // Prefeito, ou consulta com municípios no ele-c.json: um arquivo por município (os do ele-c.json,
                    // ou, sem a lista, todos os do cm.json na abrangência). Consulta sem municípios: arquivo da abrangência.
                    let alvo_mus: Vec<(String, u32)> = if mus.is_empty() { munis_el.iter().filter(|x| uf_alvo(&x.0)).cloned().collect() }
                        else { mus.iter().map(|&m| (cd.clone(), m)).collect() };
                    for (u, m) in alvo_mus { lista.push(Abrangencia::Municipio(u.as_bytes()[..2].try_into()?, m)); }
                } else if cd == "br" {
                    if cargo == 1 { lista.push(Abrangencia::Brasil); }
                    for u in &ufs_el { if cargo == 1 || u != "zz" { lista.push(Abrangencia::Uf(u.as_bytes()[..2].try_into()?)); } }
                } else {
                    lista.push(Abrangencia::Uf(cd.as_bytes()[..2].try_into()?));
                }
            }
            lista.sort();
            lista.dedup();
            let (mut vagas, mut circs, mut abrangencias) = (1u8, vec![], vec![]);
            for abr in &lista {
                let dir = match abr { Abrangencia::Brasil => "br".to_string(), Abrangencia::Uf(u) | Abrangencia::Municipio(u, _) => String::from_utf8_lossy(u).into_owned() };
                let nome = format!("{}-c{cargo:04}-e{codigo:06}-u.json", abr.codigo());
                // B-055: o eleitorado e as seções esperadas dos -u.json pré-apuração mudam até a eleição (o TSE os publica aos poucos)
                let Some(b) = bx.obter(&op.fontes, "u", &nome, &format!("{base}/{codigo}/dados/{dir}/{nome}"), op.atualizar)? else {
                    incompleto.push(format!("{nome} ausente"));
                    continue;
                };
                let v: Value = serde_json::from_slice(&b)?;
                let of = Oficial::de_json(&v);
                vagas = vagas.max(of.vagas);
                abrangencias.push(AbrangenciaJson { abr: abr.codigo(), eleitorado: of.eleitorado, secoes: of.secoes });
                if circunscricao_de(cargo, abr) != *abr { continue; }
                sem_dvt += of.cand.iter().filter(|c| c.dvt.is_empty()).count();
                let extras = extras_candidatos(&v);
                let candidatos = of.cand.iter().map(|c| {
                    let x = extras.get(&c.numero);
                    CandidatoJson {
                        numero: c.numero, nome: c.nome.clone(),
                        nome_urna: x.map(|x| x.0.clone()).filter(|s| !s.is_empty()).unwrap_or_else(|| c.nome.clone()),
                        destinacao: if op.ignorar_destinacao || c.dvt.is_empty() { "Válido".into() } else { c.dvt.clone() },
                        nascimento: (!c.nascimento.is_empty()).then(|| c.nascimento.clone()),
                        partido: x.map(|x| x.1.clone()).unwrap_or_default(),
                        vices: x.map(|x| x.2.clone()).unwrap_or_default(),
                    }
                }).collect();
                circs.push(CircunscricaoJson { abr: abr.codigo(), eleitorado: of.eleitorado, candidatos });
            }
            cargos.push(CargoJson { cargo, vagas, consulta, circunscricoes: circs, abrangencias });
        }
        eleicoes.push(EleicaoJson { codigo, turno, cargos, municipios: munis_el.iter().map(|(u, m)| format!("{u}{m:05}")).collect() });
    }

    // seções: cs.json de cada UF com eleição
    let mut secoes = vec![];
    let mut ufs_com_secoes = 0;
    for uf in &ufs {
        let nome = format!("{uf}-p{:06}-cs.json", alvo.pleito);
        let Some(b) = bx.obter(&op.fontes, "cs", &nome, &alvo.url_cs(uf), op.atualizar)? else {
            incompleto.push(format!("{nome} ausente (UF com eleição; seções desconhecidas)"));
            continue;
        };
        let c: tse::ConfigSecoes = serde_json::from_slice(&b)?;
        let agregadas = c.agregadas();
        for s in c.secoes(uf) {
            let agregada = agregadas.contains(&s.chave());
            secoes.push(SecaoJson { municipio: s.municipio, zona: s.zona, secao: s.secao, agregada });
        }
        ufs_com_secoes += 1;
    }
    secoes.sort_by_key(|s| (s.municipio, s.zona, s.secao));
    secoes.dedup_by_key(|s| (s.municipio, s.zona, s.secao));

    let n_presumidas = eleicoes.iter().flat_map(|e| e.cargos.iter()).flat_map(|c| c.circunscricoes.iter()).flat_map(|c| c.candidatos.iter()).count();
    let arq = ArquivoCandidatos {
        versao: 1, pleito: alvo.pleito,
        ufs: ufs.iter().cloned().collect(),
        municipios: municipios.into_iter().map(|(codigo, (uf, nome))| MunicipioJson { codigo, uf, nome }).collect(),
        secoes, eleicoes,
        destinacao_presumida: op.ignorar_destinacao || sem_dvt > 0,
        incompleto: incompleto.clone(),
        nomes_ufs,
    };
    let p = op.saida.join("candidatos.json");
    std::fs::write(&p, serde_json::to_string_pretty(&arq)?)?;
    eprintln!("{}: pleito {}, {} UFs ({} com seções), {} municípios, {} seções, {} eleições, {} candidatos; {} arquivos do cache, {} requisições; {:.2} s",
        p.display(), alvo.pleito, arq.ufs.len(), ufs_com_secoes, arq.municipios.len(), arq.secoes.len(), arq.eleicoes.len(), n_presumidas,
        bx.do_cache, bx.baixados, t0.elapsed().as_secs_f64());
    if arq.destinacao_presumida {
        eprintln!("destinação presumida \"Válido\" ({}): confira as cassações e indeferimentos antes da noite",
            if op.ignorar_destinacao { "--ignorar-destinacao".to_string() } else { format!("{sem_dvt} candidatos sem dvt") });
    }
    for x in &incompleto { eprintln!("  INCOMPLETO: {x}"); }
    Ok(incompleto)
}

#[cfg(test)]
mod testes {
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
}
