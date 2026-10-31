#![allow(dead_code)]
//! uv-coletor — fase de aquisição do Urna Velox.
//!
//! Recebe/recupera os Boletins de Urna (BUs) brutos publicados pelo TSE e os entrega ao
//! núcleo de totalização, em pacotes `.uvpk` e/ou diretamente ao totalizador em memória.
//! Veja `aquisicao/README.md` e `docs/arquitetura/AQUISICAO.md`.

use uv_aquisicao::{coletor, preparo, replay, tse, zip};

use clap::{Args, Parser, Subcommand};
use coletor::{ConfigRede, Medida, Recebido, Rede};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering::Relaxed;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use urna_velox::catalogo::{self, Catalogo};
use urna_velox::estado::{self, Agregador, Estado};
use urna_velox::{bu, pacote};

#[derive(Parser)]
#[command(name = "uv-coletor", version, about = "Urna Velox — coleta de BUs do TSE e entrega ao núcleo de totalização")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Args, Clone)]
struct AlvoArgs {
    /// Servidor de resultados
    #[arg(long, default_value = "https://resultados.tse.jus.br")]
    base: String,
    /// Ambiente: oficial | simulado
    #[arg(long, default_value = "oficial")]
    ambiente: String,
    /// Ciclo eleitoral (diretório), ex.: ele2026
    #[arg(long, default_value = "ele2026")]
    ciclo: String,
    /// Código do pleito (um BU traz todas as eleições do pleito). 3220 = 1º turno de 2026
    #[arg(long, default_value_t = 3220)]
    pleito: u32,
}
impl AlvoArgs {
    fn alvo(&self) -> tse::Alvo { tse::Alvo::novo(&self.base, &self.ambiente, &self.ciclo, self.pleito) }
}

#[derive(Args, Clone)]
struct RedeArgs {
    /// Seções (ou BUs) em andamento ao mesmo tempo
    #[arg(long, short = 'c', default_value_t = 64)]
    concorrencia: usize,
    /// Clientes HTTP independentes (≈ conexões TCP com HTTP/2)
    #[arg(long, default_value_t = 1)]
    conexoes: usize,
    /// Teto de requisições por segundo. Nunca passa de 100 (regra do projeto: acima disso a borda do TSE responde HTTP 429; valores maiores são reduzidos)
    #[arg(long, default_value_t = 100.0)]
    rps: f64,
    /// Controle de taxa adaptativo (AIMD): parte de --rps-inicial, corta pela metade a cada HTTP 429
    /// e sobe devagar até --rps
    #[arg(long)]
    adaptativo: bool,
    #[arg(long, default_value_t = 40.0)]
    rps_inicial: f64,
    /// Força HTTP/1.1 (padrão: HTTP/2 negociado por ALPN)
    #[arg(long)]
    http1: bool,
    #[arg(long, default_value_t = 6)]
    tentativas: u32,
    #[arg(long, default_value_t = 30)]
    timeout: u64,
    /// Disjuntor: para a coleta inteira ao somar N respostas HTTP 429 (0 = desliga)
    #[arg(long, default_value_t = 20)]
    max_429: u64,
}
impl RedeArgs {
    fn cfg(&self) -> ConfigRede {
        ConfigRede { concorrencia: self.concorrencia, conexoes: self.conexoes, rps: self.rps, adaptativo: self.adaptativo, rps_inicial: self.rps_inicial, http1: self.http1, tentativas: self.tentativas, timeout_s: self.timeout, max_429: self.max_429, teto: 0.0, pausa_429_s: 0.0, ip_local: None }
    }
}

/// Seções a pular na retomada: as que, numa coleta anterior, responderam 404 no aux.json (seção sem
/// arquivos publicados) ou não tinham BU por não ter sido instalada. Pedir de novo milhares de 404
/// seguidos faz a borda do CDN responder 429 mesmo abaixo de 100 req/s (medido em 12/10/2026).
fn secoes_sem_bu(arqs: &str) -> anyhow::Result<std::collections::HashSet<([u8; 2], u64)>> {
    let mut out = std::collections::HashSet::new();
    for p in arqs.split(',').filter(|s| !s.is_empty()) {
        let txt = std::fs::read_to_string(p)?;
        let mut l = txt.lines();
        let cab: Vec<&str> = l.next().unwrap_or("").split(',').collect();
        let c = |n: &str| cab.iter().position(|x| *x == n).ok_or_else(|| anyhow::anyhow!("{p}: sem coluna {n}"));
        let (cu, cm, cz, cs, ce) = (c("uf")?, c("municipio")?, c("zona")?, c("secao")?, c("erro")?);
        for x in l {
            let f: Vec<&str> = x.split(',').collect();
            if f.len() <= ce { continue; }
            let e = f[ce];
            if !(e == "aux HTTP 404" || e.starts_with("sem BU (situação Nao instalada")) { continue; }
            let uf: [u8; 2] = match f[cu].as_bytes() { [a, b] => [*a, *b], _ => continue };
            let (Ok(m), Ok(z), Ok(s)) = (f[cm].parse::<u32>(), f[cz].parse::<u16>(), f[cs].parse::<u16>()) else { continue };
            out.insert((uf, tse::chave(m, z, s)));
        }
    }
    Ok(out)
}

