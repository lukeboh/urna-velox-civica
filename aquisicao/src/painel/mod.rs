//! **uv-painel** — painel de monitoramento e gerenciamento dos nós (`docs/operacao/PAINEL.md`).
//!
//! Um processo por nó, **separado do `uv-servico`**: lê a `saude.json`, o `/proc`, o `no.env` e o `federacao.json` do
//! próprio nó e conversa com o painel do outro nó pela rede privada (pedidos assinados com HMAC). Uma queda ou um erro
//! do painel não atinge a coleta nem a totalização.
//!
//! Acesso: login com o Google (OpenID Connect) e uma lista fechada de e-mails (`UV_PAINEL_EMAILS`). As mudanças passam
//! pelo `uv-painel-acao` (root, ações nomeadas: instalar o `no.env` validado, iniciar/parar/reiniciar o serviço,
//! travar/destravar o atualizador) e ficam na auditoria (`auditoria.jsonl`).

pub mod amostras;
pub mod config_no;
pub mod rotas;
pub mod seguranca;

use anyhow::{bail, Context, Result};
use axum::body::{Body, Bytes};
use axum::extract::{Path as Caminho, State};
use axum::http::{header, HeaderMap, Method, StatusCode, Uri};
use axum::response::Response;
use axum::routing::{get, post};
use axum::Router;
use seguranca::{agora_s, Chaves, Sessao};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};
use std::io::Write;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering::SeqCst};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub const VERSAO: &str = env!("CARGO_PKG_VERSION");

const PAGINA: &str = include_str!("pagina/index.html");
const APP_JS: &str = include_str!("pagina/app.js");
const ESTILO: &str = include_str!("pagina/estilo.css");
const TEMA_JS: &str = include_str!("pagina/tema.js");
/// A fonte e o ícone da página de resultados (mesma identidade visual; `site/dist`, gerado pelo `site/build.mjs`).
const FONTE: &[u8] = include_bytes!("../../../site/dist/archivo.woff2");
const ICONE: &str = include_str!("../../../site/dist/icone.svg");

const COOKIE_SESSAO: &str = "uv_painel";
const COOKIE_LOGIN: &str = "uv_painel_login";

#[derive(Clone, Debug)]
pub struct Config {
    pub escuta: SocketAddr,
    /// id deste nó (o mesmo do `federacao.json`)
    pub no_id: String,
    /// pasta de saída do `uv-servico` (onde está a `saude.json`)
    pub saida: PathBuf,
    pub no_env: PathBuf,
    pub federacao: Option<PathBuf>,
    /// painéis dos outros nós (`http://10.0.1.11:8095`)
    pub pares: Vec<String>,
    /// endereço público do painel, sem barra no fim (`https://dominio`); o retorno do Google é `<url>/painel/oauth/retorno`
    pub url_publica: String,
    pub google_id: String,
    pub google_segredo: String,
    pub google_autorizacao: String,
    pub google_token: String,
    pub emails: Vec<String>,
    pub segredo: String,
    /// pasta do painel: auditoria e arquivos à espera do `uv-painel-acao`
    pub estado: PathBuf,
    /// comando do ajudante de ações (`sudo -n /opt/urna-velox/bin/uv-painel-acao`); vazio = modo local (edita o
    /// `no.env` direto e não mexe em serviço nenhum: para testes na estação)
    pub acao: Vec<String>,
    pub intervalo_s: f64,
    /// unidades systemd mostradas na página
    pub servicos: Vec<String>,
    /// quantas amostras guardar (padrão: 2 h a cada 5 s)
    pub capacidade: usize,
}

pub struct Painel {
    pub cfg: Config,
    chaves: Chaves,
    am: Mutex<amostras::Amostrador>,
    http: reqwest::Client,
    /// id do nó → URL do painel dele (aprendido nas visões)
    pares_id: Mutex<HashMap<String, String>>,
    /// uma descoberta de endereços por vez
    descobrindo: AtomicBool,
}

impl Painel {
    pub fn novo(cfg: Config) -> Result<Arc<Painel>> {
        let chaves = Chaves::de_segredo(&cfg.segredo)?;
        if cfg.emails.is_empty() { bail!("UV_PAINEL_EMAILS vazio: ninguém conseguiria entrar"); }
        if cfg.google_id.is_empty() || cfg.google_segredo.is_empty() { bail!("faltam UV_PAINEL_GOOGLE_ID e UV_PAINEL_GOOGLE_SEGREDO"); }
        if !(cfg.url_publica.starts_with("https://") || cfg.url_publica.starts_with("http://localhost") || cfg.url_publica.starts_with("http://127.0.0.1")) {
            bail!("--url-publica tem de ser https:// (http só para localhost)");
        }
        std::fs::create_dir_all(cfg.estado.join("pendente")).with_context(|| format!("criando {}", cfg.estado.display()))?;
        let http = reqwest::Client::builder().timeout(Duration::from_secs(8)).connect_timeout(Duration::from_secs(3)).build()?;
        let am = Mutex::new(amostras::Amostrador::novo(&cfg.saida, cfg.capacidade));
        Ok(Arc::new(Painel { cfg, chaves, am, http, pares_id: Mutex::new(HashMap::new()), descobrindo: AtomicBool::new(false) }))
    }

    fn seguro_cookie(&self) -> &'static str { if self.cfg.url_publica.starts_with("https://") { "; Secure" } else { "" } }

    fn redirect_uri(&self) -> String { format!("{}/painel/oauth/retorno", self.cfg.url_publica) }

    fn sessao(&self, h: &HeaderMap) -> Option<Sessao> {
        cookie(h, COOKIE_SESSAO).and_then(|t| self.chaves.ler_sessao(&t, agora_s(), &self.cfg.emails))
    }

