//! `uv-simulador`: o site de resultados do TSE, simulado em `127.0.0.1`, para provar o `uv-servico` sem baixar BUs do
//! TSE (os BUs já estão nos pacotes locais e não podem ser baixados de novo).
//!
//! Serve exatamente os caminhos do site real (ver [`crate::tse`]): `cs.json` de cada UF reconstruído no instante
//! simulado (uma seção só ganha `da`/`ha` depois do seu horário de disponibilização; `ETag` muda quando o conteúdo
//! muda e `If-None-Match` devolve 304), `aux.json` de cada seção (404 antes da disponibilização; "Nao instalada" sem
//! BU; "Anulada"/"Excluído"), o BU (`-bu.dat`, os bytes do `.uvpk`) e os `-u.json` oficiais depois do último BU. O
//! relógio é acelerado (`--acelerar`) e o cabeçalho `Date` de toda resposta traz a hora simulada. Controle do ensaio
//! (caminhos que não existem no site real; o `uv-servico --controle-replay` os usa): `POST /controle/velocidade?x=N`
//! muda a aceleração sem salto no relógio (0 = parado), e com `&ate=T&depois=M` anda a `N`× até o instante `T` e lá
//! passa a `M`× (avanço rápido que para exatamente no ponto); `POST /controle/posicao?t=T[&x=N]` põe o relógio em `T`
//! (para voltar ao começo da noite); `GET /controle` dá aceleração, relógio, começo e fim da noite e o alvo.
//!
//! Horário de disponibilização de cada seção: `disponivel_cs` das métricas da coleta; quando ele não é da noite (o
//! `cs.json` de 2024 foi republicado em 29/09/2026) ou falta, `recebido_tse`; sem nenhum dos dois, o fim da noite.
//!
//! Falhas por parâmetro: rajadas de HTTP 429, latência e erros 5xx aleatórios (semente fixa), reenvio de uma seção
//! com o mesmo BU (hash novo), anulação de uma seção depois de publicada e queda total do servidor. Cada requisição
//! vai para o `simulador.log` (`t_ms,simulado,tipo,status,bytes,caminho,ip`), a prova das taxas e das contagens.
//!
//! Federação (2.0, ADR-028): o IP de origem de cada requisição é registrado; `GET /eco-ip` devolve esse IP (a medida
//! do IP de saída de cada nó); com `--limite-ip-rps`, cada IP tem o seu limite e a sua caixa de penalidade, como na
//! borda do TSE, e o resumo traz as contagens por IP.

use crate::tse::{self, ConfigSecoes};
use bytes::Bytes;
use http_body_util::Full;
use hyper::{Request, Response, StatusCode};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use urna_velox::metricas::{segundos, Metricas};
use urna_velox::pacote::EmMemoria;

pub type ChaveUf = ([u8; 2], u64);

/// Falhas pedidas (texto dos parâmetros; os horários são resolvidos depois de carregar os dados).
#[derive(Clone, Debug, Default)]
pub struct Falhas {
    /// "hh:mm:ss+N" (ou "dd/mm/aaaa hh:mm:ss+N"): todas as requisições respondem 429 por N segundos simulados
    pub rajadas_429: Vec<String>,
    /// "a-b": latência uniforme entre a e b ms
    pub latencia_ms: String,
    /// probabilidade de responder 503
    pub erros_5xx: f64,
    pub semente: u64,
    /// "uf:município/zona/seção@hh:mm:ss": a seção ganha um hash novo com o mesmo BU (reenvio idêntico)
    pub reenvios: Vec<String>,
    /// "uf:município/zona/seção@hh:mm:ss": a seção passa a "Anulada" (BU "Excluído")
    pub anulacoes: Vec<String>,
    /// "hh:mm:ss+N": o servidor sai do ar (conexões recusadas) por N segundos **reais**
    pub quedas: Vec<String>,
}

