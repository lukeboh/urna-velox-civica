//! Segurança do `uv-painel`: HMAC-SHA256, tokens assinados (sessão, login, CSRF), assinatura entre os painéis dos nós e
//! validação do `id_token` do Google (OpenID Connect).
//!
//! Nada aqui guarda estado: a sessão é um cookie assinado (e-mail + validade) e o login em andamento é outro cookie
//! assinado (`state` + `nonce` + validade). O segredo mestre (`UV_PAINEL_SEGREDO`) é o mesmo nos dois nós, para que o
//! login feito num nó valha no outro (o DNS ou o balanceador podem mandar o retorno do Google para qualquer um).

use anyhow::{bail, Context, Result};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::io::Read;

/// HMAC-SHA256 (RFC 2104).
pub fn hmac(chave: &[u8], msg: &[u8]) -> [u8; 32] {
    const BLOCO: usize = 64;
    let mut k = [0u8; BLOCO];
    if chave.len() > BLOCO { k[..32].copy_from_slice(&Sha256::digest(chave)); } else { k[..chave.len()].copy_from_slice(chave); }
    let mut ipad = [0x36u8; BLOCO];
    let mut opad = [0x5cu8; BLOCO];
    for i in 0..BLOCO { ipad[i] ^= k[i]; opad[i] ^= k[i]; }
    let interno = Sha256::new().chain_update(ipad).chain_update(msg).finalize();
    Sha256::new().chain_update(opad).chain_update(interno).finalize().into()
}

/// Comparação em tempo constante (o tempo não depende de onde os bytes diferem).
pub fn iguais(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() { return false; }
    a.iter().zip(b).fold(0u8, |d, (x, y)| d | (x ^ y)) == 0
}

pub fn b64(b: &[u8]) -> String { URL_SAFE_NO_PAD.encode(b) }
pub fn de_b64(s: &str) -> Option<Vec<u8>> { URL_SAFE_NO_PAD.decode(s.trim_end_matches('=')).ok() }

/// `n` bytes aleatórios do sistema (/dev/urandom).
pub fn aleatorio(n: usize) -> Vec<u8> {
    let mut v = vec![0u8; n];
    std::fs::File::open("/dev/urandom").and_then(|mut f| f.read_exact(&mut v)).expect("/dev/urandom");
    v
}

pub fn agora_s() -> u64 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) }

/// Chaves derivadas do segredo mestre, uma por uso (um token de um uso nunca vale em outro).
#[derive(Clone)]
pub struct Chaves { sessao: [u8; 32], login: [u8; 32], csrf: [u8; 32], par: [u8; 32] }

impl Chaves {
    pub fn de_segredo(segredo: &str) -> Result<Chaves> {
        if segredo.len() < 32 { bail!("UV_PAINEL_SEGREDO precisa de pelo menos 32 caracteres (gere com: openssl rand -base64 48)"); }
        let s = segredo.as_bytes();
        Ok(Chaves { sessao: hmac(s, b"uv-painel/sessao"), login: hmac(s, b"uv-painel/login"), csrf: hmac(s, b"uv-painel/csrf"), par: hmac(s, b"uv-painel/par") })
    }
}

/// `conteudo.assinatura` (conteúdo em base64url de um JSON).
fn assinar(chave: &[u8], v: &impl Serialize) -> String {
    let c = b64(&serde_json::to_vec(v).expect("json"));
    let a = b64(&hmac(chave, c.as_bytes()));
    format!("{c}.{a}")
}

fn conferir<T: for<'de> Deserialize<'de>>(chave: &[u8], token: &str) -> Option<T> {
    let (c, a) = token.rsplit_once('.')?;
    let a = de_b64(a)?;
    if !iguais(&a, &hmac(chave, c.as_bytes())) { return None; }
    serde_json::from_slice(&de_b64(c)?).ok()
}

// ------------------------------------------------------------------ sessão