    /// Sessão + CSRF + origem: obrigatórios em tudo o que muda algo.
    fn sessao_para_mudar(&self, h: &HeaderMap) -> Result<Sessao, Response> {
        let s = self.sessao(h).ok_or_else(|| erro(StatusCode::UNAUTHORIZED, "sessao", "entre de novo"))?;
        let csrf = h.get("x-uv-csrf").and_then(|v| v.to_str().ok()).unwrap_or("");
        if !self.chaves.conferir_csrf(&s, csrf) { return Err(erro(StatusCode::FORBIDDEN, "csrf", "recarregue a página")); }
        if let Some(o) = h.get(header::ORIGIN).and_then(|v| v.to_str().ok()) {
            if o != origem(&self.cfg.url_publica) { return Err(erro(StatusCode::FORBIDDEN, "origem", "origem diferente do painel")); }
        }
        Ok(s)
    }

    pub fn auditar(&self, operador: &str, no: &str, acao: &str, detalhe: Value) {
        let l = json!({"t": seguranca::agora_s(), "quando": crate::texto_hora(crate::agora_brasilia()), "operador": operador, "no": no, "acao": acao, "detalhe": detalhe});
        let r = std::fs::OpenOptions::new().create(true).append(true).open(self.cfg.estado.join("auditoria.jsonl"))
            .and_then(|mut f| writeln!(f, "{l}"));
        if let Err(e) = r { eprintln!("uv-painel: auditoria: {e}"); }
        eprintln!("uv-painel: {l}");
    }

    fn auditoria(&self, n: usize) -> Value {
        let t = std::fs::read_to_string(self.cfg.estado.join("auditoria.jsonl")).unwrap_or_default();
        let v: Vec<Value> = t.lines().rev().take(n).filter_map(|l| serde_json::from_str(l).ok()).collect();
        json!(v)
    }

    // -------------------------------------------------------------- dados deste nó

    fn visao_local(&self, desde: u64) -> Value {
        let a = self.am.lock().unwrap();
        json!({"id": self.cfg.no_id, "alcancavel": true, "painel": VERSAO, "resumo": a.resumo(), "serie": a.colunas(desde),
            "intervalo_s": self.cfg.intervalo_s})
    }

    fn saude_local(&self) -> Value { self.am.lock().unwrap().saude_atual.clone().unwrap_or(Value::Null) }

