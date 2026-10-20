//! Municípios inteiros de 2024 (`tests/fixtures/2024-1t` e `2024-2t`) totalizados pelo motor e conferidos com
//! os arquivos oficiais de divulgação: votos de cada candidato e opção, brancos, nulos, nulos técnicos,
//! comparecimento e seções totalizadas.

mod comum;
use comum::{conferir_pasta, conferir_publicacoes_pasta, fixtures, totalizar};

fn conjunto(nome: &str, threads: usize) -> comum::Conjunto {
    let d = fixtures().join(nome);
    totalizar(&d.join("dados"), &[d.join("metricas.csv")], &[], threads)
}

#[test]
fn primeiro_turno_2024_identico_ao_oficial() {
    let c = conjunto("2024-1t", 1);
    let r = conferir_pasta(&c, &fixtures().join("2024-1t").join("oficial"));
    assert!(r.ok(), "{}\n{}", r.resumo(), r.detalhes(20));
    // 8 municípios (Prefeito) + 2 consultas populares (RR 03158, cargo 55; RS 84824, cargo 52)
    assert_eq!((r.arquivos, r.por_cargo.get(&11).copied(), r.por_cargo.get(&55).copied(), r.por_cargo.get(&52).copied()), (10, Some((8, 0)), Some((1, 0)), Some((1, 0))));
    assert_eq!(r.nao_finalizados, vec!["se31313-c0011-e000619-u.json".to_string()]);
    // situação de todos os candidatos e opções dos 9 arquivos finalizados, calculada pelo motor
    assert_eq!(r.situacoes.values().map(|x| x.1).sum::<usize>(), 0);
    assert_eq!((r.situacoes[&11], r.situacoes[&55], r.situacoes[&52]), ((16, 0), (2, 0), (2, 0)));
}

/// A publicação no formato JSON documentado traz os mesmos números e situações do oficial.
#[test]
fn publicacao_json_identica_ao_oficial() {
    for (nome, arquivos, situacoes) in [("2024-1t", 10, 20), ("2024-2t", 1, 2)] {
        let c = conjunto(nome, 1);
        let r = conferir_publicacoes_pasta(&c, &fixtures().join(nome).join("oficial"));
        assert!(r.ok(), "{nome}: {}\n{}", r.resumo(), r.detalhes(20));
        assert_eq!((r.arquivos, r.situacoes.values().map(|x| x.0).sum::<usize>()), (arquivos, situacoes), "{nome}");
    }
}

/// Campos e estados da publicação (formato v1).
#[test]
fn publicacao_json_campos() {
    use urna_velox::candidatos::Candidatos;
    use urna_velox::oficial::Abrangencia;
    use urna_velox::resultado::{Momento, Publicador};
    let c = conjunto("2024-1t", 1);
    let cad = Candidatos::dos_arquivos_oficiais(&fixtures().join("2024-1t").join("oficial")).unwrap();
    let p = Publicador::novo(&c.cat, Some(&cad), Some(&c.met));
    let fim = Momento { encerrada: true, seq: 333, ultimo_bu: urna_velox::metricas::segundos("06/10/2024 20:00:00"), totalizado_ms: None };
    let v = p.valor(&c.est, None, &Abrangencia::Municipio(*b"ac", 1031), 11, fim).unwrap();
    assert_eq!((v["v"].as_u64(), v["eleicao"].as_u64(), v["turno"].as_u64(), v["abr"].as_str(), v["circ"].as_str(), v["estado"].as_str()),
        (Some(1), Some(619), Some(1), Some("ac01031"), Some("ac01031"), Some("definido")));
    assert_eq!(v["ultimo_bu"].as_str(), Some("2024-10-06T20:00:00-03:00"));
    assert_eq!((v["secoes"]["total"].as_u64(), v["secoes"]["apuradas"].as_u64(), v["eleitorado"].as_u64()), (Some(14), Some(14), Some(3918)));
    let cand = v["cand"].as_array().unwrap();
    assert_eq!((cand[0]["n"].as_u64(), cand[0]["v"].as_u64(), cand[0]["st"].as_str(), cand[0]["pct"].as_f64()), (Some(15), Some(1642), Some("Eleito"), Some(51.75)));
    let soma: f64 = cand.iter().filter_map(|x| x["pct"].as_f64()).sum();
    assert!((soma - 100.0).abs() < 0.05);
    assert_eq!(p.valor(&c.est, None, &Abrangencia::Municipio(*b"sp", 62081), 11, fim).unwrap()["estado"].as_str(), Some("sem_eleito"));
    assert_eq!(p.valor(&c.est, None, &Abrangencia::Municipio(*b"se", 31313), 11, fim).unwrap()["estado"].as_str(), Some("apurando"));
    // durante a apuração (não encerrada) a situação fica vazia
    let v = p.valor(&c.est, None, &Abrangencia::Municipio(*b"ac", 1031), 11, Momento { encerrada: false, seq: 1, ultimo_bu: None, totalizado_ms: None }).unwrap();
    assert_eq!((v["estado"].as_str(), v["cand"][0]["st"].as_str(), v["ultimo_bu"].is_null()), (Some("apurando"), Some(""), true));
    assert_eq!(p.nome_arquivo(&Abrangencia::Municipio(*b"ac", 1031), 11), "ac01031-c0011-e000619.json");
}

