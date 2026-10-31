//! Difusão em tempo real (etapa 4): o `uv-servico` publica o resultado por **WebSocket** (instantâneo na inscrição +
//! atualização a cada publicação) e por um **REST mínimo**, para muitos clientes ao mesmo tempo, sem atrasar a
//! totalização. Contrato: `docs/arquitetura/PROTOCOLO_WS.md`; decisão: ADR-026.
//!
//! Caminho de um resultado: thread do motor (gera o JSON uma vez) → thread `uv-gravacao` (converte em `Bytes` e entrega
//! aqui **antes** de gravar o arquivo; canal sem limite, nunca espera) → thread `uv-difusao` ([`difundir`]: monta a
//! mensagem do tópico **uma vez** e a põe num `tokio::sync::watch`, que guarda só o último valor) → tarefas das conexões
//! (runtime `tokio` próprio, `uv-http`, com `--http-threads` threads), que mandam a mesma mensagem (`Utf8Bytes`,
//! contagem de referência) a todos os inscritos. Cliente lento: o `watch` junta as versões; a conexão recebe a mais
//! recente com um aviso de `seq` pulado; sem conseguir escrever por [`Opcoes::lento`], cai.
//!
//! O motor continua sem rede e sem `tokio` (ADR-012/014): tudo isto está no `uv-aquisicao`. Nada que chega de um
//! cliente vai além da inscrição.

use crate::servico::Amostra;
use axum::extract::ws::{CloseFrame, Message, Utf8Bytes, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::Router;
use bytes::Bytes;
use futures::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::fmt::Write as _;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering::Relaxed};
use std::sync::{Arc, Mutex, OnceLock, RwLock};
use std::time::{Duration, Instant};
use tokio::sync::watch;
use tokio_stream::wrappers::WatchStream;
use tokio_stream::StreamMap;

/// Agora em µs Unix (relógio de parede).
pub fn agora_us() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_micros() as u64).unwrap_or(0)
}

/// Limites e parâmetros da camada (ver `docs/arquitetura/PROTOCOLO_WS.md` §8).
#[derive(Clone, Debug)]
pub struct Opcoes {
    /// `endereço:porta` de escuta (ex.: `127.0.0.1:8080`)
    pub endereco: String,
    /// conexões WebSocket simultâneas; acima, 503 no *handshake*
    pub max_clientes: usize,
    /// threads do runtime da camada HTTP
    pub threads: usize,
    pub max_topicos: usize,
    /// mensagens recebidas por segundo por conexão (balde de fichas) e rajada
    pub msgs_s: f64,
    pub rajada: f64,
    /// mensagens acima do limite de taxa até a conexão cair
    pub excessos: u32,
    /// tamanho máximo de uma mensagem recebida (bytes)
    pub max_msg: usize,
    /// buffer de escrita por conexão (bytes): acima, a conexão cai (`cliente_lento`)
    pub buffer_escrita: usize,
    pub ping: Duration,
    pub sem_pong: Duration,
    /// tempo máximo esperando para escrever numa conexão (`cliente_lento`)
    pub lento: Duration,
    /// tempo máximo para os cabeçalhos HTTP do *handshake*
    pub handshake: Duration,
    /// mensagem `estado` pelo menos a cada este intervalo
    pub estado_max: Duration,
    /// multipleito (1.1): `pleitos.json` do `uv-coletor pleitos`, servido em `GET /api/v1/pleitos` (relido quando muda)
    pub pleitos: Option<std::path::PathBuf>,
    /// pasta das séries de consulta (multipleito 1.1, `crate::serie`): `<pasta>/<pleito>/serie/*.json` e o índice
    /// `<pasta>/indice.json`, servidos em `GET /consulta/*` (com o `.json.gz` pré-comprimido quando o cliente aceita)
    pub consulta: Option<std::path::PathBuf>,
}

impl Opcoes {
    pub fn padrao(endereco: &str) -> Opcoes {
        Opcoes {
            endereco: endereco.to_string(), max_clientes: 7500, threads: 4, max_topicos: 200, msgs_s: 20.0, rajada: 40.0, excessos: 40,
            max_msg: 4096, buffer_escrita: 4 << 20, ping: Duration::from_secs(20), sem_pong: Duration::from_secs(60),
            lento: Duration::from_secs(30), handshake: Duration::from_secs(10), estado_max: Duration::from_secs(10),
            pleitos: None, consulta: None,
        }
    }
}

/// A mensagem de um tópico numa versão: montada uma vez, compartilhada por todas as conexões.
pub struct Quadro {
    pub topico: Arc<str>,
    pub arquivo: Arc<str>,
    /// versão do tópico neste processo (1, 2, 3…)
    pub ver: u64,
    pub t_bu: Option<u64>,
    /// o arquivo publicado (os mesmos bytes que a `uv-gravacao` grava)
    pub json: Bytes,
    /// a mensagem, montada uma vez só quando alguém precisa dela (a primeira conexão inscrita ou o REST), nas threads
    /// da camada HTTP: sem clientes, a difusão não custa nada além de guardar o `Arc` (B com `--http` e 0 clientes)
    montado: std::sync::OnceLock<Option<Montado>>,
}

/// A mensagem `atualizacao` de uma versão de tópico.
pub struct Montado {
    pub seq: u64,
    pub secoes: u64,
    /// instante em que a mensagem foi montada (o mesmo para todas as conexões)
    pub t_envio: u64,
    /// mensagem inteira
    pub msg: Utf8Bytes,
    /// o arquivo publicado (fatia de `msg`, mesmos bytes de `Quadro::json`)
    pub res: Bytes,
}

#[derive(serde::Deserialize)]
struct Cab { seq: u64, secoes: CabSecoes }
#[derive(serde::Deserialize)]
struct CabSecoes { apuradas: u64 }