/// Sessão do operador (cookie `uv_painel`).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Sessao {
    /// e-mail confirmado pelo Google (minúsculas)
    pub e: String,
    /// validade (segundos Unix)
    pub x: u64,
}

pub const DURACAO_SESSAO_S: u64 = 12 * 3600;
pub const DURACAO_LOGIN_S: u64 = 600;

impl Chaves {
    pub fn emitir_sessao(&self, email: &str, agora: u64) -> String {
        assinar(&self.sessao, &Sessao { e: email.to_lowercase(), x: agora + DURACAO_SESSAO_S })
    }

    /// Sessão válida, no prazo e de um e-mail que **ainda** está na lista (tirar o e-mail da lista derruba a sessão).
    pub fn ler_sessao(&self, token: &str, agora: u64, permitidos: &[String]) -> Option<Sessao> {
        let s: Sessao = conferir(&self.sessao, token)?;
        (s.x > agora && email_permitido(&s.e, permitidos)).then_some(s)
    }

    /// Token anti-CSRF da sessão: as chamadas que mudam algo mandam no cabeçalho `X-UV-CSRF`.
    pub fn csrf(&self, s: &Sessao) -> String { b64(&hmac(&self.csrf, format!("{}|{}", s.e, s.x).as_bytes())) }

    pub fn conferir_csrf(&self, s: &Sessao, recebido: &str) -> bool { iguais(recebido.as_bytes(), self.csrf(s).as_bytes()) }
}

// ------------------------------------------------------------------ login em andamento

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Login {
    /// `state` do OAuth (protege o retorno contra CSRF)
    pub s: String,
    /// `nonce` do OpenID Connect (amarra o id_token a este login)
    pub n: String,
    pub x: u64,
}

impl Chaves {
    pub fn novo_login(&self, agora: u64) -> (Login, String) {
        let l = Login { s: b64(&aleatorio(24)), n: b64(&aleatorio(24)), x: agora + DURACAO_LOGIN_S };
        let t = assinar(&self.login, &l);
        (l, t)
    }

    pub fn ler_login(&self, token: &str, agora: u64) -> Option<Login> {
        let l: Login = conferir(&self.login, token)?;
        (l.x > agora).then_some(l)
    }
}

// ------------------------------------------------------------------ entre os painéis dos nós

/// Cabeçalho que um painel manda ao do outro nó: `ts.assinatura` sobre método, caminho, instante e SHA-256 do corpo.
pub const CABECALHO_PAR: &str = "x-uv-painel-par";
pub const JANELA_PAR_S: u64 = 60;

impl Chaves {
    fn base_par(metodo: &str, caminho: &str, ts: u64, corpo: &[u8]) -> String {
        format!("{metodo}|{caminho}|{ts}|{}", hex(&Sha256::digest(corpo)))
    }

    pub fn assinar_par(&self, metodo: &str, caminho: &str, corpo: &[u8], agora: u64) -> String {
        format!("{agora}.{}", b64(&hmac(&self.par, Self::base_par(metodo, caminho, agora, corpo).as_bytes())))
    }

    pub fn conferir_par(&self, valor: &str, metodo: &str, caminho: &str, corpo: &[u8], agora: u64) -> bool {
        let Some((ts, a)) = valor.split_once('.') else { return false };
        let Ok(ts) = ts.parse::<u64>() else { return false };
        if ts.abs_diff(agora) > JANELA_PAR_S { return false; }
        let Some(a) = de_b64(a) else { return false };
        iguais(&a, &hmac(&self.par, Self::base_par(metodo, caminho, ts, corpo).as_bytes()))
    }
}

pub fn hex(b: &[u8]) -> String { b.iter().map(|x| format!("{x:02x}")).collect() }

// ------------------------------------------------------------------ lista de e-mails