/// Os casos especiais dos fixtures, com a situação calculada pelo motor.
#[test]
fn casos_especiais_da_situacao() {
    use urna_velox::candidatos::Candidatos;
    use urna_velox::oficial::Abrangencia;
    use urna_velox::resultado::totais;
    use urna_velox::situacao::{ordem, situacao_final, votos_dos_candidatos, Situacao};
    let c = conjunto("2024-1t", 1);
    let cad = Candidatos::dos_arquivos_oficiais(&fixtures().join("2024-1t").join("oficial")).unwrap();
    let calc = |uf: &[u8; 2], mu: u32| {
        let abr = Abrangencia::Municipio(*uf, mu);
        let circ = cad.circunscricao(11, &abr).unwrap();
        let v = votos_dos_candidatos(circ, &totais(&c.cat, &c.est, &abr, 11).unwrap());
        (circ, situacao_final(circ, &v), v)
    };
    // MG 46191: os dois primeiros empatados; é eleito o mais idoso
    let (circ, st, v) = calc(b"mg", 46191);
    let o = ordem(circ, &v);
    assert_eq!(v[o[0]], v[o[1]]);
    assert!(circ.candidatos[o[0]].nascimento < circ.candidatos[o[1]].nascimento);
    assert_eq!((st[o[0]], st[o[1]]), (Situacao::Eleito, Situacao::NaoEleito));
    // SP 62081: ninguém eleito (art. 224)
    let (_, st, _) = calc(b"sp", 62081);
    assert!(st.iter().all(|&s| s == Situacao::NaoEleito));
    // RS 85766: candidato anulado com votos, e mesmo assim há eleito
    let (circ, st, v) = calc(b"rs", 85766);
    assert!(circ.candidatos.iter().zip(&v).any(|(c, &q)| !c.destinacao.apto() && q > 0));
    assert_eq!(st.iter().filter(|&&s| s == Situacao::Eleito).count(), 1);
    // SE 31313: totalização não finalizada, situação pendente
    let (_, st, _) = calc(b"se", 31313);
    assert!(st.iter().all(|&s| s == Situacao::Pendente));
}

#[test]
fn segundo_turno_2024_identico_ao_oficial() {
    let c = conjunto("2024-2t", 1);
    let r = conferir_pasta(&c, &fixtures().join("2024-2t").join("oficial"));
    assert!(r.ok(), "{}\n{}", r.resumo(), r.detalhes(20));
    assert_eq!(r.arquivos, 1);
    assert_eq!(r.situacoes[&11], (2, 0));
}

/// Sem a seção anulada (MA 07315/0023/0354), só MA 07315 diverge: os votos dela deixam de ser nulos.
#[test]
fn sem_a_secao_anulada_ma07315_diverge() {
    let d = fixtures().join("2024-1t");
    let mut c = conjunto("2024-1t", 1);
    c.met.anuladas.clear();
    let (mut cat, _) = urna_velox::catalogo::Catalogo::dos_pacotes(&c.mem, &[]);
    cat.anuladas.clear();
    let (est, _) = urna_velox::estado::totalizar_lote(&cat, &c.mem, 1);
    c.cat = cat;
    c.est = est;
    let r = conferir_pasta(&c, &d.join("oficial"));
    let div: Vec<&str> = r.divergentes.iter().map(|x| x.arquivo.as_str()).collect();
    assert_eq!(div, vec!["ma07315-c0011-e000619-u.json"]);
    assert!(r.divergentes[0].difs.iter().any(|x| x.starts_with("nulos (vn)")));
}

/// O estado não depende do nº de threads nem da ordem de chegada dos BUs.
#[test]
fn checksum_igual_com_1_4_e_16_threads() {
    for nome in ["2024-1t", "2024-2t"] {
        let cs: Vec<u64> = [1, 4, 16].iter().map(|&t| conjunto(nome, t).est.checksum()).collect();
        assert!(cs[0] != 0 && cs.iter().all(|&x| x == cs[0]), "{nome}: {cs:?}");
    }
}
