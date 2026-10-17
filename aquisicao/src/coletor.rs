//! Motor de coleta: requisições HTTP concorrentes ao CDN do TSE com limite de taxa,
//! novas tentativas e entrega de cada BU recebido ao consumidor (pacote e/ou totalizador).

use urna_velox::pacote::Registro;
use crate::tse::{self, Alvo, Aux, Secao};
use bytes::Bytes;
use futures::StreamExt;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering::Relaxed};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Teto absoluto de requisições por segundo que o coletor aceita fazer, por processo, em qualquer
/// configuração: é a regra do projeto (acima disso a borda do TSE responde HTTP 429). `--rps` acima disso é reduzido.
pub const TETO_RPS: f64 = 100.0;

pub const USER_AGENT: &str = concat!("urna-velox-coletor/", env!("CARGO_PKG_VERSION"), " (+https://github.com/lukeboh/urna-velox-civica)");

#[derive(Clone, Debug)]
pub struct ConfigRede {
    /// Requisições simultâneas (seções em andamento).
    pub concorrencia: usize,
    /// Clientes HTTP independentes (com HTTP/2, cada um costuma usar uma conexão TCP).
    pub conexoes: usize,
    /// Teto de requisições por segundo; sempre limitado a `TETO_RPS` (0 = o próprio `TETO_RPS`).
    pub rps: f64,
    /// Controle adaptativo de taxa (AIMD) guiado pelos HTTP 429 do CDN
    pub adaptativo: bool,
    pub rps_inicial: f64,
    pub http1: bool,
    pub tentativas: u32,
    pub timeout_s: u64,
    /// Disjuntor: ao somar este nº de HTTP 429, a coleta para por inteiro (nenhuma requisição nova).
    /// A borda do CDN põe o IP numa "caixa de penalidade"; insistir só prolonga o bloqueio (e o IP é compartilhado).
    pub max_429: u64,
    /// Teto de req/s acima de `TETO_RPS`: só vale para um servidor local (site simulado), ver [`limites`] e
    /// [`Rede::para`]; 0 = `TETO_RPS`.
    pub teto: f64,
    /// Pausa global depois de qualquer HTTP 429 (s): nenhuma requisição nova até lá; 0 = só o AIMD/disjuntor.
    pub pausa_429_s: f64,
    /// Endereço local de saída (federação, 2.0: no simulador, cada nó sai por um `127.0.0.x` próprio); `None` = o do
    /// sistema.
    pub ip_local: Option<std::net::IpAddr>,
}

impl ConfigRede {
    /// Configuração simples (sem AIMD, sem pausa), como a do `preparar` e dos comandos de consulta.
    pub fn simples(concorrencia: usize, rps: f64, tentativas: u32, timeout_s: u64, max_429: u64) -> ConfigRede {
        ConfigRede { concorrencia, conexoes: 1, rps, adaptativo: false, rps_inicial: 0.0, http1: false, tentativas, timeout_s, max_429, teto: 0.0, pausa_429_s: 0.0, ip_local: None }
    }
}

/// Pausa mínima depois de um HTTP 429 do TSE (s): a borda põe o IP numa "caixa de penalidade" e o IP de saída é
/// compartilhado (CLAUDE.md).
pub const PAUSA_429_TSE_S: f64 = 600.0;

/// O servidor é local (`127.0.0.1`, `localhost`, `::1`)? Só um servidor local — o site simulado do `uv-simulador` —
/// pode receber mais de `TETO_RPS` req/s ou uma pausa de 429 menor que 10 min.
pub fn base_local(base: &str) -> bool {
    let sem = base.split_once("://").map_or(base, |x| x.1);
    let host_porta = sem.split('/').next().unwrap_or("");
    let host = if let Some(r) = host_porta.strip_prefix('[') { r.split(']').next().unwrap_or("") } else { host_porta.split(':').next().unwrap_or("") };
    matches!(host.to_ascii_lowercase().as_str(), "127.0.0.1" | "localhost" | "::1")
}