/// Lista de e-mails (vírgula, ponto e vírgula ou espaço) → minúsculas, sem vazios.
pub fn lista_emails(s: &str) -> Vec<String> {
    s.split([',', ';', ' ', '\n']).map(|e| e.trim().to_lowercase()).filter(|e| e.contains('@')).collect()
}

pub fn email_permitido(email: &str, permitidos: &[String]) -> bool {
    let e = email.trim().to_lowercase();
    !e.is_empty() && permitidos.iter().any(|p| *p == e)
}

// ------------------------------------------------------------------ OpenID Connect (Google)

pub const GOOGLE_AUTORIZACAO: &str = "https://accounts.google.com/o/oauth2/v2/auth";
pub const GOOGLE_TOKEN: &str = "https://oauth2.googleapis.com/token";

/// Codificação de formulário/URL (RFC 3986: só os não reservados ficam).
pub fn codificar(s: &str) -> String {
    let mut o = String::with_capacity(s.len() * 3);
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~".contains(&b) { o.push(b as char); } else { o.push_str(&format!("%{b:02X}")); }
    }
    o
}

pub fn decodificar(s: &str) -> String {
    let b = s.as_bytes();
    let mut o = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'+' => { o.push(b' '); i += 1; }
            b'%' if i + 2 < b.len() => {
                match u8::from_str_radix(std::str::from_utf8(&b[i + 1..i + 3]).unwrap_or("zz"), 16) {
                    Ok(v) => { o.push(v); i += 3; }
                    Err(_) => { o.push(b'%'); i += 1; }
                }
            }
            c => { o.push(c); i += 1; }
        }
    }
    String::from_utf8_lossy(&o).into_owned()
}

/// Parâmetros de uma query string (`a=1&b=2`), decodificados.
pub fn parametros(q: &str) -> Vec<(String, String)> {
    q.split('&').filter(|p| !p.is_empty()).map(|p| {
        let (k, v) = p.split_once('=').unwrap_or((p, ""));
        (decodificar(k), decodificar(v))
    }).collect()
}

pub fn url_autorizacao(base: &str, client_id: &str, redirect: &str, l: &Login) -> String {
    format!("{base}?response_type=code&client_id={}&redirect_uri={}&scope={}&state={}&nonce={}&prompt=select_account",
        codificar(client_id), codificar(redirect), codificar("openid email"), codificar(&l.s), codificar(&l.n))
}

/// Corpo do POST ao endpoint de token (troca do `code`).
pub fn corpo_token(code: &str, client_id: &str, segredo: &str, redirect: &str) -> String {
    format!("grant_type=authorization_code&code={}&client_id={}&client_secret={}&redirect_uri={}",
        codificar(code), codificar(client_id), codificar(segredo), codificar(redirect))
}