pub struct Config {
    /// pacotes `.uvpk` (os BUs servidos)
    pub dados: PathBuf,
    /// `cs.json` originais (`<uf>-p<pleito>-cs.json`): municípios, zonas e seções de cada UF
    pub cs: PathBuf,
    /// métricas da coleta (disponibilização, recebimento, anuladas, não instaladas)
    pub metricas: Vec<PathBuf>,
    /// `-u.json` oficiais (servidos depois do último BU)
    pub oficial: Option<PathBuf>,
    pub pleito: u32,
    pub ciclo: String,
    pub ambiente: String,
    pub ufs: Option<Vec<String>>,
    pub porta: u16,
    pub acelerar: f64,
    /// início do relógio simulado ("dd/mm/aaaa hh:mm:ss"); padrão: 1 min antes da primeira disponibilização
    pub inicio: Option<String>,
    /// disponibilizações depois deste horário entram nele (BUs reenviados dias depois)
    pub corte: Option<String>,
    /// piso dos replays ("hh:mm:ss" no dia da eleição, ou "dd/mm/aaaa hh:mm:ss"): o que saiu antes dele entra nele
    /// ([`com_piso`]: os BUs do exterior da madrugada entram juntos às 17h)
    pub piso: Option<String>,
    /// horário de cada seção = recebimento no TSE (`recebido_tse`), mesmo quando há `disponivel_cs` da noite: para
    /// conjuntos cujo `cs.json` foi coletado depois da noite (2026, 1º turno: disponibilização 4,5 h depois do
    /// recebimento na mediana, UFs republicadas inteiras)
    pub horario_recebido: bool,
    pub log: PathBuf,
    pub falhas: Falhas,
    /// Limite por IP de origem, em req/s da noite (multiplicado pela aceleração), como a CDN do TSE faz por órgão
    /// (2.0, ADR-028): acima dele, 429 e "caixa de penalidade" do IP por 600 s simulados. 0 = sem limite.
    pub limite_ip_rps: f64,
    /// TSE simulado (etapa 7, [`crate::tse_simulado`]): `-u.json` progressivos durante a noite
    pub tse: Option<crate::tse_simulado::Opcoes>,
    /// Acompanhar o serviço (B-011, modo rápido do ciclo): o relógio não passa do instante em que as seções
    /// disponibilizadas e ainda sem o BU servido seriam mais que esta % das seções com BU; 0 = desligado (o relógio
    /// anda solto, como antes)
    pub acompanhar_pct: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ev { Reenvio(u8), Anulacao }

struct Sec {
    uf: [u8; 2],
    mu: u32,
    zn: u16,
    se: u16,
    /// disponibilização (None = a seção nunca aparece com `da`/`ha`)
    disp: Option<i64>,
    receb: Option<i64>,
    /// índice do BU em `mem.itens`
    bu: Option<u32>,
    hash: String,
    situacao: &'static str,
    anulada: bool,
    nao_instalada: bool,
    /// seção principal, se esta for agregada (`nsp` do `cs.json` original, repetido no servido)
    nsp: String,
    eventos: Vec<(i64, Ev)>,
}

impl Sec {
    /// Disponibilização vista no `cs.json` no instante `t` (a original ou a de um reenvio/anulação já ocorrido).
    fn disp_em(&self, t: i64) -> Option<i64> {
        let mut d = self.disp.filter(|&x| x <= t)?;
        for &(te, _) in &self.eventos { if te <= t && te > d { d = te; } }
        Some(d)
    }
    fn anulada_em(&self, t: i64) -> bool { self.anulada || self.eventos.iter().any(|&(te, e)| e == Ev::Anulacao && te <= t) }
    fn nome_bu(&self, pleito: u32) -> String {
        format!("o{pleito:05}{}{:05}{:04}{:04}-bu.dat", String::from_utf8_lossy(&self.uf), self.mu, self.zn, self.se)
    }
    /// Hashes publicados no instante `t`: (hash, recebido, é reenvio)
    fn hashes_em(&self, t: i64) -> Vec<(String, i64, bool)> {
        let mut v = vec![(self.hash.clone(), self.receb.or(self.disp).unwrap_or(t), false)];
        for &(te, e) in &self.eventos {
            if let Ev::Reenvio(n) = e { if te <= t { v.push((hash_reenvio(&self.hash, n), te, true)); } }
        }
        v
    }
}

fn hash_reenvio(h: &str, n: u8) -> String {
    let mut s = Sha256::new();
    s.update(h.as_bytes());
    s.update([n]);
    tse::hash_dir_do_sha256(&s.finalize().into())
}

struct UfSim {
    uf: String,
    /// (código, nome, zonas (código, seções))
    muns: Vec<(String, String, Vec<(String, Vec<u32>)>)>,
    /// instantes em que o `cs.json` da UF muda (disponibilizações e eventos), em ordem
    mudancas: Vec<i64>,
    cache: Mutex<Option<(usize, Bytes, String)>>,
}

/// Relógio simulado: `base_sim + (agora − base_real) · acelerar`. Mudar a aceleração recomeça a conta do instante
/// atual, então o relógio nunca salta nem volta (só [`Relogio::posicionar`] salta, a pedido). Com um `alvo`
/// `(instante, depois)`, ao chegar ao instante o relógio para nele e passa à aceleração `depois`. O relógio para
/// sozinho em `limite` (6 h depois dos oficiais: a 300× ele não corre dias à frente com a noite já terminada).
struct Relogio { base_sim: f64, base_real: Instant, acelerar: f64, alvo: Option<(f64, f64)>, limite: f64 }

impl Relogio {
    /// Hora simulada; aplica o alvo se já chegou nele (devolve também se chegou agora).
    fn ler(&mut self) -> (f64, bool) {
        let t = self.base_sim + self.base_real.elapsed().as_secs_f64() * self.acelerar;
        if t > self.limite && self.alvo.is_none_or(|(a, _)| a > self.limite) {
            self.posicionar(self.limite);
            self.acelerar = 0.0;
            return (self.limite, true);
        }
        match self.alvo {
            Some((a, depois)) if t >= a => {
                // o instante real em que o relógio passou pelo alvo; dali em diante, a `depois`×
                let real = if self.acelerar > 0.0 { Duration::from_secs_f64(((a - self.base_sim) / self.acelerar).max(0.0)) } else { Duration::ZERO };
                self.base_real += real;
                self.base_sim = a;
                self.acelerar = depois;
                self.alvo = None;
                (self.base_sim + self.base_real.elapsed().as_secs_f64() * self.acelerar, true)
            }
            _ => (t, false),
        }
    }
    fn agora(&mut self) -> f64 { self.ler().0 }
    fn mudar(&mut self, x: f64) {
        self.base_sim = self.agora();
        self.base_real = Instant::now();
        self.acelerar = x;
    }
    fn posicionar(&mut self, t: f64) {
        self.base_sim = t;
        self.base_real = Instant::now();
        self.alvo = None;
    }
}

/// Aceleração máxima aceita pelo controle (`POST /controle/velocidade`).
pub const MAX_ACELERAR: f64 = 1e6;

/// Corpo de `GET /controle`: `t`, `inicio`, `fim` e `alvo` em segundos (hora de Brasília, escala de
/// `urna_velox::metricas::segundos`), `relogio` em texto.
fn json_controle(x: f64, t: f64, inicio: i64, fim: i64, alvo: Option<(f64, f64)>) -> String {
    let alvo = match alvo { Some((a, m)) => format!("{{\"t\":{},\"depois\":{m}}}", a as i64), None => "null".into() };
    format!("{{\"acelerar\":{x},\"relogio\":\"{}\",\"t\":{},\"inicio\":{inicio},\"fim\":{fim},\"alvo\":{alvo}}}", crate::texto_hora(t as i64), t as i64)
}

/// Contadores por tipo de requisição e por segundo real.
#[derive(Default)]
struct Estatistica {
    por_tipo: BTreeMap<(String, u16), u64>,
    por_segundo: Vec<u32>,
    por_ip: BTreeMap<std::net::IpAddr, PorIp>,
}

/// Contagem de um IP de origem: segundo real corrente e quantas requisições nele, penalidade (hora simulada), totais.
#[derive(Default, Clone)]
struct PorIp { seg: usize, n: u32, max_seg: u32, total: u64, n429: u64, penalidade_ate: f64, penalidades: u64 }

/// Pausa da "caixa de penalidade" por IP, em segundos simulados (como os 10 min da borda do TSE).
pub const PENALIDADE_IP_S: f64 = 600.0;

pub struct Sim {
    secs: Vec<Sec>,
    idx: HashMap<ChaveUf, u32>,
    ufs: HashMap<String, UfSim>,
    mem: EmMemoria,
    oficiais: HashMap<String, PathBuf>,
    pub t_oficiais: i64,
    pub inicio: i64,
    pub primeira: i64,
    pub ultima: i64,
    relogio: Mutex<Relogio>,
    /// início real do ensaio (contagem de requisições por segundo)
    t0: Mutex<Instant>,
    pleito: u32,
    ambiente: String,
    ciclo: String,
    rajadas: Vec<(i64, i64)>,
    quedas: Vec<(i64, f64)>,
    lat: (u64, u64),
    p5xx: f64,
    semente: u64,
    n_req: AtomicU64,
    log: Mutex<std::io::BufWriter<std::fs::File>>,
    est: Mutex<Estatistica>,
    caido: AtomicBool,
    pub resumo_carga: String,
    limite_ip_rps: f64,
    /// TSE simulado: `-u.json` progressivos (sem ele, os oficiais só depois do último BU)
    tse: Option<crate::tse_simulado::TseSimulado>,
    /// acompanhar o serviço (B-011): folga em seções (0 = desligado), disponibilizações das seções com BU em ordem, BU
    /// já servido por seção e quantas seções já tiveram o BU servido
    folga: u64,
    disps: Vec<i64>,
    servida: Vec<AtomicBool>,
    n_servidas: AtomicU64,
    freadas: AtomicU64,
    /// ms (desde `criado`) do último BU servido pela primeira vez: sem BU novo há mais de `FREIO_MAX`, o relógio volta a
    /// andar solto (seção que o serviço nunca busca não trava a noite)
    ult_servida_ms: AtomicU64,
    criado: Instant,
}

/// Tempo real máximo com o relógio freado sem nenhum BU novo servido (acompanhar o serviço, B-011).
const FREIO_MAX: Duration = Duration::from_secs(10);

/// "hh:mm:ss" (no dia de `dia_ref`) ou "dd/mm/aaaa hh:mm:ss".
pub fn hora(s: &str, dia_ref: i64) -> Option<i64> {
    let s = s.trim();
    if s.contains('/') { return segundos(s); }
    let mut p = s.split(':').map(|x| x.parse::<i64>().ok());
    let (h, m, x) = (p.next()??, p.next()??, p.next().flatten().unwrap_or(0));
    Some(dia_ref.div_euclid(86400) * 86400 + h * 3600 + m * 60 + x)
}

/// Piso dos replays (pedido do Luciano em 23/10/2026): o horário de uma seção publicada (ou recebida) antes do piso — os
/// BUs do exterior, que o TSE publica a partir de ~1h da madrugada — passa a ser o piso, e todas entram juntas no
/// começo da noite, em vez de uma madrugada em tempo real. `horario`: o escolhido para a seção (disponibilização ou
/// recebimento); `disponivel_cs`: a disponibilização no `cs.json` (o recebimento do `aux.json` dessas seções vem
/// agrupado às 17:08, B-049).
pub fn com_piso(horario: Option<i64>, disponivel_cs: Option<i64>, piso: Option<i64>) -> Option<i64> {
    match (horario, piso) {
        (Some(h), Some(p)) if h < p || disponivel_cs.is_some_and(|d| d < p) => Some(p),
        _ => horario,
    }
}

/// "uf:município/zona/seção" → chave
pub fn secao_de_texto(s: &str) -> Option<ChaveUf> {
    let (uf, r) = s.trim().split_once(':')?;
    let uf: [u8; 2] = uf.to_ascii_lowercase().as_bytes().try_into().ok()?;
    let p: Vec<&str> = r.split('/').collect();
    if p.len() != 3 { return None; }
    Some((uf, tse::chave(p[0].parse().ok()?, p[1].parse().ok()?, p[2].parse().ok()?)))
}

fn splitmix(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E3779B97F4A7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D049BB133111EB);
    x ^ (x >> 31)
}

