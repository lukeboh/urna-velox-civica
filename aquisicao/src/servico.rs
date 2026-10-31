//! `uv-servico`: o serviço que roda a noite da eleição inteira sem intervenção (etapa 3, `0.3.0`).
//!
//! A partir do `candidatos.json`: relê os `cs.json` de todas as UFs a cada `--intervalo-cs` (`If-None-Match`), coleta
//! cada seção nova ou com disponibilização alterada (`aux.json` → BU) dentro dos limites de rede (teto de 100 req/s e
//! pausa ≥ 10 min depois de qualquer HTTP 429 contra o TSE), totaliza em fluxo com o motor, recalcula o
//! "matematicamente eleito / 2º turno" a cada lote, publica (`<saida>/publicacao/`, gravação atômica) e grava os
//! eventos. Persistência: os BUs em `.uvpk` só de acréscimo e o **diário de seções** (`<saida>/secoes.log`, só de
//! acréscimo), que decide na retomada qual BU de cada seção está valendo — `kill -9` em qualquer instante e reinício:
//! nenhum BU perdido nem contado duas vezes. No fim (todas as abrangências com as seções esperadas), publica a situação
//! final e confere com os `-u.json` oficiais (`conferencia.md`/`.json`). Saúde em `saude.json` (a cada ≤ 5 s) e log
//! estruturado em `servico.log`.
//!
//! Arquitetura: o motor não faz rede (ADR-012/014). A rede (acompanhamento, coleta, oficiais) roda em tarefas `tokio`;
//! o motor roda numa thread nativa, que recebe as mensagens por canal ([`Msg`]) e é a única dona do estado, do diário e
//! dos pacotes.

use crate::coletor::{self, ConfigRede, Rede};
use crate::tse::{self, Alvo, Aux, ConfigSecoes, Secao};
use bytes::Bytes;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering::Relaxed};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use urna_velox::acompanhamento::Acompanhamento;
use urna_velox::candidatos::Candidatos;
use urna_velox::catalogo::Catalogo;
use urna_velox::estado::{Agregador, Aplicado, Estado};
use urna_velox::oficial::{Abrangencia, Oficial};
use urna_velox::pacote::{self, Escritor, Registro};
use urna_velox::resultado::{gravar_atomico, Momento, Publicador};

pub type ChaveUf = ([u8; 2], u64);

#[derive(Clone, Debug)]
pub struct Config {
    pub alvo: Alvo,
    /// borda pública (limite de 100 req/s por órgão) ou modo espelho, `interno` (teto calibrado ou nenhum); ver `coletor::Acesso`
    pub acesso: coletor::Acesso,
    pub candidatos: PathBuf,
    /// pacotes `.uvpk` do serviço (retomada: os que já estão aqui são relidos)
    pub dados: PathBuf,
    pub saida: PathBuf,
    /// segundos (hora do site) entre releituras dos `cs.json` de todas as UFs enquanto há seções chegando ou na fila;
    /// no site simulado são divididos pela aceleração
    pub intervalo_cs: f64,
    /// segundos (hora do site) entre ciclos quando o serviço está ocioso (fila vazia e nada novo no último ciclo): cada
    /// ciclo lê algumas UFs em rodízio — contra o TSE no máximo [`OCIOSO_REQ_S`] por segundo
    pub intervalo_cs_ocioso: f64,
    /// req/s pedidas (teto: 100 contra o TSE; ver `coletor::limites`)
    pub rps: f64,
    /// aceleração do relógio do site simulado: só com base local (`127.0.0.1`/`localhost`)
    pub aceleracao_local: f64,
    /// controle do replay (`crate::controle_replay`: velocidade 0 e 1–300×, avançar e voltar na noite): só com base
    /// local e `--http`; as esperas seguem a aceleração atual e o teto local de requisições é o da busca (900×).
    /// **Voltar no tempo apaga `dados` e `saida`** (o serviço recomeça do zero)
    pub controle_replay: bool,
    pub concorrencia: usize,
    pub aceitar_incompleto: bool,
    /// só estas UFs (acompanhamento, fim da apuração e conferência)
    pub ufs: Option<Vec<String>>,
    /// conferência periódica (minutos reais; 0 = só no fim ou com o arquivo-sinal `CONFERIR`)
    pub conferir_a_cada_min: f64,
    /// continua acompanhando depois da conferência final (padrão: sai com código 0)
    pub continuar: bool,
    /// sondagem das seções sem disponibilização quando faltam poucas para fechar uma abrangência (não instaladas)
    pub sondar: bool,
    /// intervalo mínimo entre duas publicações (ms; 0 = depois de cada lote)
    pub cadencia_ms: u64,
    pub timeout_s: u64,
    /// painel no terminal: prefixo do arquivo de publicação a mostrar (ex.: `br-c0001`); `None` = sem painel
    pub painel: Option<String>,
    /// camada HTTP/WebSocket (`--http`): `None` = desligada (`docs/arquitetura/PROTOCOLO_WS.md`)
    pub http: Option<crate::difusao::Opcoes>,
    /// modo federado (2.0, ADR-028): `None` = nó único, como até a 1.0
    pub federacao: Option<crate::federacao::no::Opcoes>,
    /// endereço local de saída (no simulador, cada nó da federação sai por um `127.0.0.x`)
    pub ip_local: Option<std::net::IpAddr>,
    /// CSVs de métricas (`recebido_tse` de cada seção, do `aux.json`): as seções novas de cada ciclo dos `cs.json`
    /// entram na fila na ordem de recebimento do BU no TSE. Vazio: na ordem de disponibilização do `cs.json`. Nos dois
    /// casos as UFs se misturam (antes, cada ciclo enfileirava UF por UF, em ordem alfabética: contra uma eleição já
    /// encerrada, a coleta ia AC, AL, AM, AP, BA… em vez da ordem da noite).
    pub ordem: Vec<PathBuf>,
    /// comparação com o TSE (etapa 7; só tem efeito com `http`): `None` = desligada
    pub tse: Option<crate::tse_resultados::Opcoes>,
    /// sem coleta: nenhum `cs.json`, `aux.json` ou BU — só a publicação da partida, a difusão e a comparação com o TSE
    /// (ensaio da comparação contra o site real, etapa 7, sem coletar uma eleição inteira)
    pub sem_coleta: bool,
    /// previsão do "matematicamente" e do fim da apuração (etapa 8, `crate::previsao`); intervalo 0 = desligada
    pub previsao: crate::previsao::Opcoes,
    /// Rótulo de simulação (ciclo diário de 18 a 23/10/2026): vai na `saude.json` e na mensagem `estado` (`simulacao`),
    /// e a página mostra no selo do topo que é uma simulação com dados reais do TSE. `None` = noite real.
    pub simulacao: Option<String>,
    /// sondas (padrão *Test Message*, `crate::sonda`, `docs/arquitetura/SONDA.md`): segundos reais entre duas; 0 = desligadas
    pub sonda_intervalo_s: f64,
    /// sonda pendente há mais que isto → alerta `sonda_atrasada` (segundos; 0 = o dobro do intervalo, no mínimo 5 s)
    pub sonda_limite_s: f64,
    /// acervo de versões da série de consulta (etapa 1.1, `crate::serie`): pasta onde toda versão de todo arquivo
    /// publicado é guardada; None = desligado
    pub serie: Option<PathBuf>,
}

/// Horário de recebimento no TSE de cada seção, das métricas em `cfg.ordem` (vazio sem arquivos).
fn carregar_ordem(arqs: &[PathBuf]) -> anyhow::Result<HashMap<ChaveUf, i64>> {
    if arqs.is_empty() { return Ok(HashMap::new()); }
    let ps: Vec<&std::path::Path> = arqs.iter().map(|p| p.as_path()).collect();
    Ok(urna_velox::metricas::Metricas::ler(&ps)?.recebido)
}

/// Chave de ordenação de uma seção nova: recebimento no TSE (se conhecido), senão disponibilização no `cs.json`;
/// empate pela UF e pela seção, para a ordem ser determinística.
fn chave_ordem(ordem: &HashMap<ChaveUf, i64>, s: &Secao) -> (i64, [u8; 2], u64) {
    let k = (s.uf, s.chave());
    (ordem.get(&k).copied().unwrap_or_else(|| t_disp(&s.disponivel)), s.uf, k.1)
}

/// Medidas em ms: as últimas `MAX` (percentis da janela recente: `p50`, `p99`) e um histograma da execução inteira
/// (`p50_total`, `p99_total`, `max_total`; resolução de 0,01 ms até 100 ms e de 1 ms até 10 s).
#[derive(Clone)]
pub struct Amostra { v: Vec<f64>, i: usize, pub n: u64, hist: Vec<u64>, max_total: f64, janela: usize }

impl Default for Amostra {
    fn default() -> Self { Amostra { v: vec![], i: 0, n: 0, hist: vec![0; Self::BALDES], max_total: 0.0, janela: Self::MAX } }
}

impl Amostra {
    const MAX: usize = 50_000;
    const BALDES: usize = 10_000 + 9_901;
    /// Janela menor (a `saude()` ordena a janela a cada 2 s na thread do motor: medidas por lote, como as dos marcos,
    /// não precisam de 50 mil amostras — cada janela grande a mais custava ~3 ms a cada 2 s, visível no p99 de BU →
    /// publicação do B).
    pub fn com_janela(n: usize) -> Amostra { Amostra { janela: n.max(1), ..Amostra::default() } }
    fn balde(x: f64) -> usize {
        if x < 100.0 { (x.max(0.0) * 100.0) as usize } else { (10_000 + (x - 100.0) as usize).min(Self::BALDES - 1) }
    }
    fn valor(b: usize) -> f64 { if b < 10_000 { (b as f64 + 0.5) / 100.0 } else { 100.0 + (b - 10_000) as f64 + 0.5 } }
    pub fn por(&mut self, x: f64) {
        if self.v.len() < self.janela { self.v.push(x); } else { self.v[self.i] = x; self.i = (self.i + 1) % self.janela; }
        self.n += 1;
        self.hist[Self::balde(x)] += 1;
        self.max_total = self.max_total.max(x);
    }
    pub fn pct(&self, p: f64) -> f64 {
        let mut s = self.v.clone();
        s.sort_by(|a, b| a.partial_cmp(b).unwrap());
        coletor::percentil(&s, p)
    }
    /// Percentil da execução inteira (pelo histograma).
    pub fn pct_total(&self, p: f64) -> f64 {
        if self.n == 0 { return 0.0; }
        let alvo = ((p / 100.0) * self.n as f64).ceil().max(1.0) as u64;
        let mut acc = 0;
        for (b, &q) in self.hist.iter().enumerate() { acc += q; if acc >= alvo { return Self::valor(b); } }
        self.max_total
    }
    /// Resumo para a `saude.json` (uma ordenação só da janela: roda a cada 2 s, para 7 amostras de até 50 mil).
    pub fn json(&self) -> Value {
        let mut s = self.v.clone();
        s.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
        json!({"n": self.n, "p50": arred(coletor::percentil(&s, 50.0)), "p99": arred(coletor::percentil(&s, 99.0)), "max": arred(s.last().copied().unwrap_or(0.0).max(0.0)),
            "p50_total": arred(self.pct_total(50.0)), "p99_total": arred(self.pct_total(99.0)), "max_total": arred(self.max_total)})
    }
}

fn arred(x: f64) -> f64 { (x * 1000.0).round() / 1000.0 }

/// Contadores da rede e da coleta.
#[derive(Default)]
pub struct Contadores {
    pub cs_lidos: AtomicU64,
    pub cs_304: AtomicU64,
    pub cs_erros: AtomicU64,
    pub aux: AtomicU64,
    pub aux_404: AtomicU64,
    pub bu_baixados: AtomicU64,
    pub bu_do_pacote: AtomicU64,
    pub confirmadas: AtomicU64,
    pub sondadas: AtomicU64,
    pub oficiais: AtomicU64,
    pub erros_rede: AtomicU64,
}

/// Estado compartilhado entre a rede (tokio) e o motor (thread).
pub struct Comum {
    pub cfg: Config,
    pub rede: Rede,
    pub parar: AtomicBool,
    /// acompanhamento dos `cs.json` em modo ocioso (rodízio rápido) ou trabalhando (todas as UFs a cada `intervalo_cs`)
    pub cs_ocioso: AtomicBool,
    /// seções na fila de coleta ou em andamento (inclui as que esperam nova tentativa)
    pub fila: AtomicI64,
    pub lat_aux: Mutex<Amostra>,
    pub lat_bu: Mutex<Amostra>,
    pub cont: Contadores,
    /// UF → seções com disponibilização no último `cs.json` lido
    pub publicadas_cs: Mutex<BTreeMap<String, u64>>,
    /// UF → seções agregadas (com `nsp`) no último `cs.json` lido: nunca sondadas (não têm urna nem `aux.json`)
    pub agregadas_cs: Mutex<HashMap<String, HashSet<u64>>>,
    /// sobe quando as agregadas de alguma UF mudam no `cs.json`: o motor as tira do total de seções (B-051)
    pub agregadas_ger: std::sync::atomic::AtomicU64,
    /// BU que vale em cada seção ([`impressao`] do hash do diretório, anulada) — para não baixar de novo o que não mudou
    pub aplicados: Mutex<HashMap<ChaveUf, (u64, bool)>>,
    /// BUs que estão nos pacotes sem valer em nenhuma seção (queda antes do diário): (sha256 do .vsc, arquivo, posição, tamanho)
    pub cache: Mutex<HashMap<ChaveUf, Vec<([u8; 32], PathBuf, u64, u32)>>>,
    /// relógio do site: maior `Date` visto (hora de Brasília, segundos)
    pub relogio_site: AtomicI64,
    /// relógio do sistema e a âncora com a noite do TSE (M-003, `crate::relogio`)
    pub relogio: Arc<crate::relogio::Relogio>,
    pub t0: Instant,
    /// multiplica as esperas (novas tentativas, ritmo dos oficiais): 1/aceleração no site simulado (bits de um `f64`;
    /// muda com o controle de replay)
    pub escala: AtomicU64,
    pub tx_motor: crossbeam_channel::Sender<Msg>,
    /// BU recebido → arquivo publicado gravado (ms; medido pela thread de gravação)
    pub lat_pub: Mutex<Amostra>,
    /// gravação dos arquivos de uma rodada da thread de gravação (ms)
    pub custo_grav: Mutex<Amostra>,
    /// `sync_data` dos pacotes e do diário a cada 2 s (ms; medido pela thread de sincronia)
    pub custo_sync: Mutex<Amostra>,
    /// camada de difusão (`--http`): recebe os arquivos da publicação antes da gravação, os eventos e o estado
    pub difusao: Option<Arc<crate::difusao::Hub>>,
    /// nó da federação (`--federacao`): decide quem coleta cada seção e troca os resultados pelo storage
    pub federado: Option<Arc<crate::federacao::no::Federado>>,
    /// comparação com o TSE (etapa 7; `--http` sem `--sem-tse`)
    pub tse: Option<Arc<crate::tse_resultados::TseResultados>>,
    log: Mutex<Option<std::fs::File>>,
    alertas_arq: Mutex<Option<std::fs::File>>,
    pub alertas: Mutex<BTreeMap<String, u64>>,
    /// verificador das sondas dentro do serviço (`crate::sonda::Vigia`); `None` = sondas desligadas
    pub sonda: Option<Mutex<crate::sonda::Vigia>>,
}

impl Comum {
    /// Uma linha JSON no `servico.log`.
    pub fn registrar(&self, nivel: &str, evento: &str, mut campos: Value) {
        if let Value::Object(m) = &mut campos {
            m.insert("t".into(), json!(iso_agora()));
            m.insert("nivel".into(), json!(nivel));
            m.insert("evento".into(), json!(evento));
        }
        if let Some(f) = self.log.lock().unwrap().as_mut() { let _ = writeln!(f, "{campos}"); }
        if nivel != "info" { eprintln!("[{nivel}] {evento}: {campos}"); }
    }
    /// Alerta: contado por tipo, uma linha em `alertas.csv` e no `servico.log`.
    pub fn alerta(&self, tipo: &str, texto: &str) {
        *self.alertas.lock().unwrap().entry(tipo.to_string()).or_default() += 1;
        if let Some(f) = self.alertas_arq.lock().unwrap().as_mut() { let _ = writeln!(f, "{},{tipo},{}", iso_agora(), texto.replace([',', '\n'], ";")); }
        self.registrar("alerta", tipo, json!({"texto": texto}));
    }
    pub(crate) fn ver_relogio(&self, data: Option<&str>) -> i64 {
        let site = data.and_then(crate::data_http_brasilia);
        let t = site.unwrap_or_else(crate::agora_brasilia);
        self.relogio_site.fetch_max(t, Relaxed);
        if let Some(s) = site { self.relogio.ver_site(s); }
        t
    }
    pub(crate) fn espera(&self, s: f64) -> Duration { Duration::from_secs_f64((s * f64::from_bits(self.escala.load(Relaxed))).max(0.01)) }
    /// Esperas para a aceleração `a` do site simulado (pausa = esperas de 1×); a âncora do relógio do sistema segue.
    pub fn definir_escala(&self, a: f64) { self.escala.store((1.0 / a.max(1.0)).to_bits(), Relaxed); self.relogio.mudar_aceleracao(a); }
    fn escala_atual(&self) -> f64 { f64::from_bits(self.escala.load(Relaxed)) }
    /// Dorme `s` segundos **em hora do site**, seguindo a velocidade do momento: em fatias de até 200 ms, cada uma contada
    /// pela escala atual — se o controle de replay muda a velocidade no meio da espera, o resto da espera já segue a nova.
    /// Com a escala fixa, dorme exatamente `s · escala` (em fatias).
    pub(crate) async fn dormir_site(&self, mut s: f64) {
        while !self.parar.load(Relaxed) {
            let esc = self.escala_atual();
            let real = s * esc;
            if real <= 0.0005 { break; }
            let fatia = real.min(0.2);
            tokio::time::sleep(Duration::from_secs_f64(fatia)).await;
            s -= fatia / esc;
        }
    }
}

fn iso_agora() -> String {
    let t = crate::agora_brasilia();
    let ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_millis()).unwrap_or(0);
    let (a, m, d, h, mi, s) = urna_velox::metricas::civil(t);
    format!("{a:04}-{m:02}-{d:02}T{h:02}:{mi:02}:{s:02}.{ms:03}-03:00")
}

/// Um BU coletado, pronto para o motor.
pub struct BuRecebido {
    pub k: ChaveUf,
    pub hash: String,
    pub vsc: [u8; 32],
    /// SHA-256 dos bytes do BU (identidade do conteúdo: reenvio idêntico × diferente)
    pub sha: [u8; 32],
    pub situacao: String,
    pub anulada: bool,
    pub disp: String,
    pub recebido: String,
    /// hora do site quando o BU foi recebido (`Date`)
    pub hora_site: i64,
    pub bytes: Bytes,
    /// já está num pacote (arquivo, posição, tamanho)
    pub pos: Option<(PathBuf, u64, u32)>,
    pub chegada: Instant,
}

/// Mensagens para o motor.
pub enum Msg {
    Bu(Box<BuRecebido>),
    NaoInstalada { k: ChaveUf, disp: String },
    /// o `aux.json` foi relido e nada mudou (só a disponibilização)
    Confirmada { k: ChaveUf, disp: String },
    /// seção que não se resolveu (aux 404 persistente, sem BU, situação desconhecida, erro de rede) depois das novas tentativas; `disp` = disponibilização no `cs.json` (vazia na sondagem)
    Pendente { k: ChaveUf, tipo: String, motivo: String, disp: String },
    /// sondada e sem `aux.json`: seção agregada (não tem BU nem conta em `s.st`)
    Agregada { k: ChaveUf },
    Oficiais { baixados: Vec<PathBuf>, faltando: Vec<String> },
    /// mensagem de teste (`crate::sonda`): atravessa o motor sem tocar no estado e segue para a gravação
    Sonda(Box<crate::sonda::Sonda>),
    Parar,
}

/// Pedidos do motor à rede.
pub enum Pedido {
    /// (URL, nome do arquivo) dos `-u.json` a baixar para a conferência
    Oficiais(Vec<(String, String)>),
    Sondar(Vec<Secao>),
    /// seções publicadas que ficaram pendentes: pedir de novo (como seção nova, tentativa 0)
    Recoletar(Vec<Secao>),
}

/// Seção publicada pendente (o `aux.json` ou o BU não vieram depois das novas tentativas: site fora do ar, erros,
/// 404 no CDN). O motor a pede de novo sozinho a cada `PEND_BASE_S · 2^min(rodada, 4)` s (hora do site: 2, 4, 8, 16,
/// 32 min…), até [`PEND_RODADAS`] vezes — antes ela só voltava se a disponibilização mudasse ou numa retomada, e uma
/// queda do site mais longa que as novas tentativas deixava a apuração sem fim (ensaio C-queda de 13/10: 160 seções).
/// Só seções com disponibilização: a regra "seção sem disponibilização não gera requisição" continua valendo.
pub struct Pend { pub motivo: String, pub disp: String, pub rodadas: u32, pub prox: Instant }
pub const PEND_BASE_S: f64 = 120.0;
pub const PEND_RODADAS: u32 = 12;

/// Tarefa de coleta de uma seção.
pub struct Tarefa { pub secao: Secao, pub tentativa: u32, pub sondagem: bool }

/// Releitura dos `cs.json` com o serviço ocioso, contra o TSE: o rodízio lê estas UFs por ciclo, e o ciclo nunca é
/// mais curto que 1 s — no máximo estas requisições por segundo (com `--intervalo-cs-ocioso` maior, menos). 4, não 5:
/// com o primeiro ciclo completo (28 UFs) a média fica abaixo dos ≤ 5 req/s do teste pré-eleição mesmo em janelas
/// curtas (com 5, 60 s deram 5,38 req/s)
pub const OCIOSO_REQ_S: f64 = 4.0;

/// Sondagem das seções sem disponibilização (não instaladas, ver [`Motor::sondar`]): cada seção é pedida de novo a cada
/// este intervalo (s, hora do site) enquanto continuar sem BU e sem "Nao instalada" (combinado com o Luciano em
/// 16/10/2026: no máximo 1 vez a cada 5 min por seção, só as sem BU)
pub const SONDA_REPETIR_S: i64 = 300;
/// ... no máximo estas seções a sondar numa UF (acima disso, alerta e nenhuma sondagem nela)
pub const SONDA_ABR: usize = 1500;
/// ... no máximo estas requisições por rodada de verificação (uma por segundo): ≤ 2 req/s
pub const SONDA_POR_RODADA: usize = 2;
/// ... numa UF do Brasil, só quando faltam no máximo estas seções para o `s.ts` (no exterior, 5 % ou 50)
pub const SONDA_FALTAM_UF: u64 = 50;

/// Se as seções sem disponibilização de uma UF entram na sondagem ([`Motor::sondar`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Sonda {
    Nao,
    /// só quando toda seção com disponibilização no `cs.json` da UF já estiver resolvida
    SeResolvidas,
    /// já, sem esperar a disponibilização no `cs.json`
    Antecipada,
}