    fn config_local(&self) -> Value {
        let texto = std::fs::read_to_string(&self.cfg.no_env).unwrap_or_default();
        let valores = config_no::ler(&texto);
        let fed = self.cfg.federacao.as_ref().map(|p| std::fs::read_to_string(p).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok()));
        let servicos: Vec<Value> = self.cfg.servicos.iter().map(|u| json!({"unidade": u, "estado": estado_unidade(u)})).collect();
        let trava = self.cfg.no_env.parent().map(|d| d.join("controle-manual").exists()).unwrap_or(false);
        json!({"no": self.cfg.no_id, "arquivo": self.cfg.no_env, "existe": self.cfg.no_env.exists(), "no_env": config_no::descrever(&valores),
            "federacao": fed.flatten(), "servicos": servicos, "controle_manual": trava, "modo_local": self.cfg.acao.is_empty(),
            "auditoria": self.auditoria(25)})
    }

    async fn acao(&self, args: Vec<String>) -> Result<String> {
        if self.cfg.acao.is_empty() { bail!("modo local: este painel não mexe em serviços (sem --acao)"); }
        let mut cmd = std::process::Command::new(&self.cfg.acao[0]);
        cmd.args(&self.cfg.acao[1..]).args(&args);
        let o = tokio::time::timeout(Duration::from_secs(120), tokio::task::spawn_blocking(move || cmd.output())).await
            .context("a ação passou de 120 s")???;
        let saida = format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr)).trim().to_string();
        if !o.status.success() { bail!("uv-painel-acao {}: {}", args.join(" "), if saida.is_empty() { o.status.to_string() } else { saida }); }
        Ok(saida)
    }

    async fn mudar_config_local(&self, operador: &str, corpo: &Value) -> Result<Value, (StatusCode, Value)> {
        let ruim = |m: String| (StatusCode::BAD_REQUEST, json!({"erro": "invalido", "mensagem": m}));
        let alt: BTreeMap<String, String> = match corpo["alteracoes"].as_object() {
            Some(o) => o.iter().map(|(k, v)| (k.clone(), match v { Value::String(s) => s.trim().to_string(), Value::Null => String::new(), o => o.to_string() })).collect(),
            None => return Err(ruim("faltou `alteracoes`".into())),
        };
        let texto = std::fs::read_to_string(&self.cfg.no_env).map_err(|e| ruim(format!("lendo {}: {e}", self.cfg.no_env.display())))?;
        let atual = config_no::ler(&texto);
        let mudou: Vec<String> = alt.iter().filter(|(k, v)| atual.get(*k).map(String::as_str).unwrap_or("") != v.as_str()).map(|(k, _)| k.clone()).collect();
        let reiniciar = corpo["reiniciar"].as_bool().unwrap_or(false);
        if mudou.is_empty() && !reiniciar { return Ok(json!({"ok": true, "mudou": [], "mensagem": "nada mudou"})); }
        if let Some(c) = config_no::confirmacao_exigida(&atual, &alt) {
            if corpo["confirmacao"].as_str().map(str::trim) != Some(c) {
                return Err((StatusCode::CONFLICT, json!({"erro": "confirmacao", "confirmacao": c,
                    "mensagem": format!("estas mudanças pedem a confirmação \"{c}\"")})));
            }
        }
        let base_arq = rotas::ler(&self.cfg.no_env).0.base_arquivos;
        let novo = config_no::aplicar_com(&texto, &alt, base_arq.as_deref()).map_err(|e| ruim(e.to_string()))?;
        let det = json!({"antes": mudou.iter().map(|k| (k.clone(), atual.get(k).cloned().unwrap_or_default())).collect::<BTreeMap<_, _>>(),
            "depois": mudou.iter().map(|k| (k.clone(), alt[k].clone())).collect::<BTreeMap<_, _>>(), "reiniciar": reiniciar});
        let falha = |e: anyhow::Error| (StatusCode::INTERNAL_SERVER_ERROR, json!({"erro": "acao", "mensagem": e.to_string()}));
        let mut saida = String::new();
        if !mudou.is_empty() {
            if self.cfg.acao.is_empty() {
                gravar_atomico(&self.cfg.no_env, &novo).map_err(falha)?;
            } else {
                let p = self.cfg.estado.join("pendente").join(format!("no.env.{}", amostras::agora_ms()));
                std::fs::write(&p, &novo).map_err(|e| falha(e.into()))?;
                let r = self.acao(vec!["aplicar-no-env".into(), p.to_string_lossy().into_owned()]).await;
                let _ = std::fs::remove_file(&p);
                saida = r.map_err(|e| { self.auditar(operador, &self.cfg.no_id, "config_falhou", json!({"erro": e.to_string(), "mudancas": det})); falha(e) })?;
            }
        }
        let mut reiniciado = false;
        if reiniciar && !self.cfg.acao.is_empty() {
            let r = self.acao(vec!["servico".into(), "reiniciar".into()]).await;
            reiniciado = r.is_ok();
            match r { Ok(s) => { saida.push('\n'); saida.push_str(&s); } Err(e) => { saida.push_str(&format!("\nreinício falhou: {e}")); } }
        }
        self.auditar(operador, &self.cfg.no_id, "config", json!({"mudancas": det, "reiniciado": reiniciado}));
        Ok(json!({"ok": true, "mudou": mudou, "reiniciado": reiniciado, "saida": saida.trim(),
            "mensagem": if reiniciar && self.cfg.acao.is_empty() { "modo local: o arquivo foi gravado, nenhum serviço reiniciado" } else { "" }}))
    }

    async fn servico_local(&self, operador: &str, corpo: &Value) -> Result<Value, (StatusCode, Value)> {
        let acao = corpo["acao"].as_str().unwrap_or("");
        let args: Vec<String> = match acao {
            "iniciar" | "parar" | "reiniciar" => vec!["servico".into(), acao.into()],
            "travar" | "destravar" => vec![acao.into()],
            _ => return Err((StatusCode::BAD_REQUEST, json!({"erro": "invalido", "mensagem": "ação desconhecida"}))),
        };
        let base = config_no::ler(&std::fs::read_to_string(&self.cfg.no_env).unwrap_or_default()).get("BASE").cloned().unwrap_or_default();
        let exigida = match acao {
            "iniciar" | "reiniciar" if config_no::eh_tse(&base) => Some("CONFIRMO TSE"),
            "iniciar" | "reiniciar" | "parar" => Some("CONFIRMO"),
            _ => None,
        };
        if let Some(c) = exigida {
            if corpo["confirmacao"].as_str().map(str::trim) != Some(c) {
                return Err((StatusCode::CONFLICT, json!({"erro": "confirmacao", "confirmacao": c, "mensagem": format!("\"{acao}\" pede a confirmação \"{c}\"")})));
            }
        }
        match self.acao(args).await {
            Ok(s) => { self.auditar(operador, &self.cfg.no_id, acao, json!({"ok": true})); Ok(json!({"ok": true, "saida": s})) }
            Err(e) => {
                self.auditar(operador, &self.cfg.no_id, acao, json!({"ok": false, "erro": e.to_string()}));
                Err((StatusCode::INTERNAL_SERVER_ERROR, json!({"erro": "acao", "mensagem": e.to_string()})))
            }
        }
    }

    // -------------------------------------------------------------- endereços do site de resultados

    fn rotas_local(&self) -> Value { rotas::estado(&self.cfg.no_env, &self.saude_local()) }

    async fn descobrir_local(&self, operador: &str, c: &Value) -> Result<Value, (StatusCode, Value)> {
        let ruim = |st: StatusCode, m: String| (st, json!({"erro": "descoberta", "mensagem": m}));
        if self.saude_local()["rede"]["pausa_restante_s"].as_f64().unwrap_or(0.0) > 0.0 {
            return Err(ruim(StatusCode::SERVICE_UNAVAILABLE, "o serviço deste nó está em pausa por HTTP 429: nenhuma requisição ao servidor até ela acabar".into()));
        }
        let ne = config_no::ler(&std::fs::read_to_string(&self.cfg.no_env).unwrap_or_default());
        let campo = |k: &str, env: &str, padrao: &str| -> String {
            c[k].as_str().map(str::trim).filter(|v| !v.is_empty()).map(String::from)
                .or_else(|| ne.get(env).cloned()).unwrap_or_else(|| padrao.to_string())
        };
        let url = c["url"].as_str().unwrap_or("").trim().to_string();
        if url.is_empty() { return Err(ruim(StatusCode::BAD_REQUEST, "informe a URL de resultados".into())); }
        let pleito: u32 = campo("pleito", "PLEITO", "0").parse().map_err(|_| ruim(StatusCode::BAD_REQUEST, "pleito inválido".into()))?;
        let mut r = match &c["rotas"] {
            Value::Null => rotas::ler(&self.cfg.no_env).0,
            v => serde_json::from_value::<crate::tse::Rotas>(v.clone()).map_err(|e| ruim(StatusCode::BAD_REQUEST, format!("modelos inválidos: {e}")))?,
        };
        if let Some(b) = c["base_arquivos"].as_str() {
            let b = b.trim().trim_end_matches('/');
            r.base_arquivos = (!b.is_empty()).then(|| b.to_string());
        }
        let pedido = rotas::Pedido { url: url.clone(), ambiente: campo("ambiente", "AMBIENTE", "oficial"), ciclo: campo("ciclo", "CICLO", "ele2026"), pleito, rotas: r };
        if self.descobrindo.swap(true, SeqCst) { return Err(ruim(StatusCode::TOO_MANY_REQUESTS, "já há uma descoberta em andamento neste nó".into())); }
        let v = rotas::descobrir(pedido).await;
        self.descobrindo.store(false, SeqCst);
        self.auditar(operador, &self.cfg.no_id, "descoberta", json!({"url": url, "ok": v["ok"], "bu_ok": v["bu_ok"], "requisicoes": v["requisicoes"]}));
        Ok(v)
    }

    async fn aplicar_rotas_local(&self, operador: &str, c: &Value) -> Result<Value, (StatusCode, Value)> {
        let ruim = |m: String| (StatusCode::BAD_REQUEST, json!({"erro": "invalido", "mensagem": m}));
        let falha = |e: anyhow::Error| (StatusCode::INTERNAL_SERVER_ERROR, json!({"erro": "acao", "mensagem": e.to_string()}));
        let base = c["base"].as_str().unwrap_or("").trim().trim_end_matches('/').to_string();
        config_no::validar("BASE", &base).map_err(|e| ruim(e.to_string()))?;
        let r: crate::tse::Rotas = match &c["rotas"] {
            Value::Null => Default::default(),
            v => serde_json::from_value(v.clone()).map_err(|e| ruim(format!("modelos inválidos: {e}")))?,
        };
        r.validar(&base).map_err(|e| ruim(e.to_string()))?;
        // o acesso do nó (borda/interno) tem de valer também com a base e a base dos arquivos novas
        {
            let mut ne = config_no::ler(&std::fs::read_to_string(&self.cfg.no_env).unwrap_or_default());
            ne.insert("BASE".into(), base.clone());
            config_no::validar_conjunto(&ne, r.base_arquivos.as_deref()).map_err(|e| ruim(e.to_string()))?;
        }
        let exigida = if config_no::eh_tse(&base) || r.base_arquivos.as_deref().is_some_and(config_no::eh_tse) { "CONFIRMO TSE" } else { "CONFIRMO" };
        if c["confirmacao"].as_str().map(str::trim) != Some(exigida) {
            return Err((StatusCode::CONFLICT, json!({"erro": "confirmacao", "confirmacao": exigida,
                "mensagem": format!("mudar de onde o nó coleta pede a confirmação \"{exigida}\"")})));
        }
        let arq = rotas::arquivo(&self.cfg.no_env);
        let (atual, _) = rotas::ler(&self.cfg.no_env);
        let mut saida = String::new();
        let muda_rotas = r != atual || (r.padrao() && arq.exists());
        if muda_rotas {
            if self.cfg.acao.is_empty() {
                if r.padrao() { let _ = std::fs::remove_file(&arq); }
                else { gravar_atomico(&arq, &(serde_json::to_string_pretty(&r).unwrap_or_default() + "\n")).map_err(falha)?; }
            } else if r.padrao() {
                saida = self.acao(vec!["remover-rotas".into()]).await.map_err(falha)?;
            } else {
                let p = self.cfg.estado.join("pendente").join(format!("rotas_tse.json.{}", amostras::agora_ms()));
                std::fs::write(&p, serde_json::to_string_pretty(&r).unwrap_or_default() + "\n").map_err(|e| falha(e.into()))?;
                let res = self.acao(vec!["aplicar-rotas".into(), p.to_string_lossy().into_owned(), base.clone()]).await;
                let _ = std::fs::remove_file(&p);
                saida = res.map_err(falha)?;
            }
        }
        let mut alt = serde_json::Map::new();
        alt.insert("BASE".into(), json!(base));
        for k in ["AMBIENTE", "CICLO", "PLEITO"] {
            if let Some(v) = c[k].as_str().map(str::trim).filter(|v| !v.is_empty()) { alt.insert(k.into(), json!(v)); }
        }
        let corpo = json!({"alteracoes": alt, "reiniciar": c["reiniciar"].as_bool().unwrap_or(false), "confirmacao": exigida});
        let mut v = self.mudar_config_local(operador, &corpo).await?;
        self.auditar(operador, &self.cfg.no_id, "rotas", json!({"base": base, "rotas_mudaram": muda_rotas, "rotas": r}));
        v["rotas_mudaram"] = json!(muda_rotas);
        if !saida.is_empty() { v["saida"] = json!(format!("{}\n{}", saida, v["saida"].as_str().unwrap_or("")).trim().to_string()); }
        Ok(v)
    }

    // -------------------------------------------------------------- outros nós

    async fn par(&self, base: &str, metodo: Method, caminho: &str, corpo: Option<Vec<u8>>) -> Result<(StatusCode, Value)> {
        let b = corpo.clone().unwrap_or_default();
        let ass = self.chaves.assinar_par(metodo.as_str(), caminho, &b, agora_s());
        // a descoberta de endereços pode levar até ~20 requisições ao site: espera mais por ela
        let prazo = Duration::from_secs(if caminho.ends_with("/descobrir") { 150 } else { 8 });
        let mut r = self.http.request(metodo, format!("{base}{caminho}")).header(seguranca::CABECALHO_PAR, ass).timeout(prazo);
        if let Some(c) = corpo { r = r.header(header::CONTENT_TYPE, "application/json").body(c); }
        let resp = r.send().await?;
        let st = StatusCode::from_u16(resp.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
        let v: Value = serde_json::from_slice(&resp.bytes().await?).unwrap_or(Value::Null);
        Ok((st, v))
    }

    async fn visao(&self, desde: u64) -> Value {
        let mut nos = vec![self.visao_local(desde)];
        let futs = self.cfg.pares.iter().map(|p| {
            let p = p.clone();
            async move { (p.clone(), self.par(&p, Method::GET, &format!("/painel/interno/visao?desde={desde}"), None).await) }
        });
        for (url, r) in futures::future::join_all(futs).await {
            match r {
                Ok((StatusCode::OK, v)) if v["id"].is_string() => {
                    self.pares_id.lock().unwrap().insert(v["id"].as_str().unwrap().to_string(), url.clone());
                    nos.push(v);
                }
                Ok((st, v)) => nos.push(json!({"id": self.nome_par(&url), "url": url, "alcancavel": false, "erro": format!("HTTP {st}: {}", v["mensagem"].as_str().or(v["erro"].as_str()).unwrap_or(""))})),
                Err(e) => nos.push(json!({"id": self.nome_par(&url), "url": url, "alcancavel": false, "erro": format!("{e:#}")})), // com as causas: conexão recusada × tempo esgotado (B-019)
            }
        }
        json!({"gerado_em": amostras::agora_ms(), "local": self.cfg.no_id, "nos": nos})
    }

    fn nome_par(&self, url: &str) -> String {
        self.pares_id.lock().unwrap().iter().find(|(_, u)| *u == url).map(|(k, _)| k.clone()).unwrap_or_else(|| url.to_string())
    }

    /// URL do painel que cuida do nó `id` (None = este nó).
    async fn onde(&self, id: &str) -> Result<Option<String>, Response> {
        if id == self.cfg.no_id { return Ok(None); }
        if let Some(u) = self.pares_id.lock().unwrap().get(id) { return Ok(Some(u.clone())); }
        let _ = self.visao(u64::MAX).await;
        self.pares_id.lock().unwrap().get(id).cloned().map(Some).ok_or_else(|| erro(StatusCode::NOT_FOUND, "no", "nó desconhecido ou fora do alcance"))
    }
}

