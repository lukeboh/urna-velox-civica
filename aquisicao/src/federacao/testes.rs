    use super::*;

    #[test]
    fn tres_orgaos_pelo_storage_local() {
        let d = std::env::temp_dir().join(format!("uv-federacao-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let a = armazem::ArmazemLocal::novo(&d).unwrap();
        let cfg = config::Federacao::de_json(r#"{"versao":1,"nos":[
            {"id":"no-a","orgao":"A","ip_saida":"127.0.0.2"},
            {"id":"no-b","orgao":"B","ip_saida":"127.0.0.3"},
            {"id":"no-c","orgao":"C","ip_saida":"127.0.0.4"}]}"#).unwrap();
        for (no, ip) in [("no-a", "127.0.0.2"), ("no-b", "127.0.0.3"), ("no-c", "127.0.0.4")] {
            publicar_batimento(&a, &Batimento { no: no.into(), ip_saida_medido: ip.into(), pausa_ate: 0, seq_bus: 0, versao: "t".into(), contador: 0, sincronizando: false }).unwrap();
        }
        // um nó tentando publicar o heartbeat de outro: ignorado
        a.gravar("nos/no-a.json.bak", b"{}").unwrap();
        a.gravar("nos/intruso.json", br#"{"no":"no-b","ip_saida_medido":"9.9.9.9"}"#).unwrap();
        let e = estado_atual(&cfg, &a).unwrap();
        assert_eq!(e.ativos.len(), 3, "{:?}", e.fora);
        assert_eq!(e.rps_total(), 285.0);
        // cada nó, sozinho, chega à mesma partição
        let e2 = estado_atual(&cfg, &a).unwrap();
        for m in 0..500u32 { assert_eq!(e.particao.dono(m, 1, 1), e2.particao.dono(m, 1, 1)); }
        let _ = std::fs::remove_dir_all(&d);
    }
