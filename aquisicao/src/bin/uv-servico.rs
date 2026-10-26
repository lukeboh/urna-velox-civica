//! `uv-servico` — o serviço da noite da eleição (módulo `uv_aquisicao::servico`): acompanha o site, coleta dentro dos
//! limites (≤ 100 req/s e pausa ≥ 10 min depois de um 429 contra o TSE), totaliza em fluxo, calcula o
//! "matematicamente", publica, retoma depois de uma queda e confere com os oficiais no fim. Ver docs/OPERACAO.md.

use clap::Parser;
use std::path::PathBuf;
use uv_aquisicao::servico::{executar, Config};
use uv_aquisicao::tse::Alvo;

#[derive(Parser)]
#[command(name = "uv-servico", version, about = "Urna Velox — serviço da noite: acompanha, coleta, totaliza, publica e confere sozinho")]
struct Cli {
    /// Código do pleito (3221 = 2º turno de 2026)
    #[arg(long)]
    pleito: u32,
    /// candidatos.json do pleito (uv-coletor preparar)
    #[arg(long)]
    candidatos: PathBuf,
    /// Pasta dos pacotes .uvpk do serviço (na retomada, os que já estão aqui são relidos)
    #[arg(long)]
    dados: PathBuf,
    /// Pasta de saída: publicacao/, secoes.log (diário), eventos.csv, saude.json, servico.log, alertas.csv, conferencia.*
    #[arg(long)]
    saida: PathBuf,
    /// Servidor de resultados (o site simulado: http://127.0.0.1:<porta>)
    #[arg(long, default_value = "https://resultados.tse.jus.br")]
    base: String,
    /// Ambiente: oficial | simulado
    #[arg(long, default_value = "oficial")]
    ambiente: String,
    /// Ciclo (diretório), ex.: ele2026
    #[arg(long, default_value = "ele2026")]
    ciclo: String,
    /// Trabalhando (seções chegando ou na fila): segundos entre releituras dos cs.json de todas as UFs. Em hora do site:
    /// com --aceleracao-local é dividido pela aceleração (não precisa ajustar à mão)
    #[arg(long, default_value_t = 30.0)]
    intervalo_cs: f64,
    /// Ocioso (fila vazia e nada novo no último ciclo): segundos entre ciclos; contra o TSE cada ciclo lê 4 UFs em
    /// rodízio (1 s = 4 req/s, 2 s = 2 req/s; mínimo 1 s), contra o simulador todas. Também em hora do site
    #[arg(long, default_value_t = 1.0)]
    intervalo_cs_ocioso: f64,
    /// Requisições por segundo (contra o TSE nunca passa de 100)
    #[arg(long, default_value_t = 95.0)]
    rps: f64,
    /// Só com servidor local (127.0.0.1/localhost): aceleração do relógio do site simulado; multiplica a taxa e divide a
    /// pausa de 429 e as esperas. Recusada contra qualquer outro servidor
    #[arg(long, default_value_t = 1.0)]
    aceleracao_local: f64,
    /// Só com servidor local (o uv-simulador) e --http: controle do replay — velocidade (pausa, 1 a 300×) e posição na
    /// noite (avançar e voltar), pela página ou por /api/v1/replay. --aceleracao-local passa a ser a velocidade inicial.
    /// IR A UM INSTANTE JÁ VISTO APAGA --saida (o serviço recomeça e retotaliza do disco): use pastas descartáveis
    #[arg(long)]
    controle_replay: bool,
    /// Seções coletadas ao mesmo tempo
    #[arg(long, default_value_t = 64)]
    concorrencia: usize,
    /// Aceita um candidatos.json com "incompleto" (com alerta)
    #[arg(long)]
    aceitar_incompleto: bool,
    /// Só estas UFs (por vírgula): acompanhamento, fim da apuração e conferência
    #[arg(long)]
    ufs: Option<String>,
    /// Conferência com os oficiais a cada N minutos (0 = só no fim ou com o arquivo <saida>/CONFERIR)
    #[arg(long, default_value_t = 0.0)]
    conferir_a_cada: f64,
    /// Continua acompanhando depois da conferência final (padrão: sai com código 0)
    #[arg(long)]
    continuar: bool,
    /// Não sonda as seções sem disponibilização (não instaladas) no fim
    #[arg(long)]
    sem_sondagem: bool,
    /// Intervalo mínimo entre publicações (ms; 0 = a cada lote)
    #[arg(long, default_value_t = 0)]
    cadencia_ms: u64,
    #[arg(long, default_value_t = 30)]
    timeout: u64,
    /// Painel no terminal com a totalização ao vivo de uma abrangência: prefixo do arquivo de publicação (sem valor =
    /// br-c0001, Presidente no Brasil; ex.: sp-c0003 = Governador de SP). Até 4 quadros/s; redirecionado, uma linha por
    /// quadro. As mensagens do serviço continuam no stderr (use 2> arquivo para não misturar)
    #[arg(long, num_args = 0..=1, default_missing_value = "br-c0001", value_name = "PREFIXO")]
    painel: Option<String>,
    /// Liga a difusão em tempo real (WebSocket /ws e REST /api/v1/, docs/PROTOCOLO_WS.md) neste endereço, ex.:
    /// 127.0.0.1:8080 (porta 0 = qualquer livre, gravada em <saida>/http_porta). Sem TLS: para expor, use um proxy reverso
    #[arg(long, value_name = "END:PORTA")]
    http: Option<String>,
    /// Conexões WebSocket simultâneas (acima disso, 503 no handshake)
    #[arg(long, default_value_t = 7500)]
    max_clientes: usize,
    /// Threads do runtime da camada HTTP (separadas da rede de coleta e do motor)
    #[arg(long, default_value_t = 4)]
    http_threads: usize,
    /// Modo federado (2.0, ADR-028): federacao.json (um nó por órgão, IP de saída declarado). Exige --no e --armazem
    #[arg(long, requires_all = ["no", "armazem"])]
    federacao: Option<PathBuf>,
    /// Id deste nó no federacao.json
    #[arg(long)]
    no: Option<String>,
    /// Diretório local da federação neste nó (os próprios lotes e as cópias dos pares; não é compartilhado)
    #[arg(long)]
    armazem: Option<PathBuf>,
    /// Onde servir a sincronização aos pares (host:porta); padrão: o "endereco" deste nó no federacao.json
    #[arg(long)]
    federacao_escuta: Option<std::net::SocketAddr>,
    /// URL do eco de IP (devolve o IP de saída que a CDN vê); padrão contra o simulador: <base>/eco-ip
    #[arg(long)]
    eco_ip: Option<String>,
    /// Endereço local de saída (no simulador, cada nó sai por um 127.0.0.x próprio)
    #[arg(long)]
    ip_local: Option<std::net::IpAddr>,
    /// CSVs de métricas (vírgula) com o `recebido_tse` de cada seção: a fila de coleta segue a ordem de recebimento
    /// dos BUs no TSE (sem isto, a ordem de disponibilização dos `cs.json`)
    #[arg(long, value_delimiter = ',')]
    ordem: Vec<PathBuf>,
    /// Comparação com o TSE (etapa 7, só com --http): segundos (hora do site) entre duas buscas do -u.json de um
    /// tópico inscrito por algum cliente (tse:<tópico>); de 30 a 60 (combinado com o Luciano, 16/10/2026)
    #[arg(long, default_value_t = 30.0)]
    tse_intervalo: f64,
    /// Comparação com o TSE: tópicos buscados ao mesmo tempo (acima disso, o novo espera vaga)
    #[arg(long, default_value_t = 200)]
    tse_max_topicos: usize,
    /// Desliga a comparação com o TSE (nenhum tópico tse:, nenhuma busca de -u.json fora a conferência)
    #[arg(long)]
    sem_tse: bool,
    /// Sem coleta: não lê os cs.json nem pede seção nenhuma — só a publicação da partida, a difusão (--http) e a
    /// comparação com o TSE (ensaio da comparação contra o site real sem coletar uma eleição inteira). Sem sondagem
    #[arg(long)]
    sem_coleta: bool,
    /// Previsão do "matematicamente" e do fim da apuração (etapa 8): segundos do relógio do serviço (o do site; no
    /// replay, o simulado) entre duas rodadas; 0 desliga
    #[arg(long, default_value_t = 30)]
    previsao_intervalo: i64,
    /// Threads da rodada de previsão (fora da thread do motor)
    #[arg(long, default_value_t = 4)]
    previsao_threads: usize,
    /// Simulação com dados reais (ciclo diário): rótulo mostrado no selo do topo da página e na saude.json, ex.:
    /// "1º turno de 2024 · cadência real"
    #[arg(long)]
    simulacao: Option<String>,
    /// Caminhos do site de resultados (rotas_tse.json: modelos com {base}, {uf}, {hash}... e, opcional,
    /// base_arquivos para buscar os BUs de outro endereço). Sem ele, os caminhos de sempre. Ver docs/PAINEL.md
    #[arg(long)]
    rotas: Option<PathBuf>,
    /// Como o nó chega ao servidor: borda (a borda pública do TSE: teto de 100 req/s por órgão, sempre) ou interno
    /// (servidor espelho dos arquivos, fora da borda pública: teto de --teto-interno, ou nenhum). Interno é recusado para resultados/cdn.tse.jus.br
    #[arg(long, default_value = "borda")]
    acesso: String,
    /// Teto de req/s no modo espelho, `--acesso interno` (0 ou ausente = sem limite)
    #[arg(long)]
    teto_interno: Option<f64>,
}

