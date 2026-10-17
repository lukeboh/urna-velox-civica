//! O nó dentro do `uv-servico` (modo `--federacao`; ADR-028, etapa A da 2.0).
//!
//! * **Quem coleta o quê.** O trabalhador de coleta só pede ao TSE as seções que a partição dá a este nó
//!   ([`Federado::minha`]); as outras ficam **delegadas**: esperam o resultado do dono e são reavaliadas a
//!   cada *heartbeat* — se a partição mudar (dono morto, em pausa por 429, IP divergente) e a seção passar a ser
//!   deste nó, ela entra na fila de coleta.
//! * **Troca entre os nós, sem storage compartilhado.** Tudo o que este nó resolve com o TSE (BU, não instalada,
//!   agregada, confirmada) vai para o diretório local dele, em `bus/<nó>/<seq>.uvtr` (lotes só de acréscimo) e
//!   `bus/<nó>/ultimo`; a [`super::sincronia`] copia os lotes dos pares (direto ou por repasse) para o diretório local
//!   de cada um, e o consumidor deste módulo entrega ao motor as mesmas mensagens da coleta local
//!   ([`crate::servico::Msg`]), com o mesmo diário e os mesmos pacotes. Cada nó termina com todos os BUs.
//! * **Heartbeat** (`nos/<nó>.json`): IP de saída medido pelo eco (`/eco-ip` no simulador), pausa por 429 em curso,
//!   versão. A cada um, o estado da federação é recalculado e a cota de req/s do limitador vira a fatia do IP.
//! * **Queda e retomada.** O que estava em memória e não chegou a um lote antes de um `kill -9` não se perde: na
//!   partida, o nó republica as linhas do diário (`secoes.log`) posteriores ao seu último lote gravado.
//!
//! Formato do lote (`.uvtr`, provisório até a etapa C): `UVTR1\n`, depois, para cada registro, `u32 LE` + cabeçalho
//! JSON ([`Cabecalho`]) + `u32 LE` + bytes do BU (0 sem BU).

use super::armazem::{Armazem, ArmazemLocal, Lido};
use super::config::Federacao;
use super::estado::{self, Batimento, EstadoFederacao, Motivo};
use crate::servico::{self, BuRecebido, ChaveUf, Comum, Msg, Tarefa};
use crate::tse::{self, Secao};
use anyhow::{bail, Context, Result};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant, SystemTime};

/// Opções do modo federado (linha de comando do `uv-servico`).
#[derive(Clone, Debug)]
pub struct Opcoes {
    pub federacao: PathBuf,
    pub no: String,
    /// diretório local do nó: os próprios *heartbeats* e lotes e as cópias dos pares (não é compartilhado)
    pub armazem: PathBuf,
    /// URL do eco de IP; padrão contra o simulador: `<base>/eco-ip`
    pub eco_ip: Option<String>,
    /// onde servir a sincronização aos pares; padrão: o `endereco` do nó no `federacao.json` (sem os dois, o nó só
    /// busca dos outros)
    pub escuta: Option<std::net::SocketAddr>,
}

const MAGICO: &[u8] = b"UVTR1\n";

/// Cabeçalho de um registro trocado entre nós.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Cabecalho {
    /// "bu" | "nao_instalada" | "agregada" | "confirmada"
    pub tipo: String,
    pub uf: String,
    pub m: u32,
    pub z: u16,
    pub s: u16,
    #[serde(default)]
    pub hash: String,
    /// SHA-256 dos bytes do BU (hex); o receptor confere
    #[serde(default)]
    pub sha: String,
    #[serde(default)]
    pub situacao: String,
    #[serde(default)]
    pub anulada: bool,
    #[serde(default)]
    pub disp: String,
    #[serde(default)]
    pub recebido: String,
    #[serde(default)]
    pub hora_site: i64,
}

pub fn codificar(regs: &[(Cabecalho, Bytes)]) -> Vec<u8> {
    let mut v = Vec::with_capacity(MAGICO.len() + regs.iter().map(|r| r.1.len() + 300).sum::<usize>());
    v.extend_from_slice(MAGICO);
    for (c, b) in regs {
        let j = serde_json::to_vec(c).expect("cabeçalho serializável");
        v.extend_from_slice(&(j.len() as u32).to_le_bytes());
        v.extend_from_slice(&j);
        v.extend_from_slice(&(b.len() as u32).to_le_bytes());
        v.extend_from_slice(b);
    }
    v
}