#[derive(Subcommand)]
enum Cmd {
    /// Lista pleitos e eleições publicados (ele-c.json)
    Eleicoes {
        #[command(flatten)]
        alvo: AlvoArgs,
    },
    /// Multipleito (1.1): descobre os pleitos no ele-c.json e os classifica (agendado, em curso, encerrado) no
    /// pleitos.json lido pela página (uv-servico --pleitos), pela playlist e pelo ciclo. Uma requisição ao TSE
    Pleitos {
        #[command(flatten)]
        alvo: AlvoArgs,
        /// Onde gravar o pleitos.json (sem isto, só a tabela)
        #[arg(long)]
        saida: Option<PathBuf>,
        /// Lê este ele-c.json em vez de baixá-lo (sem rede)
        #[arg(long)]
        ele_c: Option<PathBuf>,
        /// Guarda aqui uma cópia do ele-c.json baixado
        #[arg(long)]
        guardar: Option<PathBuf>,
        /// Pasta dos acervos de replay (<acervo>/<pleito>/.completo marca o pleito como pronto para a playlist)
        #[arg(long)]
        acervo: Option<PathBuf>,
        /// Instante da classificação ("dd/mm/aaaa hh:mm:ss", hora de Brasília); padrão: agora
        #[arg(long)]
        agora: Option<String>,
        /// Horas depois do início do dia da eleição em que o pleito deixa de estar em curso
        #[arg(long, default_value_t = uv_aquisicao::pleitos::FIM_APURACAO_H)]
        fim_apuracao_h: i64,
    },
    /// Lê as listas de seções (cs.json) das UFs e mostra quantas há
    Secoes {
        #[command(flatten)]
        alvo: AlvoArgs,
        /// UFs separadas por vírgula, ou "todas" (inclui zz = exterior)
        #[arg(long, default_value = "todas")]
        ufs: String,
        /// Pasta para guardar os cs.json
        #[arg(long)]
        cache: Option<PathBuf>,
        /// CSV com uma linha por seção (uf,municipio,zona,secao,disponivel)
        #[arg(long)]
        csv: Option<PathBuf>,
    },
    /// Baixa os BUs das seções pela API de resultados (aux.json + bu.dat)
    Baixar {
        #[command(flatten)]
        alvo: AlvoArgs,
        #[command(flatten)]
        rede: RedeArgs,
        #[command(flatten)]
        saida: SaidaArgs,
        #[arg(long, default_value = "df")]
        ufs: String,
        /// Coleta só N seções, sorteadas de forma determinística (amostra)
        #[arg(long)]
        limite: Option<usize>,
        #[arg(long, default_value_t = 2026)]
        semente: u64,
        /// Ignora o que já está nos pacotes (por padrão, retoma de onde parou)
        #[arg(long)]
        refazer: bool,
        /// CSVs de métricas de coletas anteriores: pula as seções que deram 404 no aux.json ou "Não instalada"
        #[arg(long)]
        pular_sem_bu: Option<String>,
        /// Só os aux.json (situação e horário de recebimento no TSE, nas métricas), sem baixar os BUs:
        /// para o replay quando os BUs vieram do zip de dados abertos. Implica --sem-gravar e --refazer
        #[arg(long)]
        so_aux: bool,
    },
    /// Baixa os BUs do pacote zip de dados abertos da UF (cdn.tse.jus.br)
    Zip {
        #[command(flatten)]
        alvo: AlvoArgs,
        #[command(flatten)]
        rede: RedeArgs,
        #[command(flatten)]
        saida: SaidaArgs,
        #[arg(long, default_value = "df")]
        uf: String,
        #[arg(long, default_value_t = 1)]
        turno: u8,
        /// faixas (Range só nos BUs) | completo (zip inteiro e extração local)
        #[arg(long, default_value = "faixas")]
        modo: String,
        /// URL do zip (padrão: calculada a partir do ciclo, UF e turno)
        #[arg(long)]
        url: Option<String>,
        /// Zip já baixado (ex.: acervo local de dados abertos): só extrai os BUs, sem rede
        #[arg(long)]
        local: Option<PathBuf>,
        #[arg(long)]
        limite: Option<usize>,
    },
    /// Acompanha a publicação: relê os cs.json (If-None-Match) e baixa seções novas ou alteradas
    Acompanhar {
        #[command(flatten)]
        alvo: AlvoArgs,
        #[command(flatten)]
        rede: RedeArgs,
        #[command(flatten)]
        saida: SaidaArgs,
        #[arg(long, default_value = "df")]
        ufs: String,
        /// Segundos entre rodadas
        #[arg(long, default_value_t = 30)]
        intervalo: u64,
        /// Número de rodadas (0 = sem fim)
        #[arg(long, default_value_t = 3)]
        rodadas: u32,
    },
    /// Prepara o candidatos.json pré-eleição: ele-c.json, mun-…-cm.json, os -u.json de cada abrangência e os cs.json.
    /// O que já estiver em --fontes não é baixado de novo; a rede fica em ≤ 5 req/s e para no primeiro HTTP 429.
    /// Sai com código 2 se alguma fonte faltar (lista em "incompleto" no candidatos.json).
    Preparar {
        #[command(flatten)]
        alvo: AlvoArgs,
        /// Pasta de saída onde o candidatos.json será salvo
        #[arg(long)]
        saida: PathBuf,
        /// Cache das fontes (padrão: <saida>/fontes); procura em <fontes>/<sub>/<nome> e em <fontes>/<nome>
        #[arg(long)]
        fontes: Option<PathBuf>,
        /// Não usa a rede: só o que estiver em --fontes
        #[arg(long)]
        offline: bool,
        /// Baixa de novo todas as fontes (ele-c.json, cm.json, -u.json e cs.json) mesmo se já estiverem no cache (B-055)
        #[arg(long)]
        atualizar: bool,
        /// Toda destinação = "Válido" (simula a pré-apuração a partir de arquivos já apurados)
        #[arg(long)]
        ignorar_destinacao: bool,
        /// Só estes cargos (ex.: 1,3,5); padrão: todos os majoritários e consultas
        #[arg(long)]
        cargos: Option<String>,
    },
    /// Monta o catálogo (municípios, seções e candidatos) a partir dos cs.json e dos pacotes
    Catalogo {
        #[arg(long)]
        dados: PathBuf,
        #[arg(long)]
        saida: Option<PathBuf>,
        /// vagas por cargo, ex.: 5=2 (Senado em 2026)
        #[arg(long, default_value = "5=2")]
        vagas: String,
    },
    /// Totaliza em lote os BUs dos pacotes (já em memória), como o benchmark sintético
    Totalizar {
        #[arg(long)]
        dados: PathBuf,
        #[arg(long)]
        catalogo: Option<PathBuf>,
        /// Lista de nº de threads, ex.: "1 2 4 8"
        #[arg(long, default_value = "1")]
        threads: String,
        #[arg(long, default_value_t = 1)]
        repeticoes: usize,
        /// Só estas UFs (padrão: todos os pacotes)
        #[arg(long)]
        ufs: Option<String>,
        /// Acrescenta uma linha CSV por medição
        #[arg(long)]
        resumo: Option<PathBuf>,
        /// Grava os totais por UF/cargo/candidato
        #[arg(long)]
        totais: Option<PathBuf>,
        /// Grava os totais por município/cargo/candidato (cargos municipais: Prefeito, Vereador, consultas)
        #[arg(long)]
        totais_municipio: Option<PathBuf>,
        /// CSVs de métricas da coleta pela API (ou de `baixar --so-aux`): as seções cujo BU está "Excluído"
        /// foram anuladas, e os votos delas contam como nulos, como na divulgação oficial
        #[arg(long)]
        anuladas: Option<String>,
        #[arg(long, default_value = "")]
        rotulo: String,
        /// Totaliza só estes cargos (ex.: "5" = Senador; "1,3"); os demais cargos dos BUs são pulados
        #[arg(long)]
        cargos: Option<String>,
    },
    /// Replay da noite da eleição: injeta cada BU no horário em que o TSE o recebeu (das métricas da
    /// coleta), totaliza em fluxo e publica continuamente Presidente/Governador; mede a latência por BU
    Replay {
        #[arg(long)]
        dados: PathBuf,
        #[arg(long)]
        catalogo: Option<PathBuf>,
        /// CSVs de métricas da coleta (coluna recebido_tse), separados por vírgula
        #[arg(long)]
        chegadas: String,
        /// Fator de aceleração do relógio da noite (60 = 1 h em 1 min; 0 = sem espera, tudo de uma vez)
        #[arg(long, default_value_t = 60.0)]
        acelerar: f64,
        #[arg(long)]
        ufs: Option<String>,
        /// Pasta dos resultados (publicacao/, replay_resumo.json, replay_minuto.csv, totais_uf.csv)
        #[arg(long, default_value = "replay")]
        saida: PathBuf,
        /// Não grava os arquivos de resultado a cada publicação (só consolida)
        #[arg(long)]
        sem_arquivos: bool,
        /// Chegadas depois deste horário ("dd/mm/aaaa hh:mm:ss") entram no próprio horário de corte
        /// (BUs reenviados nos dias seguintes à eleição)
        #[arg(long)]
        corte: Option<String>,
        /// Cargos publicados a cada ciclo (a totalização é sempre de todos os cargos do BU)
        #[arg(long, default_value = "1,3,5")]
        cargos: String,
        /// CSVs de métricas: seções anuladas (BU "Excluído"), cujos votos contam como nulos, e não instaladas
        #[arg(long)]
        anuladas: Option<String>,
        /// Pasta dos arquivos oficiais (-u.json) para os candidatos (destinação, nascimento, eleitorado): com ela a
        /// publicação traz o "matematicamente eleito / 2º turno" e, no fim, a situação final
        #[arg(long)]
        oficial: Option<PathBuf>,
    },
    /// Imprime o conteúdo de um BU (arquivo -bu.dat)
    Inspecionar { arquivo: PathBuf },
    /// Copia só os BUs de alguns municípios ou seções (fixtures de teste): um `.uvpk` por UF, ou `-bu.dat` soltos
    Filtrar {
        /// Pasta dos pacotes de origem
        #[arg(long)]
        dados: PathBuf,
        /// "uf:município" ou "uf:município/zona/seção", separados por vírgula (ex.: ac:1031,mg:50075/47/41)
        #[arg(long)]
        secoes: String,
        #[arg(long)]
        saida: PathBuf,
        /// Grava cada BU num arquivo solto (`o<pleito><uf><município><zona><seção>-bu.dat`) em vez de pacotes
        #[arg(long)]
        soltos: bool,
    },
    /// Copia BUs de um pacote para arquivos -bu.dat soltos (para conferência com outras ferramentas)
    Exportar {
        #[arg(long)]
        pacote: PathBuf,
        #[arg(long)]
        saida: PathBuf,
        #[arg(long, default_value_t = 10)]
        n: usize,
    },
}