/// Decide a sondagem de uma UF: `n` seções esperadas, `ap` apuradas, `publicadas` com disponibilização no último
/// `cs.json` (`None`: ainda não lido), `algo_apurado`: algum BU já aplicado, `domingo`: o relógio do site está num
/// domingo.
fn quando_sondar(zz: bool, com_nsp: bool, n: u64, ap: u64, publicadas: Option<u64>, algo_apurado: bool, domingo: bool) -> Sonda {
    // B-065: também depois que o cs.json do ZZ passa a listar seções (ele anda em lotes, atrás dos aux.json)
    if zz && com_nsp && domingo && publicadas.is_some() && ap < n { return Sonda::Antecipada; }
    if !algo_apurado || (!zz && !com_nsp) || ap == 0 || ap >= n { return Sonda::Nao; }
    let limite = if zz { (n / 20).max(50) } else { SONDA_FALTAM_UF };
    if n - ap > limite { Sonda::Nao } else { Sonda::SeResolvidas }
}

/// O instante (segundos civis de Brasília, como o `relogio_site`) cai num domingo (01/01/1970 foi quinta-feira).
fn domingo(t: i64) -> bool { (t.div_euclid(86400) + 4).rem_euclid(7) == 0 }

/// B-057: reforço da sondagem antecipada do exterior. Requisições por rodada (uma por segundo) com a regra antecipada
/// ligada e a fila de seções do Brasil quase vazia ([`SONDA_FILA_OCIOSA`]); em cada nó (cada órgão tem o seu teto). B-060: 12
/// (eram 8), porque agora todas são requisições deste nó (antes metade era do outro nó e descartada); uma cota (1/4) é das
/// sondadas há mais tempo
pub const SONDA_POR_RODADA_EXTERIOR: usize = 12;
/// ... só com a fila de pedidos abaixo disto: o exterior não toma requisições do Brasil
pub const SONDA_FILA_OCIOSA: i64 = 20;
/// ... seção "devida" (a hora em que o turno anterior a disponibilizou, de 30 min à frente até 2 h atrás): a cada 20 s
pub const SONDA_REPETIR_DEVIDA_S: i64 = 20;
/// ... seção que já devia ter chegado há mais de 2 h (e há menos de 12 h) e não veio: a cada 60 s
pub const SONDA_REPETIR_ATRASADA_S: i64 = 60;
const SONDA_DEVIDA_ANTES_S: i64 = 1800;
const SONDA_DEVIDA_DEPOIS_S: i64 = 7200;

/// Calendário de chegada do exterior: chave da seção → segundo do dia (hora do site) em que o `cs.json` do turno
/// anterior a disponibilizou. As seções agregadas (sem data) ficam de fora.
fn calendario_de_cs(texto: &str) -> HashMap<u64, i64> {
    let Ok(cs) = serde_json::from_str::<crate::tse::ConfigSecoes>(texto) else { return HashMap::new() };
    cs.secoes("zz").into_iter().filter_map(|s| urna_velox::metricas::segundos(&s.disponivel).map(|t| (s.chave(), t.rem_euclid(86400)))).collect()
}

/// `<pasta do candidatos.json>/fontes/cs/zz-p<pleito-1>-cs.json` (o 1º turno de um 2º turno); sem o arquivo, sem calendário.
fn carregar_calendario_exterior(cfg: &Config) -> HashMap<u64, i64> {
    let Some(dir) = cfg.candidatos.parent() else { return HashMap::new() };
    let p = dir.join("fontes").join("cs").join(format!("zz-p{:06}-cs.json", cfg.alvo.pleito.saturating_sub(1)));
    std::fs::read_to_string(&p).map(|t| calendario_de_cs(&t)).unwrap_or_default()
}

/// Faixa (0 = devida agora, 1 = atrasada, 2 = futura ou sem calendário) e intervalo de repetição (s) de uma seção do
/// exterior na sondagem antecipada, pelo segundo do dia esperado (`esperado`) e o relógio do site (`agora`).
fn faixa_calendario(esperado: Option<i64>, agora: i64) -> (u8, i64) {
    let Some(e) = esperado else { return (2, SONDA_REPETIR_S) };
    // atraso em relação ao horário esperado, em (-12 h, +12 h]: negativo = ainda não deu a hora
    let mut d = (agora.rem_euclid(86400) - e).rem_euclid(86400);
    if d > 43200 { d -= 86400; }
    if (-SONDA_DEVIDA_ANTES_S..=SONDA_DEVIDA_DEPOIS_S).contains(&d) { (0, SONDA_REPETIR_DEVIDA_S) }
    else if d > SONDA_DEVIDA_DEPOIS_S { (1, SONDA_REPETIR_ATRASADA_S) }
    else { (2, SONDA_REPETIR_S) }
}

/// Faixa e repetição de uma seção do exterior na sondagem antecipada: `quente` = o município dela já tem BU (B-059).
fn faixa_secao(esperado: Option<i64>, agora: i64, quente: bool) -> (u8, i64) {
    if quente { return (0, SONDA_REPETIR_DEVIDA_S); }
    faixa_calendario(esperado, agora)
}

/// Requisições da sondagem por rodada (uma por segundo): o reforço do exterior ([`SONDA_POR_RODADA_EXTERIOR`]) só com a
/// regra antecipada ligada, nenhum 429 desde a partida e a fila de pedidos quase vazia; senão [`SONDA_POR_RODADA`].
fn sondas_por_rodada(antecipada: bool, tomou_429: bool, fila: i64) -> usize {
    if antecipada && !tomou_429 && fila < SONDA_FILA_OCIOSA { SONDA_POR_RODADA_EXTERIOR } else { SONDA_POR_RODADA }
}

/// Uma seção candidata à sondagem: prioridade (faixa × 2 + 0/1), quando foi pedida pela última vez (`i64::MIN` = nunca) e a chave.
type Candidata = (u8, i64, ChaveUf);

/// A seção está devida? Nunca pedida, ou pedida há `repetir` s ou mais (hora do site).
fn devida(ultima: Option<i64>, agora: i64, repetir: i64) -> bool { ultima.is_none_or(|t| agora - t >= repetir) }

/// B-060: só as candidatas deste nó (a partição da federação) entram na disputa pelo orçamento da rodada; as do outro nó
/// são descartadas depois, sem requisição (`coletor`), e gastavam metade do orçamento à toa.
fn so_do_no(mut lista: Vec<Candidata>, minha: impl Fn(&ChaveUf) -> bool) -> Vec<Candidata> {
    lista.retain(|c| minha(&c.2));
    lista
}

/// B-060: as `por_rodada` seções da rodada. `cota` delas (só com o reforço) vão para as sondadas há mais tempo (as nunca
/// sondadas primeiro), qualquer que seja a faixa: nenhuma seção fica sem consulta por falta de orçamento, mesmo com a faixa
/// quente sozinha pedindo mais do que a rodada comporta. As demais vão por prioridade (faixa, depois a mais antiga).
fn escolher_sondas(mut lista: Vec<Candidata>, por_rodada: usize, cota: usize) -> Vec<ChaveUf> {
    let cota = cota.min(por_rodada);
    lista.sort_unstable();
    let por_prioridade = por_rodada - cota;
    let mut escolhidas: Vec<ChaveUf> = lista.iter().take(por_prioridade).map(|c| c.2).collect();
    // a cota: entre as que sobraram, as sondadas há mais tempo (as nunca sondadas têm `i64::MIN`)
    let mut resto: Vec<&Candidata> = lista.iter().skip(por_prioridade).collect();
    resto.sort_unstable_by_key(|c| (c.1, c.0, c.2));
    escolhidas.extend(resto.into_iter().take(cota).map(|c| c.2));
    escolhidas
}

// ------------------------------------------------------------------ diário de seções

/// O que vale numa seção (compacto: meio milhão de seções numa eleição geral). O BU está em `<dados>/<uf>.uvpk`, na
/// posição `pos`.
#[derive(Clone, Debug)]
pub enum Situ {
    /// `imp`: [`impressao`] do hash do diretório (o texto, 88 caracteres, fica no diário em disco)
    Bu { imp: u64, sha: [u8; 32], anulada: bool, pos: u64, tam: u32 },
    NaoInstalada,
    Agregada,
}

/// Disponibilização ("dd/mm/aaaa hh:mm:ss") em segundos (−1 se ilegível), para comparar sem guardar o texto.
pub fn t_disp(s: &str) -> i64 { urna_velox::metricas::segundos(s).unwrap_or(-1) }

/// Impressão de 64 bits do hash do diretório (FNV-1a), para a rede saber o BU que vale sem guardar o texto.
pub fn impressao(h: &str) -> u64 { h.bytes().fold(0xcbf29ce484222325u64, |a, b| (a ^ b as u64).wrapping_mul(0x100000001b3)) }

/// Pacote da UF da seção.
fn arquivo_uf(uf: [u8; 2]) -> String { format!("{}.uvpk", String::from_utf8_lossy(&uf)) }

pub const CAB_DIARIO: &str = "t_ms,acao,uf,municipio,zona,secao,hash,sha256,situacao,anulada,disponivel,recebido,arquivo,pos,tam,seq";

fn hex(b: &[u8]) -> String { b.iter().map(|x| format!("{x:02x}")).collect() }
fn de_hex(s: &str) -> Option<[u8; 32]> {
    if s.len() != 64 { return None; }
    let v: Vec<u8> = (0..32).filter_map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).ok()).collect();
    v.try_into().ok()
}

/// Uma linha do diário.
#[allow(clippy::too_many_arguments)]
fn linha_diario(acao: &str, k: ChaveUf, hash: &str, sha: Option<&[u8; 32]>, situacao: &str, anulada: bool, disp: &str, recebido: &str, arquivo: &str, pos: u64, tam: u32, seq: u64) -> String {
    let ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
    let limpo = |s: &str| s.replace([',', '\n'], " ");
    format!("{ms},{acao},{},{},{},{},{hash},{},{},{},{},{},{arquivo},{pos},{tam},{seq}\n", String::from_utf8_lossy(&k.0), (k.1 >> 32) as u32, (k.1 >> 16) as u16, k.1 as u16,
        sha.map(|s| hex(s)).unwrap_or_default(), limpo(situacao), anulada as u8, limpo(disp), limpo(recebido))
}

/// Lê o diário: o que vale em cada seção e a última disponibilização vista de cada uma (as seções cuja última linha é
/// "pendente" não entram: serão pedidas de novo). Linhas incompletas (queda no meio da escrita) são ignoradas.
pub fn ler_diario(p: &Path) -> anyhow::Result<(HashMap<ChaveUf, Situ>, HashMap<ChaveUf, i64>, usize)> {
    let (mut sit, mut disp) = (HashMap::new(), HashMap::new());
    let mut n = 0;
    let Ok(txt) = std::fs::read_to_string(p) else { return Ok((sit, disp, 0)) };
    for l in txt.lines() {
        let f: Vec<&str> = l.split(',').collect();
        if f.len() != 16 || f[0] == "t_ms" { continue; }
        let (Ok(uf), Ok(mu), Ok(zn), Ok(se)) = (<[u8; 2]>::try_from(f[2].as_bytes()), f[3].parse::<u32>(), f[4].parse::<u16>(), f[5].parse::<u16>()) else { continue };
        let k = (uf, tse::chave(mu, zn, se));
        n += 1;
        match f[1] {
            "aplicado" | "substituido" | "anulado" | "reenvio_identico" => {
                let (Some(sha), Ok(pos), Ok(tam)) = (de_hex(f[7]), f[13].parse::<u64>(), f[14].parse::<u32>()) else { continue };
                if f[12] != arquivo_uf(uf) { continue; }
                sit.insert(k, Situ::Bu { imp: impressao(f[6]), sha, anulada: f[9] == "1", pos, tam });
                disp.insert(k, t_disp(f[10]));
            }
            "nao_instalada" => { sit.insert(k, Situ::NaoInstalada); disp.insert(k, t_disp(f[10])); }
            "agregada" => { sit.entry(k).or_insert(Situ::Agregada); }
            "confirmado" => { disp.insert(k, t_disp(f[10])); }
            "pendente" | "fora_catalogo" | "erro_bu" => { disp.remove(&k); }
            _ => {}
        }
    }
    Ok((sit, disp, n))
}

/// Horário (relógio do site) do último BU aplicado segundo o diário: a maior disponibilização das seções com BU.
/// B-058: sem isso o `ultimo_bu` ficava vazio depois de um reinício até o próximo BU.
fn ultimo_bu_do_diario(sit: &HashMap<ChaveUf, Situ>, disp: &HashMap<ChaveUf, i64>) -> Option<i64> {
    sit.iter().filter(|(_, s)| matches!(s, Situ::Bu { .. })).filter_map(|(k, _)| disp.get(k).copied().filter(|&t| t > 0)).max()
}

/// Leitor de BU que não guarda nada: valida o BU antes de aplicar (um BU ilegível não chega ao estado pela metade).
struct Nulo;
impl urna_velox::bu::Sink for Nulo {
    fn cargo(&mut self, _c: u8, _comp: u32) {}
    fn voto(&mut self, _t: u8, _q: u32, _p: u16, _c: u32) {}
    fn fim_cargo(&mut self) {}
}

fn sha256(b: &[u8]) -> [u8; 32] { Sha256::digest(b).into() }

// ------------------------------------------------------------------ rede (tokio)

type FilaTx = tokio::sync::mpsc::UnboundedSender<Tarefa>;

pub(crate) fn enfileirar(com: &Comum, tx: &FilaTx, t: Tarefa) {
    com.fila.fetch_add(1, Relaxed);
    if tx.send(t).is_err() { com.fila.fetch_sub(1, Relaxed); }
}

/// Nova tentativa depois de `base · 2^min(tentativa, exp)` segundos (escalados no site simulado); a seção continua
/// contada na fila.
fn reenfileirar(com: &Arc<Comum>, tx: &FilaTx, mut t: Tarefa, base: f64, exp: u32) {
    let s = base * (1u32 << t.tentativa.min(exp)) as f64;
    t.tentativa += 1;
    com.fila.fetch_add(1, Relaxed);
    let (com, tx, d) = (com.clone(), tx.clone(), com.espera(s));
    tokio::spawn(async move {
        tokio::time::sleep(d).await;
        if com.parar.load(Relaxed) || tx.send(t).is_err() { com.fila.fetch_sub(1, Relaxed); }
    });
}

/// Relê os `cs.json` das UFs a cada `intervalo` segundos e enfileira cada seção nova ou com disponibilização
/// alterada. Seção sem disponibilização não gera requisição aqui (a sondagem, [`Motor::sondar`], cuida das exceções).
pub async fn acompanhar(com: Arc<Comum>, ufs: Vec<String>, mut conhecidas: HashMap<ChaveUf, i64>, tx: FilaTx) {
    let mut etags: HashMap<String, String> = HashMap::new();
    let mut ausentes: HashSet<String> = HashSet::new();
    let mut ciclos = 0u64;
    let mut ultimo_log = Instant::now() - Duration::from_secs(3600);
    // Dois ritmos: trabalhando (seções chegando ou na fila) = todas as UFs a cada `intervalo_cs`; ocioso = um ciclo a
    // cada `intervalo_cs_ocioso` lendo só algumas UFs em rodízio, para perceber a primeira seção nova logo sem gastar
    // taxa. Ao achar seção nova no rodízio, o ciclo seguinte é completo e imediato. O primeiro ciclo é sempre completo.
    // Contra o TSE: sempre OCIOSO_REQ_S UFs por ciclo, e o intervalo (nunca abaixo de 1 s) decide o gasto — 1 s = 4 req/s
    // (cada UF a cada ~7 s), 2 s = 2 req/s, 5 s = 0,8 req/s. Contra o simulador, todas as UFs a cada ciclo.
    let local = coletor::base_local(&com.cfg.alvo.base);
    let por_ciclo = if local { ufs.len() } else { (OCIOSO_REQ_S as usize).clamp(1, ufs.len().max(1)) };
    let intervalo_ocioso = if local { com.cfg.intervalo_cs_ocioso } else { com.cfg.intervalo_cs_ocioso.max(por_ciclo as f64 / OCIOSO_REQ_S) };
    let mut ocioso = false;
    let mut rodizio = 0usize;
    let ordem = match carregar_ordem(&com.cfg.ordem) {
        Ok(o) => o,
        Err(e) => { com.alerta("ordem_ilegivel", &format!("{e}")); HashMap::new() }
    };
    if !com.cfg.ordem.is_empty() {
        com.registrar("info", "ordem", json!({"secoes_com_recebimento": ordem.len(), "arquivos": com.cfg.ordem.len()}));
    }
    while !com.parar.load(Relaxed) {
        let t = Instant::now();
        let lote: Vec<&String> = if ocioso && por_ciclo < ufs.len() {
            let v = (0..por_ciclo).map(|i| &ufs[(rodizio + i) % ufs.len()]).collect();
            rodizio = (rodizio + por_ciclo) % ufs.len();
            v
        } else { ufs.iter().collect() };
        let rs = futures::future::join_all(lote.into_iter().map(|uf| {
            let (com, e) = (com.clone(), etags.get(uf).cloned());
            async move { (uf.clone(), com.rede.get(&com.cfg.alvo.url_cs(uf), e.as_deref(), None).await) }
        })).await;
        let (mut novas, mut mudaram) = (0usize, 0usize);
        // seções novas deste ciclo, de todas as UFs: entram na fila só depois, na ordem de recebimento no TSE
        let mut lote_novas: Vec<((i64, [u8; 2], u64), Secao)> = Vec::new();
        for (uf, r) in rs {
            match r {
                Ok(r) => {
                    com.cont.cs_lidos.fetch_add(1, Relaxed);
                    com.ver_relogio(r.data.as_deref());
                    if r.status == 304 { com.cont.cs_304.fetch_add(1, Relaxed); continue; }
                    if r.status != 200 {
                        com.cont.cs_erros.fetch_add(1, Relaxed);
                        if ausentes.insert(uf.clone()) { com.alerta("cs_ausente", &format!("{uf}: cs.json HTTP {}", r.status)); }
                        continue;
                    }
                    ausentes.remove(&uf);
                    let c: ConfigSecoes = match serde_json::from_slice(&r.corpo) {
                        Ok(c) => c,
                        Err(e) => { com.cont.cs_erros.fetch_add(1, Relaxed); com.alerta("cs_ilegivel", &format!("{uf}: {e}")); continue; }
                    };
                    if let Some(e) = r.etag { etags.insert(uf.clone(), e); }
                    mudaram += 1;
                    let mut publicadas = 0u64;
                    for s in c.secoes(&uf) {
                        if s.disponivel.trim().is_empty() { continue; }
                        publicadas += 1;
                        let k = (s.uf, s.chave());
                        let d = t_disp(&s.disponivel);
                        if conhecidas.get(&k) != Some(&d) {
                            conhecidas.insert(k, d);
                            lote_novas.push((chave_ordem(&ordem, &s), s));
                            novas += 1;
                        }
                    }
                    let ag = c.agregadas();
                    if com.agregadas_cs.lock().unwrap().insert(uf.clone(), ag.clone()).as_ref() != Some(&ag) {
                        com.agregadas_ger.fetch_add(1, Relaxed);
                    }
                    com.publicadas_cs.lock().unwrap().insert(uf, publicadas);
                }
                Err(e) => { com.cont.cs_erros.fetch_add(1, Relaxed); com.cont.erros_rede.fetch_add(1, Relaxed); com.registrar("aviso", "cs_erro", json!({"uf": uf, "erro": e.to_string()})); }
            }
        }
        lote_novas.sort_unstable_by_key(|x| x.0);
        for (_, s) in lote_novas { enfileirar(&com, &tx, Tarefa { secao: s, tentativa: 0, sondagem: false }); }
        ciclos += 1;
        // cada ciclo interpreta até 31,5 MB de cs.json (517 mil seções em 2026) e solta tudo: devolve ao sistema a
        // memória livre, senão a fragmentação das arenas do glibc faz o RSS subir sem parar (1,3 GB a 60×)
        if mudaram > 0 { devolver_memoria(); }
        if novas > 0 || ultimo_log.elapsed() > Duration::from_secs(60) {
            ultimo_log = Instant::now();
            com.registrar("info", "ciclo_cs", json!({"ciclo": ciclos, "cs_alterados": mudaram, "secoes_novas": novas, "fila": com.fila.load(Relaxed),
                "relogio_site": crate::texto_hora(com.relogio_site.load(Relaxed)), "duracao_ms": t.elapsed().as_millis() as u64}));
        }
        // próximo ritmo: ocioso se nada novo e a fila está vazia; achou seção nova no rodízio → ciclo completo já
        let agora_ocioso = novas == 0 && com.fila.load(Relaxed) <= 0;
        // espera em hora do site (o controle de replay pode mudar a velocidade no meio dela)
        let espera_site = if agora_ocioso { intervalo_ocioso } else if ocioso { 0.0 } else { com.cfg.intervalo_cs };
        if agora_ocioso != ocioso {
            com.registrar("info", "ritmo_cs", json!({"modo": if agora_ocioso { "ocioso" } else { "trabalhando" }, "ufs_por_ciclo": if agora_ocioso { por_ciclo } else { ufs.len() },
                "relogio_site": crate::texto_hora(com.relogio_site.load(Relaxed))}));
        }
        ocioso = agora_ocioso;
        com.cs_ocioso.store(ocioso, Relaxed);
        // ciclo de pelo menos 50 ms reais (e 10 ms de espera quando há espera), descontado o tempo do próprio ciclo
        let esc = com.escala_atual();
        let real = if espera_site > 0.0 { (espera_site * esc).max(0.01) } else { 0.0 }.max(0.05);
        com.dormir_site((real - t.elapsed().as_secs_f64()) / esc).await;
    }
}

/// Trabalhador de coleta: tira tarefas da fila até o serviço parar.
pub async fn coletor(com: Arc<Comum>, rx: Arc<tokio::sync::Mutex<tokio::sync::mpsc::UnboundedReceiver<Tarefa>>>, tx: FilaTx) {
    loop {
        let t = { rx.lock().await.recv().await };
        let Some(t) = t else { break };
        // federação: só as seções que a partição dá a este nó vão ao TSE; as outras esperam o dono pelo storage
        let minha = match &com.federado {
            Some(f) if !f.minha(&t.secao) => { if !t.sondagem { f.delegar(t.secao.clone()); } false }
            // é deste nó, mas o resultado desta disponibilização já veio de um par (ex.: coletada por outro enquanto
            // este estava fora, e o motor ainda não aplicou): não vai ao TSE de novo
            Some(f) => { f.resolver(&(t.secao.uf, t.secao.chave())); !f.ja_resolvida(&t.secao) }
            None => true,
        };
        if minha && !com.parar.load(Relaxed) { coletar(&com, t, &tx).await; }
        com.fila.fetch_sub(1, Relaxed);
    }
}