// ------------------------------------------------------------------ utilitários HTTP

fn cabecalhos(b: axum::http::response::Builder) -> axum::http::response::Builder {
    b.header("content-security-policy", "default-src 'self'; img-src 'self' data:; style-src 'self'; script-src 'self'; connect-src 'self'; frame-ancestors 'none'; base-uri 'none'; form-action 'self'")
        .header("x-content-type-options", "nosniff").header("referrer-policy", "no-referrer").header("x-frame-options", "DENY")
        .header(header::CACHE_CONTROL, "no-store")
}

fn resposta(st: StatusCode, tipo: &str, corpo: impl Into<Body>) -> Response {
    cabecalhos(Response::builder().status(st)).header(header::CONTENT_TYPE, tipo).body(corpo.into()).unwrap()
}

fn json_r(st: StatusCode, v: &Value) -> Response { resposta(st, "application/json; charset=utf-8", v.to_string()) }

fn erro(st: StatusCode, codigo: &str, msg: &str) -> Response { json_r(st, &json!({"erro": codigo, "mensagem": msg})) }

fn redirecionar(destino: &str, cookies: &[String]) -> Response {
    let mut b = cabecalhos(Response::builder().status(StatusCode::SEE_OTHER)).header(header::LOCATION, destino);
    for c in cookies { b = b.header(header::SET_COOKIE, c); }
    b.body(Body::empty()).unwrap()
}