pub fn decodificar(b: &[u8]) -> Result<Vec<(Cabecalho, Bytes)>> {
    if !b.starts_with(MAGICO) { bail!("lote sem o cabeçalho UVTR1"); }
    let mut i = MAGICO.len();
    let mut out = Vec::new();
    let ler_u32 = |i: &mut usize| -> Result<usize> {
        let x = b.get(*i..*i + 4).context("lote truncado")?;
        *i += 4;
        Ok(u32::from_le_bytes(x.try_into().unwrap()) as usize)
    };
    while i < b.len() {
        let n = ler_u32(&mut i)?;
        let c: Cabecalho = serde_json::from_slice(b.get(i..i + n).context("lote truncado")?)?;
        i += n;
        let n = ler_u32(&mut i)?;
        let bu = Bytes::copy_from_slice(b.get(i..i + n).context("lote truncado")?);
        i += n;
        out.push((c, bu));
    }
    Ok(out)
}

fn chave_lote(no: &str, seq: u64) -> String { format!("bus/{no}/{seq:010}.uvtr") }
fn chave_ultimo(no: &str) -> String { format!("bus/{no}/ultimo") }

fn k_de(c: &Cabecalho) -> Option<ChaveUf> {
    let uf: [u8; 2] = c.uf.as_bytes().try_into().ok()?;
    Some((uf, tse::chave(c.m, c.z, c.s)))
}

fn hex(b: &[u8]) -> String { b.iter().map(|x| format!("{x:02x}")).collect() }

#[derive(Default)]
pub struct ContFed {
    pub publicados: AtomicU64,
    pub lotes_gravados: AtomicU64,
    pub recebidos: AtomicU64,
    pub ja_aplicados: AtomicU64,
    pub lotes_lidos: AtomicU64,
    pub rejeitados: AtomicU64,
    pub delegadas_total: AtomicU64,
    pub assumidas: AtomicU64,
    pub republicados: AtomicU64,
    pub mudancas_particao: AtomicU64,
    /// seções deste nó que não foram ao TSE porque o resultado já tinha vindo de um par
    pub puladas: AtomicU64,
}

pub struct Federado {
    pub cfg: Federacao,
    pub id: String,
    armazem: Arc<dyn Armazem>,
    eco: String,
    estado: RwLock<EstadoFederacao>,
    ip_medido: Mutex<String>,
    saida: Mutex<Vec<(Cabecalho, Bytes)>>,
    delegadas: Mutex<HashMap<ChaveUf, Secao>>,
    /// disponibilização do último resultado recebido de outro nó, por seção: o resultado pode chegar pelo storage antes
    /// de o `cs.json` deste nó mostrar a seção, e então ela não deve ficar delegada
    resolvidas: Mutex<HashMap<ChaveUf, i64>>,
    seq: AtomicU64,
    lidos: Mutex<HashMap<String, (u64, Option<String>)>>,
    pub cont: ContFed,
    pub sinc: super::sincronia::ContSinc,
    batidas: AtomicU64,
    /// o servidor da sincronização para quando isto vira `true` (depois de uma carência no fim do serviço, para os
    /// pares levarem os últimos lotes)
    pub fim: std::sync::atomic::AtomicBool,
    /// intervalos reais (já com a aceleração do simulador)
    pub batimento: Duration,
    pub troca: Duration,
    morte_s: u64,
    /// Na partida o nó fica fora da partição ("sincronizando") até ter consumido, de cada par vivo, os lotes que o
    /// par já tinha publicado quando foi visto pela primeira vez (`alvos`): assim um nó que volta depois de uma queda
    /// não pede ao TSE o que os outros coletaram enquanto ele estava fora. No começo da noite os alvos são 0 e a
    /// espera é de um *heartbeat*.
    sincronizando: std::sync::atomic::AtomicBool,
    alvos: Mutex<HashMap<String, u64>>,
    inicio: Instant,
    /// por par: (último `contador` visto no heartbeat dele, batida deste nó em que ele mudou). Um par só pode ser dado
    /// como morto depois de este nó ter batido normalmente uma janela inteira de morte sem ver o contador dele mudar:
    /// numa parada da máquina inteira (E/S, agendador), quem parou também não condena os outros.
    vistos: Mutex<HashMap<String, (u64, u64)>>,
}

