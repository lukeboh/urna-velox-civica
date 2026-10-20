//! Ponta a ponta: um "Google" falso (endpoint de token), dois painéis (nó A e nó B) que se enxergam, login aceito e
//! recusado, visão dos dois nós, mudança de configuração no outro nó (com CSRF e confirmação) e pedidos sem assinatura.

use super::*;
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};

static N: AtomicU64 = AtomicU64::new(0);

/// Corpo JSON da resposta (o reqwest do projeto não tem o recurso "json").
trait Js { async fn js(self) -> Value; }
impl Js for reqwest::Response {
    async fn js(self) -> Value { serde_json::from_slice(&self.bytes().await.unwrap()).unwrap() }
}

fn pasta() -> PathBuf {
    let p = std::env::temp_dir().join(format!("uv-painel-teste-{}-{}", std::process::id(), N.fetch_add(1, Relaxed)));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(p.join("saida")).unwrap();
    p
}

/// O que o falso Google devolve no próximo token: (e-mail, nonce).
type Proximo = Arc<Mutex<(String, String)>>;

async fn google_falso(prox: Proximo) -> String {
    let app = Router::new().route("/token", post(move |corpo: Bytes| {
        let prox = prox.clone();
        async move {
            let q: HashMap<String, String> = seguranca::parametros(std::str::from_utf8(&corpo).unwrap()).into_iter().collect();
            if q.get("code").map(String::as_str) != Some("codigo-bom") || q.get("client_secret").map(String::as_str) != Some("segredo-google") {
                return json_r(StatusCode::BAD_REQUEST, &json!({"error": "invalid_grant"}));
            }
            let (email, nonce) = prox.lock().unwrap().clone();
            let c = json!({"iss": "https://accounts.google.com", "aud": "cliente.apps", "exp": agora_s() + 300, "nonce": nonce,
                "email": email, "email_verified": true});
            let tok = format!("e30.{}.x", seguranca::b64(c.to_string().as_bytes()));
            json_r(StatusCode::OK, &json!({"id_token": tok}))
        }
    }));
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let a = l.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(l, app).await.unwrap() });
    format!("http://{a}")
}

async fn painel(no: &str, google: &str, pares: Vec<String>, l: tokio::net::TcpListener) -> (Arc<Painel>, PathBuf) {
    let d = pasta();
    std::fs::write(d.join("no.env"), "# teste\nPLEITO=3220\nCICLO=ele2026\nAMBIENTE=oficial\nBASE=http://127.0.0.1:18080\nRPS=95\nNO_ID=x\n").unwrap();
    std::fs::write(d.join("saida/saude.json"), json!({"versao": "0.8.33", "estado": "coletando", "secoes": {"aplicadas": 10}}).to_string()).unwrap();
    let a = l.local_addr().unwrap();
    let cfg = Config {
        escuta: a, no_id: no.into(), saida: d.join("saida"), no_env: d.join("no.env"), federacao: None, pares,
        url_publica: format!("http://localhost:{}", a.port()), google_id: "cliente.apps".into(), google_segredo: "segredo-google".into(),
        google_autorizacao: format!("{google}/auth"), google_token: format!("{google}/token"),
        emails: seguranca::lista_emails("operador@exemplo.org"), segredo: "segredo-comum-dos-nos-de-teste-0123456789".into(),
        estado: d.join("painel"), acao: vec![], intervalo_s: 1.0, servicos: vec![], capacidade: 100,
    };
    let p = Painel::novo(cfg).unwrap();
    p.am.lock().unwrap().amostrar(amostras::agora_ms());
    let r = rotas(p.clone());
    tokio::spawn(async move { axum::serve(l, r).await.unwrap() });
    (p, d)
}

fn cliente() -> reqwest::Client { reqwest::Client::builder().redirect(reqwest::redirect::Policy::none()).build().unwrap() }

