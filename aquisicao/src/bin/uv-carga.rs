//! `uv-carga` — cliente de carga e de conferência da difusão do `uv-servico` (`docs/arquitetura/PROTOCOLO_WS.md`, etapa 4).
//!
//! Abre N conexões WebSocket (em rampa), inscreve cada uma num conjunto de tópicos, mede **BU → cliente**
//! (`agora − t_bu`) e **servidor → cliente** (`agora − t_envio`), conta `seq` pulados, regressões de `seq` e conexões
//! perdidas e, no fim (o serviço avisa `desligamento`), confere que a última mensagem de cada tópico em cada conexão é
//! **idêntica byte a byte** ao arquivo em `<saida>/publicacao/` e que os eventos recebidos são os do `eventos.csv`.
//! `--lentos K`: K conexões que se inscrevem e não leem nada (testam a juntada e o corte). `--reconectar`: reconecta
//! depois de uma queda (ensaio `kill -9`) e confere que o `instantaneo` não volta no `seq`. `--limites`: tenta quebrar
//! os limites do servidor (item 5 do handoff) e relata o que aconteceu. Relatório JSON com histogramas.

use bytes::Bytes;
use clap::Parser;
use futures::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio_tungstenite::tungstenite::Message;
use uv_aquisicao::difusao::agora_us;
use uv_aquisicao::servico::Amostra;

#[derive(Parser, Clone)]
#[command(name = "uv-carga", version, about = "Urna Velox — cliente de carga e conferência da difusão (WebSocket) do uv-servico")]
struct Cli {
    /// endereço do serviço (host:porta); ou use --porta-arquivo
    #[arg(long, default_value = "127.0.0.1:8080")]
    endereco: String,
    /// arquivo com a porta (o <saida>/http_porta do serviço); espera ele aparecer
    #[arg(long)]
    porta_arquivo: Option<PathBuf>,
    /// conexões que leem normalmente
    #[arg(long, default_value_t = 1000)]
    clientes: usize,
    /// conexões que se inscrevem e não leem nada
    #[arg(long, default_value_t = 0)]
    lentos: usize,
    /// conexões abertas por segundo
    #[arg(long, default_value_t = 200.0)]
    rampa: f64,
    /// perfil de inscrição: misto (50 % br-c0001; 50 % br-c0001 + uma UF c0001/c0003), br, todos (todos os tópicos), ou
    /// uma lista de tópicos separados por vírgula
    #[arg(long, default_value = "misto")]
    perfil: String,
    /// também pede todos os eventos (op eventos)
    #[arg(long)]
    eventos: bool,
    #[arg(long, default_value_t = 42)]
    semente: u64,
    /// pasta publicacao/ do serviço, para a conferência byte a byte no fim
    #[arg(long)]
    publicacao: Option<PathBuf>,
    /// eventos.csv do serviço, para conferir os eventos recebidos
    #[arg(long)]
    eventos_csv: Option<PathBuf>,
    /// tempo máximo (s); sem isto, até o serviço desligar
    #[arg(long, default_value_t = 7200.0)]
    duracao: f64,
    /// reconecta depois de uma queda (até o serviço avisar desligamento ou acabar a duração)
    #[arg(long)]
    reconectar: bool,
    /// threads do runtime
    #[arg(long, default_value_t = 4)]
    threads: usize,
    /// relatório JSON
    #[arg(long)]
    relatorio: Option<PathBuf>,
    /// em vez da carga: tenta quebrar os limites do servidor e relata
    #[arg(long)]
    limites: bool,
    /// grava em JSONL as duas primeiras mensagens de cada tipo da conexão 0 (exemplos para o protocolo)
    #[arg(long)]
    amostras: Option<PathBuf>,
}