impl Federado {
    /// Lê o `federacao.json`, confere o nó e abre o storage. `escala` = 1/aceleração (simulador) ou 1.
    pub fn novo(op: &Opcoes, base: &str, escala: f64) -> Result<Federado> {
        let cfg = Federacao::ler(&op.federacao)?;
        let no = cfg.no(&op.no).with_context(|| format!("nó '{}' não está em {}", op.no, op.federacao.display()))?;
        let eco = match &op.eco_ip {
            Some(e) => e.clone(),
            None if crate::coletor::base_local(base) => format!("{}/eco-ip", base.trim_end_matches('/')),
            None => bail!("--eco-ip é obrigatório contra o site real: o nó precisa medir o IP de saída que a CDN vê (ADR-028)"),
        };
        // relógio da federação no simulador: heartbeat e morte encolhem com a aceleração, com pisos para não confundir
        // uma pausa do sistema com a morte de um nó
        let morte_s = ((cfg.morte_s as f64 * escala).max(3.0)).ceil() as u64;
        let batimento = Duration::from_secs_f64((cfg.batimento_s as f64 * escala).max(morte_s as f64 / 4.0).min(cfg.batimento_s as f64));
        let troca = Duration::from_secs_f64((2.0 * escala).clamp(0.1, 2.0));
        let armazem: Arc<dyn Armazem> = Arc::new(ArmazemLocal::novo(&op.armazem)?);
        let seq = match armazem.ler(&chave_ultimo(&no.id), None)? {
            Lido::Novo(o) => String::from_utf8_lossy(&o.bytes).trim().parse().unwrap_or(0),
            _ => 0,
        };
        Ok(Federado {
            id: no.id.clone(), estado: RwLock::new(estado::calcular(&cfg, &[], SystemTime::now())), cfg, armazem, eco,
            ip_medido: Mutex::default(), saida: Mutex::default(), delegadas: Mutex::default(), resolvidas: Mutex::default(), seq: AtomicU64::new(seq),
            lidos: Mutex::default(), cont: ContFed::default(), sinc: Default::default(), batidas: AtomicU64::new(0), fim: Default::default(), batimento, troca, morte_s,
            sincronizando: std::sync::atomic::AtomicBool::new(true), alvos: Mutex::default(), inicio: Instant::now(), vistos: Mutex::default(),
        })
    }

    /// Ainda alcançando os pares depois da partida?
    pub fn sincronizando(&self) -> bool { self.sincronizando.load(Relaxed) }

    /// Decide se o nó já alcançou os pares (chamado a cada *heartbeat*, com os *heartbeats* lidos do diretório local).
    /// Pronto quando passaram ao menos dois *heartbeats* desde a partida (a sincronização já buscou os dos pares) e, para cada
    /// par vivo, o consumidor já entregou ao motor os lotes até o `seq_bus` que o par anunciava na primeira vez em que
    /// foi visto. Limite: 20 × o tempo de morte; passou disso, entra assim mesmo e avisa.
    /// Devolve `Some((segundos, alvos))` na transição para "pronto".
    fn avaliar_sincronia(&self, lidos: &[estado::BatimentoLido], agora: SystemTime) -> Option<(f64, Value, bool)> {
        if !self.sincronizando() { return None; }
        let passou = self.inicio.elapsed();
        // dois heartbeats: as cópias locais dos heartbeats dos pares (de antes de uma queda) já foram renovadas pela
        // sincronização, e um par vivo não é confundido com morto por causa de uma cópia velha
        if passou < self.batimento * 2 { return None; }
        let morte = Duration::from_secs(self.morte_s);
        let mut alvos = self.alvos.lock().unwrap();
        let consumidos = self.lidos.lock().unwrap();
        let mut pronto = true;
        for b in lidos {
            let no = b.batimento.no.as_str();
            if no == self.id || self.cfg.no(no).is_none() { continue; }
            if agora.duration_since(b.recebido).unwrap_or(Duration::ZERO) > morte { continue; }
            let alvo = *alvos.entry(no.to_string()).or_insert(b.batimento.seq_bus);
            if consumidos.get(no).map_or(0, |x| x.0) < alvo { pronto = false; }
        }
        let estourou = passou > morte * 20;
        if !pronto && !estourou { return None; }
        self.sincronizando.store(false, Relaxed);
        let a = json!(alvos.iter().map(|(n, s)| (n.clone(), json!({"alvo": s, "consumido": consumidos.get(n).map_or(0, |x| x.0)}))).collect::<serde_json::Map<_, _>>());
        Some((passou.as_secs_f64(), a, !pronto))
    }

