//! Comparação Urna Velox × TSE (etapa 7, `docs/handoffs/HANDOFF_0.7.0.md`): o `uv-servico` busca no site de resultados do TSE
//! o arquivo de resultado (`<abr>-c<cargo>-e<eleição>-u.json`) **só da abrangência e cargo que algum cliente tem na
//! tela com a comparação ligada** (inscrição no tópico `tse:<tópico>`, `docs/arquitetura/PROTOCOLO_WS.md` §4.6) e o distribui a
//! todos os clientes desse tópico.
//!
//! Regra de rede (combinada com o Luciano em 16/10/2026, `CLAUDE.md`): no máximo uma busca a cada `--tse-intervalo`
//! (30–60 s, hora do site) por (abrangência, cargo), não importa quantos clientes; **nada em segundo plano** (sem
//! inscrito, nenhuma requisição); no máximo `--tse-max-topicos` tópicos buscados ao mesmo tempo (os outros esperam
//! vaga, com o estado `indisponivel`); a mesma [`Rede`](crate::coletor::Rede) da coleta (100 req/s do órgão, pausa
//! de ≥ 10 min depois de um HTTP 429), com prioridade abaixo dela ([`crate::coletor::Limitador::esperar_baixa`]);
//! `If-None-Match` e `If-Modified-Since` (304 não traz corpo). Na federação, cada nó busca para os seus clientes.
//!
//! O parse é o do módulo `oficial` do motor (o mesmo das conferências), a partir dos bytes
//! ([`urna_velox::oficial::Oficial::de_bytes`]).

use crate::coletor::Cond;
use crate::servico::Comum;
use axum::extract::ws::Utf8Bytes;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
use tokio::sync::watch;
use urna_velox::oficial::Oficial;

pub const INTERVALO_MIN_S: f64 = 30.0;
pub const INTERVALO_MAX_S: f64 = 60.0;

/// Opções (`--tse-intervalo`, `--tse-max-topicos`).
#[derive(Clone, Debug)]
pub struct Opcoes {
    /// segundos (hora do site) entre duas buscas do mesmo tópico: 30 a 60
    pub intervalo_s: f64,
    /// tópicos buscados ao mesmo tempo
    pub max_topicos: usize,
}
impl Default for Opcoes { fn default() -> Self { Opcoes { intervalo_s: INTERVALO_MIN_S, max_topicos: 200 } } }
impl Opcoes {
    pub fn validar(&self) -> anyhow::Result<()> {
        anyhow::ensure!((INTERVALO_MIN_S..=INTERVALO_MAX_S).contains(&self.intervalo_s),
            "--tse-intervalo {} recusado: de {INTERVALO_MIN_S} a {INTERVALO_MAX_S} s (combinado com o Luciano, 16/10/2026)", self.intervalo_s);
        anyhow::ensure!(self.max_topicos >= 1, "--tse-max-topicos deve ser ≥ 1");
        Ok(())
    }
}

/// Estado da fonte TSE de um tópico (campo `estado` da mensagem `tse`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Estado {
    /// inscrito, primeira busca ainda não respondida
    Aguardando,
    /// arquivo obtido e confirmado (200 ou 304) há no máximo dois intervalos
    Ok,
    /// sem resposta válida (200/304) do TSE há mais de dois intervalos (ver [`TseResultados::atualizar_estados`])
    Desatualizado,
    /// 404, erro de rede ou arquivo ilegível; ou sem vaga (`motivo` = `limite_topicos`)
    Indisponivel,
    /// pausa da rede depois de um HTTP 429 (nenhuma requisição ao TSE até o fim dela)
    Pausado,
}
impl Estado {
    pub fn texto(self) -> &'static str {
        match self { Estado::Aguardando => "aguardando", Estado::Ok => "ok", Estado::Desatualizado => "desatualizado", Estado::Indisponivel => "indisponivel", Estado::Pausado => "pausado" }
    }
}

/// A mensagem `tse` de um tópico numa versão (montada uma vez; `instantaneo` = a mesma com `"instantaneo":true`).
pub struct Quadro {
    pub ver: u64,
    pub atualizacao: Utf8Bytes,
    pub instantaneo: Utf8Bytes,
}