/// Coleta uma seção: `aux.json` → situação → BU (do pacote, se já estiver lá) → mensagem para o motor.
async fn coletar(com: &Arc<Comum>, t: Tarefa, tx: &FilaTx) {
    let s = &t.secao;
    let uf = s.uf_str().to_string();
    let k = (s.uf, s.chave());
    let alvo = &com.cfg.alvo;
    let pendente = |tipo: &str, motivo: String| { let _ = com.tx_motor.send(Msg::Pendente { k, tipo: tipo.into(), motivo, disp: s.disponivel.clone() }); };
    com.cont.aux.fetch_add(1, Relaxed);
    if t.sondagem { com.cont.sondadas.fetch_add(1, Relaxed); }
    let r = match com.rede.get(&alvo.url_aux(&uf, s.municipio, s.zona, s.secao), None, None).await {
        Ok(r) => r,
        Err(e) => {
            com.cont.erros_rede.fetch_add(1, Relaxed);
            // sondagem: a próxima rodada (SONDA_REPETIR_S) pede de novo; não vira pendente
            if t.sondagem {}
            else if t.tentativa < 8 { reenfileirar(com, tx, t, 5.0, 5); }
            else { pendente("erro_rede", format!("aux: {e}")); }
            return;
        }
    };
    com.ver_relogio(r.data.as_deref());
    if r.status == 404 {
        com.cont.aux_404.fetch_add(1, Relaxed);
        if t.sondagem {
            // com as agregadas no cs.json (nsp), 404 é seção ainda sem aux.json: continua esperada e é sondada de novo
            // depois de SONDA_REPETIR_S; sem essa informação na UF (cs.json antigo), 404 quer dizer agregada
            let com_nsp = com.agregadas_cs.lock().unwrap().get(&uf).is_some_and(|a| !a.is_empty());
            if !com_nsp {
                if let Some(f) = &com.federado { f.publicar_sem_bu("agregada", k, ""); }
                let _ = com.tx_motor.send(Msg::Agregada { k });
            }
        }
        else if t.tentativa < 6 { reenfileirar(com, tx, t, 15.0, 6); }
        else { pendente("aux_404", format!("aux.json 404 com disponibilização {}", s.disponivel)); }
        return;
    }
    if r.status != 200 {
        if t.sondagem {}
        else if t.tentativa < 8 { reenfileirar(com, tx, t, 5.0, 5); } else { pendente("erro_rede", format!("aux HTTP {}", r.status)); }
        return;
    }
    com.lat_aux.lock().unwrap().por(r.lat * 1e3);
    let aux: Aux = match serde_json::from_slice(&r.corpo) {
        Ok(a) => a,
        Err(e) => { pendente("aux_ilegivel", format!("aux.json: {e}")); return; }
    };
    let st = aux.st.to_lowercase();
    let Some((hash, nome, st_hash, receb)) = aux.escolher_bu() else {
        if st.starts_with("nao instalad") || st.starts_with("não instalad") {
            if let Some(f) = &com.federado { f.publicar_sem_bu("nao_instalada", k, &s.disponivel); }
            let _ = com.tx_motor.send(Msg::NaoInstalada { k, disp: s.disponivel.clone() });
        } else if t.sondagem {
            // sondada, sem BU e sem "Nao instalada": continua esperada (o TSE ainda pode publicar o BU ou marcá-la)
        } else if t.tentativa < 6 {
            reenfileirar(com, tx, t, 60.0, 0);
        } else {
            pendente("sem_bu", format!("sem BU (situação \"{}\")", aux.st));
        }
        return;
    };
    let sth = st_hash.to_lowercase();
    if !(sth.starts_with("totalizad") || sth.starts_with("recebid") || sth.starts_with("exclu")) {
        pendente("situacao_desconhecida", format!("situação desconhecida: seção \"{}\", BU \"{st_hash}\"", aux.st));
        return;
    }
    let anulada = sth.starts_with("exclu") || st.starts_with("anulad");
    if com.aplicados.lock().unwrap().get(&k).is_some_and(|&(h, a)| h == impressao(hash) && a == anulada) {
        com.cont.confirmadas.fetch_add(1, Relaxed);
        if let Some(f) = &com.federado { f.publicar_sem_bu("confirmada", k, &s.disponivel); }
        let _ = com.tx_motor.send(Msg::Confirmada { k, disp: s.disponivel.clone() });
        return;
    }
    let vsc = tse::sha256_do_hash_dir(hash);
    let no_pacote = if vsc != [0; 32] { com.cache.lock().unwrap().get(&k).and_then(|v| v.iter().find(|x| x.0 == vsc).cloned()) } else { None };
    let (bytes, pos, hora) = match no_pacote.and_then(|(_, a, p, n)| pacote::ler_bu_em(&a, p, n).ok().map(|b| (b, a, p, n))) {
        Some((b, a, p, n)) => { com.cont.bu_do_pacote.fetch_add(1, Relaxed); (Bytes::from(b), Some((a, p, n)), com.relogio_site.load(Relaxed)) }
        None => {
            let url = alvo.url_arquivo(&uf, s.municipio, s.zona, s.secao, hash, nome);
            let b = match com.rede.get(&url, None, None).await {
                Ok(b) => b,
                Err(e) => {
                    com.cont.erros_rede.fetch_add(1, Relaxed);
                    if t.tentativa < 8 { reenfileirar(com, tx, t, 5.0, 5); } else { pendente("erro_rede", format!("BU: {e}")); }
                    return;
                }
            };
            let hora = com.ver_relogio(b.data.as_deref());
            if b.status != 200 {
                if t.tentativa < 6 { reenfileirar(com, tx, t, 15.0, 6); } else { pendente("bu_indisponivel", format!("BU HTTP {}", b.status)); }
                return;
            }
            com.cont.bu_baixados.fetch_add(1, Relaxed);
            com.lat_bu.lock().unwrap().por(b.lat * 1e3);
            (b.corpo, None, hora)
        }
    };
    let sha = sha256(&bytes);
    let bu = BuRecebido {
        k, hash: hash.to_string(), vsc, sha, situacao: st_hash.to_string(), anulada, disp: s.disponivel.clone(), recebido: receb,
        hora_site: hora, bytes, pos, chegada: Instant::now(),
    };
    if let Some(f) = &com.federado { f.publicar_bu(&bu); }
    let _ = com.tx_motor.send(Msg::Bu(Box::new(bu)));
}

/// Atende os pedidos do motor: baixar os oficiais (≤ 5 req/s) e sondar seções sem disponibilização (≤ 2 req/s).
pub async fn atender(com: Arc<Comum>, mut rx: tokio::sync::mpsc::UnboundedReceiver<Pedido>, tx: FilaTx) {
    while let Some(p) = rx.recv().await {
        match p {
            Pedido::Oficiais(lista) => {
                let com = com.clone();
                tokio::spawn(async move {
                    let dir = com.cfg.saida.join("oficial");
                    let _ = tokio::fs::create_dir_all(&dir).await;
                    let (mut baixados, mut faltando) = (vec![], vec![]);
                    for (url, nome) in lista {
                        if com.parar.load(Relaxed) { break; }
                        tokio::time::sleep(com.espera(0.2)).await;
                        com.cont.oficiais.fetch_add(1, Relaxed);
                        match com.rede.get(&url, None, None).await {
                            Ok(r) if r.status == 200 => {
                                let p = dir.join(&nome);
                                if tokio::fs::write(&p, &r.corpo).await.is_ok() { baixados.push(p); } else { faltando.push(nome); }
                            }
                            Ok(r) => faltando.push(format!("{nome} (HTTP {})", r.status)),
                            Err(e) => faltando.push(format!("{nome} ({e})")),
                        }
                    }
                    let _ = com.tx_motor.send(Msg::Oficiais { baixados, faltando });
                });
            }
            Pedido::Sondar(secs) => {
                let (com, tx) = (com.clone(), tx.clone());
                tokio::spawn(async move {
                    for s in secs {
                        if com.parar.load(Relaxed) { break; }
                        tokio::time::sleep(com.espera(0.5)).await;
                        enfileirar(&com, &tx, Tarefa { secao: s, tentativa: 0, sondagem: true });
                    }
                });
            }
            // o limitador da rede dá o ritmo (mesma fila das seções novas)
            Pedido::Recoletar(secs) => { for s in secs { enfileirar(&com, &tx, Tarefa { secao: s, tentativa: 0, sondagem: false }); } }
        }
    }
}

// ------------------------------------------------------------------ gravação da publicação (thread nativa)

/// Pedidos à thread de gravação.
pub enum Gravar {
    /// arquivos (nome, JSON) de uma publicação e a chegada dos BUs que ela inclui; `superadas`: a difusão recebe também
    /// as versões que outra mais nova do mesmo arquivo superou no mesmo lote do gravador (só na retotalização do replay)
    Lote { arquivos: Vec<(String, String)>, chegadas: Vec<Instant>, superadas: bool, t_noite: Option<i64> },
    /// seções que acenderam num lote (chave, hora da noite do BU): só para o acervo da série (`crate::serie`)
    Secoes(Vec<(u64, i64)>),
    /// responde depois que tudo o que foi pedido antes estiver gravado
    Sincronizar(std::sync::mpsc::Sender<()>),
    /// sonda (`crate::sonda`): segue para a difusão depois dos arquivos que vieram antes dela
    Sonda(Box<crate::sonda::Sonda>),
}

/// Thread de gravação: grava os arquivos da publicação (gravação atômica) fora do laço do motor, juntando as versões do
/// mesmo arquivo que se acumularam (só a mais nova é gravada), e mede BU recebido → arquivo gravado.
/// Com `acervo` (etapa 1.1), toda versão de cada lote vai também para o acervo de versões da série, antes de juntar.
fn gravador(com: Arc<Comum>, dir: PathBuf, rx: crossbeam_channel::Receiver<Gravar>, mut painel: Option<Painel>, mut acervo: Option<crate::serie::Acervo>) {
    while let Ok(primeiro) = rx.recv() {
        let mut msgs = vec![primeiro];
        while let Ok(x) = rx.try_recv() { msgs.push(x); }
        // nome → (JSON, instante em µs do BU mais recente do lote que o gerou); `Bytes` para a difusão compartilhar
        // o mesmo buffer com o arquivo, sem cópia
        let mut arquivos: HashMap<String, (Bytes, Option<u64>)> = HashMap::new();
        // para a difusão vão, na ordem, as versões das publicações da retotalização do replay (`superadas`: as que outra
        // mais nova superou ainda fecham pontos da curva da apuração, que chega em rajada) e, das demais, só a mais
        // nova de cada arquivo, como na 0.4.0: na coleta ao vivo copiar todas para a difusão pesava no p99 de BU →
        // publicação (0.5.0: 8,8–17,2 ms na 0.4.0 × 16,1–62,3 ms na 0.4.11)
        let mut versoes: Vec<(String, Bytes, Option<u64>)> = vec![];
        // arquivos cuja versão mais nova veio de uma publicação comum (entram em `versoes` no fim)
        let mut comuns: HashSet<String> = HashSet::new();
        let (mut chegadas, mut respostas, mut sondas) = (vec![], vec![], vec![]);
        let (agora_i, agora_us) = (Instant::now(), crate::difusao::agora_us());
        for m in msgs {
            match m {
                Gravar::Lote { arquivos: a, chegadas: c, superadas, t_noite } => {
                    let t_bu = c.iter().max().map(|&x| agora_us.saturating_sub((agora_i - x).as_micros() as u64));
                    for (n, j) in a {
                        if let Some(ac) = acervo.as_mut() {
                            if let Err(e) = ac.versao(t_noite, crate::relogio::agora_ms(), &n, &j) {
                                com.alerta("erro_serie", &format!("acervo de versões {}: {e}; a série desta partida para aqui", ac.caminho.display()));
                                acervo = None;
                            }
                        }
                        let b = Bytes::from(j);
                        if com.difusao.is_some() {
                            if superadas { comuns.remove(&n); versoes.push((n.clone(), b.clone(), t_bu)); } else { comuns.insert(n.clone()); }
                        }
                        arquivos.insert(n, (b, t_bu));
                    }
                    chegadas.extend(c);
                }
                Gravar::Secoes(v) => {
                    if let Some(ac) = acervo.as_mut() {
                        if let Err(e) = ac.secoes(crate::relogio::agora_ms(), &v) {
                            com.alerta("erro_serie", &format!("acervo de versões {}: {e}; a série desta partida para aqui", ac.caminho.display()));
                            acervo = None;
                        }
                    }
                }
                Gravar::Sincronizar(r) => respostas.push(r),
                Gravar::Sonda(s) => sondas.push(s),
            }
        }
        if !arquivos.is_empty() {
            // difusão antes da gravação: só entrega a outra thread (canal sem limite), nunca espera um cliente
            if let Some(h) = &com.difusao {
                for n in comuns { if let Some((b, t)) = arquivos.get(&n) { versoes.push((n, b.clone(), *t)); } }
                h.entregar(crate::difusao::Entrada::Arquivos(versoes));
            }
            let t = Instant::now();
            for (n, (j, _)) in &arquivos {
                // os bytes vieram de uma `String`
                let j = std::str::from_utf8(j).unwrap_or_default();
                if let Err(e) = gravar_atomico(&dir, n, j) { com.alerta("erro_publicacao", &format!("{n}: {e}")); }
            }
            com.custo_grav.lock().unwrap().por(t.elapsed().as_secs_f64() * 1e3);
            if let Some(p) = painel.as_mut() { p.ver(&arquivos); }
        }
        let agora = Instant::now();
        if !chegadas.is_empty() {
            let mut l = com.lat_pub.lock().unwrap();
            for c in chegadas { l.por((agora - c).as_secs_f64() * 1e3); }
        }
        // a descarga antes das respostas: quem sincroniza (conferência, saída) encontra o acervo legível até aqui
        if let Some(ac) = acervo.as_mut() {
            if let Err(e) = ac.descarregar() { com.alerta("erro_serie", &format!("acervo de versões {}: {e}", ac.caminho.display())); acervo = None; }
        }
        for r in respostas { let _ = r.send(()); }
        if !sondas.is_empty() { entregar_sondas(&com, sondas); }
    }
}

/// As sondas que passaram pela gravação: carimbo, verificador do serviço e difusão (atrás dos arquivos já entregues).
fn entregar_sondas(com: &Comum, sondas: Vec<Box<crate::sonda::Sonda>>) {
    let t = crate::difusao::agora_us();
    let mut v = com.sonda.as_ref().map(|x| x.lock().unwrap());
    let mut corpos = Vec::with_capacity(sondas.len());
    for mut s in sondas {
        s.t_gravacao = Some(t);
        if let Some(v) = v.as_mut() { v.concluir(&s, t); }
        if com.difusao.is_some() { corpos.push(s.json()); }
    }
    drop(v);
    if let Some(h) = &com.difusao { h.entregar(crate::difusao::Entrada::Sonda(corpos)); }
}

// ------------------------------------------------------------------ painel no terminal (--painel)

/// Painel de acompanhamento no terminal (`--painel [prefixo]`): a thread de gravação redesenha a abrangência cujo
/// arquivo de publicação começa com o prefixo (padrão `br-c0001`, Presidente no Brasil) a partir do JSON que acabou de
/// gravar, no máximo 4 vezes por segundo e sempre quando a publicação encerra — nada roda na thread do motor. Num
/// terminal limpa a tela e desenha a tabela; redirecionado para arquivo, escreve uma linha de resumo por quadro.
pub struct Painel {
    prefixo: String,
    nomes: HashMap<(u8, u32), String>,
    arquivo: Option<String>,
    ultimo: Option<Instant>,
    t0: Instant,
    terminal: bool,
}

/// 1234567 → "1.234.567"
fn mil(x: u64) -> String {
    let s = x.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 { out.push('.'); }
        out.push(c);
    }
    out
}

fn nome_cargo(c: u64) -> String {
    match c { 1 => "PRESIDENTE".into(), 3 => "GOVERNADOR".into(), 5 => "SENADOR".into(), 11 => "PREFEITO".into(), x => format!("CARGO {x}") }
}

impl Painel {
    pub fn novo(prefixo: &str, arq: &urna_velox::candidatos::ArquivoCandidatos) -> Painel {
        use std::io::IsTerminal;
        let mut nomes = HashMap::new();
        for e in &arq.eleicoes {
            for c in &e.cargos {
                for ci in &c.circunscricoes {
                    for k in &ci.candidatos {
                        let n = if k.nome_urna.trim().is_empty() { k.nome.clone() } else { k.nome_urna.clone() };
                        nomes.entry((c.cargo, k.numero)).or_insert(n);
                    }
                }
            }
        }
        Painel { prefixo: prefixo.to_string(), nomes, arquivo: None, ultimo: None, t0: Instant::now(), terminal: std::io::stdout().is_terminal() }
    }

    /// Arquivos (nome → JSON) de uma rodada da thread de gravação.
    fn ver(&mut self, arquivos: &HashMap<String, (Bytes, Option<u64>)>) {
        if self.arquivo.is_none() {
            self.arquivo = arquivos.keys().filter(|n| n.starts_with(&self.prefixo)).min().cloned();
        }
        let Some((j, _)) = self.arquivo.as_ref().and_then(|n| arquivos.get(n)) else { return };
        let Ok(v) = serde_json::from_slice::<Value>(j) else { return };
        let encerrada = v["encerrada"].as_bool().unwrap_or(false);
        if !encerrada && self.ultimo.is_some_and(|t| t.elapsed() < Duration::from_millis(250)) { return; }
        self.ultimo = Some(Instant::now());
        let txt = self.desenhar(&v, self.terminal);
        let mut out = std::io::stdout().lock();
        let _ = if self.terminal { write!(out, "\x1b[H\x1b[2J{txt}") } else { writeln!(out, "{txt}") };
        let _ = out.flush();
    }

    fn desenhar(&self, v: &Value, completo: bool) -> String {
        let u = |k: &str| v[k].as_u64().unwrap_or(0);
        let (ap, tot) = (v["secoes"]["apuradas"].as_u64().unwrap_or(0), v["secoes"]["total"].as_u64().unwrap_or(0));
        let pct_s = if tot > 0 { 100.0 * ap as f64 / tot as f64 } else { 0.0 };
        let cargo = u("cargo");
        let estado = v["estado"].as_str().unwrap_or("").to_uppercase();
        let enc = if v["encerrada"].as_bool().unwrap_or(false) { "   (ENCERRADA)" } else { "" };
        let cand = v["cand"].as_array().cloned().unwrap_or_default();
        let nome = |k: &Value| self.nomes.get(&(cargo as u8, k["n"].as_u64().unwrap_or(0) as u32)).cloned().unwrap_or_else(|| "?".into());
        if !completo {
            let top: Vec<String> = cand.iter().take(2).map(|k| format!("{} {} {:.2}%", k["n"], nome(k), k["pct"].as_f64().unwrap_or(0.0))).collect();
            return format!("[{:6.1} s] {} {} seq {} seções {}/{} ({:.2}%) {} | {}{enc}", self.t0.elapsed().as_secs_f64(), nome_cargo(cargo),
                v["abr"].as_str().unwrap_or("").to_uppercase(), u("seq"), ap, tot, pct_s, estado, top.join(" · "));
        }
        let mut s = format!(
            "{} · {} · eleição {} ({}º turno)        {:6.1} s\n\
             seções {}/{} ({pct_s:5.2} %)   BUs aplicados (seq) {}   último BU {}\n\
             comparecimento {} de {}   válidos {}   brancos {}   nulos {}\n\
             estado: {estado}{enc}\n\n",
            nome_cargo(cargo), v["abr"].as_str().unwrap_or("").to_uppercase(), u("eleicao"), u("turno"), self.t0.elapsed().as_secs_f64(),
            mil(ap), mil(tot), mil(u("seq")), v["ultimo_bu"].as_str().unwrap_or("—"),
            mil(u("comparecimento")), mil(u("eleitorado")), mil(u("validos")), mil(u("vb")), mil(u("vn") + u("vnt")));
        let maior = cand.iter().filter_map(|k| k["v"].as_u64()).max().unwrap_or(1).max(1);
        for k in &cand {
            let votos = k["v"].as_u64().unwrap_or(0);
            let barra = "█".repeat((40 * votos / maior) as usize);
            let mut n = nome(k);
            if n.chars().count() > 22 { n = n.chars().take(22).collect(); }
            let mat = match k["mat"].as_object() {
                Some(m) => format!("  ← matematicamente {} (seq {}, {})", m.get("estado").and_then(|x| x.as_str()).unwrap_or(""),
                    mil(m.get("seq").and_then(|x| x.as_u64()).unwrap_or(0)), m.get("em").and_then(|x| x.as_str()).map(|t| t.get(11..19).unwrap_or(t)).unwrap_or("")),
                None => String::new(),
            };
            let st = k["st"].as_str().filter(|x| !x.is_empty()).map(|x| format!("  [{x}]")).unwrap_or_default();
            s += &format!("{:>3} {n:<22} {:>12} {:6.2} % {barra}{mat}{st}\n", k["n"].as_u64().unwrap_or(0), mil(votos), k["pct"].as_f64().unwrap_or(0.0));
        }
        s
    }
}

/// Thread de sincronia: força o disco (`sync_data`) nas cópias dos descritores dos pacotes e do diário que o motor
/// manda a cada 2 s, na ordem recebida (pacotes antes do diário). O `fsync` de ~30 arquivos levava até ~200 ms na
/// thread do motor e atrasava os BUs que chegavam nesse intervalo (p99 de BU → publicação de 172 ms a 60×). Contra
/// morte do processo nada muda (os dados já estão no SO); contra falta de energia, um BU que o diário cite e que
/// não tenha chegado ao disco é pedido de novo na retomada (`retomada_bu_ausente`).
fn sincronizador(com: Arc<Comum>, rx: crossbeam_channel::Receiver<Vec<std::fs::File>>) {
    while let Ok(arqs) = rx.recv() {
        let t = Instant::now();
        for f in &arqs {
            if let Err(e) = f.sync_data() { com.alerta("erro_disco", &format!("sync_data: {e}")); }
        }
        com.custo_sync.lock().unwrap().por(t.elapsed().as_secs_f64() * 1e3);
    }
}

// ------------------------------------------------------------------ motor (thread nativa)

fn texto_secao(k: ChaveUf) -> String {
    format!("{}:{}/{}/{}", String::from_utf8_lossy(&k.0), (k.1 >> 32) as u32, (k.1 >> 16) as u16, k.1 as u16)
}

/// Limita as arenas do alocador (glibc cria até 8 por núcleo: com 28 threads de rede, memória livre presa em dezenas
/// de arenas).
fn limitar_arenas() {
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    unsafe { libc::mallopt(libc::M_ARENA_MAX, 2); }
}

/// Devolve ao sistema operacional a memória livre do alocador.
fn devolver_memoria() {
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    unsafe { libc::malloc_trim(0); }
}

/// Campo de /proc/self/status em MB.
fn status_mb(campo: &str) -> f64 {
    std::fs::read_to_string("/proc/self/status").ok()
        .and_then(|s| s.lines().find(|l| l.starts_with(campo)).and_then(|l| l.split_whitespace().nth(1)?.parse::<f64>().ok()))
        .map(|k| k / 1024.0).unwrap_or(-1.0)
}

/// Tempo de CPU do processo (s).
fn cpu_s() -> f64 {
    let Ok(s) = std::fs::read_to_string("/proc/self/stat") else { return 0.0 };
    let Some(r) = s.rsplit_once(')').map(|x| x.1) else { return 0.0 };
    let f: Vec<&str> = r.split_whitespace().collect();
    let tck = unsafe { libc::sysconf(libc::_SC_CLK_TCK) }.max(1) as f64;
    (f.get(11).and_then(|x| x.parse::<f64>().ok()).unwrap_or(0.0) + f.get(12).and_then(|x| x.parse::<f64>().ok()).unwrap_or(0.0)) / tck
}