/// Limites de rede para uma base: (teto de req/s, req/s pedidas, pausa depois de um 429 em s).
///
/// Contra o TSE (qualquer host não local): teto `TETO_RPS` (100), req/s ≤ 100 e pausa ≥ 10 min, sem exceção — a
/// `aceleracao` (> 1) é recusada. Contra um servidor local com o relógio acelerado `aceleracao` vezes: teto e req/s
/// multiplicados e pausa dividida pela aceleração, para que a proporção com o relógio simulado seja a mesma da noite.
pub fn limites(base: &str, rps: f64, aceleracao: f64) -> anyhow::Result<(f64, f64, f64)> {
    let rps = if rps <= 0.0 { 95.0 } else { rps };
    if base_local(base) {
        let a = aceleracao.max(1.0);
        Ok((TETO_RPS * a, rps.min(TETO_RPS) * a, PAUSA_429_TSE_S / a))
    } else {
        anyhow::ensure!(aceleracao <= 1.0, "aceleração {aceleracao} recusada: só vale contra um servidor local (127.0.0.1/localhost), não contra {base}");
        Ok((TETO_RPS, rps.min(TETO_RPS), PAUSA_429_TSE_S))
    }
}

/// Espera máxima de uma requisição de prioridade baixa por uma vaga livre ([`Limitador::esperar_baixa`]).
pub const ESPERA_BAIXA_MAX: Duration = Duration::from_secs(10);

/// Espaçamento das requisições (GCRA simplificado) com controle adaptativo opcional (AIMD).
///
/// * fixo: cada chamada reserva o próximo horário livre a `1/rps` do anterior;
/// * adaptativo: começa em `rps_inicial`; a cada HTTP 429 corta a taxa pela metade (no máximo um
///   corte por segundo) e pausa todas as requisições por 2 s; a cada segundo sem 429 (e 3 s depois
///   do último corte) soma `passo` req/s, até `rps_max`. É o mesmo princípio do controle de
///   congestionamento do TCP: o coletor descobre sozinho a taxa que o CDN aceita naquele momento.
pub struct Limitador { adaptativo: bool, rps_max: f64, passo: f64, t0: Instant, e: Mutex<EstLim>, inicios: Mutex<Vec<u32>> }
/// `cota`: limite abaixo do teto ajustado em execução (federação: a fatia do IP de saída que cabe a este nó).
struct EstLim { prox: Instant, rps: f64, cota: f64, pausa_ate: Instant, ultimo_corte: Instant, ultimo_aumento: Instant, historico: Vec<(f64, f64)> }