    /// O diretório local do nó (os próprios dados e as cópias dos pares).
    pub fn armazem(&self) -> &dyn Armazem { self.armazem.as_ref() }
    /// Endereço de sincronização de um nó (`federacao.json`).
    pub fn endereco_de(&self, id: &str) -> Option<String> { self.cfg.no(id).and_then(|n| n.endereco.clone()) }

    /// A seção cabe a este nó na partição atual?
    pub fn minha(&self, s: &Secao) -> bool {
        self.estado.read().unwrap().particao.dono(s.municipio, s.zona, s.secao) == Some(self.id.as_str())
    }

    /// A seção é de outro nó: espera o resultado pelo storage.
    pub fn delegar(&self, s: Secao) {
        let k = (s.uf, s.chave());
        // o resultado desta disponibilização já veio pelo storage
        if self.resolvidas.lock().unwrap().get(&k).is_some_and(|&d| d >= servico::t_disp(&s.disponivel)) { return; }
        if self.delegadas.lock().unwrap().insert(k, s).is_none() { self.cont.delegadas_total.fetch_add(1, Relaxed); }
    }
    pub fn resolver(&self, k: &ChaveUf) { self.delegadas.lock().unwrap().remove(k); }
    /// O resultado desta disponibilização da seção já chegou de outro nó? (conta as puladas)
    pub fn ja_resolvida(&self, s: &Secao) -> bool {
        let r = self.resolvidas.lock().unwrap().get(&(s.uf, s.chave())).is_some_and(|&d| d >= servico::t_disp(&s.disponivel));
        if r { self.cont.puladas.fetch_add(1, Relaxed); }
        r
    }
    /// Resultado recebido de outro nó para a disponibilização `disp` (texto; vazio = qualquer uma, ex.: agregada).
    fn resolvida(&self, k: ChaveUf, disp: &str) {
        let d = if disp.trim().is_empty() { i64::MAX } else { servico::t_disp(disp) };
        let mut r = self.resolvidas.lock().unwrap();
        let e = r.entry(k).or_insert(d);
        if d > *e { *e = d; }
        drop(r);
        let mut dl = self.delegadas.lock().unwrap();
        if dl.get(&k).is_some_and(|s| servico::t_disp(&s.disponivel) <= d) { dl.remove(&k); }
    }

    fn guardar(&self, c: Cabecalho, b: Bytes) {
        self.cont.publicados.fetch_add(1, Relaxed);
        self.saida.lock().unwrap().push((c, b));
    }
    fn cab(tipo: &str, k: ChaveUf) -> Cabecalho {
        Cabecalho { tipo: tipo.into(), uf: String::from_utf8_lossy(&k.0).into(), m: (k.1 >> 32) as u32, z: (k.1 >> 16) as u16, s: k.1 as u16,
            hash: String::new(), sha: String::new(), situacao: String::new(), anulada: false, disp: String::new(), recebido: String::new(), hora_site: 0 }
    }
    /// BU coletado do TSE por este nó.
    pub fn publicar_bu(&self, b: &BuRecebido) {
        let mut c = Self::cab("bu", b.k);
        c.hash = b.hash.clone(); c.sha = hex(&b.sha); c.situacao = b.situacao.clone(); c.anulada = b.anulada;
        c.disp = b.disp.clone(); c.recebido = b.recebido.clone(); c.hora_site = b.hora_site;
        self.guardar(c, b.bytes.clone());
    }
    /// Resultado sem BU: "nao_instalada", "agregada" ou "confirmada".
    pub fn publicar_sem_bu(&self, tipo: &str, k: ChaveUf, disp: &str) {
        let mut c = Self::cab(tipo, k);
        c.disp = disp.to_string();
        self.guardar(c, Bytes::new());
    }