#[derive(Default)]
struct Medidas {
    buscas: AtomicU64,
    r200: AtomicU64,
    r304: AtomicU64,
    r404: AtomicU64,
    erros: AtomicU64,
    ilegiveis: AtomicU64,
    /// buscas por tópico acima do teto de uma por intervalo (prova da regra: deve ficar 0)
    fora_do_intervalo: AtomicU64,
}

struct Topico {
    url: String,
    arquivo: String,
    inscritos: usize,
    /// tem vaga entre os `max_topicos` buscados
    ativo: bool,
    /// ordem de chegada na espera por vaga
    espera_desde: Instant,
    tx: watch::Sender<Option<Arc<Quadro>>>,
    ver: u64,
    etag: Option<String>,
    modificado: Option<String>,
    /// resumo do último arquivo (o corpo da mensagem sem envelope)
    dados: Option<Value>,
    geracao: String,
    /// hora do site em que esta versão do arquivo apareceu pela primeira vez
    novo_em_site: i64,
    /// última resposta válida (200/304), instante real
    ultima_ok: Option<Instant>,
    /// hora do site da última busca respondida
    buscado_site: i64,
    /// próxima busca permitida (instante real) e se há uma em andamento
    prox: Instant,
    /// relógio do site (`Date` das respostas) na última busca
    ultima_site: i64,
    ultima_busca: Option<Instant>,
    em_voo: bool,
    estado: Estado,
    motivo: Option<&'static str>,
}

/// Dono do mapa (abrangência, cargo) → último arquivo do TSE; compartilhado pela difusão (inscrições) e pela rede
/// (buscas, [`buscar`]).
pub struct TseResultados {
    pub op: Opcoes,
    /// tópico → (URL do `-u.json`, nome do arquivo); definido pelo motor na partida ([`TseResultados::definir_urls`])
    urls: RwLock<HashMap<String, (String, String)>>,
    /// tópico → eleitorado do `candidatos.json` ([`TseResultados::definir_eleitorado`]; B-055) e os tópicos já
    /// alertados por eleitorado divergente (um alerta por tópico)
    eleitorado: RwLock<HashMap<String, u64>>,
    eleitorado_alertado: Mutex<std::collections::HashSet<String>>,
    t: Mutex<HashMap<Arc<str>, Topico>>,
    acordar: tokio::sync::Notify,
    med: Medidas,
    /// (instante, buscas) para o req/s do `saude.json`
    taxa_ant: Mutex<Option<(Instant, u64)>>,
}

/// Inscrição de uma conexão num tópico `tse:`; ao cair (fim da conexão, `cancelar`), o tópico perde um inscrito e,
/// sem nenhum, deixa de ser buscado na hora.
pub struct Inscricao { tse: Arc<TseResultados>, topico: Arc<str> }
impl Drop for Inscricao { fn drop(&mut self) { self.tse.sair(&self.topico); } }

impl TseResultados {
    pub fn novo(op: Opcoes) -> Arc<TseResultados> {
        Arc::new(TseResultados { op, urls: RwLock::default(), eleitorado: RwLock::default(), eleitorado_alertado: Mutex::default(), t: Mutex::default(), acordar: tokio::sync::Notify::new(), med: Medidas::default(), taxa_ant: Mutex::default() })
    }

    /// URLs dos `-u.json` de cada tópico publicado (os mesmos que a conferência baixa).
    pub fn definir_urls(&self, m: HashMap<String, (String, String)>) { *self.urls.write().unwrap() = m; }

    /// Seções (total, totalizadas) do último arquivo lido do tópico (M-020, a trava).
    pub fn secoes_de(&self, topico: &str) -> Option<(u64, u64)> {
        let t = self.t.lock().unwrap();
        let d = t.get(topico)?.dados.as_ref()?;
        Some((d["secoes"]["total"].as_u64()?, d["secoes"]["totalizadas"].as_u64()?))
    }

    /// Eleitorado de cada tópico no `candidatos.json` (o que o "matematicamente" usa), para conferir com o `e.te` do TSE.
    pub fn definir_eleitorado(&self, m: HashMap<String, u64>) { *self.eleitorado.write().unwrap() = m; }

