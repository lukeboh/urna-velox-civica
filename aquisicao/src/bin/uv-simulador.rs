//! `uv-simulador` — site de resultados do TSE simulado em 127.0.0.1, com relógio acelerado e injeção de falhas
//! (módulo `uv_aquisicao::simulador`). Para provar o `uv-servico` sem baixar BUs do TSE.

use clap::Parser;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use uv_aquisicao::simulador::{servir, Config, Falhas, Sim};

#[derive(Parser)]
#[command(name = "uv-simulador", version, about = "Urna Velox — site de resultados simulado (127.0.0.1) para ensaiar o uv-servico")]
struct Cli {
    /// Pasta dos pacotes .uvpk (os BUs servidos)
    #[arg(long)]
    dados: PathBuf,
    /// Pasta dos cs.json originais (<uf>-p<pleito>-cs.json); padrão: <dados>/config
    #[arg(long)]
    cs: Option<PathBuf>,
    /// CSVs de métricas da coleta, por vírgula (disponibilização, recebimento, anuladas, não instaladas)
    #[arg(long)]
    metricas: String,
    /// Pasta dos -u.json oficiais (servidos depois do último BU)
    #[arg(long)]
    oficial: Option<PathBuf>,
    #[arg(long)]
    pleito: u32,
    #[arg(long, default_value = "ele2026")]
    ciclo: String,
    #[arg(long, default_value = "oficial")]
    ambiente: String,
    /// Só estas UFs (por vírgula)
    #[arg(long)]
    ufs: Option<String>,
    /// Porta em 127.0.0.1 (0 = qualquer livre; a escolhida sai na primeira linha: porta=N)
    #[arg(long, default_value_t = 0)]
    porta: u16,
    /// Arquivo onde gravar a porta escolhida
    #[arg(long)]
    porta_arquivo: Option<PathBuf>,
    /// Aceleração do relógio (10 = 10 s simulados por segundo real)
    #[arg(long, default_value_t = 10.0)]
    acelerar: f64,
    /// Início do relógio simulado ("dd/mm/aaaa hh:mm:ss"); padrão: 1 min antes da primeira disponibilização
    #[arg(long)]
    inicio: Option<String>,
    /// Disponibilizações depois deste horário entram nele ("dd/mm/aaaa hh:mm:ss")
    #[arg(long)]
    corte: Option<String>,
    /// Horário de cada seção: "disponivel" (disponivel_cs das métricas, ou o recebimento quando ela não é da noite) ou
    /// "recebido" (recebido_tse; para cs.json coletados depois da noite, como o de 2026)
    #[arg(long, default_value = "disponivel", value_parser = ["disponivel", "recebido"])]
    horario: String,
    /// Registro de cada requisição (CSV)
    #[arg(long, default_value = "simulador.log")]
    log: PathBuf,
    /// Rajada de HTTP 429: "hh:mm:ss+N" (N segundos simulados); repetível
    #[arg(long)]
    rajada_429: Vec<String>,
    /// Latência aleatória "a-b" ms
    #[arg(long, default_value = "")]
    latencia_ms: String,
    /// Probabilidade de HTTP 503 por requisição
    #[arg(long, default_value_t = 0.0)]
    erros_5xx: f64,
    #[arg(long, default_value_t = 2026)]
    semente: u64,
    /// Reenvio com o mesmo BU (hash novo): "uf:município/zona/seção@hh:mm:ss"; repetível
    #[arg(long)]
    reenviar: Vec<String>,
    /// Anulação depois de publicada: "uf:município/zona/seção@hh:mm:ss"; repetível
    #[arg(long)]
    anular: Vec<String>,
    /// Queda total do servidor: "hh:mm:ss+N" (N segundos reais); repetível
    #[arg(long)]
    queda: Vec<String>,
    /// Limite por IP de origem em req/s da noite (multiplicado pela aceleração), como a borda do TSE por órgão: acima
    /// dele, HTTP 429 e caixa de penalidade do IP por 600 s simulados (2.0, federação). 0 = sem limite
    #[arg(long, default_value_t = 0.0)]
    limite_ip_rps: f64,
    /// TSE simulado (etapa 7): candidatos.json do pleito; os -u.json passam a ser servidos durante a noite, progressivos,
    /// com o atraso da divulgação oficial, e iguais ao oficial depois da última divulgação
    #[arg(long, value_name = "CANDIDATOS")]
    tse_simulado: Option<PathBuf>,
    /// TSE simulado: segundos da noite entre duas gerações dos arquivos
    #[arg(long, default_value_t = 15)]
    tse_passo: i64,
}

fn main() -> anyhow::Result<()> {
    let a = Cli::parse();
    let cfg = Config {
        cs: a.cs.clone().unwrap_or_else(|| a.dados.join("config")),
        dados: a.dados,
        metricas: a.metricas.split(',').filter(|s| !s.is_empty()).map(PathBuf::from).collect(),
        oficial: a.oficial,
        pleito: a.pleito,
        ciclo: a.ciclo,
        ambiente: a.ambiente,
        ufs: a.ufs.map(|u| u.split(',').map(|x| x.trim().to_lowercase()).filter(|x| !x.is_empty()).collect()),
        porta: a.porta,
        acelerar: a.acelerar,
        inicio: a.inicio,
        corte: a.corte,
        horario_recebido: a.horario == "recebido",
        log: a.log.clone(),
        falhas: Falhas { rajadas_429: a.rajada_429, latencia_ms: a.latencia_ms, erros_5xx: a.erros_5xx, semente: a.semente,
            reenvios: a.reenviar, anulacoes: a.anular, quedas: a.queda },
        limite_ip_rps: a.limite_ip_rps,
        tse: a.tse_simulado.map(|c| { let mut o = uv_aquisicao::tse_simulado::Opcoes::padrao(c); o.passo_s = a.tse_passo.max(1); o.semente = a.semente; o }),
    };
    let sim = Arc::new(Sim::carregar(&cfg)?);
    eprintln!("uv-simulador {}: {}", env!("CARGO_PKG_VERSION"), sim.resumo_carga);
    let resumo = a.log.with_file_name("simulador_resumo.json");
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;
    let porta_arquivo = a.porta_arquivo;
    let s2 = sim.clone();
    let r2 = resumo.clone();
    rt.block_on(async move {
        let fim = async {
            #[cfg(unix)]
            {
                let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).expect("SIGTERM");
                tokio::select! { _ = tokio::signal::ctrl_c() => {}, _ = term.recv() => {} }
            }
            #[cfg(not(unix))]
            { let _ = tokio::signal::ctrl_c().await; }
        };
        tokio::select! {
            r = servir(sim, cfg.porta, resumo, move |p| {
                println!("porta={p}");
                let _ = std::io::stdout().flush();
                if let Some(f) = &porta_arquivo { let _ = std::fs::write(f, format!("{p}\n")); }
            }) => r,
            _ = fim => { s2.gravar_resumo(&r2); eprintln!("uv-simulador: encerrado"); Ok(()) }
        }
    })
}