/// Valida o `id_token` recebido **direto do endpoint de token do Google, por TLS** e devolve o e-mail.
///
/// Pelo OpenID Connect Core §3.1.3.7, um id_token obtido assim pode ter a origem garantida pela validação TLS do
/// servidor no lugar da assinatura; as demais conferências continuam obrigatórias e são feitas aqui: emissor,
/// audiência (o nosso client_id), validade, `nonce` deste login e e-mail verificado.
pub fn validar_id_token(id_token: &str, client_id: &str, nonce: &str, agora: u64) -> Result<String> {
    let mut p = id_token.split('.');
    let (_, corpo) = (p.next(), p.next().context("id_token sem corpo")?);
    let c: Value = serde_json::from_slice(&de_b64(corpo).context("id_token: base64 inválido")?).context("id_token: JSON inválido")?;
    let iss = c["iss"].as_str().unwrap_or("");
    if iss != "https://accounts.google.com" && iss != "accounts.google.com" { bail!("emissor inesperado: {iss}"); }
    let aud_ok = match &c["aud"] {
        Value::String(a) => a == client_id,
        Value::Array(v) => v.iter().any(|a| a.as_str() == Some(client_id)),
        _ => false,
    };
    if !aud_ok { bail!("audiência diferente do client_id"); }
    let exp = c["exp"].as_u64().or_else(|| c["exp"].as_str().and_then(|s| s.parse().ok())).unwrap_or(0);
    if exp + 60 < agora { bail!("id_token vencido"); }
    if c["nonce"].as_str() != Some(nonce) { bail!("nonce diferente do deste login"); }
    let verificado = matches!(&c["email_verified"], Value::Bool(true)) || c["email_verified"].as_str() == Some("true");
    if !verificado { bail!("e-mail não verificado pelo Google"); }
    let email = c["email"].as_str().context("id_token sem e-mail")?.to_lowercase();
    Ok(email)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn hmac_rfc4231_caso_2() {
        let r = hmac(b"Jefe", b"what do ya want for nothing?");
        assert_eq!(hex(&r), "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843");
    }

    #[test]
    fn hmac_chave_longa_rfc4231_caso_6() {
        let r = hmac(&[0xaa; 131], b"Test Using Larger Than Block-Size Key - Hash Key First");
        assert_eq!(hex(&r), "60e431591ee0b67f0d8a26aacbf5b77f8e0bc6213728c5140546040f0ee37f54");
    }

    fn chaves() -> Chaves { Chaves::de_segredo("um-segredo-de-teste-bem-comprido-0123456789").unwrap() }

    #[test]
    fn segredo_curto_recusado() { assert!(Chaves::de_segredo("curto").is_err()); }

    #[test]
    fn sessao_valida_vencida_adulterada_e_fora_da_lista() {
        let k = chaves();
        let lista = lista_emails("Operador@Exemplo.org, outro@exemplo.org");
        let t = k.emitir_sessao("operador@exemplo.org", 1000);
        assert_eq!(k.ler_sessao(&t, 1001, &lista).unwrap().e, "operador@exemplo.org");
        assert!(k.ler_sessao(&t, 1000 + DURACAO_SESSAO_S + 1, &lista).is_none(), "vencida");
        assert!(k.ler_sessao(&t, 1001, &lista_emails("outro@exemplo.org")).is_none(), "e-mail tirado da lista");
        let (c, a) = t.rsplit_once('.').unwrap();
        let forjado = b64(br#"{"e":"intruso@exemplo.org","x":99999999999}"#);
        assert!(k.ler_sessao(&format!("{forjado}.{a}"), 1001, &lista_emails("intruso@exemplo.org")).is_none(), "conteúdo trocado");
        assert!(k.ler_sessao(&format!("{c}.AAAA"), 1001, &lista).is_none(), "assinatura trocada");
        let outro = Chaves::de_segredo("outro-segredo-de-teste-bem-comprido-0123456789").unwrap();
        assert!(outro.ler_sessao(&t, 1001, &lista).is_none(), "outro segredo");
    }

    #[test]
    fn token_de_um_uso_nao_vale_em_outro() {
        let k = chaves();
        let (_, tl) = k.novo_login(1000);
        assert!(k.ler_sessao(&tl, 1001, &lista_emails("a@b.c")).is_none());
        let ts = k.emitir_sessao("a@b.c", 1000);
        assert!(k.ler_login(&ts, 1001).is_none());
    }

    #[test]
    fn login_vence() {
        let k = chaves();
        let (l, t) = k.novo_login(1000);
        assert_eq!(k.ler_login(&t, 1001).unwrap(), l);
        assert!(k.ler_login(&t, 1000 + DURACAO_LOGIN_S + 1).is_none());
    }

    #[test]
    fn csrf_amarrado_a_sessao() {
        let k = chaves();
        let s = Sessao { e: "a@b.c".into(), x: 5000 };
        let c = k.csrf(&s);
        assert!(k.conferir_csrf(&s, &c));
        assert!(!k.conferir_csrf(&Sessao { e: "a@b.c".into(), x: 5001 }, &c));
        assert!(!k.conferir_csrf(&s, ""));
    }

    #[test]
    fn assinatura_entre_nos() {
        let k = chaves();
        let a = k.assinar_par("POST", "/painel/interno/config", b"{}", 1000);
        assert!(k.conferir_par(&a, "POST", "/painel/interno/config", b"{}", 1030));
        assert!(!k.conferir_par(&a, "POST", "/painel/interno/config", b"{}", 1000 + JANELA_PAR_S + 1), "velha");
        assert!(!k.conferir_par(&a, "POST", "/painel/interno/config", b"{\"x\":1}", 1000), "corpo trocado");
        assert!(!k.conferir_par(&a, "GET", "/painel/interno/config", b"{}", 1000), "método trocado");
        assert!(!k.conferir_par(&a, "POST", "/painel/interno/servico", b"{}", 1000), "caminho trocado");
        assert!(!k.conferir_par("lixo", "POST", "/x", b"", 1000));
    }

    #[test]
    fn lista_e_comparacao_de_emails() {
        let l = lista_emails(" A@B.com ; c@d.org,,sem-arroba ");
        assert_eq!(l, vec!["a@b.com", "c@d.org"]);
        assert!(email_permitido("A@b.COM", &l));
        assert!(!email_permitido("", &l));
        assert!(!email_permitido("x@b.com", &l));
        assert!(!email_permitido("a@b.com", &[]));
    }

    #[test]
    fn codificacao_url() {
        assert_eq!(codificar("a b/c?d=é"), "a%20b%2Fc%3Fd%3D%C3%A9");
        assert_eq!(decodificar("a%20b+c%2F%C3%A9%"), "a b c/é%");
        assert_eq!(parametros("code=4%2F0A&state=x%3D&vazio"), vec![("code".into(), "4/0A".into()), ("state".into(), "x=".into()), ("vazio".into(), "".into())]);
    }

    fn id_token(c: Value) -> String { format!("{}.{}.assinatura", b64(br#"{"alg":"RS256"}"#), b64(&serde_json::to_vec(&c).unwrap())) }

    #[test]
    fn id_token_do_google() {
        let base = serde_json::json!({"iss": "https://accounts.google.com", "aud": "cliente.apps", "exp": 2000, "nonce": "n1",
            "email": "Operador@Exemplo.org", "email_verified": true});
        assert_eq!(validar_id_token(&id_token(base.clone()), "cliente.apps", "n1", 1500).unwrap(), "operador@exemplo.org");
        let com = |k: &str, v: Value| { let mut c = base.clone(); c[k] = v; id_token(c) };
        assert!(validar_id_token(&com("iss", "https://evil".into()), "cliente.apps", "n1", 1500).is_err());
        assert!(validar_id_token(&com("aud", "outro".into()), "cliente.apps", "n1", 1500).is_err());
        assert!(validar_id_token(&com("nonce", "n2".into()), "cliente.apps", "n1", 1500).is_err());
        assert!(validar_id_token(&com("email_verified", false.into()), "cliente.apps", "n1", 1500).is_err());
        assert!(validar_id_token(&id_token(base.clone()), "cliente.apps", "n1", 2100).is_err(), "vencido");
        assert!(validar_id_token("sem-pontos", "cliente.apps", "n1", 1500).is_err());
    }

    #[test]
    fn url_de_autorizacao() {
        let l = Login { s: "estado".into(), n: "nonce".into(), x: 0 };
        let u = url_autorizacao(GOOGLE_AUTORIZACAO, "id.apps", "https://x.org/painel/oauth/retorno", &l);
        assert!(u.starts_with("https://accounts.google.com/o/oauth2/v2/auth?response_type=code&client_id=id.apps"));
        assert!(u.contains("redirect_uri=https%3A%2F%2Fx.org%2Fpainel%2Foauth%2Fretorno"));
        assert!(u.contains("scope=openid%20email") && u.contains("state=estado") && u.contains("nonce=nonce"));
    }
}