impl Limitador {
    pub fn novo(rps: f64) -> Self { Self::com(rps, false, rps) }
    pub fn com(rps_max: f64, adaptativo: bool, rps_inicial: f64) -> Self { Self::com_teto(rps_max, TETO_RPS, adaptativo, rps_inicial) }
    /// Como [`Limitador::com`], com outro teto (só para servidor local: ver [`Rede::para`]).
    fn com_teto(rps_max: f64, teto: f64, adaptativo: bool, rps_inicial: f64) -> Self {
        let rps_max = if rps_max <= 0.0 { teto } else { rps_max.min(teto) };
        let agora = Instant::now();
        let antes = agora - Duration::from_secs(10);
        let rps = if adaptativo { rps_inicial.min(rps_max).max(1.0) } else { rps_max };
        Limitador { adaptativo, rps_max, passo: (rps_max / 100.0).clamp(1.0, 10.0), t0: agora,
            e: Mutex::new(EstLim { prox: agora, rps, cota: rps_max, pausa_ate: antes, ultimo_corte: antes, ultimo_aumento: agora, historico: vec![(0.0, rps)] }),
            inicios: Mutex::new(vec![]) }
    }
    pub async fn esperar(&self) {
        let alvo = loop {
            let alvo = {
                let mut e = self.e.lock().unwrap();
                if e.rps <= 0.0 { return; }
                let agora = Instant::now();
                let base = if e.pausa_ate > agora { e.pausa_ate } else { agora };
                // sem rajadas: o intervalo mínimo entre duas requisições é sempre 1/rps
                let t = e.prox.max(base);
                e.prox = t + Duration::from_secs_f64(1.0 / e.rps);
                t
            };
            if alvo > Instant::now() { tokio::time::sleep_until(alvo.into()).await; }
            // uma pausa decretada enquanto esperava (HTTP 429 de outra requisição) vale para esta também
            let pausa = self.e.lock().unwrap().pausa_ate;
            if pausa <= Instant::now() { break alvo; }
        };
        // registra o segundo em que a requisição parte (prova do teto: nenhum segundo passa de rps)
        let s = (alvo - self.t0).as_secs() as usize;
        let mut v = self.inicios.lock().unwrap();
        if v.len() <= s { v.resize(s + 1, 0); }
        v[s] += 1;
    }
    /// Vez de prioridade baixa (comparação com o TSE, etapa 7): só pega uma vaga quando nenhuma requisição da coleta
    /// está esperando a sua (a próxima vaga do limitador já está livre), olhando a cada 100 ms; depois de
    /// [`ESPERA_BAIXA_MAX`] esperando, entra na fila normal (não fica sem vez numa coleta que nunca para). Conta no
    /// mesmo teto de req/s.
    pub async fn esperar_baixa(&self) {
        let t0 = Instant::now();
        loop {
            let livre = {
                let e = self.e.lock().unwrap();
                let agora = Instant::now();
                e.rps > 0.0 && e.pausa_ate <= agora && e.prox <= agora
            };
            if livre || t0.elapsed() >= ESPERA_BAIXA_MAX { break; }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        self.esperar().await
    }
    /// Pausa global: nenhuma requisição parte antes de `agora + d` (nem as que já estavam esperando a vez).
    pub fn pausar(&self, d: Duration) {
        let mut e = self.e.lock().unwrap();
        let ate = Instant::now() + d;
        if ate > e.pausa_ate { e.pausa_ate = ate; }
    }
    /// Quanto falta da pausa em curso (zero se não houver).
    pub fn pausa_restante(&self) -> Duration { self.e.lock().unwrap().pausa_ate.saturating_duration_since(Instant::now()) }
    pub fn sinal_429(&self) {
        if !self.adaptativo { return; }
        let mut e = self.e.lock().unwrap();
        let agora = Instant::now();
        // não encurta uma pausa maior já decretada (ex.: os 10 min depois de um 429)
        if agora + Duration::from_secs(2) > e.pausa_ate { e.pausa_ate = agora + Duration::from_secs(2); }
        if agora - e.ultimo_corte > Duration::from_secs(1) {
            e.rps = (e.rps * 0.5).max(2.0);
            e.ultimo_corte = agora;
            let (t, r) = ((agora - self.t0).as_secs_f64(), e.rps);
            e.historico.push((t, r));
        }
    }
    pub fn sinal_ok(&self) {
        if !self.adaptativo { return; }
        let mut e = self.e.lock().unwrap();
        let agora = Instant::now();
        let max = self.rps_max.min(e.cota);
        if agora - e.ultimo_aumento >= Duration::from_secs(1) && agora - e.ultimo_corte > Duration::from_secs(3) && e.rps < max {
            e.rps = (e.rps + self.passo).min(max);
            e.ultimo_aumento = agora;
            let (t, r) = ((agora - self.t0).as_secs_f64(), e.rps);
            e.historico.push((t, r));
        }
    }
    pub fn rps_atual(&self) -> f64 { self.e.lock().unwrap().rps }
    /// Limita a taxa a `rps` (nunca acima do teto): a federação chama a cada mudança de quem divide o IP de saída. Sem
    /// AIMD, a taxa passa a ser a cota; com AIMD, só é cortada (e passa a subir até a cota).
    pub fn definir_cota(&self, rps: f64) {
        let mut e = self.e.lock().unwrap();
        let cota = rps.min(self.rps_max).max(0.1);
        if (cota - e.cota).abs() < 1e-9 { return; }
        e.cota = cota;
        e.rps = if self.adaptativo { e.rps.min(cota) } else { cota };
        let (t, r) = ((Instant::now() - self.t0).as_secs_f64(), e.rps);
        e.historico.push((t, r));
    }
    pub fn cota(&self) -> f64 { self.e.lock().unwrap().cota }
    /// Requisições iniciadas em cada segundo desde a criação do limitador.
    pub fn inicios_por_segundo(&self) -> Vec<u32> { self.inicios.lock().unwrap().clone() }
    pub fn historico(&self) -> Vec<(f64, f64)> { self.e.lock().unwrap().historico.clone() }
}

/// Resposta HTTP. `data`: cabeçalho `Date` (no site simulado, o relógio simulado).
/// `modificado`: cabeçalho `Last-Modified` (para o `If-Modified-Since` seguinte).
pub struct Resposta { pub status: u16, pub corpo: Bytes, pub etag: Option<String>, pub lat: f64, pub tentativas: u32, pub versao: String, pub data: Option<String>, pub modificado: Option<String> }

/// Pedido condicional e prioridade de um GET ([`Rede::get_com`]).
#[derive(Clone, Copy, Debug, Default)]
pub struct Cond<'a> {
    /// If-None-Match
    pub etag: Option<&'a str>,
    /// If-Modified-Since
    pub modificado: Option<&'a str>,
    /// prioridade abaixo da coleta ([`Limitador::esperar_baixa`]): a comparação com o TSE (etapa 7)
    pub baixa: bool,
}