    pub fn saude_json(&self) -> Value {
        let e = self.estado.read().unwrap();
        let me = e.ativo(&self.id);
        let c = &self.cont;
        json!({
            "no": self.id, "ip_saida_medido": *self.ip_medido.lock().unwrap(), "participa": me.is_some(), "sincronizando": self.sincronizando(),
            "rps_cota": me.map(|n| n.rps), "peso": me.map(|n| n.peso), "nos_no_ip": me.map(|n| n.nos_no_ip),
            "ativos": e.ativos.iter().map(|n| json!({"no": n.id, "ip": n.ip_saida, "rps": n.rps, "peso": n.peso})).collect::<Vec<_>>(),
            "fora": e.fora.iter().map(|(n, m)| json!({"no": n, "motivo": motivo(m)})).collect::<Vec<_>>(),
            "rps_federacao": e.rps_total(),
            "delegadas_agora": self.delegadas.lock().unwrap().len(),
            "delegadas_total": c.delegadas_total.load(Relaxed), "assumidas": c.assumidas.load(Relaxed),
            "publicados": c.publicados.load(Relaxed), "lotes_gravados": c.lotes_gravados.load(Relaxed), "republicados": c.republicados.load(Relaxed),
            "recebidos": c.recebidos.load(Relaxed), "ja_aplicados": c.ja_aplicados.load(Relaxed), "rejeitados": c.rejeitados.load(Relaxed),
            "lotes_lidos": c.lotes_lidos.load(Relaxed), "mudancas_particao": c.mudancas_particao.load(Relaxed), "puladas": c.puladas.load(Relaxed),
            "sincronia": {"servidas": self.sinc.servidas.load(Relaxed), "servidas_304": self.sinc.servidas_304.load(Relaxed),
                "buscas": self.sinc.buscas.load(Relaxed), "buscas_erro": self.sinc.buscas_erro.load(Relaxed),
                "lotes_copiados": self.sinc.lotes_copiados.load(Relaxed), "por_repasse": self.sinc.por_repasse.load(Relaxed)},
        })
    }
}

fn motivo(m: &Motivo) -> String {
    match m {
        Motivo::Desconhecido => "desconhecido".into(),
        Motivo::Morto => "morto".into(),
        Motivo::EmPausa => "em_pausa".into(),
        Motivo::IpDivergente { declarado, medido } => format!("ip_divergente (declarado {declarado}, medido {medido})"),
        Motivo::SoRecebe => "so_recebe".into(),
        Motivo::Sincronizando => "sincronizando".into(),
    }
}

// ------------------------------------------------------------------ tarefas (tokio)

/// Mede o IP de saída pelo eco (a requisição passa pelo limitador, como as outras, e conta no teto do IP).
async fn medir_ip(f: &Federado, com: &Arc<Comum>) {
    match com.rede.get(&f.eco, None, None).await {
        Ok(r) if r.status == 200 => {
            let ip = String::from_utf8_lossy(&r.corpo).trim().to_string();
            let antes = std::mem::replace(&mut *f.ip_medido.lock().unwrap(), ip.clone());
            if antes != ip { com.registrar("info", "ip_saida", json!({"no": f.id, "medido": ip, "declarado": f.cfg.no(&f.id).map(|n| n.ip_saida.clone())})); }
        }
        Ok(r) => com.registrar("aviso", "eco_ip", json!({"status": r.status})),
        Err(e) => com.registrar("aviso", "eco_ip", json!({"erro": e.to_string()})),
    }
}

/// Os pares cujo heartbeat mudou há menos de uma janela de morte **em batidas deste nó** contam como vivos (a idade em
/// tempo só vale depois disso), e este nó está sempre vivo para si mesmo. Ver [`Federado::vistos`].
fn ajustar_vida(f: &Federado, lidos: &mut [estado::BatimentoLido], agora: SystemTime) {
    let minha = f.batidas.load(Relaxed);
    let janela = (f.morte_s as f64 / f.batimento.as_secs_f64().max(1e-3)).ceil() as u64;
    let mut v = f.vistos.lock().unwrap();
    for b in lidos.iter_mut() {
        if b.batimento.no == f.id { b.recebido = agora; continue; }
        let e = v.entry(b.batimento.no.clone()).or_insert((b.batimento.contador, minha));
        if e.0 != b.batimento.contador { *e = (b.batimento.contador, minha); }
        if minha.saturating_sub(e.1) <= janela && b.recebido < agora { b.recebido = agora; }
    }
}

/// Publica o heartbeat deste nó (uma batida).
fn publicar(f: &Federado, pausa_ate: u64) -> Result<()> {
    let b = Batimento { no: f.id.clone(), ip_saida_medido: f.ip_medido.lock().unwrap().clone(), pausa_ate, seq_bus: f.seq.load(Relaxed),
        versao: urna_velox::VERSAO.to_string(), contador: f.batidas.fetch_add(1, Relaxed) + 1, sincronizando: f.sincronizando() };
    super::publicar_batimento(f.armazem.as_ref(), &b)
}

