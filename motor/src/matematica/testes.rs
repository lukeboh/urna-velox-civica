use super::*;
use crate::situacao::testes::circ;
use Mat::{Eleito as E, Nao as N, SegundoTurno as T};
const ABS: Sistema = Sistema::MaioriaAbsoluta { base: Base::Aptos };
const NOM: Sistema = Sistema::MaioriaAbsoluta { base: Base::Nominal };
const S1: Sistema = Sistema::MaioriaSimples { vagas: 1 };
const S2: Sistema = Sistema::MaioriaSimples { vagas: 2 };
// 10 é o mais idoso, 30 o mais novo
const TRES: [(u32, bool, u32); 3] = [(10, true, 19600101), (20, true, 19700101), (30, true, 19800101)];

#[test]
fn tabela() {
    let casos: Vec<(&str, Circunscricao, Vec<u64>, u64, MatCirc, Vec<Mat>)> = vec![
        ("simples: diferença maior que o restante", circ(S1, false, &TRES), vec![100, 89, 0], 10, MatCirc::Eleito, vec![E, N, N]),
        ("simples: diferença igual ao restante, líder mais idoso", circ(S1, false, &TRES), vec![100, 90, 0], 10, MatCirc::Eleito, vec![E, N, N]),
        ("simples: diferença igual ao restante, líder mais novo", circ(S1, false, &[(10, true, 19800101), (20, true, 19600101)]), vec![100, 90], 10, MatCirc::Apurando, vec![N, N]),
        ("simples: diferença igual ao restante, sem data", circ(S1, false, &[(10, true, 0), (20, true, 19600101)]), vec![100, 90], 10, MatCirc::Apurando, vec![N, N]),
        ("simples: diferença menor que o restante", circ(S1, false, &TRES), vec![100, 95, 0], 10, MatCirc::Apurando, vec![N, N, N]),
        ("simples: R = 0 decide (empate pela idade)", circ(S1, false, &[(10, true, 19800101), (20, true, 19600101)]), vec![50, 50], 0, MatCirc::Eleito, vec![N, E]),
        ("simples: R = 0 com empate sem datas não decide", circ(S1, false, &[(10, true, 0), (20, true, 0)]), vec![50, 50], 0, MatCirc::Apurando, vec![N, N]),
        ("candidato único", circ(S1, false, &[(10, true, 0)]), vec![0], 1000, MatCirc::Eleito, vec![E]),
        ("2 vagas: cada eleitor dá 2 votos", circ(S2, false, &[(10, true, 0), (20, true, 0), (30, true, 0), (40, true, 0)]), vec![100, 80, 60, 50], 20, MatCirc::Apurando, vec![E, N, N, N]),
        ("2 vagas: as duas garantidas", circ(S2, false, &[(10, true, 0), (20, true, 0), (30, true, 0), (40, true, 0)]), vec![100, 80, 60, 50], 19, MatCirc::Eleito, vec![E, E, N, N]),
        ("2 vagas: o 2º precisa de dois adversários juntos", circ(S2, false, &[(10, true, 0), (20, true, 0), (30, true, 0), (40, true, 0)]), vec![100, 80, 79, 70], 10, MatCirc::Apurando, vec![E, N, N, N]),
        ("absoluta: líder com mais da metade mesmo com o restante contra", circ(ABS, false, &TRES), vec![60, 20, 10], 9, MatCirc::Eleito, vec![E, N, N]),
        ("absoluta: ainda pode haver 2º turno", circ(ABS, false, &TRES), vec![60, 20, 10], 30, MatCirc::Apurando, vec![N, N, N]),
        ("absoluta: matematicamente 2º turno", circ(ABS, false, &TRES), vec![40, 35, 25], 0, MatCirc::SegundoTurno, vec![T, T, N]),
        ("absoluta: 2º turno certo, 2ª vaga ainda em disputa", circ(ABS, false, &TRES), vec![400, 300, 295], 10, MatCirc::SegundoTurno, vec![T, N, N]),
        ("absoluta: ninguém alcança a maioria mas o restante ainda pode", circ(ABS, false, &TRES), vec![40, 35, 25], 21, MatCirc::Apurando, vec![N, N, N]),
        ("absoluta sobre a votação nominal (anulado na base)", circ(NOM, false, &[(10, true, 0), (20, true, 0), (30, false, 0)]), vec![60, 20, 30], 0, MatCirc::Eleito, vec![E, N, N]),
        ("art. 224: anulados ainda podem passar de 50 %", circ(S1, true, &[(10, true, 0), (20, true, 0), (30, false, 0), (40, false, 0)]), vec![60, 0, 30, 30], 10, MatCirc::Apurando, vec![N, N, N, N]),
        ("art. 224: anulado ainda pode passar o líder", circ(S1, true, &[(10, true, 0), (20, true, 0), (30, false, 0)]), vec![100, 10, 95], 6, MatCirc::Apurando, vec![N, N, N]),
        ("art. 224: líder seguro", circ(S1, true, &[(10, true, 0), (20, true, 0), (30, false, 0)]), vec![100, 10, 20], 50, MatCirc::Eleito, vec![E, N, N]),
        ("art. 224: mais votado anulado", circ(S1, true, &[(10, false, 0), (20, true, 0)]), vec![100, 10], 0, MatCirc::Apurando, vec![N, N]),
    ];
    for (nome, c, v, r, ec, esperado) in casos {
        let a = avaliar(&c, &v, r);
        assert_eq!((a.circ, a.cand), (ec, esperado), "{nome}");
    }
}