fn cabecalho(tipo: &str, topico: &str, seq: u64, secoes: u64, ver: u64, t_bu: Option<u64>, t_envio: u64, extra: usize) -> String {
    let mut m = String::with_capacity(200 + extra);
    let tb = t_bu.map_or("null".to_string(), |x| x.to_string());
    let _ = write!(m, r#"{{"tipo":"{tipo}","topico":"{topico}","seq":{seq},"secoes_apuradas":{secoes},"ver":{ver},"t_bu":{tb},"t_envio":{t_envio},"resultado":"#);
    m
}

impl Quadro {
    /// `json` = bytes do arquivo publicado (o `resultado` da mensagem). Barato: não lê nem copia nada.
    pub fn novo(topico: Arc<str>, arquivo: Arc<str>, json: Bytes, ver: u64, t_bu: Option<u64>) -> Quadro {
        Quadro { topico, arquivo, ver, t_bu, json, montado: std::sync::OnceLock::new() }
    }
    /// A mensagem `atualizacao` (montada na primeira chamada); `None` se o arquivo não é um resultado legível.
    pub fn m(&self) -> Option<&Montado> {
        self.montado.get_or_init(|| {
            let texto = std::str::from_utf8(&self.json).ok()?;
            let c: Cab = serde_json::from_str(texto).ok()?;
            let t_envio = agora_us();
            let mut m = cabecalho("atualizacao", &self.topico, c.seq, c.secoes.apuradas, self.ver, self.t_bu, t_envio, texto.len());
            let ini = m.len();
            m.push_str(texto);
            m.push('}');
            let b = Bytes::from(m);
            let res = b.slice(ini..b.len() - 1);
            let msg = Utf8Bytes::try_from(b).ok()?;
            Some(Montado { seq: c.seq, secoes: c.secoes.apuradas, t_envio, msg, res })
        }).as_ref()
    }
    /// A mesma mensagem como `instantaneo` (resposta a uma inscrição), com `t_envio` de agora.
    pub fn instantaneo(&self) -> Option<Utf8Bytes> {
        let mo = self.m()?;
        let mut m = cabecalho("instantaneo", &self.topico, mo.seq, mo.secoes, self.ver, self.t_bu, agora_us(), mo.res.len());
        // os bytes vieram de um `str`
        m.push_str(std::str::from_utf8(&mo.res).unwrap_or(""));
        m.push('}');
        Some(Utf8Bytes::from(m))
    }
}

/// Corpo de `GET /api/v1/nomes` (protocolo §6), a partir do `candidatos.json`: nomes das UFs e dos municípios (chave =
/// código da abrangência, `sp71072`), e por eleição/cargo as abrangências publicadas e os candidatos de cada
/// circunscrição com nome de urna, nome, partido e vice/suplentes. Sem nada que não esteja nos arquivos públicos do TSE.
pub fn montar_nomes(arq: &urna_velox::candidatos::ArquivoCandidatos) -> Vec<u8> {
    let munis: serde_json::Map<String, Value> = arq.municipios.iter().filter(|m| !m.nome.is_empty())
        .map(|m| (format!("{}{:05}", m.uf, m.codigo), json!(m.nome))).collect();
    let eleicoes: Vec<Value> = arq.eleicoes.iter().map(|e| json!({"codigo": e.codigo, "turno": e.turno, "cargos": e.cargos.iter().map(|c| json!({
        "cargo": c.cargo, "vagas": c.vagas, "consulta": c.consulta,
        "abrangencias": c.abrangencias.iter().map(|a| a.abr.clone()).collect::<Vec<_>>(),
        "circunscricoes": c.circunscricoes.iter().map(|ci| json!({"abr": ci.abr, "eleitorado": ci.eleitorado, "candidatos": ci.candidatos.iter().map(|k| json!({
            "n": k.numero, "nome_urna": if k.nome_urna.trim().is_empty() { &k.nome } else { &k.nome_urna }, "nome": k.nome, "partido": k.partido,
            "vices": k.vices.iter().map(|v| json!({"tp": v.tp, "nome_urna": if v.nome_urna.is_empty() { &v.nome } else { &v.nome_urna }, "partido": v.partido})).collect::<Vec<_>>(),
        })).collect::<Vec<_>>()})).collect::<Vec<_>>(),
    })).collect::<Vec<_>>()})).collect();
    json!({"pleito": arq.pleito, "ufs": arq.nomes_ufs, "municipios": munis, "eleicoes": eleicoes}).to_string().into_bytes()
}

/// Tópico de um arquivo publicado: `br-c0001-e006257.json` → `br-c0001`.
pub fn topico_de(arquivo: &str) -> Option<&str> {
    let t = arquivo.strip_suffix(".json")?.rsplit_once("-e")?.0;
    (!t.is_empty() && t.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')).then_some(t)
}

/// Um evento como a thread do motor o entrega.
#[derive(Clone, Debug)]
pub struct EventoBruto {
    /// `matematicamente` (linha do `eventos.csv`) ou `apuracao`
    pub classe: &'static str,
    pub seq: u64,
    /// horário do BU que provocou o evento (relógio do TSE/site)
    pub horario: String,
    /// horário do sistema em que o Urna Velox o viu (M-003; ISO com ms)
    pub sistema: Option<String>,
    pub abrangencia: Option<String>,
    pub cargo: Option<u8>,
    pub candidato: Option<u32>,
    pub estado: String,
    pub motivo: String,
}

struct Evento {
    n: u64,
    /// tópico da circunscrição; `None` = vai para todos
    topico: Option<Arc<str>>,
    msg: Utf8Bytes,
    valor: Value,
}

/// O que chega à thread de difusão.
pub enum Entrada {
    /// (nome do arquivo, bytes, instante do BU mais recente do lote em µs)
    Arquivos(Vec<(String, Bytes, Option<u64>)>),
    Eventos(Vec<EventoBruto>),
    /// subconjunto da `saude.json` (ver [`Hub::saude_json`] e `servico::Motor::saude`)
    Estado(Value),
    /// marcos da noite (`crate::marcos`): cada um com `classe`; `faltam_para_mat` guarda só o último valor por
    /// circunscrição, os outros entram no histórico
    Marcos(Vec<Value>),
    /// previsões do "matematicamente" e do fim da apuração (etapa 8, `crate::previsao::mensagem`): guarda só a última
    /// de cada tópico; o `historico` vai só na mensagem da inscrição
    Previsoes(Vec<Value>),
    /// sondas (`crate::sonda`, corpo de [`crate::sonda::Sonda::json`]) que passaram pela gravação: só para quem pediu
    /// `op: sonda` (`docs/arquitetura/PROTOCOLO_WS.md` §4.9)
    Sonda(Vec<Value>),
    /// seções aplicadas (chave `município << 32 | zona << 16 | seção`, horário do BU) para o mapa das seções
    /// (`crate::mapa`): as do lote e, na partida, as do diário
    Secoes(Vec<(u64, i64)>),
}

/// Sondas guardadas para o `GET /api/v1/sonda`.
pub const SONDAS_GUARDADAS: usize = 20;

/// Última previsão de um tópico: versão (global, crescente), a mensagem das atualizações (sem o histórico), a da
/// inscrição (com o histórico) e o valor (para o REST).
struct Prev { ver: u64, msg: Utf8Bytes, inst: Utf8Bytes, corpo: Bytes }

/// Último `faltam_para_mat` de uma circunscrição: versão (global, crescente), mensagem e valor.
struct Faltam { ver: u64, msg: Utf8Bytes, valor: Value }

struct EstadoQ { corpo: Bytes, msg: Utf8Bytes }

/// Medidas da camada (vão para a `saude.json` e para o relatório).
#[derive(Default)]
pub struct Medidas {
    pub clientes: AtomicI64,
    pub conexoes: AtomicU64,
    pub inscricoes: AtomicI64,
    pub recusadas: AtomicU64,
    pub tcp_abertas: AtomicI64,
    pub tcp_recusadas: AtomicU64,
    pub msgs: AtomicU64,
    pub bytes: AtomicU64,
    pub juntadas: AtomicU64,
    pub avisos_pulo: AtomicU64,
    pub erros_cliente: AtomicU64,
    pub rest: AtomicU64,
    pub rest_304: AtomicU64,
    pub desconexoes: Mutex<BTreeMap<String, u64>>,
    /// BU recebido → mensagem escrita no socket (ms), por mensagem de resultado enviada
    pub lat_envio: Mutex<Amostra>,
    /// BU recebido → mensagem montada (ms), por versão de tópico
    pub lat_difusao: Mutex<Amostra>,
    /// sonda injetada → mensagem `sonda` escrita no socket (ms; só as deste serviço, por conexão inscrita)
    pub lat_sonda: Mutex<Amostra>,
    /// mensagens `sonda` escritas nos sockets
    pub sondas_enviadas: AtomicU64,
    ant: Mutex<Option<(Instant, u64, u64)>>,
}

/// Curva da apuração de um tópico (`GET /api/v1/curva/<topico>`, desde a `0.4.10`): um ponto por minuto da noite
/// (o último ponto acompanha a publicação mais recente), com o % de seções e o % de cada candidato.
pub const CURVA_PASSO_S: i64 = 60;
/// Desde a M-003 o eixo `t` é o **relógio do sistema** (segundos com fração, hora de Brasília: o `totalizado_em` da
/// publicação); o passo continua sendo um minuto **da noite** (no simulado a 500×, 0,12 s do sistema). Publicação sem
/// `totalizado_em` (as do motor e dos testes antigos) usa o `ultimo_bu`, como antes.
/// `sistema`: o `t` é do relógio do sistema (`totalizado_em`); senão, a hora da noite (`ultimo_bu`). Uma curva tem um
/// relógio só (B-082).
struct PontoCurva { t: f64, ap: u64, tot: u64, cand: Vec<(u32, f64)>, sistema: bool }
/// O que a curva lê de um arquivo publicado (o resto do JSON é ignorado).
#[derive(serde::Deserialize)]
struct PubCurva { ultimo_bu: Option<String>, #[serde(default)] totalizado_em: Option<String>, secoes: Option<SecCurva>, #[serde(default)] cand: Vec<CandCurva> }
#[derive(serde::Deserialize)]
struct SecCurva { total: u64, apuradas: u64 }
#[derive(serde::Deserialize)]
struct CandCurva { n: u32, pct: Option<f64> }

/// "2026-10-04T19:54:46-03:00" → segundos na escala de `urna_velox::metricas::segundos` (horário civil de Brasília).
fn segundos_iso(s: &str) -> Option<i64> {
    let b = s.as_bytes();
    if b.len() < 19 { return None; }
    urna_velox::metricas::segundos(&format!("{}/{}/{} {}", &s[8..10], &s[5..7], &s[0..4], &s[11..19]))
}

/// O centro da difusão: últimos valores por tópico, eventos, estado; compartilhado pela thread de difusão, pelo
/// runtime HTTP e pela saúde do serviço.
pub struct Hub {
    pub op: Opcoes,
    tx: crossbeam_channel::Sender<Entrada>,
    topicos: RwLock<HashMap<Arc<str>, watch::Sender<Arc<Quadro>>>>,
    eventos: RwLock<Vec<Arc<Evento>>>,
    n_eventos: watch::Sender<usize>,
    estado: watch::Sender<Option<Arc<EstadoQ>>>,
    desligar: watch::Sender<bool>,
    desligando: AtomicBool,
    /// endereço efetivo de escuta (com a porta escolhida, se a pedida foi 0)
    escuta: Mutex<String>,
    /// último bloco `difusao` da saúde (calculado pela thread de difusão a cada `Entrada::Estado`)
    saude: Mutex<Value>,
    saude_pedida: AtomicBool,
    /// marcos com histórico (`primeira_secao`, `secao_decisiva`, `bu_marco`, `ultima_secao`)
    marcos: RwLock<Vec<Arc<Evento>>>,
    n_marcos: watch::Sender<usize>,
    /// último `faltam_para_mat` por tópico da circunscrição, e a versão mais alta
    faltam: RwLock<BTreeMap<Arc<str>, Faltam>>,
    faltam_ver: watch::Sender<u64>,
    /// `GET /api/v1/nomes`: corpo e `ETag` (fixos por processo; ver [`Hub::definir_nomes`])
    nomes: RwLock<Option<(Bytes, String)>>,
    /// controle do replay: velocidade e posição na noite (só com `--controle-replay`)
    replay: OnceLock<Arc<crate::controle_replay::Replay>>,
    /// curva da apuração de cada tópico ([`CURVA_PASSO_S`])
    curvas: Mutex<HashMap<Arc<str>, Vec<PontoCurva>>>,
    /// comparação com o TSE (etapa 7): tópicos `tse:<tópico>` (ver `crate::tse_resultados`)
    tse: OnceLock<Arc<crate::tse_resultados::TseResultados>>,
    /// previsão (etapa 8): última de cada tópico e a versão mais alta
    previsoes: RwLock<HashMap<Arc<str>, Arc<Prev>>>,
    prev_ver: watch::Sender<u64>,
    /// relógio do sistema do serviço (M-003): a curva anda por ele, com o passo de um minuto da noite
    relogio: OnceLock<Arc<crate::relogio::Relogio>>,
    /// sondas (`crate::sonda`): as últimas [`SONDAS_GUARDADAS`] (corpo com `t_difusao`) e a contagem, que acorda as
    /// conexões inscritas
    sondas: RwLock<VecDeque<Arc<Value>>>,
    n_sondas: watch::Sender<u64>,
    /// mapa das seções (`crate::mapa`): índices do catálogo e seções acesas; definido na partida do motor
    mapa: OnceLock<Arc<crate::mapa::Mapa>>,
    /// `GET /api/v1/pleitos`: data de modificação, corpo e `ETag` do último `pleitos.json` lido ([`Opcoes::pleitos`])
    pleitos: Mutex<Option<(std::time::SystemTime, Bytes, String)>>,
    pub med: Medidas,
}

impl Hub {
    pub fn novo(op: Opcoes) -> (Arc<Hub>, crossbeam_channel::Receiver<Entrada>) {
        let (tx, rx) = crossbeam_channel::unbounded();
        let h = Hub {
            op, tx, topicos: RwLock::default(), eventos: RwLock::default(), n_eventos: watch::Sender::new(0), estado: watch::Sender::new(None),
            desligar: watch::Sender::new(false), desligando: AtomicBool::new(false), escuta: Mutex::default(),
            saude: Mutex::new(Value::Null), saude_pedida: AtomicBool::new(false), marcos: RwLock::default(), n_marcos: watch::Sender::new(0),
            faltam: RwLock::default(), faltam_ver: watch::Sender::new(0), nomes: RwLock::new(None), replay: OnceLock::new(), curvas: Mutex::default(), tse: OnceLock::new(), med: Medidas::default(),
            previsoes: RwLock::default(), prev_ver: watch::Sender::new(0), relogio: OnceLock::new(), mapa: OnceLock::new(),
            sondas: RwLock::default(), n_sondas: watch::Sender::new(0), pleitos: Mutex::new(None),
        };
        (Arc::new(h), rx)
    }

    /// O corpo de `GET /api/v1/nomes` (UFs, municípios, candidatos com partido e vice), montado uma vez na partida a
    /// partir do `candidatos.json`; o `ETag` é o hash do corpo (fixo enquanto o processo vive).
    pub fn definir_nomes(&self, corpo: Vec<u8>) {
        use sha2::Digest;
        let h = sha2::Sha256::digest(&corpo);
        let etag = format!("\"{}\"", h.iter().take(8).map(|b| format!("{b:02x}")).collect::<String>());
        *self.nomes.write().unwrap() = Some((Bytes::from(corpo), etag));
    }

    pub fn n_marcos(&self) -> usize { self.marcos.read().unwrap().len() }

    /// O mapa das seções (`crate::mapa`), com as seções do catálogo; uma vez por processo (antes das conexões).
    pub fn definir_mapa(&self, m: crate::mapa::Mapa) { let _ = self.mapa.set(Arc::new(m)); }
    /// O mapa das seções, se já definido (para tirar do total as agregadas da noite, B-064).
    pub fn mapa(&self) -> Option<&Arc<crate::mapa::Mapa>> { self.mapa.get() }

    /// Um arquivo publicado entra na curva do tópico: o último ponto acompanha a publicação; um ponto novo a cada
    /// [`CURVA_PASSO_S`] da noite. Publicação com horário anterior ao último ponto (não acontece numa sessão) é ignorada.
    fn amostrar_curva(&self, t: &Arc<str>, json: &[u8]) {
        let Ok(p) = serde_json::from_slice::<PubCurva>(json) else { return };
        let ts = match &p.totalizado_em {
            Some(s) => crate::relogio::ms_iso(s).map(|x| x as f64 / 1000.0),
            None => p.ultimo_bu.as_deref().and_then(segundos_iso).map(|x| x as f64),
        };
        let (Some(ts), Some(s)) = (ts, p.secoes) else { return };
        let sistema = p.totalizado_em.is_some();
        let passo = self.passo_curva(sistema);
        let ponto = PontoCurva { t: ts, ap: s.apuradas, tot: s.total, cand: p.cand.iter().filter_map(|c| c.pct.map(|x| (c.n, x))).collect(), sistema };
        let mut m = self.curvas.lock().unwrap();
        let v = m.entry(t.clone()).or_default();
        // B-082: um relógio só por curva. O do sistema manda (a curva da noite que veio antes, de uma publicação sem
        // `totalizado_em` como a da retomada, sai); depois dele, publicação com a hora da noite é ignorada
        if let Some(u) = v.last() { if u.sistema != sistema { if sistema { v.clear(); } else { return; } } }
        let n = v.len();
        if n > 0 && ts < v[n - 1].t { return; }
        // o último ponto é provisório (é trocado pela publicação seguinte) enquanto estiver a menos de um passo do
        // penúltimo; daí em diante fica, e a próxima publicação abre um ponto novo
        if n >= 2 && v[n - 1].t - v[n - 2].t < passo { v[n - 1] = ponto; } else { v.push(ponto); }
    }

    /// Passo da curva em segundos do eixo: um minuto da noite (no relógio do sistema, dividido pela aceleração).
    fn passo_curva(&self, sistema: bool) -> f64 {
        let x = if sistema { self.relogio.get().and_then(|r| r.ancora()).map_or(1.0, |a| a.acelerar) } else { 1.0 };
        CURVA_PASSO_S as f64 / x.max(1.0)
    }

    /// Liga o relógio do sistema do serviço (M-003; antes de [`servir`]).
    pub fn definir_relogio(&self, r: Arc<crate::relogio::Relogio>) { let _ = self.relogio.set(r); }

    /// Uma versão de arquivo que não vira mensagem (outra mais nova do mesmo tópico chegou junto, como na rajada da
    /// retotalização do replay) só entra na curva se fechar um ponto: lê o `ultimo_bu` sem interpretar o JSON e só
    /// chama [`Hub::amostrar_curva`] quando o ponto provisório passaria de um passo. Assim a curva de depois de voltar
    /// no tempo tem a noite inteira, com ~2 leituras completas por tópico por passo em vez de uma por publicação.
    fn amostrar_curva_superada(&self, nome: &str, json: &[u8]) {
        let Some(t) = topico_de(nome) else { return };
        // o `totalizado_em` (relógio do sistema, M-003) vem logo depois do `ultimo_bu`; sem ele, o `ultimo_bu`
        let achar = |chave: &[u8], n: usize| json.windows(chave.len()).take(4096).position(|w| w == chave)
            .and_then(|i| json.get(i + chave.len()..i + chave.len() + n)).and_then(|s| std::str::from_utf8(s).ok());
        let (ts, sistema) = match achar(b"\"totalizado_em\":\"", 23).and_then(crate::relogio::ms_iso) {
            Some(ms) => (ms as f64 / 1000.0, true),
            None => match achar(b"\"ultimo_bu\":\"", 19).and_then(segundos_iso) { Some(s) => (s as f64, false), None => return },
        };
        let passo = self.passo_curva(sistema);
        let precisa = {
            let m = self.curvas.lock().unwrap();
            match m.get(t) {
                Some(v) if v.len() >= 2 => { let n = v.len(); v[n - 1].t - v[n - 2].t >= passo || ts - v[n - 2].t >= passo }
                _ => true,
            }
        };
        if precisa { self.amostrar_curva(&t.into(), json); }
    }

    /// Liga a comparação com o TSE (tópicos `tse:`; antes de [`servir`]).
    pub fn definir_tse(&self, t: Arc<crate::tse_resultados::TseResultados>) { let _ = self.tse.set(t); }
    pub fn tse(&self) -> Option<&Arc<crate::tse_resultados::TseResultados>> { self.tse.get() }

    /// Liga o controle do replay (antes de [`servir`]).
    pub fn definir_replay(&self, r: crate::controle_replay::Replay) { let _ = self.replay.set(Arc::new(r)); }
    /// Bloco `replay` da mensagem `estado` (só com o controle ligado).
    pub fn replay_json(&self) -> Option<Value> { self.replay.get().map(|r| r.json_estado()) }
    /// Entrega sem esperar (canal sem limite): a totalização nunca espera um cliente.
    pub fn entregar(&self, e: Entrada) { let _ = self.tx.send(e); }

    /// Avisa as conexões e para de aceitar novas (parada limpa).
    pub fn desligar(&self) {
        self.desligando.store(true, Relaxed);
        self.desligar.send_replace(true);
    }

    fn desconexao(&self, motivo: &str) { *self.med.desconexoes.lock().unwrap().entry(motivo.to_string()).or_default() += 1; }

    pub fn n_topicos(&self) -> usize { self.topicos.read().unwrap().len() }

    /// O último bloco `difusao` calculado (barato: para a thread do motor), e pede o próximo à thread de difusão.
    pub fn saude_guardada(&self) -> Value {
        self.saude_pedida.store(true, Relaxed);
        self.saude.lock().unwrap().clone()
    }

    /// Bloco `difusao` da `saude.json`.
    pub fn saude_json(&self) -> Value {
        let m = &self.med;
        let (msgs, bytes) = (m.msgs.load(Relaxed), m.bytes.load(Relaxed));
        let agora = Instant::now();
        let (mut msgs_s, mut bytes_s) = (0.0, 0.0);
        {
            let mut a = m.ant.lock().unwrap();
            if let Some((t, m0, b0)) = *a {
                let dt = (agora - t).as_secs_f64().max(1e-3);
                msgs_s = (msgs - m0) as f64 / dt;
                bytes_s = (bytes - b0) as f64 / dt;
            }
            *a = Some((agora, msgs, bytes));
        }
        json!({"endereco": *self.escuta.lock().unwrap(), "clientes": m.clientes.load(Relaxed), "conexoes_total": m.conexoes.load(Relaxed),
            "inscricoes": m.inscricoes.load(Relaxed), "recusadas_max_clientes": m.recusadas.load(Relaxed),
            "tcp_abertas": m.tcp_abertas.load(Relaxed), "tcp_recusadas": m.tcp_recusadas.load(Relaxed),
            "mensagens": msgs, "bytes": bytes, "mensagens_s": (msgs_s * 10.0).round() / 10.0, "bytes_s": bytes_s.round(),
            "juntadas": m.juntadas.load(Relaxed), "avisos_seq_pulado": m.avisos_pulo.load(Relaxed), "erros_cliente": m.erros_cliente.load(Relaxed),
            "rest": m.rest.load(Relaxed), "rest_304": m.rest_304.load(Relaxed),
            "desconexoes": m.desconexoes.lock().unwrap().clone(), "topicos": self.n_topicos(), "eventos": self.eventos.read().unwrap().len(),
            "marcos": self.n_marcos(), "faltam_circunscricoes": self.faltam.read().unwrap().len(),
            "mapa": self.mapa.get().map(|mp| { let (n, fora) = mp.contagem(); json!({"secoes": mp.total(), "acesas": n, "fora_do_catalogo": fora, "fotografias": mp.fotos.load(Relaxed)}) }),
            "sondas_enviadas": m.sondas_enviadas.load(Relaxed),
            "latencias_ms": {"bu_envio": m.lat_envio.lock().unwrap().json(), "bu_difusao": m.lat_difusao.lock().unwrap().json(),
                "sonda_envio": m.lat_sonda.lock().unwrap().json()}})
    }

    /// Recebe as sondas na thread de difusão: carimba `t_difusao` e guarda as últimas.
    fn guardar_sondas(&self, v: Vec<Value>) {
        if v.is_empty() { return; }
        let (t, n) = (agora_us(), v.len() as u64);
        let mut g = self.sondas.write().unwrap();
        for mut x in v {
            x["t_difusao"] = json!(t);
            g.push_back(Arc::new(x));
            while g.len() > SONDAS_GUARDADAS { g.pop_front(); }
        }
        drop(g);
        self.n_sondas.send_modify(|x| *x += n);
    }

    /// `GET /api/v1/sonda`: as últimas sondas (a mais nova no fim) e a latência até o socket.
    pub fn sondas_json(&self) -> Value {
        let v: Vec<Value> = self.sondas.read().unwrap().iter().map(|x| (**x).clone()).collect();
        json!({"ultimas": v, "enviadas": self.med.sondas_enviadas.load(Relaxed), "latencia_envio_ms": self.med.lat_sonda.lock().unwrap().json()})
    }
}

/// Mensagem `sonda` de uma sonda guardada, para uma conexão (com `t_envio`; `instantaneo` na inscrição).
fn msg_sonda(x: &Value, instantaneo: bool) -> Utf8Bytes {
    let mut m = x.clone();
    m["tipo"] = json!("sonda");
    m["instantaneo"] = json!(instantaneo);
    m["t_envio"] = json!(agora_us());
    Utf8Bytes::from(m.to_string())
}

// ------------------------------------------------------------------ thread de difusão

/// Thread `uv-difusao`: monta cada mensagem uma vez e a põe no `watch` do tópico; eventos e estado.
pub fn difundir(h: Arc<Hub>, rx: crossbeam_channel::Receiver<Entrada>) {
    let mut ver: HashMap<Arc<str>, u64> = HashMap::new();
    let mut est_chave = String::new();
    let mut est_t = Instant::now() - Duration::from_secs(3600);
    while let Ok(primeira) = rx.recv() {
        // junta o que se acumulou: de cada arquivo, só a versão mais nova vira mensagem (ninguém veria as outras)
        let mut fila = vec![primeira];
        while let Ok(x) = rx.try_recv() { fila.push(x); }
        let mut arqs: Vec<(String, Bytes, Option<u64>)> = vec![];
        let mut pos: HashMap<String, usize> = HashMap::new();
        let mut outras = vec![];
        for e in fila {
            match e {
                Entrada::Arquivos(v) => for (n, b, t) in v {
                    match pos.get(&n) {
                        // a versão que sai da fila ainda pode fechar um ponto da curva (rajada da retotalização)
                        Some(&i) => { let ant = std::mem::replace(&mut arqs[i], (n, b, t)); h.amostrar_curva_superada(&ant.0, &ant.1); }
                        None => { pos.insert(n.clone(), arqs.len()); arqs.push((n, b, t)); }
                    }
                },
                o => outras.push(o),
            }
        }
        if !arqs.is_empty() { outras.insert(0, Entrada::Arquivos(arqs)); }
        let mut lat: Vec<f64> = vec![];
        for e in outras { match e {
            Entrada::Arquivos(v) => {
                for (nome, b, t_bu) in v {
                    let Some(t) = topico_de(&nome) else { continue };
                    let (t, a): (Arc<str>, Arc<str>) = (t.into(), nome.as_str().into());
                    let n = ver.entry(t.clone()).or_default();
                    *n += 1;
                    h.amostrar_curva(&t, &b);
                    let q = Quadro::novo(t.clone(), a, b, *n, t_bu);
                    if let Some(tb) = q.t_bu { lat.push(agora_us().saturating_sub(tb) as f64 / 1e3); }
                    let q = Arc::new(q);
                    let existe = h.topicos.read().unwrap().get(&t).map(|tx| { tx.send_replace(q.clone()); }).is_some();
                    if !existe { h.topicos.write().unwrap().insert(t, watch::Sender::new(q)); }
                }
            }
            Entrada::Eventos(v) => {
                let mut evs = h.eventos.write().unwrap();
                for e in v {
                    let n = evs.len() as u64;
                    let topico: Option<Arc<str>> = e.abrangencia.as_ref().zip(e.cargo).map(|(a, c)| format!("{a}-c{c:04}").into());
                    let mut valor = json!({"n": n, "classe": e.classe, "seq": e.seq, "horario": e.horario, "sistema": e.sistema, "abrangencia": e.abrangencia, "cargo": e.cargo,
                        "candidato": e.candidato, "estado": e.estado, "topico": topico.as_deref()});
                    if !e.motivo.is_empty() { valor["motivo"] = json!(e.motivo); }
                    let mut m = valor.clone();
                    m["tipo"] = json!("evento");
                    m["t_envio"] = json!(agora_us());
                    evs.push(Arc::new(Evento { n, topico, msg: Utf8Bytes::from(m.to_string()), valor }));
                }
                let n = evs.len();
                drop(evs);
                h.n_eventos.send_replace(n);
            }
            Entrada::Estado(mut v) => {
                v["clientes"] = json!(h.med.clientes.load(Relaxed));
                // muda de verdade: estado, pausa, conferências, tipos de alerta, velocidade do replay (não as contagens que
                // andam o tempo todo)
                let chave = format!("{}|{}|{}|{}|{:?}|{}|{}|{}", v["estado"], v["encerrada"], v["pausa_429"]["ativa"], v["conferencias"],
                    v["alertas"].as_object().map(|o| o.keys().cloned().collect::<Vec<_>>()), v["replay"],
                    v["relogio_sistema"]["sistema0"], v["relogio_sistema"]["acelerar"]);
                if chave == est_chave && est_t.elapsed() < h.op.estado_max { continue; }
                est_chave = chave;
                est_t = Instant::now();
                let corpo = Bytes::from(v.to_string());
                v["tipo"] = json!("estado");
                v["t_envio"] = json!(agora_us());
                h.estado.send_replace(Some(Arc::new(EstadoQ { corpo, msg: Utf8Bytes::from(v.to_string()) })));
            }
            Entrada::Marcos(v) => {
                let mut novos_hist = false;
                for mut valor in v {
                    let topico: Option<Arc<str>> = valor["abrangencia"].as_str().zip(valor["cargo"].as_u64()).map(|(a, c)| format!("{a}-c{c:04}").into());
                    valor["topico"] = json!(topico.as_deref());
                    // M-003: horário do sistema em que o marco saiu (o histórico da retomada vem com null: não se sabe)
                    if valor.get("sistema").is_none() { valor["sistema"] = json!(crate::relogio::iso_ms(crate::relogio::agora_ms())); }
                    if valor["classe"] == "faltam_para_mat" {
                        let Some(t) = topico else { continue };
                        let ver = *h.faltam_ver.borrow() + 1;
                        valor["n"] = json!(ver);
                        let mut m = valor.clone();
                        m["tipo"] = json!("marco");
                        m["t_envio"] = json!(agora_us());
                        h.faltam.write().unwrap().insert(t, Faltam { ver, msg: Utf8Bytes::from(m.to_string()), valor });
                        h.faltam_ver.send_replace(ver);
                    } else {
                        let mut ms = h.marcos.write().unwrap();
                        let n = ms.len() as u64;
                        valor["n"] = json!(n);
                        let mut m = valor.clone();
                        m["tipo"] = json!("marco");
                        m["t_envio"] = json!(agora_us());
                        ms.push(Arc::new(Evento { n, topico, msg: Utf8Bytes::from(m.to_string()), valor }));
                        novos_hist = true;
                    }
                }
                if novos_hist { let n = h.n_marcos(); h.n_marcos.send_replace(n); }
            }
            Entrada::Previsoes(v) => {
                let mut ver = *h.prev_ver.borrow();
                let mut ps = h.previsoes.write().unwrap();
                for mut valor in v {
                    let Some(t) = valor["topico"].as_str().map(Arc::<str>::from) else { continue };
                    ver += 1;
                    let corpo = Bytes::from(valor.to_string());
                    valor["tipo"] = json!("previsao");
                    valor["t_envio"] = json!(agora_us());
                    valor["instantaneo"] = json!(true);
                    let inst = Utf8Bytes::from(valor.to_string());
                    valor["instantaneo"] = json!(false);
                    if let Some(o) = valor.as_object_mut() { o.remove("historico"); }
                    ps.insert(t, Arc::new(Prev { ver, msg: Utf8Bytes::from(valor.to_string()), inst, corpo }));
                }
                drop(ps);
                h.prev_ver.send_replace(ver);
            }
            Entrada::Sonda(v) => h.guardar_sondas(v),
            Entrada::Secoes(v) => { if let Some(m) = h.mapa.get() { m.aplicar(&v); } }
        } }
        if !lat.is_empty() { let mut l = h.med.lat_difusao.lock().unwrap(); for x in lat { l.por(x); } }
        // o bloco `difusao` da saúde é calculado aqui (percentis de janelas de até 50 mil amostras), não na thread do
        // motor: ela só copia o último (ver `Hub::saude_guardada`)
        if h.saude_pedida.swap(false, Relaxed) { *h.saude.lock().unwrap() = h.saude_json(); }
    }
}

// ------------------------------------------------------------------ servidor HTTP/WebSocket

/// Serve até [`Hub::desligar`]. `pronto` recebe o endereço de escuta (porta efetiva se a pedida for 0).
pub async fn servir(h: Arc<Hub>, pronto: impl FnOnce(std::net::SocketAddr)) -> anyhow::Result<()> {
    use hyper::server::conn::http1;
    use hyper_util::rt::{TokioIo, TokioTimer};
    use hyper_util::service::TowerToHyperService;
    let app = Router::new()
        .route("/ws", get(ws))
        .route("/api/v1/estado", get(rest_estado))
        .route("/api/v1/topicos", get(rest_topicos))
        .route("/api/v1/resultado/{topico}", get(rest_resultado))
        .route("/api/v1/eventos", get(rest_eventos))
        .route("/api/v1/marcos", get(rest_marcos))
        .route("/api/v1/nomes", get(rest_nomes))
        .route("/api/v1/curva/{topico}", get(rest_curva))
        .route("/api/v1/previsao/{topico}", get(rest_previsao))
        .route("/api/v1/sonda", get(rest_sonda))
        .route("/api/v1/mapa/secoes", get(rest_mapa_secoes))
        .route("/api/v1/mapa/estado", get(rest_mapa_estado))
        .route("/api/v1/pleitos", get(rest_pleitos))
        .route("/consulta/{*caminho}", get(rest_consulta))
        // controle de velocidade do replay (só com --controle-replay; 404 sem ele)
        .route("/api/v1/replay", get(rest_replay))
        .route("/api/v1/replay/velocidade", post(rest_replay_velocidade))
        .route("/api/v1/replay/posicao", post(rest_replay_posicao))
        // página de resultados embutida (etapa 5): tabela fixa, nada do cliente chega ao disco
        .route("/", get(pagina_index))
        .route("/static/{arquivo}", get(pagina_estatico))
        .route("/termos", get(pagina_termos))
        .route("/privacidade", get(pagina_privacidade))
        .route("/robots.txt", get(pagina_robots))
        .route("/sitemap.xml", get(pagina_sitemap))
        .fallback(|| async { (StatusCode::NOT_FOUND, cors(), "rota desconhecida\n") })
        .with_state(h.clone());
    let ouvinte = tokio::net::TcpListener::bind(&h.op.endereco).await?;
    let a = ouvinte.local_addr()?;
    *h.escuta.lock().unwrap() = a.to_string();
    pronto(a);
    if let Some(r) = h.replay.get().cloned() {
        let mut d = h.desligar.subscribe();
        tokio::spawn(async move { tokio::select! { _ = r.acompanhar() => {}, _ = d.changed() => {} } });
    }
    let mut des = h.desligar.subscribe();
    let limite_tcp = (h.op.max_clientes + 256) as i64;
    loop {
        tokio::select! {
            r = ouvinte.accept() => {
                let Ok((s, _)) = r else { tokio::time::sleep(Duration::from_millis(10)).await; continue };
                if h.med.tcp_abertas.load(Relaxed) >= limite_tcp { h.med.tcp_recusadas.fetch_add(1, Relaxed); drop(s); continue; }
                let _ = s.set_nodelay(true);
                h.med.tcp_abertas.fetch_add(1, Relaxed);
                let (svc, h2) = (TowerToHyperService::new(app.clone()), h.clone());
                tokio::spawn(async move {
                    let c = http1::Builder::new().timer(TokioTimer::new()).header_read_timeout(h2.op.handshake).keep_alive(true)
                        .serve_connection(TokioIo::new(s), svc).with_upgrades();
                    let _ = c.await;
                    h2.med.tcp_abertas.fetch_sub(1, Relaxed);
                });
            }
            _ = des.changed() => break,
        }
    }
    drop(ouvinte);
    // as conexões mandam o aviso e fecham; espera até 3 s
    let t = Instant::now();
    while h.med.clientes.load(Relaxed) > 0 && t.elapsed() < Duration::from_secs(3) { tokio::time::sleep(Duration::from_millis(20)).await; }
    Ok(())
}

fn cors() -> [(header::HeaderName, HeaderValue); 2] {
    [(header::ACCESS_CONTROL_ALLOW_ORIGIN, HeaderValue::from_static("*")), (header::CONTENT_TYPE, HeaderValue::from_static("application/json; charset=utf-8"))]
}

async fn rest_estado(State(h): State<Arc<Hub>>) -> Response {
    h.med.rest.fetch_add(1, Relaxed);
    let e = h.estado.borrow().clone();
    match e {
        Some(e) => (cors(), [(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"))], e.corpo.clone()).into_response(),
        None => (StatusCode::SERVICE_UNAVAILABLE, cors(), "{\"erro\":\"estado ainda não calculado\"}").into_response(),
    }
}

async fn rest_topicos(State(h): State<Arc<Hub>>) -> Response {
    h.med.rest.fetch_add(1, Relaxed);
    let mut v: Vec<Value> = h.topicos.read().unwrap().values().map(|tx| { let q = tx.borrow(); json!({"topico": &*q.topico, "arquivo": &*q.arquivo, "seq": q.m().map(|m| m.seq)}) }).collect();
    v.sort_by(|a, b| a["topico"].as_str().cmp(&b["topico"].as_str()));
    (cors(), [(header::CACHE_CONTROL, HeaderValue::from_static("public, max-age=1"))], Value::Array(v).to_string()).into_response()
}

async fn rest_resultado(State(h): State<Arc<Hub>>, Path(t): Path<String>, cab: HeaderMap) -> Response {
    h.med.rest.fetch_add(1, Relaxed);
    // só a tabela de tópicos: nenhum caminho vindo do cliente chega ao disco
    let q = h.topicos.read().unwrap().get(t.as_str()).map(|tx| tx.borrow().clone());
    let Some(q) = q else { return (StatusCode::NOT_FOUND, cors(), "{\"erro\":\"topico_desconhecido\"}").into_response() };
    let Some(seq) = q.m().map(|m| m.seq) else { return (StatusCode::INTERNAL_SERVER_ERROR, cors(), "{\"erro\":\"resultado_ilegivel\"}").into_response() };
    let etag = format!("\"{seq}\"");
    let etag_v = HeaderValue::from_str(&etag).unwrap_or(HeaderValue::from_static("\"0\""));
    let cache = (header::CACHE_CONTROL, HeaderValue::from_static("public, max-age=1"));
    if cab.get(header::IF_NONE_MATCH).and_then(|v| v.to_str().ok()).is_some_and(|v| v.split(',').any(|x| x.trim() == etag || x.trim() == "*")) {
        h.med.rest_304.fetch_add(1, Relaxed);
        return (StatusCode::NOT_MODIFIED, cors(), [(header::ETAG, etag_v), cache]).into_response();
    }
    (cors(), [(header::ETAG, etag_v), cache], q.json.clone()).into_response()
}

async fn rest_eventos(State(h): State<Arc<Hub>>) -> Response {
    h.med.rest.fetch_add(1, Relaxed);
    let v: Vec<Value> = h.eventos.read().unwrap().iter().map(|e| e.valor.clone()).collect();
    (cors(), [(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"))], Value::Array(v).to_string()).into_response()
}

async fn rest_marcos(State(h): State<Arc<Hub>>) -> Response {
    h.med.rest.fetch_add(1, Relaxed);
    let mut v: Vec<Value> = h.marcos.read().unwrap().iter().map(|e| e.valor.clone()).collect();
    v.extend(h.faltam.read().unwrap().values().map(|f| f.valor.clone()));
    (cors(), [(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"))], Value::Array(v).to_string()).into_response()
}

async fn rest_nomes(State(h): State<Arc<Hub>>, cab: HeaderMap) -> Response {
    h.med.rest.fetch_add(1, Relaxed);
    let n = h.nomes.read().unwrap().clone();
    let Some((corpo, etag)) = n else { return (StatusCode::SERVICE_UNAVAILABLE, cors(), "{\"erro\":\"nomes indisponíveis\"}").into_response() };
    let etag_v = HeaderValue::from_str(&etag).unwrap_or(HeaderValue::from_static("\"0\""));
    let cache = (header::CACHE_CONTROL, HeaderValue::from_static("public, max-age=60"));
    if cab.get(header::IF_NONE_MATCH).and_then(|v| v.to_str().ok()).is_some_and(|v| v.split(',').any(|x| x.trim() == etag)) {
        h.med.rest_304.fetch_add(1, Relaxed);
        return (StatusCode::NOT_MODIFIED, cors(), [(header::ETAG, etag_v), cache]).into_response();
    }
    (cors(), [(header::ETAG, etag_v), cache], corpo).into_response()
}

/// Corpo e `ETag` do `pleitos.json`, relido só quando a data de modificação muda; `None` sem o arquivo (ou ilegível).
fn pleitos_atual(h: &Hub) -> Option<(Bytes, String)> {
    use sha2::Digest;
    let p = h.op.pleitos.as_ref()?;
    let modif = std::fs::metadata(p).and_then(|m| m.modified()).ok()?;
    let mut c = h.pleitos.lock().unwrap();
    if let Some((m, corpo, etag)) = c.as_ref() { if *m == modif { return Some((corpo.clone(), etag.clone())); } }
    let corpo = std::fs::read(p).ok()?;
    serde_json::from_slice::<Value>(&corpo).ok()?["pleitos"].as_array()?;
    let etag = format!("\"{}\"", sha2::Sha256::digest(&corpo).iter().take(8).map(|b| format!("{b:02x}")).collect::<String>());
    let corpo = Bytes::from(corpo);
    *c = Some((modif, corpo.clone(), etag.clone()));
    Some((corpo, etag))
}

/// `GET /api/v1/pleitos` (multipleito, 1.1): os pleitos do `ele-c.json` com a situação de cada um (`crate::pleitos`).
async fn rest_pleitos(State(h): State<Arc<Hub>>, cab: HeaderMap) -> Response {
    h.med.rest.fetch_add(1, Relaxed);
    let Some((corpo, etag)) = pleitos_atual(&h) else {
        return (StatusCode::NOT_FOUND, cors(), "{\"erro\":\"sem lista de pleitos (uv-servico --pleitos)\"}").into_response();
    };
    let etag_v = HeaderValue::from_str(&etag).unwrap_or(HeaderValue::from_static("\"0\""));
    let cache = (header::CACHE_CONTROL, HeaderValue::from_static("public, max-age=60"));
    if cab.get(header::IF_NONE_MATCH).and_then(|v| v.to_str().ok()).is_some_and(|v| v.split(',').any(|x| x.trim() == etag)) {
        h.med.rest_304.fetch_add(1, Relaxed);
        return (StatusCode::NOT_MODIFIED, cors(), [(header::ETAG, etag_v), cache]).into_response();
    }
    (cors(), [(header::ETAG, etag_v), cache], corpo).into_response()
}

/// `GET /consulta/<caminho>` (multipleito 1.1): os arquivos estáticos das séries de consulta (`--consulta <pasta>`), só
/// `.json` com nome simples (letras, dígitos, `-`, `_`, `.` e `/`, sem `..`), com o `.json.gz` pré-comprimido para quem
/// aceita gzip. Cache: 5 min no índice geral, 1 h nos arquivos de um pleito (imutáveis depois de gerados).
async fn rest_consulta(State(h): State<Arc<Hub>>, Path(c): Path<String>, cab: HeaderMap) -> Response {
    h.med.rest.fetch_add(1, Relaxed);
    let nao = || (StatusCode::NOT_FOUND, cors(), "{\"erro\":\"nao_encontrado\"}").into_response();
    let Some(raiz) = h.op.consulta.clone() else { return nao() };
    let simples = !c.is_empty() && c.len() <= 200 && c.ends_with(".json")
        && c.bytes().all(|b| b.is_ascii_alphanumeric() || b"-_./".contains(&b))
        && c.split('/').all(|p| !p.is_empty() && !p.starts_with('.'));
    if !simples { return nao(); }
    let p = raiz.join(&c);
    let gzip = cab.get(header::ACCEPT_ENCODING).and_then(|v| v.to_str().ok()).is_some_and(|v| v.contains("gzip"));
    // série de um pleito em curso (marcador EM_CURSO do uv-serie --em-curso): refeita a cada poucos minutos, cache curto
    let em_curso = c.split('/').next().is_some_and(|p| raiz.join(p).join("serie").join("EM_CURSO").exists());
    let cache = HeaderValue::from_static(if em_curso { "public, max-age=30" } else if c == "indice.json" { "public, max-age=300" } else { "public, max-age=3600" });
    let vary = (header::VARY, HeaderValue::from_static("Accept-Encoding"));
    if gzip {
        if let Ok(b) = tokio::fs::read(format!("{}.gz", p.display())).await {
            return (cors(), [(header::CACHE_CONTROL, cache), (header::CONTENT_ENCODING, HeaderValue::from_static("gzip")), vary], b).into_response();
        }
    }
    match tokio::fs::read(&p).await {
        Ok(b) => (cors(), [(header::CACHE_CONTROL, cache), vary], b).into_response(),
        Err(_) => nao(),
    }
}

/// `GET /api/v1/mapa/secoes` (mapa das seções, `crate::mapa`): a lista das seções do catálogo, fixa por processo.
async fn rest_mapa_secoes(State(h): State<Arc<Hub>>, cab: HeaderMap) -> Response {
    h.med.rest.fetch_add(1, Relaxed);
    let Some(m) = h.mapa.get() else { return (StatusCode::SERVICE_UNAVAILABLE, cors(), "{\"erro\":\"mapa indisponível\"}").into_response() };
    let etag_v = HeaderValue::from_str(&m.etag).unwrap_or(HeaderValue::from_static("\"0\""));
    let cache = (header::CACHE_CONTROL, HeaderValue::from_static("public, max-age=300"));
    if cab.get(header::IF_NONE_MATCH).and_then(|v| v.to_str().ok()).is_some_and(|v| v.split(',').any(|x| x.trim() == m.etag)) {
        h.med.rest_304.fetch_add(1, Relaxed);
        return (StatusCode::NOT_MODIFIED, cors(), [(header::ETAG, etag_v), cache]).into_response();
    }
    (cors(), [(header::ETAG, etag_v), cache], m.secoes.clone()).into_response()
}

/// `GET /api/v1/mapa/estado`: fotografia das seções acesas (`{"epoca", "total", "n", "fora", "h"}`, `crate::mapa`).
async fn rest_mapa_estado(State(h): State<Arc<Hub>>) -> Response {
    h.med.rest.fetch_add(1, Relaxed);
    let Some(m) = h.mapa.get() else { return (StatusCode::SERVICE_UNAVAILABLE, cors(), "{\"erro\":\"mapa indisponível\"}").into_response() };
    (cors(), [(header::CACHE_CONTROL, HeaderValue::from_static("public, max-age=5"))], m.estado()).into_response()
}

/// `GET /api/v1/curva/<topico>`: `{"topico", "passo_s", "relogio", "pontos": [[t, apuradas, total, [[n, pct], …]], …]}`
/// (t em segundos, com fração, na hora de Brasília; desde a M-003 no relógio do sistema — `relogio: "sistema"` —, com
/// `passo_s` = um minuto da noite nesse relógio); tópico sem publicação → 404.
async fn rest_curva(State(h): State<Arc<Hub>>, Path(t): Path<String>) -> Response {
    h.med.rest.fetch_add(1, Relaxed);
    let corpo = {
        let m = h.curvas.lock().unwrap();
        let Some(v) = m.get(t.as_str()) else { return (StatusCode::NOT_FOUND, cors(), "{\"erro\":\"topico_desconhecido\"}").into_response() };
        let pontos: Vec<Value> = v.iter().map(|p| json!([(p.t * 1000.0).round() / 1000.0, p.ap, p.tot, p.cand.iter().map(|(n, x)| json!([n, x])).collect::<Vec<_>>()])).collect();
        json!({"topico": t, "passo_s": h.passo_curva(true), "relogio": if h.relogio.get().is_some() { "sistema" } else { "site" }, "pontos": pontos}).to_string()
    };
    (cors(), [(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"))], corpo).into_response()
}

/// `GET /api/v1/previsao/<topico>` (etapa 8): a última previsão do tópico, com o histórico (o objeto da §4.7 sem
/// `tipo`/`t_envio`); sem previsão ainda (pouco apurado) ou tópico inexistente → 404.
async fn rest_previsao(State(h): State<Arc<Hub>>, Path(t): Path<String>) -> Response {
    h.med.rest.fetch_add(1, Relaxed);
    let p = h.previsoes.read().unwrap().get(t.as_str()).cloned();
    match p {
        Some(p) => (cors(), [(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"))], p.corpo.clone()).into_response(),
        None => (StatusCode::NOT_FOUND, cors(), "{\"erro\":\"sem_previsao\"}").into_response(),
    }
}

async fn rest_sonda(State(h): State<Arc<Hub>>) -> Response {
    h.med.rest.fetch_add(1, Relaxed);
    let mut r = (StatusCode::OK, cors(), h.sondas_json().to_string()).into_response();
    r.headers_mut().insert(header::CONTENT_TYPE, HeaderValue::from_static("application/json"));
    r.headers_mut().insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    r
}

fn cab_replay() -> [(header::HeaderName, HeaderValue); 2] {
    [(header::CONTENT_TYPE, HeaderValue::from_static("application/json; charset=utf-8")), (header::CACHE_CONTROL, HeaderValue::from_static("no-store"))]
}

async fn rest_replay(State(h): State<Arc<Hub>>) -> Response {
    h.med.rest.fetch_add(1, Relaxed);
    match h.replay.get() {
        Some(r) => (cab_replay(), r.json().to_string()).into_response(),
        None => (StatusCode::NOT_FOUND, cab_replay(), "{\"erro\":\"sem controle de replay (uv-servico --controle-replay)\"}").into_response(),
    }
}

/// Corpo JSON de um pedido de controle do replay, ou a resposta de erro. Sem `Access-Control-Allow-Origin`: só a própria
/// página comanda o replay — o corpo precisa ser `application/json` (outra origem exigiria a pré-verificação CORS, que
/// não é atendida) e, quando o navegador manda `Origin`, ela tem de ser este mesmo endereço.
fn pedido_replay(h: &Hub, cab: &HeaderMap, corpo: &Bytes) -> Result<(Arc<crate::controle_replay::Replay>, Value), Response> {
    let erro = |st: StatusCode, m: &str| (st, cab_replay(), json!({"erro": m}).to_string()).into_response();
    let Some(r) = h.replay.get().cloned() else { return Err(erro(StatusCode::NOT_FOUND, "sem controle de replay (uv-servico --controle-replay)")) };
    if !cab.get(header::CONTENT_TYPE).and_then(|v| v.to_str().ok()).is_some_and(|v| v.trim_start().starts_with("application/json")) {
        return Err(erro(StatusCode::UNSUPPORTED_MEDIA_TYPE, "Content-Type: application/json"));
    }
    if let Some(o) = cab.get(header::ORIGIN).and_then(|v| v.to_str().ok()) {
        let host = cab.get(header::HOST).and_then(|v| v.to_str().ok()).unwrap_or("");
        if o.split_once("://").map(|x| x.1) != Some(host) { return Err(erro(StatusCode::FORBIDDEN, "origem diferente da página")); }
    }
    match serde_json::from_slice::<Value>(corpo) {
        Ok(v) => Ok((r, v)),
        Err(_) => Err(erro(StatusCode::BAD_REQUEST, "corpo JSON inválido")),
    }
}

fn resposta_pedido(r: &crate::controle_replay::Replay, res: Result<Value, crate::controle_replay::Pedido>) -> Response {
    use crate::controle_replay::Pedido;
    match res {
        Ok(extra) => {
            let mut v = r.json();
            if let Value::Object(m) = extra { for (k, x) in m { v[k] = x; } }
            (cab_replay(), v.to_string()).into_response()
        }
        Err(Pedido::Invalido(m)) => (StatusCode::BAD_REQUEST, cab_replay(), json!({"erro": m}).to_string()).into_response(),
        Err(Pedido::Simulador(m)) => (StatusCode::BAD_GATEWAY, cab_replay(), json!({"erro": format!("simulador: {m}")}).to_string()).into_response(),
    }
}

/// `POST /api/v1/replay/velocidade` com `{"acelerar": x}` (0 = pausa; 1 a 300).
async fn rest_replay_velocidade(State(h): State<Arc<Hub>>, cab: HeaderMap, corpo: Bytes) -> Response {
    h.med.rest.fetch_add(1, Relaxed);
    let (r, v) = match pedido_replay(&h, &cab, &corpo) { Ok(x) => x, Err(e) => return e };
    let Some(x) = v.get("acelerar").and_then(Value::as_f64) else { return resposta_pedido(&r, Err(crate::controle_replay::Pedido::Invalido("acelerar: 0 (pausa) ou de 1 a 300"))) };
    let res = r.velocidade(x).await.map(|_| Value::Null);
    resposta_pedido(&r, res)
}

/// `POST /api/v1/replay/posicao` com `{"t": segundos}` (hora de Brasília, como `inicio`/`fim` de `GET /api/v1/replay`):
/// para a frente, busca; para trás, o serviço recomeça do zero (`{"reinicio": true}`) e as páginas se recarregam.
async fn rest_replay_posicao(State(h): State<Arc<Hub>>, cab: HeaderMap, corpo: Bytes) -> Response {
    h.med.rest.fetch_add(1, Relaxed);
    let (r, v) = match pedido_replay(&h, &cab, &corpo) { Ok(x) => x, Err(e) => return e };
    let Some(t) = v.get("t").and_then(Value::as_f64).filter(|t| t.is_finite()) else { return resposta_pedido(&r, Err(crate::controle_replay::Pedido::Invalido("t: segundos"))) };
    let res = r.posicao(t as i64).await.map(|reinicio| json!({"reinicio": reinicio}));
    resposta_pedido(&r, res)
}

fn cab_pagina(tipo: &'static str, cache: &'static str) -> [(header::HeaderName, HeaderValue); 5] {
    [(header::CONTENT_TYPE, HeaderValue::from_static(tipo)), (header::CACHE_CONTROL, HeaderValue::from_static(cache)),
     (header::CONTENT_SECURITY_POLICY, HeaderValue::from_static(crate::pagina::CSP)), (header::X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff")),
     (header::REFERRER_POLICY, HeaderValue::from_static("no-referrer"))]
}

async fn pagina_index(State(h): State<Arc<Hub>>) -> Response {
    h.med.rest.fetch_add(1, Relaxed);
    (cab_pagina("text/html; charset=utf-8", "no-cache"), crate::pagina::index()).into_response()
}

async fn pagina_termos(State(h): State<Arc<Hub>>) -> Response {
    h.med.rest.fetch_add(1, Relaxed);
    (cab_pagina("text/html; charset=utf-8", "no-cache"), crate::pagina::termos()).into_response()
}

async fn pagina_privacidade(State(h): State<Arc<Hub>>) -> Response {
    h.med.rest.fetch_add(1, Relaxed);
    (cab_pagina("text/html; charset=utf-8", "no-cache"), crate::pagina::privacidade()).into_response()
}

async fn pagina_estatico(State(h): State<Arc<Hub>>, Path(a): Path<String>, cab: HeaderMap) -> Response {
    h.med.rest.fetch_add(1, Relaxed);
    // só um nome exato da tabela embutida
    let Some(r) = crate::pagina::estatico(&format!("/static/{a}")) else {
        return (StatusCode::NOT_FOUND, cors(), "rota desconhecida\n").into_response();
    };
    let vary = (header::VARY, HeaderValue::from_static("Accept-Encoding"));
    // gzip quando o navegador aceita (M-017: o mapa.json tem ~3 MB; o app.js também ganha)
    if crate::pagina::aceita_gzip(cab.get(header::ACCEPT_ENCODING).and_then(|v| v.to_str().ok())) {
        if let Some(z) = crate::pagina::gzip(r) {
            return (cab_pagina(r.tipo, "public, max-age=31536000, immutable"), [vary, (header::CONTENT_ENCODING, HeaderValue::from_static("gzip"))], z).into_response();
        }
    }
    (cab_pagina(r.tipo, "public, max-age=31536000, immutable"), [vary], Bytes::from_static(r.corpo)).into_response()
}

async fn pagina_robots(State(h): State<Arc<Hub>>) -> Response {
    h.med.rest.fetch_add(1, Relaxed);
    ([(header::CONTENT_TYPE, HeaderValue::from_static("text/plain; charset=utf-8")),
      (header::CACHE_CONTROL, HeaderValue::from_static("public, max-age=86400")),
      (header::X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"))],
     crate::pagina::robots()).into_response()
}

async fn pagina_sitemap(State(h): State<Arc<Hub>>) -> Response {
    h.med.rest.fetch_add(1, Relaxed);
    ([(header::CONTENT_TYPE, HeaderValue::from_static("application/xml; charset=utf-8")),
      (header::CACHE_CONTROL, HeaderValue::from_static("public, max-age=43200")),
      (header::X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"))],
     crate::pagina::sitemap()).into_response()
}

async fn ws(State(h): State<Arc<Hub>>, up: WebSocketUpgrade) -> Response {
    if h.desligando.load(Relaxed) { return (StatusCode::SERVICE_UNAVAILABLE, "desligando\n").into_response(); }
    if h.med.clientes.fetch_add(1, Relaxed) >= h.op.max_clientes as i64 {
        h.med.clientes.fetch_sub(1, Relaxed);
        h.med.recusadas.fetch_add(1, Relaxed);
        return (StatusCode::SERVICE_UNAVAILABLE, "limite de clientes\n").into_response();
    }
    let h2 = h.clone();
    up.max_message_size(h.op.max_msg).max_frame_size(h.op.max_msg).write_buffer_size(64 * 1024).max_write_buffer_size(h.op.buffer_escrita)
        .on_failed_upgrade(move |_| { h2.med.clientes.fetch_sub(1, Relaxed); h2.desconexao("handshake"); })
        .on_upgrade(move |s| async move {
            let motivo = conexao(&h, s).await;
            h.med.clientes.fetch_sub(1, Relaxed);
            h.desconexao(motivo);
        })
}

/// Intervalo mínimo entre dois avisos `seq_pulado` do mesmo tópico numa conexão: com o `br-c0001` mudando ~100 vezes
/// por segundo a 60×, quase toda conexão junta versões, e um aviso por pulo dobrava as mensagens (D1 da `0.3.2`:
/// 21,9 M avisos em 50,9 M mensagens). O aviso leva a soma das versões juntadas desde o anterior.
const AVISO_PULO: Duration = Duration::from_secs(5);

/// Prioridade menor (nice 10) para a thread que chama: as threads da camada HTTP cedem a CPU ao motor, à gravação e à
/// coleta (D1 da `0.3.2`, sem isso: lote do motor p50 0,055 → 0,335 ms com 1.000 clientes nos mesmos núcleos).
pub fn prioridade_baixa() {
    #[cfg(target_os = "linux")]
    unsafe { let _ = libc::setpriority(libc::PRIO_PROCESS, libc::gettid() as libc::id_t, 10); }
}

/// Comando do leitor de uma conexão para a tarefa principal dela.
enum Cmd { Op(Value), Erro(&'static str, String), Fim(&'static str) }

fn msg_erro(motivo: &str, detalhe: &str) -> Utf8Bytes { Utf8Bytes::from(json!({"tipo": "erro", "motivo": motivo, "detalhe": detalhe}).to_string()) }

/// Escreve uma mensagem; `Err(motivo)` = a conexão deve cair.
async fn escrever(h: &Hub, tx: &mut futures::stream::SplitSink<WebSocket, Message>, m: Message) -> Result<(), &'static str> {
    let n = match &m { Message::Text(t) => t.len(), Message::Binary(b) => b.len(), _ => 0 } as u64;
    match tokio::time::timeout(h.op.lento, tx.send(m)).await {
        Err(_) => Err("cliente_lento"),
        Ok(Err(e)) => Err(if e.to_string().to_lowercase().contains("buffer") { "cliente_lento" } else { "erro_escrita" }),
        Ok(Ok(())) => { if n > 0 { h.med.msgs.fetch_add(1, Relaxed); h.med.bytes.fetch_add(n, Relaxed); } Ok(()) }
    }
}

async fn fechar(h: &Hub, tx: &mut futures::stream::SplitSink<WebSocket, Message>, codigo: u16, motivo: &'static str) -> &'static str {
    let _ = tokio::time::timeout(Duration::from_secs(1), tx.send(Message::Close(Some(CloseFrame { code: codigo, reason: Utf8Bytes::from_static(motivo) })))).await;
    let _ = h;
    motivo
}

/// Leitor de uma conexão: limites de tamanho (no próprio WebSocket), de taxa e de formato; só repassa comandos.
async fn ler(h: Arc<Hub>, mut rx: futures::stream::SplitStream<WebSocket>, cmd: tokio::sync::mpsc::Sender<Cmd>, ultimo: Arc<AtomicU64>) {
    let t0 = Instant::now();
    let (mut fichas, mut t_fichas, mut excessos) = (h.op.rajada, Instant::now(), 0u32);
    let fim = |m| { let _ = cmd.try_send(Cmd::Fim(m)); };
    while let Some(m) = rx.next().await {
        ultimo.store(t0.elapsed().as_millis() as u64, Relaxed);
        let texto = match m {
            Err(e) => {
                let s = e.to_string().to_lowercase();
                return fim(if s.contains("too long") || s.contains("capacity") || s.contains("space limit") || s.contains("size") { "mensagem_grande" } else { "erro_leitura" });
            }
            Ok(Message::Close(_)) => return fim("cliente_fechou"),
            Ok(Message::Text(t)) => t,
            Ok(Message::Binary(_)) => { h.med.erros_cliente.fetch_add(1, Relaxed); let _ = cmd.try_send(Cmd::Erro("binario", "só mensagens de texto JSON".into())); continue; }
            Ok(_) => continue,
        };
        let agora = Instant::now();
        fichas = (fichas + (agora - t_fichas).as_secs_f64() * h.op.msgs_s).min(h.op.rajada);
        t_fichas = agora;
        if fichas < 1.0 {
            excessos += 1;
            h.med.erros_cliente.fetch_add(1, Relaxed);
            if excessos >= h.op.excessos { return fim("taxa"); }
            let _ = cmd.try_send(Cmd::Erro("taxa", format!("mais de {} mensagens/s", h.op.msgs_s)));
            continue;
        }
        fichas -= 1.0;
        let c = match serde_json::from_str::<Value>(&texto) {
            Ok(v) if v.is_object() => Cmd::Op(v),
            _ => { h.med.erros_cliente.fetch_add(1, Relaxed); Cmd::Erro("json_invalido", "esperado um objeto JSON".into()) }
        };
        // canal cheio = o cliente manda mais rápido do que a conexão consegue responder: conta como excesso
        if cmd.try_send(c).is_err() { excessos += 1; if excessos >= h.op.excessos { return fim("taxa"); } }
    }
    fim("cliente_fechou")
}

/// Uma conexão WebSocket do começo ao fim. Devolve o motivo do fim.
async fn conexao(h: &Arc<Hub>, s: WebSocket) -> &'static str {
    h.med.conexoes.fetch_add(1, Relaxed);
    let (mut tx, rx) = s.split();
    let t0 = Instant::now();
    let ultimo = Arc::new(AtomicU64::new(0));
    let (ctx, mut crx) = tokio::sync::mpsc::channel::<Cmd>(64);
    let leitor = tokio::spawn(ler(h.clone(), rx, ctx, ultimo.clone()));
    let r = conexao_laco(h, &mut tx, &mut crx, &ultimo, t0).await;
    leitor.abort();
    r
}

async fn conexao_laco(h: &Arc<Hub>, tx: &mut futures::stream::SplitSink<WebSocket, Message>, crx: &mut tokio::sync::mpsc::Receiver<Cmd>, ultimo: &AtomicU64, t0: Instant) -> &'static str {
    let mut subs: StreamMap<Arc<str>, WatchStream<Arc<Quadro>>> = StreamMap::new();
    // comparação com o TSE (etapa 7): `tse:<tópico>` → inscrição (enquanto existir, o servidor busca o arquivo do TSE)
    // e o receptor (para o instantâneo de uma nova inscrição no mesmo tópico); versão da última mensagem enviada
    type QTse = Option<Arc<crate::tse_resultados::Quadro>>;
    let mut tse_subs: StreamMap<Arc<str>, WatchStream<QTse>> = StreamMap::new();
    let mut tse_insc: HashMap<Arc<str>, (crate::tse_resultados::Inscricao, watch::Receiver<QTse>)> = HashMap::new();
    let mut tse_ver: HashMap<Arc<str>, u64> = HashMap::new();
    // tópico → (ver, seq) da última mensagem enviada
    let mut ult: HashMap<Arc<str>, (u64, u64)> = HashMap::new();
    // tópico → (juntadas ainda não avisadas, último aviso): no máximo um `seq_pulado` por tópico a cada AVISO_PULO
    let mut pulos: HashMap<Arc<str>, (u64, Option<Instant>)> = HashMap::new();
    let mut ev_todos = false;
    let mut ev_env: HashSet<u64> = HashSet::new();
    let mut ev_rx = h.n_eventos.subscribe();
    let mut ev_prox = *ev_rx.borrow_and_update();
    let mut est_rx = h.estado.subscribe();
    // marcos (`op: marcos`): próximo índice do histórico a mandar e a versão do último `faltam_para_mat` mandado
    let mut mc_todos = false;
    let mut mc_rx = h.n_marcos.subscribe();
    let mut mc_prox = 0usize;
    let mut fa_rx = h.faltam_ver.subscribe();
    let mut fa_ver = 0u64;
    // `faltam_para_mat` só destes tópicos (`op: marcos` com `faltam: [...]`, B-014); `None` = de todas as circunscrições
    let mut fa_quero: Option<HashSet<Arc<str>>> = None;
    // `escopo: "uf"` em `eventos`/`marcos` (B-014): de fora dos tópicos inscritos, só os do Brasil e das UFs
    let (mut ev_uf, mut mc_uf) = (false, false);
    let amplo = |e: &Evento| e.valor["abrangencia"].as_str().is_none_or(|a| a.len() <= 2);
    // previsão (etapa 8): versão da última mensagem `previsao` mandada por tópico inscrito
    let mut pv_rx = h.prev_ver.subscribe();
    let mut pv_env: HashMap<Arc<str>, u64> = HashMap::new();
    // sondas (`op: sonda`, §4.9): ligadas nesta conexão e quantas guardadas já se viu (o contador do hub)
    let mut sd_ligada = false;
    let mut sd_rx = h.n_sondas.subscribe();
    let mut sd_visto = 0u64;
    // mapa das seções (`op: mapa`, `crate::mapa`): nº de seções acesas já mandadas, mensagem pendente e o instante a
    // partir do qual pode sair a próxima (no máximo uma a cada `INTERVALO_MS`)
    let mut mp_rx: Option<watch::Receiver<usize>> = None;
    let mut mp_prox = 0usize;
    let mut mp_pend = false;
    let mut mp_pode = tokio::time::Instant::now();
    let mut des = h.desligar.subscribe();
    let mut ping = tokio::time::interval_at(tokio::time::Instant::now() + h.op.ping, h.op.ping);
    ping.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let fim = |subs: &StreamMap<_, _>, m| { h.med.inscricoes.fetch_sub(subs.len() as i64, Relaxed); m };
    let e = est_rx.borrow_and_update().clone();
    if let Some(e) = e { if let Err(m) = escrever(h, tx, Message::Text(e.msg.clone())).await { return m; } }
    if *des.borrow_and_update() { return fechar(h, tx, 1001, "desligamento").await; }
    macro_rules! enviar { ($m:expr) => { if let Err(m) = escrever(h, tx, $m).await { return fim(&subs, m); } }; }
    loop {
        tokio::select! {
            biased;
            _ = des.changed() => {
                let _ = escrever(h, tx, Message::Text(Utf8Bytes::from_static(r#"{"tipo":"aviso","motivo":"desligamento"}"#))).await;
                let m = fechar(h, tx, 1001, "desligamento").await;
                return fim(&subs, m);
            }
            c = crx.recv() => match c {
                None => return fim(&subs, "cliente_fechou"),
                Some(Cmd::Fim(m)) => {
                    let m = if m == "cliente_fechou" || m == "erro_leitura" { m } else { fechar(h, tx, if m == "mensagem_grande" { 1009 } else { 1008 }, m).await };
                    return fim(&subs, m);
                }
                Some(Cmd::Erro(m, d)) => enviar!(Message::Text(msg_erro(m, &d))),
                Some(Cmd::Op(v)) => {
                    let lista: Vec<String> = v["topicos"].as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default();
                    match v["op"].as_str() {
                        Some("inscrever") => {
                            for t in lista {
                                if let Some(base) = t.strip_prefix("tse:") {
                                    let chave: Arc<str> = Arc::from(t.as_str());
                                    let novo = !tse_insc.contains_key(&chave);
                                    if novo && subs.len() + tse_insc.len() >= h.op.max_topicos {
                                        h.med.erros_cliente.fetch_add(1, Relaxed);
                                        enviar!(Message::Text(msg_erro("limite_topicos", &format!("no máximo {} tópicos por conexão", h.op.max_topicos))));
                                        break;
                                    }
                                    let Some(tse) = h.tse.get() else {
                                        h.med.erros_cliente.fetch_add(1, Relaxed);
                                        enviar!(Message::Text(msg_erro("comparacao_desligada", "este servidor não compara com o TSE")));
                                        continue;
                                    };
                                    let rx = if novo {
                                        match tse.inscrever(base) {
                                            Ok((g, rx)) => { tse_insc.insert(chave.clone(), (g, rx.clone())); tse_subs.insert(chave.clone(), WatchStream::from_changes(rx.clone())); rx }
                                            Err(m) => {
                                                h.med.erros_cliente.fetch_add(1, Relaxed);
                                                enviar!(Message::Text(msg_erro(m, &t.chars().take(64).collect::<String>())));
                                                continue;
                                            }
                                        }
                                    } else { tse_insc[&chave].1.clone() };
                                    let q = rx.borrow().clone();
                                    if let Some(q) = q { tse_ver.insert(chave, q.ver); enviar!(Message::Text(q.instantaneo.clone())); }
                                    continue;
                                }
                                let novo = !subs.contains_key(t.as_str());
                                if novo && subs.len() + tse_insc.len() >= h.op.max_topicos {
                                    h.med.erros_cliente.fetch_add(1, Relaxed);
                                    enviar!(Message::Text(msg_erro("limite_topicos", &format!("no máximo {} tópicos por conexão", h.op.max_topicos))));
                                    break;
                                }
                                let rx = h.topicos.read().unwrap().get(t.as_str()).map(|s| s.subscribe());
                                let Some(mut rx) = rx else {
                                    h.med.erros_cliente.fetch_add(1, Relaxed);
                                    enviar!(Message::Text(msg_erro("topico_desconhecido", &t.chars().take(64).collect::<String>())));
                                    continue;
                                };
                                let q = rx.borrow_and_update().clone();
                                let chave = q.topico.clone();
                                let (Some(ins), Some(seq)) = (q.instantaneo(), q.m().map(|m| m.seq)) else {
                                    enviar!(Message::Text(msg_erro("resultado_ilegivel", &chave)));
                                    continue;
                                };
                                enviar!(Message::Text(ins));
                                ult.insert(chave.clone(), (q.ver, seq));
                                if novo { h.med.inscricoes.fetch_add(1, Relaxed); }
                                subs.insert(chave.clone(), WatchStream::from_changes(rx));
                                // histórico de eventos do tópico
                                let hist: Vec<Arc<Evento>> = h.eventos.read().unwrap()[..ev_prox].iter().filter(|e| e.topico.as_deref() == Some(&*chave) && !ev_env.contains(&e.n)).cloned().collect();
                                for e in hist { ev_env.insert(e.n); enviar!(Message::Text(e.msg.clone())); }
                                // a última previsão do tópico, com o histórico da noite
                                let _ = pv_rx.borrow_and_update();
                                let pv = h.previsoes.read().unwrap().get(&chave).cloned();
                                if let Some(p) = pv { pv_env.insert(chave.clone(), p.ver); enviar!(Message::Text(p.inst.clone())); }
                            }
                        }
                        Some("cancelar") => {
                            for t in lista {
                                if t.starts_with("tse:") {
                                    // sem a inscrição, o tópico perde o inscrito (e, sem nenhum, não é mais buscado)
                                    tse_subs.remove(t.as_str());
                                    tse_insc.remove(t.as_str());
                                    tse_ver.remove(t.as_str());
                                    continue;
                                }
                                if subs.remove(t.as_str()).is_some() { h.med.inscricoes.fetch_sub(1, Relaxed); ult.remove(t.as_str()); pv_env.remove(t.as_str()); }
                            }
                        }
                        Some("eventos") => {
                            ev_todos = true;
                            ev_uf = v["escopo"] == "uf";
                            let hist: Vec<Arc<Evento>> = h.eventos.read().unwrap()[..ev_prox].iter()
                                .filter(|e| !ev_env.contains(&e.n) && (!ev_uf || amplo(e) || e.topico.as_ref().is_some_and(|t| subs.contains_key(t)))).cloned().collect();
                            for e in hist { ev_env.insert(e.n); enviar!(Message::Text(e.msg.clone())); }
                        }
                        Some("marcos") => {
                            // histórico inteiro (só na primeira vez) e o último `faltam_para_mat` de cada circunscrição
                            mc_todos = true;
                            mc_uf = v["escopo"] == "uf";
                            let _ = mc_rx.borrow_and_update();
                            let hist: Vec<Arc<Evento>> = h.marcos.read().unwrap()[mc_prox..].to_vec();
                            mc_prox += hist.len();
                            let hist: Vec<Arc<Evento>> = hist.into_iter().filter(|e| !mc_uf || amplo(e) || e.topico.as_ref().is_some_and(|t| subs.contains_key(t))).collect();
                            for e in hist { enviar!(Message::Text(e.msg.clone())); }
                            let _ = fa_rx.borrow_and_update();
                            // `faltam`: lista dos tópicos de que o cliente quer o `faltam_para_mat` (no máximo max_topicos);
                            // pedir de novo troca a lista e manda o valor atual dos tópicos que entraram nela
                            let antes = fa_quero.clone();
                            if let Some(a) = v["faltam"].as_array() {
                                fa_quero = Some(a.iter().filter_map(|x| x.as_str()).take(h.op.max_topicos).map(Arc::from).collect());
                            } else if !v.get("faltam").is_some_and(|x| x.is_null()) && v.get("faltam").is_some() {
                                h.med.erros_cliente.fetch_add(1, Relaxed);
                                enviar!(Message::Text(msg_erro("faltam_invalido", "faltam: lista de tópicos")));
                            }
                            let novos = |t: &Arc<str>| antes.as_ref().is_some_and(|a| !a.contains(t));
                            let fs: Vec<(u64, Utf8Bytes)> = h.faltam.read().unwrap().iter()
                                .filter(|(t, f)| fa_quero.as_ref().is_none_or(|q| q.contains(*t)) && (f.ver > fa_ver || novos(t)))
                                .map(|(_, f)| (f.ver, f.msg.clone())).collect();
                            for (v, m) in fs { fa_ver = fa_ver.max(v); enviar!(Message::Text(m)); }
                        }
                        Some("sonda") => {
                            // {"op":"sonda"} liga (manda a última, com `instantaneo: true`); {"op":"sonda","ligar":false} desliga
                            sd_ligada = v["ligar"].as_bool().unwrap_or(true);
                            sd_visto = *sd_rx.borrow_and_update();
                            if sd_ligada {
                                let ult = h.sondas.read().unwrap().back().cloned();
                                if let Some(x) = ult { enviar!(Message::Text(msg_sonda(&x, true))); }
                            }
                        }
                        Some("mapa") => {
                            // `desde`: quantas seções acesas o cliente já tem (do `GET /api/v1/mapa/estado` ou das mensagens);
                            // `epoca` de outro processo: a numeração mudou, o cliente busca a fotografia de novo
                            let Some(m) = h.mapa.get().cloned() else {
                                h.med.erros_cliente.fetch_add(1, Relaxed);
                                enviar!(Message::Text(msg_erro("mapa_indisponivel", "este servidor não tem o mapa das seções")));
                                continue;
                            };
                            let mut rx = m.n.subscribe();
                            let _ = rx.borrow_and_update();
                            mp_rx = Some(rx);
                            mp_pend = false;
                            if v["epoca"].as_u64().is_some_and(|e| e != m.epoca) {
                                mp_prox = usize::MAX;
                                enviar!(Message::Text(Utf8Bytes::from(json!({"tipo": "mapa", "epoca": m.epoca, "reiniciar": true}).to_string())));
                                continue;
                            }
                            let (n, msgs) = m.mensagens(v["desde"].as_u64().unwrap_or(0) as usize);
                            mp_prox = n;
                            for x in msgs { enviar!(Message::Text(Utf8Bytes::from(x))); }
                            mp_pode = tokio::time::Instant::now() + Duration::from_millis(crate::mapa::INTERVALO_MS);
                        }
                        _ => { h.med.erros_cliente.fetch_add(1, Relaxed); enviar!(Message::Text(msg_erro("op_desconhecida", "ops: inscrever, cancelar, eventos, marcos, mapa, sonda"))); }
                    }
                }
            },
            Some((t, q)) = subs.next(), if !subs.is_empty() => {
                let (v0, s0) = ult.get(&t).copied().unwrap_or((0, 0));
                // a primeira conexão a ver esta versão monta a mensagem (uma vez); as outras reaproveitam
                let Some(m) = q.m() else { continue };
                if q.ver <= v0 || m.seq < s0 { continue; }
                if q.ver > v0 + 1 {
                    let n = q.ver - v0 - 1;
                    h.med.juntadas.fetch_add(n, Relaxed);
                    let p = pulos.entry(t.clone()).or_insert((0, None));
                    p.0 += n;
                    if p.1.is_none_or(|x| x.elapsed() >= AVISO_PULO) {
                        h.med.avisos_pulo.fetch_add(1, Relaxed);
                        let a = json!({"tipo": "aviso", "motivo": "seq_pulado", "topico": &*t, "de": s0, "para": m.seq, "juntadas": p.0}).to_string();
                        *p = (0, Some(Instant::now()));
                        enviar!(Message::Text(Utf8Bytes::from(a)));
                    }
                }
                enviar!(Message::Text(m.msg.clone()));
                ult.insert(t, (q.ver, m.seq));
                if let Some(tb) = q.t_bu { h.med.lat_envio.lock().unwrap().por(agora_us().saturating_sub(tb) as f64 / 1e3); }
            }
            Some((t, q)) = tse_subs.next(), if !tse_subs.is_empty() => {
                let Some(q) = q else { continue };
                if tse_ver.get(&t).is_some_and(|&v| q.ver <= v) { continue; }
                tse_ver.insert(t, q.ver);
                enviar!(Message::Text(q.atualizacao.clone()));
            }
            Ok(()) = ev_rx.changed() => {
                let n = *ev_rx.borrow_and_update();
                let novos: Vec<Arc<Evento>> = h.eventos.read().unwrap()[ev_prox.min(n)..n].to_vec();
                ev_prox = n;
                for e in novos {
                    let quer = (ev_todos && (!ev_uf || amplo(&e))) || e.topico.as_ref().is_none_or(|t| subs.contains_key(t));
                    if quer && ev_env.insert(e.n) { enviar!(Message::Text(e.msg.clone())); }
                }
            }
            Ok(()) = mc_rx.changed(), if mc_todos => {
                let n = *mc_rx.borrow_and_update();
                let novos: Vec<Arc<Evento>> = h.marcos.read().unwrap()[mc_prox.min(n)..n].to_vec();
                mc_prox = mc_prox.max(n);
                for e in novos {
                    if mc_uf && !amplo(&e) && !e.topico.as_ref().is_some_and(|t| subs.contains_key(t)) { continue; }
                    enviar!(Message::Text(e.msg.clone()));
                }
            }
            Ok(()) = fa_rx.changed(), if mc_todos => {
                let ver_agora = *fa_rx.borrow_and_update();
                let mut fs: Vec<(u64, Utf8Bytes)> = h.faltam.read().unwrap().iter()
                    .filter(|(t, f)| f.ver > fa_ver && fa_quero.as_ref().is_none_or(|q| q.contains(*t))).map(|(_, f)| (f.ver, f.msg.clone())).collect();
                // a versão anda mesmo sem nada para este cliente (o que ficou de fora não é mandado depois)
                fa_ver = fa_ver.max(ver_agora);
                fs.sort_by_key(|x| x.0);
                for (v, m) in fs { fa_ver = fa_ver.max(v); enviar!(Message::Text(m)); }
            }
            Ok(()) = async { mp_rx.as_mut().expect("mapa").changed().await }, if mp_rx.is_some() && !mp_pend && mp_prox != usize::MAX => {
                if let Some(rx) = mp_rx.as_mut() { let _ = rx.borrow_and_update(); }
                mp_pend = true;
            }
            _ = tokio::time::sleep_until(mp_pode), if mp_pend => {
                mp_pend = false;
                let msgs = match h.mapa.get() { Some(m) => { let (n, v) = m.mensagens(mp_prox); mp_prox = n; v } None => vec![] };
                for x in msgs { enviar!(Message::Text(Utf8Bytes::from(x))); }
                mp_pode = tokio::time::Instant::now() + Duration::from_millis(crate::mapa::INTERVALO_MS);
            }
            Ok(()) = est_rx.changed() => {
                let e = est_rx.borrow_and_update().clone();
                if let Some(e) = e { enviar!(Message::Text(e.msg.clone())); }
            }
            Ok(()) = pv_rx.changed(), if !subs.is_empty() => {
                let _ = pv_rx.borrow_and_update();
                // só os tópicos inscritos com previsão mais nova que a mandada (≤ 200 por conexão)
                let novas: Vec<(Arc<str>, Arc<Prev>)> = {
                    let ps = h.previsoes.read().unwrap();
                    subs.keys().filter_map(|t| ps.get(t).filter(|p| pv_env.get(t).is_none_or(|&v| p.ver > v)).map(|p| (t.clone(), p.clone()))).collect()
                };
                for (t, p) in novas {
                    // primeira previsão do tópico nesta conexão: a de inscrição, com o histórico
                    let m = if pv_env.contains_key(&t) { p.msg.clone() } else { p.inst.clone() };
                    pv_env.insert(t, p.ver);
                    enviar!(Message::Text(m));
                }
            }
            Ok(()) = sd_rx.changed(), if sd_ligada => {
                let n = *sd_rx.borrow_and_update();
                // as novas desde a última vista (o contador do hub soma as sondas guardadas; no máximo as guardadas)
                let novas = n.saturating_sub(sd_visto) as usize;
                sd_visto = n;
                let lista: Vec<Arc<Value>> = { let g = h.sondas.read().unwrap(); let k = g.len().min(novas); g.iter().skip(g.len() - k).cloned().collect() };
                for x in lista {
                    enviar!(Message::Text(msg_sonda(&x, false)));
                    h.med.sondas_enviadas.fetch_add(1, Relaxed);
                    if x["local"] == true { if let Some(t) = x["t_injecao"].as_u64() { h.med.lat_sonda.lock().unwrap().por(agora_us().saturating_sub(t) as f64 / 1e3); } }
                }
            }
            _ = ping.tick() => {
                let desde = t0.elapsed().as_millis() as u64 - ultimo.load(Relaxed);
                if desde > h.op.sem_pong.as_millis() as u64 { let m = fechar(h, tx, 1001, "sem_pong").await; return fim(&subs, m); }
                enviar!(Message::Ping(Bytes::new()));
            }
        }
    }
}

#[cfg(test)]
mod testes;