/// Um *heartbeat*: publica, recalcula o estado, ajusta a cota e reavalia as delegadas. (O IP é medido em outra tarefa:
/// a medida passa pela fila do limitador e, no pico, seguraria o heartbeat por segundos.)
async fn bater(f: &Federado, com: &Arc<Comum>, tx: &tokio::sync::mpsc::UnboundedSender<Tarefa>, acel: f64) {
    let agora = f.armazem.agora().unwrap_or_else(|_| SystemTime::now());
    let pausa = com.rede.lim.pausa_restante();
    let pausa_ate = if pausa.is_zero() { 0 } else { (agora + pausa).duration_since(SystemTime::UNIX_EPOCH).map(|d| d.as_secs() + 1).unwrap_or(0) };
    // ainda alcançando os pares? (decide com os heartbeats que a sincronização já trouxe)
    if f.sincronizando() {
        if let Ok(antes) = super::ler_batimentos(f.armazem.as_ref()) {
            if let Some((s, alvos, estourou)) = f.avaliar_sincronia(&antes, agora) {
                if estourou { com.alerta("federacao_sincronia_demorada", &format!("{} entrou na partição sem alcançar os pares em {s:.1} s: {alvos}", f.id)); }
                com.registrar("info", "sincronizado", json!({"no": f.id, "segundos": s, "pares": alvos, "estourou": estourou}));
            }
        }
    }
    if let Err(e) = publicar(f, pausa_ate) { com.registrar("aviso", "heartbeat", json!({"erro": format!("{e:#}")})); return; }
    let mut lidos = match super::ler_batimentos(f.armazem.as_ref()) { Ok(x) => x, Err(e) => { com.registrar("aviso", "heartbeats", json!({"erro": format!("{e:#}")})); return; } };
    let mut cfg = f.cfg.clone();
    cfg.morte_s = f.morte_s;
    let agora = f.armazem.agora().unwrap_or(agora);
    ajustar_vida(f, &mut lidos, agora);
    let novo = estado::calcular(&cfg, &lidos, agora);
    let mudou = {
        let e = f.estado.read().unwrap();
        e.ativos != novo.ativos || e.fora.iter().map(|x| &x.0).ne(novo.fora.iter().map(|x| &x.0))
    };
    if let Some(me) = novo.ativo(&f.id) { com.rede.lim.definir_cota(me.rps * acel); }
    if mudou {
        f.cont.mudancas_particao.fetch_add(1, Relaxed);
        com.registrar("info", "particao", json!({"ativos": novo.ativos.iter().map(|n| json!({"no": n.id, "ip": n.ip_saida, "rps": n.rps, "peso": n.peso})).collect::<Vec<_>>(),
            "fora": novo.fora.iter().map(|(n, m)| json!({"no": n, "motivo": motivo(m)})).collect::<Vec<_>>()}));
    }
    *f.estado.write().unwrap() = novo;
    // delegadas que passaram a ser deste nó entram na fila
    let assumir: Vec<Secao> = {
        let mut d = f.delegadas.lock().unwrap();
        let ks: Vec<ChaveUf> = d.iter().filter(|(_, s)| f.minha(s)).map(|(k, _)| *k).collect();
        ks.iter().filter_map(|k| d.remove(k)).collect()
    };
    if !assumir.is_empty() {
        f.cont.assumidas.fetch_add(assumir.len() as u64, Relaxed);
        com.registrar("info", "assumidas", json!({"secoes": assumir.len()}));
        for s in assumir { servico::enfileirar(com, tx, Tarefa { secao: s, tentativa: 0, sondagem: false }); }
    }
}

pub async fn batimentos(f: Arc<Federado>, com: Arc<Comum>, tx: tokio::sync::mpsc::UnboundedSender<Tarefa>, acel: f64) {
    // o IP é medido na partida (antes do primeiro heartbeat) e depois a cada ~30 heartbeats, numa tarefa à parte
    medir_ip(&f, &com).await;
    let (fm, cm) = (f.clone(), com.clone());
    tokio::spawn(async move {
        while !cm.parar.load(Relaxed) {
            tokio::time::sleep(fm.batimento * 30).await;
            medir_ip(&fm, &cm).await;
        }
    });
    while !com.parar.load(Relaxed) {
        bater(&f, &com, &tx, acel).await;
        tokio::time::sleep(f.batimento).await;
    }
    // na carência do fim o nó ainda serve os últimos lotes aos pares: continua batendo (só publica, sem recalcular a
    // partição nem assumir seções), para os pares não o darem como morto antes de ele sair
    while !f.fim.load(Relaxed) {
        let _ = publicar(&f, 0);
        tokio::time::sleep(f.batimento).await;
    }
}