fn sha256(b: &[u8]) -> [u8; 32] { Sha256::digest(b).into() }

impl Sim {
    /// Lê pacotes, `cs.json`, métricas e oficiais e monta o site simulado (o relógio só começa em [`servir`]).
    pub fn carregar(cfg: &Config) -> anyhow::Result<Sim> {
        let t_carga = Instant::now();
        let quer = |uf: &str| cfg.ufs.as_ref().is_none_or(|v| v.iter().any(|x| x == uf));
        let ps: Vec<&std::path::Path> = cfg.metricas.iter().map(|p| p.as_path()).collect();
        let met = Metricas::ler(&ps)?;
        // dia da eleição: o dia mais comum entre os recebimentos; disponibilização fora da noite (republicação) não vale
        let mut dias: HashMap<i64, usize> = HashMap::new();
        for t in met.recebido.values() { *dias.entry(t.div_euclid(86400)).or_default() += 1; }
        let dia0 = dias.into_iter().max_by_key(|x| (x.1, -x.0)).map(|x| x.0 * 86400).ok_or_else(|| anyhow::anyhow!("métricas sem recebido_tse"))?;
        let valida = |t: i64| t >= dia0 - 86400 && t < dia0 + 4 * 86400;
        let mem = EmMemoria::carregar(&cfg.dados, cfg.ufs.as_deref())?;
        let mut bu_de: HashMap<ChaveUf, u32> = HashMap::new();
        for (i, it) in mem.itens.iter().enumerate() { bu_de.entry((it.0.uf, it.0.chave())).or_insert(i as u32); }

        let mut arqs: Vec<PathBuf> = std::fs::read_dir(&cfg.cs)?.flatten().map(|e| e.path())
            .filter(|p| p.file_name().and_then(|x| x.to_str()).is_some_and(|n| n.ends_with(&format!("-p{:06}-cs.json", cfg.pleito)))).collect();
        arqs.sort();
        let piso = match cfg.piso.as_deref() { Some(x) => Some(hora(x, dia0).ok_or_else(|| anyhow::anyhow!("--piso inválido: {x}"))?), None => None };
        let (mut secs, mut idx, mut ufs) = (vec![], HashMap::new(), HashMap::new());
        let (mut republicadas, mut so_receb) = (0usize, 0usize);
        for p in arqs {
            let nome = p.file_name().unwrap().to_string_lossy().into_owned();
            let uf = nome[..2].to_string();
            if !quer(&uf) { continue; }
            let ufb: [u8; 2] = uf.as_bytes().try_into()?;
            let c: ConfigSecoes = serde_json::from_slice(&std::fs::read(&p)?)?;
            let mut muns = vec![];
            for a in &c.abr {
                for m in &a.mu {
                    let mu: u32 = m.cd.parse()?;
                    let mut zonas = vec![];
                    for z in &m.zon {
                        let zn: u16 = z.cd.parse()?;
                        let mut ss = vec![];
                        for s in &z.sec {
                            let se: u16 = s.ns.parse()?;
                            let k = tse::chave(mu, zn, se);
                            let ku = (ufb, k);
                            if idx.contains_key(&ku) { continue; }
                            let bu = bu_de.get(&ku).copied();
                            let receb = met.recebido.get(&ku).copied();
                            let disp_cs = met.disponivel.get(&ku).copied().filter(|&t| valida(t));
                            let disp = match bu {
                                Some(_) if cfg.horario_recebido && receb.is_some() => receb,
                                Some(_) => match met.disponivel.get(&ku).copied() {
                                    Some(t) if valida(t) => Some(t),
                                    Some(_) => { republicadas += 1; receb }
                                    None => { so_receb += receb.is_some() as usize; receb }
                                },
                                None => None,
                            };
                            let disp = com_piso(disp, disp_cs, piso);
                            let (hash, situacao) = match bu {
                                Some(i) => {
                                    let r = &mem.itens[i as usize].0;
                                    let v = if r.sha256_vsc != [0; 32] { r.sha256_vsc } else { sha256(mem.bu(i as usize)) };
                                    (tse::hash_dir_do_sha256(&v), if met.anuladas.contains(&k) { "Excluído" } else if r.situacao == 1 { "Totalizado" } else { "Recebido" })
                                }
                                None => (String::new(), ""),
                            };
                            let id = secs.len() as u32;
                            idx.insert(ku, id);
                            ss.push(id);
                            secs.push(Sec { uf: ufb, mu, zn, se, disp, receb, bu, hash, situacao, anulada: bu.is_some() && met.anuladas.contains(&k),
                                nao_instalada: bu.is_none() && met.nao_instaladas.contains(&ku), nsp: s.nsp.trim().to_string(), eventos: vec![] });
                        }
                        zonas.push((z.cd.clone(), ss));
                    }
                    muns.push((m.cd.clone(), m.nm.clone(), zonas));
                }
            }
            ufs.insert(uf.clone(), UfSim { uf, muns, mudancas: vec![], cache: Mutex::new(None) });
        }
        anyhow::ensure!(!secs.is_empty(), "nenhum cs.json do pleito {} em {}", cfg.pleito, cfg.cs.display());
        let fora = bu_de.keys().filter(|k| !idx.contains_key(*k)).count();
        // sem horário nenhum: entram no fim da noite
        let ultima0 = secs.iter().filter_map(|s| s.disp).max().unwrap_or(dia0);
        let mut sem_horario = 0;
        for s in secs.iter_mut() { if s.bu.is_some() && s.disp.is_none() { s.disp = Some(ultima0 + 1); sem_horario += 1; } }
        let mut cortadas = 0;
        if let Some(c) = &cfg.corte {
            let c = segundos(c).ok_or_else(|| anyhow::anyhow!("--corte inválido: {c}"))?;
            for s in secs.iter_mut() { if let Some(d) = s.disp { if d > c { s.disp = Some(c); cortadas += 1; } } }
        }
        let primeira = secs.iter().filter_map(|s| s.disp).min().unwrap_or(dia0);
        let inicio = match &cfg.inicio { Some(x) => segundos(x).ok_or_else(|| anyhow::anyhow!("--inicio inválido: {x}"))?, None => primeira - 60 };

        // falhas
        let mais = |x: &str| -> anyhow::Result<(i64, f64)> {
            let (h, n) = x.rsplit_once('+').ok_or_else(|| anyhow::anyhow!("falha sem duração (+N): {x}"))?;
            Ok((hora(h, inicio).ok_or_else(|| anyhow::anyhow!("horário inválido: {h}"))?, n.trim().parse()?))
        };
        let mut rajadas = vec![];
        for x in &cfg.falhas.rajadas_429 { let (t, n) = mais(x)?; rajadas.push((t, t + n as i64)); }
        let mut quedas = vec![];
        for x in &cfg.falhas.quedas { quedas.push(mais(x)?); }
        quedas.sort_by(|a, b| a.0.cmp(&b.0));
        for (lista, ev) in [(&cfg.falhas.reenvios, true), (&cfg.falhas.anulacoes, false)] {
            for x in lista.iter() {
                let (s, h) = x.split_once('@').ok_or_else(|| anyhow::anyhow!("falha sem horário (@hh:mm:ss): {x}"))?;
                let k = secao_de_texto(s).ok_or_else(|| anyhow::anyhow!("seção inválida: {s}"))?;
                let t = hora(h, inicio).ok_or_else(|| anyhow::anyhow!("horário inválido: {h}"))?;
                let &i = idx.get(&k).ok_or_else(|| anyhow::anyhow!("seção {s} não está nos cs.json"))?;
                let sec = &mut secs[i as usize];
                anyhow::ensure!(sec.bu.is_some(), "seção {s} não tem BU nos pacotes");
                // antes da disponibilização o evento não seria um reenvio/anulação "depois de publicada": a seção já
                // apareceria com os dois hashes (ou anulada) — o ensaio não provaria nada, então é recusado
                let disp = sec.disp.ok_or_else(|| anyhow::anyhow!("seção {s} sem disponibilização: {x} nunca seria visto"))?;
                anyhow::ensure!(t > disp, "{x}: a seção {s} só é publicada às {} (hora do site); escolha um horário depois disso",
                    crate::texto_hora(disp));
                if !ev {
                    // anulação pedida numa seção que as métricas já dão como anulada (caso real): publicada primeiro como
                    // "Totalizado", anulada no horário pedido
                    sec.anulada = false;
                    if sec.situacao == "Excluído" { sec.situacao = "Totalizado"; }
                }
                let n = sec.eventos.iter().filter(|e| matches!(e.1, Ev::Reenvio(_))).count() as u8 + 1;
                sec.eventos.push((t, if ev { Ev::Reenvio(n) } else { Ev::Anulacao }));
                sec.eventos.sort_by_key(|e| e.0);
            }
        }
        let lat = match cfg.falhas.latencia_ms.split_once('-') {
            Some((a, b)) => (a.trim().parse()?, b.trim().parse()?),
            None if cfg.falhas.latencia_ms.trim().is_empty() => (0, 0),
            None => { let a: u64 = cfg.falhas.latencia_ms.trim().parse()?; (a, a) }
        };

        let mut mud: HashMap<[u8; 2], Vec<i64>> = HashMap::new();
        for s in &secs {
            let v = mud.entry(s.uf).or_default();
            if let Some(d) = s.disp { v.push(d); }
            for &(t, _) in &s.eventos { v.push(t); }
        }
        for u in ufs.values_mut() {
            let mut v = mud.remove(u.uf.as_bytes()).unwrap_or_default();
            v.sort_unstable();
            u.mudancas = v;
        }
        let ultima = secs.iter().flat_map(|s| s.disp.into_iter().chain(s.eventos.iter().map(|e| e.0))).max().unwrap_or(dia0);
        let t_oficiais = ultima + 1;

        let mut oficiais = HashMap::new();
        if let Some(d) = &cfg.oficial {
            for e in std::fs::read_dir(d)?.flatten() {
                let n = e.file_name().to_string_lossy().into_owned();
                let Some((abr, _c, el)) = urna_velox::oficial::nome_arquivo(&n) else { continue };
                let cod = abr.codigo();
                if cfg.ufs.is_some() && cod != "br" && !quer(&cod[..2]) { continue; }
                let dir = if cod == "br" { "br".to_string() } else { cod[..2].to_string() };
                oficiais.insert(format!("/{}/{}/{el}/dados/{dir}/{n}", cfg.ambiente, cfg.ciclo), e.path());
            }
        }
        // TSE simulado: -u.json progressivos, com o atraso da divulgação oficial por seção
        let tse = match &cfg.tse {
            Some(op) => {
                let st: Vec<crate::tse_simulado::SecTse> = secs.iter().map(|s| crate::tse_simulado::SecTse {
                    k: (s.uf, tse::chave(s.mu, s.zn, s.se)), disp: s.bu.and(s.disp), nao_instalada: s.nao_instalada }).collect();
                let nomes: HashMap<String, PathBuf> = oficiais.iter().filter_map(|(p, c)| p.rsplit('/').next().map(|n| (n.to_string(), c.clone()))).collect();
                Some(crate::tse_simulado::TseSimulado::iniciar(op.clone(), cfg.dados.clone(), cfg.ufs.clone(), st, nomes)?)
            }
            None => None,
        };
        let log = std::io::BufWriter::with_capacity(1 << 20, std::fs::File::create(&cfg.log)?);
        let com_bu = secs.iter().filter(|s| s.bu.is_some()).count();
        let resumo_carga = format!(concat!("{} seções em {} UFs ({} com BU, {} não instaladas, {} anuladas desde o início); {} BUs fora dos cs.json; ",
            "disponibilização: {} pela métrica de recebimento (republicadas), {} sem disponibilização com recebimento, {} sem horário (no fim), {} cortadas; ",
            "primeira {} · última {} · oficiais a partir de {} · início do relógio {} · {} oficiais; carga em {:.1} s"),
            secs.len(), ufs.len(), com_bu, secs.iter().filter(|s| s.nao_instalada).count(), secs.iter().filter(|s| s.anulada).count(), fora,
            republicadas, so_receb, sem_horario, cortadas, crate::texto_hora(primeira), crate::texto_hora(ultima), crate::texto_hora(t_oficiais),
            crate::texto_hora(inicio), oficiais.len(), t_carga.elapsed().as_secs_f64())
            + &tse.as_ref().map(|t| format!("; TSE simulado: divulgação até {} (atraso p50 {:.0} s, p90 {:.0} s, máx. {:.0} s)",
                crate::texto_hora(t.t_final), t.atraso.0, t.atraso.1, t.atraso.2)).unwrap_or_default();
        let mut disps: Vec<i64> = secs.iter().filter(|s| s.bu.is_some()).filter_map(|s| s.disp).collect();
        disps.sort_unstable();
        let folga = if cfg.acompanhar_pct > 0.0 { ((disps.len() as f64 * cfg.acompanhar_pct / 100.0).ceil() as u64).max(1) } else { 0 };
        let servida = (0..secs.len()).map(|_| AtomicBool::new(false)).collect();
        Ok(Sim {
            folga, disps, servida, n_servidas: AtomicU64::new(0), freadas: AtomicU64::new(0), ult_servida_ms: AtomicU64::new(0), criado: Instant::now(),
            secs, idx, ufs, mem, oficiais, t_oficiais, inicio, primeira, ultima,
            relogio: Mutex::new(Relogio { base_sim: inicio as f64, base_real: Instant::now(), acelerar: cfg.acelerar.max(0.001), alvo: None,
                limite: (t_oficiais + 6 * 3600) as f64 }),
            t0: Mutex::new(Instant::now()),
            pleito: cfg.pleito, ambiente: cfg.ambiente.clone(), ciclo: cfg.ciclo.clone(), rajadas, quedas, lat, p5xx: cfg.falhas.erros_5xx,
            semente: cfg.falhas.semente, n_req: AtomicU64::new(0), log: Mutex::new(log), est: Mutex::new(Estatistica::default()),
            caido: AtomicBool::new(false), resumo_carga, limite_ip_rps: cfg.limite_ip_rps.max(0.0), tse,
        })
    }

