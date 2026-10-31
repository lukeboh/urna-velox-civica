    use super::*;
    #[test]
    fn datas() {
        let a = segundos("04/10/2026 23:59:30").unwrap();
        let b = segundos("05/10/2026 00:00:30").unwrap();
        assert_eq!(b - a, 60);
        assert_eq!(hhmm(a), "2026-10-04 23:59");
        assert_eq!(hhmm(b), "2026-10-05 00:00");
        assert_eq!(iso_brasilia(segundos("27/10/2024 18:40:05").unwrap()), "2024-10-27T18:40:05-03:00");
        assert_eq!(segundos(" "), None);
    }
    #[test]
    fn csv_de_metricas() {
        let p = std::env::temp_dir().join(format!("uv-metricas-{}.csv", std::process::id()));
        std::fs::write(&p, "t_s,uf,municipio,zona,secao,lat_aux_ms,lat_bu_ms,bytes_bu,tentativas,situacao,recebido_tse,disponivel_cs,erro\n\
            1.0,ma,7315,23,354,1,0,0,1,Excluído,06/10/2024 20:23:07,06/10/2024 21:45:24,\n\
            2.0,zz,29629,1,3014,1,0,0,1,,, ,sem BU (situação Nao instalada)\n\
            3.0,ac,1031,8,12,1,1,7000,1,Totalizado,06/10/2024 17:05:00,06/10/2024 17:06:00,\n").unwrap();
        let m = Metricas::ler(&[&p]).unwrap();
        let _ = std::fs::remove_file(&p);
        assert!(m.anuladas.contains(&crate::chave(7315, 23, 354)));
        assert!(m.nao_instaladas.contains(&(*b"zz", crate::chave(29629, 1, 3014))));
        assert_eq!(m.recebido.len(), 2);
        assert_eq!(m.nao_instaladas_por_municipio().get(&(*b"zz", 29629)), Some(&1));
    }