    /// B-055: texto do alerta `eleitorado_divergente` quando o `e.te` do `-u.json` do TSE (`te`, maior que zero) difere do
    /// eleitorado do `candidatos.json` no tópico; `None` se confere, se não há o que comparar ou se o tópico já alertou.
    /// Não faz requisição: usa o arquivo que a comparação já baixou.
    pub fn conferir_eleitorado(&self, topico: &str, te: u64) -> Option<String> {
        let cat = *self.eleitorado.read().unwrap().get(topico)?;
        if te == 0 || te == cat || !self.eleitorado_alertado.lock().unwrap().insert(topico.to_string()) { return None; }
        Some(format!("{topico}: eleitorado do candidatos.json {cat}, TSE {te} (diferença {}): regerar o candidatos.json \
            (uv-coletor preparar --atualizar) e reiniciar o serviço; o \"matematicamente\" usa o do candidatos.json",
            te as i64 - cat as i64))
    }

    /// Inscreve uma conexão no tópico (sem o prefixo `tse:`). Erro: tópico sem `-u.json` conhecido.
    pub fn inscrever(self: &Arc<Self>, topico: &str) -> Result<(Inscricao, watch::Receiver<Option<Arc<Quadro>>>), &'static str> {
        let Some((url, arquivo)) = self.urls.read().unwrap().get(topico).cloned() else { return Err("topico_desconhecido") };
        let mut t = self.t.lock().unwrap();
        let ativos = t.values().filter(|x| x.ativo).count();
        let max = self.op.max_topicos;
        let chave: Arc<str> = Arc::from(topico);
        let e = t.entry(chave.clone()).or_insert_with(|| Topico {
            url, arquivo, inscritos: 0, ativo: false, espera_desde: Instant::now(), tx: watch::Sender::new(None), ver: 0, etag: None, modificado: None,
            dados: None, geracao: String::new(), novo_em_site: 0, ultima_ok: None, buscado_site: 0, prox: Instant::now(), ultima_site: i64::MIN, ultima_busca: None, em_voo: false,
            estado: Estado::Aguardando, motivo: None,
        });
        e.inscritos += 1;
        if !e.ativo {
            if e.inscritos == 1 { e.espera_desde = Instant::now(); }
            if ativos < max {
                e.ativo = true;
                if e.dados.is_none() { e.estado = Estado::Aguardando; e.motivo = None; }
            } else {
                e.estado = Estado::Indisponivel;
                e.motivo = Some("limite_topicos");
            }
            // a mensagem inicial (aguardando ou sem vaga) existe desde já: o instantâneo da inscrição a usa
            publicar(&chave, e);
        } else if e.tx.borrow().is_none() {
            publicar(&chave, e);
        }
        let rx = e.tx.subscribe();
        drop(t);
        self.acordar.notify_one();
        Ok((Inscricao { tse: self.clone(), topico: chave }, rx))
    }

    fn sair(&self, topico: &Arc<str>) {
        let mut t = self.t.lock().unwrap();
        let Some(e) = t.get_mut(topico) else { return };
        e.inscritos = e.inscritos.saturating_sub(1);
        if e.inscritos > 0 { return; }
        let liberou = e.ativo;
        e.ativo = false;
        // vaga livre: o tópico com inscritos que espera há mais tempo
        if liberou {
            let prox = t.iter().filter(|(_, x)| !x.ativo && x.inscritos > 0).min_by_key(|(_, x)| x.espera_desde).map(|(k, _)| k.clone());
            if let Some(k) = prox {
                let x = t.get_mut(&k).unwrap();
                x.ativo = true;
                x.estado = if x.dados.is_some() { Estado::Ok } else { Estado::Aguardando };
                x.motivo = None;
                publicar(&k, x);
            }
        }
        drop(t);
        self.acordar.notify_one();
    }

    /// Bloco `tse_resultados` do `saude.json`.
    pub fn saude_json(&self, relogio_site: i64) -> Value {
        let t = self.t.lock().unwrap();
        let ativos: Vec<&Topico> = t.values().filter(|x| x.ativo && x.inscritos > 0).collect();
        let idade = ativos.iter().filter(|x| x.dados.is_some()).filter_map(|x| segundos_br(&x.geracao)).map(|g| relogio_site - g).max();
        let mut estados: std::collections::BTreeMap<&str, u64> = Default::default();
        for x in t.values().filter(|x| x.inscritos > 0) { *estados.entry(x.estado.texto()).or_default() += 1; }
        let buscas = self.med.buscas.load(Relaxed);
        let agora = Instant::now();
        let req_s = {
            let mut a = self.taxa_ant.lock().unwrap();
            let r = a.map(|(t0, b0)| (buscas - b0) as f64 / (agora - t0).as_secs_f64().max(1e-3)).unwrap_or(0.0);
            *a = Some((agora, buscas));
            r
        };
        let m = &self.med;
        json!({"intervalo_s": self.op.intervalo_s, "max_topicos": self.op.max_topicos, "topicos_ativos": ativos.len(),
            "inscritos": t.values().map(|x| x.inscritos).sum::<usize>(),
            "esperando_vaga": t.values().filter(|x| !x.ativo && x.inscritos > 0).count(), "estados": estados,
            "buscas": buscas, "req_s": (req_s * 1000.0).round() / 1000.0, "r200": m.r200.load(Relaxed), "r304": m.r304.load(Relaxed),
            "r404": m.r404.load(Relaxed), "erros": m.erros.load(Relaxed), "ilegiveis": m.ilegiveis.load(Relaxed),
            "fora_do_intervalo": m.fora_do_intervalo.load(Relaxed), "idade_max_s": idade})
    }

    /// Estados que dependem do tempo: `pausado` durante a pausa de 429, `desatualizado` depois de dois intervalos sem
    /// resposta válida, e a volta a `ok`. Publica de novo os tópicos que mudaram.
    fn atualizar_estados(&self, com: &Comum) {
        let pausa = !com.rede.lim.pausa_restante().is_zero();
        let limite = com.espera(2.0 * self.op.intervalo_s);
        let inteiro = Duration::from_secs_f64(2.0 * self.op.intervalo_s);
        let site = com.relogio_site.load(Relaxed);
        // mesma conta da busca: dois intervalos na hora do site (ou em tempo real, se o relógio do site não andou)
        let velho = |e: &Topico| e.ultima_ok.is_some_and(|u| u.elapsed() > limite
            && (site > e.buscado_site + (2.0 * self.op.intervalo_s).ceil() as i64 || u.elapsed() > inteiro));
        let mut t = self.t.lock().unwrap();
        for (k, e) in t.iter_mut() {
            if !e.ativo || e.inscritos == 0 { continue; }
            let novo = if pausa { Estado::Pausado }
                else if e.estado == Estado::Pausado { if e.dados.is_some() { Estado::Ok } else { Estado::Aguardando } }
                else if e.estado == Estado::Ok && velho(e) { Estado::Desatualizado }
                else { e.estado };
            if novo != e.estado { e.estado = novo; if novo != Estado::Indisponivel { e.motivo = None; } publicar(k, e); }
        }
    }
}

