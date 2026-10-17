//! Ensaios do `uv-servico` contra o `uv-simulador` (etapa 3, critérios A e C), com os dados locais (`UV_DADOS`):
//! simulador e serviço como processos filhos, porta livre escolhida pelo simulador. Rodam com
//! `UV_DADOS=/home/dev/uvreal cargo test --release -p uv-aquisicao -- --ignored --test-threads 1`.
//!
//! Desde a etapa 4, também a difusão (`--http`, `docs/PROTOCOLO_WS.md`): D4 reduzido com um cliente WebSocket no
//! próprio teste, clientes lentos e `kill -9` com reconexão (os dois com o `uv-carga`).
//!
//! O critério A completo (20 UFs a 10×) e o B (2026 a 60×) ficam fora dos testes (25 e 15 min); aqui o 2º turno de
//! 2024 vai reduzido a AM e RO (2 municípios, 5.102 BUs) a 120×, e o caso real de anulação (MA 07315/0023/0354, 1º
//! turno de 2024) só com o MA.

use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

fn uv() -> PathBuf { PathBuf::from(std::env::var("UV_DADOS").expect("defina UV_DADOS (ex.: /home/dev/uvreal)")) }

/// Conjunto de dados
struct Conj { dados: PathBuf, cs: PathBuf, metricas: String, oficial: PathBuf, pleito: u32, ciclo: &'static str, candidatos: PathBuf }

fn conj(nome: &str) -> Conj {
    let u = uv();
    match nome {
        "2024-2t" => Conj { dados: u.join("2024-2t/dados"), cs: u.join("2024-2t/dados/config"), metricas: u.join("2024-2t/metricas_2t.csv").display().to_string(),
            oficial: u.join("2024-2t/oficial"), pleito: 453, ciclo: "ele2024", candidatos: u.join("2024-2t/preparo/candidatos.json") },
        "2024-1t" => Conj { dados: u.join("2024-1t/dados"), cs: u.join("2024-1t/dados/config"), metricas: u.join("2024-1t/metricas_aux.csv").display().to_string(),
            oficial: u.join("2024-1t/oficial"), pleito: 452, ciclo: "ele2024", candidatos: u.join("2024-1t/preparo/candidatos.json") },
        // 1º turno de 2026 (pleito 3220): pacotes, cs.json e oficiais na raiz; as métricas da coleta nacional têm as
        // 41 seções do exterior "Nao instalada"
        "2026-1t" => Conj { dados: u.join("dados"), cs: u.join("config"), metricas: u.join("metricas_nacional.csv").display().to_string(),
            oficial: u.join("oficial"), pleito: 3220, ciclo: "ele2026", candidatos: u.join("etapa5/preparo-2026-1t/candidatos.json") },
        _ => panic!("conjunto {nome}"),
    }
}

/// Pasta temporária de um ensaio: apagada quando o ensaio termina bem (fica se ele falhar, para o diagnóstico, ou com
/// `UV_MANTER_ENSAIO=1`). Antes de criar, apaga as sobras de execuções anteriores cujo processo já morreu (o nome leva
/// o PID, que muda a cada execução: sem isto as pastas, até centenas de MB cada, se acumulavam no TMPDIR).
struct Pasta(PathBuf);
impl std::ops::Deref for Pasta { type Target = PathBuf; fn deref(&self) -> &PathBuf { &self.0 } }
impl Drop for Pasta {
    fn drop(&mut self) {
        if !std::thread::panicking() && std::env::var_os("UV_MANTER_ENSAIO").is_none() { let _ = std::fs::remove_dir_all(&self.0); }
    }
}

/// Apaga `<TMPDIR>/<prefixo><pid>-*` de processos que não existem mais.
fn limpar_sobras(prefixo: &str) {
    let Ok(rd) = std::fs::read_dir(std::env::temp_dir()) else { return };
    for e in rd.flatten() {
        let n = e.file_name().to_string_lossy().into_owned();
        let Some(resto) = n.strip_prefix(prefixo) else { continue };
        let Some(pid) = resto.split('-').next().and_then(|p| p.parse::<u32>().ok()) else { continue };
        if pid != std::process::id() && !Path::new(&format!("/proc/{pid}")).exists() { let _ = std::fs::remove_dir_all(e.path()); }
    }
}

fn pasta(nome: &str) -> Pasta {
    limpar_sobras("uv-ensaio-");
    let d = std::env::temp_dir().join(format!("uv-ensaio-{}-{nome}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    Pasta(d)
}

struct Simulador { filho: Child, porta: u16 }
impl Drop for Simulador { fn drop(&mut self) { let _ = self.filho.kill(); let _ = self.filho.wait(); } }

fn simulador(dir: &Path, c: &Conj, ufs: &str, acel: f64, extra: &[&str]) -> Simulador {
    let pa = dir.join("porta");
    let filho = Command::new(env!("CARGO_BIN_EXE_uv-simulador"))
        .args(["--dados", c.dados.to_str().unwrap(), "--cs", c.cs.to_str().unwrap(), "--metricas", &c.metricas, "--oficial", c.oficial.to_str().unwrap(),
            "--pleito", &c.pleito.to_string(), "--ciclo", c.ciclo, "--ufs", ufs, "--acelerar", &acel.to_string(),
            "--log", dir.join("simulador.log").to_str().unwrap(), "--porta-arquivo", pa.to_str().unwrap()])
        .args(extra).stdout(Stdio::null()).stderr(std::fs::File::create(dir.join("simulador.err")).unwrap()).spawn().unwrap();
    let t = Instant::now();
    loop {
        if let Some(p) = std::fs::read_to_string(&pa).ok().and_then(|s| s.trim().parse().ok()) { return Simulador { filho, porta: p }; }
        assert!(t.elapsed() < Duration::from_secs(60), "o simulador não abriu a porta");
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn servico(dir: &Path, c: &Conj, ufs: &str, porta: u16, acel: f64) -> Child { servico_com(dir, c, ufs, porta, acel, &[]) }

fn servico_com(dir: &Path, c: &Conj, ufs: &str, porta: u16, acel: f64, extra: &[&str]) -> Child {
    Command::new(env!("CARGO_BIN_EXE_uv-servico"))
        .args(["--pleito", &c.pleito.to_string(), "--ciclo", c.ciclo, "--candidatos", c.candidatos.to_str().unwrap(),
            "--dados", dir.join("dados").to_str().unwrap(), "--saida", dir.join("saida").to_str().unwrap(),
            "--base", &format!("http://127.0.0.1:{porta}"), "--aceleracao-local", &acel.to_string(), "--ufs", ufs])
        .args(extra)
        .stdout(Stdio::null()).stderr(std::fs::OpenOptions::new().create(true).append(true).open(dir.join("servico.err")).unwrap()).spawn().unwrap()
}

/// Porta livre agora (para o serviço manter a mesma porta HTTP entre reinícios).
fn porta_livre() -> u16 { std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port() }

/// `uv-carga` como processo filho; devolve o filho e o caminho do relatório.
fn carga(dir: &Path, http: u16, args: &[&str]) -> (Child, PathBuf) {
    let rel = dir.join("carga.json");
    let f = Command::new(env!("CARGO_BIN_EXE_uv-carga"))
        .args(["--endereco", &format!("127.0.0.1:{http}"), "--publicacao", dir.join("saida/publicacao").to_str().unwrap(),
            "--eventos-csv", dir.join("saida/eventos.csv").to_str().unwrap(), "--relatorio", rel.to_str().unwrap(), "--duracao", "600"])
        .args(args).stdout(Stdio::null()).stderr(std::fs::File::create(dir.join("carga.err")).unwrap()).spawn().unwrap();
    (f, rel)
}

fn esperar_carga(mut f: Child, rel: &Path) -> Value {
    let t = Instant::now();
    loop {
        if let Some(st) = f.try_wait().unwrap() { assert!(st.success(), "uv-carga saiu com {st}"); return json(rel); }
        if t.elapsed() > Duration::from_secs(120) { let _ = f.kill(); panic!("uv-carga não terminou 120 s depois do serviço"); }
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn esperar(mut f: Child, limite_s: u64) {
    let t = Instant::now();
    loop {
        if let Some(st) = f.try_wait().unwrap() { assert!(st.success(), "uv-servico saiu com {st}"); return; }
        if t.elapsed() > Duration::from_secs(limite_s) { let _ = f.kill(); panic!("uv-servico não terminou em {limite_s} s"); }
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn json(p: &Path) -> Value { serde_json::from_slice(&std::fs::read(p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))).unwrap() }

/// Publicação final sem os campos que dependem do instante (horário do último BU e o "matematicamente" de cada
/// candidato, que tem a sequência e a hora em que aconteceu).
fn publicacao(dir: &Path) -> BTreeMap<String, Value> {
    let mut out = BTreeMap::new();
    for e in std::fs::read_dir(dir.join("saida/publicacao")).unwrap().flatten() {
        let n = e.file_name().to_string_lossy().into_owned();
        if !n.ends_with(".json") || n.starts_with('.') { continue; }
        let mut v = json(&e.path());
        v.as_object_mut().unwrap().remove("ultimo_bu");
        for c in v["cand"].as_array_mut().unwrap() { c.as_object_mut().unwrap().remove("mat"); }
        out.insert(n, v);
    }
    out
}

/// Confere o fim de um ensaio: conferência sem divergência, nenhum duplicado, nenhuma revogação, parada limpa.
fn conferido(dir: &Path, arquivos: u64) -> Value {
    let c = json(&dir.join("saida/conferencia.json"));
    assert_eq!(c["arquivos"].as_u64(), Some(arquivos), "{c}");
    assert_eq!(c["identicos"].as_u64(), Some(arquivos), "divergências: {}", c["divergentes"]);
    assert_eq!(c["situacoes"]["divergentes"].as_u64(), Some(0));
    assert_eq!(c["matematicamente"]["revogacoes"].as_u64(), Some(0));
    let s = json(&dir.join("saida/saude.json"));
    assert_eq!(s["motor"]["duplicados"].as_u64(), Some(0), "{}", s["motor"]);
    assert_eq!(s["motor"]["erros"].as_u64(), Some(0), "{}", s["motor"]);
    assert_eq!(s["estado"].as_str(), Some("parado"));
    s
}

const UFS: &str = "am,ro";
const ACEL: f64 = 120.0;

/// Critério A reduzido: AM e RO do 2º turno de 2024, do início ao fim, com a conferência automática do próprio
/// serviço; 0 HTTP 429, nenhuma requisição de seção sem disponibilização.
#[test]
#[ignore]
fn criterio_a_reduzido() {
    let c = conj("2024-2t");
    let d = pasta("a");
    let sim = simulador(&d, &c, UFS, ACEL, &[]);
    esperar(servico(&d, &c, UFS, sim.porta, ACEL), 300);
    let s = conferido(&d, 2);
    assert_eq!(s["rede"]["http429"].as_u64(), Some(0));
    assert_eq!(s["secoes"]["aplicadas"].as_u64(), Some(5102));
    assert_eq!(s["rede"]["aux_404"].as_u64(), Some(0), "pediu aux.json de seção sem disponibilização");
    assert_eq!(s["rede"]["aux"].as_u64(), Some(5102));
}

/// C(1): três `kill -9` em pontos diferentes, reinício, e o mesmo resultado final de uma execução sem queda.
#[test]
#[ignore]
fn kill9_tres_vezes() {
    let c = conj("2024-2t");
    let r = pasta("kill-ref");
    let sim = simulador(&r, &c, UFS, ACEL, &[]);
    esperar(servico(&r, &c, UFS, sim.porta, ACEL), 300);
    drop(sim);
    let referencia = publicacao(&r);
    let d = pasta("kill");
    let sim = simulador(&d, &c, UFS, ACEL, &[]);
    for alvo in [800u64, 2500, 4200] {
        let mut f = servico(&d, &c, UFS, sim.porta, ACEL);
        let t = Instant::now();
        loop {
            let seq = std::fs::read(d.join("saida/saude.json")).ok().and_then(|b| serde_json::from_slice::<Value>(&b).ok()).and_then(|v| v["secoes"]["aplicadas"].as_u64()).unwrap_or(0);
            if seq >= alvo { break; }
            assert!(t.elapsed() < Duration::from_secs(200), "não chegou a {alvo} BUs");
            std::thread::sleep(Duration::from_millis(20));
        }
        // SIGKILL: sem parada limpa
        Command::new("kill").args(["-9", &f.id().to_string()]).status().unwrap();
        let _ = f.wait();
    }
    esperar(servico(&d, &c, UFS, sim.porta, ACEL), 300);
    conferido(&d, 2);
    let diario = std::fs::read_to_string(d.join("saida/secoes.log")).unwrap();
    let aplicados = diario.lines().map(|l| l.split(',').collect::<Vec<_>>()).filter(|f| f.len() == 16 && f[1] == "aplicado").count();
    assert_eq!(aplicados, 5102, "cada seção aplicada uma vez só no diário");
    assert_eq!(publicacao(&d), referencia, "resultado final diferente do da execução sem queda");
}

/// C(2): rajada de HTTP 429 → pausa (600 s simulados = 5 s reais a 120×) sem nenhuma requisição nova, e retomada sem perda.
#[test]
#[ignore]
fn rajada_de_429() {
    let c = conj("2024-2t");
    let d = pasta("429");
    let sim = simulador(&d, &c, UFS, ACEL, &["--rajada-429", "17:30:00+60"]);
    esperar(servico(&d, &c, UFS, sim.porta, ACEL), 300);
    let s = conferido(&d, 2);
    assert!(s["rede"]["http429"].as_u64().unwrap() >= 1 && s["rede"]["pausas_429"].as_u64().unwrap() >= 1, "{}", s["rede"]);
    // no registro do simulador: depois do primeiro 429, nenhuma requisição até o fim da pausa (5 s reais), fora as
    // que já estavam a caminho (até 100 ms)
    let log = std::fs::read_to_string(d.join("simulador.log")).unwrap();
    let reqs: Vec<(f64, u16)> = log.lines().filter(|l| !l.starts_with('#'))
        .filter_map(|l| { let f: Vec<&str> = l.split(',').collect(); Some((f.first()?.parse().ok()?, f.get(3)?.parse().ok()?)) }).collect();
    let t429 = reqs.iter().find(|x| x.1 == 429).expect("nenhum 429 no registro").0;
    let na_pausa = reqs.iter().filter(|x| x.0 > t429 + 100.0 && x.0 < t429 + 4900.0).count();
    assert_eq!(na_pausa, 0, "requisições durante a pausa");
    assert!(reqs.iter().any(|x| x.0 >= t429 + 4900.0));
}

/// C(3): reenvio com o mesmo BU (hash novo) → totais inalterados, alerta.
#[test]
#[ignore]
fn reenvio_identico() {
    let c = conj("2024-2t");
    let d = pasta("reenvio");
    let sim = simulador(&d, &c, UFS, ACEL, &["--reenviar", "am:2550/1/550@17:50:00"]);
    esperar(servico(&d, &c, UFS, sim.porta, ACEL), 300);
    let s = conferido(&d, 2);
    assert_eq!(s["alertas"]["reenvio_identico"].as_u64(), Some(1), "{}", s["alertas"]);
    assert_eq!(s["acoes"]["reenvio_identico"].as_u64(), Some(1));
    assert_eq!(s["secoes"]["aplicadas"].as_u64(), Some(5102));
}

/// C(4): anulação depois de publicada — caso real MA 07315/0023/0354 (1º turno de 2024): o simulador publica a seção
/// "Totalizado" às 21:45:24 (disponibilização real) e a anula às 21:47; os votos dela viram nulos e a conferência com o
/// oficial (que a tem anulada) bate.
#[test]
#[ignore]
fn anulacao_depois_de_publicada() {
    let c = conj("2024-1t");
    let d = pasta("anulacao");
    let acel = 300.0;
    let sim = simulador(&d, &c, "ma", acel, &["--anular", "ma:7315/23/354@21:47:00"]);
    esperar(servico(&d, &c, "ma", sim.porta, acel), 400);
    let cf = json(&d.join("saida/conferencia.json"));
    assert!(cf["arquivos"].as_u64().unwrap() > 200, "{cf}");
    // com a destinação presumida "Válido", só divergências de situação admitidas, e nunca no município da seção anulada
    for x in cf["divergentes"].as_array().unwrap() {
        assert_ne!(x["arquivo"].as_str().unwrap(), "ma07315-c0011-e000619-u.json", "{x}");
        assert!(x["difs"].as_array().unwrap().iter().all(|d| d.as_str().unwrap().starts_with("situação")), "{x}");
    }
    let s = json(&d.join("saida/saude.json"));
    assert_eq!(s["alertas"]["anulacao"].as_u64(), Some(1), "{}", s["alertas"]);
    assert_eq!(s["acoes"]["anulado"].as_u64(), Some(1));
    assert_eq!(s["motor"]["duplicados"].as_u64(), Some(0));
}

/// C(5): o simulador sai do ar no meio da noite por 60 s reais (2 h simuladas a 120×), no pico de chegada de AM e RO;
/// o serviço retoma sozinho, sem pendentes. Com só AM e RO não há fila no limitador, então nenhuma seção está em coleta
/// no instante da queda e o caminho "seção pendente pedida de novo" (v0.2.6) não é exercitado aqui — quem o prova é o
/// ensaio completo C-queda (20 UFs a 60×, queda de 120 s: 160 seções pendentes na v0.2.5, resolvidas na v0.2.6;
/// docs/RELATORIO_0.3.0.md §3.3).
#[test]
#[ignore]
fn queda_do_simulador() {
    let c = conj("2024-2t");
    let d = pasta("queda");
    let sim = simulador(&d, &c, UFS, ACEL, &["--queda", "17:26:00+60"]);
    esperar(servico(&d, &c, UFS, sim.porta, ACEL), 400);
    let s = conferido(&d, 2);
    assert_eq!(s["secoes"]["aplicadas"].as_u64(), Some(5102));
    assert_eq!(s["secoes"]["pendentes"].as_u64(), Some(0));
    let log = std::fs::read_to_string(d.join("simulador.log")).unwrap();
    assert!(log.contains("# queda") && log.contains("# de volta"));
}

/// Não instaladas (achado de 16/10/2026): exterior do 1º turno de 2026, 1.310 BUs e 41 seções sem BU que o TSE marca
/// "Nao instalada". O serviço as sonda sem esperar silêncio, nunca pede o `aux.json` das 1.364 agregadas (`nsp` no
/// `cs.json`), conta as 41 como totalizadas (1.351 = `s.st` oficial), encerra sozinho e a conferência bate.
#[test]
#[ignore]
fn nao_instaladas_exterior() {
    let c = conj("2026-1t");
    let d = pasta("nao-inst");
    let acel = 300.0;
    let sim = simulador(&d, &c, "zz", acel, &["--corte", "05/10/2026 02:00:00", "--horario", "recebido"]);
    esperar(servico(&d, &c, "zz", sim.porta, acel), 400);
    let cf = json(&d.join("saida/conferencia.json"));
    let n = cf["arquivos"].as_u64().unwrap();
    assert!(n >= 1, "{cf}");
    assert_eq!(cf["identicos"].as_u64(), Some(n), "divergências: {}", cf["divergentes"]);
    let s = json(&d.join("saida/saude.json"));
    assert_eq!(s["secoes"]["aplicadas"].as_u64(), Some(1310), "{}", s["secoes"]);
    assert_eq!(s["secoes"]["nao_instaladas"].as_u64(), Some(41), "{}", s["secoes"]);
    assert_eq!(s["secoes"]["agregadas"].as_u64(), Some(0), "{}", s["secoes"]);
    assert!(s["apuracao"]["encerrada"].as_bool().unwrap_or(false) || s["estado"].as_str() == Some("parado"), "{}", s["apuracao"]);
    // nenhum aux.json de seção agregada (nsp no cs.json): 404 só de seção com BU ainda por chegar, sondada quando
    // faltavam poucas no exterior (continua esperada e chega depois)
    let cs: Value = json(&c.cs.join("zz-p003220-cs.json"));
    let mut agregadas = std::collections::HashSet::new();
    for m in cs["abr"][0]["mu"].as_array().unwrap() {
        for z in m["zon"].as_array().unwrap() {
            for x in z["sec"].as_array().unwrap() {
                if x.get("nsp").is_some() {
                    let (mu, zn, se): (u32, u32, u32) = (m["cd"].as_str().unwrap().parse().unwrap(), z["cd"].as_str().unwrap().parse().unwrap(), x["ns"].as_str().unwrap().parse().unwrap());
                    agregadas.insert(format!("-m{mu:05}-z{zn:04}-s{se:04}-aux.json"));
                }
            }
        }
    }
    assert_eq!(agregadas.len(), 1364);
    let log = std::fs::read_to_string(d.join("simulador.log")).unwrap();
    let aux: Vec<&str> = log.lines().filter(|l| l.contains("-aux.json")).collect();
    let de_agregada = aux.iter().filter(|l| agregadas.iter().any(|a| l.contains(a.as_str()))).count();
    assert_eq!(de_agregada, 0, "aux.json pedido de seção agregada");
    let aux_404 = aux.iter().filter(|l| l.split(',').nth(3) == Some("404")).count() as u64;
    assert_eq!(s["rede"]["aux_404"].as_u64(), Some(aux_404));
    assert!(aux_404 < 100, "{aux_404} aux.json 404");
    eprintln!("nao_instaladas_exterior: {} aux.json, {aux_404} 404, {} sondagens", aux.len(), s["rede"]["sondadas"]);
    assert_eq!(s["rede"]["http429"].as_u64(), Some(0));
}

// ------------------------------------------------------------------ difusão (etapa 4, docs/PROTOCOLO_WS.md)

/// D4 reduzido: AM e RO a 120× com `--http`; um cliente WebSocket (aqui mesmo) inscrito em todos os tópicos e nos
/// eventos: o `seq` de cada tópico só sobe, a última mensagem de cada tópico é o arquivo publicado byte a byte, os
/// eventos são os do `eventos.csv`, recebe o `desligamento` no fim e a conferência do serviço segue 2/2.
#[test]
#[ignore]
fn difusao_d4_reduzido() {
    use futures::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::Message as M;
    let c = conj("2024-2t");
    let d = pasta("d4");
    let sim = simulador(&d, &c, UFS, ACEL, &[]);
    let http = porta_livre();
    let f = servico_com(&d, &c, UFS, sim.porta, ACEL, &["--http", &format!("127.0.0.1:{http}")]);
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    let (ult, evs, seqs, tipos) = rt.block_on(async {
        let t = Instant::now();
        let mut ws = loop {
            match tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{http}/ws")).await {
                Ok((ws, _)) => break ws,
                Err(_) => { assert!(t.elapsed() < Duration::from_secs(60), "o serviço não abriu a porta HTTP"); tokio::time::sleep(Duration::from_millis(100)).await; }
            }
        };
        let tops: Value = serde_json::from_str(&reqwest::get(format!("http://127.0.0.1:{http}/api/v1/topicos")).await.unwrap().text().await.unwrap()).unwrap();
        let tops: Vec<String> = tops.as_array().unwrap().iter().map(|x| x["topico"].as_str().unwrap().to_string()).collect();
        assert_eq!(tops.len(), 2, "{tops:?}");
        ws.send(M::text(serde_json::json!({"op": "inscrever", "topicos": tops}).to_string())).await.unwrap();
        ws.send(M::text(r#"{"op":"eventos"}"#)).await.unwrap();
        let mut ult: BTreeMap<String, String> = BTreeMap::new();
        let mut seqs: BTreeMap<String, Vec<u64>> = BTreeMap::new();
        let mut evs = std::collections::BTreeSet::new();
        let mut tipos: BTreeMap<String, u64> = BTreeMap::new();
        while let Ok(Some(Ok(m))) = tokio::time::timeout(Duration::from_secs(300), ws.next()).await {
            let M::Text(t) = m else { if matches!(m, M::Close(_)) { break } else { continue } };
            let s = t.as_str();
            let v: Value = serde_json::from_str(s).unwrap();
            let tipo = v["tipo"].as_str().unwrap().to_string();
            *tipos.entry(tipo.clone()).or_default() += 1;
            match tipo.as_str() {
                "atualizacao" | "instantaneo" => {
                    let top = v["topico"].as_str().unwrap().to_string();
                    let i = s.find("\"resultado\":").unwrap() + 12;
                    assert_eq!(v["seq"], v["resultado"]["seq"]);
                    assert_eq!(v["secoes_apuradas"], v["resultado"]["secoes"]["apuradas"]);
                    seqs.entry(top.clone()).or_default().push(v["seq"].as_u64().unwrap());
                    ult.insert(top, s[i..s.len() - 1].to_string());
                }
                "evento" if v["classe"] == "matematicamente" => {
                    evs.insert(format!("{}|{}|{}|{}", v["abrangencia"].as_str().unwrap(), v["cargo"], v["candidato"].as_u64().map(|x| x.to_string()).unwrap_or_default(), v["estado"].as_str().unwrap()));
                }
                _ => {}
            }
        }
        (ult, evs, seqs, tipos)
    });
    esperar(f, 300);
    conferido(&d, 2);
    assert!(tipos.get("aviso").is_some() && tipos["instantaneo"] == 2, "{tipos:?}");
    for (t, v) in &seqs { assert!(v.windows(2).all(|w| w[0] <= w[1]), "{t}: seq voltou"); assert!(v.len() > 10, "{t}: só {} mensagens", v.len()); }
    for e in std::fs::read_dir(d.join("saida/publicacao")).unwrap().flatten() {
        let n = e.file_name().to_string_lossy().into_owned();
        let Some(t) = uv_aquisicao::difusao::topico_de(&n) else { continue };
        assert_eq!(ult.get(t).map(|s| s.as_bytes()), Some(&std::fs::read(e.path()).unwrap()[..]), "{t}: última mensagem ≠ arquivo");
    }
    let csv: std::collections::BTreeSet<String> = std::fs::read_to_string(d.join("saida/eventos.csv")).unwrap().lines().skip(1).filter(|l| !l.starts_with('#'))
        .map(|l| { let f: Vec<&str> = l.split(',').collect(); format!("{}|{}|{}|{}", f[2], f[3], f[4], f[5]) }).collect();
    assert!(!csv.is_empty());
    assert_eq!(evs, csv, "eventos pelo WebSocket × eventos.csv");
}

/// Clientes lentos não atrasam o serviço: 50 conexões que não leem nada + 10 normais; o serviço termina no mesmo tempo,
/// as normais recebem tudo (última mensagem = arquivo) e BU → publicação continua baixo.
#[test]
#[ignore]
fn difusao_cliente_lento() {
    let c = conj("2024-2t");
    let d = pasta("lento");
    let sim = simulador(&d, &c, UFS, ACEL, &[]);
    let http = porta_livre();
    let f = servico_com(&d, &c, UFS, sim.porta, ACEL, &["--http", &format!("127.0.0.1:{http}")]);
    let (cg, rel) = carga(&d, http, &["--clientes", "10", "--lentos", "50", "--perfil", "todos", "--rampa", "100"]);
    let t = Instant::now();
    esperar(f, 300);
    let dur = t.elapsed().as_secs_f64();
    let r = esperar_carga(cg, &rel);
    let s = conferido(&d, 2);
    assert_eq!(r["conferencia"]["publicacao"]["identicos"], r["conferencia"]["publicacao"]["conferidos"], "{}", r["conferencia"]);
    assert_eq!(r["conferencia"]["publicacao"]["conferidos"].as_u64(), Some(20));
    assert_eq!(r["seq"]["regressoes"].as_u64(), Some(0));
    assert_eq!(r["conexoes"]["lentas"].as_object().unwrap().values().filter_map(|x| x.as_u64()).sum::<u64>(), 50, "{}", r["conexoes"]);
    let p99 = s["latencias_ms"]["bu_publicacao"]["p99_total"].as_f64().unwrap();
    assert!(p99 < 50.0, "BU → publicação p99 {p99} ms com clientes lentos");
    // o A reduzido sem clientes leva ~40 s a 120×: com os lentos, a mesma ordem
    assert!(dur < 120.0, "serviço levou {dur:.0} s");
}

/// `kill -9` do serviço no meio da noite com 20 clientes que reconectam: depois do reinício, o `instantaneo` de cada
/// tópico nunca volta no `seq`, nenhuma mensagem regride, e no fim a última mensagem = arquivo em todas as conexões.
#[test]
#[ignore]
fn difusao_kill9_reconexao() {
    let c = conj("2024-2t");
    let d = pasta("d3kill");
    let sim = simulador(&d, &c, UFS, ACEL, &[]);
    let http = porta_livre();
    let hx = format!("127.0.0.1:{http}");
    let mut f = servico_com(&d, &c, UFS, sim.porta, ACEL, &["--http", &hx]);
    let (cg, rel) = carga(&d, http, &["--clientes", "20", "--perfil", "todos", "--eventos", "--reconectar", "--rampa", "100"]);
    let t = Instant::now();
    loop {
        let seq = std::fs::read(d.join("saida/saude.json")).ok().and_then(|b| serde_json::from_slice::<Value>(&b).ok()).and_then(|v| v["secoes"]["aplicadas"].as_u64()).unwrap_or(0);
        if seq >= 2500 { break; }
        assert!(t.elapsed() < Duration::from_secs(200), "não chegou a 2.500 BUs");
        std::thread::sleep(Duration::from_millis(20));
    }
    Command::new("kill").args(["-9", &f.id().to_string()]).status().unwrap();
    let _ = f.wait();
    std::thread::sleep(Duration::from_secs(2));
    f = servico_com(&d, &c, UFS, sim.porta, ACEL, &["--http", &hx]);
    esperar(f, 300);
    let r = esperar_carga(cg, &rel);
    conferido(&d, 2);
    assert!(r["conexoes"]["reconexoes"].as_u64().unwrap() >= 20, "{}", r["conexoes"]);
    assert_eq!(r["seq"]["reconexao_instantaneo_menor"].as_u64(), Some(0), "{}", r["seq"]);
    assert_eq!(r["seq"]["regressoes"].as_u64(), Some(0), "{}", r["seq"]);
    assert_eq!(r["conferencia"]["publicacao"]["identicos"].as_u64(), Some(40), "{}", r["conferencia"]);
    assert_eq!(r["conferencia"]["eventos"]["iguais_ao_csv"].as_u64(), Some(20), "{}", r["conferencia"]["eventos"]);
}

// ------------------------------------------------------------------ comparação com o TSE (etapa 7, PROTOCOLO_WS §4.6)

/// Comparação Urna Velox × TSE contra o simulador (AM e RO do 2º turno de 2024 a 120×, intervalo de 30 s da noite):
/// nenhuma busca de `-u.json` antes da inscrição `tse:` nem do tópico não inscrito; ≤ 1 busca por intervalo do tópico
/// inscrito; 404 (antes dos oficiais) vira `indisponivel`; quando o arquivo aparece, `ok` com o resumo do oficial; a
/// segunda conexão no mesmo tópico recebe o instantâneo sem busca nova; conferência do serviço segue 2/2.
#[test]
#[ignore]
fn comparacao_tse_rede() {
    use futures::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::Message as M;
    let c = conj("2024-2t");
    let d = pasta("tse-rede");
    let sim = simulador(&d, &c, UFS, ACEL, &[]);
    let http = porta_livre();
    // --continuar: o serviço fica no ar depois da conferência, até o teste ver o arquivo do TSE e pedir a parada
    let f = servico_com(&d, &c, UFS, sim.porta, ACEL, &["--http", &format!("127.0.0.1:{http}"), "--continuar"]);
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    let (msgs, _alvo, outro, segunda) = rt.block_on(async {
        let t = Instant::now();
        let conectar = || async {
            loop {
                match tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{http}/ws")).await {
                    Ok((ws, _)) => break ws,
                    Err(_) => { assert!(t.elapsed() < Duration::from_secs(60), "o serviço não abriu a porta HTTP"); tokio::time::sleep(Duration::from_millis(100)).await; }
                }
            }
        };
        let mut ws = conectar().await;
        let tops: Value = serde_json::from_str(&reqwest::get(format!("http://127.0.0.1:{http}/api/v1/topicos")).await.unwrap().text().await.unwrap()).unwrap();
        let mut tops: Vec<String> = tops.as_array().unwrap().iter().map(|x| x["topico"].as_str().unwrap().to_string()).collect();
        tops.sort();
        assert_eq!(tops.len(), 2, "{tops:?}");
        let (alvo, outro) = (tops[0].clone(), tops[1].clone());
        // 2 s reais (4 min da noite) conectado sem a comparação: nenhuma busca
        tokio::time::sleep(Duration::from_secs(2)).await;
        ws.send(M::text(serde_json::json!({"op": "inscrever", "topicos": [format!("tse:{alvo}"), "tse:xx-c0001"]}).to_string())).await.unwrap();
        let mut msgs: Vec<Value> = vec![];
        let mut segunda: Option<Value> = None;
        let mut erros = 0;
        while let Ok(Some(Ok(m))) = tokio::time::timeout(Duration::from_secs(300), ws.next()).await {
            let M::Text(t) = m else { if matches!(m, M::Close(_)) { break } else { continue } };
            let v: Value = serde_json::from_str(t.as_str()).unwrap();
            if v["tipo"] == "erro" { assert_eq!(v["motivo"], "topico_desconhecido", "{v}"); erros += 1; }
            if v["tipo"] != "tse" { continue; }
            // no primeiro "ok", uma segunda conexão no mesmo tópico: instantâneo já com os dados
            if v["estado"] == "ok" && segunda.is_none() {
                let mut w2 = conectar().await;
                w2.send(M::text(serde_json::json!({"op": "inscrever", "topicos": [format!("tse:{alvo}")]}).to_string())).await.unwrap();
                while let Ok(Some(Ok(m))) = tokio::time::timeout(Duration::from_secs(10), w2.next()).await {
                    if let M::Text(t) = m { let x: Value = serde_json::from_str(t.as_str()).unwrap(); if x["tipo"] == "tse" { segunda = Some(x); break; } }
                }
                let _ = w2.close(None).await;
            }
            let ok = v["estado"] == "ok";
            msgs.push(v);
            if ok && segunda.is_some() { break; }
        }
        assert_eq!(erros, 1);
        (msgs, alvo, outro, segunda)
    });
    // a conferência do próprio serviço (no encerramento) antes da parada
    let t = Instant::now();
    while !d.join("saida/conferencia.json").exists() {
        assert!(t.elapsed() < Duration::from_secs(120), "o serviço não conferiu");
        std::thread::sleep(Duration::from_millis(200));
    }
    std::thread::sleep(Duration::from_millis(500));
    Command::new("kill").args(["-TERM", &f.id().to_string()]).status().unwrap();
    esperar(f, 60);
    conferido(&d, 2);
    // mensagens: aguardando → indisponivel (404, antes dos oficiais) → ok com o resumo do oficial
    assert_eq!(msgs[0]["instantaneo"], true);
    assert_eq!(msgs[0]["estado"], "aguardando", "{}", msgs[0]);
    assert!(msgs.iter().any(|m| m["estado"] == "indisponivel" && m["motivo"] == "404"), "{msgs:?}");
    let ultimo = msgs.iter().rev().find(|m| m["estado"] == "ok").expect("nenhum estado ok");
    let nome = ultimo["arquivo"].as_str().unwrap();
    let of = urna_velox::oficial::Oficial::ler(&c.oficial.join(nome)).unwrap();
    assert_eq!(ultimo["dados"], uv_aquisicao::tse_resultados::resumo(&of), "resumo ≠ oficial");
    assert_eq!(ultimo["dados"]["cand"][0]["pos"], 1);
    let s2 = segunda.expect("a segunda conexão não recebeu o instantâneo");
    assert_eq!((s2["instantaneo"].as_bool(), s2["estado"].as_str()), (Some(true), Some("ok")));
    assert_eq!(s2["dados"], ultimo["dados"]);
    let vers: Vec<u64> = msgs.iter().map(|m| m["ver"].as_u64().unwrap()).collect();
    assert!(vers.windows(2).all(|w| w[0] < w[1]), "ver não crescente: {vers:?}");
    // rede, pelo registro do simulador: buscas do -u.json do tópico inscrito com ≥ 28,5 s da noite entre elas (fora a
    // conferência do fim, que baixa todos uma vez); o outro tópico só na conferência
    let log = std::fs::read_to_string(d.join("simulador.log")).unwrap();
    let us: Vec<(i64, String)> = log.lines().filter(|l| !l.starts_with('#')).filter_map(|l| {
        let x: Vec<&str> = l.split(',').collect();
        (x.len() >= 6 && x[5].ends_with("-u.json")).then(|| (urna_velox::metricas::segundos(x[1]).unwrap(), x[5].to_string()))
    }).collect();
    let (de_alvo, do_outro): (Vec<_>, Vec<_>) = us.iter().partition(|(_, p)| p.contains(&format!("/{nome}")));
    assert!(do_outro.len() <= 1, "o tópico não inscrito {outro} foi buscado {} vezes", do_outro.len());
    assert!(de_alvo.len() >= 3, "só {} buscas de {nome}", de_alvo.len());
    // a conferência do fim baixa o arquivo uma vez, no meio da cadência da comparação (o registro do simulador não as
    // distingue): tirada a requisição que mais encurta, nenhum intervalo pode ficar curto
    let curtas = |v: &[i64]| v.windows(2).filter(|w| w[1] - w[0] < 28).count();
    let ts: Vec<i64> = de_alvo.iter().map(|x| x.0).collect();
    let curtos = (0..ts.len()).map(|i| { let mut v = ts.clone(); v.remove(i); curtas(&v) }).min().unwrap_or(0);
    assert_eq!(curtos, 0, "{curtos} buscas de {nome} a menos de um intervalo (fora a da conferência): {de_alvo:?}");
    let s = json(&d.join("saida/saude.json"));
    let t = &s["tse_resultados"];
    assert_eq!(t["fora_do_intervalo"].as_u64(), Some(0), "{t}");
    assert_eq!(s["rede"]["http429"].as_u64(), Some(0));
    eprintln!("comparacao_tse_rede: {} mensagens tse, {} buscas de {nome} ({curtos} curtas), {}", msgs.len(), de_alvo.len(), t);
}

/// TSE simulado (etapa 7): o simulador serve os `-u.json` progressivos (atraso de divulgação sorteado por
/// seção). Contra AM e RO do 2º turno de 2024 a 120×, um cliente inscrito em `tse:` de Manaus vê o TSE andar em
/// degraus — seções e votos nunca voltam, geração sempre mais nova —, atrás do Urna Velox, e no fim igual ao oficial;
/// o "matematicamente" aparece no arquivo do TSE (`md`) depois do evento do Urna Velox (`eventos.csv`); a conferência
/// do fim espera o TSE fechar (100 % das seções) e fica 2/2.
#[test]
#[ignore]
fn tse_simulado_progressivo() {
    use futures::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::Message as M;
    let c = conj("2024-2t");
    let d = pasta("tse-sim");
    let cand = c.candidatos.to_str().unwrap().to_string();
    let sim = simulador(&d, &c, UFS, ACEL, &["--tse-simulado", &cand]);
    let http = porta_livre();
    let f = servico_com(&d, &c, UFS, sim.porta, ACEL, &["--http", &format!("127.0.0.1:{http}"), "--continuar"]);
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    let alvo = "am02550-c0011";
    let msgs: Vec<Value> = rt.block_on(async {
        let t = Instant::now();
        let mut ws = loop {
            match tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{http}/ws")).await {
                Ok((ws, _)) => break ws,
                Err(_) => { assert!(t.elapsed() < Duration::from_secs(60), "o serviço não abriu a porta HTTP"); tokio::time::sleep(Duration::from_millis(100)).await; }
            }
        };
        ws.send(M::text(serde_json::json!({"op": "inscrever", "topicos": [format!("tse:{alvo}")]}).to_string())).await.unwrap();
        let mut v = vec![];
        while let Ok(Some(Ok(m))) = tokio::time::timeout(Duration::from_secs(300), ws.next()).await {
            let M::Text(t) = m else { if matches!(m, M::Close(_)) { break } else { continue } };
            let x: Value = serde_json::from_str(t.as_str()).unwrap();
            if x["tipo"] != "tse" { continue; }
            // o arquivo final do TSE (o oficial: totalização finalizada)
            let fim = x["dados"]["finalizada"] == true;
            v.push(x);
            if fim { break; }
        }
        v
    });
    // a conferência do próprio serviço (que espera o TSE fechar) antes da parada
    let t = Instant::now();
    loop {
        let ok = std::fs::read(d.join("saida/conferencia.json")).ok().and_then(|b| serde_json::from_slice::<Value>(&b).ok())
            .is_some_and(|c| c["incompletos"].as_array().is_some_and(|a| a.is_empty()));
        if ok { break; }
        assert!(t.elapsed() < Duration::from_secs(200), "a conferência não fechou com o TSE completo");
        std::thread::sleep(Duration::from_millis(200));
    }
    Command::new("kill").args(["-TERM", &f.id().to_string()]).status().unwrap();
    esperar(f, 60);
    conferido(&d, 2);
    let com_dados: Vec<&Value> = msgs.iter().filter(|m| m["dados"].is_object()).collect();
    assert!(com_dados.len() >= 10, "só {} arquivos do TSE", com_dados.len());
    // degraus: seções e votos nunca voltam; a geração sempre avança
    let st: Vec<u64> = com_dados.iter().map(|m| m["dados"]["secoes"]["totalizadas"].as_u64().unwrap()).collect();
    assert!(st.windows(2).all(|w| w[0] <= w[1]), "seções do TSE voltaram: {st:?}");
    let g: Vec<i64> = com_dados.iter().map(|m| urna_velox::metricas::segundos(m["dados"]["geracao"].as_str().unwrap()).unwrap()).collect();
    // (a mesma geração pode vir de novo quando só o estado da fonte muda: desatualizado → ok)
    assert!(g.windows(2).all(|w| w[0] <= w[1]), "geração voltou: {g:?}");
    let distintas = { let mut x = g.clone(); x.dedup(); x.len() };
    assert!(distintas >= 10, "só {distintas} gerações distintas");
    // fim: o oficial
    let ultimo = com_dados.last().unwrap();
    let of = urna_velox::oficial::Oficial::ler(&c.oficial.join(ultimo["arquivo"].as_str().unwrap())).unwrap();
    assert_eq!(ultimo["dados"], uv_aquisicao::tse_resultados::resumo(&of), "TSE do fim ≠ oficial");
    // "matematicamente": no TSE (md = e) depois do evento do Urna Velox
    let ev = std::fs::read_to_string(d.join("saida/eventos.csv")).unwrap();
    let uv = ev.lines().skip(1).filter(|l| !l.starts_with('#')).map(|l| l.split(',').collect::<Vec<_>>())
        .find(|f| f.len() > 5 && f[2] == "am02550" && f[5] == "eleito").map(|f| {
            // "aaaa-mm-dd hh:mm:ss" → "dd/mm/aaaa hh:mm:ss"
            let (dt, h) = f[1].split_once(' ').unwrap();
            let p: Vec<&str> = dt.split('-').collect();
            urna_velox::metricas::segundos(&format!("{}/{}/{} {h}", p[2], p[1], p[0])).unwrap()
        });
    let tse = com_dados.iter().find(|m| m["dados"]["md"] == "e" || m["dados"]["finalizada"] == true)
        .map(|m| urna_velox::metricas::segundos(m["dados"]["geracao"].as_str().unwrap()).unwrap());
    let (uv, tse) = (uv.expect("sem evento de Manaus no eventos.csv"), tse.expect("o TSE não mostrou o eleito"));
    assert!(tse >= uv, "TSE ({tse}) antes do Urna Velox ({uv})");
    let s = json(&d.join("saida/saude.json"));
    assert_eq!(s["tse_resultados"]["fora_do_intervalo"].as_u64(), Some(0));
    assert_eq!(s["rede"]["http429"].as_u64(), Some(0));
    eprintln!("tse_simulado_progressivo: {} arquivos do TSE, seções {:?}…, Urna Velox decidiu {} s antes do TSE", com_dados.len(), &st[..st.len().min(8)], tse - uv);
}
