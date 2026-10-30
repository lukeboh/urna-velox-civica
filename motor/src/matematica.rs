//! **Matematicamente eleito / matematicamente 2º turno**: quando o resultado já não pode mudar, qualquer que seja
//! o voto do eleitorado ainda não apurado.
//!
//! `R` = eleitorado da circunscrição − eleitores aptos das seções já totalizadas (mínimo 0). O pior caso é todo
//! `R` comparecer e votar contra o candidato analisado (no Senador com 2 vagas, cada eleitor dá até 2 votos, a
//! candidatos diferentes). Seções não instaladas não entram no apurado (não têm BU): `R` fica maior que o real,
//! o que só atrasa a declaração — nunca a antecipa.
//!
//! Empate final é decidido pela idade (o mais idoso fica à frente), como na situação final: para um adversário
//! `j` alcançar `i` ele precisa de `custo(j, i) = v_i − v_j` votos se é mais idoso que `i`, e `v_i − v_j + 1` se
//! não é; sem data de nascimento de algum dos dois vale o caso do adversário mais idoso (custo menor).
//!
//! * **Maioria simples, `k` vagas, `e` votos por eleitor:** o `p`-ésimo apto (`p ≤ k`) está garantido se os
//!   `k − p + 1` aptos de menor custo abaixo dele não conseguem todos alcançá-lo: com os custos ordenados
//!   `c_1 ≤ … ≤ c_m`, garantido se `c_m > R` ou `Σ c > e·R`. Para `k = e = 1`: `v_1 − v_2 > R`, ou
//!   `v_1 − v_2 = R` com o líder mais idoso.
//! * **Maioria absoluta (1º turno):** o líder está matematicamente eleito se `2·v_1 > V + R` (`V` = base: votos
//!   dos aptos ou votação nominal). A circunscrição está matematicamente em 2º turno se nenhum apto alcança mais a
//!   maioria: `2·v_c + R ≤ V` para todo `c`; cada candidato garantido entre os 2 primeiros (maioria simples,
//!   `k = 2`) está matematicamente no 2º turno.
//! * **Art. 224 (todo cargo majoritário desde a 1.0.35, B-070; antes só Prefeito):** só se declara "matematicamente" se o mais votado de todos é apto, se nenhum não apto
//!   pode passar o candidato em questão (o 2º colocado, no 2º turno) e se os votos de não aptos não podem mais
//!   passar de 50 % da votação nominal: `2·A + R ≤ N`.

use crate::candidatos::{Base, Circunscricao, Sistema};
use crate::situacao::ordem;

/// Estado "matemático" de um candidato.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Mat {
    Nao,
    Eleito,
    SegundoTurno,
}

/// Estado "matemático" da circunscrição.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MatCirc {
    /// ainda pode mudar
    Apurando,
    /// todas as vagas já têm eleito garantido
    Eleito,
    /// haverá 2º turno (nenhum apto alcança mais a maioria absoluta)
    SegundoTurno,
}

