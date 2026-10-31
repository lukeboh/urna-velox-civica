use super::*;

const S: u64 = 1_000_000;

fn passada(id: u64, t: u64, motor: u64, grav: u64) -> Sonda {
    Sonda { t_motor: Some(t + motor), t_gravacao: Some(t + grav), seq: 7, ..Sonda::nova(id, 99, t) }
}

#[test]
fn json_ida_e_volta() {
    let s = passada(3, 10 * S, 1_500, 2_250);
    let v = s.json();
    assert_eq!(v["local"], true);
    assert_eq!(v["origem"], Value::Null);
    assert_eq!(v["ms"]["motor"], 1.5);
    assert_eq!(v["ms"]["gravacao"], 2.25);
    assert!(v.get("t_recebida").is_none());
    assert_eq!(Sonda::de_json(&v), Some(s));
    // remota: ms contam da chegada; federacao_ms cruza os relógios (pode ser negativo)
    let mut r = Sonda::remota("no-b", 4, 5, 10 * S, 10 * S - 2_000);
    r.t_motor = Some(10 * S + 1_000);
    let v = r.json();
    assert_eq!((v["local"].as_bool(), v["origem"].as_str()), (Some(false), Some("no-b")));
    assert_eq!(v["ms"]["motor"], 3.0);
    assert_eq!(v["ms"]["federacao"], -2.0);
    assert_eq!(v["ms"]["gravacao"], Value::Null);
    assert_eq!(Sonda::de_json(&v), Some(r));
    assert_eq!(Sonda::de_json(&json!({"id": 1})), None);
    assert_eq!(Sonda::de_json(&json!({"t_injecao": 1})), None);
}

#[test]
fn limite_padrao_e_explicito() {
    assert_eq!(Vigia::novo(30.0, 0.0).limite_us, 60 * S);
    assert_eq!(Vigia::novo(1.0, 0.0).limite_us, 5 * S);
    assert_eq!(Vigia::novo(30.0, 12.5).limite_us, 12_500_000);
    assert_eq!(Vigia::novo(-1.0, 0.0).intervalo_us, 0);
}

#[test]
fn caminho_andando_nao_alerta() {
    let mut v = Vigia::novo(1.0, 3.0);
    for i in 1..=20u64 {
        let t = i * S;
        v.injetar(i, t);
        assert_eq!(v.verificar(t + 100), None);
        v.concluir(&passada(i, t, 200, 400), t + 500);
        assert_eq!(v.verificar(t + 600), None);
    }
    let j = v.json(21 * S);
    assert_eq!((j["injetadas"].as_u64(), j["concluidas"].as_u64(), j["pendentes"].as_u64(), j["perdidas"].as_u64()), (Some(20), Some(20), Some(0), Some(0)));
    assert_eq!(j["atrasada"], false);
    assert_eq!(j["atraso_s"], 0.0);
    assert_eq!(j["ultima"]["id"], 20);
    assert_eq!(j["latencias_ms"]["motor"]["p50"], 0.2);
    assert_eq!(j["latencias_ms"]["gravacao"]["p50"], 0.4);
    assert_eq!(j["latencias_ms"]["total"]["n"], 20);
    assert_eq!(j["idade_ultima_s"], 1.0);
}