fn set_cookie(r: &reqwest::Response, nome: &str) -> Option<String> {
    r.headers().get_all("set-cookie").iter().filter_map(|v| v.to_str().ok())
        .find(|c| c.starts_with(&format!("{nome}="))).map(|c| c.split(';').next().unwrap().to_string())
}

/// Faz o login e devolve o cookie de sessão (ou o Location final, se recusado).
async fn login(base: &str, prox: &Proximo, email: &str) -> Result<String, String> {
    let c = cliente();
    let r = c.get(format!("{base}/painel/entrar")).send().await.unwrap();
    assert_eq!(r.status(), 303);
    let loc = r.headers()["location"].to_str().unwrap().to_string();
    let ck = set_cookie(&r, COOKIE_LOGIN).unwrap();
    let q: HashMap<String, String> = seguranca::parametros(loc.split_once('?').unwrap().1).into_iter().collect();
    assert!(q["redirect_uri"].ends_with("/painel/oauth/retorno"));
    *prox.lock().unwrap() = (email.into(), q["nonce"].clone());
    let r = c.get(format!("{base}/painel/oauth/retorno?code=codigo-bom&state={}", seguranca::codificar(&q["state"]))).header("cookie", ck).send().await.unwrap();
    assert_eq!(r.status(), 303);
    match set_cookie(&r, COOKIE_SESSAO) {
        Some(s) => Ok(s),
        None => Err(r.headers()["location"].to_str().unwrap().to_string()),
    }
}

