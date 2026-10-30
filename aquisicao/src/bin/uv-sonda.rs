//! `uv-sonda` — verificador de fora das sondas do `uv-servico` (padrão *Test Message*, `uv_aquisicao::sonda`,
//! `docs/arquitetura/SONDA.md`).
//!
//! Conecta no WebSocket de um ou mais serviços (ou da borda que os publica), pede `{"op":"sonda"}` e confere se as
//! mensagens de teste continuam chegando: com o serviço vivo e o caminho dos BUs parado (motor travado, gravação ou
//! difusão engasgadas, proxy no meio do caminho), as sondas param de chegar e o alvo fica `atrasada` — o que a saúde
//! do processo não mostra. Grava o relatório em `--saida` (o painel dos nós lê `<saida do serviço>/sonda.json`) e,
//! com `--duracao`, sai com código 1 se algum alvo não estiver `ok` no fim (para roteiros e ensaios).
//!
//! Não fala com o TSE: só com os serviços indicados.

use clap::Parser;
use futures::{SinkExt, StreamExt};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio_tungstenite::tungstenite::Message;
use uv_aquisicao::difusao::agora_us;
use uv_aquisicao::sonda::{Situacao, Verificador};

#[derive(Parser)]
#[command(name = "uv-sonda", version, about = "Urna Velox — verificador das sondas (Test Message): o caminho dos BUs está andando?")]
struct Cli {
    /// WebSocket do serviço (repetível): ws://127.0.0.1:8080/ws, wss://dominio/ws ou só 127.0.0.1:8080
    #[arg(long = "url", required = true)]
    urls: Vec<String>,
    /// Sem sonda nova por mais que isto (segundos), o alvo fica "atrasada". Padrão: 2,5 × os 30 s do serviço
    #[arg(long, default_value_t = 75.0)]
    limite: f64,
    /// Relatório JSON, regravado a cada segundo (gravação atômica)
    #[arg(long)]
    saida: Option<PathBuf>,
    /// Segundos até sair (0 = para sempre). Com duração, o código de saída diz se todos os alvos terminaram "ok"
    #[arg(long, default_value_t = 0.0)]
    duracao: f64,
    /// Com --duracao: sondas novas que cada alvo tem de entregar para o sucesso
    #[arg(long, default_value_t = 1)]
    min_sondas: u64,
}

/// "127.0.0.1:8080" → "ws://127.0.0.1:8080/ws"; URLs ws:// e wss:// ficam como estão.
fn url_ws(u: &str) -> String {
    if u.starts_with("ws://") || u.starts_with("wss://") { u.to_string() } else { format!("ws://{}/ws", u.trim_end_matches('/')) }
}

fn gravar(p: &std::path::Path, texto: &str) -> std::io::Result<()> {
    let tmp = p.with_extension("json.tmp");
    std::fs::write(&tmp, texto)?;
    std::fs::rename(&tmp, p)
}

/// Uma conexão do começo ao fim: devolve o motivo da queda.
async fn conexao(url: &str, v: &Mutex<Verificador>) -> String {
    let ws = match tokio::time::timeout(Duration::from_secs(10), tokio_tungstenite::connect_async(url)).await {
        Ok(Ok((ws, _))) => ws,
        Ok(Err(e)) => return format!("falha_conexao: {e}"),
        Err(_) => return "falha_conexao: tempo esgotado".into(),
    };
    let (mut tx, mut rx) = ws.split();
    if tx.send(Message::text(r#"{"op":"sonda"}"#)).await.is_err() { return "erro_envio".into(); }
    v.lock().unwrap().conectou(url, agora_us());
    while let Some(m) = rx.next().await {
        match m {
            Ok(Message::Text(t)) => {
                let Ok(j) = serde_json::from_str::<serde_json::Value>(t.as_str()) else { continue };
                if j["tipo"] == "aviso" && j["motivo"] == "desligamento" { return "desligamento".into(); }
                v.lock().unwrap().receber(url, &j, agora_us());
            }
            Ok(Message::Close(c)) => return c.map(|c| format!("close {} {}", u16::from(c.code), c.reason)).unwrap_or("close".into()),
            Ok(_) => {}
            Err(e) => return format!("erro_leitura: {e}"),
        }
    }
    "fim".into()
}

/// Mantém a conexão com o alvo, com espera crescente entre as tentativas (1 s, 2 s, 4 s… até 30 s).
async fn acompanhar(url: String, v: Arc<Mutex<Verificador>>) {
    let mut espera = 1u64;
    loop {
        let t = Instant::now();
        let motivo = conexao(&url, &v).await;
        v.lock().unwrap().caiu(&url, &motivo);
        eprintln!("uv-sonda: {url}: {motivo}");
        if t.elapsed() > Duration::from_secs(30) { espera = 1; }
        tokio::time::sleep(Duration::from_secs(espera)).await;
        espera = (espera * 2).min(30);
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let a = Cli::parse();
    anyhow::ensure!(a.limite > 0.0 && a.limite.is_finite(), "--limite tem de ser positivo");
    let urls: Vec<String> = a.urls.iter().map(|u| url_ws(u)).collect();
    let v = Arc::new(Mutex::new(Verificador::novo(a.limite, &urls)));
    for u in &urls { tokio::spawn(acompanhar(u.clone(), v.clone())); }
    let t0 = Instant::now();
    let mut ant: Vec<Situacao> = vec![];
    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
        let agora = agora_us();
        let (rel, sit) = { let g = v.lock().unwrap(); (g.json(agora), urls.iter().map(|u| g.situacao(u, agora)).collect::<Vec<_>>()) };
        if sit != ant {
            for (u, s) in urls.iter().zip(&sit) { eprintln!("uv-sonda: {u}: {}", s.texto()); }
            ant = sit;
        }
        if let Some(p) = &a.saida {
            if let Err(e) = gravar(p, &(serde_json::to_string_pretty(&rel)? + "\n")) { eprintln!("uv-sonda: {}: {e}", p.display()); }
        }
        if a.duracao > 0.0 && t0.elapsed().as_secs_f64() >= a.duracao {
            let g = v.lock().unwrap();
            let ok = g.tudo_ok(agora) && urls.iter().all(|u| g.recebidas(u) >= a.min_sondas);
            println!("{}", serde_json::to_string(&rel)?);
            std::process::exit(if ok { 0 } else { 1 });
        }
    }
}