#[test]
fn motor_parado_alerta_uma_vez_e_normaliza() {
    let mut v = Vigia::novo(1.0, 3.0);
    v.injetar(1, S);
    v.concluir(&passada(1, S, 10, 20), S + 30);
    // o motor para: 2, 3, 4, 5 não voltam
    for i in 2..=5 { v.injetar(i, i * S); }
    assert_eq!(v.verificar(5 * S), None, "a mais antiga pendente tem 3 s = limite");
    let t = v.verificar(5 * S + 1);
    assert_eq!(t, Some(Transicao::Atrasou { atraso_ms: 3_000_001.0 / 1e3, pendentes: 4, id: 2 }));
    assert!(v.atrasada());
    assert_eq!(v.verificar(9 * S), None, "um alerta por episódio");
    assert_eq!(v.json(9 * S)["atraso_s"], 7.0);
    // volta: a 5 chega e leva as anteriores (FIFO: 2, 3 e 4 também passaram, mas só a 5 foi vista → perdidas)
    v.concluir(&passada(5, 5 * S, 10, 20), 10 * S);
    assert_eq!(v.verificar(10 * S), Some(Transicao::Normalizou { duracao_ms: 4_999_999.0 / 1e3 }));
    assert!(!v.atrasada());
    let j = v.json(10 * S);
    assert_eq!((j["perdidas"].as_u64(), j["episodios_atraso"].as_u64(), j["pendentes"].as_u64()), (Some(3), Some(1), Some(0)));
    // novo episódio conta de novo
    v.injetar(6, 11 * S);
    assert!(matches!(v.verificar(15 * S), Some(Transicao::Atrasou { id: 6, pendentes: 1, .. })));
    assert_eq!(v.json(15 * S)["episodios_atraso"], 2);
}

#[test]
fn pendentes_tem_teto() {
    let mut v = Vigia::novo(1.0, 1.0);
    for i in 1..=(MAX_PENDENTES as u64 + 5) { v.injetar(i, i); }
    let j = v.json(MAX_PENDENTES as u64 + 10);
    assert_eq!(j["pendentes"].as_u64(), Some(MAX_PENDENTES as u64 + 5));
    assert_eq!(j["injetadas"].as_u64(), Some(MAX_PENDENTES as u64 + 5));
    match v.verificar(10 * S) { Some(Transicao::Atrasou { pendentes, id, .. }) => assert_eq!((pendentes, id), (MAX_PENDENTES + 5, 1)), x => panic!("{x:?}") }
}

#[test]
fn sondas_dos_pares() {
    let mut v = Vigia::novo(1.0, 2.0);
    let r = |id, ses| Sonda { t_motor: Some(10 * S + 300), t_gravacao: Some(10 * S + 600), ..Sonda::remota("no-b", id, ses, 10 * S - 1_000, 10 * S) };
    v.concluir(&r(1, 7), 10 * S + 700);
    v.concluir(&r(2, 7), 10 * S + 700);
    v.concluir(&r(5, 7), 10 * S + 700); // 3 e 4 faltaram
    v.concluir(&r(5, 7), 10 * S + 700); // repetida
    v.concluir(&r(4, 7), 10 * S + 700); // atrasada = repetida (já passou da 5)
    v.concluir(&r(1, 8), 10 * S + 700); // o par reiniciou
    let j = v.json(11 * S);
    let b = &j["remotas"]["no-b"];
    assert_eq!((b["recebidas"].as_u64(), b["ultima"].as_u64(), b["puladas"].as_u64(), b["repetidas"].as_u64(), b["reinicios"].as_u64()),
        (Some(4), Some(1), Some(2), Some(2), Some(1)), "{b}");
    assert_eq!(b["federacao_ms"], 1.0);
    assert_eq!(b["total_ms"], 0.7);
    assert_eq!(b["atrasada"], false);
    assert_eq!(v.json(15 * S)["remotas"]["no-b"]["atrasada"], true, "idade > 2 × limite");
    // as dos pares não mexem nas locais
    assert_eq!((j["concluidas"].as_u64(), j["perdidas"].as_u64()), (Some(0), Some(0)));
    assert_eq!(j["ultima"], Value::Null);
}

fn msg(id: u64, ses: u64, t_inj: u64, inst: bool) -> Value {
    let mut m = Sonda::nova(id, ses, t_inj).json();
    m["tipo"] = json!("sonda");
    m["t_difusao"] = json!(t_inj + 2_000);
    if inst { m["instantaneo"] = json!(true); }
    m
}