#[tokio::test]
async fn ponta_a_ponta_dois_nos() {
    let prox: Proximo = Arc::new(Mutex::new(Default::default()));
    let g = google_falso(prox.clone()).await;
    let la = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let lb = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let (ua, ub) = (format!("http://{}", la.local_addr().unwrap()), format!("http://{}", lb.local_addr().unwrap()));
    let (_pa, da) = painel("no-a", &g, vec![ub.clone()], la).await;
    let (_pb, db) = painel("no-b", &g, vec![ua.clone()], lb).await;
    let c = cliente();

    // sem sessão: página de login, API fechada
    let r = c.get(format!("{ua}/painel/")).send().await.unwrap();
    assert!(r.text().await.unwrap().contains("Entrar com o Google"));
    assert_eq!(c.get(format!("{ua}/painel/api/visao")).send().await.unwrap().status(), 401);

    // e-mail fora da lista: recusado e auditado
    assert_eq!(login(&ua, &prox, "intruso@exemplo.org").await.unwrap_err(), "/painel/?erro=negado");
    assert!(std::fs::read_to_string(da.join("painel/auditoria.jsonl")).unwrap().contains("fora_da_lista"));

    // state errado: recusado
    let r = c.get(format!("{ua}/painel/entrar")).send().await.unwrap();
    let ck = set_cookie(&r, COOKIE_LOGIN).unwrap();
    let r = c.get(format!("{ua}/painel/oauth/retorno?code=codigo-bom&state=outro")).header("cookie", ck).send().await.unwrap();
    assert_eq!(r.headers()["location"], "/painel/?erro=expirado");
    // sem o cookie do login: recusado
    let r = c.get(format!("{ua}/painel/oauth/retorno?code=codigo-bom&state=x")).send().await.unwrap();
    assert_eq!(r.headers()["location"], "/painel/?erro=expirado");

    // e-mail da lista (com maiúsculas): entra; a sessão do A vale no B (mesmo segredo)
    let s = login(&ua, &prox, "Operador@Exemplo.org").await.unwrap();
    let r = c.get(format!("{ua}/painel/")).header("cookie", &s).send().await.unwrap();
    assert!(r.text().await.unwrap().contains("app.js"));
    let eu: Value = c.get(format!("{ub}/painel/api/eu")).header("cookie", &s).send().await.unwrap().js().await;
    assert_eq!(eu["email"], "operador@exemplo.org");
    let csrf = eu["csrf"].as_str().unwrap().to_string();

    // visão dos dois nós a partir do A
    let v: Value = c.get(format!("{ua}/painel/api/visao")).header("cookie", &s).send().await.unwrap().js().await;
    let ids: Vec<&str> = v["nos"].as_array().unwrap().iter().map(|n| n["id"].as_str().unwrap()).collect();
    assert_eq!(ids, ["no-a", "no-b"]);
    assert_eq!(v["nos"][1]["alcancavel"], true);
    assert_eq!(v["nos"][1]["resumo"]["servico"]["versao"], "0.8.33");

    // configuração do B lida e mudada pelo painel do A
    let cfg: Value = c.get(format!("{ua}/painel/api/no/no-b/config")).header("cookie", &s).send().await.unwrap().js().await;
    assert_eq!(cfg["no"], "no-b");
    let url = format!("{ua}/painel/api/no/no-b/config");
    let corpo = |v: Value| v.to_string();
    // sem CSRF
    let r = c.post(&url).header("cookie", &s).body(corpo(json!({"alteracoes": {"UFS": "sp"}}))).send().await.unwrap();
    assert_eq!(r.status(), 403);
    // origem de outro site
    let r = c.post(&url).header("cookie", &s).header("x-uv-csrf", &csrf).header("origin", "https://evil.example").body(corpo(json!({"alteracoes": {"UFS": "sp"}}))).send().await.unwrap();
    assert_eq!(r.status(), 403);
    // não sensível: aplica
    let r = c.post(&url).header("cookie", &s).header("x-uv-csrf", &csrf).body(corpo(json!({"alteracoes": {"UFS": "sp,rj"}}))).send().await.unwrap();
    assert_eq!(r.status(), 200, "{}", r.text().await.unwrap());
    assert!(std::fs::read_to_string(db.join("no.env")).unwrap().contains("UFS=sp,rj\n"));
    assert!(!std::fs::read_to_string(da.join("no.env")).unwrap().contains("UFS="), "o do A não muda");
    // sensível sem confirmação: 409 com a frase pedida; com ela, aplica
    let r = c.post(&url).header("cookie", &s).header("x-uv-csrf", &csrf).body(corpo(json!({"alteracoes": {"RPS": "50"}}))).send().await.unwrap();
    assert_eq!(r.status(), 409);
    assert_eq!(r.js().await["confirmacao"], "CONFIRMO");
    let r = c.post(&url).header("cookie", &s).header("x-uv-csrf", &csrf).body(corpo(json!({"alteracoes": {"RPS": "50"}, "confirmacao": "CONFIRMO"}))).send().await.unwrap();
    assert_eq!(r.status(), 200);
    assert!(std::fs::read_to_string(db.join("no.env")).unwrap().contains("RPS=50\n"));
    // apontar para o TSE pede a outra frase
    let r = c.post(&url).header("cookie", &s).header("x-uv-csrf", &csrf).body(corpo(json!({"alteracoes": {"BASE": "https://resultados.tse.jus.br"}, "confirmacao": "CONFIRMO"}))).send().await.unwrap();
    assert_eq!(r.js().await["confirmacao"], "CONFIRMO TSE");
    // acima de 100 req/s: contra o simulador local vale (como no uv-servico); modo espelho com o simulador, não
    let r = c.post(&url).header("cookie", &s).header("x-uv-csrf", &csrf).body(corpo(json!({"alteracoes": {"RPS": "150"}, "confirmacao": "CONFIRMO"}))).send().await.unwrap();
    assert_eq!(r.status(), 200);
    let r = c.post(&url).header("cookie", &s).header("x-uv-csrf", &csrf).body(corpo(json!({"alteracoes": {"ACESSO": "interno"}, "confirmacao": "CONFIRMO"}))).send().await.unwrap();
    assert_eq!(r.status(), 400);
    let r = c.post(&url).header("cookie", &s).header("x-uv-csrf", &csrf).body(corpo(json!({"alteracoes": {"RPS": "50"}, "confirmacao": "CONFIRMO"}))).send().await.unwrap();
    assert_eq!(r.status(), 200);
    // injeção no no.env: recusada
    let r = c.post(&url).header("cookie", &s).header("x-uv-csrf", &csrf).body(corpo(json!({"alteracoes": {"UFS": "$(id)"}}))).send().await.unwrap();
    assert_eq!(r.status(), 400);
    // a auditoria do B registra quem mudou (pelo painel do A)
    let aud = std::fs::read_to_string(db.join("painel/auditoria.jsonl")).unwrap();
    assert!(aud.contains("operador@exemplo.org") && aud.contains("\"RPS\":\"50\""));

    // serviço em modo local: recusado com explicação (sem ajudante de ações)
    let r = c.post(format!("{ua}/painel/api/no/no-a/servico")).header("cookie", &s).header("x-uv-csrf", &csrf)
        .body(corpo(json!({"acao": "parar", "confirmacao": "CONFIRMO"}))).send().await.unwrap();
    assert_eq!(r.status(), 500);
    assert!(r.text().await.unwrap().contains("modo local"));

    // rotas internas: sem assinatura, nada
    assert_eq!(c.get(format!("{ub}/painel/interno/visao")).send().await.unwrap().status(), 403);
    assert_eq!(c.post(format!("{ub}/painel/interno/config")).body("{\"alteracoes\":{\"UFS\":\"ba\"}}").send().await.unwrap().status(), 403);
    // nó desconhecido
    assert_eq!(c.get(format!("{ua}/painel/api/no/no-z/config")).header("cookie", &s).send().await.unwrap().status(), 404);

    // forward_auth do Caddy: com sessão 200 (e o e-mail), sem sessão 401; os dois auditados
    let r = c.get(format!("{ua}/painel/auth/verificar")).header("cookie", &s).header("x-forwarded-method", "POST")
        .header("x-forwarded-uri", "/api/v1/replay/posicao").send().await.unwrap();
    assert_eq!(r.status(), 200);
    assert_eq!(r.headers()["x-uv-operador"], "operador@exemplo.org");
    let r = c.get(format!("{ua}/painel/auth/verificar")).header("x-forwarded-method", "POST").header("x-forwarded-uri", "/api/v1/replay/posicao").send().await.unwrap();
    assert_eq!(r.status(), 401);
    let aud = std::fs::read_to_string(da.join("painel/auditoria.jsonl")).unwrap();
    assert!(aud.contains("\"acao\":\"controle\"") && aud.contains("controle_recusado"));

    // sair
    let r = c.post(format!("{ua}/painel/sair")).header("cookie", &s).send().await.unwrap();
    assert!(set_cookie(&r, COOKIE_SESSAO).unwrap().ends_with("="));

    // cabeçalhos de segurança
    let r = c.get(format!("{ua}/painel/")).send().await.unwrap();
    assert!(r.headers()["content-security-policy"].to_str().unwrap().contains("frame-ancestors 'none'"));
    assert_eq!(r.headers()["cache-control"], "no-store");

    for d in [da, db] { let _ = std::fs::remove_dir_all(d); }
}

