use super::*;
use crate::candidatos::Sistema;
use crate::situacao::testes::circ;
const S1: Sistema = Sistema::MaioriaSimples { vagas: 1 };

#[test]
fn sorteio_deterministico_e_percentis() {
    let (mut a, mut b) = (Rng::novo(7), Rng::novo(7));
    let va: Vec<u64> = (0..5).map(|_| a.u64()).collect();
    assert_eq!(va, (0..5).map(|_| b.u64()).collect::<Vec<_>>());
    let mut r = Rng::novo(1);
    let mut n: Vec<f64> = (0..20000).map(|_| r.normal()).collect();
    let m = n.iter().sum::<f64>() / n.len() as f64;
    assert!(m.abs() < 0.03, "média {m}");
    assert!((percentil(&mut n, 0.9) - 1.2816).abs() < 0.05);
    assert_eq!(percentil(&mut [3.0, 1.0, 2.0], 0.5), 2.0);
}

#[test]
fn serie_velocidade_e_janela() {
    let mut s = Serie::default();
    for i in 0..=40 { s.registrar(1000 + i * 30, (i * 300) as u64, i as u64, 600.0); }
    // 300 aptos a cada 30 s = 10 aptos/s
    assert!((s.ema.unwrap() - 10.0).abs() < 1e-6);
    assert_eq!(s.primeiro, Some(1030));
    let t = s.taxas(1800);
    assert!(!t.is_empty() && t.iter().all(|x| (x - 10.0).abs() < 1e-9));
    // instante repetido atualiza; anterior é ignorado
    s.registrar(1000 + 40 * 30, 12_345, 41, 600.0);
    assert_eq!(s.ultimo().unwrap().aptos, 12_345);
    s.registrar(10, 1, 1, 600.0);
    assert_eq!(s.ultimo().unwrap().aptos, 12_345);
}

#[test]
fn gabarito_ajusta_deslocamento_e_escala() {
    let g = grade();
    assert_eq!(g.len(), 119);
    assert!((g[g.len() - 1] - 1.0).abs() < 1e-12 && g.windows(2).all(|w| w[1] > w[0]));
    // curva da noite: F = t/3600 (1 h para tudo)
    let pts: Vec<(i64, f64)> = (0..=3600).step_by(30).map(|t| (t as i64, t as f64 / 3600.0)).collect();
    let c = tau_da_serie(&pts, 0, &g).unwrap();
    assert!((c[50] - 1800.0).abs() < 1.0 && (c[g.len() - 1] - 3600.0).abs() < 1.0);
    // noite corrente 2× mais lenta e começando 600 s depois: t = 600 + 2·τ(F)
    let obs: Vec<(i64, f64)> = (0..=60).map(|i| { let f = 0.3 + i as f64 * 0.002; ((600.0 + 2.0 * 3600.0 * f) as i64, f) }).collect();
    let a = ajustar(&c, &g, &obs).unwrap();
    assert!((a.b - 2.0).abs() < 0.01 && (a.a - 600.0).abs() < 5.0, "{a:?}");
    // pouca variação: só deslocamento
    let a1 = ajustar(&c, &g, &obs[..2]).unwrap();
    assert_eq!(a1.b, 1.0);
    assert_eq!(mediana_curvas(&[vec![1.0, 5.0], vec![2.0, 3.0], vec![3.0, 4.0]]), vec![2.0, 4.0]);
}

/// Um estrato, 60 % × 40 %, metade apurada, 1 voto por apto: decide quando 200 + 200f > 1000 − 1000f (f > 2/3).
#[test]
fn definicao_bate_com_a_conta() {
    let c = circ(S1, false, &[(10, true, 19600101), (20, true, 19700101)]);
    let c = Circunscricao { eleitorado: 2000, ..c };
    let est = [EstratoVivo { eleitorado: 2000.0, apurado: 1000, votos: vec![600, 400] }];
    let e = EntradaMat { circ: &c, votos: &[600, 400], apurado: 1000, estratos: &est };
    let x = vec![vec![600.0, 400.0]];
    let (ap, tipo, antes) = definicao(&e, &x, &[1000.0], &[1.0], &mut vec![], &mut vec![]).unwrap();
    assert_eq!(tipo, MatCirc::Eleito);
    assert!(antes);
    assert!((ap - (1000.0 + 2000.0 / 3.0)).abs() < 2.0, "{ap}");
    // Monte Carlo com horário = fração × 1000 s: a mediana fica perto de 833 s
    let par = Parametros { sorteios: 300, ..Parametros::default() };
    let mut rng = Rng::novo(3);
    let pm = prever_mat(&e, &par, &mut rng, &mut |f, _| f * 1000.0).unwrap();
    assert_eq!(pm.tipo, "matematicamente_eleito");
    assert!((pm.faixa.p50 - 833).abs() <= 15, "{pm:?}");
    assert!(pm.faixa.p10 <= pm.faixa.p50 && pm.faixa.p50 <= pm.faixa.p90 && pm.prob_antes_fim > 0.95);
    // mesma semente, mesmo resultado
    let mut rng = Rng::novo(3);
    assert_eq!(prever_mat(&e, &par, &mut rng, &mut |f, _| f * 1000.0).unwrap(), pm);
    // empate perfeito com datas iguais e sem restante: não decide, nem prevê
    let c2 = circ(S1, false, &[(10, true, 0), (20, true, 0)]);
    let c2 = Circunscricao { eleitorado: 2000, ..c2 };
    let e2 = EntradaMat { circ: &c2, votos: &[500, 500], apurado: 1000, estratos: &[EstratoVivo { eleitorado: 2000.0, apurado: 1000, votos: vec![500, 500] }] };
    let pm2 = prever_mat(&e2, &Parametros { sorteios: 50, deriva: 0.0, ..Parametros::default() }, &mut Rng::novo(1), &mut |f, _| f * 1000.0).unwrap();
    assert!(pm2.prob_antes_fim < 0.5, "{pm2:?}");
}