pub fn cookie(h: &HeaderMap, nome: &str) -> Option<String> {
    h.get_all(header::COOKIE).iter().filter_map(|v| v.to_str().ok()).flat_map(|v| v.split(';'))
        .filter_map(|p| p.trim().split_once('=')).find(|(k, _)| *k == nome).map(|(_, v)| v.to_string())
}

/// `https://dominio:porta` de uma URL.
pub fn origem(url: &str) -> String {
    let (esq, resto) = url.split_once("://").unwrap_or(("https", url));
    format!("{esq}://{}", resto.split('/').next().unwrap_or(""))
}

fn query(u: &Uri) -> Vec<(String, String)> { seguranca::parametros(u.query().unwrap_or("")) }

fn param(u: &Uri, k: &str) -> Option<String> { query(u).into_iter().find(|(a, _)| a == k).map(|(_, v)| v) }

fn estado_unidade(u: &str) -> String {
    match std::process::Command::new("systemctl").args(["is-active", u]).output() {
        Ok(o) => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        Err(_) => "indisponível".into(),
    }
}

fn gravar_atomico(p: &std::path::Path, texto: &str) -> Result<()> {
    let tmp = p.with_extension(format!("tmp{}", std::process::id()));
    std::fs::write(&tmp, texto)?;
    std::fs::rename(&tmp, p)?;
    Ok(())
}

