//! Reprodução da noite da eleição, sem esperar o relógio: aplica os BUs na ordem em que chegaram (horário de
//! recebimento no TSE, ou o de outra simulação de aquisição) e, a cada horário distinto (lote), reavalia as
//! circunscrições que receberam BU ([`Acompanhamento`]). BUs sem horário entram no fim, depois do último.
//!
//! É o que permite medir em que instante o Urna Velox declara o "matematicamente eleito" e conferir que a
//! declaração nunca é revogada e sempre coincide com o eleito oficial.

use crate::acompanhamento::Acompanhamento;
use crate::candidatos::Candidatos;
use crate::catalogo::Catalogo;
use crate::estado::{Agregador, Estado};
use crate::metricas::ChaveUf;
use crate::pacote::EmMemoria;
use std::collections::HashMap;
use std::time::Instant;

#[derive(Debug, Default, Clone)]
pub struct Medidas {
    pub bus: usize,
    pub sem_horario: usize,
    pub lotes: usize,
    pub avaliacoes: u64,
    /// tempo total (aplicação + reavaliação), em segundos, numa thread
    pub segundos: f64,
    pub primeiro: i64,
    pub ultimo: i64,
}

impl Medidas {
    pub fn us_por_bu(&self) -> f64 { self.segundos * 1e6 / self.bus.max(1) as f64 }
}

/// Ordem de chegada: (horário, índice do BU em `mem`). BUs sem horário vão para depois do último, na ordem dos
/// pacotes. Devolve também quantos ficaram sem horário.
pub fn ordem_de_chegada(mem: &EmMemoria, chegadas: &HashMap<ChaveUf, i64>) -> (Vec<(i64, usize)>, usize) {
    let mut ev: Vec<(i64, usize)> = vec![];
    let mut sem = vec![];
    for (i, (r, _, _, _)) in mem.itens.iter().enumerate() {
        match chegadas.get(&(r.uf, r.chave())) { Some(&t) => ev.push((t, i)), None => sem.push(i) }
    }
    ev.sort();
    let fim = ev.last().map_or(0, |x| x.0) + 1;
    let n_sem = sem.len();
    ev.extend(sem.into_iter().map(|i| (fim, i)));
    (ev, n_sem)
}

/// Aplica os BUs em ordem de chegada numa thread, reavaliando as circunscrições a cada horário distinto.
pub fn reproduzir<'a>(cat: &Catalogo, cad: &'a Candidatos, mem: &EmMemoria, chegadas: &HashMap<ChaveUf, i64>, cargos: Option<&[u8]>) -> (Estado, Acompanhamento<'a>, Medidas) {
    let (ev, sem_horario) = ordem_de_chegada(mem, chegadas);
    let est = Estado::novo(cat);
    let mut ac = Acompanhamento::novo(cat, cad, cargos);
    let mut md = Medidas { bus: ev.len(), sem_horario, primeiro: ev.first().map_or(0, |x| x.0), ultimo: ev.last().map_or(0, |x| x.0), ..Default::default() };
    let t0 = Instant::now();
    {
        let mut ag = Agregador::novo(cat, &est);
        let mut k = 0;
        while k < ev.len() {
            let t = ev[k].0;
            while k < ev.len() && ev[k].0 == t {
                let (r, _, _, _) = &mem.itens[ev[k].1];
                est.aplicar(cat, r.chave(), mem.bu(ev[k].1), &mut ag);
                if let Some(m) = cat.muni(r.municipio) { ac.bu_aplicado(m as usize); }
                k += 1;
            }
            ac.atualizar(&est, k as u64, t);
            md.lotes += 1;
        }
    }
    md.segundos = t0.elapsed().as_secs_f64();
    md.avaliacoes = ac.avaliacoes;
    (est, ac, md)
}

/// Diferença da hora local da UF para a de Brasília (sem horário de verão desde 2019). Os arquivos oficiais
/// trazem a data e a hora da totalização (`dt`/`ht`) na hora local; `aux.json` e `cs.json`, na de Brasília.
pub fn fuso(uf: &[u8; 2]) -> i64 {
    match uf { b"ac" => -2, b"am" | b"mt" | b"ms" | b"ro" | b"rr" => -1, _ => 0 }
}

/// Antecedência do Urna Velox sobre a totalização do TSE, por circunscrição.
#[derive(Clone, Debug)]
pub struct Antecedencia {
    pub circ: usize,
    /// data e hora da totalização no arquivo oficial (hora local) e convertida para Brasília (segundos)
    pub tse: String,
    pub tse_brasilia: Option<i64>,
    /// totalizado de novo depois do dia da eleição: fica fora das estatísticas
    pub retotalizado: bool,
    /// matematicamente decidida (eleito ou 2º turno) e último BU da circunscrição, já com a latência do Urna Velox
    pub mat: Option<f64>,
    pub fim: Option<f64>,
    /// os candidatos declarados conferem com a situação oficial
    pub confere: bool,
}

/// Estatística no formato da simulação Python (`simular_noite_municipal.py`): n, mín., mediana, máx., média, negativos.
pub fn estatistica(v: &[f64]) -> String {
    if v.is_empty() { return "{}".into(); }
    let mut s = v.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = s.len();
    let med = if n % 2 == 1 { s[n / 2] } else { (s[n / 2 - 1] + s[n / 2]) / 2.0 };
    format!("{{\"n\":{n},\"min\":{:.0},\"p50\":{med:.1},\"max\":{:.0},\"media\":{:.0},\"negativos\":{}}}", s[0], s[n - 1],
        s.iter().sum::<f64>() / n as f64, s.iter().filter(|&&x| x < 0.0).count())
}

/// Compara os instantes do acompanhamento com a hora de totalização dos arquivos oficiais. `dia`: dia da
/// eleição ("dd/mm/aaaa"); `latencia_s`: latência do Urna Velox somada a cada instante (totalização + publicação).
pub fn antecedencias(ac: &Acompanhamento, ofs: &[crate::oficial::Oficial], dia: &str, latencia_s: f64) -> Vec<Antecedencia> {
    use crate::matematica::{Mat, MatCirc};
    use crate::oficial::Abrangencia;
    let mut out = vec![];
    for (i, c) in ac.circs.iter().enumerate() {
        let Some(of) = ofs.iter().find(|o| o.cargo == c.cargo && o.abr.as_ref() == Some(c.abr())) else { continue };
        let uf = match c.abr() { Abrangencia::Uf(u) | Abrangencia::Municipio(u, _) => *u, Abrangencia::Brasil => *b"df" };
        let tse_brasilia = crate::metricas::segundos(&format!("{} {}", of.data, of.hora)).map(|t| t - 3600 * fuso(&uf));
        let mat = (c.mat_circ != MatCirc::Apurando).then(|| c.circ_em.map(|x| x.1 as f64 + latencia_s)).flatten();
        let confere = of.finalizada && c.mat.iter().enumerate().all(|(k, m)| {
            let st = of.cand.iter().find(|x| x.numero == c.circ.candidatos[k].numero).map_or("", |x| x.st.as_str());
            match m { Mat::Eleito => st == "Eleito", Mat::SegundoTurno => st == "2º turno", Mat::Nao => true }
        });
        out.push(Antecedencia { circ: i, tse: format!("{} {}", of.data, of.hora), tse_brasilia, retotalizado: of.data != dia,
            mat, fim: c.ultimo.map(|x| x.1 as f64 + latencia_s), confere });
    }
    out
}
