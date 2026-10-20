//! `uv-painel` — painel de monitoramento e gerenciamento dos nós (módulo `uv_aquisicao::painel`, `docs/PAINEL.md`).
//!
//! Os segredos vêm do ambiente (no servidor, `/etc/urna-velox/painel.env`, lido pelo systemd), nunca da linha de
//! comando: `UV_PAINEL_GOOGLE_ID`, `UV_PAINEL_GOOGLE_SEGREDO`, `UV_PAINEL_SEGREDO` (o mesmo nos dois nós) e
//! `UV_PAINEL_EMAILS` (quem pode entrar).

use anyhow::Context;
use clap::Parser;
use std::path::PathBuf;
use uv_aquisicao::painel::{seguranca, servir, Config, Painel};

#[derive(Parser)]
#[command(name = "uv-painel", version, about = "Urna Velox — painel dos nós: desempenho, federação e configuração, com login do Google")]
struct Cli {
    /// Onde escutar (o Caddy encaminha /painel/* para cá; o painel do outro nó chega pela rede privada)
    #[arg(long, default_value = "127.0.0.1:8095")]
    escuta: std::net::SocketAddr,
    /// Id deste nó (o mesmo do federacao.json)
    #[arg(long)]
    no: String,
    /// Pasta de saída do uv-servico (saude.json)
    #[arg(long, default_value = "/srv/urna-velox/saida")]
    saida: PathBuf,
    #[arg(long, default_value = "/etc/urna-velox/no.env")]
    no_env: PathBuf,
    #[arg(long, default_value = "/etc/urna-velox/federacao.json")]
    federacao: PathBuf,
    /// Painel de outro nó (repetível): http://<ip privado>:8095 (a 8090 é da playlist de simulados)
    #[arg(long = "par")]
    pares: Vec<String>,
    /// Endereço público do painel, sem barra no fim (https://dominio). O Google devolve o login em
    /// <url>/painel/oauth/retorno: esse endereço tem de estar cadastrado no cliente OAuth
    /// (ou UV_PAINEL_URL no ambiente)
    #[arg(long)]
    url_publica: Option<String>,
    /// Pasta do painel: auditoria.jsonl e arquivos à espera do uv-painel-acao
    #[arg(long, default_value = "/srv/urna-velox/painel")]
    estado: PathBuf,
    /// Comando do ajudante de ações (separado por espaços). Vazio = modo local: grava o no.env direto e não mexe em
    /// serviço nenhum (testes na estação)
    #[arg(long, default_value = "sudo -n /opt/urna-velox/bin/uv-painel-acao")]
    acao: String,
    /// Segundos entre amostras
    #[arg(long, default_value_t = 5.0)]
    intervalo: f64,
    /// Horas de histórico em memória
    #[arg(long, default_value_t = 2.0)]
    horas: f64,
    /// Unidades systemd mostradas na página (vírgula)
    #[arg(long, default_value = "urna-velox.service,uv-ciclo-no.service,uv-playlist.service,uv-painel.service,caddy.service,urna-velox-atualizador.timer,chrony.service")]
    servicos: String,
    #[arg(long, hide = true, default_value = seguranca::GOOGLE_AUTORIZACAO)]
    google_autorizacao: String,
    #[arg(long, hide = true, default_value = seguranca::GOOGLE_TOKEN)]
    google_token: String,
}

fn var(k: &str) -> anyhow::Result<String> {
    std::env::var(k).map(|v| v.trim().to_string()).with_context(|| format!("falta a variável de ambiente {k} (ver docs/PAINEL.md)"))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // usado pelo uv-painel-acao (root) antes de instalar um rotas_tse.json: uv-painel validar-rotas <arquivo> <base>
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("validar-rotas") {
        anyhow::ensure!(args.len() == 4, "uso: uv-painel validar-rotas <rotas_tse.json> <base>");
        let r = uv_aquisicao::tse::Rotas::ler(std::path::Path::new(&args[2]))?;
        r.validar(&args[3])?;
        println!("rotas válidas");
        return Ok(());
    }
    let c = Cli::parse();
    let cfg = Config {
        escuta: c.escuta, no_id: c.no, saida: c.saida, no_env: c.no_env,
        federacao: Some(c.federacao), pares: c.pares.iter().map(|p| p.trim_end_matches('/').to_string()).filter(|p| !p.is_empty()).collect(),
        url_publica: match c.url_publica { Some(u) => u, None => var("UV_PAINEL_URL")? }.trim_end_matches('/').to_string(),
        google_id: var("UV_PAINEL_GOOGLE_ID")?, google_segredo: var("UV_PAINEL_GOOGLE_SEGREDO")?,
        google_autorizacao: c.google_autorizacao, google_token: c.google_token,
        emails: seguranca::lista_emails(&var("UV_PAINEL_EMAILS")?), segredo: var("UV_PAINEL_SEGREDO")?,
        estado: c.estado, acao: c.acao.split_whitespace().map(String::from).collect(),
        intervalo_s: c.intervalo.max(1.0), servicos: c.servicos.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect(),
        capacidade: ((c.horas.max(0.1) * 3600.0) / c.intervalo.max(1.0)).ceil() as usize,
    };
    servir(Painel::novo(cfg)?).await
}
