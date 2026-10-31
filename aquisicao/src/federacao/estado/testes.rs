    use super::*;

    fn cfg() -> Federacao {
        Federacao::de_json(r#"{"versao":1,"nos":[
            {"id":"a","orgao":"A","ip_saida":"10.0.0.1"},
            {"id":"b","orgao":"B","ip_saida":"10.0.0.2"},
            {"id":"c","orgao":"C","ip_saida":"10.0.0.2"},
            {"id":"d","orgao":"D","ip_saida":"10.0.0.4"}]}"#).unwrap()
    }

    fn bat(no: &str, ip: &str, ha_s: u64, agora: SystemTime) -> BatimentoLido {
        BatimentoLido {
            batimento: Batimento { no: no.into(), ip_saida_medido: ip.into(), pausa_ate: 0, seq_bus: 0, versao: String::new(), contador: 0, sincronizando: false },
            recebido: agora - Duration::from_secs(ha_s),
        }
    }

    #[test]
    fn grupos_por_ip_e_pesos() {
        let agora = SystemTime::UNIX_EPOCH + Duration::from_secs(2_000_000_000);
        let bs = [bat("a", "10.0.0.1", 3, agora), bat("b", "10.0.0.2", 5, agora), bat("c", "10.0.0.2", 1, agora), bat("d", "10.0.0.4", 9, agora)];
        let e = calcular(&cfg(), &bs, agora);
        assert!(e.fora.is_empty(), "{:?}", e.fora);
        let a = e.ativo("a").unwrap();
        assert_eq!((a.nos_no_ip, a.rps, a.peso), (1, 95.0, 19));
        let b = e.ativo("b").unwrap();
        assert_eq!((b.nos_no_ip, b.rps, b.peso), (2, 47.5, 9));
        assert_eq!(e.ativo("c").unwrap().peso, 9);
        assert_eq!(e.rps_total(), 95.0 * 3.0);
    }

    #[test]
    fn mortos_pausados_divergentes_e_desconhecidos_ficam_fora() {
        let agora = SystemTime::UNIX_EPOCH + Duration::from_secs(2_000_000_000);
        let mut pausado = bat("b", "10.0.0.2", 2, agora);
        pausado.batimento.pausa_ate = 2_000_000_000 + 300;
        let bs = [
            bat("a", "10.0.0.1", 31, agora),       // morto (> 30 s)
            pausado,                                 // em pausa por 429
            bat("c", "200.0.0.9", 2, agora),         // saiu por outro IP (NAT não declarado)
            bat("x", "10.0.0.7", 1, agora),          // não está na federação
            // d nunca publicou
        ];
        let e = calcular(&cfg(), &bs, agora);
        assert!(e.ativos.is_empty());
        assert!(e.particao.vazia());
        let motivos: Vec<(&str, &Motivo)> = e.fora.iter().map(|(i, m)| (i.as_str(), m)).collect();
        assert!(motivos.contains(&("a", &Motivo::Morto)));
        assert!(motivos.contains(&("d", &Motivo::Morto)));
        assert!(motivos.contains(&("b", &Motivo::EmPausa)));
        assert!(motivos.contains(&("x", &Motivo::Desconhecido)));
        assert!(motivos.contains(&("c", &Motivo::IpDivergente { declarado: "10.0.0.2".into(), medido: "200.0.0.9".into() })));
    }

    #[test]
    fn pausa_de_um_ip_redistribui_e_o_outro_no_do_ip_ganha_capacidade() {
        let agora = SystemTime::UNIX_EPOCH + Duration::from_secs(2_000_000_000);
        // c em pausa: b fica sozinho no IP 10.0.0.2 e volta a ter os 95 req/s do IP
        let mut c = bat("c", "10.0.0.2", 1, agora);
        c.batimento.pausa_ate = 2_000_000_000 + 600;
        let e = calcular(&cfg(), &[bat("a", "10.0.0.1", 1, agora), bat("b", "10.0.0.2", 1, agora), c], agora);
        assert_eq!(e.ativo("b").unwrap().rps, 95.0);
        assert!(e.ativo("c").is_none());
        // nenhuma seção fica com quem está fora
        for m in 0..2000u32 { assert_ne!(e.particao.dono(m, 1, 1), Some("c")); }
    }

    #[test]
    fn no_que_so_recebe_fica_fora_da_particao() {
        let agora = SystemTime::UNIX_EPOCH + Duration::from_secs(2_000_000_000);
        let mut c = cfg();
        c.nos[3].coleta = false;
        let e = calcular(&c, &[bat("a", "10.0.0.1", 1, agora), bat("d", "10.0.0.4", 1, agora)], agora);
        assert!(e.ativo("d").is_none());
        assert!(e.fora.contains(&("d".to_string(), Motivo::SoRecebe)));
        for m in 0..2000u32 { assert_eq!(e.particao.dono(m, 1, 1), Some("a")); }
    }

    #[test]
    fn no_sincronizando_fica_fora_ate_alcancar_os_pares() {
        let agora = SystemTime::UNIX_EPOCH + Duration::from_secs(2_000_000_000);
        let mut d = bat("d", "10.0.0.4", 1, agora);
        d.batimento.sincronizando = true;
        let e = calcular(&cfg(), &[bat("a", "10.0.0.1", 1, agora), d.clone()], agora);
        assert!(e.ativo("d").is_none());
        assert!(e.fora.contains(&("d".to_string(), Motivo::Sincronizando)));
        for m in 0..2000u32 { assert_eq!(e.particao.dono(m, 1, 1), Some("a")); }
        // alcançou: volta à partição
        d.batimento.sincronizando = false;
        assert!(calcular(&cfg(), &[bat("a", "10.0.0.1", 1, agora), d], agora).ativo("d").is_some());
        // o campo é opcional no JSON (heartbeats de versões anteriores)
        let j = r#"{"no":"a","ip_saida_medido":"10.0.0.1"}"#;
        assert!(!serde_json::from_str::<Batimento>(j).unwrap().sincronizando);
    }

    #[test]
    fn heartbeat_mais_recente_vale() {
        let agora = SystemTime::UNIX_EPOCH + Duration::from_secs(2_000_000_000);
        let mut velho = bat("a", "10.0.0.1", 20, agora);
        velho.batimento.pausa_ate = 2_000_000_000 + 600;
        let e = calcular(&cfg(), &[velho, bat("a", "10.0.0.1", 2, agora)], agora);
        assert!(e.ativo("a").is_some());
    }
