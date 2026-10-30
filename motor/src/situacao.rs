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
pub(crate) mod testes {
    use super::*;
    use crate::candidatos::{Candidato, Destinacao};
    use crate::oficial::Abrangencia;

    /// Circunscrição de teste: (número, apto, nascimento aaaammdd ou 0 = sem data)
    pub(crate) fn circ(sistema: Sistema, art224: bool, cands: &[(u32, bool, u32)]) -> Circunscricao {
        Circunscricao {
            abr: Abrangencia::Uf(*b"df"), eleitorado: 0, finalizada: true, sistema, art224,
            candidatos: cands.iter().map(|&(n, a, d)| Candidato { numero: n, destinacao: if a { Destinacao::Valido } else { Destinacao::Anulado }, nascimento: (d > 0).then_some(d) }).collect(),
        }
    }
    use Situacao::{Eleito as E, NaoEleito as N, SegundoTurno as T};
    const ABS: Sistema = Sistema::MaioriaAbsoluta { base: Base::Aptos };
    const NOM: Sistema = Sistema::MaioriaAbsoluta { base: Base::Nominal };
    const S1: Sistema = Sistema::MaioriaSimples { vagas: 1 };
    const S2: Sistema = Sistema::MaioriaSimples { vagas: 2 };

    #[test]
    fn tabela() {
        let tres = [(10, true, 19600101), (20, true, 19700101), (30, true, 19800101)];
        let casos: Vec<(&str, Circunscricao, Vec<u64>, Vec<Situacao>)> = vec![
            ("maioria absoluta atingida", circ(ABS, false, &tres), vec![51, 30, 19], vec![E, N, N]),
            ("maioria absoluta: exatamente a metade não basta", circ(ABS, false, &tres), vec![50, 30, 20], vec![T, T, N]),
            ("maioria absoluta não atingida: 2º turno", circ(ABS, false, &tres), vec![40, 25, 35], vec![T, N, T]),
            ("maioria absoluta sobre os aptos: anulado fora da base", circ(ABS, false, &[(10, true, 0), (20, true, 0), (30, false, 0)]), vec![51, 49, 1000], vec![E, N, N]),
            ("maioria absoluta sobre a votação nominal (Prefeito > 200 mil)", circ(NOM, true, &[(10, true, 0), (20, true, 0), (30, false, 0)]), vec![50, 30, 21], vec![T, T, N]),
            ("maioria simples, 1 vaga", circ(S1, false, &tres), vec![30, 35, 35], vec![N, E, N]),
            ("maioria simples, 2 vagas (Senador)", circ(S2, false, &tres), vec![30, 35, 40], vec![N, E, E]),
            ("empate decidido pela idade: o mais idoso", circ(S1, false, &[(10, true, 19800101), (20, true, 19600101)]), vec![100, 100], vec![N, E]),
            ("empate: sem data de nascimento vai por último", circ(S1, false, &[(10, true, 0), (20, true, 19900101)]), vec![100, 100], vec![N, E]),
            ("empate sem datas: o menor número", circ(S1, false, &[(20, true, 0), (10, true, 0)]), vec![7, 7], vec![N, E]),
            ("2 vagas com não apto entre os mais votados", circ(S2, false, &[(10, false, 0), (20, true, 0), (30, true, 0), (40, true, 0)]), vec![99, 50, 40, 30], vec![N, E, E, N]),
            ("art. 224: mais votado anulado → ninguém eleito", circ(S1, true, &[(10, false, 0), (20, true, 0), (30, true, 0)]), vec![40, 35, 25], vec![N, N, N]),
            ("art. 224: anulados > 50 % → ninguém eleito", circ(S1, true, &[(10, true, 0), (20, false, 0), (30, false, 0)]), vec![40, 35, 26], vec![N, N, N]),
            ("art. 224: anulados = 50 % ainda elege", circ(S1, true, &[(10, true, 0), (20, false, 0), (30, false, 0)]), vec![50, 30, 20], vec![E, N, N]),
            ("sem art. 224: mais votado anulado, elege o 1º apto", circ(S1, false, &[(10, false, 0), (20, true, 0)]), vec![40, 35], vec![N, E]),
            ("sem aptos: ninguém eleito", circ(S1, false, &[(10, false, 0)]), vec![5], vec![N]),
            ("sem votos: maioria absoluta vai ao 2º turno", circ(ABS, false, &tres), vec![0, 0, 0], vec![T, T, N]),
        ];
        for (nome, c, v, esperado) in casos { assert_eq!(situacao_final(&c, &v), esperado, "{nome}"); }
    }

    #[test]
    fn totalizacao_nao_finalizada_fica_pendente() {
        let mut c = circ(S1, false, &[(10, true, 0), (20, true, 0)]);
        c.finalizada = false;
        assert_eq!(situacao_final(&c, &[10, 5]), vec![Situacao::Pendente; 2]);
        assert_eq!(Situacao::Pendente.texto(), "");
    }

    #[test]
    fn duzentos_mil_exatos_e_maioria_simples() {
        use crate::candidatos::Sistema as S;
        let (s, a) = S::para(11, 1, 200_000, 1, false);
        let mut c = circ(s, a, &[(10, true, 0), (20, true, 0), (30, true, 0)]);
        c.eleitorado = 200_000;
        assert_eq!(situacao_final(&c, &[40, 35, 25]), vec![E, N, N]);
        let (s, a) = S::para(11, 1, 200_001, 1, false);
        let c = circ(s, a, &[(10, true, 0), (20, true, 0), (30, true, 0)]);
        assert_eq!(situacao_final(&c, &[40, 35, 25]), vec![T, T, N]);
    }
}