pub struct Rede {
    clientes: Vec<reqwest::Client>,
    pub lim: Limitador,
    rr: AtomicUsize,
    cfg: ConfigRede,
    pub reqs: AtomicU64,
    pub bytes: AtomicU64,
    pub tentativas_extras: AtomicU64,
    pub http429: AtomicU64,
    pub nao_modificado: AtomicU64,
    pub versao: Mutex<String>,
    /// Disjuntor acionado (excesso de HTTP 429): nenhuma requisição nova é feita.
    pub parado: std::sync::atomic::AtomicBool,
    /// Rede criada para um servidor local com limites afrouxados ([`Rede::para`]): só aceita URLs desse servidor.
    restrita_a: Option<String>,
    /// Pausas de 429 decretadas
    pub pausas: AtomicU64,
}

impl Rede {
    /// Rede para uma base, com os limites de [`limites`]: contra o TSE, teto de 100 req/s e pausa ≥ 10 min depois de um
    /// 429, qualquer que seja `cfg`; contra um servidor local, o teto e a pausa pedidos em `cfg` — e então a rede recusa
    /// URLs de outro servidor.
    pub fn para(base: &str, cfg: &ConfigRede) -> anyhow::Result<Rede> {
        let mut c = cfg.clone();
        let local = base_local(base);
        if !local {
            c.teto = c.teto.min(TETO_RPS);
            c.rps = c.rps.min(TETO_RPS);
            c.pausa_429_s = c.pausa_429_s.max(PAUSA_429_TSE_S);
        }
        let mut r = Rede::criar(&c, if local { c.teto.max(TETO_RPS) } else { TETO_RPS })?;
        if local && c.teto > TETO_RPS { r.restrita_a = Some(base.trim_end_matches('/').to_string() + "/"); }
        Ok(r)
    }

    pub fn nova(cfg: &ConfigRede) -> anyhow::Result<Rede> { Rede::criar(cfg, TETO_RPS) }