/// Grava no storage o que este nó resolveu desde o último lote.
pub fn gravar_lote(f: &Federado) -> Result<()> {
    let regs = std::mem::take(&mut *f.saida.lock().unwrap());
    if regs.is_empty() { return Ok(()); }
    let seq = f.seq.load(Relaxed) + 1;
    if let Err(e) = f.armazem.gravar(&chave_lote(&f.id, seq), &codificar(&regs)) {
        // devolve para a próxima tentativa, na ordem
        let mut s = f.saida.lock().unwrap();
        let resto = std::mem::take(&mut *s);
        *s = regs; s.extend(resto);
        return Err(e);
    }
    f.armazem.gravar(&chave_ultimo(&f.id), seq.to_string().as_bytes())?;
    f.seq.store(seq, Relaxed);
    f.cont.lotes_gravados.fetch_add(1, Relaxed);
    Ok(())
}

pub async fn publicador(f: Arc<Federado>, com: Arc<Comum>) {
    while !com.parar.load(Relaxed) {
        tokio::time::sleep(f.troca).await;
        if let Err(e) = gravar_lote(&f) { com.registrar("aviso", "lote", json!({"erro": format!("{e:#}")})); }
    }
    let _ = gravar_lote(&f);
}

/// Lê os lotes novos dos outros nós e os entrega ao motor.
pub async fn consumidor(f: Arc<Federado>, com: Arc<Comum>) {
    let outros: Vec<String> = f.cfg.nos.iter().filter(|n| n.id != f.id).map(|n| n.id.clone()).collect();
    while !com.parar.load(Relaxed) {
        for no in &outros {
            if let Err(e) = consumir(&f, &com, no) { com.registrar("aviso", "consumo", json!({"no": no, "erro": format!("{e:#}")})); }
        }
        tokio::time::sleep(f.troca).await;
    }
}

fn consumir(f: &Federado, com: &Arc<Comum>, no: &str) -> Result<()> {
    let (lido, etag) = f.lidos.lock().unwrap().get(no).cloned().unwrap_or((0, None));
    let ultimo = match f.armazem.ler(&chave_ultimo(no), etag.as_deref())? {
        Lido::Novo(o) => {
            let n: u64 = String::from_utf8_lossy(&o.bytes).trim().parse().context("bus/<nó>/ultimo ilegível")?;
            f.lidos.lock().unwrap().insert(no.to_string(), (lido, Some(o.versao.etag)));
            n
        }
        _ => return Ok(()),
    };
    for seq in lido + 1..=ultimo {
        let o = match f.armazem.ler(&chave_lote(no, seq), None)? { Lido::Novo(o) => o, _ => bail!("lote {seq} de {no} ausente") };
        f.cont.lotes_lidos.fetch_add(1, Relaxed);
        for (c, bu) in decodificar(&o.bytes)? {
            entregar(f, com, c, bu);
        }
        if let Some(x) = f.lidos.lock().unwrap().get_mut(no) { x.0 = seq; }
    }
    Ok(())
}