/// Força bruta: em circunscrições pequenas, todo "matematicamente" declarado se confirma em qualquer
/// distribuição dos votos restantes (e o resultado final pela situação nunca o contradiz).
#[test]
fn nunca_declara_o_que_ainda_pode_mudar() {
    use crate::situacao::{situacao_final, Situacao};
    let mut x = 88172645463325252u64;
    let mut rnd = |m: u64| { x ^= x << 13; x ^= x >> 7; x ^= x << 17; x % m };
    for caso in 0..3000 {
        let n = 2 + rnd(3) as usize;
        let sistema = [S1, S2, ABS, NOM][rnd(4) as usize];
        let art224 = rnd(3) == 0;
        let cands: Vec<(u32, bool, u32)> = (0..n).map(|i| ((i as u32 + 1) * 10, rnd(5) != 0, if rnd(4) == 0 { 0 } else { 19500101 + rnd(3) as u32 * 10000 })).collect();
        let c = circ(sistema, art224, &cands);
        let votos: Vec<u64> = (0..n).map(|_| rnd(8)).collect();
        let r = rnd(5);
        let a = avaliar(&c, &votos, r);
        // todas as formas de distribuir até r eleitores (cada um vota em até `e` candidatos distintos ou se abstém)
        let e = if let Sistema::MaioriaSimples { vagas } = sistema { vagas as usize } else { 1 };
        let mut finais = vec![votos.clone()];
        for _ in 0..r {
            let mut prox = vec![];
            for f in &finais {
                prox.push(f.clone());
                for i in 0..n {
                    let mut g = f.clone(); g[i] += 1;
                    if e == 2 { for j in i + 1..n { let mut h = g.clone(); h[j] += 1; prox.push(h); } }
                    prox.push(g);
                }
            }
            prox.sort(); prox.dedup();
            finais = prox;
        }
        for f in &finais {
            let st = situacao_final(&c, f);
            for i in 0..n {
                match a.cand[i] {
                    Mat::Eleito => assert_eq!(st[i], Situacao::Eleito, "caso {caso}: {cands:?} {votos:?} r={r} {sistema:?} art224={art224} final {f:?}"),
                    Mat::SegundoTurno => assert_eq!(st[i], Situacao::SegundoTurno, "caso {caso}: {cands:?} {votos:?} r={r} final {f:?}"),
                    Mat::Nao => {}
                }
            }
            if a.circ == MatCirc::SegundoTurno { assert!(st.contains(&Situacao::SegundoTurno), "caso {caso}: {cands:?} {votos:?} r={r} final {f:?}"); }
        }
    }
}