    fn criar(cfg: &ConfigRede, teto: f64) -> anyhow::Result<Rede> {
        let mut clientes = vec![];
        for _ in 0..cfg.conexoes.max(1) {
            let mut b = reqwest::Client::builder()
                .user_agent(USER_AGENT)
                .gzip(true)
                .tcp_nodelay(true)
                .timeout(Duration::from_secs(cfg.timeout_s))
                .connect_timeout(Duration::from_secs(10))
                .pool_max_idle_per_host(cfg.concorrencia.max(8));
            if let Some(ip) = cfg.ip_local { b = b.local_address(ip); }
            b = if cfg.http1 { b.http1_only() } else { b.http2_adaptive_window(true) };
            clientes.push(b.build()?);
        }
        let teto_lim = if cfg.teto > 0.0 { cfg.teto.min(teto) } else { TETO_RPS };
        Ok(Rede {
            clientes, lim: Limitador::com_teto(cfg.rps, teto_lim, cfg.adaptativo, cfg.rps_inicial), rr: AtomicUsize::new(0), cfg: cfg.clone(),
            reqs: AtomicU64::new(0), bytes: AtomicU64::new(0), tentativas_extras: AtomicU64::new(0),
            http429: AtomicU64::new(0), nao_modificado: AtomicU64::new(0), versao: Mutex::new(String::new()),
            parado: std::sync::atomic::AtomicBool::new(false), restrita_a: None, pausas: AtomicU64::new(0),
        })
    }

    /// Teto de req/s efetivo do limitador.
    pub fn teto(&self) -> f64 { self.lim.rps_max }

    /// GET com novas tentativas (erro de rede, 429 e 5xx) e espera exponencial.
    /// `etag`: envia If-None-Match (resposta 304 = não mudou). `faixa`: cabeçalho Range.
    pub async fn get(&self, url: &str, etag: Option<&str>, faixa: Option<(u64, u64)>) -> anyhow::Result<Resposta> {
        self.get_com(url, Cond { etag, ..Cond::default() }, faixa).await
    }

    /// Como [`Rede::get`], com `If-Modified-Since` e a prioridade baixa (ver [`Cond`]).
    pub async fn get_com(&self, url: &str, c: Cond<'_>, faixa: Option<(u64, u64)>) -> anyhow::Result<Resposta> {
        let etag = c.etag;
        if let Some(b) = &self.restrita_a {
            anyhow::ensure!(url.starts_with(b.as_str()), "{url}: esta rede tem limites de servidor local e só aceita {b}");
        }
        let mut ultimo_erro = String::new();
        for k in 0..self.cfg.tentativas.max(1) {
            if self.parado.load(Relaxed) { anyhow::bail!("{url}: coleta interrompida pelo disjuntor de HTTP 429"); }
            if k > 0 {
                self.tentativas_extras.fetch_add(1, Relaxed);
                let jit = (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().subsec_nanos() % 200) as u64;
                tokio::time::sleep(Duration::from_millis(250 * (1 << k.min(5)) + jit)).await;
            }
            if c.baixa { self.lim.esperar_baixa().await; } else { self.lim.esperar().await; }
            if self.parado.load(Relaxed) { anyhow::bail!("{url}: coleta interrompida pelo disjuntor de HTTP 429"); }
            let cli = &self.clientes[self.rr.fetch_add(1, Relaxed) % self.clientes.len()];
            let mut rq = cli.get(url);
            if let Some(e) = etag { rq = rq.header("If-None-Match", e); }
            if let Some(m) = c.modificado { rq = rq.header("If-Modified-Since", m); }
            if let Some((a, b)) = faixa { rq = rq.header("Range", format!("bytes={a}-{b}")); }
            let t0 = Instant::now();
            self.reqs.fetch_add(1, Relaxed);
            match rq.send().await {
                Ok(r) => {
                    let st = r.status().as_u16();
                    let versao = format!("{:?}", r.version());
                    let et = r.headers().get("etag").and_then(|v| v.to_str().ok()).map(String::from);
                    let data = r.headers().get("date").and_then(|v| v.to_str().ok()).map(String::from);
                    let modificado = r.headers().get("last-modified").and_then(|v| v.to_str().ok()).map(String::from);
                    if st == 429 || st >= 500 {
                        if st == 429 {
                            let n = self.http429.fetch_add(1, Relaxed) + 1;
                            // pausa global: nenhuma requisição nova (de nenhuma tarefa) até o fim dela
                            if self.cfg.pausa_429_s > 0.0 {
                                if self.lim.pausa_restante().is_zero() { self.pausas.fetch_add(1, Relaxed); }
                                self.lim.pausar(Duration::from_secs_f64(self.cfg.pausa_429_s));
                            }
                            self.lim.sinal_429();
                            if self.cfg.max_429 > 0 && n >= self.cfg.max_429 && !self.parado.swap(true, Relaxed) {
                                eprintln!("\n!! {n} respostas HTTP 429: coleta INTERROMPIDA (disjuntor). Espere ao menos 10 min antes de retomar; a retomada pula o que já foi coletado.");
                            }
                        }
                        ultimo_erro = format!("HTTP {st}");
                        let _ = r.bytes().await;
                        continue;
                    }
                    match r.bytes().await {
                        Ok(corpo) => {
                            self.bytes.fetch_add(corpo.len() as u64, Relaxed);
                            if st == 304 { self.nao_modificado.fetch_add(1, Relaxed); }
                            self.lim.sinal_ok();
                            {
                                let mut v = self.versao.lock().unwrap();
                                if v.is_empty() { *v = versao.clone(); }
                            }
                            return Ok(Resposta { status: st, corpo, etag: et, lat: t0.elapsed().as_secs_f64(), tentativas: k + 1, versao, data, modificado });
                        }
                        Err(e) => { ultimo_erro = format!("corpo: {e}"); }
                    }
                }
                Err(e) => { ultimo_erro = format!("{e}"); }
            }
        }
        anyhow::bail!("{url}: {ultimo_erro}")
    }
}

