//! Ensaios da federação (2.0, etapa A; ADR-028; `docs/HANDOFF_0.6.0.md`): vários `uv-servico` (um por "órgão", cada
//! um saindo por um `127.0.0.x` próprio) contra um `uv-simulador` que impõe o limite **por IP de origem**, com o
//! storage comum num diretório. Rodam com
//! `UV_DADOS=/home/dev/uvreal cargo test --release -p uv-aquisicao --test federacao -- --ignored --test-threads 1`.
//!
//! Conjunto: 2º turno de 2024 reduzido a AM e RO (2 municípios, 5.102 BUs) a 120×, como os ensaios do serviço. Para
//! que o limite por IP seja exercitado com tão poucas seções, a vazão de coleta da federação é reduzida
//! (`teto_coleta_rps` 5 req/s da noite = 600 req/s reais a 120×) e o simulador pune acima de 6 req/s da noite por IP.

use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

fn uv() -> PathBuf { PathBuf::from(std::env::var("UV_DADOS").expect("defina UV_DADOS (ex.: /home/dev/uvreal)")) }

const UFS: &str = "am,ro";
const ACEL: f64 = 120.0;
const BUS: u64 = 5102;
/// req/s da noite por IP de saída na coleta (federacao.json) e limite do simulador por IP (acima: 429 + penalidade)
const COLETA_RPS: f64 = 5.0;
const LIMITE_IP: f64 = 6.0;