    /// Relógio simulado (segundos, hora de Brasília).
    pub fn agora(&self) -> f64 {
        let (t, chegou, x) = {
            let mut r = self.relogio.lock().unwrap();
            let (mut t, c) = r.ler();
            // acompanhar o serviço (B-011): o relógio para no instante em que a fila de seções disponibilizadas e ainda
            // sem BU servido chegaria à folga; anda de novo, da mesma aceleração, quando o serviço busca os BUs
            let parado = self.criado.elapsed().saturating_sub(Duration::from_millis(self.ult_servida_ms.load(Relaxed)));
            if self.folga > 0 && (parado < FREIO_MAX || self.n_servidas.load(Relaxed) == 0) {
                let k = (self.n_servidas.load(Relaxed) + self.folga) as usize;
                if let Some(&teto) = self.disps.get(k) {
                    let teto = teto as f64;
                    if t > teto {
                        r.base_sim = teto;
                        r.base_real = Instant::now();
                        t = teto;
                        self.freadas.fetch_add(1, Relaxed);
                    }
                }
            }
            (t, c, r.acelerar)
        };
        if chegou { self.anotar(&format!("alvo alcançado; aceleração {x}×"), t); }
        t
    }

    /// Aceleração atual do relógio simulado.
    pub fn aceleracao(&self) -> f64 { self.relogio.lock().unwrap().acelerar }