/// BU recebido, pronto para o núcleo.
pub struct Recebido { pub reg: Registro, pub bu: Bytes }

/// Medida de uma seção coletada (uma linha do CSV de métricas).
#[derive(Clone, Debug, Default)]
pub struct Medida {
    pub t_fim: f64,
    pub uf: String,
    pub municipio: u32,
    pub zona: u16,
    pub secao: u16,
    pub lat_aux: f64,
    pub lat_bu: f64,
    pub bytes_bu: usize,
    pub tentativas: u32,
    pub situacao: String,
    pub recebido_tse: String,
    pub disponivel_cs: String,
    pub erro: String,
}
impl Medida {
    pub const CAB_CSV: &'static str = "t_s,uf,municipio,zona,secao,lat_aux_ms,lat_bu_ms,bytes_bu,tentativas,situacao,recebido_tse,disponivel_cs,erro";
    pub fn csv(&self) -> String {
        format!("{:.4},{},{},{},{},{:.2},{:.2},{},{},{},{},{},{}", self.t_fim, self.uf, self.municipio, self.zona, self.secao,
            self.lat_aux * 1e3, self.lat_bu * 1e3, self.bytes_bu, self.tentativas, self.situacao, self.recebido_tse,
            self.disponivel_cs, self.erro.replace(',', ";"))
    }
}

