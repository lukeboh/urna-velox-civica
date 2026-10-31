use super::*;

const EXEMPLO: &str = "# nó A\nPLEITO=3221\nCICLO=ele2026\nAMBIENTE=oficial\nBASE=https://resultados.tse.jus.br\nRPS=95 # por IP\n\
NO_ID=no-a\nFED_PORTA=7700\nUFS=\"sp,rj\"\nEXTRA='--aceitar-incompleto --x 1'\nOUTRA=1\n";

fn m(p: &[(&str, &str)]) -> BTreeMap<String, String> { p.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect() }

#[test]
fn le_como_o_bash() {
    let v = ler(EXEMPLO);
    assert_eq!(v["RPS"], "95");
    assert_eq!(v["UFS"], "sp,rj");
    assert_eq!(v["EXTRA"], "--aceitar-incompleto --x 1");
    assert_eq!(v["OUTRA"], "1");
    assert!(!v.contains_key("nó A"));
}

#[test]
fn valida_cada_campo() {
    assert!(validar("RPS", "95").is_ok());
    assert!(validar("RPS", "100").is_ok());
    assert!(validar("RPS", "101").is_ok(), "campo sozinho: o teto depende do acesso (validar_conjunto)");
    assert!(validar("TETO_RPS_INTERNO", "0").is_ok(), "0 = sem limite");
    // M-020: trava de 0 (nunca à frente do TSE) a 100 (sem limite); vazio = desligada; não pede CONFIRMO
    for ok in ["0", "10", "2.5", "100", ""] { assert!(validar("TRAVA_PP", ok).is_ok(), "TRAVA_PP={ok}"); }
    for ruim in ["101", "-1", "dez", "1e9"] { assert!(validar("TRAVA_PP", ruim).is_err(), "TRAVA_PP={ruim}"); }
    assert!(campo("TRAVA_PP").is_some_and(|c| c.editavel && !c.sensivel && c.opcional));
    // multipleito (§3.6 do HANDOFF_1.1.0): o no.env pode ficar sem PLEITO (o ciclo escolhe)
    assert!(campo("PLEITO").is_some_and(|c| c.editavel && c.sensivel && c.opcional));
    assert!(validar("ACESSO", "qualquer").is_err());
    assert!(validar("RPS", "0").is_err());
    assert!(validar("RPS", "abc").is_err());
    assert!(validar("RPS", "").is_err(), "obrigatório");
    assert!(validar("UFS", "").is_ok());
    assert!(validar("UFS", "sp,RJ").is_ok());
    assert!(validar("UFS", "sp,rjx").is_err());
    assert!(validar("ACELERACAO", "5000").is_err());
    assert!(validar("AMBIENTE", "teste").is_err());
    assert!(validar("PLEITO", "32a1").is_err());
    assert!(validar("CANDIDATOS", "/etc/shadow").is_err());
    assert!(validar("CANDIDATOS", "/srv/urna-velox/../etc/shadow").is_err());
    assert!(validar("CANDIDATOS", "/srv/urna-velox/simulado/candidatos.json").is_ok());
    assert!(validar("NO_ID", "no-z").is_err(), "não editável");
    assert!(validar("EXTRA", "--x").is_err(), "não editável");
    assert!(validar("DESCONHECIDA", "1").is_err());
}

#[test]
fn nada_que_o_bash_execute() {
    for v in ["$(reboot)", "`id`", "95;rm -rf /", "a b", "x|y", "a&&b", "\"x\"", "'x'", "a\nb", "$HOME", "x>y", "(x)"] {
        assert!(validar("PLEITO", v).is_err() && validar("UFS", v).is_err() && validar("BASE", &format!("https://x/{v}")).is_err(), "{v:?}");
    }
}

#[test]
fn aplica_mantendo_o_resto() {
    let t = aplicar(EXEMPLO, &m(&[("RPS", "50"), ("UFS", ""), ("ACELERACAO", "10")])).unwrap();
    assert!(t.contains("RPS=50\n") && !t.contains("RPS=95"));
    assert!(!t.contains("UFS="), "opcional vazio sai do arquivo");
    assert!(t.ends_with("ACELERACAO=10\n"), "nova vai no fim");
    assert!(t.starts_with("# nó A\nPLEITO=3221\n"));
    assert!(t.contains("EXTRA='--aceitar-incompleto --x 1'\n") && t.contains("OUTRA=1\n"), "o resto fica igual");
    let v = ler(&t);
    assert_eq!(v["RPS"], "50");
    assert_eq!(v.get("UFS"), None);
}