/// Espaço livre (GB) no sistema de arquivos de `p`.
pub fn disco_livre_gb(p: &Path) -> f64 {
    use std::os::unix::ffi::OsStrExt;
    let Ok(c) = std::ffi::CString::new(p.as_os_str().as_bytes()) else { return -1.0 };
    let mut s: libc::statvfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statvfs(c.as_ptr(), &mut s) } != 0 { return -1.0; }
    s.f_bavail as f64 * s.f_frsize as f64 / 1e9
}

struct Motor<'a> {
    com: &'a Comum,
    cat: &'a Catalogo,
    cad: &'a Candidatos,
    est: &'a Estado,
    ag: Agregador<'a>,
    pubr: Publicador<'a>,
    ac: Acompanhamento<'a>,
    secoes: HashMap<ChaveUf, Situ>,
    pendentes: HashMap<ChaveUf, Pend>,
    /// seções pedidas de novo por estarem pendentes (soma das rodadas)
    repedidas: u64,
    /// seção sondada → hora do site da última sondagem
    sondadas: HashMap<ChaveUf, i64>,
    /// abrangências cuja sondagem foi recusada por passar dos limites (um alerta só por abrangência)
    sonda_recusada: HashSet<String>,
    /// calendário de chegada do exterior (B-057): do `cs.json` do turno anterior
    calendario_zz: HashMap<u64, i64>,
    esc: Escritor,
    diario: std::fs::File,
    eventos: std::fs::File,
    n_eventos: usize,
    pub_dir: PathBuf,
    /// abrangências publicadas por cargo (só as das UFs acompanhadas)
    abrs: Vec<(u8, Vec<Abrangencia>)>,
    /// abrangências publicadas (sem repetir entre cargos), cada uma com o `s.ts` pré-apuração como piso (0 = sem `s.ts`).
    /// O esperado de cada uma é o maior entre o total de seções do catálogo (listadas no `cs.json` menos as agregadas,
    /// como a página mostra; igual ao `s.ts` oficial final em 2024 e 2026) e esse piso ([`Motor::esperadas`], B-051)
    esperado: Vec<(Abrangencia, u64)>,
    /// geração das agregadas do `cs.json` já tiradas do total ([`Comum::agregadas_ger`])
    agregadas_ger_vista: u64,
    /// M-020: BUs retidos pela trava (na ordem de chegada), as seções deles, e se a trava vale (só contra o TSE de
    /// verdade, depois da retomada)
    retidos: crate::trava::Fila<ChaveUf, Box<BuRecebido>>,
    travar: bool,
    secoes_muni: Vec<Vec<u64>>,
    munis_sujos: Vec<bool>,
    ufs_sujas: Vec<bool>,
    sujo: bool,
    /// chegada dos BUs aplicados que ainda não foram publicados
    aguardando: Vec<Instant>,
    ultimo_bu_site: Option<i64>,
    /// relógio do sistema (ms, hora de Brasília) quando o último BU foi aplicado (M-003: `totalizado_em`)
    ultimo_bu_sis: Option<i64>,
    /// dentro da retotalização do replay ([`reconstruir`]): as publicações levam as versões superadas para a difusão
    retotalizando: bool,
    /// thread de gravação dos arquivos publicados
    gravacao: crossbeam_channel::Sender<Gravar>,
    /// thread de sincronia (capacidade 1: se a anterior não terminou, a sincronia fica para a próxima rodada)
    sincronia: crossbeam_channel::Sender<Vec<std::fs::File>>,
    custo_mat: Amostra,
    custo_pub: Amostra,
    /// lote inteiro na thread do motor: pacotes, estado, diário, matematicamente e eventos (ms)
    custo_lote: Amostra,
    /// rodada de 2 s da saúde na thread do motor: descarregar os pacotes e gravar a `saude.json` (ms)
    custo_saude: Amostra,
    /// verificação de 1 s (fim da apuração, sondagem, conferência periódica) na thread do motor (ms)
    custo_verif: Amostra,
    ultima_pub: Instant,
    encerrada: bool,
    conferindo: bool,
    conferencias: u32,
    tentativas_oficiais: u32,
    prox_oficiais: Option<Instant>,
    prox_conf_periodica: Option<Instant>,
    pedidos: tokio::sync::mpsc::UnboundedSender<Pedido>,
    acoes: BTreeMap<&'static str, u64>,
    saude_ant: (Instant, u64, f64),
    /// pausas de 429 já avisadas como alerta
    pausas_avisadas: u64,
    sair: bool,
    /// marcos da noite (etapa 5, `crate::marcos`) e as seções que mudaram o estado no lote em curso
    marcos: crate::marcos::Marcos,
    lote_secoes: Vec<crate::marcos::SecaoAplicada>,
    marcos_novos: Vec<Value>,
    /// marcos na thread do motor, por lote com BU (ms): primeira seção, contagem, seção decisiva
    custo_marcos: Amostra,
    marcos_ms: f64,
    /// conta periódica do `faltam_para_mat`, viradas e % das seções na verificação de 1 s (ms)
    custo_faltam: Amostra,
    rodada_verif: u64,
    /// previsão (etapa 8): o instante em que cada circunscrição ficou matematicamente decidida vai para a thread
    /// `uv-previsao`; o bloco `previsao` da saúde vem de lá
    prev_tx: Option<crossbeam_channel::Sender<crate::previsao::Definicao>>,
    prev_saude: Arc<Mutex<Value>>,
}