/// "dd/mm/aaaa hh:mm:ss" → segundos (escala de `urna_velox::metricas::segundos`).
fn segundos_br(s: &str) -> Option<i64> { if s.is_empty() { None } else { urna_velox::metricas::segundos(s) } }

/// Resumo de um arquivo do TSE para a mensagem `tse`: votos, % e posição por candidato, seções, comparecimento.
/// Posição: pelos votos do próprio arquivo; empate como no motor — o mais idoso à frente (`dt`) —, depois o `seq` e o
/// número (só para exibir; a situação de cada candidato vem do arquivo, `st`/`e`). O `seq` sozinho não serve: nos arquivos de UF do 3220 ele é a ordem nacional,
/// e nos municipais de 2024 é um sequencial que não vai de 1 a n.
pub fn resumo(o: &Oficial) -> Value {
    let mut idx: Vec<usize> = (0..o.cand.len()).collect();
    let seq = |i: usize| if o.cand[i].seq > 0 { o.cand[i].seq as u64 } else { u64::MAX };
    let nasc = |i: usize| urna_velox::candidatos::data(&o.cand[i].nascimento).unwrap_or(u32::MAX);
    idx.sort_by(|&a, &b| o.cand[b].vap.cmp(&o.cand[a].vap).then(nasc(a).cmp(&nasc(b))).then(seq(a).cmp(&seq(b))).then(o.cand[a].numero.cmp(&o.cand[b].numero)));
    // % dos válidos: o do arquivo (pvapn); sem ele, votos ÷ soma dos votos a candidatos com destinação "Válido"
    let soma: u64 = o.cand.iter().filter(|c| c.dvt.is_empty() || c.dvt.starts_with("Válido")).map(|c| c.vap).sum();
    let cand: Vec<Value> = idx.iter().enumerate().map(|(pos, &i)| {
        let c = &o.cand[i];
        let valido = c.dvt.is_empty() || c.dvt.starts_with("Válido");
        let pct = c.pvap.or_else(|| (valido && soma > 0).then(|| (c.vap as f64 * 10000.0 / soma as f64).round() / 100.0));
        json!({"n": c.numero, "v": c.vap, "pct": pct, "pos": pos + 1, "e": c.eleito, "st": c.st, "dvt": c.dvt})
    }).collect();
    json!({"geracao": o.geracao, "totalizacao": format!("{} {}", o.data, o.hora).trim(), "andamento": o.andamento, "md": o.md, "finalizada": o.finalizada,
        "secoes": {"total": o.secoes, "totalizadas": o.secoes_totalizadas, "nao_instaladas": o.nao_instaladas},
        "eleitorado": o.eleitorado, "comparecimento": o.comparecimento, "abstencao": o.abstencao,
        "vb": o.vb, "vn": o.vn, "vnt": o.vnt, "validos": o.validos, "cand": cand})
}