#[test]
fn chave_repetida_vale_a_ultima() {
    let t = aplicar("RPS=10\nRPS=20\n", &m(&[("RPS", "30")])).unwrap();
    assert_eq!(t, "# (substituída pelo painel) RPS=10\nRPS=30\n");
    assert_eq!(ler(&t)["RPS"], "30");
}

#[test]
fn aplicar_recusa_valor_ruim() {
    assert!(aplicar(EXEMPLO, &m(&[("RPS", "150")])).is_err(), "borda do TSE: acima de 100 recusado");
    assert!(aplicar(EXEMPLO, &m(&[("NO_ID", "no-z")])).is_err());
}

#[test]
fn confirmacoes() {
    let atual = ler(EXEMPLO);
    assert_eq!(confirmacao_exigida(&atual, &m(&[("UFS", "sp")])), None, "não sensível");
    assert_eq!(confirmacao_exigida(&atual, &m(&[("RPS", "95")])), None, "sem mudança de fato");
    assert_eq!(confirmacao_exigida(&atual, &m(&[("RPS", "50")])), Some("CONFIRMO TSE"), "continua no TSE");
    assert_eq!(confirmacao_exigida(&atual, &m(&[("BASE", "http://127.0.0.1:18080")])), Some("CONFIRMO"));
    let local = ler("BASE=http://127.0.0.1:18080\nPLEITO=3220\n");
    assert_eq!(confirmacao_exigida(&local, &m(&[("PLEITO", "3221")])), Some("CONFIRMO"));
    assert_eq!(confirmacao_exigida(&local, &m(&[("BASE", "https://resultados.tse.jus.br")])), Some("CONFIRMO TSE"));
    assert!(eh_tse("https://cdn.tse.jus.br/x") && eh_tse("resultados.tse.jus.br") && !eh_tse("https://tse.jus.br.evil.org"));
}

#[test]
fn descreve_para_a_pagina() {
    let d = descrever(&ler(EXEMPLO));
    assert_eq!(d["outros"]["OUTRA"], "1");
    let rps = d["campos"].as_array().unwrap().iter().find(|c| c["chave"] == "RPS").unwrap();
    assert_eq!(rps["valor"], "95");
    assert_eq!(rps["sensivel"], true);
}

#[test]
fn acesso_borda_e_interno() {
    // borda pública: 100 é o teto; modo espelho recusado
    assert!(aplicar(EXEMPLO, &m(&[("RPS", "100")])).is_ok());
    assert!(aplicar(EXEMPLO, &m(&[("ACESSO", "interno")])).is_err(), "interno com a base na borda pública");
    assert!(aplicar(EXEMPLO, &m(&[("TETO_RPS_INTERNO", "300")])).is_err(), "teto do espelho sem ACESSO=interno");
    // servidor interno: interno libera o teto
    let int = &m(&[("BASE", "https://espelho.exemplo.org"), ("ACESSO", "interno"), ("TETO_RPS_INTERNO", "0"), ("RPS", "800")]);
    let t = aplicar(EXEMPLO, int).unwrap();
    assert!(t.contains("ACESSO=interno\n") && t.contains("RPS=800\n") && t.contains("TETO_RPS_INTERNO=0\n"));
    // ...mas não se os BUs vierem da borda pública
    assert!(aplicar_com(EXEMPLO, int, Some("https://cdn.tse.jus.br")).is_err());
    // servidor interno na borda (modo padrão): continua o teto de 100
    assert!(aplicar(EXEMPLO, &m(&[("BASE", "https://espelho.exemplo.org"), ("RPS", "800")])).is_err());
    // simulador local: acima de 100 vale (como no uv-servico), interno não
    assert!(aplicar("BASE=http://127.0.0.1:18080\nRPS=95\n", &m(&[("RPS", "900")])).is_ok());
    assert!(aplicar("BASE=http://127.0.0.1:18080\nRPS=95\n", &m(&[("ACESSO", "interno")])).is_err());
    // voltar à borda: ACESSO e teto vazios saem do arquivo
    let t2 = aplicar(&t, &m(&[("BASE", "https://resultados.tse.jus.br"), ("ACESSO", ""), ("TETO_RPS_INTERNO", ""), ("RPS", "95")])).unwrap();
    assert!(!t2.contains("ACESSO=") && !t2.contains("TETO_RPS_INTERNO="));
    assert_eq!(confirmacao_exigida(&ler(EXEMPLO), &m(&[("ACESSO", "interno")])), Some("CONFIRMO TSE"), "sensível");
}