#[test]
fn faltam_casos() {
    // maioria simples, líder mais idoso: decide com v1 − v2 ≥ r' → r' = 10, faltam = 100 − 10
    assert_eq!(faltam_para_mat(&circ(S1, false, &TRES), &[100, 90, 0], 100), Some(90));
    // líder mais novo: precisa v1 − v2 > r' → r' = 9
    assert_eq!(faltam_para_mat(&circ(S1, false, &[(10, true, 19800101), (20, true, 19600101)]), &[100, 90], 100), Some(91));
    assert_eq!(faltam_para_mat(&circ(S1, false, &TRES), &[100, 89, 0], 10), Some(0), "já decidida");
    assert_eq!(faltam_para_mat(&circ(S1, false, &[(10, true, 0), (20, true, 0)]), &[50, 50], 10), None, "empate sem data");
    assert_eq!(faltam_para_mat(&circ(S1, false, &[(10, true, 0), (20, true, 0)]), &[50, 50], 0), None);
    // maioria absoluta: 2·60 > 90 + r' → r' ≤ 29 (eleito); 2º turno exigiria 2·60 + r' ≤ 90 (nunca)
    assert_eq!(faltam_para_mat(&circ(ABS, false, &TRES), &[60, 20, 10], 1000), Some(971));
    // 2º turno certo quando ninguém alcança a maioria: 2·40 + r' ≤ 100 → r' = 20
    assert_eq!(faltam_para_mat(&circ(ABS, false, &TRES), &[40, 35, 25], 500), Some(480));
    // a resposta é a fronteira: decide em r − faltam e não decide em r − faltam + 1
    let c = circ(S2, false, &[(10, true, 0), (20, true, 0), (30, true, 0), (40, true, 0)]);
    let v = [100, 80, 60, 50];
    let f = faltam_para_mat(&c, &v, 300).unwrap();
    assert_ne!(avaliar(&c, &v, 300 - f).circ, MatCirc::Apurando);
    assert_eq!(avaliar(&c, &v, 300 - f + 1).circ, MatCirc::Apurando);
}

/// Ao longo de apurações aleatórias (cada eleitor apurado vota em alguém, em ninguém ou se abstém), `faltam` nunca
/// sobe e chega a 0 no mesmo passo em que `avaliar` deixa de dar `Apurando`; e é sempre a fronteira exata.
#[test]
fn faltam_nunca_sobe() {
    let mut x = 0x9e3779b97f4a7c15u64;
    let mut rnd = |m: u64| { x ^= x << 13; x ^= x >> 7; x ^= x << 17; x % m };
    for caso in 0..400 {
        let n = 2 + rnd(4) as usize;
        let sistema = [S1, S2, ABS, NOM][rnd(4) as usize];
        // só aptos: com não apto liderando (art. 224) o "matematicamente" pode deixar de ser possível
        let cands: Vec<(u32, bool, u32)> = (0..n).map(|i| ((i as u32 + 1) * 10, true, if rnd(3) == 0 { 0 } else { 19500101 + rnd(30) as u32 * 10000 })).collect();
        let c = circ(sistema, false, &cands);
        let e = if let Sistema::MaioriaSimples { vagas } = sistema { vagas as usize } else { 1 };
        let mut votos = vec![0u64; n];
        let mut r = 200 + rnd(300);
        let pesos: Vec<u64> = (0..n).map(|_| 1 + rnd(6)).collect();
        let tot: u64 = pesos.iter().sum();
        let mut ant: Option<u64> = None;
        while r > 0 {
            r -= 1;
            if rnd(10) >= 2 {
                // vota: candidato sorteado com peso; com 2 votos, um segundo diferente
                let mut s = rnd(tot);
                let i = (0..n).find(|&i| { if s < pesos[i] { true } else { s -= pesos[i]; false } }).unwrap();
                votos[i] += 1;
                if e == 2 && rnd(2) == 0 { let j = (i + 1 + rnd(n as u64 - 1) as usize) % n; votos[j] += 1; }
            }
            let f = faltam_para_mat(&c, &votos, r);
            let decidida = avaliar(&c, &votos, r).circ != MatCirc::Apurando;
            assert_eq!(f == Some(0), decidida, "caso {caso}: {votos:?} r={r} {f:?}");
            if let (Some(a), Some(b)) = (ant, f) { assert!(b <= a, "caso {caso}: faltam subiu {a} → {b} ({votos:?} r={r} {sistema:?})"); }
            if let Some(b) = f.filter(|&b| b > 0) {
                assert_ne!(avaliar(&c, &votos, r - b).circ, MatCirc::Apurando, "caso {caso}");
                assert_eq!(avaliar(&c, &votos, r - b + 1).circ, MatCirc::Apurando, "caso {caso}");
            }
            if f.is_some() { ant = f; }
        }
    }
}