    fn anotar(&self, o_que: &str, t: f64) {
        { let mut l = self.log.lock().unwrap(); let _ = writeln!(l, "# {o_que} às {}", crate::texto_hora(t as i64)); }
        eprintln!("uv-simulador: {o_que} (relógio {})", crate::texto_hora(t as i64));
    }

    /// Muda a aceleração a partir de agora (0 = relógio parado), sem salto no relógio; com `alvo = (T, M)`, anda até
    /// `T` e lá passa a `M`×.
    pub fn definir_aceleracao(&self, x: f64, alvo: Option<(f64, f64)>) {
        let t = {
            let mut r = self.relogio.lock().unwrap();
            r.mudar(x);
            r.alvo = alvo.filter(|&(a, _)| a > r.base_sim);
            r.base_sim
        };
        match alvo {
            Some((a, m)) => self.anotar(&format!("aceleração {x}× até {}, depois {m}×", crate::texto_hora(a as i64)), t),
            None => self.anotar(&format!("aceleração {x}×"), t),
        }
    }

    /// Põe o relógio em `t` (limitado ao começo da noite e a 1 h depois dos oficiais) e, se pedido, muda a aceleração.
    pub fn posicionar(&self, t: f64, x: Option<f64>) {
        let t = t.clamp(self.inicio as f64, (self.t_oficiais + 3600) as f64);
        {
            let mut r = self.relogio.lock().unwrap();
            r.posicionar(t);
            if let Some(x) = x { r.acelerar = x; }
        }
        self.anotar(&format!("relógio posto em {}{}", crate::texto_hora(t as i64), x.map(|x| format!(", {x}×")).unwrap_or_default()), t);
    }

    fn controle_json(&self) -> String {
        let t = self.agora();
        let (x, alvo) = { let r = self.relogio.lock().unwrap(); (r.acelerar, r.alvo) };
        json_controle(x, t, self.inicio, self.ultima, alvo)
    }

    /// Caminhos de controle do ensaio (não existem no site real): `GET /controle`, `POST /controle/velocidade` e
    /// `POST /controle/posicao` (ver o começo do módulo).
    fn controle(&self, metodo: &hyper::Method, path: &str, consulta: Option<&str>) -> Response<Full<Bytes>> {
        let resp = |st: u16, corpo: String| Response::builder().status(st).header("content-type", "application/json")
            .header("cache-control", "no-store").body(Full::new(Bytes::from(corpo))).unwrap();
        let arg = |k: &str| consulta.and_then(|q| q.split('&').find_map(|p| p.strip_prefix(k).and_then(|v| v.strip_prefix('=')))).and_then(|v| v.parse::<f64>().ok());
        let vel_ok = |x: f64| x.is_finite() && (0.0..=MAX_ACELERAR).contains(&x);
        match (metodo.as_str(), path) {
            ("GET", "/controle") => {}
            ("POST", "/controle/velocidade") => {
                let (x, ate, depois) = (arg("x"), arg("ate"), arg("depois"));
                match (x, ate, depois) {
                    (Some(x), None, None) if vel_ok(x) => self.definir_aceleracao(x, None),
                    (Some(x), Some(a), Some(m)) if vel_ok(x) && vel_ok(m) && a.is_finite() => self.definir_aceleracao(x, Some((a, m))),
                    _ => return resp(400, "{\"erro\":\"x (e, juntos, ate e depois) entre 0 e 1000000\"}".into()),
                }
            }
            ("POST", "/controle/posicao") => {
                let (t, x) = (arg("t"), arg("x"));
                match (t, x) {
                    (Some(t), x) if t.is_finite() && x.is_none_or(vel_ok) => self.posicionar(t, x),
                    _ => return resp(400, "{\"erro\":\"t (segundos) e, se houver, x entre 0 e 1000000\"}".into()),
                }
            }
            _ => return resp(404, String::new()),
        }
        resp(200, self.controle_json())
    }

