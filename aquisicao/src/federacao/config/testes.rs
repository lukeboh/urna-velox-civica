    use super::*;

    const BASE: &str = r#"{"versao":1,"nos":[
        {"id":"no-a","orgao":"Órgão A","ip_saida":"200.1.2.3"},
        {"id":"no-b","orgao":"Órgão B","ip_saida":"189.4.5.6"}]}"#;

    #[test]
    fn padroes() {
        let f = Federacao::de_json(BASE).unwrap();
        assert_eq!(f.nos[0].teto_rps, 100.0);
        assert_eq!((f.teto_coleta_rps, f.unidade_peso_rps, f.batimento_s, f.morte_s, f.pausa_429_s), (95.0, 5.0, 10, 30, 600));
        assert_eq!(f.no("no-b").unwrap().orgao, "Órgão B");
    }

    #[test]
    fn um_no_por_orgao() {
        let t = BASE.replace("Órgão B", " órgão a ");
        assert!(Federacao::de_json(&t).unwrap_err().to_string().contains("um nó por órgão"));
    }

    #[test]
    fn mesmo_orgao_so_com_um_coletando() {
        let t = BASE.replace("Órgão B", "Órgão A");
        assert!(Federacao::de_json(&t).is_err());
        let t = t.replace(r#""ip_saida":"189.4.5.6""#, r#""ip_saida":"189.4.5.6","coleta":false"#);
        let f = Federacao::de_json(&t).unwrap();
        assert!(f.nos[0].coleta && !f.nos[1].coleta);
    }

    #[test]
    fn teto_acima_de_100_so_com_acordo() {
        let t = BASE.replace(r#""ip_saida":"189.4.5.6""#, r#""ip_saida":"189.4.5.6","teto_rps":150"#);
        assert!(Federacao::de_json(&t).unwrap_err().to_string().contains("acordo"));
        let t = BASE.replace(r#""ip_saida":"189.4.5.6""#, r#""ip_saida":"189.4.5.6","teto_rps":150,"acordo":"Autorização 123/2026""#);
        assert_eq!(Federacao::de_json(&t).unwrap().nos[1].teto_rps, 150.0);
    }

    #[test]
    fn recusas() {
        assert!(Federacao::de_json(&BASE.replace("200.1.2.3", "não-é-ip")).is_err());
        assert!(Federacao::de_json(&BASE.replace("\"no-b\"", "\"no-a\"")).is_err());
        assert!(Federacao::de_json(&BASE.replace("\"no-b\"", "\"no b\"")).is_err());
        assert!(Federacao::de_json(&BASE.replace("\"versao\":1", "\"versao\":2")).is_err());
        assert!(Federacao::de_json(&BASE.replace("]}", "],\"teto_coleta_rps\":120}")).is_err());
    }