#[derive(Args, Clone)]
struct SaidaArgs {
    /// Pasta dos pacotes .uvpk (e do catálogo e cs.json)
    #[arg(long, default_value = "dados")]
    dados: PathBuf,
    /// Não grava pacotes (só mede a coleta)
    #[arg(long)]
    sem_gravar: bool,
    /// Totaliza em fluxo, à medida que os BUs chegam (exige catálogo)
    #[arg(long)]
    totalizar: bool,
    #[arg(long)]
    catalogo: Option<PathBuf>,
    #[arg(long, default_value_t = 4)]
    threads_totalizacao: usize,
    /// CSV com uma linha por seção coletada
    #[arg(long)]
    metricas: Option<PathBuf>,
    /// CSV de resumo (uma linha por execução, acrescentada)
    #[arg(long)]
    resumo: Option<PathBuf>,
    #[arg(long, default_value = "")]
    rotulo: String,
}

fn lista_ufs(s: &str) -> Vec<String> {
    if s == "todas" { tse::UFS.iter().map(|x| x.to_string()).collect() } else { s.split(',').map(|x| x.trim().to_lowercase()).filter(|x| !x.is_empty()).collect() }
}

/// "1,3,5" → [1, 3, 5]
fn lista_cargos(s: &str) -> anyhow::Result<Vec<u8>> {
    s.split(',').filter(|x| !x.trim().is_empty()).map(|x| x.trim().parse::<u8>().map_err(|e| anyhow::anyhow!("cargo inválido '{x}': {e}"))).collect()
}

fn agora() -> String {
    let d = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
    format!("{d}")
}

fn acrescentar(p: &Path, cab: &str, linha: &str) -> std::io::Result<()> {
    use std::io::Write;
    let novo = !p.exists();
    if let Some(d) = p.parent() { if !d.as_os_str().is_empty() { std::fs::create_dir_all(d)?; } }
    let mut f = std::fs::OpenOptions::new().create(true).append(true).open(p)?;
    if novo { writeln!(f, "{cab}")?; }
    writeln!(f, "{linha}")
}

/// Consumidor dos BUs recebidos: grava os pacotes e/ou totaliza em fluxo.
struct Consumidor {
    tx: crossbeam_channel::Sender<Recebido>,
    h: std::thread::JoinHandle<anyhow::Result<ResultadoConsumo>>,
}
struct ResultadoConsumo { registros: u64, bytes: u64, ultimo_aplicado: Option<Instant>, estado: Option<(Arc<Catalogo>, Arc<Estado>)> }