#[test]
fn verificador_situacoes() {
    let a = "ws://x/ws".to_string();
    let mut v = Verificador::novo(3.0, std::slice::from_ref(&a));
    assert_eq!(v.situacao(&a, 0), Situacao::Desconectado);
    assert_eq!(v.situacao("outro", 0), Situacao::Desconectado);
    v.conectou(&a, 10 * S);
    assert_eq!(v.situacao(&a, 11 * S), Situacao::Esperando);
    assert_eq!(v.situacao(&a, 14 * S), Situacao::Atrasada, "conectado e nada chega");
    // a de inscrição (antiga) não prova nada
    assert!(!v.receber(&a, &msg(4, 1, 2 * S, true), 14 * S));
    assert_eq!(v.situacao(&a, 14 * S), Situacao::Atrasada);
    assert!(v.receber(&a, &msg(5, 1, 14 * S, false), 14 * S + 5_000));
    assert_eq!(v.situacao(&a, 15 * S), Situacao::Ok);
    assert!(v.tudo_ok(15 * S));
    assert_eq!(v.recebidas(&a), 1);
    // mensagens que não são sonda, ou ilegíveis
    assert!(!v.receber(&a, &json!({"tipo": "estado"}), 15 * S));
    assert!(!v.receber(&a, &json!({"tipo": "sonda", "id": 1}), 15 * S));
    // pulos, repetidas e reinício
    assert!(v.receber(&a, &msg(8, 1, 16 * S, false), 16 * S));
    assert!(!v.receber(&a, &msg(8, 1, 16 * S, false), 16 * S));
    assert!(v.receber(&a, &msg(1, 2, 17 * S, false), 17 * S));
    v.caiu(&a, "erro_leitura");
    v.caiu(&a, "erro_leitura");
    assert_eq!(v.situacao(&a, 17 * S), Situacao::Desconectado);
    assert!(!v.tudo_ok(17 * S));
    let j = v.json(17 * S);
    let x = &j["alvos"][&a];
    assert_eq!((x["recebidas"].as_u64(), x["puladas"].as_u64(), x["repetidas"].as_u64(), x["reinicios"].as_u64(), x["quedas"].as_u64(), x["conexoes"].as_u64()),
        (Some(3), Some(2), Some(1), Some(1), Some(1), Some(1)), "{x}");
    assert_eq!(x["situacao"], "desconectado");
    assert_eq!(x["ultima_queda"], "erro_leitura");
    assert_eq!(x["latencias_ms"]["servidor"]["p50"], 2.0);
    assert_eq!(x["latencias_ms"]["ponta"]["max"], 5.0);
    assert_eq!(j["ok"], false);
    assert_eq!(j["limite_s"], 3.0);
}

#[test]
fn verificador_separa_as_dos_pares() {
    let a = "a".to_string();
    let mut v = Verificador::novo(3.0, std::slice::from_ref(&a));
    v.conectou(&a, 0);
    let mut m = Sonda::remota("no-c", 3, 1, S, S + 10).json();
    m["tipo"] = json!("sonda");
    assert!(!v.receber(&a, &m, S));
    m["id"] = json!(4);
    m["instantaneo"] = json!(true);
    assert!(!v.receber(&a, &m, S));
    let j = v.json(S);
    assert_eq!(j["alvos"]["a"]["remotas"]["no-c"], json!({"recebidas": 1, "ultima": 4}));
    assert_eq!(j["alvos"]["a"]["recebidas"], 0, "a do par não prova o caminho deste servidor");
    // alvo que não estava na lista entra ao conectar
    v.conectou("novo", 0);
    assert_eq!(v.situacao("novo", 1), Situacao::Esperando);
}

#[test]
fn situacao_texto() {
    assert_eq!([Situacao::Ok, Situacao::Esperando, Situacao::Atrasada, Situacao::Desconectado].map(Situacao::texto), ["ok", "esperando", "atrasada", "desconectado"]);
}
