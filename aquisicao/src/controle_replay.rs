//! Controle do replay (`uv-servico --controle-replay`, só contra o `uv-simulador` local): velocidade (pausa e 1× a
//! [`REPLAY_MAX`]) e **posição na noite** — avançar e voltar no tempo, como a barra de um vídeo. Contrato:
//! `docs/PROTOCOLO_WS.md` §11.
//!
//! O relógio é o do simulador; este módulo só o comanda (`POST <base>/controle/...`) e o acompanha (`GET
//! <base>/controle` a cada 250 ms).
//!
//! **Ir para um instante já visto** (para trás, ou para a frente até onde a simulação já chegou) = **retotalização
//! local**: o motor só soma BUs, então o serviço para, guarda o diário da sessão no diário-mestre do replay
//! ([`DIARIO_MESTRE`], em `--dados`, junto dos pacotes com os BUs já baixados), apaga `--saida` e recomeça, reaplicando
//! do disco, em ordem cronológica de disponibilização e em blocos de [`BLOCO_S`] s da noite, todos os BUs disponíveis
//! até o instante — sem rede, em segundos. Eventos e marcos saem com o horário da disponibilização do bloco. Depois o
//! simulador é posto no instante e a apuração segue ao vivo dali. As páginas percebem a sessão nova (`sessao`) e se
//! recarregam.
//!
//! **Ir além do que já foi visto** = retotalização até o ponto mais distante já visto (se ele estiver à frente) e, dali,
//! busca: o simulador anda à velocidade de busca até o instante e lá volta sozinho à
//! velocidade escolhida; o serviço baixa e soma os BUs na ordem em que ficam disponíveis, só que mais rápido
//! ([`REPLAY_MAX`] com fila de coleta grande, até 3× isso com ela quase vazia: antes das 18 h e depois das 23 h quase
//! não chegam BUs). Os horários de eventos e marcos de dentro do trecho buscado ficam com a precisão dessa velocidade
//! (minutos).

use serde_json::{json, Value};
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering::Relaxed};
use std::sync::Mutex;
use std::time::Duration;

/// Maior velocidade em que o serviço acompanha o simulador no pico da noite (medido em 14/10/2026 no 1º turno de
/// 2026: a 200× a fila de coleta fica em até 4,5 mil seções; a 300×, até 68 mil — ~3,5 s de atraso, recuperado em
/// seguida; a 600×, 152 mil — ~8 s). Também é a velocidade de busca com a fila grande.
pub const REPLAY_MAX: f64 = 300.0;
/// Velocidade de busca quando a fila de coleta está quase vazia.
const BUSCA_RAPIDA: f64 = 3.0 * REPLAY_MAX;
/// Fila de coleta (seções) abaixo da qual a busca acelera.
const FILA_RAPIDA: i64 = 2000;

/// Diário-mestre do replay, em `--dados`: as linhas de BU de todos os diários das sessões (sem repetição), para a
/// retotalização local.
pub const DIARIO_MESTRE: &str = "replay_diario.log";
/// Blocos da retotalização (segundos da noite): eventos e marcos de cada bloco saem com o horário do fim dele.
pub const BLOCO_S: i64 = 30;

/// Reinício pedido pela página: (retotalizar até, velocidade escolhida, buscar depois até). O `uv-servico` lê depois que
/// [`crate::servico::executar`] termina.
static REINICIO: Mutex<Option<(i64, f64, Option<i64>)>> = Mutex::new(None);
/// Retotalização a fazer na partida da sessão nova (o motor reaplica até o alvo; o controle põe o simulador lá e, se
/// houver, busca dali até o segundo instante).
static RECONSTRUCAO: Mutex<Option<(i64, Option<i64>)>> = Mutex::new(None);
/// Até onde a simulação já chegou (maior relógio do simulador visto, em todas as sessões deste processo).
static ALCANCADO: AtomicI64 = AtomicI64::new(i64::MIN);

