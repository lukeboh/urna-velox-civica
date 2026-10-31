use super::*;

fn k(uf: &[u8; 2], mu: u32, zn: u16, se: u16) -> ChaveUf { (*uf, urna_velox::chave(mu, zn, se)) }

#[test]
fn primeira_secao_e_contagem() {
    let munis: HashSet<([u8; 2], u32)> = [(*b"sp", 71072)].into_iter().collect();
    let mut m = Marcos::abrir(None, HashMap::from([(71072, "SÃO PAULO".to_string())]), munis, 0).unwrap();
    let v = m.aplicado(&SecaoAplicada { k: k(b"sp", 71072, 1, 10), hora: 0, seq: 1 });
    let abrs: Vec<&str> = v.iter().map(|x| x["abrangencia"].as_str().unwrap()).collect();
    assert_eq!(abrs, ["br", "sp", "sp71072"]);
    assert_eq!((v[2]["zona"].as_u64(), v[2]["secao"].as_u64(), v[2]["municipio_nome"].as_str()), (Some(1), Some(10), Some("SÃO PAULO")));
    assert!(m.aplicado(&SecaoAplicada { k: k(b"sp", 71072, 2, 1), hora: 0, seq: 2 }).is_empty(), "só a primeira");
    let v = m.aplicado(&SecaoAplicada { k: k(b"rj", 60011, 2, 1), hora: 0, seq: 10_000 });
    assert_eq!(v.iter().map(|x| x["classe"].as_str().unwrap()).collect::<Vec<_>>(), ["primeira_secao", "bu_marco"]);
    assert_eq!(m.encerrada().len(), 1);
    assert!(m.encerrada().is_empty(), "uma vez por seção");
    assert!(dentro(&Abrangencia::Brasil, &k(b"zz", 1, 1, 1)) && dentro(&Abrangencia::Uf(*b"rj"), &k(b"rj", 1, 1, 1)));
    assert!(!dentro(&Abrangencia::Municipio(*b"sp", 71072), &k(b"sp", 71073, 1, 1)));
}

#[test]
fn csv_ida_e_volta() {
    let d = std::env::temp_dir().join(format!("uv-marcos-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    {
        let mut m = Marcos::abrir(Some(&d), HashMap::new(), HashSet::new(), 0).unwrap();
        m.aplicado(&SecaoAplicada { k: k(b"am", 2550, 1, 550), hora: 1_000, seq: 1 });
    }
    let mut m = Marcos::abrir(Some(&d), HashMap::from([(2550, "MANAUS".to_string())]), HashSet::new(), 0).unwrap();
    assert_eq!(m.historico.len(), 2);
    assert_eq!((m.historico[1]["abrangencia"].as_str(), m.historico[1]["municipio_nome"].as_str(), m.historico[1]["secao"].as_u64()), (Some("am"), Some("MANAUS"), Some(550)));
    assert!(m.aplicado(&SecaoAplicada { k: k(b"am", 2550, 1, 551), hora: 1_001, seq: 2 }).is_empty(), "retomada: a primeira seção não se repete");
    let _ = std::fs::remove_dir_all(&d);
}
