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