/// Coleta uma seção pela API de resultados: aux.json → escolhe o hash → …-bu.dat.
/// `so_aux`: lê só o aux.json (situação e horário de recebimento no TSE, para o replay) e não baixa o BU —
/// para quando os BUs já vieram do zip de dados abertos.
pub async fn coletar_secao(rede: &Rede, alvo: &Alvo, s: &Secao, t0: Instant, so_aux: bool) -> (Medida, Option<Recebido>) {
    let uf = s.uf_str();
    let mut m = Medida { uf: uf.to_string(), municipio: s.municipio, zona: s.zona, secao: s.secao, disponivel_cs: s.disponivel.clone(), ..Default::default() };
    let r = match rede.get(&alvo.url_aux(uf, s.municipio, s.zona, s.secao), None, None).await {
        Ok(r) => r,
        Err(e) => { m.erro = format!("aux: {e}"); m.t_fim = t0.elapsed().as_secs_f64(); return (m, None); }
    };
    m.lat_aux = r.lat;
    m.tentativas = r.tentativas;
    if r.status != 200 { m.erro = format!("aux HTTP {}", r.status); m.t_fim = t0.elapsed().as_secs_f64(); return (m, None); }
    let aux: Aux = match serde_json::from_slice(&r.corpo) {
        Ok(a) => a,
        Err(e) => { m.erro = format!("aux json: {e}"); m.t_fim = t0.elapsed().as_secs_f64(); return (m, None); }
    };
    let Some((hash, nome, st, receb)) = aux.escolher_bu() else {
        m.erro = format!("sem BU (situação {})", aux.st);
        m.t_fim = t0.elapsed().as_secs_f64();
        return (m, None);
    };
    m.situacao = st.to_string();
    m.recebido_tse = receb;
    if so_aux { m.t_fim = t0.elapsed().as_secs_f64(); return (m, None); }
    let b = match rede.get(&alvo.url_arquivo(uf, s.municipio, s.zona, s.secao, hash, nome), None, None).await {
        Ok(b) => b,
        Err(e) => { m.erro = format!("bu: {e}"); m.t_fim = t0.elapsed().as_secs_f64(); return (m, None); }
    };
    m.lat_bu = b.lat;
    m.tentativas += b.tentativas;
    m.t_fim = t0.elapsed().as_secs_f64();
    if b.status != 200 { m.erro = format!("bu HTTP {}", b.status); return (m, None); }
    m.bytes_bu = b.corpo.len();
    let reg = Registro {
        pleito: alvo.pleito, uf: s.uf, municipio: s.municipio, zona: s.zona, secao: s.secao,
        situacao: st.starts_with("Totalizad") as u8, origem: 1, sha256_vsc: tse::sha256_do_hash_dir(hash),
    };
    (m, Some(Recebido { reg, bu: b.corpo }))
}

/// Coleta um conjunto de seções com até `concorrencia` em andamento, entregando cada BU em `tx`.
pub async fn coletar(rede: Arc<Rede>, alvo: Arc<Alvo>, secoes: Vec<Secao>, concorrencia: usize,
                     tx: crossbeam_channel::Sender<Recebido>, t0: Instant, progresso: bool, so_aux: bool) -> Vec<Medida> {
    let total = secoes.len();
    let feitas = Arc::new(AtomicUsize::new(0));
    let ultimo = Arc::new(Mutex::new(Instant::now()));
    futures::stream::iter(secoes.into_iter().map(|s| {
        let (rede, alvo, tx, feitas, ultimo) = (rede.clone(), alvo.clone(), tx.clone(), feitas.clone(), ultimo.clone());
        async move {
            let (m, r) = coletar_secao(&rede, &alvo, &s, t0, so_aux).await;
            if let Some(r) = r { let _ = tx.send(r); }
            let f = feitas.fetch_add(1, Relaxed) + 1;
            if progresso {
                let mut u = ultimo.lock().unwrap();
                if u.elapsed() > Duration::from_secs(5) || f == total {
                    *u = Instant::now();
                    let t = t0.elapsed().as_secs_f64();
                    eprintln!("  [{t:7.1} s] {f}/{total} seções ({:.0}/s, {:.1} MB, teto atual {:.0} req/s, 429: {})", f as f64 / t,
                        rede.bytes.load(Relaxed) as f64 / 1e6, rede.lim.rps_atual(), rede.http429.load(Relaxed));
                }
            }
            m
        }
    })).buffer_unordered(concorrencia.max(1)).collect().await
}

/// Lê as listas de seções (cs.json) das UFs. Devolve (uf, config) na ordem pedida.
pub async fn ler_configs(rede: &Rede, alvo: &Alvo, ufs: &[String], cache: Option<&std::path::Path>) -> anyhow::Result<Vec<(String, tse::ConfigSecoes)>> {
    let tarefas = ufs.iter().map(|uf| async move {
        let r = rede.get(&alvo.url_cs(uf), None, None).await?;
        anyhow::ensure!(r.status == 200, "cs {uf}: HTTP {}", r.status);
        if let Some(d) = cache {
            std::fs::create_dir_all(d)?;
            std::fs::write(d.join(format!("{uf}-p{:06}-cs.json", alvo.pleito)), &r.corpo)?;
        }
        let c: tse::ConfigSecoes = serde_json::from_slice(&r.corpo)?;
        Ok::<_, anyhow::Error>((uf.clone(), c))
    });
    futures::future::join_all(tarefas).await.into_iter().collect()
}

