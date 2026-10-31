    use super::*;
    #[test]
    fn nomes_e_abrangencias() {
        assert_eq!(nome_arquivo("ac01031-c0011-e000619-u.json"), Some((Abrangencia::Municipio(*b"ac", 1031), 11, 619)));
        assert_eq!(nome_arquivo("br-c0001-e006257-u.json"), Some((Abrangencia::Brasil, 1, 6257)));
        assert_eq!(nome_arquivo("zz-c0001-e006257-u.json"), Some((Abrangencia::Uf(*b"zz"), 1, 6257)));
        assert_eq!(nome_arquivo("mun-e000619-cm.json"), None);
        assert_eq!(Abrangencia::Municipio(*b"ma", 7315).codigo(), "ma07315");
    }
    #[test]
    fn cargo_e_consulta() {
        let v: Value = serde_json::from_str(r#"{"ele":"633","t":"1","tf":"s","perg":[{"cd":"55","resp":[{"n":"1","ds":"SIM","st":"Eleito","vap":"4387"},{"n":"2","ds":"NÃO","st":"Não eleito","vap":"871"}]}],
            "s":{"ts":"25","st":"25"},"e":{"te":"6134","c":"5319"},"v":{"vb":"50","vn":"11","vnt":"0"}}"#).unwrap();
        let o = Oficial::de_json(&v);
        assert!(o.consulta && o.finalizada);
        assert_eq!((o.cargo, o.eleicao, o.turno, o.eleitorado, o.comparecimento, o.vb, o.vn, o.secoes_totalizadas), (55, 633, 1, 6134, 5319, 50, 11, 25));
        assert_eq!(o.cand.len(), 2);
        assert_eq!((o.cand[0].numero, o.cand[0].vap, o.cand[0].dvt.as_str()), (1, 4387, "Válido"));
        let v: Value = serde_json::from_str(r#"{"ele":"6259","t":"1","tf":"n","mntf":"Aguarde","carg":[{"cd":"5","nv":"2","agr":[{"par":[{"cand":[{"n":"277","nmu":"X","dt":"02/02/1961","dvt":"Válido","st":"","vap":"14463"}]}]}]}],"e":{},"v":{},"s":{}}"#).unwrap();
        let o = Oficial::de_json(&v);
        assert!(!o.consulta && !o.finalizada);
        assert_eq!((o.cargo, o.vagas, o.cand[0].numero, o.cand[0].nascimento.as_str()), (5, 2, 277, "02/02/1961"));
    }
    /// Campos da comparação com o TSE (etapa 7) e a leitura de memória (o mesmo resultado da leitura do arquivo).
    #[test]
    fn campos_da_comparacao_e_bytes() {
        let b = br#"{"ele":"6257","t":"1","tf":"n","and":"p","md":"s","dg":"04/10/2026","hg":"20:41:10","carg":[{"cd":"1","agr":[{"par":[{"cand":[
            {"n":"13","seq":"1","e":"s","vap":"900","pvap":"52,94","pvapn":"52.941176471"},{"n":"22","seq":"2","e":"n","vap":"800","pvap":"47,06"}]}]}]}],
            "s":{"ts":"10","st":"5","sni":"1"},"e":{"te":"2000","c":"1800","a":"150","esni":"50"},"v":{"vb":"50","vn":"40","vnt":"10","vv":"1700"}}"#;
        let o = Oficial::de_bytes("br-c0001-e006257-u.json", b).unwrap();
        assert_eq!((o.abr.clone(), o.cargo, o.eleicao), (Some(Abrangencia::Brasil), 1, 6257));
        assert_eq!((o.geracao.as_str(), o.andamento.as_str(), o.md.as_str()), ("04/10/2026 20:41:10", "p", "s"));
        assert_eq!((o.nao_instaladas, o.eleitorado_nao_instaladas, o.abstencao, o.validos), (1, 50, 150, 1700));
        assert_eq!((o.cand[0].seq, o.cand[0].eleito, o.cand[0].pvap), (1, true, Some(52.941176471)));
        assert_eq!((o.cand[1].seq, o.cand[1].eleito, o.cand[1].pvap), (2, false, Some(47.06)));
        let d = std::env::temp_dir().join(format!("uv-of-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let arq = d.join("br-c0001-e006257-u.json");
        std::fs::write(&arq, b).unwrap();
        let l = Oficial::ler(&arq).unwrap();
        let _ = std::fs::remove_dir_all(&d);
        assert_eq!((l.cand.len(), l.cand[0].vap, l.secoes_totalizadas, l.geracao), (2, 900, 5, o.geracao));
    }