fn main() -> anyhow::Result<()> {
    let a = Cli::parse();
    let mut cfg = Config {
        alvo: {
            let mut alvo = Alvo::novo(&a.base, &a.ambiente, &a.ciclo, a.pleito);
            if let Some(r) = &a.rotas {
                let rotas = uv_aquisicao::tse::Rotas::ler(r)?;
                rotas.validar(&alvo.base)?;
                if !rotas.padrao() { eprintln!("uv-servico: caminhos do site de {} (base dos arquivos: {})", r.display(), rotas.base_arquivos.as_deref().unwrap_or("a mesma")); }
                alvo.rotas = std::sync::Arc::new(rotas);
            }
            alvo
        },
        acesso: uv_aquisicao::coletor::Acesso::de_args(&a.acesso, a.teto_interno)?,
        candidatos: a.candidatos,
        dados: a.dados,
        saida: a.saida,
        intervalo_cs: a.intervalo_cs,
        intervalo_cs_ocioso: a.intervalo_cs_ocioso,
        rps: a.rps,
        aceleracao_local: a.aceleracao_local,
        controle_replay: a.controle_replay,
        concorrencia: a.concorrencia,
        aceitar_incompleto: a.aceitar_incompleto,
        ufs: a.ufs.map(|u| u.split(',').map(|x| x.trim().to_lowercase()).filter(|x| !x.is_empty()).collect()),
        conferir_a_cada_min: a.conferir_a_cada,
        continuar: a.continuar,
        sondar: !a.sem_sondagem && !a.sem_coleta,
        cadencia_ms: a.cadencia_ms,
        timeout_s: a.timeout,
        painel: a.painel,
        http: a.http.map(|e| {
            let mut o = uv_aquisicao::difusao::Opcoes::padrao(&e);
            o.max_clientes = a.max_clientes;
            o.threads = a.http_threads;
            o
        }),
        federacao: a.federacao.map(|f| uv_aquisicao::federacao::no::Opcoes {
            federacao: f, no: a.no.unwrap_or_default(), armazem: a.armazem.unwrap_or_default(), eco_ip: a.eco_ip,
            escuta: a.federacao_escuta,
        }),
        ip_local: a.ip_local,
        ordem: a.ordem,
        tse: (!a.sem_tse).then_some(uv_aquisicao::tse_resultados::Opcoes { intervalo_s: a.tse_intervalo, max_topicos: a.tse_max_topicos }),
        sem_coleta: a.sem_coleta,
        previsao: uv_aquisicao::previsao::Opcoes { intervalo_s: a.previsao_intervalo.max(0), threads: a.previsao_threads.max(1) },
        simulacao: a.simulacao.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()),
    };
    if let Some(t) = &cfg.tse { t.validar()?; }
    // com --controle-replay, ir a um instante já visto para o serviço e pede um reinício: o diário da sessão vai para o
    // diário-mestre (em --dados, com os pacotes), a saída é apagada e a sessão nova retotaliza do disco até o instante
    // (uv_aquisicao::controle_replay)
    loop {
        executar(cfg.clone())?;
        match uv_aquisicao::controle_replay::reinicio_pedido() {
            Some((alvo, x, depois)) if cfg.controle_replay => {
                use uv_aquisicao::controle_replay as cr;
                let t0 = std::time::Instant::now();
                let n = cr::guardar_diario(&cfg.saida.join("secoes.log"), &cfg.dados.join(cr::DIARIO_MESTRE))?;
                eprintln!("uv-servico: diário-mestre do replay com {n} BUs ({:.1} s)", t0.elapsed().as_secs_f64());
                if let Err(e) = std::fs::remove_dir_all(&cfg.saida) { if e.kind() != std::io::ErrorKind::NotFound { anyhow::bail!("reinício: apagar {}: {e}", cfg.saida.display()); } }
                // a velocidade escolhida (0 = pausa) vale depois da retotalização
                cfg.aceleracao_local = x;
                cr::definir_reconstrucao(alvo, depois);
                eprintln!("uv-servico: reinício para retotalizar até {} ({n} BUs no diário-mestre)", uv_aquisicao::texto_hora(alvo));
            }
            _ => return Ok(()),
        }
    }
}