impl<'a> Motor<'a> {
    fn m_de(&self, k: ChaveUf) -> Option<usize> {
        self.cat.muni((k.1 >> 32) as u32).map(|m| m as usize).filter(|&m| self.cat.ufs[self.cat.muni_uf[m] as usize] == k.0)
    }
    /// O estado mudou na UF `uf` (município `m`): republicar e esquecer os totais guardados da UF.
    fn sujar(&mut self, m: Option<usize>, uf: [u8; 2]) {
        self.pubr.invalidar_uf(uf);
        if let Some(m) = m { self.munis_sujos[m] = true; }
        if let Some(u) = self.cat.ufs.iter().position(|x| *x == uf) { self.ufs_sujas[u] = true; }
        self.sujo = true;
    }
    fn seq(&self) -> u64 { self.est.bus.load(Relaxed) }
    fn acao(&mut self, a: &'static str) { *self.acoes.entry(a).or_default() += 1; }

    /// Aplica um lote de mensagens: grava os BUs novos nos pacotes, aplica ao estado, escreve o diário, reavalia o
    /// "matematicamente" e grava os eventos.
    fn lote(&mut self, msgs: Vec<Msg>) -> anyhow::Result<()> {
        if msgs.is_empty() && self.retidos.is_empty() { return Ok(()); }
        let mut bus = vec![];
        let mut outras = vec![];
        for m in msgs { match m { Msg::Bu(b) => bus.push(b), x => outras.push(x) } }
        // 1. BUs com conteúdo novo vão para os pacotes (e para o disco do SO) antes do diário que os cita
        for b in bus.iter_mut() {
            if b.pos.is_some() || matches!(self.secoes.get(&b.k), Some(Situ::Bu { sha, .. }) if *sha == b.sha) { continue; }
            let k = b.k.1;
            let reg = Registro { pleito: self.cat.pleito, uf: b.k.0, municipio: (k >> 32) as u32, zona: (k >> 16) as u16, secao: k as u16,
                situacao: b.situacao.to_lowercase().starts_with("totalizad") as u8, origem: 1, sha256_vsc: b.vsc };
            let pos = self.esc.gravar_pos(&reg, &b.bytes)?;
            b.pos = Some((Escritor::caminho(&self.com.cfg.dados, &b.k.0), pos, b.bytes.len() as u32));
        }
        if !bus.is_empty() { self.esc.descarregar(false)?; }
        // 2. estado
        let mut linhas = String::new();
        let mut em = None;
        let bus_antes = self.seq();
        let mut mudou = false;
        self.lote_secoes.clear();
        let bus = self.pela_trava(bus);
        for b in bus { if let Some(t) = self.aplicar_bu(*b, &mut linhas) { em = Some(em.map_or(t, |x: i64| x.max(t))); mudou = true; } }
        for m in outras { self.outra(m, &mut linhas); }
        if !linhas.is_empty() { self.diario.write_all(linhas.as_bytes())?; }
        // 3. matematicamente e eventos
        if mudou || self.seq() != bus_antes {
            let t = Instant::now();
            let em = em.or(self.ultimo_bu_site).unwrap_or_else(|| self.com.relogio_site.load(Relaxed));
            let n0 = self.ac.eventos.len();
            let n = self.ac.atualizar(self.est, self.seq(), em);
            self.custo_mat.por(t.elapsed().as_secs_f64() * 1e3);
            if n > 0 {
                self.gravar_eventos()?;
                // marcos: seção decisiva e `faltam = 0` no seq do evento
                let t = Instant::now();
                let v = self.marcos.decisivas(&self.ac, n0, &self.lote_secoes);
                self.marcos_novos.extend(v);
                self.marcos_ms += t.elapsed().as_secs_f64() * 1e3;
            }
        }
        if !self.lote_secoes.is_empty() { self.custo_marcos.por(std::mem::take(&mut self.marcos_ms)); }
        self.entregar_marcos();
        // mapa das seções (crate::mapa): as seções do lote acendem (as já acesas não mudam)
        if !self.lote_secoes.is_empty() {
            if let Some(h) = &self.com.difusao { h.entregar(crate::difusao::Entrada::Secoes(self.lote_secoes.iter().map(|x| (x.k.1, x.hora)).collect())); }
            // série de consulta (etapa 1.1): as seções do lote, com a hora da noite, vão para o acervo de versões
            if self.com.cfg.serie.is_some() { let _ = self.gravacao.send(Gravar::Secoes(self.lote_secoes.iter().map(|x| (x.k.1, x.hora)).collect())); }
        }
        Ok(())
    }

    /// Falha provocada para os ensaios (`docs/arquitetura/SONDA.md`): com o arquivo-sinal `<saida>/TRAVAR_MOTOR`, a thread do motor
    /// dorme até ele sumir — BUs, publicação, saúde e sondas param, o resto do processo segue vivo (é o caso que a
    /// sonda existe para pegar). Só contra o site simulado local: contra qualquer outro servidor o arquivo é ignorado.
    fn travar_se_pedido(&self) {
        // contra o site real nem olha o disco
        if !coletor::base_local(&self.com.cfg.alvo.base) { return; }
        let sinal = self.com.cfg.saida.join("TRAVAR_MOTOR");
        if !sinal.exists() { return; }
        self.com.registrar("aviso", "motor_travado", json!({"texto": "arquivo-sinal TRAVAR_MOTOR (falha provocada, só no simulado)"}));
        while sinal.exists() && !self.com.parar.load(Relaxed) { std::thread::sleep(Duration::from_millis(50)); }
        self.com.registrar("info", "motor_destravado", json!({}));
    }

    /// Sondas (`crate::sonda`) que chegaram ao motor: carimbo e seq, e seguem para a thread de gravação. Não tocam no
    /// estado, no diário, nos pacotes nem na publicação.
    fn passar_sondas(&mut self, sondas: Vec<Box<crate::sonda::Sonda>>) {
        let (t, seq) = (crate::difusao::agora_us(), self.seq());
        for mut s in sondas {
            s.t_motor = Some(t);
            s.seq = seq;
            let _ = self.gravacao.send(Gravar::Sonda(s));
        }
    }

    /// M-020: com a trava ligada, os BUs do lote entram na fila dos retidos (na ordem de chegada) e saem dela só até o
    /// teto (% do TSE + `TRAVA_PP` p.p. das seções do Brasil, Presidente). Desligada, solta tudo o que estiver retido.
    fn pela_trava(&mut self, bus: Vec<Box<BuRecebido>>) -> Vec<Box<BuRecebido>> {
        use crate::trava::TRAVA;
        let ligada = self.travar && TRAVA.ligada();
        if !ligada && self.retidos.is_empty() { return bus; }
        let limite = ligada.then(|| {
            let (tot, ap) = self.pubr.secoes(self.est, &Abrangencia::Brasil);
            crate::trava::aplicaveis(TRAVA.pp(), TRAVA.tse_pct(), tot, ap)
        });
        let sai = self.retidos.passar(bus.into_iter().map(|b| (b.k, b)).collect(), limite);
        TRAVA.retidos.store(self.retidos.len() as u64, Relaxed);
        TRAVA.soltos.fetch_add(sai.len() as u64, Relaxed);
        sai
    }

    /// M-020: relê o `TRAVA_PP` do `no.env` e a % do TSE (o `br-c0001` que a comparação busca).
    fn atualizar_trava(&mut self) {
        if !self.travar { return; }
        let p = std::env::var_os("UV_NO_ENV").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/etc/urna-velox/no.env"));
        crate::trava::TRAVA.reler(&p);
        if let Some((tot, st)) = self.com.tse.as_ref().and_then(|t| t.secoes_de(crate::trava::TOPICO)) { crate::trava::TRAVA.definir_tse(tot, st); }
    }

    /// Manda os marcos acumulados à difusão (já estão no `marcos.csv`).
    fn entregar_marcos(&mut self) {
        if self.marcos_novos.is_empty() { return; }
        let v = std::mem::take(&mut self.marcos_novos);
        if let Some(h) = &self.com.difusao { h.entregar(crate::difusao::Entrada::Marcos(v)); }
    }

    fn gravar_eventos(&mut self) -> anyhow::Result<()> {
        let mut s = String::new();
        let novos: Vec<_> = self.ac.eventos[self.n_eventos..].to_vec();
        let mut difundir = vec![];
        for e in &novos {
            let c = &self.ac.circs[e.circ];
            let (a, m, d, h, mi, se) = urna_velox::metricas::civil(e.em);
            let horario = format!("{a:04}-{m:02}-{d:02} {h:02}:{mi:02}:{se:02}");
            let _ = writeln!(s, "{},{horario},{},{},{},{}", e.seq, c.abr().codigo(), c.cargo, e.numero.map(|n| n.to_string()).unwrap_or_default(), e.estado);
            if self.com.difusao.is_some() {
                // M-003: horário do sistema em que o Urna Velox viu o evento (na retotalização do replay, o equivalente pela âncora)
                let anc = if self.retotalizando { self.com.relogio.ancora() } else { None };
                let sistema = Some(crate::relogio::iso_ms(anc.map_or_else(crate::relogio::agora_ms, |a| a.sistema_ms(e.em as f64))));
                difundir.push(crate::difusao::EventoBruto { classe: "matematicamente", seq: e.seq, horario, sistema, abrangencia: Some(c.abr().codigo()), cargo: Some(c.cargo),
                    candidato: e.numero, estado: e.estado.to_string(), motivo: if e.estado.starts_with("revogado:") { e.motivo.to_string() } else { String::new() } });
            }
            // a previsão anda pelo relógio do sistema (M-003): o instante da decisão é o do sistema, na escala da noite
            let em_sis = if self.retotalizando { None } else { self.com.relogio.noite_agora() };
            if let (Some(tx), None, true) = (&self.prev_tx, e.numero, e.estado.starts_with("matematicamente_")) { let _ = tx.send((c.cargo, c.abr().clone(), em_sis.unwrap_or(e.em))); }
            if e.estado.starts_with("revogado:") {
                self.com.alerta("revogacao", &format!("{} cargo {} candidato {}: {} — motivo: {}", c.abr().codigo(), c.cargo,
                    e.numero.map(|n| n.to_string()).unwrap_or("-".into()), e.estado, if e.motivo.is_empty() { "BUs aplicados" } else { &e.motivo }));
            }
        }
        self.n_eventos = self.ac.eventos.len();
        self.eventos.write_all(s.as_bytes())?;
        if let Some(h) = &self.com.difusao { if !difundir.is_empty() { h.entregar(crate::difusao::Entrada::Eventos(difundir)); } }
        Ok(())
    }

    /// Evento de apuração (encerrada / reaberta) para a difusão.
    fn evento_apuracao(&self, estado: &str, motivo: &str) {
        let Some(h) = &self.com.difusao else { return };
        let t = self.com.relogio_site.load(Relaxed);
        let (a, m, d, hh, mi, se) = urna_velox::metricas::civil(t);
        h.entregar(crate::difusao::Entrada::Eventos(vec![crate::difusao::EventoBruto { classe: "apuracao", seq: self.seq(),
            horario: format!("{a:04}-{m:02}-{d:02} {hh:02}:{mi:02}:{se:02}"), sistema: Some(crate::relogio::iso_ms(crate::relogio::agora_ms())), abrangencia: None, cargo: None, candidato: None, estado: estado.into(), motivo: motivo.into() }]));
    }

    /// Aplica (ou substitui, ou anula) o BU de uma seção. Devolve a hora do site se o estado mudou.
    fn aplicar_bu(&mut self, b: BuRecebido, linhas: &mut String) -> Option<i64> {
        let k = b.k;
        let seq = self.seq();
        let (arq, pos, tam) = b.pos.as_ref().map(|(p, x, n)| (p.file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default(), *x, *n)).unwrap_or_default();
        let m = self.m_de(k);
        if m.is_none() || !self.cat.secoes.contains_key(&k.1) {
            self.com.alerta("fora_catalogo", &format!("BU da seção {} fora do catálogo (candidatos.json)", texto_secao(k)));
            linhas.push_str(&linha_diario("fora_catalogo", k, &b.hash, Some(&b.sha), &b.situacao, b.anulada, &b.disp, &b.recebido, &arq, pos, tam, seq));
            self.acao("fora_catalogo");
            return None;
        }
        if urna_velox::bu::ler_bu(&b.bytes, &mut Nulo).is_err() {
            self.com.alerta("erro_bu", &format!("BU ilegível na seção {} (hash {})", texto_secao(k), b.hash));
            linhas.push_str(&linha_diario("erro_bu", k, &b.hash, Some(&b.sha), &b.situacao, b.anulada, &b.disp, &b.recebido, &arq, pos, tam, seq));
            self.acao("erro_bu");
            return None;
        }
        let atual = self.secoes.get(&k).cloned();
        let arquivo = arquivo_uf(k.0);
        let (acao, pos, tam) = match atual {
            Some(Situ::Bu { imp, sha, anulada, pos: p0, tam: t0 }) => {
                if imp == impressao(&b.hash) && anulada == b.anulada {
                    linhas.push_str(&linha_diario("confirmado", k, &b.hash, Some(&sha), &b.situacao, anulada, &b.disp, &b.recebido, &arquivo, p0, t0, seq));
                    self.acao("confirmado");
                    return None;
                }
                if sha == b.sha && anulada == b.anulada {
                    self.com.alerta("reenvio_identico", &format!("seção {}: hash novo {} com o mesmo BU (SHA-256 {}); totais inalterados", texto_secao(k), b.hash, &hex(&b.sha)[..16]));
                    linhas.push_str(&linha_diario("reenvio_identico", k, &b.hash, Some(&b.sha), &b.situacao, b.anulada, &b.disp, &b.recebido, &arquivo, p0, t0, seq));
                    self.secoes.insert(k, Situ::Bu { imp: impressao(&b.hash), sha, anulada, pos: p0, tam: t0 });
                    self.com.aplicados.lock().unwrap().insert(k, (impressao(&b.hash), b.anulada));
                    self.acao("reenvio_identico");
                    return None;
                }
                let antigo = match pacote::ler_bu_em(&self.com.cfg.dados.join(&arquivo), p0, t0) {
                    Ok(x) => x,
                    Err(e) => { self.com.alerta("erro_interno", &format!("seção {}: BU anterior ilegível no pacote ({e})", texto_secao(k))); return None; }
                };
                let (motivo, tipo, acao) = if sha == b.sha {
                    if b.anulada { ("seção anulada depois de totalizada", "anulacao", "anulado") } else { ("anulação desfeita", "anulacao_desfeita", "anulado") }
                } else {
                    ("BU reenviado com conteúdo diferente", "reenvio_diferente", "substituido")
                };
                match self.est.substituir(self.cat, k.1, (&antigo, anulada), (&b.bytes, b.anulada), &mut self.ag) {
                    Aplicado::Ok => {}
                    x => { self.com.alerta("erro_interno", &format!("seção {}: substituição {x:?}", texto_secao(k))); return None; }
                }
                self.ac.bu_alterado(m.unwrap(), &format!("{motivo} ({})", texto_secao(k)));
                self.com.alerta(tipo, &format!("seção {}: {motivo}; hash novo {} (SHA-256 {} → {})", texto_secao(k), b.hash, &hex(&sha)[..16], &hex(&b.sha)[..16]));
                if sha == b.sha { (acao, p0, t0) } else { (acao, pos, tam) }
            }
            Some(Situ::NaoInstalada) => {
                self.com.alerta("bu_em_nao_instalada", &format!("seção {} estava não instalada e recebeu BU (hash {}); BU ignorado", texto_secao(k), b.hash));
                return None;
            }
            _ => {
                match self.est.aplicar_secao(self.cat, k.1, &b.bytes, b.anulada, &mut self.ag) {
                    Aplicado::Ok => {}
                    x => {
                        self.com.alerta("erro_bu", &format!("seção {}: aplicação {x:?}", texto_secao(k)));
                        linhas.push_str(&linha_diario("erro_bu", k, &b.hash, Some(&b.sha), &b.situacao, b.anulada, &b.disp, &b.recebido, &arq, pos, tam, seq));
                        return None;
                    }
                }
                if b.anulada { self.com.alerta("secao_anulada", &format!("seção {} anulada (BU \"{}\"): votos contados como nulos", texto_secao(k), b.situacao)); }
                self.ac.bu_aplicado(m.unwrap());
                ("aplicado", pos, tam)
            }
        };
        linhas.push_str(&linha_diario(acao, k, &b.hash, Some(&b.sha), &b.situacao, b.anulada, &b.disp, &b.recebido, &arquivo, pos, tam, self.seq()));
        self.acao(acao);
        // marcos (etapa 5): a seção entra na lista do lote (seção decisiva) e, no primeiro BU, primeira seção e contagem
        let sa = crate::marcos::SecaoAplicada { k, hora: b.hora_site, seq: self.seq() };
        if acao == "aplicado" {
            let t = Instant::now();
            let v = self.marcos.aplicado(&sa);
            self.marcos_novos.extend(v);
            self.marcos_ms += t.elapsed().as_secs_f64() * 1e3;
        }
        self.lote_secoes.push(sa);
        self.secoes.insert(k, Situ::Bu { imp: impressao(&b.hash), sha: b.sha, anulada: b.anulada, pos, tam });
        self.com.aplicados.lock().unwrap().insert(k, (impressao(&b.hash), b.anulada));
        self.pendentes.remove(&k);
        self.sujar(m, k.0);
        self.reabrir(&format!("BU {acao} na seção {}", texto_secao(k)));
        self.aguardando.push(b.chegada);
        self.ultimo_bu_site = Some(self.ultimo_bu_site.map_or(b.hora_site, |x| x.max(b.hora_site)));
        // na retotalização do replay (voltar no tempo) tudo é aplicado de uma vez: o horário do sistema de cada BU é o
        // equivalente do horário dele pela âncora (fixada no alvo da retotalização)
        let anc = if self.retotalizando { self.com.relogio.ancora() } else { None };
        self.ultimo_bu_sis = Some(anc.map_or_else(crate::relogio::agora_ms, |a| a.sistema_ms(b.hora_site as f64)));
        Some(b.hora_site)
    }

    fn outra(&mut self, m: Msg, linhas: &mut String) {
        let seq = self.seq();
        match m {
            Msg::NaoInstalada { k, disp } => match self.secoes.get(&k) {
                None | Some(Situ::Agregada) => {
                    self.pubr.nao_instalada(k.0, k.1);
                    self.secoes.insert(k, Situ::NaoInstalada);
                    self.pendentes.remove(&k);
                    linhas.push_str(&linha_diario("nao_instalada", k, "", None, "Nao instalada", false, &disp, "", "", 0, 0, seq));
                    self.acao("nao_instalada");
                    let mm = self.m_de(k);
                    self.sujar(mm, k.0);
                }
                Some(Situ::NaoInstalada) => {}
                Some(Situ::Bu { .. }) => self.com.alerta("nao_instalada_com_bu", &format!("seção {} com BU aplicado aparece como não instalada; mantido o BU", texto_secao(k))),
            },
            // a rede só manda Confirmada quando a disponibilização no cs.json mudou e o BU que vale é o mesmo: o diário
            // guarda a disponibilização nova, para a retomada não pedir a seção outra vez
            Msg::Confirmada { k, disp } => {
                if matches!(self.secoes.get(&k), Some(Situ::Bu { .. })) {
                    linhas.push_str(&linha_diario("confirmado", k, "", None, "", false, &disp, "", "", 0, 0, seq));
                    self.acao("confirmado");
                }
            }
            Msg::Pendente { k, tipo, motivo, disp } => {
                self.com.alerta(&tipo, &format!("seção {}: {motivo}", texto_secao(k)));
                if !matches!(self.secoes.get(&k), Some(Situ::Bu { .. } | Situ::NaoInstalada)) {
                    // mantém a contagem de rodadas se ela já estava pendente (a nova tentativa falhou de novo)
                    let prox = Instant::now() + self.com.espera(PEND_BASE_S);
                    let p = self.pendentes.entry(k).or_insert(Pend { motivo: String::new(), disp: String::new(), rodadas: 0, prox });
                    p.motivo = motivo.clone();
                    if !disp.trim().is_empty() { p.disp = disp; }
                }
                linhas.push_str(&linha_diario("pendente", k, "", None, &motivo, false, "", "", "", 0, 0, seq));
                self.acao("pendente");
            }
            Msg::Agregada { k } => {
                if !self.secoes.contains_key(&k) {
                    self.secoes.insert(k, Situ::Agregada);
                    linhas.push_str(&linha_diario("agregada", k, "", None, "", false, "", "", "", 0, 0, seq));
                    self.acao("agregada");
                    // fora do total de seções (B-051): senão o fim da apuração esperaria por ela
                    if let Some(m) = self.agregada(k.0, k.1) { self.sujar(Some(m), k.0); }
                }
            }
            Msg::Oficiais { .. } | Msg::Parar | Msg::Bu(_) | Msg::Sonda(_) => {}
        }
    }

    /// Publica as abrangências que receberam BU desde a publicação anterior (todas, com `tudo`): gera os JSON aqui e
    /// entrega à thread de gravação.
    fn publicar(&mut self, tudo: bool) -> anyhow::Result<()> {
        if !self.sujo && !tudo { return Ok(()); }
        let t = Instant::now();
        let mo = Momento { encerrada: self.encerrada, seq: self.seq(), ultimo_bu: self.ultimo_bu_site, totalizado_ms: self.ultimo_bu_sis };
        if self.encerrada { self.pubr.limpar_cache(); }
        let mut arquivos = vec![];
        for (cargo, abrs) in &self.abrs {
            for abr in abrs {
                let tocada = tudo || match abr {
                    Abrangencia::Brasil => true,
                    Abrangencia::Uf(u) => self.cat.ufs.iter().position(|x| x == u).is_some_and(|i| self.ufs_sujas[i]),
                    Abrangencia::Municipio(u, mu) => self.cat.muni(*mu).is_some_and(|m| self.munis_sujos[m as usize] && self.cat.ufs[self.cat.muni_uf[m as usize] as usize] == *u),
                };
                if !tocada { continue; }
                if let Some(j) = self.pubr.json(self.est, Some(&self.ac), abr, *cargo, mo) { arquivos.push((self.pubr.nome_arquivo(abr, *cargo), j)); }
            }
        }
        self.munis_sujos.iter_mut().for_each(|x| *x = false);
        self.ufs_sujas.iter_mut().for_each(|x| *x = false);
        self.sujo = false;
        if !arquivos.is_empty() { self.custo_pub.por(t.elapsed().as_secs_f64() * 1e3); }
        let chegadas = std::mem::take(&mut self.aguardando);
        if !arquivos.is_empty() || !chegadas.is_empty() {
            self.gravacao.send(Gravar::Lote { arquivos, chegadas, superadas: self.retotalizando, t_noite: self.ultimo_bu_site }).map_err(|_| anyhow::anyhow!("a thread de gravação parou"))?;
        }
        self.ultima_pub = Instant::now();
        Ok(())
    }

    /// Espera a thread de gravação gravar tudo o que já foi publicado (antes de conferir a pasta ou de sair).
    fn sincronizar(&self) -> anyhow::Result<()> {
        let (tx, rx) = std::sync::mpsc::channel();
        self.gravacao.send(Gravar::Sincronizar(tx)).map_err(|_| anyhow::anyhow!("a thread de gravação parou"))?;
        rx.recv_timeout(Duration::from_secs(120)).map_err(|_| anyhow::anyhow!("a thread de gravação não respondeu"))
    }

    fn munis_de(&self, abr: &Abrangencia) -> Vec<usize> {
        let cat = self.cat;
        match abr {
            Abrangencia::Brasil => (0..cat.muni_cod.len()).collect(),
            Abrangencia::Uf(u) => (0..cat.muni_cod.len()).filter(|&m| cat.ufs[cat.muni_uf[m] as usize] == *u).collect(),
            Abrangencia::Municipio(u, mu) => cat.muni(*mu).map(|m| m as usize).filter(|&m| cat.ufs[cat.muni_uf[m] as usize] == *u).into_iter().collect(),
        }
    }

    /// Seções esperadas numa abrangência: o total do catálogo (listadas no `cs.json` menos as agregadas — as do
    /// `candidatos.json` e as descobertas na noite —, mais as não instaladas fora dele; o mesmo número da página) ou o
    /// `s.ts` pré-apuração, o que for maior. Até a 0.9.1 valia só o `s.ts`, que pode vir desatualizado para baixo (no
    /// 3221 em 23/10: SP 33.403 contra 103.904 no `cs.json`) e encerrava a apuração cedo (B-051).
    fn esperadas(&self, abr: &Abrangencia, piso: u64) -> u64 { self.pubr.secoes(self.est, abr).0.max(piso) }

    /// A abrangência tem todas as seções esperadas totalizadas (BUs + não instaladas).
    fn completa(&self, abr: &Abrangencia, piso: u64) -> bool { self.pubr.secoes(self.est, abr).1 >= self.esperadas(abr, piso) }

    /// Seção do catálogo que se revelou agregada na noite: sai do total de seções da página ([`Publicador::agregada`])
    /// e do mapa das seções (B-064). Devolve o município quando o total mudou.
    fn agregada(&mut self, uf: [u8; 2], k: u64) -> Option<usize> {
        let r = self.pubr.agregada(uf, k);
        if r.is_some() { if let Some(m) = self.com.difusao.as_ref().and_then(|h| h.mapa()) { m.retirar(k); } }
        r
    }

    /// Agregadas novas no `cs.json` lido na noite (`nsp` que o `candidatos.json` não tinha): fora do total de seções.
    fn tirar_agregadas_do_cs(&mut self) {
        let g = self.com.agregadas_ger.load(Relaxed);
        if g == self.agregadas_ger_vista { return; }
        self.agregadas_ger_vista = g;
        let mut novas = vec![];
        for (uf, ag) in self.com.agregadas_cs.lock().unwrap().iter() {
            let Ok(u) = <[u8; 2]>::try_from(uf.as_bytes()) else { continue };
            for &k in ag { novas.push((u, k)); }
        }
        let mut n = 0u64;
        for (u, k) in novas { if let Some(m) = self.agregada(u, k) { self.sujar(Some(m), u); n += 1; } }
        if n > 0 { self.com.registrar("info", "agregadas_do_cs", json!({"secoes": n, "texto": "seções agregadas no cs.json que o candidatos.json não tinha: fora do total de seções"})); }
    }

    /// Fim da apuração: todas as abrangências com as seções esperadas ([`Motor::esperadas`]) totalizadas, nenhuma seção
    /// na fila nem pendente, e toda seção com disponibilização no último `cs.json` resolvida. Se chegar BU novo depois,
    /// a publicação encerrada é reaberta (ver `aplicar_bu`), e contra o site real o serviço continua acompanhando
    /// depois da conferência.
    fn concluida(&self) -> bool {
        if self.esperado.is_empty() || self.com.fila.load(Relaxed) > 0 || !self.pendentes.is_empty() || !self.retidos.is_empty() { return false; }
        let publicadas: u64 = self.com.publicadas_cs.lock().unwrap().values().sum();
        let resolvidas = self.secoes.values().filter(|s| matches!(s, Situ::Bu { .. } | Situ::NaoInstalada)).count() as u64;
        resolvidas >= publicadas && self.esperado.iter().all(|(abr, n)| self.completa(abr, *n))
    }

    /// O estado mudou depois da apuração encerrada (BU atrasado, reenvio, anulação): a publicação volta a "apurando"
    /// até o próximo fim, com alerta.
    fn reabrir(&mut self, motivo: &str) {
        if !self.encerrada { return; }
        self.encerrada = false;
        self.pubr.limpar_cache();
        self.pubr.invalidar_tudo();
        // todas as abrangências voltam a "apurando" na próxima publicação
        self.munis_sujos.iter_mut().for_each(|x| *x = true);
        self.ufs_sujas.iter_mut().for_each(|x| *x = true);
        self.sujo = true;
        self.com.alerta("apuracao_reaberta", &format!("{motivo} depois da apuração encerrada: publicação reaberta"));
        self.evento_apuracao("apuracao_reaberta", motivo);
    }

    fn encerrar(&mut self) -> anyhow::Result<()> {
        self.encerrada = true;
        // a publicação final é calculada do estado, sem nenhum total guardado
        self.pubr.invalidar_tudo();
        self.publicar(true)?;
        self.sincronizar()?;
        self.evento_apuracao("apuracao_encerrada", "");
        let v = self.marcos.encerrada();
        self.marcos_novos.extend(v);
        self.entregar_marcos();
        let n: usize = self.abrs.iter().map(|x| x.1.len()).sum();
        self.com.registrar("info", "apuracao_encerrada", json!({"seq": self.seq(), "publicacoes": n, "relogio_site": crate::texto_hora(self.com.relogio_site.load(Relaxed)),
            "ultimo_bu": self.ultimo_bu_site.map(crate::texto_hora)}));
        eprintln!("uv-servico: apuração encerrada ({} BUs); conferindo com os oficiais", self.seq());
        self.pedir_oficiais("fim da apuração");
        Ok(())
    }

    fn pedir_oficiais(&mut self, motivo: &str) {
        if self.conferindo { return; }
        // sem coleta (ensaio da comparação contra o site real): a conferência baixaria todos os -u.json do pleito, o que
        // não está no combinado com o Luciano (só os da tela, CLAUDE.md); fica de fora
        if self.com.cfg.sem_coleta {
            self.com.registrar("info", "conferencia_sem_coleta", json!({"motivo": motivo, "texto": "sem conferência: --sem-coleta não baixa os -u.json do pleito"}));
            self.conferindo = true;
            return;
        }
        let a = &self.com.cfg.alvo;
        let mut lista = vec![];
        for (cargo, abrs) in &self.abrs {
            for abr in abrs {
                let Some((e, _)) = self.cad.eleicao_de(*cargo, abr) else { continue };
                let nome = format!("{}-c{cargo:04}-e{e:06}-u.json", abr.codigo());
                let dir = match abr { Abrangencia::Brasil => "br".to_string(), Abrangencia::Uf(u) | Abrangencia::Municipio(u, _) => String::from_utf8_lossy(u).into_owned() };
                lista.push((format!("{}/{}/{}/{e}/dados/{dir}/{nome}", a.base, a.ambiente, a.ciclo), nome));
            }
        }
        self.com.registrar("info", "conferencia_pedida", json!({"motivo": motivo, "arquivos": lista.len()}));
        self.conferindo = true;
        let _ = self.pedidos.send(Pedido::Oficiais(lista));
    }
}

impl<'a> Motor<'a> {
    /// Confere com os `-u.json` baixados: a publicação encerrada gravada (no fim) ou calculada agora (conferência
    /// pedida no meio da noite); e a antecedência do "matematicamente" e da apuração completa em relação ao TSE.
    fn conferir(&mut self, baixados: Vec<PathBuf>, faltando: Vec<String>) -> anyhow::Result<()> {
        use urna_velox::conferencia::{conferir_pasta_publicacoes, conferir_publicacoes};
        self.conferindo = false;
        self.conferencias += 1;
        let ofs: Vec<Oficial> = baixados.iter().filter_map(|p| Oficial::ler(p).ok()).collect();
        let mo = Momento { encerrada: true, seq: self.seq(), ultimo_bu: self.ultimo_bu_site, totalizado_ms: self.ultimo_bu_sis };
        let r = if self.encerrada {
            self.sincronizar()?;
            conferir_pasta_publicacoes(&self.pub_dir, &ofs)
        } else {
            conferir_publicacoes(&self.pubr, self.est, &ofs, mo)
        };
        // oficiais ainda abaixo de 100 % das seções (o TSE divulga minutos depois do Urna Velox: no fim da noite o
        // arquivo dele pode ainda estar a caminho) — a conferência do fim é refeita até eles fecharem
        let incompletos: Vec<String> = ofs.iter().filter(|o| o.secoes_totalizadas < o.secoes)
            .map(|o| o.arquivo.file_name().map(|x| x.to_string_lossy().into_owned()).unwrap_or_default()).collect();
        // dia da eleição: o mais comum nas datas de totalização dos oficiais
        let mut dias: BTreeMap<&str, usize> = BTreeMap::new();
        for o in &ofs { *dias.entry(o.data.as_str()).or_default() += 1; }
        let dia = dias.iter().max_by_key(|x| x.1).map(|x| x.0.to_string()).unwrap_or_default();
        let ants = urna_velox::noite::antecedencias(&self.ac, &ofs, &dia, 0.0);
        let (mut am, mut af) = (vec![], vec![]);
        let mut nao_confere = 0;
        for a in &ants {
            let c = &self.ac.circs[a.circ];
            if !a.retotalizado {
                if let Some(x) = a.tse_brasilia.zip(a.mat).map(|(t, m)| t as f64 - m) { am.push(x); }
                if let Some(x) = a.tse_brasilia.zip(a.fim).map(|(t, f)| t as f64 - f) { af.push(x); }
            }
            if !a.confere && c.mat_circ != urna_velox::matematica::MatCirc::Apurando { nao_confere += 1; }
        }
        let est_mat = urna_velox::noite::estatistica(&am);
        let est_fim = urna_velox::noite::estatistica(&af);
        let sit: (usize, usize) = r.situacoes.values().fold((0, 0), |a, x| (a.0 + x.0, a.1 + x.1));
        let j = json!({
            "momento": iso_agora(), "relogio_site": crate::texto_hora(self.com.relogio_site.load(Relaxed)), "encerrada": self.encerrada, "seq": self.seq(),
            "arquivos": r.arquivos, "identicos": r.identicos, "divergentes": r.divergentes.iter().map(|d| json!({"arquivo": d.arquivo, "cargo": d.cargo, "difs": d.difs})).collect::<Vec<_>>(),
            "situacoes": {"identicas": sit.0, "divergentes": sit.1}, "nao_finalizados": r.nao_finalizados, "faltando": faltando, "incompletos": incompletos,
            "matematicamente": {"circunscricoes": self.ac.circs.len(), "decididas": self.ac.circs.iter().filter(|c| c.mat_circ != urna_velox::matematica::MatCirc::Apurando).count(),
                "revogacoes": self.ac.revogacoes(), "nao_confere": nao_confere, "dia": dia,
                "antecedencia_mat_s": serde_json::from_str::<Value>(&est_mat).unwrap_or(Value::Null),
                "antecedencia_fim_s": serde_json::from_str::<Value>(&est_fim).unwrap_or(Value::Null)},
        });
        gravar_atomico(&self.com.cfg.saida, "conferencia.json", &(serde_json::to_string_pretty(&j)? + "\n"))?;
        let mut md = format!("# Conferência com os arquivos oficiais\n\n{} · relógio do site {} · {} BUs · publicação {}\n\n",
            iso_agora(), crate::texto_hora(self.com.relogio_site.load(Relaxed)), self.seq(), if self.encerrada { "encerrada (arquivos gravados)" } else { "calculada agora (apuração em curso)" });
        let _ = writeln!(md, "* {}", r.resumo());
        if !r.nao_finalizados.is_empty() { let _ = writeln!(md, "* totalização oficial não finalizada: {}", r.nao_finalizados.join(", ")); }
        if !faltando.is_empty() { let _ = writeln!(md, "* oficiais não obtidos ({}): {}", faltando.len(), faltando.join(", ")); }
        if !incompletos.is_empty() { let _ = writeln!(md, "* oficiais ainda abaixo de 100 % das seções ({}): {}", incompletos.len(), incompletos.iter().take(20).cloned().collect::<Vec<_>>().join(", ")); }
        let _ = writeln!(md, "* matematicamente: {} circunscrições, {} decididas, {} revogações, {nao_confere} diferentes do oficial",
            self.ac.circs.len(), j["matematicamente"]["decididas"], self.ac.revogacoes());
        let _ = writeln!(md, "* antecedência sobre a totalização do TSE (s, sem retotalizados; dia {dia}): matematicamente {est_mat}; apuração completa {est_fim}");
        if !r.divergentes.is_empty() { let _ = write!(md, "\n## Divergências\n\n{}", r.detalhes(50)); }
        gravar_atomico(&self.com.cfg.saida, "conferencia.md", &md)?;
        self.com.registrar("info", "conferencia", json!({"arquivos": r.arquivos, "identicos": r.identicos, "divergentes": r.divergentes.len(),
            "situacoes_identicas": sit.0, "situacoes_divergentes": sit.1, "faltando": faltando.len(), "antecedencia_mat_s": est_mat}));
        eprintln!("uv-servico: conferência: {}", r.resumo());
        // nova tentativa em 60 s (hora do site): até 10 com oficiais ausentes; até 60 (1 h) com oficiais ainda abaixo de
        // 100 % das seções (o TSE fechando depois do Urna Velox)
        let esperar = !faltando.is_empty() || !incompletos.is_empty();
        let limite = if faltando.is_empty() { 60 } else { 10 };
        // divergência enquanto o TSE ainda não fechou não é alerta (a conferência vai ser refeita)
        let aguardando_tse = self.encerrada && faltando.is_empty() && !incompletos.is_empty() && self.tentativas_oficiais < limite;
        if !r.divergentes.is_empty() && !aguardando_tse { self.com.alerta("conferencia_divergente", &format!("{} de {} arquivos divergem do oficial (ver conferencia.md)", r.divergentes.len(), r.arquivos)); }
        if !faltando.is_empty() { self.com.alerta("oficiais_ausentes", &format!("{} arquivos oficiais não obtidos", faltando.len())); }
        if esperar && self.encerrada && self.tentativas_oficiais < limite {
            self.tentativas_oficiais += 1;
            self.prox_oficiais = Some(Instant::now() + self.com.espera(60.0));
        }
        if self.encerrada && (!esperar || self.tentativas_oficiais >= limite) && !self.com.cfg.continuar { self.sair = true; }
        Ok(())
    }

    /// Verificações de segundo a segundo: fim da apuração, arquivo-sinal CONFERIR, conferência periódica, nova
    /// tentativa dos oficiais e sondagem das seções sem disponibilização.
    fn verificar(&mut self) -> anyhow::Result<()> {
        self.tirar_agregadas_do_cs();
        if !self.encerrada && self.concluida() { self.encerrar()?; }
        let sinal = self.com.cfg.saida.join("CONFERIR");
        if sinal.exists() {
            let _ = std::fs::remove_file(&sinal);
            self.pedir_oficiais("arquivo-sinal CONFERIR");
        }
        self.travar_se_pedido();
        if let Some(t) = self.prox_conf_periodica {
            if Instant::now() >= t {
                self.prox_conf_periodica = Some(t + Duration::from_secs_f64(self.com.cfg.conferir_a_cada_min * 60.0));
                self.pedir_oficiais("conferência periódica");
            }
        }
        if let Some(t) = self.prox_oficiais { if Instant::now() >= t { self.prox_oficiais = None; self.pedir_oficiais("nova tentativa dos oficiais"); } }
        self.repedir_pendentes();
        self.sondar();
        // marcos: eleitores que faltam para o "matematicamente" (no máximo 1×/s por circunscrição). A conta roda na thread
        // uv-faltam (B-009): aqui só a fotografia das circunscrições que mudaram e as respostas da rodada anterior; o
        // orçamento só vale sem a thread (testes de unidade)
        let t = Instant::now();
        let agora = self.ultimo_bu_site.unwrap_or_else(|| self.com.relogio_site.load(Relaxed));
        let v = self.marcos.faltam(&self.ac, self.seq(), agora, Duration::from_micros(500));
        self.marcos_novos.extend(v);
        // viradas de liderança (1×/s) e, a cada 5 s, os marcos de % das seções (Brasil e UFs ainda não completas)
        let v = self.marcos.viradas(&self.ac, self.seq(), agora);
        self.marcos_novos.extend(v);
        self.rodada_verif += 1;
        if self.rodada_verif % 5 == 0 {
            let mut abrs: Vec<&Abrangencia> = self.abrs.iter().flat_map(|x| x.1.iter())
                .filter(|a| matches!(a, Abrangencia::Brasil | Abrangencia::Uf(_)) && !self.marcos.pct_completo(&a.codigo())).collect();
            abrs.sort();
            abrs.dedup();
            let pcts: Vec<(String, u64, u64)> = abrs.into_iter().map(|a| { let (t, ap) = self.pubr.secoes(self.est, a); (a.codigo(), t, ap) }).collect();
            let v = self.marcos.pct_secoes(&pcts, self.seq(), agora);
            self.marcos_novos.extend(v);
        }
        self.custo_faltam.por(t.elapsed().as_secs_f64() * 1e3);
        self.entregar_marcos();
        Ok(())
    }