#[derive(Default)]
struct Global {
    lat_bu: Mutex<Amostra>,
    lat_srv: Mutex<Amostra>,
    msgs: AtomicU64,
    bytes: AtomicU64,
    por_tipo: Mutex<BTreeMap<String, u64>>,
    avisos_pulo: AtomicU64,
    juntadas: AtomicU64,
    regressoes: AtomicU64,
    conectadas: AtomicU64,
    falhas_conexao: AtomicU64,
    reconexoes: AtomicU64,
    reconexao_seq_menor: AtomicU64,
    fins: Mutex<BTreeMap<String, u64>>,
    erros: Mutex<BTreeMap<String, u64>>,
    desligamento: AtomicBool,
    /// resultado de cada conexão normal: últimos bytes por tópico, eventos recebidos (chave sem seq)
    finais: Mutex<Vec<(usize, Vec<String>, bool, HashMap<String, (u64, Bytes)>, BTreeSet<String>)>>,
    lentos_fim: Mutex<BTreeMap<String, u64>>,
    /// mudanças do `estado` vistas pela conexão 0: (s desde o início, estado, encerrada, pausa ativa, retomada_em, alertas)
    estados: Mutex<Vec<Value>>,
    /// avisos e eventos vistos pela conexão 0 (fora os de matematicamente, que vão na conferência)
    avisos_c0: Mutex<Vec<Value>>,
    amostras: Mutex<Vec<String>>,
    t0: Mutex<Option<Instant>>,
}

fn conta(m: &Mutex<BTreeMap<String, u64>>, k: &str) { *m.lock().unwrap().entry(k.to_string()).or_default() += 1; }

/// Gerador simples e reprodutível (xorshift).
struct Rng(u64);
impl Rng {
    fn prox(&mut self) -> u64 { self.0 ^= self.0 << 13; self.0 ^= self.0 >> 7; self.0 ^= self.0 << 17; self.0 }
    fn ate(&mut self, n: usize) -> usize { (self.prox() % n.max(1) as u64) as usize }
}

/// Campo numérico do cabeçalho (antes de `"resultado":`).
fn campo_num(cab: &str, k: &str) -> Option<u64> {
    let i = cab.find(&format!("\"{k}\":"))? + k.len() + 3;
    let f = cab[i..].find([',', '}']).map(|x| i + x)?;
    cab[i..f].trim().parse().ok()
}
fn campo_txt<'a>(cab: &'a str, k: &str) -> Option<&'a str> {
    let i = cab.find(&format!("\"{k}\":\""))? + k.len() + 4;
    let f = cab[i..].find('"')? + i;
    Some(&cab[i..f])
}

fn chave_evento(v: &Value) -> String {
    let s = |k: &str| match &v[k] { Value::Null => String::new(), Value::String(x) => x.clone(), x => x.to_string() };
    format!("{}|{}|{}|{}", s("abrangencia"), s("cargo"), s("candidato"), s("estado"))
}