/// Monta a mensagem do tópico com os dados e o estado atuais e a entrega a quem está inscrito.
fn publicar(k: &Arc<str>, e: &mut Topico) {
    e.ver += 1;
    let base = json!({"topico": format!("tse:{k}"), "ver": e.ver, "estado": e.estado.texto(), "motivo": e.motivo, "arquivo": e.arquivo,
        "buscado": if e.buscado_site > 0 { Value::from(crate::texto_hora(e.buscado_site)) } else { Value::Null },
        "novo_em": if e.novo_em_site > 0 { Value::from(crate::texto_hora(e.novo_em_site)) } else { Value::Null },
        "dados": e.dados.clone().unwrap_or(Value::Null)});
    let t = crate::difusao::agora_us();
    let monta = |inst: bool| {
        let mut v = base.clone();
        let o = v.as_object_mut().unwrap();
        let mut m = serde_json::Map::new();
        m.insert("tipo".into(), json!("tse"));
        m.insert("instantaneo".into(), json!(inst));
        m.extend(std::mem::take(o));
        m.insert("t_envio".into(), json!(t));
        Utf8Bytes::from(Value::Object(m).to_string())
    };
    let q = Arc::new(Quadro { ver: e.ver, atualizacao: monta(false), instantaneo: monta(true) });
    e.tx.send_replace(Some(q));
}

/// Laço das buscas (no runtime da coleta): a cada segundo (ou quando uma inscrição muda), busca os tópicos ativos
/// com inscritos cuja vez chegou — uma busca por tópico por intervalo, no máximo uma em andamento por tópico.
pub async fn buscar(tse: Arc<TseResultados>, com: Arc<Comum>) {
    while !com.parar.load(Relaxed) {
        tse.atualizar_estados(&com);
        let agora = Instant::now();
        let intervalo = com.espera(tse.op.intervalo_s);
        let inteiro = Duration::from_secs_f64(tse.op.intervalo_s);
        let site = com.relogio_site.load(Relaxed);
        // o intervalo vale na hora do site: com o site acelerado, a espera real encolhe; mas se o relógio do site não andou
        // (simulador parado), só um intervalo inteiro de tempo real libera a próxima busca
        let liberado = |e: &Topico| agora >= e.prox
            && (e.ultima_site == i64::MIN || site >= e.ultima_site + tse.op.intervalo_s.ceil() as i64 || e.ultima_busca.is_none_or(|u| agora.duration_since(u) >= inteiro));
        let vez: Vec<(Arc<str>, String, String, Option<String>, Option<String>)> = {
            let mut t = tse.t.lock().unwrap();
            t.iter_mut().filter(|(_, e)| e.ativo && e.inscritos > 0 && !e.em_voo && liberado(e)).map(|(k, e)| {
                e.em_voo = true;
                // prova da regra: duas buscas do mesmo tópico a menos de um intervalo (com folga de 5 %) não acontecem
                if e.ultima_busca.is_some_and(|u| agora.duration_since(u) < intervalo.mul_f64(0.95)) { tse.med.fora_do_intervalo.fetch_add(1, Relaxed); }
                e.ultima_busca = Some(agora);
                e.ultima_site = site;
                e.prox = agora + intervalo;
                (k.clone(), e.url.clone(), e.arquivo.clone(), e.etag.clone(), e.modificado.clone())
            }).collect()
        };
        for (k, url, arq, etag, modif) in vez {
            let (tse, com) = (tse.clone(), com.clone());
            tokio::spawn(async move { buscar_um(&tse, &com, k, url, arq, etag, modif).await });
        }
        tokio::select! {
            _ = tse.acordar.notified() => {}
            _ = tokio::time::sleep(Duration::from_millis(250)) => {}
        }
    }
}