fn entregar(f: &Federado, com: &Arc<Comum>, c: Cabecalho, bu: Bytes) {
    let Some(k) = k_de(&c) else { f.cont.rejeitados.fetch_add(1, Relaxed); return };
    f.cont.recebidos.fetch_add(1, Relaxed);
    f.resolvida(k, &c.disp);
    let msg = match c.tipo.as_str() {
        "bu" => {
            let sha: [u8; 32] = Sha::de(&bu);
            if hex(&sha) != c.sha {
                // até a etapa C (assinatura da urna) só isto: o BU tem de ser o que o dono declarou
                f.cont.rejeitados.fetch_add(1, Relaxed);
                com.alerta("federacao_bu_rejeitado", &format!("{}:{}/{}/{} sha256 não confere", c.uf, c.m, c.z, c.s));
                return;
            }
            if com.aplicados.lock().unwrap().get(&k).is_some_and(|&(h, a)| h == servico::impressao(&c.hash) && a == c.anulada) {
                f.cont.ja_aplicados.fetch_add(1, Relaxed);
                return;
            }
            Msg::Bu(Box::new(BuRecebido {
                k, vsc: tse::sha256_do_hash_dir(&c.hash), hash: c.hash, sha, situacao: c.situacao, anulada: c.anulada, disp: c.disp,
                recebido: c.recebido, hora_site: c.hora_site, bytes: bu, pos: None, chegada: Instant::now(),
            }))
        }
        "nao_instalada" => Msg::NaoInstalada { k, disp: c.disp },
        "agregada" => Msg::Agregada { k },
        "confirmada" => Msg::Confirmada { k, disp: c.disp },
        _ => { f.cont.rejeitados.fetch_add(1, Relaxed); return; }
    };
    let _ = com.tx_motor.send(msg);
}

struct Sha;
impl Sha {
    fn de(b: &[u8]) -> [u8; 32] { use sha2::Digest; sha2::Sha256::digest(b).into() }
}

/// Na partida: republica o que o diário registrou depois do último lote deste nó no storage (o que estava em memória
/// num `kill -9`). Republicar a mais é inofensivo: quem já tem ignora (`ja_aplicados`).
pub fn republicar(f: &Federado, saida: &std::path::Path, dados: &std::path::Path) -> Result<usize> {
    let ultimo = f.seq.load(Relaxed);
    if ultimo == 0 { return Ok(0); }
    let corte_ms = match f.armazem.ler(&chave_lote(&f.id, ultimo), None)? {
        Lido::Novo(o) => o.versao.modificado.duration_since(SystemTime::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0).saturating_sub(30_000),
        _ => 0,
    };
    let Ok(txt) = std::fs::read_to_string(saida.join("secoes.log")) else { return Ok(0) };
    let mut n = 0;
    for l in txt.lines() {
        let c: Vec<&str> = l.split(',').collect();
        if c.len() != 16 || c[0] == "t_ms" { continue; }
        if c[0].parse::<u64>().unwrap_or(0) < corte_ms { continue; }
        let (Ok(uf), Ok(m), Ok(z), Ok(s)) = (<[u8; 2]>::try_from(c[2].as_bytes()), c[3].parse::<u32>(), c[4].parse::<u16>(), c[5].parse::<u16>()) else { continue };
        let k = (uf, tse::chave(m, z, s));
        match c[1] {
            "aplicado" | "substituido" | "anulado" => {
                let (Ok(pos), Ok(tam)) = (c[13].parse::<u64>(), c[14].parse::<u32>()) else { continue };
                let Ok(bytes) = urna_velox::pacote::ler_bu_em(&dados.join(c[12]), pos, tam) else { continue };
                let mut cab = Federado::cab("bu", k);
                cab.hash = c[6].into(); cab.sha = c[7].into(); cab.situacao = c[8].into(); cab.anulada = c[9] == "1";
                cab.disp = c[10].into(); cab.recebido = c[11].into();
                f.guardar(cab, Bytes::from(bytes));
            }
            "nao_instalada" => f.publicar_sem_bu("nao_instalada", k, c[10]),
            "agregada" => f.publicar_sem_bu("agregada", k, ""),
            _ => continue,
        }
        n += 1;
    }
    f.cont.republicados.fetch_add(n as u64, Relaxed);
    Ok(n)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn lote_ida_e_volta() {
        let c1 = Cabecalho { tipo: "bu".into(), uf: "am".into(), m: 2550, z: 1, s: 550, hash: "abc".into(), sha: "00".into(), situacao: "Totalizado".into(),
            anulada: false, disp: "27/10/2024 17:01:02".into(), recebido: "x".into(), hora_site: 9 };
        let mut c2 = c1.clone();
        c2.tipo = "nao_instalada".into();
        let regs = vec![(c1, Bytes::from_static(b"\x30\x80bu")), (c2, Bytes::new())];
        let b = codificar(&regs);
        assert_eq!(decodificar(&b).unwrap(), regs);
        assert!(decodificar(&b[..b.len() - 1]).is_err());
        assert!(decodificar(b"outra coisa").is_err());
        assert_eq!(k_de(&regs[0].0), Some((*b"am", tse::chave(2550, 1, 550))));
    }
}