/// O reinício pedido (e o esquece): (retotalizar até, velocidade escolhida, buscar depois até).
pub fn reinicio_pedido() -> Option<(i64, f64, Option<i64>)> { REINICIO.lock().unwrap().take() }
/// A sessão nova, ao subir, retotaliza até `alvo` (e busca até `depois`, se houver).
pub fn definir_reconstrucao(alvo: i64, depois: Option<i64>) { *RECONSTRUCAO.lock().unwrap() = Some((alvo, depois)); }
/// O alvo da retotalização pendente (o motor lê; o controle esquece depois de pôr o simulador no alvo).
pub fn reconstrucao_pendente() -> Option<i64> { RECONSTRUCAO.lock().unwrap().map(|x| x.0) }

/// Junta o diário de uma sessão (`<saida>/secoes.log`) ao diário-mestre: só as linhas de BU, sem repetir (seção,
/// SHA-256, anulada). Devolve quantas linhas o mestre tem.
pub fn guardar_diario(sessao: &std::path::Path, mestre: &std::path::Path) -> anyhow::Result<usize> {
    let mut vistos = std::collections::HashSet::new();
    let mut saida = String::from(crate::servico::CAB_DIARIO);
    saida.push('\n');
    let mut n = 0;
    for p in [mestre, sessao] {
        let Ok(txt) = std::fs::read_to_string(p) else { continue };
        for l in txt.lines() {
            let f: Vec<&str> = l.split(',').collect();
            if f.len() != 16 || !matches!(f[1], "aplicado" | "substituido" | "anulado" | "reenvio_identico") { continue; }
            if !vistos.insert((f[2].to_string(), f[3].to_string(), f[4].to_string(), f[5].to_string(), f[7].to_string(), f[9].to_string())) { continue; }
            saida.push_str(l);
            saida.push('\n');
            n += 1;
        }
    }
    let tmp = mestre.with_extension("tmp");
    std::fs::write(&tmp, saida)?;
    std::fs::rename(&tmp, mestre)?;
    Ok(n)
}

/// Ligações com o serviço: ajustar as esperas à aceleração real, ler a fila de coleta e parar para reiniciar.
pub struct Ganchos {
    pub ao_mudar: Box<dyn Fn(f64) + Send + Sync>,
    pub fila: Box<dyn Fn() -> i64 + Send + Sync>,
    pub parar: Box<dyn Fn() + Send + Sync>,
}

pub struct Replay {
    base: String,
    /// velocidade escolhida (a de "play"; 0 = pausa)
    escolhida: AtomicU64,
    /// último `GET <base>/controle` (acelerar, t, inicio, fim, alvo)
    sim: Mutex<Value>,
    /// aceleração real do simulador, vista pela última vez (para ajustar as esperas do serviço)
    real: AtomicU64,
    /// busca em andamento: (de, alvo), em segundos
    busca: Mutex<Option<(i64, i64)>>,
    /// identifica esta sessão do serviço (muda a cada reinício; as páginas se recarregam)
    pub sessao: String,
    g: Ganchos,
}