/// Pasta temporária de um ensaio: apagada quando o ensaio termina bem (fica se ele falhar, para o diagnóstico, ou com
/// `UV_MANTER_ENSAIO=1`). Antes de criar, apaga as sobras de execuções anteriores cujo processo já morreu (o nome leva
/// o PID, que muda a cada execução: sem isto as pastas se acumulavam no TMPDIR).
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
    limpar_sobras("uv-federacao-");
    let d = std::env::temp_dir().join(format!("uv-federacao-{}-{nome}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    Pasta(d)
}

struct Simulador { filho: Child, porta: u16 }
impl Drop for Simulador { fn drop(&mut self) { let _ = self.filho.kill(); let _ = self.filho.wait(); } }

fn simulador(dir: &Path) -> Simulador {
    let u = uv();
    let pa = dir.join("porta");
    let filho = Command::new(env!("CARGO_BIN_EXE_uv-simulador"))
        .args(["--dados", u.join("2024-2t/dados").to_str().unwrap(), "--cs", u.join("2024-2t/dados/config").to_str().unwrap(),
            "--metricas", u.join("2024-2t/metricas_2t.csv").to_str().unwrap(), "--oficial", u.join("2024-2t/oficial").to_str().unwrap(),
            "--pleito", "453", "--ciclo", "ele2024", "--ufs", UFS, "--acelerar", &ACEL.to_string(),
            "--log", dir.join("simulador.log").to_str().unwrap(), "--porta-arquivo", pa.to_str().unwrap(), "--limite-ip-rps", &LIMITE_IP.to_string()])
        .stdout(Stdio::null()).stderr(std::fs::File::create(dir.join("simulador.err")).unwrap()).spawn().unwrap();
    let t = Instant::now();
    loop {
        if let Some(p) = std::fs::read_to_string(&pa).ok().and_then(|s| s.trim().parse().ok()) { return Simulador { filho, porta: p }; }
        assert!(t.elapsed() < Duration::from_secs(60), "o simulador não abriu a porta");
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn porta_livre() -> u16 { std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port() }

/// `federacao.json` (no arquivo `nome`) com os nós: (id, IP de saída declarado, endereço de sincronização).
fn federacao_em(dir: &Path, nome: &str, nos: &[(&str, &str, String)]) -> PathBuf {
    let v: Vec<Value> = nos.iter().map(|(id, ip, end)| serde_json::json!({"id": id, "orgao": format!("Órgão {id}"), "ip_saida": ip, "endereco": end})).collect();
    let p = dir.join(nome);
    std::fs::write(&p, serde_json::to_vec_pretty(&serde_json::json!({"versao": 1, "nos": v, "teto_coleta_rps": COLETA_RPS, "unidade_peso_rps": 0.25})).unwrap()).unwrap();
    p
}

/// `federacao.json` com os nós (id, IP declarado); cada um escuta a sincronização no próprio IP, numa porta livre.
fn federacao(dir: &Path, nos: &[(&str, &str)]) -> PathBuf {
    let v: Vec<(&str, &str, String)> = nos.iter().map(|(id, ip)| (*id, *ip, format!("{ip}:{}", porta_livre()))).collect();
    federacao_em(dir, "federacao.json", &v)
}

/// Um nó: `uv-servico --federacao`, saindo por `ip_local`, com pasta própria em `<dir>/<id>` (inclusive o diretório da
/// federação: nada é compartilhado entre os nós além do que trocam por HTTP).
fn no(dir: &Path, fed: &Path, id: &str, ip_local: &str, porta: u16) -> Child {
    let u = uv();
    let d = dir.join(id);
    std::fs::create_dir_all(&d).unwrap();
    Command::new(env!("CARGO_BIN_EXE_uv-servico"))
        .args(["--pleito", "453", "--ciclo", "ele2024", "--candidatos", u.join("2024-2t/preparo/candidatos.json").to_str().unwrap(),
            "--dados", d.join("dados").to_str().unwrap(), "--saida", d.join("saida").to_str().unwrap(),
            "--base", &format!("http://127.0.0.1:{porta}"), "--aceleracao-local", &ACEL.to_string(), "--ufs", UFS,
            "--rps", &COLETA_RPS.to_string(),
            "--federacao", fed.to_str().unwrap(), "--no", id, "--armazem", d.join("federacao").to_str().unwrap(), "--ip-local", ip_local])
        .stdout(Stdio::null()).stderr(std::fs::OpenOptions::new().create(true).append(true).open(d.join("servico.err")).unwrap()).spawn().unwrap()
}

fn esperar(mut f: Child, limite_s: u64, nome: &str) {
    let t = Instant::now();
    loop {
        if let Some(st) = f.try_wait().unwrap() { assert!(st.success(), "{nome}: uv-servico saiu com {st}"); return; }
        if t.elapsed() > Duration::from_secs(limite_s) { let _ = f.kill(); panic!("{nome}: uv-servico não terminou em {limite_s} s"); }
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn json(p: &Path) -> Value { serde_json::from_slice(&std::fs::read(p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))).unwrap() }

/// Fim de um nó: conferência 2/2 sem divergência, nenhum duplicado, todos os BUs aplicados.
fn conferido(dir: &Path, id: &str) -> Value {
    let d = dir.join(id);
    let c = json(&d.join("saida/conferencia.json"));
    assert_eq!(c["arquivos"].as_u64(), Some(2), "{id}: {c}");
    assert_eq!(c["identicos"].as_u64(), Some(2), "{id}: divergências {}", c["divergentes"]);
    assert_eq!(c["situacoes"]["divergentes"].as_u64(), Some(0), "{id}");
    assert_eq!(c["matematicamente"]["revogacoes"].as_u64(), Some(0), "{id}");
    let s = json(&d.join("saida/saude.json"));
    assert_eq!(s["motor"]["duplicados"].as_u64(), Some(0), "{id}: {}", s["motor"]);
    assert_eq!(s["motor"]["erros"].as_u64(), Some(0), "{id}: {}", s["motor"]);
    assert_eq!(s["secoes"]["aplicadas"].as_u64(), Some(BUS), "{id}: {}", s["secoes"]);
    assert_eq!(s["estado"].as_str(), Some("parado"), "{id}");
    s
}

/// Requisições do simulador por IP: (aux por IP, 429 por IP, máximo por segundo real por IP).
struct PorIp { aux: std::collections::BTreeMap<String, u64>, http429: u64, max_seg: std::collections::BTreeMap<String, u64> }
fn por_ip(dir: &Path) -> PorIp {
    let log = std::fs::read_to_string(dir.join("simulador.log")).unwrap();
    let (mut aux, mut seg) = (std::collections::BTreeMap::new(), std::collections::BTreeMap::<(String, u64), u64>::new());
    let mut http429 = 0;
    for l in log.lines().filter(|l| !l.starts_with('#')) {
        let f: Vec<&str> = l.split(',').collect();
        if f.len() < 7 { continue; }
        let ip = f[6].to_string();
        if f[3] == "429" { http429 += 1; }
        if f[2] == "aux" && f[3] == "200" { *aux.entry(ip.clone()).or_default() += 1; }
        let s = (f[0].parse::<f64>().unwrap_or(0.0) / 1000.0) as u64;
        *seg.entry((ip, s)).or_default() += 1;
    }
    let mut max_seg = std::collections::BTreeMap::new();
    for ((ip, _), n) in seg { let e = max_seg.entry(ip).or_insert(0); if n > *e { *e = n; } }
    PorIp { aux, http429, max_seg }
}

/// Teto real por IP no simulador (req por segundo real)
fn teto_ip_real() -> u64 { (COLETA_RPS * ACEL).ceil() as u64 }

/// A: três órgãos, cada um no seu IP. Cada nó termina com 2/2 e 5.102 BUs; cada seção pedida ao TSE por um nó só;
/// por IP, nunca acima do teto e nenhum 429.
#[test]
#[ignore]
fn tres_orgaos() {
    let d = pasta("tres");
    let fed = federacao(&d, &[("a", "127.0.0.2"), ("b", "127.0.0.3"), ("c", "127.0.0.4")]);
    let sim = simulador(&d);
    let filhos: Vec<(Child, &str)> = [("a", "127.0.0.2"), ("b", "127.0.0.3"), ("c", "127.0.0.4")].iter().map(|(id, ip)| (no(&d, &fed, id, ip, sim.porta), *id)).collect();
    for (f, id) in filhos { esperar(f, 600, id); }
    drop(sim);
    let mut aux_total = 0;
    for id in ["a", "b", "c"] {
        let s = conferido(&d, id);
        let fe = &s["federacao"];
        assert_eq!(fe["ativos"].as_array().map(|v| v.len()), Some(3), "{id}: {fe}");
        aux_total += s["rede"]["aux"].as_u64().unwrap();
        assert!(fe["recebidos"].as_u64().unwrap() > 0, "{id} não recebeu nada pelo storage: {fe}");
        assert_eq!(fe["delegadas_agora"].as_u64(), Some(0), "{id}: seções delegadas sem resultado no fim: {fe}");
        assert_eq!(fe["rejeitados"].as_u64(), Some(0), "{id}: {fe}");
    }
    let p = por_ip(&d);
    assert_eq!(p.http429, 0, "HTTP 429 no simulador");
    assert_eq!(p.aux.len(), 3, "{:?}", p.aux);
    for (ip, n) in &p.aux { assert!(*n > BUS / 6, "{ip} coletou só {n}"); }
    // cada seção pedida por um nó só (com folga para as delegadas assumidas na partida, antes de os nós se verem)
    assert!(aux_total <= BUS + BUS / 50, "aux somados {aux_total} para {BUS} seções");
    for (ip, m) in &p.max_seg { assert!(*m <= teto_ip_real() + 2, "{ip}: {m} req num segundo (teto {})", teto_ip_real()); }
}

/// A: dois órgãos atrás do mesmo IP de saída dividem o teto do IP; com o limite do simulador acima do teto de um IP e
/// abaixo do dobro, se dividirem errado, levam 429.
#[test]
#[ignore]
fn dois_orgaos_no_mesmo_ip() {
    let d = pasta("mesmo-ip");
    let fed = federacao(&d, &[("a", "127.0.0.2"), ("b", "127.0.0.2"), ("c", "127.0.0.3")]);
    let sim = simulador(&d);
    let filhos: Vec<(Child, &str)> = [("a", "127.0.0.2"), ("b", "127.0.0.2"), ("c", "127.0.0.3")].iter().map(|(id, ip)| (no(&d, &fed, id, ip, sim.porta), *id)).collect();
    for (f, id) in filhos { esperar(f, 900, id); }
    drop(sim);
    for id in ["a", "b", "c"] { conferido(&d, id); }
    let s = json(&d.join("a/saida/saude.json"));
    assert_eq!(s["federacao"]["nos_no_ip"].as_u64(), Some(2), "{}", s["federacao"]);
    assert_eq!(s["federacao"]["rps_cota"].as_f64(), Some(COLETA_RPS / 2.0));
    let p = por_ip(&d);
    assert_eq!(p.http429, 0, "HTTP 429: o IP compartilhado passou do limite");
    let (a2, a3) = (p.aux["127.0.0.2"], p.aux["127.0.0.3"]);
    // o IP com dois nós tem a mesma capacidade do IP com um: cada IP coleta ~metade
    assert!((a2 as f64 / (a2 + a3) as f64 - 0.5).abs() < 0.1, "127.0.0.2: {a2}, 127.0.0.3: {a3}");
    for (ip, m) in &p.max_seg { assert!(*m <= teto_ip_real() + 2, "{ip}: {m} req num segundo"); }
}

/// A: `kill -9` de um nó no meio da noite, sem reinício: os outros dois assumem as seções dele e terminam com 2/2.
#[test]
#[ignore]
fn kill9_de_um_no() {
    let d = pasta("kill");
    let nos = [("a", "127.0.0.2"), ("b", "127.0.0.3"), ("c", "127.0.0.4")];
    let fed = federacao(&d, &nos);
    let sim = simulador(&d);
    let mut filhos: Vec<(Child, &str)> = nos.iter().map(|(id, ip)| (no(&d, &fed, id, ip, sim.porta), *id)).collect();
    // espera o nó a ter 1.500 BUs e derruba o b
    let t = Instant::now();
    loop {
        let n = std::fs::read(d.join("a/saida/saude.json")).ok().and_then(|b| serde_json::from_slice::<Value>(&b).ok())
            .and_then(|v| v["secoes"]["aplicadas"].as_u64()).unwrap_or(0);
        if n >= 1500 { break; }
        assert!(t.elapsed() < Duration::from_secs(300), "não chegou a 1.500 BUs");
        std::thread::sleep(Duration::from_millis(50));
    }
    let (mut b, _) = filhos.remove(1);
    Command::new("kill").args(["-9", &b.id().to_string()]).status().unwrap();
    let _ = b.wait();
    for (f, id) in filhos { esperar(f, 900, id); }
    drop(sim);
    for id in ["a", "c"] {
        let s = conferido(&d, id);
        let fe = &s["federacao"];
        assert!(fe["fora"].as_array().unwrap().iter().any(|x| x["no"] == "b"), "{id}: b não saiu da partição: {fe}");
        assert!(fe["mudancas_particao"].as_u64().unwrap() >= 1, "{id}: {fe}");
    }
    let assumidas: u64 = ["a", "c"].iter().map(|id| json(&d.join(format!("{id}/saida/saude.json")))["federacao"]["assumidas"].as_u64().unwrap()).sum();
    assert!(assumidas > 0, "ninguém assumiu as seções do b");
    assert_eq!(por_ip(&d).http429, 0);
}

/// A: `kill -9` de um nó e reinício: o que estava em memória é republicado do diário; os três terminam com 2/2.
#[test]
#[ignore]
fn kill9_e_reinicio() {
    let d = pasta("kill-reinicio");
    let nos = [("a", "127.0.0.2"), ("b", "127.0.0.3"), ("c", "127.0.0.4")];
    let fed = federacao(&d, &nos);
    let sim = simulador(&d);
    let mut filhos: Vec<(Child, &str)> = nos.iter().map(|(id, ip)| (no(&d, &fed, id, ip, sim.porta), *id)).collect();
    let t = Instant::now();
    loop {
        let n = std::fs::read(d.join("b/saida/saude.json")).ok().and_then(|b| serde_json::from_slice::<Value>(&b).ok())
            .and_then(|v| v["federacao"]["publicados"].as_u64()).unwrap_or(0);
        if n >= 400 { break; }
        assert!(t.elapsed() < Duration::from_secs(300), "o b não chegou a 400 publicados");
        std::thread::sleep(Duration::from_millis(20));
    }
    let (mut b, _) = filhos.remove(1);
    Command::new("kill").args(["-9", &b.id().to_string()]).status().unwrap();
    let _ = b.wait();
    // fica fora o bastante para os outros o declararem morto e coletarem as seções dele (morte: 3 s a 120×)
    std::thread::sleep(Duration::from_secs(6));
    filhos.push((no(&d, &fed, "b", "127.0.0.3", sim.porta), "b"));
    for (f, id) in filhos { esperar(f, 900, id); }
    drop(sim);
    for id in ["a", "b", "c"] { conferido(&d, id); }
    assert_eq!(por_ip(&d).http429, 0);
    // o b volta "sincronizando" e só entra na partição depois de alcançar os pares: não pede de novo o que eles
    // coletaram enquanto ele estava fora (sobram só as seções que ele tinha resolvido e não publicou antes do kill -9)
    let rep = aux_repetidos(&d);
    assert!(rep <= BUS / 50, "{rep} seções pedidas ao TSE por mais de um nó");
    let s = json(&d.join("b/saida/saude.json"));
    assert_eq!(s["federacao"]["sincronizando"].as_bool(), Some(false), "{}", s["federacao"]);
}

/// Seções (aux.json 200) pedidas ao simulador mais de uma vez, somando todos os IPs.
fn aux_repetidos(dir: &Path) -> u64 {
    let log = std::fs::read_to_string(dir.join("simulador.log")).unwrap();
    let mut n = std::collections::HashMap::<&str, u64>::new();
    for l in log.lines().filter(|l| !l.starts_with('#')) {
        let f: Vec<&str> = l.split(',').collect();
        if f.len() >= 7 && f[2] == "aux" && f[3] == "200" { *n.entry(f[5]).or_default() += 1; }
    }
    n.values().filter(|&&k| k > 1).count() as u64
}

/// A: nó cujo IP de saída medido não é o declarado (NAT não declarado) não coleta nada, mas termina com o resultado
/// completo pelo storage.
#[test]
#[ignore]
fn ip_divergente_nao_coleta() {
    let d = pasta("ip-divergente");
    let fed = federacao(&d, &[("a", "127.0.0.2"), ("b", "127.0.0.3"), ("x", "127.0.0.9")]);
    let sim = simulador(&d);
    let filhos: Vec<(Child, &str)> = [("a", "127.0.0.2"), ("b", "127.0.0.3"), ("x", "127.0.0.5")].iter().map(|(id, ip)| (no(&d, &fed, id, ip, sim.porta), *id)).collect();
    for (f, id) in filhos { esperar(f, 900, id); }
    drop(sim);
    for id in ["a", "b", "x"] { conferido(&d, id); }
    let s = json(&d.join("x/saida/saude.json"));
    assert_eq!(s["federacao"]["participa"].as_bool(), Some(false), "{}", s["federacao"]);
    assert_eq!(s["rede"]["aux"].as_u64(), Some(0), "o nó com IP divergente pediu aux.json");
    assert_eq!(por_ip(&d).aux.get("127.0.0.5"), None);
}

/// A (sincronização P2P): o nó `a` não alcança o `c` (no `federacao.json` dele, o endereço do `c` aponta para uma porta
/// fechada) e recebe os BUs do `c` por **repasse** do `b`; os três terminam com 2/2, e o `c` continua vivo para o `a`.
#[test]
#[ignore]
fn repasse_quando_um_par_nao_alcanca_outro() {
    let d = pasta("repasse");
    let nos: Vec<(&str, &str, String)> = [("a", "127.0.0.2"), ("b", "127.0.0.3"), ("c", "127.0.0.4")].iter()
        .map(|(id, ip)| (*id, *ip, format!("{ip}:{}", porta_livre()))).collect();
    let fed = federacao_em(&d, "federacao.json", &nos);
    // a visão do `a`: o `c` num endereço onde ninguém escuta
    let mut visao_a = nos.clone();
    visao_a[2].2 = format!("127.0.0.4:{}", porta_livre());
    let fed_a = federacao_em(&d, "federacao-a.json", &visao_a);
    let sim = simulador(&d);
    let filhos = vec![(no(&d, &fed_a, "a", "127.0.0.2", sim.porta), "a"), (no(&d, &fed, "b", "127.0.0.3", sim.porta), "b"), (no(&d, &fed, "c", "127.0.0.4", sim.porta), "c")];
    for (f, id) in filhos { esperar(f, 900, id); }
    drop(sim);
    for id in ["a", "b", "c"] { conferido(&d, id); }
    let s = json(&d.join("a/saida/saude.json"));
    let fe = &s["federacao"];
    assert_eq!(fe["ativos"].as_array().map(|v| v.len()), Some(3), "o `a` deveria ver o `c` vivo pelo repasse: {fe}");
    assert!(fe["sincronia"]["por_repasse"].as_u64().unwrap() > 0, "nenhum lote por repasse: {fe}");
    assert_eq!(por_ip(&d).http429, 0);
}