fn pagina_login(msg: &str) -> Response {
    // "saiu" é informação, não erro: aviso neutro
    let classe = if msg.starts_with("Você saiu") { "aviso info" } else { "aviso" };
    let aviso = if msg.is_empty() { String::new() } else { format!("<p class=\"{classe}\">{}</p>", html(msg)) };
    let corpo = format!("<!doctype html><html lang=\"pt-BR\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
<title>Painel dos nós</title><link rel=\"icon\" href=\"/painel/icone.svg\" type=\"image/svg+xml\">\
<link rel=\"stylesheet\" href=\"/painel/estilo.css\"><script src=\"/painel/tema.js\"></script></head>\
<body class=\"login\"><div class=\"aurora\" aria-hidden=\"true\"></div><main class=\"cartao-login brilho\">\
<button class=\"icone-btn tema-login\" id=\"tema\" type=\"button\" aria-label=\"Trocar tema claro/escuro\">◐</button>\
<div class=\"marca\"><b>Urna <span>Velox</span></b></div><p class=\"sub\">Painel dos nós · acesso restrito</p>{aviso}\
<a class=\"botao primario\" href=\"/painel/entrar\">Entrar com o Google</a></main></body></html>");
    resposta(StatusCode::OK, "text/html; charset=utf-8", corpo)
}

