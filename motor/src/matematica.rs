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
mod testes;
