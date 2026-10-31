    use super::*;
    #[test]
    fn nomes_e_hashes() {
        assert_eq!(chave_do_nome("o03220df9701200150300-bu.dat"), Some((3220, *b"df", 97012, 15, 300)));
        let h = "35307a474c3238496d3479377954436274424e44546471474f6477656a4539704a676730765342684b58733d";
        let s = sha256_do_hash_dir(h);
        assert_eq!(s[0], 0xe7);
        assert_eq!(hash_dir_do_sha256(&s), h);
        let a = Alvo::novo("https://resultados.tse.jus.br", "oficial", "ele2026", 3220);
        assert_eq!(a.url_aux("df", 97012, 15, 300), "https://resultados.tse.jus.br/oficial/ele2026/arquivo-urna/3220/dados/df/97012/0015/0300/p003220-df-m97012-z0015-s0300-aux.json");
    }

    #[test]
    fn bu_do_sistema_de_apuracao() {
        // aux.json real (MG 50075/0047/0041, 1º turno 2026): seção apurada pelo SA, sem -bu.dat
        let j = r#"{ "st" : "Totalizada", "hashes" : [ { "hash" : "616e", "dr" : "04/10/2026", "hr" : "22:29:08", "st" : "Totalizado", "arq" : [
            { "nm" : "o03220mg5007500470041-rdv.dat", "tp" : "rdv" }, { "nm" : "o03220mg5007500470041-logsa.jez", "tp" : "logsa" },
            { "nm" : "o03220mg5007500470041-sa.vsc", "tp" : "sa" }, { "nm" : "o03220mg5007500470041-busa.dat", "tp" : "busa" } ] } ] }"#;
        let a: Aux = serde_json::from_str(j).unwrap();
        assert_eq!(a.escolher_bu().map(|x| x.1), Some("o03220mg5007500470041-busa.dat"));
        assert_eq!(chave_do_nome("o03220mg5007500470041-busa.dat"), Some((3220, *b"mg", 50075, 47, 41)));
        assert!(crate::zip::e_bu("o03220mg5007500470041-busa.dat") && !crate::zip::e_bu("o03220mg5007500470041-sa.vsc"));
    }

    /// Os modelos padrão reproduzem, caractere a caractere, os endereços de antes (fórmulas da 0.8.32).
    #[test]
    fn rotas_padrao_iguais_as_de_antes() {
        let a = Alvo::novo("https://resultados.tse.jus.br/", "oficial", "ele2026", 3221);
        let (b, amb, c, p) = ("https://resultados.tse.jus.br", "oficial", "ele2026", 3221u32);
        let raiz = format!("{b}/{amb}/{c}/arquivo-urna/{p}");
        assert_eq!(a.url_ele_c(), format!("{b}/{amb}/comum/config/ele-c.json"));
        for uf in ["sp", "zz", "df"] {
            assert_eq!(a.url_cs(uf), format!("{raiz}/config/{uf}/{uf}-p{p:06}-cs.json"));
            for (mu, zn, se) in [(71072u32, 1u16, 9u16), (1031, 250, 1234), (99999, 9999, 9999)] {
                let dir = format!("{raiz}/dados/{uf}/{mu:05}/{zn:04}/{se:04}");
                assert_eq!(a.url_aux(uf, mu, zn, se), format!("{dir}/p{p:06}-{uf}-m{mu:05}-z{zn:04}-s{se:04}-aux.json"));
                assert_eq!(a.url_arquivo(uf, mu, zn, se, "6162", "o03221sp7107200010009-bu.dat"), format!("{dir}/6162/o03221sp7107200010009-bu.dat"));
            }
        }
        assert_eq!(a.url_mun_cm(620), format!("{b}/{amb}/{c}/620/config/mun-e{:06}-cm.json", 620));
        assert_eq!(a.url_u_json(620, "br", 1, "br"), format!("{b}/{amb}/{c}/620/dados/br/br-c{:04}-e{:06}-u.json", 1, 620));
        assert!(Rotas::default().padrao());
        assert!(Rotas::default().validar(b).is_ok());
        assert_eq!(Rotas::de_json("{}").unwrap(), Rotas::default(), "arquivo vazio = padrão");
    }

    #[test]
    fn rotas_trocadas_e_base_dos_arquivos() {
        let mut r = Rotas::de_json(r#"{"base_arquivos": "https://cdn.tse.jus.br/espelho/",
            "arquivo": "{base_arquivos}/{amb}/{ciclo}/bu/{pleito}/{uf}/{mun05}-{zona04}-{secao04}/{nome}"}"#).unwrap();
        assert!(r.validar("https://resultados.tse.jus.br").is_ok());
        let a = Alvo { rotas: std::sync::Arc::new(r.clone()), ..Alvo::novo("https://resultados.tse.jus.br", "oficial", "ele2026", 3221) };
        assert_eq!(a.url_arquivo("sp", 71072, 1, 9, "6162", "o-bu.dat"), "https://cdn.tse.jus.br/espelho/oficial/ele2026/bu/3221/sp/71072-0001-0009/o-bu.dat");
        assert!(a.url_aux("sp", 71072, 1, 9).starts_with("https://resultados.tse.jus.br/oficial/ele2026/arquivo-urna/3221/dados/sp/"), "aux continua na base");
        // base local + arquivos remotos (ou o contrário): recusado
        assert!(r.validar("http://127.0.0.1:18080").is_err());
        r.base_arquivos = Some("http://127.0.0.1:9000".into());
        assert!(r.validar("http://127.0.0.1:18080").is_ok());
        assert!(r.validar("https://resultados.tse.jus.br").is_err());
    }

    #[test]
    fn rotas_recusadas() {
        let ruim = |campo: &str, v: &str| {
            let mut j = serde_json::json!({});
            j[campo] = v.into();
            Rotas::de_json(&j.to_string()).unwrap().validar("https://resultados.tse.jus.br").is_err()
        };
        assert!(ruim("arquivo", "https://evil.example/{nome}"), "servidor escrito no modelo");
        assert!(ruim("aux", "{base}/x/{uf}/{mun05}/{zona04}"), "falta {{secao04}}");
        assert!(ruim("cs", "{base}/{uf}/{secao04}.json"), "variável de seção no cs");
        assert!(ruim("cs", "{base}/{uf}/../../etc"), "..");
        assert!(ruim("cs", "{base}/{uf}?x=1"), "query");
        assert!(ruim("cs", "{base}/{uf}/{desconhecida}"));
        assert!(ruim("cs", "{base}/{uf"));
        assert!(ruim("arquivo", "{base}/{hash}"), "sem {{nome}}");
        assert!(ruim("base_arquivos", "ftp://x"));
        assert!(ruim("base_arquivos", "https://x/$(id)"));
        assert!(!ruim("cs", "{base}/{amb}/novo/{pleito}/{uf}/secoes-{uf}.json"));
    }

    #[test]
    fn modelo_com_nome_desconhecido_fica_intacto() {
        assert_eq!(montar_modelo("a/{x}/{y}/b{", &|n: &str, o: &mut String| { if n == "x" { o.push('1'); true } else { false } }), "a/1/{y}/b{");
    }