fn iniciar_consumidor(s: &SaidaArgs) -> anyhow::Result<Consumidor> {
    let (tx, rx) = crossbeam_channel::unbounded::<Recebido>();
    let gravar = !s.sem_gravar;
    let dir = s.dados.clone();
    let tot = if s.totalizar {
        let p = s.catalogo.clone().unwrap_or_else(|| s.dados.join("catalogo.txt"));
        let cat = Arc::new(Catalogo::carregar(&p)?);
        let est = Arc::new(Estado::novo(&cat));
        eprintln!("totalização em fluxo: catálogo {} ({} municípios, {} seções, estado {:.1} MB)", p.display(), cat.muni_cod.len(), cat.secoes.len(), cat.memoria_estado_mb());
        Some((cat, est, s.threads_totalizacao.max(1)))
    } else { None };
    let h = std::thread::spawn(move || {
        let mut esc = if gravar { Some(pacote::Escritor::novo(&dir)?) } else { None };
        let ultimo = Arc::new(Mutex::new(None::<Instant>));
        let (ttx, trx) = crossbeam_channel::bounded::<Recebido>(4096);
        let mut hs = vec![];
        if let Some((cat, est, n)) = &tot {
            for _ in 0..*n {
                let (cat, est, trx, ultimo) = (cat.clone(), est.clone(), trx.clone(), ultimo.clone());
                hs.push(std::thread::spawn(move || {
                    let mut ag = Agregador::novo(&cat, &est);
                    for r in trx.iter() {
                        est.aplicar(&cat, r.reg.chave(), &r.bu, &mut ag);
                        *ultimo.lock().unwrap() = Some(Instant::now());
                    }
                }));
            }
        }
        drop(trx);
        let (mut registros, mut bytes) = (0, 0);
        for r in rx.iter() {
            registros += 1;
            bytes += r.bu.len() as u64;
            if let Some(e) = esc.as_mut() { e.gravar(&r.reg, &r.bu)?; }
            if tot.is_some() { let _ = ttx.send(r); }
        }
        drop(ttx);
        for h in hs { let _ = h.join(); }
        if let Some(e) = esc.as_mut() { e.fechar()?; }
        let u = *ultimo.lock().unwrap();
        Ok(ResultadoConsumo { registros, bytes, ultimo_aplicado: u, estado: tot.map(|(c, e, _)| (c, e)) })
    });
    Ok(Consumidor { tx, h })
}

const CAB_RESUMO: &str = "modo,rotulo,ufs,secoes,ok,erros,concorrencia,conexoes,http,rps_limite,segundos,secoes_por_s,req_por_s,mb,mb_por_s,lat_aux_p50_ms,lat_bu_p50_ms,lat_p95_ms,lat_p99_ms,tentativas_extras,http429,lag_totalizacao_ms,adaptativo,rps_final,max_req_em_1s,data";

#[allow(clippy::too_many_arguments)]
fn relatar(modo: &str, s: &SaidaArgs, rede_args: &RedeArgs, rede: &Rede, ufs: &str, medidas: &[Medida], seg: f64,
           fim_coleta: Instant, cons: ResultadoConsumo) -> anyhow::Result<()> {
    let ok = medidas.iter().filter(|m| m.erro.is_empty()).count();
    let erros = medidas.len() - ok;
    let mut lat_aux: Vec<f64> = medidas.iter().filter(|m| m.lat_aux > 0.0).map(|m| m.lat_aux * 1e3).collect();
    let mut lat_bu: Vec<f64> = medidas.iter().filter(|m| m.lat_bu > 0.0).map(|m| m.lat_bu * 1e3).collect();
    let mut todas: Vec<f64> = lat_aux.iter().chain(lat_bu.iter()).copied().collect();
    for v in [&mut lat_aux, &mut lat_bu, &mut todas] { v.sort_by(|a, b| a.partial_cmp(b).unwrap()); }
    let reqs = rede.reqs.load(Relaxed);
    let mb = rede.bytes.load(Relaxed) as f64 / 1e6;
    let lag = cons.ultimo_aplicado.map(|u| if u > fim_coleta { (u - fim_coleta).as_secs_f64() * 1e3 } else { 0.0 });
    let versao = rede.versao.lock().unwrap().clone();
    let linha = format!("{modo},{},{},{},{ok},{erros},{},{},{},{},{seg:.3},{:.1},{:.1},{mb:.2},{:.2},{:.1},{:.1},{:.1},{:.1},{},{},{},{},{:.0},{},{}",
        s.rotulo, ufs.replace(',', "+"), medidas.len(), rede_args.concorrencia, rede_args.conexoes, versao, if rede_args.rps <= 0.0 { coletor::TETO_RPS } else { rede_args.rps.min(coletor::TETO_RPS) },
        ok as f64 / seg, reqs as f64 / seg, mb / seg,
        coletor::percentil(&lat_aux, 50.0), coletor::percentil(&lat_bu, 50.0), coletor::percentil(&todas, 95.0), coletor::percentil(&todas, 99.0),
        rede.tentativas_extras.load(Relaxed), rede.http429.load(Relaxed), lag.map(|x| format!("{x:.1}")).unwrap_or_default(),
        rede_args.adaptativo as u8, rede.lim.rps_atual(), rede.lim.inicios_por_segundo().iter().max().copied().unwrap_or(0), agora());
    eprintln!("\n== {modo}: {ok} BUs em {seg:.1} s = {:.0} seções/s, {:.0} req/s, {mb:.1} MB ({:.1} MB/s), {versao}", ok as f64 / seg, reqs as f64 / seg, mb / seg);
    eprintln!("   máximo de requisições iniciadas num mesmo segundo: {} (teto {})", rede.lim.inicios_por_segundo().iter().max().copied().unwrap_or(0), coletor::TETO_RPS);
    eprintln!("   latência p50 aux {:.0} ms, bu {:.0} ms; p95 {:.0} ms; p99 {:.0} ms; novas tentativas {}; HTTP 429: {}; erros: {erros}",
        coletor::percentil(&lat_aux, 50.0), coletor::percentil(&lat_bu, 50.0), coletor::percentil(&todas, 95.0), coletor::percentil(&todas, 99.0),
        rede.tentativas_extras.load(Relaxed), rede.http429.load(Relaxed));
    if cons.registros > 0 && !s.sem_gravar { eprintln!("   pacotes: {} BUs, {:.1} MB em {}", cons.registros, cons.bytes as f64 / 1e6, s.dados.display()); }
    for m in medidas.iter().filter(|m| !m.erro.is_empty()).take(5) { eprintln!("   erro {} {:05}/{:04}/{:04}: {}", m.uf, m.municipio, m.zona, m.secao, m.erro); }
    if let Some((cat, est)) = &cons.estado {
        eprintln!("   totalização em fluxo: {} BUs aplicados, atraso após o último BU: {:.1} ms; cargos inválidos {}, desconhecidos {}, duplicados {}, fora do catálogo {}, erros {}",
            est.bus.load(Relaxed), lag.unwrap_or(0.0), est.cargos_invalidos.load(Relaxed), est.desconhecidos.load(Relaxed),
            est.duplicados.load(Relaxed), est.fora_catalogo.load(Relaxed), est.erros.load(Relaxed));
        let uf = est.consolidar_uf(cat);
        eprint!("{}", estado::resumo(cat, &uf, 3));
    }
    println!("{linha}");
    if let Some(p) = &s.resumo { acrescentar(p, CAB_RESUMO, &linha)?; }
    if let Some(p) = &s.metricas {
        let mut txt = String::from(Medida::CAB_CSV);
        txt.push('\n');
        for m in medidas { txt += &m.csv(); txt.push('\n'); }
        if let Some(d) = p.parent() { std::fs::create_dir_all(d)?; }
        std::fs::write(p, txt)?;
        let mut ini = String::from("segundo,requisicoes\n");
        for (i, n) in rede.lim.inicios_por_segundo().iter().enumerate() { ini += &format!("{i},{n}\n"); }
        std::fs::write(p.with_extension("inicios.csv"), ini)?;
        if rede_args.adaptativo {
            let mut t = String::from("t_s,rps\n");
            for (a, b) in rede.lim.historico() { t += &format!("{a:.2},{b:.1}\n"); }
            std::fs::write(p.with_extension("taxa.csv"), t)?;
        }
    }
    Ok(())
}

