//! BUs reais avulsos (`tests/fixtures/bu`): o leitor reproduz o texto esperado (`.txt`, gerado por
//! `fixtures/gerar.sh` e conferido com o leitor da v0.0.22) e a seção anulada vira nulos.

mod comum;
use std::sync::atomic::Ordering::Relaxed;
use urna_velox::bu::{ler_bu, Texto};
use urna_velox::catalogo::Catalogo;
use urna_velox::estado::totalizar_lote;
use urna_velox::resultado::totais_municipio;

fn bus() -> Vec<std::path::PathBuf> {
    let mut v: Vec<_> = std::fs::read_dir(comum::fixtures().join("bu")).unwrap().flatten().map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "dat")).collect();
    v.sort();
    v
}

#[test]
fn leitor_reproduz_o_texto_esperado() {
    let v = bus();
    assert_eq!(v.len(), 5);
    for p in v {
        let mut t = Texto(String::new());
        ler_bu(&std::fs::read(&p).unwrap(), &mut t).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
        let esperado = std::fs::read_to_string(p.with_extension("txt")).unwrap().replace("\r\n", "\n");
        assert_eq!(t.0, esperado, "{}", p.display());
    }
}

#[test]
fn busa_2026_tem_votos_para_presidente() {
    let p = comum::fixtures().join("bu").join("o03220mg5007500470041-busa.dat");
    let mut t = Texto(String::new());
    ler_bu(&std::fs::read(p).unwrap(), &mut t).unwrap();
    assert!(t.0.starts_with("pleito 3220 municipio 50075 zona 0047 secao 0041\n"));
    assert!(t.0.contains("  cargo 1 comparecimento"));
}

#[test]
fn truncar_qualquer_bu_real_nunca_entra_em_panico() {
    for p in bus() {
        let b = std::fs::read(&p).unwrap();
        for n in (0..b.len()).step_by(7) {
            let _ = ler_bu(&b[..n], &mut Texto(String::new()));
        }
    }
}

/// MA 07315/0023/0354 (1º turno de 2024): seção anulada pela Junta. Com a seção marcada como anulada, todos os
/// votos do BU vão para os nulos (como o `vsan` da divulgação); sem a marca, os votos nominais aparecem.
#[test]
fn secao_anulada_conta_tudo_como_nulo() {
    let p = comum::fixtures().join("bu").join("o00452ma0731500230354-bu.dat");
    let (dir, mem) = comum::pacote_de_soltos("anulada", &[p]);
    for anulada in [false, true] {
        let (mut cat, _) = Catalogo::dos_pacotes(&mem, &[]);
        if anulada { cat.anuladas.insert(urna_velox::chave(7315, 23, 354)); }
        let (est, _) = totalizar_lote(&cat, &mem, 1);
        assert_eq!((est.bus.load(Relaxed), est.cargos_invalidos.load(Relaxed)), (1, 0));
        let t = totais_municipio(&cat, &est, 0, cat.ci(11).unwrap()).unwrap();
        assert!(t.comparecimento > 0);
        if anulada {
            assert!(t.nominais.is_empty() && t.branco == 0 && t.legenda == 0);
            assert_eq!(t.nulo, t.comparecimento);
        } else {
            assert!(t.nominais.values().sum::<u64>() > 0);
            assert_eq!(t.nominais.values().sum::<u64>() + t.branco + t.nulo, t.comparecimento);
        }
    }
    let _ = std::fs::remove_dir_all(dir);
}