fn html(s: &str) -> String { s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;") }

// ------------------------------------------------------------------ rotas

type P = State<Arc<Painel>>;

async fn raiz() -> Response { redirecionar("/painel/", &[]) }

async fn inicio(State(p): P, h: HeaderMap, u: Uri) -> Response {
    if p.sessao(&h).is_some() { return resposta(StatusCode::OK, "text/html; charset=utf-8", PAGINA); }
    let msg = match param(&u, "erro").as_deref() {
        Some("negado") => "Este e-mail não tem acesso ao painel.",
        Some("expirado") => "O login demorou demais ou foi aberto em outra aba. Tente de novo.",
        Some("google") => "O Google não concluiu o login. Tente de novo.",
        Some("saiu") => "Você saiu do painel.",
        _ => "",
    };
    pagina_login(msg)
}

async fn app_js() -> Response { resposta(StatusCode::OK, "text/javascript; charset=utf-8", APP_JS) }
async fn estilo() -> Response { resposta(StatusCode::OK, "text/css; charset=utf-8", ESTILO) }
async fn tema_js() -> Response { resposta(StatusCode::OK, "text/javascript; charset=utf-8", TEMA_JS) }
async fn icone() -> Response { resposta(StatusCode::OK, "image/svg+xml", ICONE) }
async fn fonte() -> Response {
    let mut r = resposta(StatusCode::OK, "font/woff2", FONTE);
    r.headers_mut().insert(header::CACHE_CONTROL, axum::http::HeaderValue::from_static("public, max-age=86400"));
    r
}
async fn vivo() -> Response { resposta(StatusCode::OK, "text/plain; charset=utf-8", "ok\n") }

async fn entrar(State(p): P) -> Response {
    let (l, t) = p.chaves.novo_login(agora_s());
    let c = format!("{COOKIE_LOGIN}={t}; Path=/painel; HttpOnly; SameSite=Lax; Max-Age={}{}", seguranca::DURACAO_LOGIN_S, p.seguro_cookie());
    redirecionar(&seguranca::url_autorizacao(&p.cfg.google_autorizacao, &p.cfg.google_id, &p.redirect_uri(), &l), &[c])
}

async fn retorno(State(p): P, h: HeaderMap, u: Uri) -> Response {
    let limpa = format!("{COOKIE_LOGIN}=; Path=/painel; HttpOnly; SameSite=Lax; Max-Age=0{}", p.seguro_cookie());
    let q: HashMap<String, String> = query(&u).into_iter().collect();
    let Some(l) = cookie(&h, COOKIE_LOGIN).and_then(|t| p.chaves.ler_login(&t, agora_s())) else {
        return redirecionar("/painel/?erro=expirado", &[limpa]);
    };
    if q.get("state").map(|s| seguranca::iguais(s.as_bytes(), l.s.as_bytes())) != Some(true) {
        p.auditar("?", &p.cfg.no_id, "login_recusado", json!({"motivo": "state"}));
        return redirecionar("/painel/?erro=expirado", &[limpa]);
    }
    let Some(code) = q.get("code").filter(|c| !c.is_empty()) else {
        p.auditar("?", &p.cfg.no_id, "login_recusado", json!({"motivo": "google", "erro": q.get("error")}));
        return redirecionar("/painel/?erro=google", &[limpa]);
    };
    let corpo = seguranca::corpo_token(code, &p.cfg.google_id, &p.cfg.google_segredo, &p.redirect_uri());
    let r = async {
        let resp = p.http.post(&p.cfg.google_token).header(header::CONTENT_TYPE, "application/x-www-form-urlencoded").body(corpo).send().await?;
        let st = resp.status();
        let v: Value = serde_json::from_slice(&resp.bytes().await?).unwrap_or(Value::Null);
        if !st.is_success() { bail!("token: HTTP {st} {}", v["error"].as_str().unwrap_or("")); }
        let id = v["id_token"].as_str().context("resposta sem id_token")?;
        seguranca::validar_id_token(id, &p.cfg.google_id, &l.n, agora_s())
    }.await;
    match r {
        Ok(email) if seguranca::email_permitido(&email, &p.cfg.emails) => {
            p.auditar(&email, &p.cfg.no_id, "login", json!({"ok": true}));
            // Path=/: o Caddy confere esta sessão (forward_auth em /painel/auth/verificar) antes dos controles do serviço
            let s = format!("{COOKIE_SESSAO}={}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}{}", p.chaves.emitir_sessao(&email, agora_s()),
                seguranca::DURACAO_SESSAO_S, p.seguro_cookie());
            redirecionar("/painel/", &[limpa, s])
        }
        Ok(email) => {
            p.auditar(&email, &p.cfg.no_id, "login_recusado", json!({"motivo": "fora_da_lista"}));
            redirecionar("/painel/?erro=negado", &[limpa])
        }
        Err(e) => {
            p.auditar("?", &p.cfg.no_id, "login_recusado", json!({"motivo": "id_token", "erro": e.to_string()}));
            redirecionar("/painel/?erro=google", &[limpa])
        }
    }
}

async fn sair(State(p): P, h: HeaderMap) -> Response {
    if let Some(s) = p.sessao(&h) { p.auditar(&s.e, &p.cfg.no_id, "saida", json!({})); }
    let c = format!("{COOKIE_SESSAO}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0{}", p.seguro_cookie());
    redirecionar("/painel/?erro=saiu", &[c])
}

/// Para o `forward_auth` do Caddy: 200 com a sessão do painel válida (e o e-mail em `X-UV-Operador`, que o Caddy
/// repassa ao serviço), 401 sem ela. Protege, fora do painel, os controles que mudam o serviço (replay e afins).
async fn verificar(State(p): P, h: HeaderMap) -> Response {
    let alvo = h.get("x-forwarded-uri").and_then(|v| v.to_str().ok()).unwrap_or("?").to_string();
    let metodo = h.get("x-forwarded-method").and_then(|v| v.to_str().ok()).unwrap_or("?").to_string();
    match p.sessao(&h) {
        Some(s) => {
            if metodo != "GET" && metodo != "HEAD" { p.auditar(&s.e, &p.cfg.no_id, "controle", json!({"metodo": metodo, "uri": alvo})); }
            let mut r = resposta(StatusCode::OK, "text/plain; charset=utf-8", "ok\n");
            if let Ok(v) = axum::http::HeaderValue::from_str(&s.e) { r.headers_mut().insert("x-uv-operador", v); }
            r
        }
        None => {
            p.auditar("?", &p.cfg.no_id, "controle_recusado", json!({"metodo": metodo, "uri": alvo}));
            json_r(StatusCode::UNAUTHORIZED, &json!({"erro": "sessao", "mensagem": "entre no painel (/painel/) com o Google para usar este controle"}))
        }
    }
}

async fn eu(State(p): P, h: HeaderMap) -> Response {
    match p.sessao(&h) {
        Some(s) => json_r(StatusCode::OK, &json!({"email": s.e, "expira": s.x, "csrf": p.chaves.csrf(&s), "no": p.cfg.no_id, "painel": VERSAO})),
        None => erro(StatusCode::UNAUTHORIZED, "sessao", "entre de novo"),
    }
}

async fn api_visao(State(p): P, h: HeaderMap, u: Uri) -> Response {
    if p.sessao(&h).is_none() { return erro(StatusCode::UNAUTHORIZED, "sessao", "entre de novo"); }
    let desde = param(&u, "desde").and_then(|d| d.parse().ok()).unwrap_or(0);
    json_r(StatusCode::OK, &p.visao(desde).await)
}

async fn api_get_no(p: &Painel, h: &HeaderMap, id: &str, o_que: &str) -> Response {
    if p.sessao(h).is_none() { return erro(StatusCode::UNAUTHORIZED, "sessao", "entre de novo"); }
    match p.onde(id).await {
        Err(r) => r,
        Ok(None) => json_r(StatusCode::OK, &match o_que { "saude" => p.saude_local(), "rotas" => p.rotas_local(), _ => p.config_local() }),
        Ok(Some(url)) => match p.par(&url, Method::GET, &format!("/painel/interno/{o_que}"), None).await {
            Ok((st, v)) => json_r(st, &v),
            Err(e) => erro(StatusCode::BAD_GATEWAY, "par", &e.to_string()),
        },
    }
}

async fn api_saude(State(p): P, h: HeaderMap, Caminho(id): Caminho<String>) -> Response { api_get_no(&p, &h, &id, "saude").await }
async fn api_config(State(p): P, h: HeaderMap, Caminho(id): Caminho<String>) -> Response { api_get_no(&p, &h, &id, "config").await }
async fn api_rotas(State(p): P, h: HeaderMap, Caminho(id): Caminho<String>) -> Response { api_get_no(&p, &h, &id, "rotas").await }

async fn api_mudar(p: &Painel, h: &HeaderMap, id: &str, o_que: &str, corpo: &[u8]) -> Response {
    let s = match p.sessao_para_mudar(h) { Ok(s) => s, Err(r) => return r };
    let Ok(mut v) = serde_json::from_slice::<Value>(corpo) else { return erro(StatusCode::BAD_REQUEST, "invalido", "JSON inválido") };
    match p.onde(id).await {
        Err(r) => r,
        Ok(None) => {
            let r = match o_que {
                "config" => p.mudar_config_local(&s.e, &v).await,
                "rotas" => p.aplicar_rotas_local(&s.e, &v).await,
                "descobrir" => p.descobrir_local(&s.e, &v).await,
                _ => p.servico_local(&s.e, &v).await,
            };
            match r { Ok(v) => json_r(StatusCode::OK, &v), Err((st, v)) => json_r(st, &v) }
        }
        Ok(Some(url)) => {
            v["operador"] = json!(s.e);
            match p.par(&url, Method::POST, &format!("/painel/interno/{o_que}"), Some(v.to_string().into_bytes())).await {
                Ok((st, v)) => json_r(st, &v),
                Err(e) => erro(StatusCode::BAD_GATEWAY, "par", &e.to_string()),
            }
        }
    }
}

async fn api_mudar_config(State(p): P, h: HeaderMap, Caminho(id): Caminho<String>, c: Bytes) -> Response { api_mudar(&p, &h, &id, "config", &c).await }
async fn api_mudar_servico(State(p): P, h: HeaderMap, Caminho(id): Caminho<String>, c: Bytes) -> Response { api_mudar(&p, &h, &id, "servico", &c).await }
async fn api_mudar_rotas(State(p): P, h: HeaderMap, Caminho(id): Caminho<String>, c: Bytes) -> Response { api_mudar(&p, &h, &id, "rotas", &c).await }
async fn api_descobrir(State(p): P, h: HeaderMap, Caminho(id): Caminho<String>, c: Bytes) -> Response { api_mudar(&p, &h, &id, "descobrir", &c).await }

/// Pedido de outro painel: assinatura HMAC sobre método, caminho com a query e corpo.
fn par_ok(p: &Painel, m: &Method, u: &Uri, h: &HeaderMap, corpo: &[u8]) -> bool {
    let a = h.get(seguranca::CABECALHO_PAR).and_then(|v| v.to_str().ok()).unwrap_or("");
    let caminho = u.path_and_query().map(|x| x.as_str()).unwrap_or("");
    p.chaves.conferir_par(a, m.as_str(), caminho, corpo, agora_s())
}

async fn interno_get(State(p): P, m: Method, u: Uri, h: HeaderMap, Caminho(o_que): Caminho<String>) -> Response {
    if !par_ok(&p, &m, &u, &h, b"") { return erro(StatusCode::FORBIDDEN, "par", "assinatura inválida"); }
    match o_que.as_str() {
        "visao" => json_r(StatusCode::OK, &p.visao_local(param(&u, "desde").and_then(|d| d.parse().ok()).unwrap_or(0))),
        "saude" => json_r(StatusCode::OK, &p.saude_local()),
        "config" => json_r(StatusCode::OK, &p.config_local()),
        "rotas" => json_r(StatusCode::OK, &p.rotas_local()),
        _ => erro(StatusCode::NOT_FOUND, "rota", ""),
    }
}

async fn interno_post(State(p): P, m: Method, u: Uri, h: HeaderMap, Caminho(o_que): Caminho<String>, c: Bytes) -> Response {
    if !par_ok(&p, &m, &u, &h, &c) { return erro(StatusCode::FORBIDDEN, "par", "assinatura inválida"); }
    let Ok(v) = serde_json::from_slice::<Value>(&c) else { return erro(StatusCode::BAD_REQUEST, "invalido", "JSON inválido") };
    let op = v["operador"].as_str().unwrap_or("?").to_string();
    let r = match o_que.as_str() {
        "config" => p.mudar_config_local(&op, &v).await,
        "servico" => p.servico_local(&op, &v).await,
        "rotas" => p.aplicar_rotas_local(&op, &v).await,
        "descobrir" => p.descobrir_local(&op, &v).await,
        _ => return erro(StatusCode::NOT_FOUND, "rota", ""),
    };
    match r { Ok(v) => json_r(StatusCode::OK, &v), Err((st, v)) => json_r(st, &v) }
}

pub fn rotas(p: Arc<Painel>) -> Router {
    Router::new()
        .route("/painel", get(raiz))
        .route("/painel/", get(inicio))
        .route("/painel/app.js", get(app_js))
        .route("/painel/estilo.css", get(estilo))
        .route("/painel/tema.js", get(tema_js))
        .route("/painel/icone.svg", get(icone))
        .route("/painel/archivo.woff2", get(fonte))
        .route("/painel/vivo", get(vivo))
        .route("/painel/auth/verificar", get(verificar))
        .route("/painel/entrar", get(entrar))
        .route("/painel/oauth/retorno", get(retorno))
        .route("/painel/sair", post(sair))
        .route("/painel/api/eu", get(eu))
        .route("/painel/api/visao", get(api_visao))
        .route("/painel/api/no/{id}/saude", get(api_saude))
        .route("/painel/api/no/{id}/config", get(api_config).post(api_mudar_config))
        .route("/painel/api/no/{id}/servico", post(api_mudar_servico))
        .route("/painel/api/no/{id}/rotas", get(api_rotas).post(api_mudar_rotas))
        .route("/painel/api/no/{id}/descobrir", post(api_descobrir))
        .route("/painel/interno/{o_que}", get(interno_get).post(interno_post))
        .fallback(|| async { erro(StatusCode::NOT_FOUND, "rota", "") })
        .with_state(p)
}

/// Amostra o nó a cada `intervalo_s` e serve o painel até o processo receber SIGTERM/SIGINT.
pub async fn servir(p: Arc<Painel>) -> Result<()> {
    let pa = p.clone();
    tokio::spawn(async move {
        let mut iv = tokio::time::interval(Duration::from_secs_f64(pa.cfg.intervalo_s.max(1.0)));
        iv.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            iv.tick().await;
            let pb = pa.clone();
            let _ = tokio::task::spawn_blocking(move || pb.am.lock().unwrap().amostrar(amostras::agora_ms())).await;
        }
    });
    let l = tokio::net::TcpListener::bind(p.cfg.escuta).await.with_context(|| format!("escutando {}", p.cfg.escuta))?;
    eprintln!("uv-painel {VERSAO}: nó {} em http://{}/painel/ (público: {}/painel/), pares: {:?}, {}", p.cfg.no_id, l.local_addr()?,
        p.cfg.url_publica, p.cfg.pares, if p.cfg.acao.is_empty() { "modo local (sem ações)" } else { "com ações" });
    axum::serve(l, rotas(p)).with_graceful_shutdown(async {
        let mut t = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).expect("SIGTERM");
        tokio::select! { _ = t.recv() => {}, _ = tokio::signal::ctrl_c() => {} }
    }).await?;
    Ok(())
}

#[cfg(test)]
mod testes;