#[derive(Debug)]
pub enum Pedido { Invalido(&'static str), Simulador(String) }

impl Replay {
    pub fn novo(base: &str, escolhida: f64, g: Ganchos) -> Replay {
        Replay {
            base: base.trim_end_matches('/').to_string(), escolhida: AtomicU64::new(escolhida.to_bits()), sim: Mutex::new(Value::Null),
            real: AtomicU64::new(f64::NAN.to_bits()), busca: Mutex::new(None), sessao: format!("{:x}", crate::difusao::agora_us()), g,
        }
    }

    pub fn escolhida(&self) -> f64 { f64::from_bits(self.escolhida.load(Relaxed)) }
    fn busca(&self) -> Option<(i64, i64)> { *self.busca.lock().unwrap() }

    /// Bloco `replay` da mensagem `estado` (só o que muda de vez em quando: o relógio vivo fica em `GET /api/v1/replay`).
    pub fn json_estado(&self) -> Value {
        let x = self.escolhida();
        json!({"acelerar": x, "pausado": x == 0.0, "min": 1, "max": REPLAY_MAX, "sessao": self.sessao,
            "busca": self.busca().map(|(de, alvo)| json!({"de": de, "alvo": alvo}))})
    }

    /// `GET /api/v1/replay`: o bloco do `estado` mais o relógio do simulador (`t`, `relogio`), `inicio` e `fim` da noite
    /// e a aceleração real (`real`, diferente de `acelerar` durante uma busca).
    pub fn json(&self) -> Value {
        let mut v = self.json_estado();
        let s = self.sim.lock().unwrap().clone();
        for k in ["t", "relogio", "inicio", "fim"] { v[k] = s.get(k).cloned().unwrap_or(Value::Null); }
        v["real"] = s.get("acelerar").cloned().unwrap_or(Value::Null);
        v
    }

    fn cliente() -> anyhow::Result<reqwest::Client> {
        Ok(reqwest::Client::builder().no_proxy().timeout(Duration::from_secs(3)).build()?)
    }
    async fn pedir(&self, caminho: &str, consulta: &str) -> Result<Value, Pedido> {
        let c = Self::cliente().map_err(|e| Pedido::Simulador(format!("{e:#}")))?;
        let url = format!("{}{caminho}{}{consulta}", self.base, if consulta.is_empty() { "" } else { "?" });
        let r = if caminho == "/controle" { c.get(url) } else { c.post(url) }.send().await.map_err(|e| Pedido::Simulador(format!("{e:#}")))?;
        if !r.status().is_success() { return Err(Pedido::Simulador(format!("HTTP {}", r.status()))); }
        let b = r.bytes().await.map_err(|e| Pedido::Simulador(format!("{e:#}")))?;
        let v: Value = serde_json::from_slice(&b).map_err(|e| Pedido::Simulador(format!("{e}")))?;
        self.observar(&v);
        Ok(v)
    }
    /// Guarda o estado do simulador e ajusta as esperas do serviço quando a aceleração real muda.
    fn observar(&self, v: &Value) {
        if let Some(x) = v.get("acelerar").and_then(Value::as_f64) {
            if f64::from_bits(self.real.swap(x.to_bits(), Relaxed)) != x { (self.g.ao_mudar)(x); }
        }
        *self.sim.lock().unwrap() = v.clone();
    }
    fn t_sim(&self) -> Option<i64> { self.sim.lock().unwrap().get("t").and_then(Value::as_i64) }

    /// Velocidade escolhida (0 = pausa; 1 a [`REPLAY_MAX`]). Durante uma busca só muda a velocidade de depois dela.
    pub async fn velocidade(&self, x: f64) -> Result<(), Pedido> {
        if !(x.is_finite() && (x == 0.0 || (1.0..=REPLAY_MAX).contains(&x))) { return Err(Pedido::Invalido("acelerar: 0 (pausa) ou de 1 a 300")); }
        match self.busca() {
            Some((_, alvo)) => {
                let v = f64::from_bits(self.real.load(Relaxed));
                let v = if v.is_finite() && v > 0.0 { v } else { REPLAY_MAX };
                self.pedir("/controle/velocidade", &format!("x={v}&ate={alvo}&depois={x}")).await?;
            }
            None => { self.pedir("/controle/velocidade", &format!("x={x}")).await?; }
        }
        self.escolhida.store(x.to_bits(), Relaxed);
        Ok(())
    }

    /// Ir para o instante `t` (segundos). Até onde a simulação já chegou: retotalização local (reinício; devolve
    /// `true`). Além disso: busca.
    pub async fn posicao(&self, t: i64) -> Result<bool, Pedido> {
        let s = self.pedir("/controle", "").await?;
        let (agora, inicio, fim) = (s["t"].as_i64().unwrap_or(0), s["inicio"].as_i64().unwrap_or(0), s["fim"].as_i64().unwrap_or(0));
        if fim <= inicio { return Err(Pedido::Simulador("simulador sem começo/fim da noite".into())); }
        let t = t.clamp(inicio, fim);
        let visto = ALCANCADO.load(Relaxed).max(agora);
        // além do visto, com o visto pouco à frente (ou atrás): busca daqui
        if t > visto && visto <= agora + 600 {
            self.buscar(agora, t).await?;
            return Ok(false);
        }
        // instante já visto (ou o visto bem à frente): o serviço recomeça retotalizando do disco até lá — e, além do
        // visto, busca dali. O simulador vai JÁ para o instante, parado: a sessão nova começa a ler os `cs.json` antes de
        // a camada HTTP subir e não pode ver seções de depois do instante
        let (ate, depois) = if t > visto { (visto, Some(t)) } else { (t, None) };
        self.pedir("/controle/posicao", &format!("t={ate}&x=0")).await?;
        *self.busca.lock().unwrap() = None;
        *REINICIO.lock().unwrap() = Some((ate, self.escolhida(), depois));
        (self.g.parar)();
        Ok(true)
    }

    async fn buscar(&self, de: i64, alvo: i64) -> Result<(), Pedido> {
        *self.busca.lock().unwrap() = Some((de, alvo));
        let r = self.pedir("/controle/velocidade", &format!("x={REPLAY_MAX}&ate={alvo}&depois={}", self.escolhida())).await;
        if r.is_err() { *self.busca.lock().unwrap() = None; }
        r.map(|_| ())
    }

    /// Tarefa da sessão: na partida, toca (ou faz a busca de um reinício); depois acompanha o simulador a cada 250 ms,
    /// termina a busca quando ele chega ao alvo e ajusta a velocidade de busca pela fila de coleta.
    pub async fn acompanhar(&self) {
        // partida: depois de uma retotalização, o simulador vai para o instante dela; senão, toca de onde está
        let pendente = RECONSTRUCAO.lock().unwrap().take();
        let partida = match pendente {
            // o simulador já está parado no instante (posto lá antes do reinício)
            Some((_, None)) => self.pedir("/controle/velocidade", &format!("x={}", self.escolhida())).await.map(|_| ()),
            Some((alvo, Some(depois))) => self.buscar(alvo, depois).await,
            None => self.pedir("/controle/velocidade", &format!("x={}", self.escolhida())).await.map(|_| ()),
        };
        match partida {
            Ok(()) => eprintln!("uv-servico: controle de replay ligado (sessão {}, {}×{})", self.sessao, self.escolhida(),
                pendente.map(|(a, d)| format!(", retotalizado até {}{}", crate::texto_hora(a), d.map(|d| format!(", buscando até {}", crate::texto_hora(d))).unwrap_or_default())).unwrap_or_default()),
            Err(e) => eprintln!("uv-servico: controle de replay sem resposta do simulador ({e:?})"),
        }
        loop {
            tokio::time::sleep(Duration::from_millis(250)).await;
            let Ok(s) = self.pedir("/controle", "").await else { continue };
            if let Some(t) = s["t"].as_i64() { ALCANCADO.fetch_max(t, Relaxed); }
            let Some((_, alvo)) = self.busca() else { continue };
            if s["alvo"].is_null() || self.t_sim().is_some_and(|t| t >= alvo) {
                *self.busca.lock().unwrap() = None;
                continue;
            }
            let quer = if (self.g.fila)() < FILA_RAPIDA { BUSCA_RAPIDA } else { REPLAY_MAX };
            if s["acelerar"].as_f64() != Some(quer) {
                let _ = self.pedir("/controle/velocidade", &format!("x={quer}&ate={alvo}&depois={}", self.escolhida())).await;
            }
        }
    }
}