    fn cs_json(&self, u: &UfSim, t: i64) -> (Bytes, String) {
        let versao = u.mudancas.partition_point(|&x| x <= t);
        if let Some((v, b, e)) = &*u.cache.lock().unwrap() { if *v == versao { return (b.clone(), e.clone()); } }
        let n: usize = u.muns.iter().map(|m| m.2.iter().map(|z| z.1.len()).sum::<usize>()).sum();
        let mut s = String::with_capacity(64 * n + 1024);
        let (d, h) = crate::texto_hora(t).split_once(' ').map(|(a, b)| (a.to_string(), b.to_string())).unwrap();
        let _ = write!(s, "{{ \"dg\" : \"{d}\", \"hg\" : \"{h}\", \"f\" : \"o\", \"cdp\" : \"{}\", \"abr\" : [ \n{{ \"cd\" : \"{}\", \"mu\" : [ ", self.pleito, u.uf);
        for (mi, (cd, nm, zonas)) in u.muns.iter().enumerate() {
            if mi > 0 { s.push_str(", "); }
            let _ = write!(s, "\n{{ \"cd\" : \"{cd}\", \"nm\" : {}, \"zon\" : [ ", serde_json::to_string(nm).unwrap_or_default());
            for (zi, (zcd, ss)) in zonas.iter().enumerate() {
                if zi > 0 { s.push_str(", "); }
                let _ = write!(s, "\n{{ \"cd\" : \"{zcd}\", \"sec\" : [ ");
                for (si, &id) in ss.iter().enumerate() {
                    if si > 0 { s.push_str(", "); }
                    let sec = &self.secs[id as usize];
                    // agregada: como no cs.json real, só ns e nsp (sem da/ha)
                    if !sec.nsp.is_empty() && sec.bu.is_none() {
                        let _ = write!(s, "\n{{ \"ns\" : \"{:04}\", \"nsp\" : \"{}\" }}", sec.se, sec.nsp);
                        continue;
                    }
                    let (da, ha) = match sec.disp_em(t) { Some(x) => { let tx = crate::texto_hora(x); let (a, b) = tx.split_once(' ').unwrap(); (a.to_string(), b.to_string()) } None => (String::new(), String::new()) };
                    let _ = write!(s, "\n{{ \"ns\" : \"{:04}\", \"da\" : \"{da}\", \"ha\" : \"{ha}\" }}", sec.se);
                }
                s.push_str(" ] }");
            }
            s.push_str(" ] }");
        }
        s.push_str(" ] } ] }\n");
        let b = Bytes::from(s);
        let e = format!("\"{}-{}\"", u.uf, versao);
        *u.cache.lock().unwrap() = Some((versao, b.clone(), e.clone()));
        (b, e)
    }

    fn aux_json(&self, sec: &Sec, t: i64) -> Option<Bytes> {
        let (d, h) = { let tx = crate::texto_hora(t); let (a, b) = tx.split_once(' ').unwrap(); (a.to_string(), b.to_string()) };
        if sec.nao_instalada {
            return Some(Bytes::from(format!("{{ \"dg\" : \"{d}\", \"hg\" : \"{h}\", \"f\" : \"o\", \"st\" : \"Nao instalada\", \"hashes\" : [ ] }}\n")));
        }
        sec.bu?;
        sec.disp.filter(|&x| x <= t)?;
        let anulada = sec.anulada_em(t);
        let mut s = format!("{{ \"dg\" : \"{d}\", \"hg\" : \"{h}\", \"f\" : \"o\", \"st\" : \"{}\", \"hashes\" : [ ", if anulada { "Anulada" } else { "Totalizada" });
        let nome = sec.nome_bu(self.pleito);
        let base = &nome[..nome.len() - "-bu.dat".len()];
        for (i, (hash, rec, reenvio)) in sec.hashes_em(t).iter().enumerate() {
            if i > 0 { s.push_str(", "); }
            let st = if anulada { "Excluído" } else if *reenvio { "Totalizado" } else { sec.situacao };
            let tx = crate::texto_hora(*rec);
            let (dr, hr) = tx.split_once(' ').unwrap();
            let _ = write!(s, "\n{{ \"hash\" : \"{hash}\", \"dr\" : \"{dr}\", \"hr\" : \"{hr}\", \"st\" : \"{st}\", \"arq\" : [ \n{{ \"nm\" : \"{nome}\", \"tp\" : \"bu\" }}, \n{{ \"nm\" : \"{base}-vota.vsc\", \"tp\" : \"vota\" }} ] }}");
        }
        s.push_str(" ] }\n");
        Some(Bytes::from(s))
    }