async fn endereco(a: &Cli) -> String {
    let Some(p) = &a.porta_arquivo else { return a.endereco.clone() };
    loop {
        if let Ok(s) = std::fs::read_to_string(p) { if let Ok(n) = s.trim().parse::<u16>() { return format!("127.0.0.1:{n}"); } }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

fn urls(end: &str) -> (String, String) {
    let limpo = end.trim();
    if limpo.starts_with("wss://") {
        let host = &limpo[6..];
        (format!("https://{host}"), format!("wss://{host}/ws"))
    } else if limpo.starts_with("ws://") {
        let host = &limpo[5..];
        (format!("http://{host}"), format!("ws://{host}/ws"))
    } else if limpo.starts_with("https://") {
        let host = &limpo[8..];
        (format!("https://{host}"), format!("wss://{host}/ws"))
    } else if limpo.starts_with("http://") {
        let host = &limpo[7..];
        (format!("http://{host}"), format!("ws://{host}/ws"))
    } else if limpo.contains(":443") || (!limpo.contains(":") && !limpo.starts_with("127.0.0.1") && !limpo.starts_with("localhost")) {
        (format!("https://{limpo}"), format!("wss://{limpo}/ws"))
    } else {
        (format!("http://{limpo}"), format!("ws://{limpo}/ws"))
    }
}

async fn topicos(end: &str) -> anyhow::Result<Vec<String>> {
    let (http_base, _) = urls(end);
    let r = reqwest::get(format!("{http_base}/api/v1/topicos")).await?.text().await?;
    let v: Value = serde_json::from_str(&r)?;
    Ok(v.as_array().map(|a| a.iter().filter_map(|x| x["topico"].as_str().map(String::from)).collect()).unwrap_or_default())
}

/// Tópicos da conexão `i` pelo perfil.
fn escolher(perfil: &str, todos: &[String], i: usize, rng: &mut Rng) -> Vec<String> {
    match perfil {
        "br" => vec!["br-c0001".into()],
        "todos" => todos.to_vec(),
        "misto" => {
            let principal = if todos.iter().any(|t| t == "br-c0001") {
                "br-c0001".to_string()
            } else if !todos.is_empty() {
                todos[0].clone()
            } else {
                "br-c0001".to_string()
            };
            let mut v = vec![principal];
            if i % 2 == 1 && todos.len() > 1 {
                let ufs: Vec<&String> = todos.iter().filter(|t| t.len() == 8 && !t.starts_with("br") && (t.ends_with("c0001") || t.ends_with("c0003"))).collect();
                if !ufs.is_empty() {
                    v.push(ufs[rng.ate(ufs.len())].clone());
                } else {
                    v.push(todos[rng.ate(todos.len())].clone());
                }
            }
            v
        }
        lista => lista.split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect(),
    }
}

/// Uma conexão normal: lê até o fim, reconectando se pedido. Devolve os últimos bytes por tópico e os eventos.
async fn cliente(g: Arc<Global>, a: Arc<Cli>, end: String, i: usize, tops: Vec<String>, fim_em: Instant) {
    let mut ult: HashMap<String, (u64, Bytes)> = HashMap::new();
    let mut evs: BTreeSet<String> = BTreeSet::new();
    let mut tentativa = 0u32;
    let mut primeira = true;
    'conexoes: loop {
        if Instant::now() >= fim_em { break; }
        let (_, ws_url) = urls(&end);
        let ws = match tokio_tungstenite::connect_async(&ws_url).await {
            Ok((ws, _)) => ws,
            Err(e) => {
                g.falhas_conexao.fetch_add(1, Relaxed);
                if !a.reconectar || g.desligamento.load(Relaxed) { conta(&g.fins, &format!("falha_conexao: {}", e.to_string().chars().take(60).collect::<String>())); break; }
                tentativa += 1;
                tokio::time::sleep(Duration::from_millis((250u64 << tentativa.min(5)).min(5000))).await;
                continue;
            }
        };
        if !primeira { g.reconexoes.fetch_add(1, Relaxed); }
        primeira = false;
        tentativa = 0;
        g.conectadas.fetch_add(1, Relaxed);
        let (mut tx, mut rx) = ws.split();
        let mut ins = json!({"op": "inscrever", "topicos": tops}).to_string();
        if tx.send(Message::text(ins.clone())).await.is_err() { conta(&g.fins, "erro_envio"); continue; }
        if a.eventos { ins = json!({"op": "eventos"}).to_string(); let _ = tx.send(Message::text(ins)).await; }
        // depois de uma reconexão, o primeiro instantaneo de cada tópico não pode voltar no seq
        let mut conferir_reconexao: HashMap<String, u64> = ult.iter().map(|(t, (s, _))| (t.clone(), *s)).collect();
        loop {
            let resto = fim_em.saturating_duration_since(Instant::now());
            let m = match tokio::time::timeout(resto, rx.next()).await {
                Err(_) => { conta(&g.fins, "duracao"); break 'conexoes; }
                Ok(None) => { conta(&g.fins, if g.desligamento.load(Relaxed) { "fechada_desligamento" } else { "fechada_sem_aviso" }); break; }
                Ok(Some(Err(e))) => { conta(&g.fins, &format!("erro: {}", e.to_string().chars().take(60).collect::<String>())); break; }
                Ok(Some(Ok(m))) => m,
            };
            let agora = agora_us();
            let t = match m {
                Message::Text(t) => t,
                Message::Close(c) => {
                    let motivo = c.map(|c| format!("close {} {}", u16::from(c.code), c.reason)).unwrap_or("close".into());
                    conta(&g.fins, &motivo);
                    if motivo.contains("desligamento") { break 'conexoes; }
                    break;
                }
                _ => continue,
            };
            g.msgs.fetch_add(1, Relaxed);
            g.bytes.fetch_add(t.len() as u64, Relaxed);
            let s = t.as_str();
            let cab_fim = s.find("\"resultado\":").unwrap_or(s.len());
            let cab = &s[..cab_fim];
            let tipo = campo_txt(cab, "tipo").unwrap_or("?").to_string();
            conta(&g.por_tipo, &tipo);
            if i == 0 {
                if a.amostras.is_some() {
                    let mut am = g.amostras.lock().unwrap();
                    if am.iter().filter(|x| x.contains(&format!("\"tipo\":\"{tipo}\""))).count() < 2 && am.len() < 40 { am.push(s.to_string()); }
                }
                let seg = g.t0.lock().unwrap().map(|t| (t.elapsed().as_secs_f64() * 10.0).round() / 10.0).unwrap_or(0.0);
                if tipo == "estado" {
                    if let Ok(v) = serde_json::from_str::<Value>(s) {
                        let e = json!({"s": seg, "estado": v["estado"], "encerrada": v["encerrada"], "pausa": v["pausa_429"]["ativa"], "retomada_em": v["pausa_429"]["retomada_em"],
                            "relogio_site": v["relogio_site"], "alertas": v["alertas"]});
                        let mut es = g.estados.lock().unwrap();
                        let muda = es.last().is_none_or(|u: &Value| u["estado"] != e["estado"] || u["encerrada"] != e["encerrada"] || u["pausa"] != e["pausa"]
                            || u["alertas"].as_object().map(|o| o.keys().collect::<Vec<_>>()) != e["alertas"].as_object().map(|o| o.keys().collect::<Vec<_>>()));
                        if muda { es.push(e); }
                    }
                } else if tipo == "aviso" || (tipo == "evento" && !s.contains("\"classe\":\"matematicamente\"")) {
                    if let Ok(mut v) = serde_json::from_str::<Value>(s) {
                        let mut av = g.avisos_c0.lock().unwrap();
                        if v["motivo"] != "seq_pulado" && av.len() < 50 { v["s"] = json!(seg); av.push(v); }
                    }
                }
            }
            match tipo.as_str() {
                "atualizacao" | "instantaneo" => {
                    let (Some(top), Some(seq)) = (campo_txt(cab, "topico"), campo_num(cab, "seq")) else { conta(&g.erros, "mensagem_ilegivel"); continue };
                    let ini = cab_fim + "\"resultado\":".len();
                    let res = Bytes::copy_from_slice(&s.as_bytes()[ini..s.len() - 1]);
                    if let Some(s0) = conferir_reconexao.remove(top) { if seq < s0 { g.reconexao_seq_menor.fetch_add(1, Relaxed); } }
                    if let Some((s0, _)) = ult.get(top) { if seq < *s0 { g.regressoes.fetch_add(1, Relaxed); } }
                    if tipo == "atualizacao" {
                        if let Some(tb) = campo_num(cab, "t_bu") { g.lat_bu.lock().unwrap().por(agora.saturating_sub(tb) as f64 / 1e3); }
                        if let Some(te) = campo_num(cab, "t_envio") { g.lat_srv.lock().unwrap().por(agora.saturating_sub(te) as f64 / 1e3); }
                    }
                    ult.insert(top.to_string(), (seq, res));
                }
                "evento" => {
                    if let Ok(v) = serde_json::from_str::<Value>(s) { if v["classe"] == "matematicamente" { evs.insert(chave_evento(&v)); } }
                }
                "aviso" => {
                    if let Ok(v) = serde_json::from_str::<Value>(s) {
                        match v["motivo"].as_str() {
                            Some("seq_pulado") => { g.avisos_pulo.fetch_add(1, Relaxed); g.juntadas.fetch_add(v["juntadas"].as_u64().unwrap_or(0), Relaxed); }
                            Some("desligamento") => { g.desligamento.store(true, Relaxed); }
                            _ => {}
                        }
                    }
                }
                "erro" => { if let Ok(v) = serde_json::from_str::<Value>(s) { conta(&g.erros, v["motivo"].as_str().unwrap_or("?")); } }
                _ => {}
            }
        }
        if !a.reconectar || g.desligamento.load(Relaxed) { break; }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    g.finais.lock().unwrap().push((i, tops, a.eventos, ult, evs));
}

/// Conexão lenta: inscreve e não lê nada até o fim; depois vê se o servidor a derrubou.
async fn lento(g: Arc<Global>, end: String, tops: Vec<String>, fim: Arc<tokio::sync::Notify>) {
    let (_, ws_url) = urls(&end);
    let Ok((mut ws, _)) = tokio_tungstenite::connect_async(&ws_url).await else { conta(&g.lentos_fim, "falha_conexao"); return };
    if ws.send(Message::text(json!({"op": "inscrever", "topicos": tops}).to_string())).await.is_err() { conta(&g.lentos_fim, "erro_envio"); return; }
    fim.notified().await;
    // drena o que ficou no buffer até o fechamento: se o servidor cortou, vem um erro ou um close antes do desligamento
    let mut motivo = "viva".to_string();
    while let Ok(Some(m)) = tokio::time::timeout(Duration::from_secs(5), ws.next()).await {
        match m {
            Ok(Message::Close(c)) => { motivo = c.map(|c| format!("close {} {}", u16::from(c.code), c.reason)).unwrap_or("close".into()); break; }
            Ok(Message::Text(t)) if t.as_str().contains("\"desligamento\"") => { motivo = "viva_ate_desligamento".into(); break; }
            Ok(_) => continue,
            Err(e) => { motivo = format!("cortada: {}", e.to_string().chars().take(50).collect::<String>()); break; }
        }
    }
    conta(&g.lentos_fim, &motivo);
}

fn cpu_s() -> f64 {
    let Ok(s) = std::fs::read_to_string("/proc/self/stat") else { return 0.0 };
    let Some(r) = s.rsplit_once(')').map(|x| x.1) else { return 0.0 };
    let f: Vec<&str> = r.split_whitespace().collect();
    let tck = unsafe { libc::sysconf(libc::_SC_CLK_TCK) }.max(1) as f64;
    (f.get(11).and_then(|x| x.parse::<f64>().ok()).unwrap_or(0.0) + f.get(12).and_then(|x| x.parse::<f64>().ok()).unwrap_or(0.0)) / tck
}

/// Conferência: últimos bytes de cada tópico × arquivo; eventos × eventos.csv.
fn conferir(a: &Cli, g: &Global) -> Value {
    let finais = g.finais.lock().unwrap();
    let mut arquivos: HashMap<String, Vec<u8>> = HashMap::new();
    if let Some(d) = &a.publicacao {
        if let Ok(rd) = std::fs::read_dir(d) {
            for e in rd.flatten() {
                let n = e.file_name().to_string_lossy().into_owned();
                if let Some(t) = uv_aquisicao::difusao::topico_de(&n) { if let Ok(b) = std::fs::read(e.path()) { arquivos.insert(t.to_string(), b); } }
            }
        }
    }
    let (mut conferidos, mut identicos, mut sem_msg) = (0u64, 0u64, 0u64);
    let mut divergentes: Vec<String> = vec![];
    let mut topicos_vistos = BTreeSet::new();
    if a.publicacao.is_some() {
        for (i, tops, _, ult, _) in finais.iter() {
            for t in tops {
                let Some(arq) = arquivos.get(t) else { continue };
                conferidos += 1;
                topicos_vistos.insert(t.clone());
                match ult.get(t) {
                    Some((_, b)) if b[..] == arq[..] => identicos += 1,
                    Some((s, _)) => if divergentes.len() < 20 { divergentes.push(format!("conexão {i} {t}: última mensagem seq {s} ≠ arquivo")) },
                    None => { sem_msg += 1; if divergentes.len() < 20 { divergentes.push(format!("conexão {i} {t}: sem mensagem")) } }
                }
            }
        }
    }
    // eventos esperados por tópico (sem seq: a retomada recalcula com outro seq)
    let mut ev_csv: HashMap<String, BTreeSet<String>> = HashMap::new();
    let mut ev_todos = BTreeSet::new();
    if let Some(p) = &a.eventos_csv {
        for l in std::fs::read_to_string(p).unwrap_or_default().lines().skip(1) {
            if l.starts_with('#') || l.trim().is_empty() { continue; }
            let f: Vec<&str> = l.split(',').collect();
            if f.len() < 6 { continue; }
            let k = format!("{}|{}|{}|{}", f[2], f[3], f[4], f[5]);
            ev_csv.entry(format!("{}-c{:04}", f[2], f[3].parse::<u32>().unwrap_or(0))).or_default().insert(k.clone());
            ev_todos.insert(k);
        }
    }
    let (mut ev_conferidas, mut ev_iguais) = (0u64, 0u64);
    let mut ev_div = vec![];
    if a.eventos_csv.is_some() {
        for (i, tops, todos, _, evs) in finais.iter() {
            let esperado: BTreeSet<String> = if *todos { ev_todos.clone() } else { tops.iter().flat_map(|t| ev_csv.get(t).cloned().unwrap_or_default()).collect() };
            ev_conferidas += 1;
            if *evs == esperado { ev_iguais += 1; } else if ev_div.len() < 10 {
                ev_div.push(json!({"conexao": i, "faltando": esperado.difference(evs).collect::<Vec<_>>(), "sobrando": evs.difference(&esperado).collect::<Vec<_>>()}));
            }
        }
    }
    let br_presidente = ev_csv.get("br-c0001").map(|s| s.len()).unwrap_or(0);
    json!({"publicacao": {"conferidos": conferidos, "identicos": identicos, "sem_mensagem": sem_msg, "topicos": topicos_vistos.len(), "divergentes": divergentes},
        "eventos": {"conexoes": ev_conferidas, "iguais_ao_csv": ev_iguais, "eventos_csv": ev_todos.len(), "eventos_br_c0001": br_presidente, "diferencas": ev_div}})
}

// ------------------------------------------------------------------ teste dos limites (--limites)

async fn ws_abrir(end: &str) -> anyhow::Result<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>> {
    let (_, ws_url) = urls(end);
    Ok(tokio_tungstenite::connect_async(ws_url).await?.0)
}

/// Lê até fechar (ou 10 s): devolve os tipos/motivos vistos e como terminou.
async fn ate_fechar(ws: &mut tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>, max: Duration) -> (BTreeMap<String, u64>, String) {
    let mut vistos = BTreeMap::new();
    let t = Instant::now();
    loop {
        let resto = max.saturating_sub(t.elapsed());
        match tokio::time::timeout(resto, ws.next()).await {
            Err(_) => return (vistos, "aberta".into()),
            Ok(None) => return (vistos, "fechada".into()),
            Ok(Some(Err(e))) => return (vistos, format!("erro: {}", e.to_string().chars().take(60).collect::<String>())),
            Ok(Some(Ok(Message::Close(c)))) => return (vistos, c.map(|c| format!("close {} {}", u16::from(c.code), c.reason)).unwrap_or("close".into())),
            Ok(Some(Ok(Message::Text(x)))) => {
                let v: Value = serde_json::from_str(x.as_str()).unwrap_or(Value::Null);
                let k = format!("{}{}", v["tipo"].as_str().unwrap_or("?"), v["motivo"].as_str().map(|m| format!(":{m}")).unwrap_or_default());
                *vistos.entry(k).or_insert(0) += 1;
            }
            Ok(Some(Ok(_))) => {}
        }
    }
}

async fn limites(end: &str) -> anyhow::Result<Value> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut r = serde_json::Map::new();
    // 1. mensagem de 1 MB
    let mut ws = ws_abrir(end).await?;
    let grande = format!("{{\"op\":\"inscrever\",\"topicos\":[\"{}\"]}}", "x".repeat(1 << 20));
    let envio = ws.send(Message::text(grande)).await.map(|_| "enviada".to_string()).unwrap_or_else(|e| format!("erro no envio: {e}"));
    let (v, fim) = ate_fechar(&mut ws, Duration::from_secs(10)).await;
    r.insert("mensagem_1mb".into(), json!({"envio": envio, "recebido": v, "fim": fim}));
    // 2. 1.000 mensagens em 1 s
    let mut ws = ws_abrir(end).await?;
    let t = Instant::now();
    let mut enviadas = 0;
    for _ in 0..1000 {
        if ws.send(Message::text(r#"{"op":"cancelar","topicos":["br-c0001"]}"#)).await.is_err() { break; }
        enviadas += 1;
        tokio::time::sleep(Duration::from_micros(900)).await;
    }
    let dur = t.elapsed().as_secs_f64();
    let (v, fim) = ate_fechar(&mut ws, Duration::from_secs(10)).await;
    r.insert("taxa_1000_msgs".into(), json!({"enviadas": enviadas, "segundos": (dur * 100.0).round() / 100.0, "recebido": v, "fim": fim}));
    // 3. limite de 200 tópicos (com os existentes, repetidos sob nomes que não existem quando são poucos: os
    //    inexistentes dão `topico_desconhecido` e não contam), tópico com caminho, JSON inválido, op desconhecida,
    //    binário — em mensagens pequenas (< 4 KB), espaçadas para não cair no limite de taxa
    let mut ws = ws_abrir(end).await?;
    let todos = topicos(end).await.unwrap_or_default();
    let mut n_ins = 0;
    for bloco in todos.chunks(40).take(8) {
        ws.send(Message::text(json!({"op": "inscrever", "topicos": bloco}).to_string())).await?;
        n_ins += bloco.len();
        tokio::time::sleep(Duration::from_millis(120)).await;
    }
    for m in [r#"{"op":"inscrever","topicos":["../../etc/passwd","br-c0001-e006257.json"]}"#, "isto não é json", r#"{"op":"apagar"}"#] {
        ws.send(Message::text(m)).await?;
        tokio::time::sleep(Duration::from_millis(120)).await;
    }
    ws.send(Message::binary(vec![1u8, 2, 3])).await?;
    let (v, fim) = ate_fechar(&mut ws, Duration::from_secs(3)).await;
    r.insert("formato_e_topicos".into(), json!({"topicos_existentes": todos.len(), "pedidos_existentes": n_ins, "recebido": v, "fim": fim}));
    // 4. handshake incompleto (cabeçalho pela metade)
    let mut s = tokio::net::TcpStream::connect(end).await?;
    s.write_all(b"GET /ws HTTP/1.1\r\nHost: x\r\n").await?;
    let t = Instant::now();
    let mut buf = [0u8; 256];
    let fechou = match tokio::time::timeout(Duration::from_secs(20), s.read(&mut buf)).await { Ok(Ok(0)) => "fechada".to_string(), Ok(Ok(n)) => format!("resposta: {}", String::from_utf8_lossy(&buf[..n]).lines().next().unwrap_or("")), Ok(Err(e)) => format!("erro: {e}"), Err(_) => "aberta após 20 s".into() };
    r.insert("handshake_incompleto".into(), json!({"resultado": fechou, "segundos": (t.elapsed().as_secs_f64() * 10.0).round() / 10.0}));
    // 5. REST: caminho vindo do cliente, ETag/304, rota de escrita
    let (http_base, _) = urls(end);
    let c = reqwest::Client::new();
    let mut rest = serde_json::Map::new();
    for p in ["/api/v1/resultado/..%2F..%2Fetc%2Fpasswd", "/api/v1/resultado/../../saude.json", "/api/v1/resultado/br-c0001-e006257.json", "/api/v1/../../saude.json", "/publicacao/br-c0001-e006257.json"] {
        let s = c.get(format!("{http_base}{p}")).send().await.map(|r| r.status().as_u16()).unwrap_or(0);
        rest.insert(p.into(), json!(s));
    }
    let s = c.post(format!("{http_base}/api/v1/resultado/br-c0001")).body("x").send().await.map(|r| r.status().as_u16()).unwrap_or(0);
    rest.insert("POST /api/v1/resultado/br-c0001".into(), json!(s));
    let t0 = todos.first().cloned().unwrap_or_else(|| "br-c0001".into());
    let r1 = c.get(format!("{http_base}/api/v1/resultado/{t0}")).send().await?;
    let st1 = r1.status().as_u16();
    let etag = r1.headers().get("etag").and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
    let cors = r1.headers().get("access-control-allow-origin").and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
    let cache = r1.headers().get("cache-control").and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
    let s2 = c.get(format!("{http_base}/api/v1/resultado/{t0}")).header("If-None-Match", &etag).send().await?.status().as_u16();
    rest.insert("etag".into(), json!({"topico": t0, "status": st1, "etag": etag, "cors": cors, "cache_control": cache, "com_if_none_match": s2}));
    r.insert("rest".into(), Value::Object(rest));
    Ok(Value::Object(r))
}

fn main() -> anyhow::Result<()> {
    let a = Arc::new(Cli::parse());
    let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(a.threads.max(1)).enable_all().build()?;
    let rel = rt.block_on(async {
        let end = endereco(&a).await;
        // o serviço pode estar subindo: espera a lista de tópicos
        let t = Instant::now();
        let todos = loop {
            match topicos(&end).await { Ok(v) if !v.is_empty() => break v, _ if t.elapsed() > Duration::from_secs(120) => anyhow::bail!("{end}: sem /api/v1/topicos"), _ => tokio::time::sleep(Duration::from_millis(300)).await }
        };
        if a.limites { return Ok(json!({"endereco": end, "limites": limites(&end).await?})); }
        let g = Arc::new(Global::default());
        let t0 = Instant::now();
        *g.t0.lock().unwrap() = Some(t0);
        let n = a.clientes + a.lentos;
        let tempo_rampa = if a.rampa > 0.0 { n as f64 / a.rampa } else { 0.0 };
        let fim_em = t0 + Duration::from_secs_f64(a.duracao + tempo_rampa);
        let mut rng = Rng(a.semente.max(1));
        let mut hs = vec![];
        let fim_lentos = Arc::new(tokio::sync::Notify::new());
        let mut hl = vec![];
        for i in 0..n {
            let tops = escolher(&a.perfil, &todos, i, &mut rng);
            if i >= a.clientes { hl.push(tokio::spawn(lento(g.clone(), end.clone(), tops, fim_lentos.clone()))); }
            else { hs.push(tokio::spawn(cliente(g.clone(), a.clone(), end.clone(), i, tops, fim_em))); }
            if a.rampa > 0.0 { tokio::time::sleep(Duration::from_secs_f64(1.0 / a.rampa)).await; }
        }
        eprintln!("uv-carga: {} conexões abertas em {:.1} s ({} lentas) em {end}", n, t0.elapsed().as_secs_f64(), a.lentos);
        let cpu0 = cpu_s();
        let mut prox = Instant::now();
        while hs.iter().any(|h| !h.is_finished()) {
            tokio::time::sleep(Duration::from_millis(200)).await;
            if Instant::now() >= prox {
                prox = Instant::now() + Duration::from_secs(10);
                eprintln!("uv-carga: {:.0} s, {} mensagens, BU→cliente p99 {:.2} ms", t0.elapsed().as_secs_f64(), g.msgs.load(Relaxed), g.lat_bu.lock().unwrap().pct_total(99.0));
            }
        }
        fim_lentos.notify_waiters();
        for h in hl { let _ = h.await; }
        let dur = t0.elapsed().as_secs_f64();
        let conf = conferir(&a, &g);
        if let Some(p) = &a.amostras { let _ = std::fs::write(p, g.amostras.lock().unwrap().join("\n") + "\n"); }
        Ok(json!({
            "conexao0": {"estados": g.estados.lock().unwrap().clone(), "avisos_e_eventos_de_apuracao": g.avisos_c0.lock().unwrap().clone()},
            "versao": urna_velox::VERSAO, "endereco": end, "clientes": a.clientes, "lentos": a.lentos, "perfil": a.perfil, "semente": a.semente, "eventos": a.eventos,
            "duracao_s": (dur * 10.0).round() / 10.0, "cpu_pct_medio": ((cpu_s() - cpu0) / dur * 1000.0).round() / 10.0,
            "conexoes": {"abertas": g.conectadas.load(Relaxed), "falhas": g.falhas_conexao.load(Relaxed), "reconexoes": g.reconexoes.load(Relaxed),
                "fins": g.fins.lock().unwrap().clone(), "lentas": g.lentos_fim.lock().unwrap().clone()},
            "mensagens": {"total": g.msgs.load(Relaxed), "bytes": g.bytes.load(Relaxed), "por_tipo": g.por_tipo.lock().unwrap().clone(), "erros": g.erros.lock().unwrap().clone()},
            "seq": {"avisos_pulado": g.avisos_pulo.load(Relaxed), "juntadas": g.juntadas.load(Relaxed), "regressoes": g.regressoes.load(Relaxed),
                "reconexao_instantaneo_menor": g.reconexao_seq_menor.load(Relaxed)},
            "latencias_ms": {"bu_cliente": g.lat_bu.lock().unwrap().json(), "servidor_cliente": g.lat_srv.lock().unwrap().json()},
            "conferencia": conf,
        }))
    })?;
    let txt = serde_json::to_string_pretty(&rel)?;
    match &a.relatorio { Some(p) => std::fs::write(p, txt + "\n")?, None => println!("{txt}") }
    Ok(())
}