/// Estratos com tendências opostas: a previsão estratificada usa o restante de cada um (o líder do apurado perde).
#[test]
fn estratos_mudam_a_projecao() {
    let c = Circunscricao { eleitorado: 3000, ..circ(S1, false, &[(10, true, 19600101), (20, true, 19700101)]) };
    // estrato A quase todo apurado (70 × 30); estrato B pouco apurado (20 × 80) e com muito restante
    let est = [
        EstratoVivo { eleitorado: 1000.0, apurado: 1000, votos: vec![700, 300] },
        EstratoVivo { eleitorado: 2000.0, apurado: 100_u64.max(1), votos: vec![20, 80] },
    ];
    let e = EntradaMat { circ: &c, votos: &[720, 380], apurado: 1100, estratos: &est };
    let par = Parametros { k_votos: 10.0, k_aptos: 10.0, sorteios: 100, ..Parametros::default() };
    let pm = prever_mat(&e, &par, &mut Rng::novo(5), &mut |f, _| f * 1000.0).unwrap();
    // no fim: 700 + 20 + 0,2·1900 ≈ 1100 contra 300 + 80 + 0,8·1900 ≈ 1900: o candidato 20 é eleito, e só bem
    // adiante (com proporções globais o 10 pareceria eleito cedo)
    assert_eq!(pm.tipo, "matematicamente_eleito");
    assert!(pm.faixa.p50 > 600, "{pm:?}");
}

#[test]
fn eleitorado_dos_estratos_fecha_com_a_circunscricao() {
    let defs = vec![
        EstratoDef { regioes: vec![], munis: vec![], eleitorado: Some(1000.0), secoes: 4 },
        EstratoDef { regioes: vec![], munis: vec![], eleitorado: None, secoes: 10 },
        EstratoDef { regioes: vec![], munis: vec![], eleitorado: None, secoes: 30 },
    ];
    let mut v = vec![
        EstratoVivo { eleitorado: 0.0, apurado: 500, votos: vec![] },
        EstratoVivo { eleitorado: 0.0, apurado: 1000, votos: vec![] },
        EstratoVivo { eleitorado: 0.0, apurado: 0, votos: vec![] },
    ];
    estimar_eleitorado(&mut v, &defs, &[2, 4, 0], 10_000.0);
    let soma: f64 = v.iter().map(|s| s.eleitorado).sum();
    assert!((soma - 10_000.0).abs() < 1e-6, "{soma}");
    assert_eq!(v[0].eleitorado, 1000.0);
    assert!(v.iter().all(|s| s.eleitorado >= s.apurado as f64));
    assert!(v[2].eleitorado > v[1].eleitorado, "mais seções, mais eleitorado: {v:?}");
}

/// 100 BUs/min por 40 min, parada de 30 min, volta: detecta 5 min depois de parar e libera quando o ritmo volta.
#[test]
fn parada_da_chegada() {
    let par = Parametros::default();
    let mut s = Serie::default();
    let (mut bus, mut estados) = (0u64, vec![]);
    let mut p: Option<Parada> = None;
    for k in 0..=200 {
        let t = 1000 + k * 30;
        let m = (t - 1000) / 60;
        if !(40..70).contains(&m) { bus += 50; }
        s.registrar(t, bus, bus, 600.0);
        p = detectar_parada(&s, t, 0.5, p, &par);
        estados.push((m, p.is_some()));
    }
    let ini = estados.iter().find(|x| x.1).map(|x| x.0).unwrap();
    let fim = estados.iter().rev().find(|x| x.1).map(|x| x.0).unwrap();
    assert!((44..=46).contains(&ini), "detectou em {ini}");
    assert!((70..=72).contains(&fim), "liberou em {fim}");
    // começo da noite e cauda: nunca
    assert!(detectar_parada(&s, 1000 + 55 * 60, 0.99, None, &par).is_none());
}

#[test]
fn juntar_pequenos_preserva_totais() {
    let v: Vec<EstratoVivo> = (0..50).map(|i| EstratoVivo { eleitorado: 100.0 + i as f64, apurado: 50, votos: vec![i, 2 * i] }).collect();
    let j = juntar_pequenos(v, 10);
    assert_eq!(j.len(), 10);
    assert!((j.iter().map(|s| s.eleitorado).sum::<f64>() - (0..50).map(|i| 100.0 + i as f64).sum::<f64>()).abs() < 1e-9);
    assert_eq!(j.iter().map(|s| s.apurado).sum::<u64>(), 50 * 50);
    assert_eq!(j.iter().map(|s| s.votos[1]).sum::<u64>(), (0..50).map(|i| 2 * i).sum::<u64>());
    // os maiores restantes ficam separados
    assert_eq!(j[0].eleitorado, 149.0);
}

#[test]
fn gabaritos_embutidos_leem() {
    let g = Gabaritos::embutidos();
    assert!(g.curvas.iter().all(|c| c.aptos.len() == g.grade.len() && c.secoes.len() == g.grade.len()));
}