    /// Tipo da requisição (para o registro): cs, aux, bu, oficial ou outro.
    fn tipo(path: &str) -> &'static str {
        if path.ends_with("-cs.json") { "cs" } else if path.ends_with("-aux.json") { "aux" } else if path.ends_with(".dat") { "bu" }
        else if path.ends_with("-u.json") { "oficial" } else { "outro" }
    }

    /// (status, corpo, etag, content-type)
    fn rotear(&self, path: &str, t: i64, inm: Option<&str>) -> (u16, Bytes, Option<String>, &'static str) {
        const JSON: &str = "application/json";
        let nada = (404, Bytes::new(), None, "text/plain");
        let seg: Vec<&str> = path.trim_start_matches('/').split('/').collect();
        if seg.len() < 3 || seg[0] != self.ambiente || seg[1] != self.ciclo { return nada; }
        if seg.get(2) == Some(&"arquivo-urna") {
            if seg.get(3) != Some(&self.pleito.to_string().as_str()) { return nada; }
            match (seg.get(4), seg.len()) {
                (Some(&"config"), 7) => {
                    let Some(u) = self.ufs.get(seg[5]) else { return nada };
                    if seg[6] != format!("{}-p{:06}-cs.json", u.uf, self.pleito) { return nada; }
                    let (b, e) = self.cs_json(u, t);
                    if inm == Some(e.as_str()) { return (304, Bytes::new(), Some(e), JSON); }
                    (200, b, Some(e), JSON)
                }
                (Some(&"dados"), 10 | 11) => {
                    let (Ok(mu), Ok(zn), Ok(se)) = (seg[6].parse::<u32>(), seg[7].parse::<u16>(), seg[8].parse::<u16>()) else { return nada };
                    let Ok(uf) = <[u8; 2]>::try_from(seg[5].as_bytes()) else { return nada };
                    let Some(&i) = self.idx.get(&(uf, tse::chave(mu, zn, se))) else { return nada };
                    let sec = &self.secs[i as usize];
                    if seg.len() == 10 {
                        if seg[9] != format!("p{:06}-{}-m{mu:05}-z{zn:04}-s{se:04}-aux.json", self.pleito, seg[5]) { return nada; }
                        match self.aux_json(sec, t) { Some(b) => (200, b, None, JSON), None => nada }
                    } else {
                        let Some(b) = sec.bu else { return nada };
                        if sec.disp.is_none_or(|d| d > t) || seg[10] != sec.nome_bu(self.pleito) { return nada; }
                        if !sec.hashes_em(t).iter().any(|h| h.0 == seg[9]) { return nada; }
                        if !self.servida[i as usize].swap(true, Relaxed) {
                            self.n_servidas.fetch_add(1, Relaxed);
                            self.ult_servida_ms.store(self.criado.elapsed().as_millis() as u64, Relaxed);
                        }
                        (200, Bytes::copy_from_slice(self.mem.bu(b as usize)), None, "application/octet-stream")
                    }
                }
                _ => nada,
            }
        } else if let (Some(tse), true) = (&self.tse, self.oficiais.contains_key(path)) {
            // TSE simulado: o arquivo gerado no último passo da noite (o oficial, depois da última divulgação)
            match tse.arquivo(path.rsplit('/').next().unwrap_or(""), t) {
                Some((_, e)) if inm == Some(e.as_str()) => (304, Bytes::new(), Some(e), JSON),
                Some((b, e)) => (200, b, Some(e), JSON),
                None => nada,
            }
        } else if t >= self.t_oficiais {
            match self.oficiais.get(path).and_then(|p| std::fs::read(p).ok()) { Some(b) => (200, Bytes::from(b), None, JSON), None => nada }
        } else {
            nada
        }
    }

    /// Conta a requisição do IP no segundo real corrente; `true` = responder 429 (acima do limite por IP ou na caixa de
    /// penalidade). Sem limite configurado, só conta.
    fn contar_ip(&self, ip: std::net::IpAddr, tf: f64) -> bool {
        let s = self.t0.lock().unwrap().elapsed().as_secs() as usize;
        // aceleração atual (o controle de replay pode mudá-la), lida antes de travar o estado
        let x = self.aceleracao();
        let mut e = self.est.lock().unwrap();
        let p = e.por_ip.entry(ip).or_default();
        if p.seg != s { p.seg = s; p.n = 0; }
        p.n += 1;
        p.total += 1;
        p.max_seg = p.max_seg.max(p.n);
        if self.limite_ip_rps <= 0.0 { return false; }
        if tf < p.penalidade_ate { p.n429 += 1; return true; }
        if p.n as f64 > self.limite_ip_rps * x {
            p.penalidade_ate = tf + PENALIDADE_IP_S;
            p.penalidades += 1;
            p.n429 += 1;
            return true;
        }
        false
    }

    async fn responder(&self, path: String, inm: Option<String>, ip: std::net::IpAddr) -> Response<Full<Bytes>> {
        let tf = self.agora();
        let t = tf.floor() as i64;
        let n = self.n_req.fetch_add(1, Relaxed);
        let r = splitmix(self.semente ^ splitmix(n));
        if self.lat.1 > 0 {
            let ms = self.lat.0 + r % (self.lat.1 - self.lat.0 + 1);
            tokio::time::sleep(Duration::from_millis(ms)).await;
        }
        let tipo = Self::tipo(&path);
        let bloqueado = self.contar_ip(ip, tf);
        let (status, corpo, etag, ct) = if bloqueado || self.rajadas.iter().any(|&(a, b)| t >= a && t < b) {
            (429, Bytes::new(), None, "text/plain")
        } else if path == "/eco-ip" {
            // eco do IP de origem (a federação mede o IP de saída de cada nó, ADR-028)
            (200, Bytes::from(ip.to_string()), None, "text/plain")
        } else if self.p5xx > 0.0 && ((splitmix(r) >> 11) as f64 / (1u64 << 53) as f64) < self.p5xx {
            (503, Bytes::new(), None, "text/plain")
        } else {
            self.rotear(&path, t, inm.as_deref())
        };
        let agora_ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs_f64() * 1e3).unwrap_or(0.0);
        {
            let mut l = self.log.lock().unwrap();
            let _ = writeln!(l, "{agora_ms:.3},{},{tipo},{status},{},{path},{ip}", crate::texto_hora(t), corpo.len());
        }
        {
            let s = (self.t0.lock().unwrap().elapsed().as_secs()) as usize;
            let mut e = self.est.lock().unwrap();
            *e.por_tipo.entry((tipo.to_string(), status)).or_default() += 1;
            if e.por_segundo.len() <= s { e.por_segundo.resize(s + 1, 0); }
            e.por_segundo[s] += 1;
        }
        let mut b = Response::builder().status(StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR))
            .header("date", crate::data_http_de_brasilia(t)).header("content-type", ct).header("cache-control", "no-cache");
        if let Some(e) = etag { b = b.header("etag", e); }
        b.body(Full::new(corpo)).unwrap()
    }

    /// Grava o resumo (requisições por tipo e status, máximo por segundo real) e esvazia o registro.
    pub fn gravar_resumo(&self, p: &std::path::Path) {
        let _ = self.log.lock().unwrap().flush();
        let e = self.est.lock().unwrap();
        let mut s = String::from("{");
        let _ = write!(s, "\"fora_do_ar\":{},", self.caido.load(Relaxed));
        let _ = write!(s, "\"acompanhar\":{{\"folga_secoes\":{},\"bus_servidos\":{},\"freadas\":{}}},", self.folga, self.n_servidas.load(Relaxed), self.freadas.load(Relaxed));
        let _ = write!(s, "\"relogio\":\"{}\",\"inicio\":\"{}\",\"primeira\":\"{}\",\"ultima\":\"{}\",\"oficiais\":\"{}\",\"acelerar\":{},\"requisicoes\":{},\"max_por_segundo\":{},\"por_tipo\":{{",
            crate::texto_hora(self.agora() as i64), crate::texto_hora(self.inicio), crate::texto_hora(self.primeira), crate::texto_hora(self.ultima),
            crate::texto_hora(self.t_oficiais), self.aceleracao(), self.n_req.load(Relaxed), e.por_segundo.iter().max().copied().unwrap_or(0));
        for (i, ((tp, st), q)) in e.por_tipo.iter().enumerate() { let _ = write!(s, "{}\"{tp} {st}\":{q}", if i > 0 { "," } else { "" }); }
        let _ = write!(s, "}},\"limite_ip_rps\":{},\"por_ip\":{{", self.limite_ip_rps);
        for (i, (ip, p)) in e.por_ip.iter().enumerate() {
            let _ = write!(s, "{}\"{ip}\":{{\"requisicoes\":{},\"max_por_segundo\":{},\"http429\":{},\"penalidades\":{}}}", if i > 0 { "," } else { "" },
                p.total, p.max_seg, p.n429, p.penalidades);
        }
        s.push_str("}}\n");
        let _ = std::fs::write(p, s);
    }
}

