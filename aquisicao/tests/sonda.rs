//! Sondas (padrão *Test Message*, `uv_aquisicao::sonda`, `docs/arquitetura/SONDA.md`) de ponta a ponta, num `uv-servico` de
//! verdade, sem dados reais e sem rede: `--sem-coleta` com o `candidatos.json` das fixtures do motor (nenhum `cs.json`,
//! `aux.json` ou BU; nenhuma requisição). Rodam no `cargo test` comum (não precisam de `UV_DADOS`).
//!
//! * as sondas atravessam motor → gravação → difusão → WebSocket, na ordem, com os carimbos em ordem;
//! * o `uv-sonda` (verificador de fora) confirma o caminho e sai com 0; sem sondas, sai com 1;
//! * com o motor travado (falha provocada `TRAVAR_MOTOR`) o processo segue vivo, mas a sonda não volta: alerta
//!   `sonda_atrasada` (uma vez), o `uv-sonda` vê `atrasada`; destravado, `sonda_normalizada` e nada perdido.

use futures::{SinkExt, StreamExt};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};
use tokio_tungstenite::tungstenite::Message;

fn candidatos() -> PathBuf { PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../motor/tests/fixtures/candidatos_exemplo.json") }

/// Pasta temporária apagada no fim (fica se o teste falhar, para o diagnóstico).
struct Pasta(PathBuf);
impl std::ops::Deref for Pasta { type Target = PathBuf; fn deref(&self) -> &PathBuf { &self.0 } }
impl Drop for Pasta {
    fn drop(&mut self) { if !std::thread::panicking() && std::env::var_os("UV_MANTER_ENSAIO").is_none() { let _ = std::fs::remove_dir_all(&self.0); } }
}
fn pasta(nome: &str) -> Pasta {
    let d = std::env::temp_dir().join(format!("uv-sonda-{}-{nome}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    Pasta(d)
}

/// O serviço como processo filho; parado com SIGTERM (parada limpa) no fim.
struct Servico { filho: Child, porta: u16, saida: PathBuf }
impl Drop for Servico {
    fn drop(&mut self) {
        unsafe { libc::kill(self.filho.id() as i32, libc::SIGTERM); }
        let t = Instant::now();
        while t.elapsed() < Duration::from_secs(10) { if let Ok(Some(_)) = self.filho.try_wait() { return; } std::thread::sleep(Duration::from_millis(20)); }
        let _ = self.filho.kill();
        let _ = self.filho.wait();
    }
}

fn servico(dir: &Path, extra: &[&str]) -> Servico {
    let saida = dir.join("saida");
    let filho = Command::new(env!("CARGO_BIN_EXE_uv-servico"))
        .args(["--pleito", "3221", "--ciclo", "ele2026", "--candidatos", candidatos().to_str().unwrap(),
            "--dados", dir.join("dados").to_str().unwrap(), "--saida", saida.to_str().unwrap(),
            // nenhuma requisição: sem coleta e sem comparação com o TSE (a base local só habilita a falha provocada)
            "--base", "http://127.0.0.1:9", "--sem-coleta", "--sem-tse", "--previsao-intervalo", "0", "--http", "127.0.0.1:0"])
        .args(extra)
        .stdout(Stdio::null()).stderr(std::fs::File::create(dir.join("servico.err")).unwrap()).spawn().unwrap();
    let t = Instant::now();
    loop {
        if let Some(p) = std::fs::read_to_string(saida.join("http_porta")).ok().and_then(|s| s.trim().parse().ok()) { return Servico { filho, porta: p, saida }; }
        assert!(t.elapsed() < Duration::from_secs(30), "o serviço não abriu a porta: {}", std::fs::read_to_string(dir.join("servico.err")).unwrap_or_default());
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn json(p: &Path) -> Value { serde_json::from_slice(&std::fs::read(p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))).unwrap() }

/// Espera a condição sobre a `saude.json` (gravada a cada 2 s pelo motor).
fn esperar_saude(s: &Servico, limite: Duration, f: impl Fn(&Value) -> bool) -> Value {
    let t = Instant::now();
    loop {
        if let Ok(b) = std::fs::read(s.saida.join("saude.json")) { if let Ok(v) = serde_json::from_slice::<Value>(&b) { if f(&v) { return v; } } }
        assert!(t.elapsed() < limite, "saude.json não chegou à condição em {limite:?}");
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn esperar_texto(p: &Path, trecho: &str, limite: Duration) -> String {
    let t = Instant::now();
    loop {
        let x = std::fs::read_to_string(p).unwrap_or_default();
        if x.contains(trecho) { return x; }
        assert!(t.elapsed() < limite, "{} sem {trecho:?} em {limite:?}:\n{x}", p.display());
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// `uv-sonda` como processo filho, com duração: código de saída e o relatório.
fn verificador(dir: &Path, porta: u16, args: &[&str]) -> (bool, Value) {
    let rel = dir.join(format!("verificador-{}.json", args.join("_").replace(['/', '.'], "")));
    let st = Command::new(env!("CARGO_BIN_EXE_uv-sonda"))
        .args(["--url", &format!("127.0.0.1:{porta}"), "--saida", rel.to_str().unwrap()]).args(args)
        .stdout(Stdio::null()).stderr(Stdio::null()).status().unwrap();
    (st.success(), json(&rel))
}

type Ws = tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

async fn prox_sonda(ws: &mut Ws, limite: Duration) -> Option<Value> {
    let fim = tokio::time::Instant::now() + limite;
    loop {
        let m = tokio::time::timeout_at(fim, ws.next()).await.ok()?;
        if let Some(Ok(Message::Text(t))) = m {
            let v: Value = serde_json::from_str(t.as_str()).unwrap();
            if v["tipo"] == "sonda" { return Some(v); }
        }
    }
}

#[test]
fn sondas_atravessam_o_servico() {
    let d = pasta("ponta");
    let s = servico(&d, &["--sonda-intervalo", "0.2"]);
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    let vistas = rt.block_on(async {
        let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{}/ws", s.porta)).await.unwrap();
        ws.send(Message::text(r#"{"op":"sonda"}"#)).await.unwrap();
        let mut v = vec![];
        while v.len() < 6 {
            let m = prox_sonda(&mut ws, Duration::from_secs(10)).await.expect("as sondas pararam de chegar");
            if m["instantaneo"] == false { v.push(m); }
        }
        v
    });
    let ids: Vec<u64> = vistas.iter().map(|m| m["id"].as_u64().unwrap()).collect();
    assert!(ids.windows(2).all(|w| w[1] == w[0] + 1), "ids consecutivos: {ids:?}");
    for m in &vistas {
        assert_eq!((m["local"].as_bool(), m["origem"].as_str(), m["seq"].as_u64()), (Some(true), None, Some(0)), "{m}");
        let ts: Vec<u64> = ["t_injecao", "t_motor", "t_gravacao", "t_difusao", "t_envio"].iter().map(|k| m[*k].as_u64().unwrap()).collect();
        assert!(ts.windows(2).all(|w| w[0] <= w[1]), "carimbos em ordem: {ts:?}");
        assert!(ts[4] - ts[0] < 2_000_000, "injeção → socket em menos de 2 s com o serviço ocioso: {m}");
        assert_eq!(m["sessao"], vistas[0]["sessao"]);
    }
    // verificador de fora: caminho andando → código 0
    let (ok, rel) = verificador(&d, s.porta, &["--duracao", "2", "--limite", "1", "--min-sondas", "3"]);
    assert!(ok, "{rel}");
    let a = &rel["alvos"][format!("ws://127.0.0.1:{}/ws", s.porta)];
    assert_eq!((a["situacao"].as_str(), a["puladas"].as_u64(), a["repetidas"].as_u64()), (Some("ok"), Some(0), Some(0)), "{a}");
    assert!(a["recebidas"].as_u64().unwrap() >= 3);
    // saúde: as sondas voltaram, nenhuma perdida, sem atraso; e a sonda não fez nenhuma requisição
    // (o bloco `difusao` é calculado pela thread de difusão e copiado na rodada seguinte da saúde: até 2 s de atraso)
    let sa = esperar_saude(&s, Duration::from_secs(10), |v| v["sonda"]["concluidas"].as_u64().unwrap_or(0) >= 10
        && v["difusao"]["sondas_enviadas"].as_u64().unwrap_or(0) >= 6);
    let b = &sa["sonda"];
    assert_eq!((b["atrasada"].as_bool(), b["perdidas"].as_u64(), b["episodios_atraso"].as_u64()), (Some(false), Some(0), Some(0)), "{b}");
    assert_eq!(b["intervalo_s"], 0.2);
    assert_eq!(b["limite_s"], 5.0, "padrão: dobro do intervalo, no mínimo 5 s");
    assert!(b["latencias_ms"]["total"]["n"].as_u64().unwrap() >= 10);
    assert_eq!(sa["rede"]["requisicoes"], 0, "a sonda não fala com o TSE");
    assert_eq!(sa["secoes"]["aplicadas"], 0, "a sonda não é BU");
    assert!(sa["difusao"]["sondas_enviadas"].as_u64().unwrap() >= 6);
    assert!(sa["difusao"]["latencias_ms"]["sonda_envio"]["n"].as_u64().unwrap() >= 6);
    // REST
    let r: Value = serde_json::from_str(&rt.block_on(async { reqwest::get(format!("http://127.0.0.1:{}/api/v1/sonda", s.porta)).await.unwrap().text().await.unwrap() })).unwrap();
    assert!(!r["ultimas"].as_array().unwrap().is_empty());
    // o `estado` traz o atraso BU recebido → publicado (a telemetria da página): sem BU, n = 0
    let e: Value = serde_json::from_str(&rt.block_on(async { reqwest::get(format!("http://127.0.0.1:{}/api/v1/estado", s.porta)).await.unwrap().text().await.unwrap() })).unwrap();
    assert_eq!(e["latencia_bu_ms"]["n"], 0, "{e}");
    assert!(e["latencia_bu_ms"].get("p50").is_some() && e["latencia_bu_ms"].get("p99").is_some(), "{e}");
    // nada da sonda na publicação nem no diário
    for e in std::fs::read_dir(s.saida.join("publicacao")).unwrap().flatten() {
        assert!(!std::fs::read_to_string(e.path()).unwrap_or_default().contains("sonda"), "{}", e.path().display());
    }
    let diario = std::fs::read_to_string(s.saida.join("secoes.log")).unwrap();
    assert_eq!(diario.lines().count(), 1, "só o cabeçalho: {diario}");
    assert!(!std::fs::read_to_string(s.saida.join("alertas.csv")).unwrap().contains("sonda_"));
}

#[test]
fn sem_sondas_o_verificador_acusa() {
    let d = pasta("desligadas");
    let s = servico(&d, &["--sonda-intervalo", "0"]);
    let sa = esperar_saude(&s, Duration::from_secs(10), |v| v["estado"] == "acompanhando");
    assert!(sa.get("sonda").is_none(), "sondas desligadas: sem bloco na saúde");
    let (ok, rel) = verificador(&d, s.porta, &["--duracao", "2", "--limite", "1"]);
    assert!(!ok, "nenhuma sonda: o verificador tem de falhar");
    let a = &rel["alvos"][format!("ws://127.0.0.1:{}/ws", s.porta)];
    assert_eq!((a["situacao"].as_str(), a["recebidas"].as_u64()), (Some("atrasada"), Some(0)), "{a}");
    // alvo que não responde: desconectado
    let (ok, rel) = verificador(&d, 9, &["--duracao", "1.5", "--limite", "1"]);
    assert!(!ok);
    assert_eq!(rel["alvos"]["ws://127.0.0.1:9/ws"]["situacao"], "desconectado");
}

#[test]
fn motor_travado_dispara_o_alerta() {
    let d = pasta("travado");
    let s = servico(&d, &["--sonda-intervalo", "0.2", "--sonda-limite", "1"]);
    esperar_saude(&s, Duration::from_secs(10), |v| v["sonda"]["concluidas"].as_u64().unwrap_or(0) >= 3);
    let alertas = s.saida.join("alertas.csv");
    // trava a thread do motor (falha provocada): o processo segue vivo, as sondas param no canal
    std::fs::write(s.saida.join("TRAVAR_MOTOR"), "").unwrap();
    esperar_texto(&s.saida.join("servico.log"), "motor_travado", Duration::from_secs(5));
    let (ok, rel) = verificador(&d, s.porta, &["--duracao", "2.5", "--limite", "1"]);
    assert!(!ok, "motor travado: o verificador de fora não pode dar ok");
    assert_eq!(rel["alvos"][format!("ws://127.0.0.1:{}/ws", s.porta)]["situacao"], "atrasada", "{rel}");
    let txt = esperar_texto(&alertas, "sonda_atrasada", Duration::from_secs(5));
    // um alerta por episódio, mesmo com o motor parado por mais tempo
    std::thread::sleep(Duration::from_millis(1500));
    assert_eq!(std::fs::read_to_string(&alertas).unwrap().matches("sonda_atrasada").count(), 1, "{txt}");
    // destrava: as sondas acumuladas passam, o episódio fecha
    std::fs::remove_file(s.saida.join("TRAVAR_MOTOR")).unwrap();
    esperar_texto(&s.saida.join("servico.log"), "sonda_normalizada", Duration::from_secs(10));
    let sa = esperar_saude(&s, Duration::from_secs(10), |v| v["sonda"]["atrasada"] == false && v["sonda"]["pendentes"].as_u64() == Some(0));
    let b = &sa["sonda"];
    assert_eq!((b["episodios_atraso"].as_u64(), b["perdidas"].as_u64()), (Some(1), Some(0)), "FIFO: nada se perde, só atrasa: {b}");
    assert!(sa["alertas"]["sonda_atrasada"].as_u64() == Some(1), "{}", sa["alertas"]);
    let (ok, rel) = verificador(&d, s.porta, &["--duracao", "2", "--limite", "1", "--min-sondas", "3"]);
    assert!(ok, "depois de destravar: {rel}");
}

/// Federação de dois nós sem dados reais (`--sem-coleta`): as sondas de cada nó vão nos lotes, o outro as passa pelo
/// próprio motor (eco) e as mostra em `sonda.remotas`, e saem no WebSocket dele com a origem — o `uv-sonda` as separa.
#[test]
fn eco_entre_dois_nos() {
    let d = pasta("federacao");
    let porta = || std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
    let nos: Vec<Value> = ["a", "b"].iter().map(|id| serde_json::json!({"id": id, "orgao": format!("Órgão {id}"), "ip_saida": "127.0.0.1",
        "endereco": format!("127.0.0.1:{}", porta())})).collect();
    let fed = d.join("federacao.json");
    std::fs::write(&fed, serde_json::to_vec(&serde_json::json!({"versao": 1, "nos": nos})).unwrap()).unwrap();
    let no = |id: &str| {
        let di = d.join(id);
        std::fs::create_dir_all(&di).unwrap();
        servico(&di, &["--sonda-intervalo", "0.3", "--federacao", fed.to_str().unwrap(), "--no", id, "--armazem", di.join("federacao").to_str().unwrap()])
    };
    let (a, b) = (no("a"), no("b"));
    for (s, par) in [(&a, "b"), (&b, "a")] {
        let sa = esperar_saude(s, Duration::from_secs(60), |v| v["sonda"]["remotas"][par]["recebidas"].as_u64().unwrap_or(0) >= 3);
        let r = &sa["sonda"]["remotas"][par];
        assert_eq!((r["puladas"].as_u64(), r["repetidas"].as_u64(), r["reinicios"].as_u64()), (Some(0), Some(0), Some(0)), "{r}");
        assert!(r["federacao_ms"].as_f64().is_some() && r["total_ms"].as_f64().is_some(), "{r}");
        assert!(sa["federacao"]["sondas_publicadas"].as_u64().unwrap() > 0 && sa["federacao"]["sondas_recebidas"].as_u64().unwrap() >= 3, "{}", sa["federacao"]);
        assert_eq!(sa["federacao"]["publicados"], 0, "sonda não conta como resultado publicado");
        assert_eq!(sa["federacao"]["rejeitados"], 0);
        assert_eq!(sa["secoes"]["aplicadas"], 0);
        assert_eq!((sa["sonda"]["perdidas"].as_u64(), sa["sonda"]["atrasada"].as_bool()), (Some(0), Some(false)));
    }
    // no WebSocket de b: as próprias (ok) e as de a, separadas pelo verificador
    let (ok, rel) = verificador(&d, b.porta, &["--duracao", "3", "--limite", "2", "--min-sondas", "3"]);
    assert!(ok, "{rel}");
    let x = &rel["alvos"][format!("ws://127.0.0.1:{}/ws", b.porta)];
    assert!(x["remotas"]["a"]["recebidas"].as_u64().unwrap_or(0) >= 1, "eco de a no WebSocket de b: {x}");
}