    /// Pede de novo as seções publicadas que ficaram pendentes ([`Pend`]), com espera crescente.
    fn repedir_pendentes(&mut self) {
        if self.pendentes.is_empty() || self.com.parar.load(Relaxed) { return; }
        let (com, agora) = (self.com, Instant::now());
        let mut lista = vec![];
        let mut desistiu = vec![];
        for (k, p) in self.pendentes.iter_mut() {
            if p.disp.trim().is_empty() || p.rodadas >= PEND_RODADAS || agora < p.prox { continue; }
            p.rodadas += 1;
            p.prox = agora + com.espera(PEND_BASE_S * (1u32 << p.rodadas.min(4)) as f64);
            if p.rodadas == PEND_RODADAS { desistiu.push(*k); }
            lista.push(Secao { uf: k.0, municipio: (k.1 >> 32) as u32, zona: (k.1 >> 16) as u16, secao: k.1 as u16, disponivel: p.disp.clone() });
        }
        for k in desistiu {
            com.alerta("pendente_sem_solucao", &format!("seção {}: pedida {PEND_RODADAS} vezes sem resolver ({}); não é pedida de novo até a disponibilização mudar ou uma retomada",
                texto_secao(k), self.pendentes[&k].motivo));
        }
        if lista.is_empty() { return; }
        self.repedidas += lista.len() as u64;
        com.registrar("info", "repedir_pendentes", json!({"secoes": lista.len(), "pendentes": self.pendentes.len()}));
        let _ = self.pedidos.send(Pedido::Recoletar(lista));
    }

    /// Seções não instaladas não têm disponibilização no `cs.json` (2026: 41 no exterior) mas contam em `s.st` como
    /// totalizadas. Regra (Luciano, 16/10/2026): enquanto o TSE não disser que a seção não foi instalada, ela continua
    /// esperada; quando o `aux.json` dela disser "Nao instalada", sai das que faltam e conta como totalizada, como no
    /// oficial. Isso pode acontecer a qualquer hora da noite, até o fim da totalização.
    ///
    /// Por UF: quando toda seção com disponibilização no `cs.json` da UF já está resolvida e faltam poucas para o
    /// `s.ts` (exterior: 5 % ou 50; Brasil: [`SONDA_FALTAM_UF`]), pede o `aux.json` das seções da UF ainda sem BU,
    /// fora as agregadas (`nsp` no `cs.json`, que não têm `aux.json`), cada uma de novo a cada [`SONDA_REPETIR_S`]
    /// (hora do site), no máximo [`SONDA_POR_RODADA`] por segundo e [`SONDA_ABR`] seções por UF. "Nao instalada"
    /// conta; 404 ou sem BU, a seção continua esperada. Sem `nsp` no `cs.json` da UF (arquivo antigo), o comportamento
    /// de antes: só o exterior, uma vez cada, 404 = agregada. Antes da eleição (nada apurado) não sonda nada.
    ///
    /// Exterior antes do `cs.json` (B-056, 25/10/2026): o TSE põe no ar o `aux.json` e o BU das seções do exterior
    /// horas antes de pôr a disponibilização no `cs.json` (3221: Wellington recebido às 01:51:38, `cs.json` ainda de
    /// 09/10 às 03:07). Num domingo (relógio do site), com o `cs.json` do ZZ (com `nsp`) lido — desde a 1.0.3 (B-065)
    /// também depois que ele passa a listar seções, porque anda em lotes atrás dos `aux.json` (708 às 15h46 de 25/10,
    /// Londres no ar desde 15:47) —, as seções do ZZ sem BU e fora da fila do `cs.json` são sondadas já, nas mesmas regras (fora as agregadas, ≤ [`SONDA_POR_RODADA`] por
    /// segundo, cada uma a cada [`SONDA_REPETIR_S`]), primeiro as dos municípios que já têm BU. São as duas exceções
    /// à regra "seção sem disponibilização não gera requisição"; `--sem-sondagem` desliga as duas.
    fn sondar(&mut self) {
        if !self.com.cfg.sondar || self.encerrada { return; }
        let algo_apurado = self.ultimo_bu_site.is_some();
        let com = self.com;
        let agora = com.relogio_site.load(Relaxed);
        let publicadas = com.publicadas_cs.lock().unwrap().clone();
        let agregadas = com.agregadas_cs.lock().unwrap();
        let vazio = HashSet::new();
        // (prioridade, última sondagem, seção): município com BU primeiro, depois as nunca sondadas, depois as mais antigas
        let mut lista: Vec<(u8, i64, ChaveUf)> = vec![];
        let mut antecipada = false;
        // contingência (B-057): depois de qualquer 429 o reforço do exterior acaba (volta a 2 req/s e 300 s por seção)
        let tomou_429 = com.rede.http429.load(Relaxed) > 0;
        if tomou_429 && self.sonda_recusada.insert("reforco_429".into()) {
            com.alerta("reforco_exterior_desligado", "HTTP 429 do TSE: o reforço da sondagem do exterior (8 req/s, 20 s por seção) fica desligado até reiniciar o serviço");
        }
        for (abr, n) in &self.esperado {
            let Abrangencia::Uf(u) = abr else { continue };
            let n = self.esperadas(abr, *n);
            let uf_s = String::from_utf8_lossy(u).into_owned();
            let ag = agregadas.get(&uf_s).unwrap_or(&vazio);
            let com_nsp = !ag.is_empty();
            let zz = u == b"zz";
            let ap = self.pubr.secoes(self.est, abr).1;
            let pub_uf = publicadas.get(&uf_s).copied();
            let quando = quando_sondar(zz, com_nsp, n, ap, pub_uf, algo_apurado, domingo(agora));
            if quando == Sonda::Nao { continue; }
            antecipada |= quando == Sonda::Antecipada;
            let mut resolvidas = 0u64;
            // (município com BU primeiro, seção): na antecipada, as seções do mesmo posto saem juntas
            let mut v = vec![];
            for m in self.munis_de(abr) {
                let mut com_bu = false;
                let ini = v.len();
                for &k in &self.secoes_muni[m] {
                    let ku = (*u, k);
                    match self.secoes.get(&ku) {
                        Some(Situ::Bu { .. }) => { resolvidas += 1; com_bu = true; }
                        Some(Situ::NaoInstalada) => resolvidas += 1,
                        Some(Situ::Agregada) => {}
                        None => if !ag.contains(&k) && !self.pendentes.contains_key(&ku) && !self.retidos.contem(&ku) { v.push((1u8, ku)); },
                    }
                }
                if com_bu { for x in &mut v[ini..] { x.0 = 0; } }
            }
            // fora da antecipada: toda seção com disponibilização da UF já resolvida (as que faltam são as sem
            // disponibilização)
            if quando == Sonda::SeResolvidas && resolvidas < pub_uf.unwrap_or(u64::MAX) { continue; }
            if v.is_empty() { continue; }
            if v.len() > SONDA_ABR {
                if self.sonda_recusada.insert(abr.codigo()) {
                    com.alerta("sondagem_limitada", &format!("{}: {} seções sem disponibilização; sondagem recusada (limite de {SONDA_ABR} por UF) — confira as não instaladas pelo oficial", abr.codigo(), v.len()));
                }
                continue;
            }
            for (prio, ku) in v {
                // exterior na antecipada, sem 429: faixa e repetição pelo calendário do turno anterior (B-057)
                let (f, rep) = if quando == Sonda::Antecipada && zz && !tomou_429 {
                    faixa_secao(self.calendario_zz.get(&ku.1).copied(), agora, prio == 0)
                } else { (0, SONDA_REPETIR_S) };
                let prio = f * 2 + prio;
                match self.sondadas.get(&ku) {
                    None => lista.push((prio, i64::MIN, ku)),
                    Some(&t) if com_nsp && devida(Some(t), agora, rep) => lista.push((prio, t, ku)),
                    _ => {}
                }
            }
        }
        drop(agregadas);
        // B-060: só as seções deste nó disputam o orçamento; o reforço reserva uma cota para as sondadas há mais tempo
        let lista = so_do_no(lista, |ku| match &com.federado {
            Some(f) => f.minha(&Secao { uf: ku.0, municipio: (ku.1 >> 32) as u32, zona: (ku.1 >> 16) as u16, secao: ku.1 as u16, disponivel: String::new() }),
            None => true,
        });
        if lista.is_empty() { return; }
        let candidatas = lista.len();
        let nunca = lista.iter().filter(|c| c.1 == i64::MIN).count();
        let por_rodada = sondas_por_rodada(antecipada, tomou_429, com.fila.load(Relaxed));
        let cota = if por_rodada > SONDA_POR_RODADA { (por_rodada / 4).max(1) } else { 0 };
        let mut pedido = vec![];
        for ku in escolher_sondas(lista, por_rodada, cota) {
            self.sondadas.insert(ku, agora);
            pedido.push(Secao { uf: ku.0, municipio: (ku.1 >> 32) as u32, zona: (ku.1 >> 16) as u16, secao: ku.1 as u16, disponivel: String::new() });
        }
        com.registrar("info", "sondagem", json!({"secoes": pedido.len(), "sondadas": self.sondadas.len(), "candidatas": candidatas, "nunca": nunca}));
        let _ = self.pedidos.send(Pedido::Sondar(pedido));
    }

    fn saude(&mut self, estado: &str) -> anyhow::Result<()> {
        let c = &self.com;
        let agora = Instant::now();
        let reqs = c.rede.reqs.load(Relaxed);
        let cpu = cpu_s();
        let (t_ant, r_ant, cpu_ant) = self.saude_ant;
        let dt = (agora - t_ant).as_secs_f64().max(1e-3);
        let up = c.t0.elapsed().as_secs_f64().max(1e-3);
        self.saude_ant = (agora, reqs, cpu);
        // por UF: [publicadas no cs.json, com BU, não instaladas, anuladas]; chave em bytes (sem uma String por seção)
        let mut por_uf: BTreeMap<[u8; 2], [u64; 4]> = BTreeMap::new();
        let mut agregadas = 0usize;
        for (k, s) in &self.secoes {
            match s {
                Situ::Bu { anulada, .. } => { let e = por_uf.entry(k.0).or_default(); e[1] += 1; if *anulada { e[3] += 1; } }
                Situ::NaoInstalada => por_uf.entry(k.0).or_default()[2] += 1,
                Situ::Agregada => agregadas += 1,
            }
        }
        let pubs = c.publicadas_cs.lock().unwrap().clone();
        for (uf, p) in &pubs { if let Ok(b) = <[u8; 2]>::try_from(uf.as_bytes()) { por_uf.entry(b).or_default()[0] = *p; } }
        let mut uf_json = serde_json::Map::new();
        for (uf, v) in &por_uf {
            uf_json.insert(String::from_utf8_lossy(uf).into_owned(), json!({"publicadas_cs": v[0], "aplicadas": v[1], "nao_instaladas": v[2], "anuladas": v[3]}));
        }
        let pausa = c.rede.lim.pausa_restante();
        let pausas = c.rede.pausas.load(Relaxed);
        if pausas > self.pausas_avisadas {
            self.pausas_avisadas = pausas;
            c.alerta("http429", &format!("HTTP 429 do servidor ({} no total): pausa global de {:.0} s, nenhuma requisição até {}", c.rede.http429.load(Relaxed),
                pausa.as_secs_f64(), crate::texto_hora(crate::agora_brasilia() + pausa.as_secs() as i64)));
        }
        let p = |a: &Mutex<Amostra>| a.lock().unwrap().json();
        let alertas = c.alertas.lock().unwrap().clone();
        let j = json!({
            "versao": urna_velox::VERSAO, "estado": estado, "agora": iso_agora(), "ativo_s": arred(up),
            "pleito": c.cfg.alvo.pleito, "base": c.cfg.alvo.base, "relogio_site": crate::texto_hora(c.relogio_site.load(Relaxed)),
            "secoes": {"catalogo": self.cat.secoes.len(), "publicadas_cs": pubs.values().sum::<u64>(), "aplicadas": self.seq(),
                "nao_instaladas": por_uf.values().map(|v| v[2]).sum::<u64>(), "anuladas": por_uf.values().map(|v| v[3]).sum::<u64>(),
                "pendentes": self.pendentes.len(), "repedidas": self.repedidas, "agregadas": agregadas, "sondadas": self.sondadas.len()},
            "por_uf": uf_json, "fila": c.fila.load(Relaxed), "ritmo_cs": if c.cs_ocioso.load(Relaxed) { "ocioso" } else { "trabalhando" },
            "rede": {"requisicoes": reqs, "req_s_atual": arred((reqs - r_ant) as f64 / dt), "req_s_media": arred(reqs as f64 / up), "teto_req_s": c.rede.teto(), "acesso": c.cfg.acesso.texto(),
                "http429": c.rede.http429.load(Relaxed), "pausas_429": c.rede.pausas.load(Relaxed), "pausa_restante_s": arred(pausa.as_secs_f64()),
                "retomada_em": (!pausa.is_zero()).then(|| crate::texto_hora(crate::agora_brasilia() + pausa.as_secs() as i64)),
                "nao_modificado_304": c.rede.nao_modificado.load(Relaxed), "novas_tentativas": c.rede.tentativas_extras.load(Relaxed),
                "cs": c.cont.cs_lidos.load(Relaxed), "cs_304": c.cont.cs_304.load(Relaxed), "cs_erros": c.cont.cs_erros.load(Relaxed),
                "aux": c.cont.aux.load(Relaxed), "aux_404": c.cont.aux_404.load(Relaxed), "bu_baixados": c.cont.bu_baixados.load(Relaxed),
                "bu_do_pacote": c.cont.bu_do_pacote.load(Relaxed), "confirmadas": c.cont.confirmadas.load(Relaxed), "sondadas": c.cont.sondadas.load(Relaxed),
                "oficiais": c.cont.oficiais.load(Relaxed), "erros": c.cont.erros_rede.load(Relaxed), "mb": arred(c.rede.bytes.load(Relaxed) as f64 / 1e6)},
            "latencias_ms": {"aux": p(&c.lat_aux), "bu": p(&c.lat_bu), "bu_publicacao": p(&c.lat_pub)},
            "custos_ms": {"matematicamente": self.custo_mat.json(), "publicacao": self.custo_pub.json(), "gravacao": p(&c.custo_grav),
                "lote": self.custo_lote.json(), "saude": self.custo_saude.json(), "verificacao": self.custo_verif.json(), "sincronia_disco": p(&c.custo_sync),
                "marcos": self.custo_marcos.json(), "faltam": self.custo_faltam.json()},
            "ultimo_bu": {"seq": self.seq(), "site": self.ultimo_bu_site.map(crate::texto_hora), "sistema": self.ultimo_bu_sis.map(crate::relogio::iso_ms)},
            "trava": {"pp": crate::trava::TRAVA.pp(), "ligada": self.travar && crate::trava::TRAVA.ligada(), "tse_pct": (crate::trava::TRAVA.tse_pct() * 100.0).round() / 100.0,
                "retidos": self.retidos.len(), "soltos": crate::trava::TRAVA.soltos.load(Relaxed)},
            "relogio_sistema": c.relogio.json(),
            "acoes": self.acoes, "alertas": alertas,
            "motor": {"bus": self.est.bus.load(Relaxed), "duplicados": self.est.duplicados.load(Relaxed), "fora_catalogo": self.est.fora_catalogo.load(Relaxed),
                "erros": self.est.erros.load(Relaxed), "cargos_invalidos": self.est.cargos_invalidos.load(Relaxed), "votaveis_desconhecidos": self.est.desconhecidos.load(Relaxed),
                "identificacao_divergente": self.est.divergentes.load(Relaxed), "checksum": self.est.checksum()},
            "matematicamente": {"eventos": self.ac.eventos.len(), "revogacoes": self.ac.revogacoes(),
                "decididas": self.ac.circs.iter().filter(|x| x.mat_circ != urna_velox::matematica::MatCirc::Apurando).count(), "circunscricoes": self.ac.circs.len()},
            "apuracao": {"encerrada": self.encerrada, "conferencias": self.conferencias,
                "abrangencias_completas": self.esperado.iter().filter(|(a, n)| self.completa(a, *n)).count(), "abrangencias_esperadas": self.esperado.len()},
            "recursos": {"rss_mb": status_mb("VmRSS"), "rss_anon_mb": status_mb("RssAnon"), "pico_rss_mb": status_mb("VmHWM"),
                "cpu_pct_atual": arred(100.0 * (cpu - cpu_ant) / dt), "cpu_pct_medio": arred(100.0 * cpu / up), "disco_livre_gb": arred(disco_livre_gb(&c.cfg.dados))},
        });
        let mut j = j;
        if let Some(f) = &c.federado { j["federacao"] = f.saude_json(); }
        if let Some(t) = &c.tse { j["tse_resultados"] = t.saude_json(c.relogio_site.load(Relaxed)); }
        if self.prev_tx.is_some() { j["previsao"] = self.prev_saude.lock().unwrap().clone(); }
        if let Some(s) = &c.cfg.simulacao { j["simulacao"] = json!({"rotulo": s}); }
        if let Some(v) = &c.sonda { j["sonda"] = v.lock().unwrap().json(crate::difusao::agora_us()); }
        // caminhos do site fora do padrão (--rotas): o painel dos nós mostra quais estão valendo
        if !c.cfg.alvo.rotas.padrao() { j["rotas"] = serde_json::to_value(&*c.cfg.alvo.rotas).unwrap_or_default(); }
        if let Some(h) = &c.difusao {
            // calculado pela thread de difusão (até 2 s atrás): os percentis não rodam aqui, na thread do motor
            j["difusao"] = h.saude_guardada();
            // subconjunto estável para a mensagem `estado` e o GET /api/v1/estado (docs/arquitetura/PROTOCOLO_WS.md §4.3)
            let mut e = json!({
                "versao": urna_velox::VERSAO, "estado": estado, "encerrada": self.encerrada, "pleito": c.cfg.alvo.pleito, "relogio_site": j["relogio_site"],
                "relogio_sistema": j["relogio_sistema"],
                "secoes": {"catalogo": self.cat.secoes.len(), "publicadas_cs": j["secoes"]["publicadas_cs"], "aplicadas": self.seq(),
                    "nao_instaladas": j["secoes"]["nao_instaladas"], "pendentes": self.pendentes.len()},
                "ultimo_bu": j["ultimo_bu"], "pausa_429": {"ativa": !pausa.is_zero(), "retomada_em": j["rede"]["retomada_em"], "pausas": pausas},
                "conferencias": self.conferencias, "alertas": j["alertas"], "trava": j["trava"],
                // atraso BU recebido → publicado (a telemetria da página, ao lado do atraso da sonda; ms, janela recente)
                "latencia_bu_ms": {"p50": j["latencias_ms"]["bu_publicacao"]["p50"], "p99": j["latencias_ms"]["bu_publicacao"]["p99"],
                    "n": j["latencias_ms"]["bu_publicacao"]["n"]}});
            // só com --controle-replay (docs/arquitetura/PROTOCOLO_WS.md §4.3)
            if let Some(r) = h.replay_json() { e["replay"] = r; }
            if let Some(s) = &c.cfg.simulacao { e["simulacao"] = json!({"rotulo": s}); }
            h.entregar(crate::difusao::Entrada::Estado(e));
        }
        gravar_atomico(&c.cfg.saida, "saude.json", &(serde_json::to_string_pretty(&j)? + "\n"))?;
        Ok(())
    }
}

/// Retomada: aplica o BU que vale em cada seção segundo o diário (lido dos pacotes), marca as não instaladas, e
/// guarda os BUs órfãos dos pacotes (gravados antes de uma queda, sem linha no diário) como cache.
fn retomar(m: &mut Motor, sit: HashMap<ChaveUf, Situ>, disp: &mut HashMap<ChaveUf, i64>) -> anyhow::Result<(usize, usize, usize)> {
    let dados = m.com.cfg.dados.clone();
    let mut idx: HashMap<(String, u64), (Registro, u32)> = HashMap::new();
    if let Ok(rd) = std::fs::read_dir(&dados) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().is_none_or(|x| x != "uvpk") { continue; }
            let nome = p.file_name().unwrap().to_string_lossy().into_owned();
            for (r, pos, tam) in pacote::indice(&p)? { idx.insert((nome.clone(), pos), (r, tam)); }
        }
    }
    let (mut aplicados, mut perdidos) = (0, 0);
    let mut usados = HashSet::new();
    for (k, s) in sit {
        match &s {
            Situ::Bu { imp, anulada, pos, tam, .. } => {
                let arquivo = arquivo_uf(k.0);
                let ok = idx.get(&(arquivo.clone(), *pos)).is_some_and(|(r, t)| (r.uf, r.chave()) == k && t == tam);
                let bytes = if ok { pacote::ler_bu_em(&dados.join(&arquivo), *pos, *tam).ok() } else { None };
                let Some(bytes) = bytes else {
                    m.com.alerta("retomada_bu_ausente", &format!("seção {}: BU do diário não está no pacote {arquivo}@{pos}; será pedido de novo", texto_secao(k)));
                    disp.remove(&k);
                    perdidos += 1;
                    continue;
                };
                if m.est.aplicar_secao(m.cat, k.1, &bytes, *anulada, &mut m.ag) != Aplicado::Ok {
                    m.com.alerta("retomada_bu_invalido", &format!("seção {}: BU do diário não aplicável", texto_secao(k)));
                    disp.remove(&k);
                    perdidos += 1;
                    continue;
                }
                usados.insert((arquivo, *pos));
                m.com.aplicados.lock().unwrap().insert(k, (*imp, *anulada));
                if let Some(mi) = m.m_de(k) { m.ac.bu_aplicado(mi); }
                aplicados += 1;
            }
            Situ::NaoInstalada => m.pubr.nao_instalada(k.0, k.1),
            Situ::Agregada => { m.agregada(k.0, k.1); }
        }
        m.secoes.insert(k, s);
    }
    // sem coleta (ensaio da comparação contra o site real a partir do acervo): nenhum aux.json vai validar os BUs dos
    // pacotes, e os do acervo já são os que valem — aplica o único BU de cada seção sem linha no diário (seção com mais
    // de um BU diferente fica de fora, com alerta). As não instaladas não se sabem sem
    // coleta: a coluna do Urna Velox fica com os BUs (no 3220, 499.207 de 499.248).
    if m.com.cfg.sem_coleta {
        let mut por_secao: HashMap<ChaveUf, Vec<(String, u64)>> = HashMap::new();
        for ((arq, pos), (r, _)) in &idx {
            if usados.contains(&(arq.clone(), *pos)) || m.secoes.contains_key(&(r.uf, r.chave())) { continue; }
            por_secao.entry((r.uf, r.chave())).or_default().push((arq.clone(), *pos));
        }
        let (mut adotados, mut ambiguos) = (0usize, 0usize);
        for (k, mut v) in por_secao {
            // "Totalizado" (situação 1) primeiro; os BUs dos dados abertos (sem aux.json: situação 0) valem quando são
            // os únicos da seção — no acervo do 3220, 2.677 seções
            if v.iter().any(|(a, p)| idx[&(a.clone(), *p)].0.situacao == 1) { v.retain(|(a, p)| idx[&(a.clone(), *p)].0.situacao == 1); }
            // o mesmo BU gravado mais de uma vez no pacote (mesmo SHA-256 do arquivo) conta como um só
            v.sort_by_key(|(a, p)| (idx[&(a.clone(), *p)].0.sha256_vsc, a.clone(), *p));
            v.dedup_by_key(|(a, p)| idx[&(a.clone(), *p)].0.sha256_vsc);
            if v.len() != 1 { ambiguos += 1; continue; }
            let (arq, pos) = v.into_iter().next().unwrap();
            let (sha, tam) = { let (r, t) = &idx[&(arq.clone(), pos)]; (r.sha256_vsc, *t) };
            let Ok(bytes) = pacote::ler_bu_em(&dados.join(&arq), pos, tam) else { ambiguos += 1; continue };
            if m.est.aplicar_secao(m.cat, k.1, &bytes, false, &mut m.ag) != Aplicado::Ok { ambiguos += 1; continue; }
            usados.insert((arq, pos));
            m.com.aplicados.lock().unwrap().insert(k, (0, false));
            if let Some(mi) = m.m_de(k) { m.ac.bu_aplicado(mi); }
            m.secoes.insert(k, Situ::Bu { imp: 0, sha, anulada: false, pos, tam });
            adotados += 1;
        }
        aplicados += adotados;
        m.com.registrar("info", "sem_coleta_pacotes", json!({"aplicados": adotados, "fora": ambiguos}));
        if ambiguos > 0 {
            m.com.alerta("sem_coleta_pacotes", &format!("{ambiguos} seções com mais de um BU nos pacotes (ou ilegível): fora da publicação sem coleta"));
        }
    }
    let mut cache = m.com.cache.lock().unwrap();
    let mut orfaos = 0;
    for ((arq, pos), (r, tam)) in idx {
        if usados.contains(&(arq.clone(), pos)) { continue; }
        cache.entry((r.uf, r.chave())).or_default().push((r.sha256_vsc, dados.join(&arq), pos, tam));
        orfaos += 1;
    }
    Ok((aplicados, perdidos, orfaos))
}