impl Mat {
    pub fn texto(&self) -> &'static str { match self { Mat::Nao => "", Mat::Eleito => "eleito", Mat::SegundoTurno => "2t" } }
}
impl MatCirc {
    pub fn texto(&self) -> &'static str {
        match self { MatCirc::Apurando => "apurando", MatCirc::Eleito => "matematicamente_eleito", MatCirc::SegundoTurno => "matematicamente_2t" }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Avaliacao {
    pub circ: MatCirc,
    /// na ordem de `circ.candidatos`
    pub cand: Vec<Mat>,
}

/// Votos que o adversário `j` precisa para alcançar `i` (0 se já está à frente).
fn custo(c: &Circunscricao, votos: &[u64], j: usize, i: usize) -> u64 {
    if votos[j] > votos[i] { return 0; }
    let base = votos[i] - votos[j];
    // j vence o empate se é mais idoso (mesma data: o menor número, como em `situacao::ordem`); sem data de algum
    // dos dois, supõe-se que vence (caso mais desfavorável a i)
    let (cj, ci) = (&c.candidatos[j], &c.candidatos[i]);
    let j_vence_empate = match (cj.nascimento, ci.nascimento) { (Some(a), Some(b)) => (a, cj.numero) < (b, ci.numero), _ => true };
    if j_vence_empate { base } else { base + 1 }
}

/// O apto na posição `pos` (0 = 1º) fica entre os `k` primeiros em qualquer cenário com `r` eleitores restantes,
/// cada um com até `e` votos (a candidatos diferentes)?
fn garantido(c: &Circunscricao, votos: &[u64], aptos: &[usize], pos: usize, k: usize, r: u64, e: u64) -> bool {
    let i = aptos[pos];
    let precisa = k - pos;
    let mut custos: Vec<u64> = aptos[pos + 1..].iter().map(|&j| custo(c, votos, j, i)).collect();
    if custos.len() < precisa { return true; }
    custos.sort_unstable();
    let (max, soma) = (custos[precisa - 1], custos[..precisa].iter().sum::<u64>());
    max > r || soma > e.saturating_mul(r)
}

/// Avalia a circunscrição com os votos atuais (`votos`, na ordem de `c.candidatos`) e `r` eleitores restantes.
pub fn avaliar(c: &Circunscricao, votos: &[u64], r: u64) -> Avaliacao {
    let n = c.candidatos.len();
    let mut a = Avaliacao { circ: MatCirc::Apurando, cand: vec![Mat::Nao; n] };
    let todos = ordem(c, votos);
    let apto = |i: usize| c.candidatos[i].destinacao.apto();
    let aptos: Vec<usize> = todos.iter().copied().filter(|&i| apto(i)).collect();
    if aptos.is_empty() { return a; }
    let nominal: u64 = votos.iter().sum();
    // art. 224: o alvo (quem se quer declarar) não pode ser passado por nenhum não apto, e os não aptos não
    // podem passar de 50 % da votação nominal
    // (com `e` votos por eleitor, os não aptos ganham no máximo min(e, nº de não aptos)·R votos)
    let e = if let Sistema::MaioriaSimples { vagas } = c.sistema { vagas.max(1) as u64 } else { 1 };
    let guarda = |alvo: usize| -> bool {
        if !c.art224 { return true; }
        let anulados: u64 = (0..n).filter(|&i| !apto(i)).map(|i| votos[i]).sum();
        let n_nao_aptos = (0..n).filter(|&i| !apto(i)).count() as u64;
        apto(todos[0]) && 2 * anulados + e.min(n_nao_aptos) * r <= nominal && (0..n).filter(|&j| !apto(j)).all(|j| custo(c, votos, j, alvo) > r)
    };
    match c.sistema {
        Sistema::MaioriaSimples { vagas } => {
            let k = (vagas as usize).min(aptos.len());
            let mut todas = k > 0;
            for pos in 0..k {
                if garantido(c, votos, &aptos, pos, vagas as usize, r, e) && guarda(aptos[pos]) { a.cand[aptos[pos]] = Mat::Eleito; } else { todas = false; }
            }
            if todas { a.circ = MatCirc::Eleito; }
        }
        Sistema::MaioriaAbsoluta { base } => {
            let v: u64 = match base { Base::Aptos => aptos.iter().map(|&i| votos[i]).sum(), Base::Nominal => nominal };
            let l = aptos[0];
            if 2 * votos[l] > v + r && guarda(l) {
                a.cand[l] = Mat::Eleito;
                a.circ = MatCirc::Eleito;
            } else if aptos.iter().all(|&i| 2 * votos[i] + r <= v) && guarda(aptos[aptos.len().min(2) - 1]) {
                a.circ = MatCirc::SegundoTurno;
                for pos in 0..aptos.len().min(2) {
                    if garantido(c, votos, &aptos, pos, 2, r, 1) { a.cand[aptos[pos]] = Mat::SegundoTurno; }
                }
            }
        }
    }
    a
}

/// **Eleitores que faltam apurar para o "matematicamente"** (etapa 5, marco `faltam_para_mat`): com os votos de agora,
/// quantos dos `r` eleitores restantes ainda precisam ser apurados — votando como no pior caso — até a circunscrição
/// ficar decidida (`avaliar(…).circ` ≠ `Apurando`). É `r − r'`, com `r'` o **maior** restante ≤ `r` com que ela já
/// estaria decidida. `Some(0)` = já decidida; `None` = não se decide nem com `r' = 0` (empate sem data de nascimento,
/// mais votado não apto pelo art. 224…).
///
/// Cada condição de [`avaliar`] só fica mais fácil com `r` menor (`max > r`, `Σ > e·r`, `2·v > V + r`,
/// `2·v + r ≤ V`, `2·A + e·r ≤ N`, `custo > r`), então "decidida" é monótona em `r` e basta uma busca binária
/// (~log₂ r chamadas de `avaliar`). E como um eleitor apurado diminui `r` em 1 e muda os votos no máximo como o pior
/// caso previsto, `faltam` **nunca sobe** enquanto só entram BUs: só sobe se um BU for retirado/substituído ou se um
/// candidato não apto passar a liderar (art. 224). Chega a 0 exatamente no lote em que a circunscrição vira
/// "matematicamente" (é a mesma condição do evento).
pub fn faltam_para_mat(c: &Circunscricao, votos: &[u64], r: u64) -> Option<u64> {
    let decide = |x: u64| avaliar(c, votos, x).circ != MatCirc::Apurando;
    if decide(r) { return Some(0); }
    if r == 0 || !decide(0) { return None; }
    // invariante: decide(lo) e !decide(hi)
    let (mut lo, mut hi) = (0u64, r);
    while hi - lo > 1 {
        let m = lo + (hi - lo) / 2;
        if decide(m) { lo = m } else { hi = m }
    }
    Some(r - lo)
}

#[cfg(test)]
mod testes {
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
}
