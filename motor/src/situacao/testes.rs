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