/// Retotalização local do replay: reaplica, do diário-mestre ([`crate::controle_replay::DIARIO_MESTRE`]) e dos BUs que
/// já estão nos pacotes de `--dados`, todos os BUs com disponibilização até `alvo`, em ordem cronológica e em blocos de
/// [`crate::controle_replay::BLOCO_S`] s da noite (cada bloco é um lote: eventos e marcos com o horário do fim do bloco).
/// As seções reaplicadas entram em `disp` (o acompanhamento dos `cs.json` não as pede de novo).
fn reconstruir(m: &mut Motor, alvo: i64, disp: &mut HashMap<ChaveUf, i64>) -> anyhow::Result<()> {
    let t0 = Instant::now();
    let dados = m.com.cfg.dados.clone();
    let mestre = dados.join(crate::controle_replay::DIARIO_MESTRE);
    let txt = std::fs::read_to_string(&mestre).unwrap_or_default();
    // primeiro só os metadados, em ordem cronológica; os bytes são lidos bloco a bloco (a noite inteira tem ~6 GB de BUs)
    struct Item { td: i64, seq: u64, k: ChaveUf, hash: String, sha: [u8; 32], situacao: String, anulada: bool, disp: String, recebido: String, arq: usize, pos: u64, tam: u32 }
    let mut arquivos: Vec<String> = vec![];
    let mut v: Vec<Item> = vec![];
    for l in txt.lines() {
        let f: Vec<&str> = l.split(',').collect();
        if f.len() != 16 || !matches!(f[1], "aplicado" | "substituido" | "anulado" | "reenvio_identico") { continue; }
        let td = t_disp(f[10]);
        if td < 0 || td > alvo { continue; }
        let (Ok(uf), Ok(mu), Ok(zn), Ok(se), Some(sha), Ok(pos), Ok(tam)) = (<[u8; 2]>::try_from(f[2].as_bytes()), f[3].parse::<u32>(), f[4].parse::<u16>(),
            f[5].parse::<u16>(), de_hex(f[7]), f[13].parse::<u64>(), f[14].parse::<u32>()) else { continue };
        if f[12] != arquivo_uf(uf) { continue; }
        let arq = match arquivos.iter().position(|a| a == f[12]) { Some(i) => i, None => { arquivos.push(f[12].to_string()); arquivos.len() - 1 } };
        v.push(Item { td, seq: f[15].parse().unwrap_or(0), k: (uf, tse::chave(mu, zn, se)), hash: f[6].to_string(), sha, situacao: f[8].to_string(), anulada: f[9] == "1",
            disp: f[10].to_string(), recebido: f[11].to_string(), arq, pos, tam });
    }
    v.sort_by_key(|x| (x.td, x.seq));
    let mut mapas = vec![];
    for a in &arquivos {
        let f = std::fs::File::open(dados.join(a))?;
        // SAFETY: os pacotes só crescem (escrita no fim); a região lida já estava gravada
        mapas.push(unsafe { memmap2::Mmap::map(&f)? });
    }
    let n = v.len();
    let bloco = crate::controle_replay::BLOCO_S;
    let mut lote: Vec<Msg> = vec![];
    let mut fim_bloco = i64::MIN;
    let mut faltaram = 0;
    // publica a cada 2 min da noite: a curva da apuração de cada tópico (difusão) ganha os pontos do trecho retotalizado
    let mut ult_pub = i64::MIN;
    // M-003: o alvo da retotalização é "agora" no relógio do sistema; o trecho retotalizado fica antes dele, na velocidade
    m.com.relogio.ver_site(alvo);
    m.retotalizando = true;
    for it in v {
        if it.td > fim_bloco {
            if !lote.is_empty() {
                m.lote(std::mem::take(&mut lote))?;
                // saturating: com ult_pub = i64::MIN a subtração estoura (em release dá negativo e nunca publicava)
                if fim_bloco.saturating_sub(ult_pub) >= 120 { m.publicar(false)?; ult_pub = fim_bloco; }
            }
            fim_bloco = it.td - it.td.rem_euclid(bloco) + bloco - 1;
        }
        let mp = &mapas[it.arq];
        let (i, j) = (it.pos as usize, it.pos as usize + it.tam as usize);
        if j > mp.len() { faltaram += 1; continue; }
        disp.insert(it.k, it.td);
        lote.push(Msg::Bu(Box::new(BuRecebido { k: it.k, vsc: tse::sha256_do_hash_dir(&it.hash), hash: it.hash, sha: it.sha, situacao: it.situacao, anulada: it.anulada,
            disp: it.disp, recebido: it.recebido, hora_site: it.td, bytes: Bytes::copy_from_slice(&mp[i..j]),
            pos: Some((dados.join(&arquivos[it.arq]), it.pos, it.tam)), chegada: Instant::now() })));
    }
    if !lote.is_empty() { m.lote(lote)?; }
    // as latências BU → publicação são da coleta ao vivo, não da retotalização
    m.aguardando.clear();
    // a última publicação da retotalização também leva as versões superadas (antes de voltar à coleta ao vivo)
    m.publicar(false)?;
    m.retotalizando = false;
    if faltaram > 0 { m.com.alerta("replay_bu_ausente", &format!("retotalização: {faltaram} BUs do diário-mestre fora dos pacotes; serão pedidos de novo")); }
    m.com.relogio_site.fetch_max(alvo, Relaxed);
    m.com.registrar("info", "replay_retotalizacao", json!({"alvo": crate::texto_hora(alvo), "bus": n, "seq": m.seq(), "eventos": m.ac.eventos.len(), "s": arred(t0.elapsed().as_secs_f64())}));
    eprintln!("uv-servico: retotalização até {}: {n} BUs em {:.1} s", crate::texto_hora(alvo), t0.elapsed().as_secs_f64());
    Ok(())
}

type Pronto = std::sync::mpsc::Sender<Result<(HashMap<ChaveUf, i64>, Vec<String>), String>>;

/// Corpo da thread do motor: carrega os candidatos, retoma o que o diário diz, publica a frio e entra no laço.
fn rodar_motor(com: Arc<Comum>, rx: crossbeam_channel::Receiver<Msg>, pedidos: tokio::sync::mpsc::UnboundedSender<Pedido>, pronto: &mut Option<Pronto>) -> anyhow::Result<()> {
    let cfg = com.cfg.clone();
    let t0 = Instant::now();
    let (cad, arq) = Candidatos::ler(&cfg.candidatos)?;
    anyhow::ensure!(arq.pleito == cfg.alvo.pleito, "candidatos.json do pleito {}, serviço do pleito {}", arq.pleito, cfg.alvo.pleito);
    if !arq.incompleto.is_empty() {
        anyhow::ensure!(cfg.aceitar_incompleto, "candidatos.json incompleto ({}): rode de novo `uv-coletor preparar` ou use --aceitar-incompleto", arq.incompleto.join("; "));
        com.alerta("candidatos_incompleto", &format!("candidatos.json incompleto, aceito por --aceitar-incompleto: {}", arq.incompleto.join("; ")));
    }
    if arq.destinacao_presumida {
        com.alerta("destinacao_presumida", "destinação dos candidatos presumida \"Válido\" (arquivos sem dvt): confira cassações e indeferimentos antes da noite");
    }
    let cat = Catalogo::de_arquivo_candidatos(&arq)?;
    let ufs: Vec<String> = cfg.ufs.clone().unwrap_or_else(|| arq.ufs.clone());
    for u in &ufs { anyhow::ensure!(arq.ufs.contains(u), "UF {u} não está no candidatos.json"); }
    let livre = disco_livre_gb(&cfg.dados);
    if (0.0..5.0).contains(&livre) { com.alerta("disco", &format!("só {livre:.1} GB livres em {} (os BUs de uma eleição geral somam ~6 GB)", cfg.dados.display())); }
    let est = Estado::novo(&cat);
    let ufs_b: Vec<[u8; 2]> = ufs.iter().filter_map(|u| u.as_bytes().try_into().ok()).collect();
    let filtro = cfg.ufs.is_some();
    let dentro = |a: &Abrangencia| match a { Abrangencia::Brasil => !filtro, Abrangencia::Uf(u) | Abrangencia::Municipio(u, _) => ufs_b.contains(u) };
    let mut pubr = Publicador::novo(&cat, Some(&cad), None);
    // totais por UF guardados entre publicações; `Motor::sujar` invalida a UF a cada BU aplicado/retirado
    pubr.ativar_cache_uf();
    let abrs: Vec<(u8, Vec<Abrangencia>)> = pubr.cargos().into_iter().map(|c| (c, pubr.abrangencias(c, false).into_iter().filter(|a| dentro(a)).collect())).collect();
    // comparação com o TSE: o -u.json de cada tópico publicado (o mesmo endereço da conferência)
    if let Some(t) = &com.tse {
        let a = &com.cfg.alvo;
        let (mut urls, mut eleitorado) = (HashMap::new(), HashMap::new());
        for (cargo, v) in &abrs {
            for abr in v {
                let Some((e, _)) = cad.eleicao_de(*cargo, abr) else { continue };
                if let Some(&x) = cad.eleitorado_abr.get(&(*cargo, abr.clone())) { eleitorado.insert(format!("{}-c{cargo:04}", abr.codigo()), x); }
                let nome = format!("{}-c{cargo:04}-e{e:06}-u.json", abr.codigo());
                let dir = match abr { Abrangencia::Brasil => "br".to_string(), Abrangencia::Uf(u) | Abrangencia::Municipio(u, _) => String::from_utf8_lossy(u).into_owned() };
                urls.insert(format!("{}-c{cargo:04}", abr.codigo()), (format!("{}/{}/{}/{e}/dados/{dir}/{nome}", a.base, a.ambiente, a.ciclo), nome));
            }
        }
        t.definir_urls(urls);
        t.definir_eleitorado(eleitorado);
    }
    // B-051: todas as abrangências publicadas entram no fim da apuração, com o total de seções do catálogo; o s.ts
    // pré-apuração (quando houver) fica como piso
    let mut esperado: BTreeMap<Abrangencia, u64> = BTreeMap::new();
    for (c, v) in &abrs {
        for a in v {
            let e = esperado.entry(a.clone()).or_default();
            if let Some(&n) = cad.secoes_abr.get(&(*c, a.clone())) { *e = (*e).max(n); }
        }
    }
    let mut secoes_muni = vec![vec![]; cat.muni_cod.len()];
    for &k in cat.secoes.keys() { if let Some(m) = cat.muni((k >> 32) as u32) { secoes_muni[m as usize].push(k); } }
    for v in secoes_muni.iter_mut() { v.sort_unstable(); }
    // s.ts pré-apuração × catálogo (listadas no cs.json menos as agregadas do candidatos.json; em 2024 e 2026 igual ao
    // s.ts oficial final em todas as abrangências). s.ts menor (desatualizado: 3221 em 23/10, SP 33.403 × 103.904) não
    // muda mais nada — o fim usa o catálogo — e fica só registrado. s.ts maior ainda segura o fim da abrangência à
    // espera de seções que o cs.json não lista: esse é o alerta.
    {
        let total_muni: Vec<u64> = secoes_muni.iter().map(|v| v.iter().filter(|k| !cad.agregadas.contains(k)).count() as u64).collect();
        let total = |a: &Abrangencia| -> u64 {
            match a {
                Abrangencia::Brasil => total_muni.iter().sum(),
                Abrangencia::Uf(u) => (0..total_muni.len()).filter(|&m| cat.ufs[cat.muni_uf[m] as usize] == *u).map(|m| total_muni[m]).sum(),
                Abrangencia::Municipio(u, mu) => cat.muni(*mu).map(|m| m as usize).filter(|&m| cat.ufs[cat.muni_uf[m] as usize] == *u).map_or(0, |m| total_muni[m]),
            }
        };
        let (mut acima, mut abaixo) = (vec![], vec![]);
        for (a, &n) in &esperado {
            if n == 0 { continue; }
            let t = total(a);
            if n > t { acima.push(format!("{}: s.ts {n} > {t} no catálogo", a.codigo())); }
            else if n < t { abaixo.push(format!("{}: s.ts {n} < {t}", a.codigo())); }
        }
        if !abaixo.is_empty() {
            com.registrar("info", "sts_desatualizado", json!({"abrangencias": abaixo.len(), "exemplos": abaixo.iter().take(10).collect::<Vec<_>>(),
                "texto": "s.ts pré-apuração do candidatos.json abaixo do catálogo: o fim da apuração usa o catálogo (B-051)"}));
        }
        if !acima.is_empty() {
            com.alerta("secoes_esperadas_incoerentes", &format!("s.ts pré-apuração do candidatos.json acima das seções do catálogo: o fim automático dessas abrangências espera por seções que o cs.json não lista (regere o candidatos.json): {}",
                acima.iter().take(30).cloned().collect::<Vec<_>>().join("; ")));
        }
    }
    let pub_dir = cfg.saida.join("publicacao");
    std::fs::create_dir_all(&pub_dir)?;
    let diario_p = cfg.saida.join("secoes.log");
    let novo = !diario_p.exists();
    let (sit, mut disp, n_linhas) = ler_diario(&diario_p)?;
    let mut diario = std::fs::OpenOptions::new().create(true).append(true).open(&diario_p)?;
    if novo { writeln!(diario, "{CAB_DIARIO}")?; }
    let ev_p = cfg.saida.join("eventos.csv");
    let novo = !ev_p.exists();
    let mut eventos = std::fs::OpenOptions::new().create(true).append(true).open(&ev_p)?;
    if novo { writeln!(eventos, "seq,horario,abrangencia,cargo,candidato,estado")?; }
    let nm = cat.muni_cod.len();
    let nu = cat.ufs.len();
    let (grav_tx, grav_rx) = crossbeam_channel::unbounded();
    let grav_h = {
        let (c, d) = (com.clone(), pub_dir.clone());
        let painel = cfg.painel.as_deref().map(|p| Painel::novo(p, &arq));
        // acervo de versões da série de consulta (etapa 1.1): um arquivo novo por partida
        let acervo = match &cfg.serie {
            Some(p) => match crate::serie::Acervo::abrir(p) {
                Ok(a) => { com.registrar("info", "serie", json!({"acervo": a.caminho.display().to_string()})); Some(a) }
                Err(e) => { com.alerta("erro_serie", &format!("acervo de versões em {}: {e}; sem série nesta partida", p.display())); None }
            },
            None => None,
        };
        std::thread::Builder::new().name("uv-gravacao".into()).spawn(move || gravador(c, d, grav_rx, painel, acervo))?
    };
    let (sinc_tx, sinc_rx) = crossbeam_channel::bounded(1);
    let sinc_h = {
        let c = com.clone();
        std::thread::Builder::new().name("uv-sincronia".into()).spawn(move || sincronizador(c, sinc_rx))?
    };
    let ac = Acompanhamento::novo(&cat, &cad, None);
    // marcos (etapa 5): nomes dos municípios para os marcos, municípios que são circunscrição, histórico do marcos.csv
    let nomes_mun: HashMap<u32, String> = arq.municipios.iter().filter(|x| !x.nome.is_empty()).map(|x| (x.codigo, x.nome.clone())).collect();
    let munis_circ: HashSet<([u8; 2], u32)> = ac.circs.iter().filter_map(|c| match c.abr() { Abrangencia::Municipio(u, mu) => Some((*u, *mu)), _ => None }).collect();
    let mut marcos = crate::marcos::Marcos::abrir(Some(&cfg.saida), nomes_mun, munis_circ, ac.circs.len())?;
    // B-009: a conta do faltam_para_mat na thread uv-faltam (fora da thread do motor, sem orçamento por rodada)
    marcos.contar_em_paralelo(&ac)?;
    if let Some(h) = &com.difusao {
        h.definir_nomes(crate::difusao::montar_nomes(&arq));
        // mapa das seções (crate::mapa): um índice para cada seção do catálogo
        h.definir_mapa(crate::mapa::Mapa::novo(crate::mapa::chaves_do_catalogo(&cat, &cad)));
    }
    // previsão (etapa 8): todos os tópicos publicados; roda na thread `uv-previsao` (abaixo, junto com o laço do motor)
    let (prev_tx, prev_rx) = crossbeam_channel::unbounded::<crate::previsao::Definicao>();
    let prev_saude: Arc<Mutex<Value>> = Arc::default();
    let previsor = (cfg.previsao.intervalo_s > 0).then(|| {
        let topicos: Vec<(u8, Abrangencia)> = abrs.iter().flat_map(|(c, v)| v.iter().map(move |a| (*c, a.clone()))).collect();
        urna_velox::previsao::Previsor::novo(&cat, &cad, &topicos, &cad.agregadas, urna_velox::previsao::Gabaritos::embutidos(), None,
            urna_velox::previsao::Parametros::default())
    });
    let mut m = Motor {
        com: &com, cat: &cat, cad: &cad, est: &est, ag: Agregador::novo(&cat, &est), pubr, ac,
        secoes: HashMap::new(), pendentes: HashMap::new(), repedidas: 0, sondadas: HashMap::new(), sonda_recusada: HashSet::new(), calendario_zz: carregar_calendario_exterior(&cfg), esc: Escritor::novo(&cfg.dados)?, diario, eventos, n_eventos: 0,
        pub_dir, abrs, esperado: esperado.into_iter().collect(), agregadas_ger_vista: 0, secoes_muni, munis_sujos: vec![false; nm], ufs_sujas: vec![false; nu], sujo: false,
        retidos: Default::default(), travar: false,
        aguardando: vec![], ultimo_bu_site: None, ultimo_bu_sis: None, retotalizando: false, gravacao: grav_tx, sincronia: sinc_tx, custo_mat: Amostra::default(), custo_pub: Amostra::default(),
        custo_lote: Amostra::default(), custo_saude: Amostra::default(), custo_verif: Amostra::default(),
        ultima_pub: Instant::now(), encerrada: false, conferindo: false, conferencias: 0, tentativas_oficiais: 0, prox_oficiais: None,
        prox_conf_periodica: (cfg.conferir_a_cada_min > 0.0).then(|| Instant::now() + Duration::from_secs_f64(cfg.conferir_a_cada_min * 60.0)),
        pedidos, acoes: BTreeMap::new(), saude_ant: (Instant::now(), 0, 0.0), pausas_avisadas: 0, sair: false,
        marcos, lote_secoes: vec![], marcos_novos: vec![], custo_marcos: Amostra::com_janela(4096), marcos_ms: 0.0, custo_faltam: Amostra::com_janela(1024), rodada_verif: 0,
        prev_tx: previsor.is_some().then_some(prev_tx), prev_saude: prev_saude.clone(),
    };
    // histórico dos marcos (retomada): vai para a difusão antes de a camada HTTP aceitar conexões
    if let Some(h) = &com.difusao {
        let mut hist = std::mem::take(&mut m.marcos.historico);
        // M-003: o horário do sistema em que foram vistos não ficou gravado (a difusão não carimba a hora da retomada)
        for v in hist.iter_mut() { v["sistema"] = Value::Null; }
        if !hist.is_empty() { h.entregar(crate::difusao::Entrada::Marcos(hist)); }
    }
    let t1 = Instant::now();
    let (aplicados, perdidos, orfaos) = retomar(&mut m, sit, &mut disp)?;
    // B-058: o horário do último BU (estado, resultados, aviso M-019 da página) vem do diário, sem esperar o próximo BU
    if m.ultimo_bu_site.is_none() { m.ultimo_bu_site = ultimo_bu_do_diario(&m.secoes, &disp); }
    // M-020: a trava vale depois da retomada (o diário entra inteiro), só contra o site do TSE e com a comparação ligada
    // (é dela a busca do br-c0001); a inscrição permanente faz a busca rodar sempre (combinado com o Luciano em 25/10)
    m.travar = crate::trava::vale(&cfg.alvo.base, cfg.controle_replay, com.tse.is_some());
    let _insc_trava = if m.travar { com.tse.as_ref().and_then(|t| t.inscrever(crate::trava::TOPICO).ok()) } else { None };
    // mapa das seções: as do diário acendem de uma vez, com o horário da disponibilização
    if let Some(h) = &com.difusao {
        let agora = Some(com.relogio_site.load(Relaxed)).filter(|&t| t > 0).unwrap_or_else(crate::agora_brasilia);
        let v: Vec<(u64, i64)> = m.secoes.iter().filter(|(_, x)| matches!(x, Situ::Bu { .. }))
            .map(|(k, _)| (k.1, disp.get(k).copied().filter(|&t| t > 0).unwrap_or(agora))).collect();
        if !v.is_empty() { h.entregar(crate::difusao::Entrada::Secoes(v)); }
    }
    // replay (--controle-replay): ir a um instante já visto = retotalizar do disco até ele (crate::controle_replay)
    if cfg.controle_replay {
        if let Some(alvo) = crate::controle_replay::reconstrucao_pendente() { reconstruir(&mut m, alvo, &mut disp)?; }
    }
    if aplicados > 0 {
        writeln!(m.eventos, "# retomada {}: {aplicados} BUs do diário", iso_agora())?;
        let em = com.relogio_site.load(Relaxed);
        m.ac.tudo_sujo();
        m.ac.atualizar(&est, m.seq(), if em > 0 { em } else { crate::agora_brasilia() });
        m.gravar_eventos()?;
    }
    m.publicar(true)?;
    let n_pub: usize = m.abrs.iter().map(|x| x.1.len()).sum();
    com.registrar("info", "partida", json!({"pleito": cat.pleito, "ufs": ufs.len(), "municipios": nm, "secoes": cat.secoes.len(), "circunscricoes": m.ac.circs.len(),
        "abrangencias": n_pub, "esperadas": m.esperado.len(), "estado_mb": arred(cat.memoria_estado_mb()), "diario_linhas": n_linhas, "retomados": aplicados,
        "retomada_perdidos": perdidos, "orfaos_no_pacote": orfaos, "retomada_s": arred(t1.elapsed().as_secs_f64()), "carga_s": arred(t0.elapsed().as_secs_f64()),
        "disco_livre_gb": arred(livre)}));
    eprintln!("uv-servico: pleito {}, {} UFs, {} seções, {} circunscrições, {n_pub} abrangências publicadas; retomada: {aplicados} BUs do diário ({perdidos} perdidos, {orfaos} órfãos); {:.1} s",
        cat.pleito, ufs.len(), cat.secoes.len(), m.ac.circs.len(), t0.elapsed().as_secs_f64());
    m.saude("acompanhando")?;
    if let Some(p) = pronto.take() { let _ = p.send(Ok((disp, ufs))); }

    let prev_parar = AtomicBool::new(false);
    let laco = std::thread::scope(|s| -> anyhow::Result<()> {
    // previsão (etapa 8): thread própria, só lê o estado e os candidatos (nada aqui na thread do motor espera por ela)
    if let Some(p) = previsor {
        let (comr, cadr, estr, pp, ps, saida) = (&*com, &cad, &est, &prev_parar, &*prev_saude, cfg.saida.clone());
        let op = cfg.previsao.clone();
        std::thread::Builder::new().name("uv-previsao".into()).spawn_scoped(s, move || crate::previsao::rodar(comr, cadr, estr, p, prev_rx, op, pp, ps, &saida))?;
    }
    let r = (|| -> anyhow::Result<()> {
    let mut prox_verif = Instant::now();
    let mut prox_saude = Instant::now() + Duration::from_secs(2);
    let mut parar = false;
    while !parar {
        let mut msgs = vec![];
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(x) => msgs.push(x),
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {}
            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => parar = true,
        }
        while msgs.len() < 50_000 { match rx.try_recv() { Ok(x) => msgs.push(x), Err(_) => break } }
        let mut oficiais = vec![];
        let mut sondas = vec![];
        let mut normais = Vec::with_capacity(msgs.len());
        for x in msgs {
            match x { Msg::Parar => parar = true, Msg::Oficiais { baixados, faltando } => oficiais.push((baixados, faltando)), Msg::Sonda(s) => sondas.push(s), o => normais.push(o) }
        }
        if !normais.is_empty() {
            let t = Instant::now();
            m.lote(normais)?;
            m.custo_lote.por(t.elapsed().as_secs_f64() * 1e3);
        }
        if m.sujo && (cfg.cadencia_ms == 0 || m.ultima_pub.elapsed() >= Duration::from_millis(cfg.cadencia_ms)) { m.publicar(false)?; }
        // sondas: depois do lote e da publicação que vieram com elas, para a gravação (FIFO, atrás dos arquivos)
        if !sondas.is_empty() { m.passar_sondas(sondas); }
        for (b, f) in oficiais { m.conferir(b, f)?; }
        let agora = Instant::now();
        if agora >= prox_verif {
            m.verificar()?;
            // M-020: relê o limite e a % do TSE; solta os retidos que já cabem no teto
            m.atualizar_trava();
            if !m.retidos.is_empty() { m.lote(vec![])?; }
            m.custo_verif.por(agora.elapsed().as_secs_f64() * 1e3);
            prox_verif = agora + Duration::from_secs(1);
        }
        if agora >= prox_saude {
            let t = Instant::now();
            // os dados vão para o SO aqui; o disco é forçado na thread de sincronia (pacotes antes do diário)
            let mut arqs = m.esc.descarregar_e_copiar()?;
            arqs.push(m.diario.try_clone()?);
            let _ = m.sincronia.try_send(arqs);
            m.saude(if m.encerrada { "encerrada" } else { "acompanhando" })?;
            m.custo_saude.por(t.elapsed().as_secs_f64() * 1e3);
            prox_saude = agora + Duration::from_secs(2);
        }
        if com.parar.load(Relaxed) || m.sair { parar = true; }
    }
    Ok(())
    })();
    prev_parar.store(true, Relaxed);
    r
    });
    laco?;
    // parada limpa: o lote já terminou; grava publicação, pacotes, diário e saúde
    m.publicar(true)?;
    m.sincronizar()?;
    m.esc.fechar()?;
    m.diario.sync_data()?;
    m.eventos.sync_data()?;
    m.saude("parado")?;
    com.registrar("info", "parada", json!({"seq": m.seq(), "encerrada": m.encerrada, "conferencias": m.conferencias, "ativo_s": arred(com.t0.elapsed().as_secs_f64())}));
    eprintln!("uv-servico: parado ({} BUs aplicados)", m.seq());
    drop(m);
    let _ = grav_h.join();
    let _ = sinc_h.join();
    Ok(())
}