/// Embaralhamento determinístico (xorshift) para amostras reproduzíveis.
fn embaralhar<T>(v: &mut [T], semente: u64) {
    let mut x = semente.max(1);
    for i in (1..v.len()).rev() {
        x ^= x << 13; x ^= x >> 7; x ^= x << 17;
        v.swap(i, (x % (i as u64 + 1)) as usize);
    }
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;
    match cli.cmd {
        Cmd::Eleicoes { alvo } => rt.block_on(async {
            let rede = Rede::nova(&ConfigRede::simples(4, 20.0, 3, 30, 5))?;
            let r = rede.get(&alvo.alvo().url_ele_c(), None, None).await?;
            let e: tse::EleC = serde_json::from_slice(&r.corpo)?;
            println!("pleito  ciclo    data        eleição  turno  nome");
            for p in &e.pl { for el in &p.e { println!("{:<7} {:<8} {:<11} {:<8} {:<6} {}", p.cd, p.c, p.dt, el.cd, el.t, el.nm); } }
            anyhow::Ok(())
        })?,

        Cmd::Pleitos { alvo, saida, ele_c, guardar, acervo, agora, fim_apuracao_h } => {
            use uv_aquisicao::pleitos;
            let bytes = match &ele_c {
                Some(p) => std::fs::read(p).map_err(|e| anyhow::anyhow!("{}: {e}", p.display()))?,
                None => rt.block_on(async {
                    let rede = Rede::nova(&ConfigRede::simples(1, 5.0, 3, 30, 1))?;
                    anyhow::Ok(rede.get(&alvo.alvo().url_ele_c(), None, None).await?.corpo.to_vec())
                })?,
            };
            if let Some(g) = &guardar { pleitos::gravar_atomico(g, &bytes)?; }
            let agora = match &agora {
                Some(s) => urna_velox::metricas::segundos(s).ok_or_else(|| anyhow::anyhow!("--agora inválido: {s}"))?,
                None => uv_aquisicao::agora_brasilia(),
            };
            let lista = pleitos::descobrir(&bytes, agora, &pleitos::Opcoes { acervo, fim_apuracao_h })?;
            println!("pleito  data        situação   replay  nome");
            for p in &lista {
                println!("{:<7} {:<11} {:<10} {:<7} {}", p.codigo, p.data, p.situacao.texto(), if p.replay { "sim" } else { "-" }, p.titulo);
            }
            if let Some(s) = &saida {
                let j = pleitos::json(&lista, agora, &bytes);
                pleitos::gravar_atomico(s, serde_json::to_string_pretty(&j)?.as_bytes())?;
                eprintln!("{} pleitos em {}", lista.len(), s.display());
            }
        }

        Cmd::Secoes { alvo, ufs, cache, csv } => rt.block_on(async {
            let rede = Rede::nova(&ConfigRede::simples(28, 50.0, 3, 60, 5))?;
            let t0 = Instant::now();
            let cfgs = coletor::ler_configs(&rede, &alvo.alvo(), &lista_ufs(&ufs), cache.as_deref()).await?;
            let mut total = 0;
            let mut txt = String::from("uf,municipio,zona,secao,disponivel\n");
            println!("uf  municípios  seções   gerado em");
            for (uf, c) in &cfgs {
                let s = c.secoes(uf);
                total += s.len();
                println!("{uf}  {:>10}  {:>7}   {} {}", c.municipios().len(), s.len(), c.dg, c.hg);
                if csv.is_some() { for x in &s { txt += &format!("{uf},{},{},{},{}\n", x.municipio, x.zona, x.secao, x.disponivel); } }
            }
            println!("total: {total} seções em {} UFs ({:.2} s, {:.1} MB lidos)", cfgs.len(), t0.elapsed().as_secs_f64(), rede.bytes.load(Relaxed) as f64 / 1e6);
            if let Some(p) = csv { tokio::fs::write(p, txt).await?; }
            anyhow::Ok(())
        })?,

        Cmd::Baixar { alvo, rede: ra, mut saida, ufs, limite, semente, mut refazer, pular_sem_bu, so_aux } => {
            if so_aux { saida.sem_gravar = true; saida.totalizar = false; refazer = true; }
            let alvo = Arc::new(alvo.alvo());
            let rede = Arc::new(Rede::nova(&ra.cfg())?);
            let lufs = lista_ufs(&ufs);
            let cfgs = rt.block_on(coletor::ler_configs(&rede, &alvo, &lufs, Some(&saida.dados.join("config"))))?;
            let mut secoes: Vec<tse::Secao> = cfgs.iter().flat_map(|(uf, c)| c.secoes(uf)).collect();
            let n_total = secoes.len();
            if !refazer && !saida.sem_gravar {
                let ja = pacote::ja_coletadas(&saida.dados);
                secoes.retain(|s| !ja.contains(&s.chave()));
                if secoes.len() < n_total { eprintln!("retomando: {} seções já nos pacotes", n_total - secoes.len()); }
            }
            if let Some(p) = &pular_sem_bu {
                let sem = secoes_sem_bu(p)?;
                let antes = secoes.len();
                secoes.retain(|s| !sem.contains(&(s.uf, s.chave())));
                eprintln!("pulando {} seções sem BU conhecidas ({} em {p})", antes - secoes.len(), sem.len());
            }
            if let Some(n) = limite { embaralhar(&mut secoes, semente); secoes.truncate(n); }
            eprintln!("baixando {} seções de {} UF(s) [{} no total]: concorrência {}, {} conexão(ões), teto {} req/s, {}",
                secoes.len(), lufs.len(), n_total, ra.concorrencia, ra.conexoes, rede.lim.rps_atual(), if ra.http1 { "HTTP/1.1" } else { "HTTP/2" });
            // zera os contadores (a leitura dos cs.json não entra na medida)
            let rede = Arc::new(Rede::nova(&ra.cfg())?);
            let cons = iniciar_consumidor(&saida)?;
            let t0 = Instant::now();
            let medidas = rt.block_on(coletor::coletar(rede.clone(), alvo, secoes, ra.concorrencia, cons.tx.clone(), t0, true, so_aux));
            let fim = Instant::now();
            let seg = t0.elapsed().as_secs_f64();
            drop(cons.tx);
            let rc = cons.h.join().map_err(|_| anyhow::anyhow!("consumidor falhou"))??;
            relatar(if so_aux { "api-aux" } else { "api" }, &saida, &ra, &rede, &ufs, &medidas, seg, fim, rc)?;
        }

        Cmd::Zip { alvo, rede: ra, saida, uf, turno, modo, url, local, limite } => {
            let url = url.unwrap_or_else(|| alvo.alvo().url_zip_dados_abertos(&uf, turno));
            let rede = Arc::new(Rede::nova(&ra.cfg())?);
            let cons = iniciar_consumidor(&saida)?;
            let t0 = Instant::now();
            let medidas = if let Some(arq) = &local {
                let bus = zip::extrair_local(arq)?;
                eprintln!("{}: {} BUs extraídos em {:.2} s", arq.display(), bus.len(), t0.elapsed().as_secs_f64());
                let mut v = vec![];
                for r in bus {
                    v.push(Medida { uf: String::from_utf8_lossy(&r.reg.uf).into_owned(), municipio: r.reg.municipio, zona: r.reg.zona, secao: r.reg.secao, bytes_bu: r.bu.len(), t_fim: t0.elapsed().as_secs_f64(), ..Default::default() });
                    let _ = cons.tx.send(r);
                }
                v
            } else if modo == "completo" {
                std::fs::create_dir_all(&saida.dados)?;
                let destino = saida.dados.join(format!("dados-abertos-{uf}.zip"));
                eprintln!("baixando o zip inteiro: {url}");
                let n = rt.block_on(zip::baixar_completo(&url, &destino))?;
                let t_dl = t0.elapsed().as_secs_f64();
                rede.bytes.fetch_add(n, Relaxed);
                rede.reqs.fetch_add(1, Relaxed);
                let t1 = Instant::now();
                let bus = zip::extrair_local(&destino)?;
                eprintln!("  download {:.1} s ({:.1} MB/s); extração de {} BUs {:.2} s", t_dl, n as f64 / 1e6 / t_dl, bus.len(), t1.elapsed().as_secs_f64());
                let mut v = vec![];
                for r in bus {
                    v.push(Medida { uf: uf.clone(), municipio: r.reg.municipio, zona: r.reg.zona, secao: r.reg.secao, bytes_bu: r.bu.len(), t_fim: t0.elapsed().as_secs_f64(), ..Default::default() });
                    let _ = cons.tx.send(r);
                }
                v
            } else {
                let (tam, ents) = rt.block_on(zip::listar_remoto(&rede, &url))?;
                let mut bus: Vec<zip::Entrada> = ents.iter().filter(|e| zip::e_bu(&e.nome)).cloned().collect();
                let comp_bu: u64 = bus.iter().map(|e| e.comp).sum();
                eprintln!("zip {url}: {:.1} MB, {} entradas, {} BUs ({:.1} MB comprimidos = {:.1} % do pacote); diretório lido em {:.2} s",
                    tam as f64 / 1e6, ents.len(), bus.len(), comp_bu as f64 / 1e6, 100.0 * comp_bu as f64 / tam as f64, t0.elapsed().as_secs_f64());
                if let Some(n) = limite { bus.truncate(n); }
                rt.block_on(zip::coletar_faixas(rede.clone(), url.clone(), bus, ra.concorrencia, cons.tx.clone(), t0))
            };
            let fim = Instant::now();
            let seg = t0.elapsed().as_secs_f64();
            drop(cons.tx);
            let rc = cons.h.join().map_err(|_| anyhow::anyhow!("consumidor falhou"))??;
            relatar(&format!("zip-{modo}"), &saida, &ra, &rede, &uf, &medidas, seg, fim, rc)?;
        }

        Cmd::Acompanhar { alvo, rede: ra, saida, ufs, intervalo, rodadas } => {
            let alvo = Arc::new(alvo.alvo());
            let rede = Arc::new(Rede::nova(&ra.cfg())?);
            let lufs = lista_ufs(&ufs);
            let mut etags: HashMap<String, String> = HashMap::new();
            let mut vistos: HashMap<u64, String> = HashMap::new();
            if !saida.sem_gravar { for k in pacote::ja_coletadas(&saida.dados) { vistos.insert(k, String::new()); } }
            let cons = iniciar_consumidor(&saida)?;
            let t0 = Instant::now();
            let mut todas = vec![];
            let mut r = 0;
            while rodadas == 0 || r < rodadas {
                r += 1;
                let tr = Instant::now();
                let (mut n304, mut novas) = (0, vec![]);
                for uf in &lufs {
                    let resp = rt.block_on(rede.get(&alvo.url_cs(uf), etags.get(uf).map(|s| s.as_str()), None))?;
                    if resp.status == 304 { n304 += 1; continue; }
                    if let Some(e) = resp.etag { etags.insert(uf.clone(), e); }
                    let c: tse::ConfigSecoes = serde_json::from_slice(&resp.corpo)?;
                    for s in c.secoes(uf) {
                        let k = s.chave();
                        match vistos.get(&k) {
                            Some(d) if d.is_empty() || *d == s.disponivel => {}
                            _ => { vistos.insert(k, s.disponivel.clone()); novas.push(s); }
                        }
                    }
                }
                let n_novas = novas.len();
                let m = rt.block_on(coletor::coletar(rede.clone(), alvo.clone(), novas, ra.concorrencia, cons.tx.clone(), t0, true, false));
                eprintln!("rodada {r}: {} cs.json sem mudança (304), {n_novas} seções novas/alteradas coletadas em {:.2} s", n304, tr.elapsed().as_secs_f64());
                todas.extend(m);
                if rodadas == 0 || r < rodadas { std::thread::sleep(std::time::Duration::from_secs(intervalo)); }
            }
            let fim = Instant::now();
            let seg = t0.elapsed().as_secs_f64();
            drop(cons.tx);
            let rc = cons.h.join().map_err(|_| anyhow::anyhow!("consumidor falhou"))??;
            relatar("acompanhar", &saida, &ra, &rede, &ufs, &todas, seg, fim, rc)?;
        }

        Cmd::Preparar { alvo: a_arg, saida, fontes, offline, atualizar, ignorar_destinacao, cargos } => {
            let alvo = a_arg.alvo();
            let op = preparo::Opcoes {
                fontes: fontes.unwrap_or_else(|| saida.join("fontes")),
                saida,
                offline,
                atualizar,
                ignorar_destinacao,
                cargos: match &cargos { Some(c) => lista_cargos(c)?, None => vec![] },
            };
            let incompleto = preparo::preparar(&rt, &alvo, &op)?;
            if !incompleto.is_empty() {
                eprintln!("ATENÇÃO: {} fonte(s) ausente(s); candidatos.json incompleto:", incompleto.len());
                for x in incompleto.iter().take(30) { eprintln!("  {x}"); }
                std::process::exit(2);
            }
        }

        Cmd::Catalogo { dados, saida, vagas } => {
            let t0 = Instant::now();
            let vagas: Vec<(u8, u8)> = vagas.split(',').filter_map(|x| { let (a, b) = x.split_once('=')?; Some((a.parse().ok()?, b.parse().ok()?)) }).collect();
            // municípios e seções, dos cs.json guardados pela coleta
            let mut ufs: Vec<[u8; 2]> = vec![];
            let (mut munis, mut secoes) = (vec![], vec![]);
            let mut pleito = 0;
            let mut cs: Vec<PathBuf> = std::fs::read_dir(dados.join("config"))?.flatten().map(|e| e.path()).filter(|p| p.to_string_lossy().ends_with("-cs.json")).collect();
            cs.sort();
            for p in cs {
                let nome = p.file_name().unwrap().to_string_lossy().to_string();
                let uf = nome[..2].to_string();
                pleito = nome[4..10].parse().unwrap_or(0);
                let c: tse::ConfigSecoes = serde_json::from_slice(&std::fs::read(&p)?)?;
                let ui = ufs.len() as u8;
                ufs.push(uf.as_bytes()[..2].try_into()?);
                for (m, _) in c.municipios() { munis.push((m, ui)); }
                for s in c.secoes(&uf) { secoes.push(s.chave()); }
            }
            // candidatos, dos BUs coletados
            let t1 = Instant::now();
            let mem = pacote::EmMemoria::carregar(&dados, None)?;
            let (cands, mcands, erros) = catalogo::candidatos_dos_bus(&mem, &ufs);
            let t_bus = t1.elapsed().as_secs_f64();
            let cat = Catalogo::montar(pleito, ufs, munis, cands, mcands, &vagas, secoes);
            let p = saida.unwrap_or_else(|| dados.join("catalogo.txt"));
            cat.salvar(&p)?;
            let ncand: usize = cat.cand_cod.iter().map(|v| v.len()).sum();
            eprintln!("catálogo {}: {} UFs, {} municípios, {} seções, {} cargos, {} candidatos (UF×cargo), {} contadores = {:.1} MB; {} BUs lidos em {:.2} s ({} com erro); total {:.2} s",
                p.display(), cat.ufs.len(), cat.muni_cod.len(), cat.secoes.len(), cat.cargos.len(), ncand, cat.slots, cat.memoria_estado_mb(), mem.itens.len(), t_bus, erros, t0.elapsed().as_secs_f64());
        }

        Cmd::Totalizar { dados, catalogo, threads, repeticoes, ufs, resumo, totais, totais_municipio, anuladas, rotulo, cargos } => {
            let mut cat = Catalogo::carregar(&catalogo.unwrap_or_else(|| dados.join("catalogo.txt")))?;
            if let Some(a) = &anuladas { eprintln!("seções anuladas (votos contados como nulos): {}", cat.ler_anuladas(a)?); }
            if let Some(c) = &cargos {
                let so = lista_cargos(c)?;
                cat = cat.restringir(&so);
                eprintln!("só os cargos {:?} ({})", so, so.iter().map(|&x| catalogo::nome_cargo(x)).collect::<Vec<_>>().join(", "));
            }
            let t0 = Instant::now();
            let filtro = ufs.map(|u| lista_ufs(&u));
            let mem = pacote::EmMemoria::carregar(&dados, filtro.as_deref())?;
            let t_carga = t0.elapsed().as_secs_f64();
            let mb = mem.bytes() as f64 / 1048576.0;
            eprintln!("{} BUs em memória ({mb:.1} MB, lidos em {t_carga:.2} s); estado {:.1} MB ({} contadores)", mem.itens.len(), cat.memoria_estado_mb(), cat.slots);
            let cab = "lang,variant,kind,threads,bus,seconds,bus_per_s,consolidate_ms,peak_rss_mb,corpus_mb,invalid,checksum,anon_rss_mb,rotulo";
            let mut ultimo = None;
            for t in threads.split_whitespace().filter_map(|x| x.parse::<usize>().ok()) {
                for _ in 0..repeticoes {
                    let (est, seg) = estado::totalizar_lote(&cat, &mem, t);
                    let t1 = Instant::now();
                    let uf = est.consolidar_uf(&cat);
                    let cons_ms = t1.elapsed().as_secs_f64() * 1e3;
                    let n = est.bus.load(Relaxed);
                    let linha = format!("rust,real-dense,real,{t},{n},{seg:.6},{:.0},{cons_ms:.2},{:.1},{mb:.1},{},{},{:.1},{rotulo}",
                        n as f64 / seg, campo_status_mb("VmHWM"), est.cargos_invalidos.load(Relaxed), est.checksum(), campo_status_mb("RssAnon"));
                    println!("{linha}");
                    if let Some(p) = &resumo { acrescentar(p, cab, &linha)?; }
                    ultimo = Some((est, uf));
                }
            }
            if let Some((est, uf)) = ultimo {
                eprintln!("conferência: BUs {} | cargos inválidos {} | votáveis desconhecidos {} | duplicados {} | fora do catálogo {} | identificação divergente {} | erros de leitura {}",
                    est.bus.load(Relaxed), est.cargos_invalidos.load(Relaxed), est.desconhecidos.load(Relaxed), est.duplicados.load(Relaxed),
                    est.fora_catalogo.load(Relaxed), est.divergentes.load(Relaxed), est.erros.load(Relaxed));
                eprint!("{}", estado::resumo(&cat, &uf, 28));
                if let Some(p) = totais { std::fs::write(p, estado::csv_totais_uf(&cat, &uf))?; }
                if let Some(p) = totais_municipio { std::fs::write(p, estado::csv_totais_municipio(&cat, &est, None))?; }
            }
        }

        Cmd::Replay { dados, catalogo, chegadas, acelerar, ufs, saida, sem_arquivos, corte, cargos, anuladas, oficial } => {
            let cargos_pub = lista_cargos(&cargos)?;
            let corte = match corte { Some(c) => Some(replay::segundos(&c).ok_or_else(|| anyhow::anyhow!("--corte inválido: {c}"))?), None => None };
            let mut cat = Catalogo::carregar(&catalogo.unwrap_or_else(|| dados.join("catalogo.txt")))?;
            if let Some(a) = &anuladas { eprintln!("seções anuladas (votos contados como nulos): {}", cat.ler_anuladas(a)?); }
            let cat = Arc::new(cat);
            let filtro = ufs.map(|u| lista_ufs(&u));
            let mem = pacote::EmMemoria::carregar(&dados, filtro.as_deref())?;
            let arqs: Vec<PathBuf> = chegadas.split(',').filter(|s| !s.is_empty()).map(PathBuf::from).collect();
            let ch = replay::ler_chegadas(&arqs)?;
            std::fs::create_dir_all(&saida)?;
            eprintln!("replay: {} BUs em memória, {} horários de recebimento, aceleração {}×", mem.itens.len(), ch.len(), acelerar);
            let candidatos = match &oficial { Some(p) => Some(urna_velox::candidatos::Candidatos::dos_arquivos_oficiais(p)?), None => None };
            let metricas = match &anuladas { Some(a) => Some(urna_velox::metricas::Metricas::ler_lista(a)?), None => None };
            if let Some(c) = &candidatos { eprintln!("candidatos: {} circunscrições ({})", c.n_circunscricoes(), oficial.as_ref().unwrap().display()); }
            let r = replay::executar(cat, &mem, &ch, &replay::Config { acelerar, saida: saida.clone(), gravar: !sem_arquivos, corte, cargos: cargos_pub, candidatos, metricas })?;
            print!("{r}");
        }

        Cmd::Inspecionar { arquivo } => {
            let b = std::fs::read(&arquivo)?;
            let mut t = bu::Texto(String::new());
            bu::ler_bu(&b, &mut t)?;
            print!("{}", t.0);
        }

        Cmd::Filtrar { dados, secoes, saida, soltos } => {
            // (uf, município, seção opcional)
            let mut alvo: Vec<([u8; 2], u32, Option<(u16, u16)>)> = vec![];
            for x in secoes.split(',').map(str::trim).filter(|x| !x.is_empty()) {
                let (uf, resto) = x.split_once(':').ok_or_else(|| anyhow::anyhow!("seção inválida: {x}"))?;
                let uf: [u8; 2] = uf.to_lowercase().as_bytes().try_into().map_err(|_| anyhow::anyhow!("UF inválida: {x}"))?;
                let p: Vec<&str> = resto.split('/').collect();
                let mu: u32 = p[0].parse()?;
                let se = if p.len() == 3 { Some((p[1].parse()?, p[2].parse()?)) } else { None };
                alvo.push((uf, mu, se));
            }
            let ufs: Vec<String> = alvo.iter().map(|a| String::from_utf8_lossy(&a.0).into_owned()).collect::<std::collections::BTreeSet<_>>().into_iter().collect();
            let mem = pacote::EmMemoria::carregar(&dados, Some(&ufs))?;
            std::fs::create_dir_all(&saida)?;
            let mut esc = if soltos { None } else { Some(pacote::Escritor::novo(&saida)?) };
            let mut n = 0;
            for i in 0..mem.itens.len() {
                let r = &mem.itens[i].0;
                if !alvo.iter().any(|(u, m, s)| *u == r.uf && *m == r.municipio && s.is_none_or(|(z, x)| z == r.zona && x == r.secao)) { continue; }
                match esc.as_mut() {
                    Some(e) => e.gravar(r, mem.bu(i))?,
                    None => {
                        let nome = format!("o{:05}{}{:05}{:04}{:04}-bu.dat", r.pleito, std::str::from_utf8(&r.uf)?, r.municipio, r.zona, r.secao);
                        std::fs::write(saida.join(nome), mem.bu(i))?;
                    }
                }
                n += 1;
            }
            if let Some(e) = esc.as_mut() { e.fechar()?; }
            eprintln!("{n} BUs de {} copiados para {}", dados.display(), saida.display());
        }

        Cmd::Exportar { pacote: p, saida, n } => {
            std::fs::create_dir_all(&saida)?;
            let dir = p.parent().unwrap_or(Path::new("."));
            let uf = p.file_stem().unwrap().to_string_lossy().to_string();
            let mem = pacote::EmMemoria::carregar(dir, Some(&[uf]))?;
            for i in 0..mem.itens.len().min(n) {
                let r = &mem.itens[i].0;
                let nome = format!("o{:05}{}{:05}{:04}{:04}-bu.dat", r.pleito, std::str::from_utf8(&r.uf)?, r.municipio, r.zona, r.secao);
                std::fs::write(saida.join(nome), mem.bu(i))?;
            }
            eprintln!("{} BUs exportados para {}", mem.itens.len().min(n), saida.display());
        }
    }
    Ok(())
}

/// Campo de /proc/self/status em MB (VmHWM = pico de RSS, inclui páginas do mmap; RssAnon = heap/pilhas).
fn campo_status_mb(campo: &str) -> f64 {
    std::fs::read_to_string("/proc/self/status").ok()
        .and_then(|s| s.lines().find(|l| l.starts_with(campo)).and_then(|l| l.split_whitespace().nth(1)?.parse::<f64>().ok()))
        .map(|k| k / 1024.0).unwrap_or(-1.0)
}
