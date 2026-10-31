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