/// Serve o site simulado em `127.0.0.1:porta` (0 = porta livre). `pronto` recebe a porta escolhida, logo antes de o
/// relógio simulado começar. Não termina sozinho.
pub async fn servir(sim: Arc<Sim>, porta: u16, resumo: PathBuf, pronto: impl FnOnce(u16)) -> anyhow::Result<()> {
    use hyper::server::conn::http1;
    use hyper::service::service_fn;
    use hyper_util::rt::TokioIo;
    let mut ouvinte = Some(tokio::net::TcpListener::bind(("127.0.0.1", porta)).await?);
    let porta = ouvinte.as_ref().unwrap().local_addr()?.port();
    pronto(porta);
    *sim.t0.lock().unwrap() = Instant::now();
    {
        let mut r = sim.relogio.lock().unwrap();
        r.base_sim = sim.inicio as f64;
        r.base_real = Instant::now();
    }
    {
        let (sim, resumo) = (sim.clone(), resumo.clone());
        tokio::spawn(async move { loop { tokio::time::sleep(Duration::from_secs(1)).await; sim.gravar_resumo(&resumo); } });
    }
    let mut quedas = sim.quedas.clone();
    let mut conexoes: Vec<tokio::task::AbortHandle> = vec![];
    loop {
        if let Some(&(t, dur)) = quedas.first() {
            if sim.agora() >= t as f64 {
                quedas.remove(0);
                drop(ouvinte.take());
                for h in conexoes.drain(..) { h.abort(); }
                sim.caido.store(true, Relaxed);
                { let mut l = sim.log.lock().unwrap(); let _ = writeln!(l, "# queda às {} por {dur} s reais", crate::texto_hora(sim.agora() as i64)); }
                eprintln!("uv-simulador: fora do ar por {dur} s (relógio {})", crate::texto_hora(sim.agora() as i64));
                tokio::time::sleep(Duration::from_secs_f64(dur)).await;
                loop {
                    match tokio::net::TcpListener::bind(("127.0.0.1", porta)).await {
                        Ok(l) => { ouvinte = Some(l); break; }
                        Err(_) => tokio::time::sleep(Duration::from_millis(100)).await,
                    }
                }
                sim.caido.store(false, Relaxed);
                { let mut l = sim.log.lock().unwrap(); let _ = writeln!(l, "# de volta às {}", crate::texto_hora(sim.agora() as i64)); }
                eprintln!("uv-simulador: de volta (relógio {})", crate::texto_hora(sim.agora() as i64));
                continue;
            }
        }
        let l = ouvinte.as_ref().unwrap();
        tokio::select! {
            r = l.accept() => {
                let Ok((s, origem)) = r else { continue };
                let _ = s.set_nodelay(true);
                let sim = sim.clone();
                let ip = origem.ip();
                let h = tokio::spawn(async move {
                    let svc = service_fn(move |req: Request<hyper::body::Incoming>| {
                        let sim = sim.clone();
                        let path = req.uri().path().to_string();
                        let inm = req.headers().get("if-none-match").and_then(|v| v.to_str().ok()).map(String::from);
                        let (metodo, consulta) = (req.method().clone(), req.uri().query().map(String::from));
                        async move {
                            Ok::<_, std::convert::Infallible>(if path == "/controle" || path.starts_with("/controle/") {
                                sim.controle(&metodo, &path, consulta.as_deref())
                            } else {
                                sim.responder(path, inm, ip).await
                            })
                        }
                    });
                    let _ = http1::Builder::new().keep_alive(true).serve_connection(TokioIo::new(s), svc).await;
                });
                conexoes.retain(|h| !h.is_finished());
                conexoes.push(h.abort_handle());
            }
            _ = tokio::time::sleep(Duration::from_millis(50)) => {}
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    #[test]
    fn textos() {
        let d = segundos("27/10/2024 00:00:00").unwrap();
        assert_eq!(hora("17:05:10", d + 3600), Some(d + 17 * 3600 + 5 * 60 + 10));
        assert_eq!(hora("28/10/2024 01:00:00", d), Some(d + 86400 + 3600));
        assert_eq!(secao_de_texto("ma:7315/23/354"), Some((*b"ma", tse::chave(7315, 23, 354))));
        assert_eq!(secao_de_texto("ma:7315/23"), None);
        assert_ne!(hash_reenvio("abc", 1), hash_reenvio("abc", 2));
        assert_eq!(tse::sha256_do_hash_dir(&hash_reenvio("abc", 1)) == [0; 32], false);
    }
    /// Os BUs publicados antes das 17h (exterior, 1º turno de 2026) entram juntos às 17h; os outros não mudam.
    #[test]
    fn piso_das_17h() {
        let h = |x: &str| segundos(&format!("04/10/2026 {x}"));
        let piso = h("17:00:00");
        // Wellington: publicada 01:49:24, "recebida" 17:08:44 (o lote do aux.json)
        assert_eq!(com_piso(h("17:08:44"), h("01:49:24"), piso), piso, "recebimento agrupado às 17:08");
        assert_eq!(com_piso(h("01:49:24"), h("01:49:24"), piso), piso, "pela disponibilização");
        // Budapeste, só com o horário escolhido (13:44)
        assert_eq!(com_piso(h("13:44:27"), None, piso), piso);
        // Brasil e Miami: depois das 17h, como estão
        assert_eq!(com_piso(h("17:00:31"), h("17:01:02"), piso), h("17:00:31"));
        assert_eq!(com_piso(h("20:57:03"), h("20:57:04"), piso), h("20:57:03"));
        // sem horário continua sem; sem piso, nada muda
        assert_eq!(com_piso(None, h("01:49:24"), piso), None);
        assert_eq!(com_piso(h("17:08:44"), h("01:49:24"), None), h("17:08:44"));
    }
    #[test]
    fn relogio_para_no_alvo() {
        let mut r = Relogio { base_sim: 1000.0, base_real: Instant::now(), acelerar: 100_000.0, alvo: Some((1500.0, 2.0)), limite: f64::INFINITY };
        std::thread::sleep(Duration::from_millis(30));
        // a 100 000× passaria de 1500 em 5 ms: para no alvo e segue a 2×
        let (t, chegou) = r.ler();
        assert!(chegou && (1500.0..1501.0).contains(&t), "{t}");
        assert_eq!((r.acelerar, r.alvo), (2.0, None));
        r.posicionar(1200.0);
        assert!((1200.0..1200.5).contains(&r.agora()));
        // o relógio para no limite
        r.limite = 1300.0;
        r.mudar(100_000.0);
        std::thread::sleep(Duration::from_millis(10));
        assert_eq!((r.agora(), r.acelerar), (1300.0, 0.0));
        r.limite = f64::INFINITY;
        r.posicionar(1200.0);
        let j: serde_json::Value = serde_json::from_str(&json_controle(2.0, 1200.0, 1000, 2000, Some((1800.0, 10.0)))).unwrap();
        assert_eq!((j["t"].as_i64(), j["fim"].as_i64(), j["alvo"]["t"].as_i64(), j["alvo"]["depois"].as_f64()), (Some(1200), Some(2000), Some(1800), Some(10.0)));
    }
    #[test]
    fn relogio_muda_sem_salto() {
        let mut r = Relogio { base_sim: 1000.0, base_real: Instant::now(), acelerar: 10.0, alvo: None, limite: f64::INFINITY };
        std::thread::sleep(Duration::from_millis(20));
        let a = r.agora();
        assert!(a > 1000.1);
        r.mudar(1000.0);
        let b = r.agora();
        assert!(b >= a && b - a < 1.0, "{a} → {b}");
        std::thread::sleep(Duration::from_millis(20));
        assert!(r.agora() - b > 10.0);
        r.mudar(0.0);
        let c = r.agora();
        std::thread::sleep(Duration::from_millis(10));
        assert_eq!(r.agora(), c);
        r.mudar(1.0);
        assert!(r.agora() >= c);
    }
}
