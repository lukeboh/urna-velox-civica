//! Situação dos candidatos (eleito, 2º turno, não eleito) a partir dos votos totalizados e do [`Candidatos`].
//!
//! Regras (as mesmas que deram 100 % nos arquivos oficiais de 2024 e 2026, antes em `comparar_oficial.py` e
//! `comparar_municipal.py`):
//! 1. Ordem: votos decrescentes; empate → o mais idoso (sem data de nascimento vai por último); depois o número.
//! 2. Só concorrem os **aptos** (destinação "Válido…"); os demais ficam "Não eleito".
//! 3. Art. 224 do Código Eleitoral (todo cargo majoritário — Presidente, Governador, Senador, Prefeito —, nos dois
//!    turnos; antes da 1.0.35 só Prefeito, B-070): ninguém é eleito se o mais votado de **todos** não é apto (§ 3º:
//!    candidato eleito com o registro indeferido → novas eleições), ou se os votos de não aptos passam de 50 % da
//!    votação nominal (caput). São os dois motivos que o TSE publica em `mnae` com `esae = s`.
//! 4. Maioria absoluta: elege o 1º apto se ele tem mais da metade da base — votos dos aptos (Presidente,
//!    Governador) ou votação nominal inteira (Prefeito acima de 200 mil eleitores); senão os 2 primeiros aptos
//!    vão ao 2º turno. Maioria simples: os `vagas` primeiros aptos são eleitos.
//! 5. Totalização não finalizada → situação pendente (vazia), como na divulgação.
//!
//! [`Candidatos`]: crate::candidatos::Candidatos

use crate::candidatos::{Base, Circunscricao, Sistema};
use crate::resultado::Totais;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Situacao {
    Eleito,
    SegundoTurno,
    NaoEleito,
    /// totalização não finalizada
    Pendente,
}

impl Situacao {
    /// O texto do campo `st` da divulgação.
    pub fn texto(&self) -> &'static str {
        match self { Situacao::Eleito => "Eleito", Situacao::SegundoTurno => "2º turno", Situacao::NaoEleito => "Não eleito", Situacao::Pendente => "" }
    }
}

/// Índices de `circ.candidatos` na ordem da regra 1, com os votos de cada um.
pub fn ordem(circ: &Circunscricao, votos: &[u64]) -> Vec<usize> {
    let mut o: Vec<usize> = (0..circ.candidatos.len()).collect();
    o.sort_by_key(|&i| (std::cmp::Reverse(votos[i]), circ.candidatos[i].nascimento.unwrap_or(u32::MAX), circ.candidatos[i].numero));
    o
}

/// Votos de cada candidato da lista, na ordem de `circ.candidatos`.
pub fn votos_dos_candidatos(circ: &Circunscricao, t: &Totais) -> Vec<u64> { circ.candidatos.iter().map(|c| t.votos(c.numero)).collect() }

/// Situação final de cada candidato (na ordem de `circ.candidatos`) com os votos `votos` (mesma ordem).
pub fn situacao_final(circ: &Circunscricao, votos: &[u64]) -> Vec<Situacao> {
    let n = circ.candidatos.len();
    if !circ.finalizada { return vec![Situacao::Pendente; n]; }
    let mut st = vec![Situacao::NaoEleito; n];
    let todos = ordem(circ, votos);
    let apto = |i: usize| circ.candidatos[i].destinacao.apto();
    let aptos: Vec<usize> = todos.iter().copied().filter(|&i| apto(i)).collect();
    if aptos.is_empty() { return st; }
    let nominal: u64 = votos.iter().sum();
    if circ.art224 {
        let anulados: u64 = (0..n).filter(|&i| !apto(i)).map(|i| votos[i]).sum();
        if !apto(todos[0]) || 2 * anulados > nominal { return st; }
    }
    match circ.sistema {
        Sistema::MaioriaAbsoluta { base } => {
            let den = match base { Base::Aptos => aptos.iter().map(|&i| votos[i]).sum(), Base::Nominal => nominal };
            if 2 * votos[aptos[0]] > den { st[aptos[0]] = Situacao::Eleito; } else { for &i in aptos.iter().take(2) { st[i] = Situacao::SegundoTurno; } }
        }
        Sistema::MaioriaSimples { vagas } => for &i in aptos.iter().take(vagas as usize) { st[i] = Situacao::Eleito; },
    }
    st
}

#[cfg(test)]
pub(crate) mod testes;