async fn buscar_um(tse: &TseResultados, com: &Comum, k: Arc<str>, url: String, arq: String, etag: Option<String>, modif: Option<String>) {
    tse.med.buscas.fetch_add(1, Relaxed);
    let r = com.rede.get_com(&url, Cond { etag: etag.as_deref(), modificado: modif.as_deref(), baixa: true }, None).await;
    let site = r.as_ref().ok().map(|r| com.ver_relogio(r.data.as_deref())).unwrap_or_else(|| com.relogio_site.load(Relaxed));
    // B-055: o eleitorado do candidatos.json confere com o do TSE? (o arquivo que esta busca já trouxe; nada a mais)
    if let Ok(r) = &r {
        if r.status == 200 {
            if let Ok(o) = Oficial::de_bytes(&arq, &r.corpo) {
                if let Some(x) = tse.conferir_eleitorado(&k, o.eleitorado) { com.alerta("eleitorado_divergente", &x); }
            }
        }
    }
    let mut t = tse.t.lock().unwrap();
    let Some(e) = t.get_mut(&k) else { return };
    e.em_voo = false;
    // a próxima busca conta a partir da resposta: a espera da vaga (prioridade baixa) ou a latência desta não encurtam
    // o intervalo entre duas requisições
    e.prox = e.prox.max(Instant::now() + com.espera(tse.op.intervalo_s));
    match r {
        Ok(r) if r.status == 304 => {
            tse.med.r304.fetch_add(1, Relaxed);
            e.ultima_ok = Some(Instant::now());
            e.buscado_site = site;
            if e.estado != Estado::Ok && e.dados.is_some() { e.estado = Estado::Ok; e.motivo = None; publicar(&k, e); }
        }
        Ok(r) if r.status == 200 => {
            tse.med.r200.fetch_add(1, Relaxed);
            match Oficial::de_bytes(&arq, &r.corpo) {
                Ok(o) => {
                    e.ultima_ok = Some(Instant::now());
                    e.buscado_site = site;
                    if r.etag.is_some() { e.etag = r.etag; }
                    if r.modificado.is_some() { e.modificado = r.modificado; }
                    let mudou = e.dados.is_none() || o.geracao != e.geracao || e.estado != Estado::Ok;
                    let novo_arquivo = e.dados.is_none() || o.geracao != e.geracao;
                    if novo_arquivo { e.novo_em_site = site; e.geracao = o.geracao.clone(); }
                    let d = resumo(&o);
                    let dados_mudaram = e.dados.as_ref() != Some(&d);
                    e.dados = Some(d);
                    e.estado = Estado::Ok;
                    e.motivo = None;
                    if mudou || dados_mudaram { publicar(&k, e); }
                }
                Err(_) => {
                    tse.med.ilegiveis.fetch_add(1, Relaxed);
                    if e.estado != Estado::Indisponivel || e.motivo != Some("ilegivel") { e.estado = Estado::Indisponivel; e.motivo = Some("ilegivel"); publicar(&k, e); }
                }
            }
        }
        Ok(r) => {
            let motivo = if r.status == 404 { tse.med.r404.fetch_add(1, Relaxed); "404" } else { tse.med.erros.fetch_add(1, Relaxed); "erro" };
            if e.estado != Estado::Indisponivel || e.motivo != Some(motivo) { e.estado = Estado::Indisponivel; e.motivo = Some(motivo); publicar(&k, e); }
        }
        Err(_) => {
            tse.med.erros.fetch_add(1, Relaxed);
            // a pausa de 429 aparece como `pausado` (atualizar_estados); fora dela, erro de rede
            if com.rede.lim.pausa_restante().is_zero() && (e.estado != Estado::Indisponivel || e.motivo != Some("erro")) {
                e.estado = Estado::Indisponivel; e.motivo = Some("erro"); publicar(&k, e);
            }
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    /// B-055: no 3221 o candidatos.json de 18/10 tinha Brasil 157.563.461 e o TSE, desde 15/10, 158.745.502.
    #[test]
    fn eleitorado_divergente_alerta_uma_vez_por_topico() {
        let t = TseResultados::novo(Opcoes::default());
        t.definir_eleitorado(HashMap::from([("br-c0001".to_string(), 157_563_461), ("df-c0001".to_string(), 2_258_995)]));
        let x = t.conferir_eleitorado("br-c0001", 158_745_502).expect("eleitorado divergente tem de alertar");
        assert!(x.contains("157563461") && x.contains("158745502") && x.contains("1182041"), "{x}");
        assert_eq!(t.conferir_eleitorado("br-c0001", 158_745_502), None, "um alerta por tópico");
        assert_eq!(t.conferir_eleitorado("df-c0001", 2_258_995), None, "igual: sem alerta");
        assert_eq!(t.conferir_eleitorado("to-c0001", 1_182_041), None, "tópico sem eleitorado no catálogo");
        assert_eq!(t.conferir_eleitorado("df-c0001", 0), None, "TSE ainda sem eleitorado (te 0)");
    }

    #[test]
    fn intervalo_fora_da_faixa_recusado() {
        assert!(Opcoes::default().validar().is_ok());
        assert!(Opcoes { intervalo_s: 29.0, max_topicos: 1 }.validar().is_err());
        assert!(Opcoes { intervalo_s: 61.0, max_topicos: 1 }.validar().is_err());
        assert!(Opcoes { intervalo_s: 60.0, max_topicos: 0 }.validar().is_err());
    }

    #[test]
    fn resumo_ordena_pelos_votos_do_arquivo() {
        let v: Value = serde_json::from_str(r#"{"ele":"6257","t":"1","tf":"n","and":"p","md":"s","dg":"04/10/2026","hg":"20:41:10","carg":[{"cd":"1","agr":[
            {"par":[{"cand":[{"n":"13","seq":"2","vap":"900","pvap":"45,00","dvt":"Válido","e":"s"}]}]},
            {"par":[{"cand":[{"n":"22","seq":"1","vap":"1000","pvapn":"50.000000000","dvt":"Válido","e":"s"}]}]},
            {"par":[{"cand":[{"n":"30","seq":"3","vap":"100","dvt":"Válido","e":"n"}]}]}]}],
            "s":{"ts":"10","st":"4","sni":"1"},"e":{"te":"5000","c":"2100","a":"300"},"v":{"vb":"50","vn":"40","vnt":"10","vv":"2000"}}"#).unwrap();
        let o = Oficial::de_json(&v);
        let r = resumo(&o);
        assert_eq!(r["geracao"], "04/10/2026 20:41:10");
        assert_eq!((r["secoes"]["totalizadas"].as_u64(), r["secoes"]["nao_instaladas"].as_u64()), (Some(4), Some(1)));
        let c = r["cand"].as_array().unwrap();
        assert_eq!(c.iter().map(|x| x["n"].as_u64().unwrap()).collect::<Vec<_>>(), vec![22, 13, 30]);
        assert_eq!((c[0]["pos"].as_u64(), c[0]["pct"].as_f64(), c[1]["pct"].as_f64()), (Some(1), Some(50.0), Some(45.0)));
        // sem pvap: votos ÷ válidos (100 / 2000)
        assert_eq!(c[2]["pct"].as_f64(), Some(5.0));
    }

    #[test]
    fn resumo_ignora_seq_que_nao_e_a_ordem_do_arquivo() {
        // UF do 3220: seq é a ordem nacional (16 tem seq 8 com menos votos que 27 e 21)
        let v: Value = serde_json::from_str(r#"{"carg":[{"cd":"1","agr":[{"par":[{"cand":[{"n":"16","seq":"8","vap":"71"},{"n":"27","seq":"9","vap":"215"},{"n":"21","seq":"10","vap":"140"},{"n":"22","seq":"1","vap":"300"}]}]}]}],"s":{},"e":{},"v":{}}"#).unwrap();
        let ns: Vec<u64> = resumo(&Oficial::de_json(&v))["cand"].as_array().unwrap().iter().map(|x| x["n"].as_u64().unwrap()).collect();
        assert_eq!(ns, vec![22, 27, 21, 16]);
    }

    #[test]
    fn resumo_empate_vai_para_o_mais_idoso() {
        let v: Value = serde_json::from_str(r#"{"carg":[{"cd":"11","agr":[{"par":[{"cand":[{"n":"12","seq":"1","vap":"30","dt":"01/01/1990"},{"n":"45","seq":"2","vap":"30","dt":"15/06/1961"}]}]}]}],"s":{},"e":{},"v":{}}"#).unwrap();
        let ns: Vec<u64> = resumo(&Oficial::de_json(&v))["cand"].as_array().unwrap().iter().map(|x| x["n"].as_u64().unwrap()).collect();
        assert_eq!(ns, vec![45, 12]);
    }

    #[test]
    fn resumo_sem_seq_ordena_pelos_votos() {
        let v: Value = serde_json::from_str(r#"{"carg":[{"cd":"11","agr":[{"par":[{"cand":[{"n":"15","vap":"10"},{"n":"12","vap":"30"},{"n":"10","vap":"30"}]}]}]}],"s":{},"e":{},"v":{}}"#).unwrap();
        let r = resumo(&Oficial::de_json(&v));
        let ns: Vec<u64> = r["cand"].as_array().unwrap().iter().map(|x| x["n"].as_u64().unwrap()).collect();
        assert_eq!(ns, vec![10, 12, 15]);
    }

    #[test]
    fn inscricao_liga_e_desliga_e_respeita_o_limite() {
        let tse = TseResultados::novo(Opcoes { intervalo_s: 30.0, max_topicos: 1 });
        tse.definir_urls(HashMap::from([("br-c0001".to_string(), ("http://x/br".to_string(), "br-c0001-e006257-u.json".to_string())),
            ("sp-c0001".to_string(), ("http://x/sp".to_string(), "sp-c0001-e006257-u.json".to_string()))]));
        assert_eq!(tse.inscrever("xx-c0001").err(), Some("topico_desconhecido"));
        let (a, rx) = tse.inscrever("br-c0001").unwrap();
        let q = rx.borrow().clone().unwrap();
        let m: Value = serde_json::from_str(q.instantaneo.as_str()).unwrap();
        assert_eq!((m["tipo"].as_str(), m["estado"].as_str(), m["instantaneo"].as_bool(), m["topico"].as_str()), (Some("tse"), Some("aguardando"), Some(true), Some("tse:br-c0001")));
        // o segundo tópico não tem vaga (max 1): indisponível com motivo; ganha a vaga quando o primeiro fica sem inscritos
        let (b, rx2) = tse.inscrever("sp-c0001").unwrap();
        let m: Value = serde_json::from_str(rx2.borrow().clone().unwrap().atualizacao.as_str()).unwrap();
        assert_eq!((m["estado"].as_str(), m["motivo"].as_str()), (Some("indisponivel"), Some("limite_topicos")));
        let s = tse.saude_json(0);
        assert_eq!((s["topicos_ativos"].as_u64(), s["esperando_vaga"].as_u64()), (Some(1), Some(1)));
        drop(a);
        let m: Value = serde_json::from_str(rx2.borrow().clone().unwrap().atualizacao.as_str()).unwrap();
        assert_eq!(m["estado"].as_str(), Some("aguardando"));
        let s = tse.saude_json(0);
        assert_eq!((s["topicos_ativos"].as_u64(), s["esperando_vaga"].as_u64(), s["inscritos"].as_u64()), (Some(1), Some(0), Some(1)));
        drop(b);
        assert_eq!(tse.saude_json(0)["topicos_ativos"].as_u64(), Some(0));
    }

    /// M-020: a trava lê as seções do último arquivo do tópico; sem arquivo lido, nada (vale o último valor da trava).
    #[test]
    fn secoes_do_ultimo_arquivo_para_a_trava() {
        let tse = TseResultados::novo(Opcoes::default());
        tse.definir_urls(HashMap::from([("br-c0001".to_string(), ("http://x/br".to_string(), "br-c0001-e006258-u.json".to_string()))]));
        assert_eq!(tse.secoes_de("br-c0001"), None, "nem inscrito");
        let (_insc, _rx) = tse.inscrever("br-c0001").unwrap();
        assert_eq!(tse.secoes_de("br-c0001"), None, "inscrito, sem arquivo lido");
        let v: Value = serde_json::from_str(r#"{"carg":[],"s":{"ts":"499248","st":"49925","sni":"0"},"e":{},"v":{}}"#).unwrap();
        tse.t.lock().unwrap().get_mut("br-c0001").unwrap().dados = Some(resumo(&Oficial::de_json(&v)));
        assert_eq!(tse.secoes_de("br-c0001"), Some((499_248, 49_925)));
        assert_eq!(tse.secoes_de("sp-c0001"), None);
    }
}