#[tokio::test]
async fn par_fora_do_ar_aparece_como_inalcancavel() {
    let prox: Proximo = Arc::new(Mutex::new(Default::default()));
    let g = google_falso(prox.clone()).await;
    let la = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let ua = format!("http://{}", la.local_addr().unwrap());
    let (_pa, da) = painel("no-a", &g, vec!["http://127.0.0.1:9".into()], la).await;
    let s = login(&ua, &prox, "operador@exemplo.org").await.unwrap();
    let v: Value = cliente().get(format!("{ua}/painel/api/visao")).header("cookie", &s).send().await.unwrap().js().await;
    assert_eq!(v["nos"].as_array().unwrap().len(), 2);
    assert_eq!(v["nos"][1]["alcancavel"], false);
    // B-019: a mensagem traz a causa (conexão recusada), não só "error sending request for url"
    let erro = v["nos"][1]["erro"].as_str().unwrap().to_lowercase();
    assert!(erro.contains("refused") || erro.contains("recusad"), "erro sem a causa: {erro}");
    let _ = std::fs::remove_dir_all(da);
}

#[test]
fn configuracao_recusada_sem_o_essencial() {
    let base = Config {
        escuta: "127.0.0.1:0".parse().unwrap(), no_id: "no-a".into(), saida: pasta(), no_env: "/x".into(), federacao: None, pares: vec![],
        url_publica: "https://painel.exemplo.org".into(), google_id: "id".into(), google_segredo: "s".into(),
        google_autorizacao: seguranca::GOOGLE_AUTORIZACAO.into(), google_token: seguranca::GOOGLE_TOKEN.into(),
        emails: vec!["a@b.c".into()], segredo: "x".repeat(40), estado: pasta(), acao: vec![], intervalo_s: 5.0, servicos: vec![], capacidade: 10,
    };
    assert!(Painel::novo(base.clone()).is_ok());
    assert!(Painel::novo(Config { emails: vec![], ..base.clone() }).is_err(), "sem e-mails");
    assert!(Painel::novo(Config { segredo: "curto".into(), ..base.clone() }).is_err(), "segredo curto");
    assert!(Painel::novo(Config { google_id: String::new(), ..base.clone() }).is_err(), "sem Google");
    assert!(Painel::novo(Config { url_publica: "http://painel.exemplo.org".into(), ..base.clone() }).is_err(), "http fora do localhost");
}