/// Percentil (0..100) de uma lista já ordenada.
pub fn percentil(v: &[f64], p: f64) -> f64 {
    if v.is_empty() { return 0.0; }
    let i = ((p / 100.0) * (v.len() - 1) as f64).round() as usize;
    v[i.min(v.len() - 1)]
}

#[cfg(test)]
mod testes {
    use super::*;
    #[tokio::test]
    async fn nunca_passa_do_teto() {
        let l = Limitador::com(5000.0, false, 0.0); // pedido acima do teto é reduzido a TETO_RPS
        let t0 = Instant::now();
        for _ in 0..51 { l.esperar().await; }
        // 51 requisições a no máximo 100/s ocupam pelo menos 0,5 s
        assert!(t0.elapsed() >= Duration::from_millis(495), "{:?}", t0.elapsed());
        let a = Limitador::com(1000.0, true, 500.0);
        assert!(a.rps_atual() <= TETO_RPS);
    }

    /// Só 127.0.0.1/localhost/::1 são servidores locais; nada que só pareça local.
    #[test]
    fn servidor_local() {
        for b in ["http://127.0.0.1:8080", "http://localhost", "http://LOCALHOST:1/x", "http://[::1]:9000/oficial", "127.0.0.1:5"] { assert!(base_local(b), "{b}"); }
        for b in ["https://resultados.tse.jus.br", "http://127.0.0.1.nip.io", "http://localhost@tse.jus.br", "http://localhost.tse.jus.br:80",
            "http://10.0.0.1", "http://[::2]:80", "http://0.0.0.0"] { assert!(!base_local(b), "{b}"); }
    }

    /// Contra o TSE: teto 100, req/s ≤ 100, pausa ≥ 600 s, aceleração recusada — qualquer que seja o pedido.
    /// Contra um servidor local: teto e pausa escalados pela aceleração.
    #[test]
    fn limites_nao_afrouxam_contra_o_tse() {
        let tse = "https://resultados.tse.jus.br";
        assert_eq!(limites(tse, 95.0, 1.0).unwrap(), (100.0, 95.0, 600.0));
        assert_eq!(limites(tse, 5000.0, 1.0).unwrap(), (100.0, 100.0, 600.0));
        assert_eq!(limites(tse, 0.0, 0.5).unwrap(), (100.0, 95.0, 600.0));
        assert!(limites(tse, 95.0, 10.0).is_err());
        assert!(limites("http://127.0.0.1.nip.io:80", 95.0, 10.0).is_err());
        assert_eq!(limites("http://127.0.0.1:1", 95.0, 10.0).unwrap(), (1000.0, 950.0, 60.0));
        // Rede::para contra o TSE ignora teto e pausa afrouxados na configuração
        let mut c = ConfigRede::simples(4, 5000.0, 1, 5, 0);
        c.teto = 5000.0;
        c.pausa_429_s = 1.0;
        let r = Rede::para(tse, &c).unwrap();
        assert_eq!(r.teto(), TETO_RPS);
        assert_eq!(r.cfg.pausa_429_s, PAUSA_429_TSE_S);
        assert!(r.restrita_a.is_none());
        // contra um servidor local com teto acima de 100, a rede só aceita URLs desse servidor
        let l = Rede::para("http://127.0.0.1:1", &c).unwrap();
        assert_eq!(l.teto(), 5000.0);
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        assert!(rt.block_on(l.get("https://resultados.tse.jus.br/oficial/ele2026/x", None, None)).is_err());
    }
}
