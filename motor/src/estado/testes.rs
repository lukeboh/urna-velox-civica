use super::*;
use std::collections::{BTreeMap, BTreeSet};

/// Prefeito: o mesmo número (15) é um candidato diferente em cada município; a UF soma por código,
/// e o catálogo sobrevive a salvar/carregar e a `restringir`.
#[test]
fn cargo_municipal_por_municipio() {
    let ufs = vec![*b"ac"];
    let munis = vec![(1015, 0), (1120, 0)];
    let mut cands = BTreeMap::new();
    cands.insert((0u8, 11u8), BTreeSet::new());
    let mut mc = BTreeMap::new();
    mc.insert((1015u32, 11u8), BTreeSet::from([15, 44]));
    mc.insert((1120u32, 11u8), BTreeSet::from([15, 22, 55]));
    let secoes = vec![crate::chave(1015, 2, 88), crate::chave(1120, 8, 1)];
    let cat = Catalogo::montar(452, ufs, munis, cands, mc, &[], secoes);
    assert!(cat.municipal[11]);
    assert_eq!(cat.cand_cod[0], vec![15, 22, 44, 55]);
    assert_eq!(cat.slots, (2 + EXTRA + 3 + EXTRA) as usize);
    let est = Estado::novo(&cat);
    let mut ag = Agregador::novo(&cat, &est);
    // município 1015: 15 = 7, 44 = 3; município 1120: 15 = 1, 55 = 2, 44 (não é candidato lá) = 1
    for (mu, zn, se, votos) in [(1015u32, 2u16, 88u16, vec![(15u32, 7u32), (44, 3)]), (1120, 8, 1, vec![(15, 1), (55, 2), (44, 1)])] {
        ag.chave_esperada = crate::chave(mu, zn, se);
        ag.secao(452, mu, zn, se);
        let comp: u32 = votos.iter().map(|v| v.1).sum();
        ag.cargo(11, comp);
        for (c, q) in votos { ag.voto(1, q, (c / 10) as u16, c); }
        ag.fim_cargo();
    }
    assert_eq!(est.desconhecidos.load(Relaxed), 1);
    let alertas = est.lista_alertas_vnt();
    assert_eq!(alertas.len(), 1);
    assert_eq!(alertas[0], AlertaVnt { cargo: 11, muni_cod: 1120, numero: 44, votos: 1 });
    let uf = est.consolidar_uf(&cat);
    assert_eq!(&uf[0][..4], &[8, 0, 3, 2]);
    assert_eq!(uf[0][4 + OFF_COMP as usize], 14);
    assert_eq!(uf[0][4 + OFF_VNT as usize], 1);
    let csv = csv_totais_municipio(&cat, &est, None);
    assert!(csv.contains("ac,1015,11,nominal,15,7\n") && csv.contains("ac,1120,11,nominal,15,1\n") && csv.contains("ac,1120,11,comparecimento,,4\n"));
    // retirar os mesmos votos (o mesmo caminho com sinal negativo) zera tudo, inclusive os alertas de votável
    // desconhecido; depois, reaplica para o resto do teste
    let ck = est.checksum();
    for sinal in [Sinal::Retirar, Sinal::Somar] {
        ag.sinal = sinal;
        for (mu, zn, se, votos) in [(1015u32, 2u16, 88u16, vec![(15u32, 7u32), (44, 3)]), (1120, 8, 1, vec![(15, 1), (55, 2), (44, 1)])] {
            ag.chave_esperada = crate::chave(mu, zn, se);
            ag.secao(452, mu, zn, se);
            let comp: u32 = votos.iter().map(|v| v.1).sum();
            ag.cargo(11, comp);
            for (c, q) in votos { ag.voto(1, q, (c / 10) as u16, c); }
            ag.fim_cargo();
        }
        if sinal == Sinal::Retirar {
            assert!(est.tally.iter().all(|x| x.load(Relaxed) == 0) && est.aptos.iter().all(|x| x.load(Relaxed) == 0));
            assert_eq!((est.checksum(), est.desconhecidos.load(Relaxed)), (0, 0));
            assert!(est.lista_alertas_vnt().is_empty());
        }
    }
    assert_eq!(est.checksum(), ck);
    assert_eq!(est.lista_alertas_vnt(), alertas);
    let p = std::env::temp_dir().join(format!("uvcat-teste-{}.txt", std::process::id()));
    cat.salvar(&p).unwrap();
    let c2 = Catalogo::carregar(&p).unwrap();
    let _ = std::fs::remove_file(&p);
    assert_eq!(c2.slots, cat.slots);
    assert_eq!(c2.candidatos(1, 0), &[15, 22, 55]);
    assert_eq!(cat.restringir(&[11]).slots, cat.slots);
}