#[test]
fn utilitarios() {
    let mut h = HeaderMap::new();
    h.insert(header::COOKIE, "a=1; uv_painel=abc.def; b=2".parse().unwrap());
    assert_eq!(cookie(&h, "uv_painel").as_deref(), Some("abc.def"));
    assert_eq!(cookie(&h, "uv"), None);
    assert_eq!(origem("https://dominio.org/painel/x"), "https://dominio.org");
    assert_eq!(origem("http://localhost:8090"), "http://localhost:8090");
}

// ------------------------------------------------------------------ endereços do site (descoberta e rotas)

/// Site de resultados falso com a base em /novo (fora da raiz), um espelho igual em /espelho, um com o BU trocado
/// em /errado e um servidor que só responde 429 em /bloqueado. Conta as requisições.
async fn site_falso() -> (String, Arc<AtomicU64>) {
    let n = Arc::new(AtomicU64::new(0));
    let n2 = n.clone();
    let app = Router::new().fallback(move |u: Uri| {
        let n = n2.clone();
        async move {
            n.fetch_add(1, Relaxed);
            let p = u.path().to_string();
            let bu: Vec<u8> = std::iter::once(0x30u8).chain((0..200u32).map(|i| (i % 251) as u8)).collect();
            let corpo = |v: Value| json_r(StatusCode::OK, &v);
            if p.starts_with("/bloqueado/") { return erro(StatusCode::TOO_MANY_REQUESTS, "429", ""); }
            if p == "/novo/oficial/comum/config/ele-c.json" {
                return corpo(json!({"pl": [{"cd": "3221", "c": "ele2026", "dt": "25/10/2026", "e": [{"cd": "620", "nm": "Presidente"}]}]}));
            }
            if p == "/novo/oficial/ele2026/arquivo-urna/3221/config/rr/rr-p003221-cs.json" {
                return corpo(json!({"abr": [{"cd": "RR", "mu": [{"cd": "03018", "nm": "BOA VISTA", "zon": [{"cd": "0001", "sec": [
                    {"ns": "0001", "da": "25/10/2026", "ha": "17:30:00"}, {"ns": "0002", "da": "", "ha": ""}]}]}]}]}));
            }
            let dir = "/oficial/ele2026/arquivo-urna/3221/dados/rr/03018/0001/0001";
            if p == format!("/novo{dir}/p003221-rr-m03018-z0001-s0001-aux.json") {
                return corpo(json!({"st": "Totalizada", "hashes": [{"hash": "6162", "dr": "25/10/2026", "hr": "17:31:00", "st": "Totalizado",
                    "arq": [{"nm": "o03221rr0301800010001-bu.dat", "tp": "bu"}]}]}));
            }
            for (pre, b) in [("/novo", bu.clone()), ("/espelho", bu.clone()), ("/errado", vec![0x30u8; 100])] {
                if p == format!("{pre}{dir}/6162/o03221rr0301800010001-bu.dat") {
                    return resposta(StatusCode::OK, "application/octet-stream", b);
                }
            }
            erro(StatusCode::NOT_FOUND, "404", "")
        }
    });
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let a = l.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(l, app).await.unwrap() });
    (format!("http://{a}"), n)
}