/// Thread `uv-sonda`: injeta uma sonda a cada `sonda_intervalo_s` no canal do motor (e, federado, no lote do nó, para
/// os pares) e confere o atraso das que não voltaram (alerta `sonda_atrasada`, uma vez por episódio). Roda fora da
/// thread do motor: com o motor parado, é ela que avisa.
fn injetar_sondas(com: Arc<Comum>) {
    let Some(vigia) = com.sonda.as_ref() else { return };
    let intervalo = Duration::from_secs_f64(com.cfg.sonda_intervalo_s);
    let sessao = crate::difusao::agora_us();
    let mut id = 0u64;
    let mut prox = Instant::now();
    while !com.parar.load(Relaxed) {
        let agora = Instant::now();
        if agora >= prox {
            id += 1;
            let s = crate::sonda::Sonda::nova(id, sessao, crate::difusao::agora_us());
            vigia.lock().unwrap().injetar(s.id, s.t_injecao);
            if let Some(f) = &com.federado { f.publicar_sonda(&s); }
            let _ = com.tx_motor.send(Msg::Sonda(Box::new(s)));
            prox += intervalo;
            if prox < agora { prox = agora + intervalo; }
        }
        let t = vigia.lock().unwrap().verificar(crate::difusao::agora_us());
        match t {
            Some(crate::sonda::Transicao::Atrasou { atraso_ms, pendentes, id }) => com.alerta("sonda_atrasada", &format!(
                "a sonda {id} não voltou em {:.1} s ({pendentes} pendentes): o caminho dos BUs (motor → gravação → difusão) parou ou está muito atrasado", atraso_ms / 1e3)),
            Some(crate::sonda::Transicao::Normalizou { duracao_ms }) => com.registrar("info", "sonda_normalizada", json!({"atraso_s": arred(duracao_ms / 1e3)})),
            None => {}
        }
        std::thread::sleep(intervalo.min(Duration::from_millis(100)));
    }
}

/// Roda o serviço até a conferência final (ou até `Ctrl+C`/`SIGTERM`, ou para sempre com `continuar`).
pub fn executar(mut cfg: Config) -> anyhow::Result<()> {
    limitar_arenas();
    let base = cfg.alvo.base.clone();
    // contra o site real o serviço nunca sai sozinho depois da conferência: o s.ts pré-apuração pode estar desatualizado
    // e um BU atrasado reabre a publicação (só o site simulado termina o ensaio no fim)
    if !coletor::base_local(&base) { cfg.continuar = true; }
    if cfg.controle_replay {
        anyhow::ensure!(coletor::base_local(&base), "--controle-replay só vale contra o site simulado local (127.0.0.1/localhost), não contra {base}");
        anyhow::ensure!(cfg.http.is_some(), "--controle-replay precisa de --http (o controle fica em /api/v1/replay)");
    }
    // com o controle de replay a busca anda a até 3 × 300×: teto de requisições (local) dessa velocidade
    let a_teto = if cfg.controle_replay { cfg.aceleracao_local.max(3.0 * crate::controle_replay::REPLAY_MAX) } else { cfg.aceleracao_local };
    // modo espelho: nem a base nem a dos arquivos de urna podem ser a borda pública (lá vale sempre o limite de 100)
    let base_arq = cfg.alvo.rotas.base_arquivos.clone().unwrap_or_else(|| base.clone());
    cfg.acesso.validar(&[base.as_str(), base_arq.as_str()])?;
    let (teto, rps, pausa) = coletor::limites_acesso(&base, cfg.rps, a_teto, cfg.acesso)?;
    std::fs::create_dir_all(&cfg.saida)?;
    std::fs::create_dir_all(&cfg.dados)?;
    let rede = Rede::para_acesso(&base, &ConfigRede { concorrencia: cfg.concorrencia, conexoes: 1, rps, adaptativo: false, rps_inicial: 0.0, http1: false,
        tentativas: 4, timeout_s: cfg.timeout_s, max_429: 0, teto, pausa_429_s: pausa, ip_local: cfg.ip_local }, cfg.acesso)?;
    let federado = match &cfg.federacao {
        Some(op) => {
            let escala_f = if coletor::base_local(&base) { 1.0 / cfg.aceleracao_local.max(1.0) } else { 1.0 };
            Some(Arc::new(crate::federacao::no::Federado::novo(op, &base, escala_f)?))
        }
        None => None,
    };
    let (tx_motor, rx_motor) = crossbeam_channel::unbounded();
    let log = std::fs::OpenOptions::new().create(true).append(true).open(cfg.saida.join("servico.log"))?;
    let ap = cfg.saida.join("alertas.csv");
    let novo = !ap.exists();
    let mut alertas = std::fs::OpenOptions::new().create(true).append(true).open(&ap)?;
    if novo { writeln!(alertas, "t,tipo,texto")?; }
    let local = coletor::base_local(&base);
    let escala = if local { 1.0 / cfg.aceleracao_local.max(1.0) } else { 1.0 };
    // relógio do sistema (M-003): a âncora com a noite do TSE se fixa no primeiro `Date` do site
    let relogio = Arc::new(crate::relogio::Relogio::novo(1.0 / escala));
    // difusão (--http): a thread uv-difusao existe desde a partida, para a publicação a frio já entrar nos tópicos;
    // não precisa ser esperada no fim (só monta mensagens)
    let (difusao, _dif_h) = match &cfg.http {
        Some(op) => {
            let (h, rx) = crate::difusao::Hub::novo(op.clone());
            h.definir_relogio(relogio.clone());
            let h2 = h.clone();
            let t = std::thread::Builder::new().name("uv-difusao".into()).spawn(move || crate::difusao::difundir(h2, rx))?;
            (Some(h), Some(t))
        }
        None => (None, None),
    };
    // comparação com o TSE: só com a difusão (as inscrições tse: é que ligam as buscas)
    let tse = match (&difusao, &cfg.tse) {
        (Some(h), Some(op)) => { let t = crate::tse_resultados::TseResultados::novo(op.clone()); h.definir_tse(t.clone()); Some(t) }
        _ => None,
    };
    let com = Arc::new(Comum {
        cfg: cfg.clone(), rede, parar: AtomicBool::new(false), cs_ocioso: AtomicBool::new(false), fila: AtomicI64::new(0), lat_aux: Mutex::default(), lat_bu: Mutex::default(),
        cont: Contadores::default(), publicadas_cs: Mutex::default(), agregadas_cs: Mutex::default(), agregadas_ger: Default::default(), aplicados: Mutex::default(), cache: Mutex::default(), relogio_site: AtomicI64::new(0), relogio,
        t0: Instant::now(), escala: AtomicU64::new(escala.to_bits()), tx_motor, lat_pub: Mutex::default(), custo_grav: Mutex::default(), custo_sync: Mutex::default(), difusao: difusao.clone(), federado: federado.clone(), tse: tse.clone(), log: Mutex::new(Some(log)), alertas_arq: Mutex::new(Some(alertas)), alertas: Mutex::default(),
        sonda: (cfg.sonda_intervalo_s > 0.0).then(|| Mutex::new(crate::sonda::Vigia::novo(cfg.sonda_intervalo_s, cfg.sonda_limite_s))),
    });
    com.registrar("info", "inicio", json!({"versao": urna_velox::VERSAO, "base": base, "ambiente": cfg.alvo.ambiente, "ciclo": cfg.alvo.ciclo, "pleito": cfg.alvo.pleito,
        "servidor_local": local, "acesso": cfg.acesso.texto(), "teto_req_s": com.rede.teto(), "req_s": rps, "pausa_429_s": pausa, "intervalo_cs_s": cfg.intervalo_cs, "intervalo_cs_ocioso_s": cfg.intervalo_cs_ocioso, "concorrencia": cfg.concorrencia}));
    eprintln!("uv-servico {}: {base} pleito {}, {rps} req/s (teto {}), pausa de {pausa} s depois de um 429{}", urna_velox::VERSAO, cfg.alvo.pleito, com.rede.teto(),
        if local { " — servidor local".to_string() } else if cfg.acesso.interno() { format!(" — acesso {}", cfg.acesso.texto()) } else { String::new() });
    if let (true, Some(hub)) = (cfg.controle_replay, &difusao) {
        let (f1, f2, f3) = (Arc::downgrade(&com), Arc::downgrade(&com), Arc::downgrade(&com));
        hub.definir_replay(crate::controle_replay::Replay::novo(&base, cfg.aceleracao_local, crate::controle_replay::Ganchos {
            ao_mudar: Box::new(move |x| {
                if let Some(c) = f1.upgrade() {
                    c.definir_escala(x);
                    c.registrar("info", "replay_velocidade", json!({"acelerar": x, "relogio_site": crate::texto_hora(c.relogio_site.load(Relaxed))}));
                }
            }),
            fila: Box::new(move || f2.upgrade().map(|c| c.fila.load(Relaxed)).unwrap_or(0)),
            parar: Box::new(move || {
                if let Some(c) = f3.upgrade() {
                    c.registrar("info", "replay_reinicio", json!({"relogio_site": crate::texto_hora(c.relogio_site.load(Relaxed))}));
                    c.parar.store(true, Relaxed);
                    let _ = c.tx_motor.send(Msg::Parar);
                }
            }),
        }));
    }
    let (ped_tx, ped_rx) = tokio::sync::mpsc::unbounded_channel();
    let (pronto_tx, pronto_rx) = std::sync::mpsc::channel();
    let c2 = com.clone();
    let h = std::thread::Builder::new().name("uv-motor".into()).spawn(move || {
        let mut p = Some(pronto_tx);
        let r = rodar_motor(c2.clone(), rx_motor, ped_tx, &mut p);
        if let Err(e) = &r {
            c2.registrar("erro", "motor", json!({"erro": format!("{e:#}")}));
            if let Some(p) = p { let _ = p.send(Err(format!("{e:#}"))); }
        }
        c2.parar.store(true, Relaxed);
        r
    })?;
    let (conhecidas, ufs) = match pronto_rx.recv() {
        Ok(Ok(x)) => x,
        Ok(Err(e)) => { let _ = h.join(); anyhow::bail!(e) }
        Err(_) => return h.join().map_err(|_| anyhow::anyhow!("a thread do motor falhou"))?,
    };
    // camada HTTP/WebSocket: só depois da publicação a frio (todo tópico já tem valor), num runtime próprio com poucas
    // threads, para não disputar CPU com a rede de coleta e o motor
    let http_h = match &difusao {
        Some(hub) => {
            let (hub, threads) = (hub.clone(), hub.op.threads.max(1));
            let (ptx, prx) = std::sync::mpsc::channel();
            let com2 = com.clone();
            let t = std::thread::Builder::new().name("uv-http".into()).spawn(move || {
                // threads da camada HTTP com prioridade menor: cedem a CPU ao motor, à gravação e à coleta
                crate::difusao::prioridade_baixa();
                let rt = match tokio::runtime::Builder::new_multi_thread().worker_threads(threads).thread_name("uv-http")
                    .on_thread_start(crate::difusao::prioridade_baixa).enable_all().build() {
                    Ok(rt) => rt,
                    Err(e) => { let _ = ptx.send(Err(e.to_string())); return; }
                };
                let ptx2 = ptx.clone();
                if let Err(e) = rt.block_on(crate::difusao::servir(hub, move |a| { let _ = ptx2.send(Ok(a)); })) {
                    com2.registrar("erro", "http", json!({"erro": format!("{e:#}")}));
                    let _ = ptx.send(Err(format!("{e:#}")));
                }
                rt.shutdown_timeout(Duration::from_secs(1));
            })?;
            match prx.recv_timeout(Duration::from_secs(10)) {
                Ok(Ok(a)) => {
                    com.registrar("info", "http", json!({"endereco": a.to_string(), "max_clientes": cfg.http.as_ref().map(|o| o.max_clientes), "threads": threads}));
                    eprintln!("uv-servico: difusão em http://{a} (WebSocket /ws, REST /api/v1/)");
                    // para os ensaios com porta 0: <saida>/http_porta
                    let _ = std::fs::write(cfg.saida.join("http_porta"), a.port().to_string());
                }
                Ok(Err(e)) => { com.parar.store(true, Relaxed); let _ = com.tx_motor.send(Msg::Parar); let _ = h.join(); anyhow::bail!("--http: {e}") }
                Err(_) => { com.parar.store(true, Relaxed); let _ = com.tx_motor.send(Msg::Parar); let _ = h.join(); anyhow::bail!("--http: o servidor não começou a escutar") }
            }
            Some(t)
        }
        None => None,
    };
    // sondas (padrão Test Message): só depois da partida do motor (a retomada não conta como atraso)
    if com.sonda.is_some() {
        let c = com.clone();
        std::thread::Builder::new().name("uv-sonda".into()).spawn(move || injetar_sondas(c))?;
    }
    // a rede é limitada por E/S (≤ 100 req/s contra o TSE): 4 threads bastam e não espalham memória por 28 arenas
    let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(4).enable_all().build()?;
    let r = rt.block_on(async {
        let (ftx, frx) = tokio::sync::mpsc::unbounded_channel::<Tarefa>();
        let frx = Arc::new(tokio::sync::Mutex::new(frx));
        {
            let com = com.clone();
            tokio::spawn(async move {
                #[cfg(unix)]
                {
                    let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).expect("SIGTERM");
                    tokio::select! { _ = tokio::signal::ctrl_c() => {}, _ = term.recv() => {} }
                }
                #[cfg(not(unix))]
                { let _ = tokio::signal::ctrl_c().await; }
                com.registrar("info", "sinal", json!({"acao": "parada limpa"}));
                com.parar.store(true, Relaxed);
                let _ = com.tx_motor.send(Msg::Parar);
            });
        }
        if let Some(f) = &federado {
            use crate::federacao::no;
            let acel = if local { cfg.aceleracao_local.max(1.0) } else { 1.0 };
            match no::republicar(f, &cfg.saida, &cfg.dados) {
                Ok(r) => {
                    if r.registros > 0 { com.registrar("info", "federacao_republicados", json!({"registros": r.registros, "lotes": r.lotes})); }
                    if let Some(motivo) = r.desalinhado { com.alerta("federacao_diario_desalinhado", &motivo); }
                }
                Err(e) => com.registrar("aviso", "federacao_republicar", json!({"erro": format!("{e:#}")})),
            }
            // sincronização direta com os pares (sem storage compartilhado): serve o diretório do nó e copia o dos outros
            let escuta = cfg.federacao.as_ref().and_then(|o| o.escuta)
                .or_else(|| f.endereco_de(&f.id).and_then(|e| e.parse().ok()));
            match escuta {
                Some(e) => {
                    let (f2, c2) = (f.clone(), com.clone());
                    tokio::spawn(async move {
                        if let Err(x) = crate::federacao::sincronia::servir(f2, c2.clone(), e).await {
                            c2.registrar("erro", "federacao_escuta", json!({"erro": format!("{x:#}")}));
                        }
                    });
                }
                None => com.registrar("aviso", "federacao_sem_endereco", json!({"no": f.id, "texto": "sem endereço: os pares não alcançam este nó e o veem como morto; ele só recebe"})),
            }
            tokio::spawn(crate::federacao::sincronia::sincronizar(f.clone(), com.clone()));
            tokio::spawn(crate::federacao::sincronia::vigiar_batimentos(f.clone(), com.clone()));
            tokio::spawn(no::batimentos(f.clone(), com.clone(), ftx.clone(), acel));
            tokio::spawn(no::publicador(f.clone(), com.clone()));
            tokio::spawn(no::consumidor(f.clone(), com.clone()));
            // dois heartbeats antes de coletar: os nós que partem juntos se enxergam e não coletam o mesmo (o que
            // ainda assim cair na seção errada é corrigido: delegada volta à fila quando a partição a der a este nó)
            tokio::time::sleep(f.batimento * 2).await;
            com.registrar("info", "federacao", json!({"no": f.id, "batimento_s": f.batimento.as_secs_f64(), "troca_s": f.troca.as_secs_f64(), "estado": f.saude_json()}));
        }
        if cfg.sem_coleta {
            com.registrar("info", "sem_coleta", json!({"texto": "nenhum cs.json, aux.json ou BU: só a publicação da partida, a difusão e a comparação com o TSE"}));
        } else {
            tokio::spawn(acompanhar(com.clone(), ufs, conhecidas, ftx.clone()));
            for _ in 0..cfg.concorrencia.max(1) { tokio::spawn(coletor(com.clone(), frx.clone(), ftx.clone())); }
        }
        tokio::spawn(atender(com.clone(), ped_rx, ftx.clone()));
        if let Some(t) = &tse { tokio::spawn(crate::tse_resultados::buscar(t.clone(), com.clone())); }
        let r = tokio::task::spawn_blocking(move || h.join()).await;
        com.parar.store(true, Relaxed);
        // federação: grava o último lote e continua servindo os pares por uma carência, para que levem o que falta
        if let Some(f) = &federado {
            if let Err(e) = crate::federacao::no::gravar_lote(f) { com.registrar("aviso", "lote_final", json!({"erro": format!("{e:#}")})); }
            let carencia = (f.troca * 30).max(Duration::from_secs(3)).min(Duration::from_secs(60));
            com.registrar("info", "federacao_carencia", json!({"s": carencia.as_secs_f64()}));
            tokio::time::sleep(carencia).await;
            f.fim.store(true, Relaxed);
        }
        r
    });
    rt.shutdown_timeout(Duration::from_secs(2));
    // a publicação final já foi entregue à difusão antes da gravação: agora avisa os clientes e fecha as conexões
    if let (Some(hub), Some(t)) = (&difusao, http_h) {
        // dá às conexões um instante para mandar a publicação final que a gravação entregou
        std::thread::sleep(Duration::from_millis(200));
        hub.desligar();
        let _ = t.join();
        let d = hub.saude_json();
        com.registrar("info", "http_parado", d.clone());
        // a saúde final (gravada pelo motor ao parar) recebe o bloco da difusão completo, com as conexões já fechadas
        let p = cfg.saida.join("saude.json");
        if let Ok(mut s) = std::fs::read(&p).map_err(|_| ()).and_then(|b| serde_json::from_slice::<Value>(&b).map_err(|_| ())) {
            s["difusao"] = d;
            if let Ok(t) = serde_json::to_string_pretty(&s) { let _ = gravar_atomico(&cfg.saida, "saude.json", &(t + "\n")); }
        }
    }
    match r {
        Ok(Ok(x)) => x,
        _ => anyhow::bail!("a thread do motor falhou"),
    }
}

#[cfg(test)]
mod testes;
