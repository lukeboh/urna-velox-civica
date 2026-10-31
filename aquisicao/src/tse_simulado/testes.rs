    use super::*;
    #[test]
    fn atraso_tem_a_mediana_e_o_p90_pedidos() {
        let op = Opcoes::padrao(PathBuf::new());
        let mut v: Vec<f64> = (0..20000u64).map(|i| atraso(&op, &(*b"sp", i * 7919))).collect();
        v.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let (p50, p90) = (v[10000], v[18000]);
        assert!((p50 - 120.0).abs() < 8.0, "p50 {p50}");
        assert!((p90 - 360.0).abs() < 20.0, "p90 {p90}");
        assert_eq!(atraso(&op, &(*b"am", 42)), atraso(&op, &(*b"am", 42)), "determinístico");
    }
    #[test]
    fn progressivo_troca_os_numeros_no_molde() {
        let molde: Value = serde_json::from_str(r#"{"ele":"620","t":"2","tf":"s","and":"f","esae":"n","dg":"29/09/2026","hg":"19:26:34",
            "carg":[{"cd":"11","agr":[{"par":[{"cand":[{"n":"70","seq":"1","e":"s","st":"Eleito","vap":"576171"},{"n":"22","seq":"2","e":"n","st":"Não eleito","vap":"479297"}]}]}]}],
            "s":{"ts":"10","st":"10","sni":"1"},"e":{"te":"1000","esni":"20","c":"800"},"v":{"tv":"800","vb":"10"}}"#).unwrap();
        let p = json!({"estado": "apurando", "secoes": {"total": 10, "apuradas": 5}, "apurado": 480, "comparecimento": 400, "vb": 5, "vn": 7, "vnt": 0, "validos": 388, "legenda": 0,
            "cand": [{"n": 22, "v": 200, "pct": 51.55, "mat": null}, {"n": 70, "v": 188, "pct": 48.45, "mat": null}]});
        let g = urna_velox::metricas::segundos("27/10/2024 18:00:15").unwrap();
        let o = progressivo(&molde, &p, &Abrangencia::Municipio(*b"am", 2550), g);
        let of = urna_velox::oficial::Oficial::de_json(&o);
        assert_eq!((of.geracao.as_str(), of.finalizada, of.andamento.as_str(), of.md.as_str()), ("27/10/2024 18:00:15", false, "p", "n"));
        assert_eq!((of.secoes, of.secoes_totalizadas, of.nao_instaladas, of.comparecimento, of.abstencao, of.vb, of.validos), (10, 5, 1, 400, 80, 5, 388));
        let c22 = of.cand.iter().find(|c| c.numero == 22).unwrap();
        let c70 = of.cand.iter().find(|c| c.numero == 70).unwrap();
        assert_eq!((c22.vap, c22.seq, c22.pvap, c22.eleito, c22.st.as_str()), (200, 1, Some(51.55), false, ""));
        assert_eq!((c70.vap, c70.seq), (188, 2));
        assert!(o.get("esae").is_none());
    }