#[tokio::test]
async fn descobre_e_grava_os_enderecos() {
    let prox: Proximo = Arc::new(Mutex::new(Default::default()));
    let g = google_falso(prox.clone()).await;
    let (site, contador) = site_falso().await;
    let la = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let lb = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let (ua, ub) = (format!("http://{}", la.local_addr().unwrap()), format!("http://{}", lb.local_addr().unwrap()));
    let (_pa, da) = painel("no-a", &g, vec![ub.clone()], la).await;
    let (_pb, db) = painel("no-b", &g, vec![ua.clone()], lb).await;
    let c = cliente();
    let s = login(&ua, &prox, "operador@exemplo.org").await.unwrap();
    let csrf = c.get(format!("{ua}/painel/api/eu")).header("cookie", &s).send().await.unwrap().js().await["csrf"].as_str().unwrap().to_string();
    let post = |caminho: String, v: Value| { let (c, s, csrf) = (c.clone(), s.clone(), csrf.clone()); async move {
        let r = c.post(caminho).header("cookie", &s).header("x-uv-csrf", &csrf).body(v.to_string()).send().await.unwrap();
        (r.status().as_u16(), r.js().await) } };

    // estado inicial: caminhos padrão
    let e = c.get(format!("{ua}/painel/api/no/no-b/rotas")).header("cookie", &s).send().await.unwrap().js().await;
    assert_eq!(e["padrao"], true);
    assert_eq!(e["base"], "http://127.0.0.1:18080");

    // descoberta pelo painel do A, feita no nó B, a partir do endereço da página do site
    let (st, v) = post(format!("{ua}/painel/api/no/no-b/descobrir"), json!({"url": format!("{site}/novo/oficial/app/index.html#/x"), "pleito": "3221"})).await;
    assert_eq!(st, 200, "{v}");
    assert_eq!(v["ok"], true, "{v}");
    assert_eq!(v["bu_ok"], true, "{v}");
    assert_eq!(v["proposta"]["BASE"], format!("{site}/novo"));
    assert_eq!(v["proposta"]["CICLO"], "ele2026");
    assert!(v["exemplo"]["bu"].as_str().unwrap().ends_with("/6162/o03221rr0301800010001-bu.dat"));
    assert!(v["requisicoes"].as_u64().unwrap() <= rotas::MAX_REQ as u64);
    assert_eq!(v["requisicoes"].as_u64().unwrap(), contador.load(Relaxed), "o painel conta tudo o que pediu");

    // BUs de outro endereço: idêntico → aceito; diferente → recusado
    let (_, v) = post(format!("{ua}/painel/api/no/no-a/descobrir"), json!({"url": format!("{site}/novo/"), "pleito": "3221", "base_arquivos": format!("{site}/espelho")})).await;
    assert_eq!(v["bu_ok"], true, "{v}");
    assert!(v["passos"].as_array().unwrap().iter().any(|p| p["passo"] == "Mesmo BU pela base principal" && p["ok"] == true));
    assert!(v["exemplo"]["bu"].as_str().unwrap().contains("/espelho/"));
    let (_, v) = post(format!("{ua}/painel/api/no/no-a/descobrir"), json!({"url": format!("{site}/novo/"), "pleito": "3221", "base_arquivos": format!("{site}/errado")})).await;
    assert_eq!(v["bu_ok"], false, "{v}");

    // pleito que não existe: lista os que existem
    let (_, v) = post(format!("{ua}/painel/api/no/no-a/descobrir"), json!({"url": format!("{site}/novo/oficial"), "pleito": "9999"})).await;
    assert_eq!(v["ok"], false);
    assert_eq!(v["pleitos"][0]["cd"], "3221");

    // 429: para na primeira resposta
    let antes = contador.load(Relaxed);
    let (_, v) = post(format!("{ua}/painel/api/no/no-a/descobrir"), json!({"url": format!("{site}/bloqueado/a/b/c"), "pleito": "3221"})).await;
    assert_eq!(v["ok"], false);
    assert_eq!(contador.load(Relaxed) - antes, 1, "uma requisição e parou");
    assert!(v["passos"][0]["detalhe"].as_str().unwrap().contains("429"));

    // gravar no B: pede a frase; com ela grava o rotas_tse.json e a BASE
    let rotas_novas = json!({"base_arquivos": format!("{site}/espelho")});
    let (st, v) = post(format!("{ua}/painel/api/no/no-b/rotas"), json!({"base": format!("{site}/novo"), "rotas": rotas_novas, "PLEITO": "3221"})).await;
    assert_eq!(st, 409, "{v}");
    assert_eq!(v["confirmacao"], "CONFIRMO");
    let (st, v) = post(format!("{ua}/painel/api/no/no-b/rotas"), json!({"base": format!("{site}/novo"), "rotas": rotas_novas, "PLEITO": "3221", "confirmacao": "CONFIRMO"})).await;
    assert_eq!(st, 200, "{v}");
    let ne = std::fs::read_to_string(db.join("no.env")).unwrap();
    assert!(ne.contains(&format!("BASE={site}/novo\n")) && ne.contains("PLEITO=3221\n"), "{ne}");
    let r = crate::tse::Rotas::ler(&db.join("rotas_tse.json")).unwrap();
    assert_eq!(r.base_arquivos.as_deref(), Some(format!("{site}/espelho").as_str()));
    assert!(!da.join("rotas_tse.json").exists(), "o A não muda");
    let e = c.get(format!("{ua}/painel/api/no/no-b/rotas")).header("cookie", &s).send().await.unwrap().js().await;
    assert_eq!(e["padrao"], false);
    assert!(e["exemplos"]["arquivo"].as_str().unwrap().starts_with(&format!("{site}/espelho/")));

    // modelos que tirariam a coleta do controle: recusados
    let (st, _) = post(format!("{ua}/painel/api/no/no-b/rotas"), json!({"base": format!("{site}/novo"), "rotas": {"arquivo": "https://evil.example/{nome}"}, "confirmacao": "CONFIRMO"})).await;
    assert_eq!(st, 400);
    let (st, _) = post(format!("{ua}/painel/api/no/no-b/rotas"), json!({"base": format!("{site}/novo"), "rotas": {"base_arquivos": "https://resultados.tse.jus.br"}, "confirmacao": "CONFIRMO TSE"})).await;
    assert_eq!(st, 400, "base local com arquivos remotos");

    // de volta ao padrão: o arquivo some
    let (st, v) = post(format!("{ua}/painel/api/no/no-b/rotas"), json!({"base": format!("{site}/novo"), "rotas": null, "confirmacao": "CONFIRMO"})).await;
    assert_eq!(st, 200, "{v}");
    assert!(!db.join("rotas_tse.json").exists());
    assert!(std::fs::read_to_string(db.join("painel/auditoria.jsonl")).unwrap().contains("\"acao\":\"rotas\""));

    for d in [da, db] { let _ = std::fs::remove_dir_all(d); }
}
