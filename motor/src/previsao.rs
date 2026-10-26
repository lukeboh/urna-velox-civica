//! **Previsão do matematicamente e do fim da apuração** (etapa 8, `docs/HANDOFF_0.8.0.md`): a que horas uma
//! circunscrição deve ficar matematicamente decidida (eleito ou 2º turno) e a que horas a apuração de uma abrangência
//! chega a 99 % e a 100 % das seções — **sempre uma estimativa com intervalo** (P10–P90), recalculada ao longo da noite.
//!
//! Módulo puro: sem rede e sem relógio próprio (o instante entra como parâmetro) e determinístico com a semente.
//!
//! * **Avanço** medido em eleitores aptos das seções apuradas ([`Serie`]); velocidade = média móvel exponencial de
//!   aptos/s (meia-vida [`Parametros::meia_vida_s`]) e a dispersão das taxas por minuto na janela (P10/P90).
//! * **Votos que faltam, por estrato** (a subdivisão logo abaixo da circunscrição: UF no Brasil, município na UF; no
//!   município, ele todo): restante = aptos não apurados × votos nominais por apto × proporções do estrato, com
//!   **encolhimento** para as da circunscrição quando o estrato tem poucos votos (`(v_e + k·p_c)/(n_e + k)`).
//! * **Momento da definição:** avança-se a apuração (cada estrato entregando o seu restante, numa ordem perturbada
//!   entre estratos) e aplica-se a mesma regra do motor ([`avaliar`]) com o eleitorado ainda não apurado; o primeiro
//!   ponto decidido é o momento da definição (busca binária na fração do restante).
//! * **Intervalo:** Monte Carlo com semente fixa: proporções sorteadas em torno da estimativa (erro de amostragem e uma
//!   deriva comum), comparecimento, ordem de chegada entre estratos e a conversão em horário (gabarito ou velocidade).
//! * **Fim da apuração:** curvas fração apurada × tempo das noites anteriores ([`Gabaritos`]) ajustadas à curva corrente
//!   (deslocamento e escala, nos últimos 30 min) e projetadas até 99 % e 100 % das seções.
//!
//! O [`Previsor`] guarda as séries de cada abrangência (tópico) e roda a previsão sobre o [`Estado`] e o
//! [`Acompanhamento`] do motor; quem chama decide quando (o `uv-servico`, a cada 30 s fora da thread do motor; o
//! `uv-motor previsao`, no relógio da noite reproduzida).

use crate::acompanhamento::Acompanhamento;
use crate::candidatos::{Candidatos, Circunscricao};
use crate::catalogo::{Catalogo, AUSENTE};
use crate::estado::Estado;
use crate::matematica::{avaliar, MatCirc};
use crate::oficial::Abrangencia;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::Ordering::Relaxed;

// ------------------------------------------------------------------------------------------------ parâmetros

/// Como converter "fração apurada em que se define" em horário.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tempo {
    /// ritmo constante: a velocidade atual (média móvel), sorteada dentro da dispersão da janela
    Velocidade,
    /// a curva de avanço dos gabaritos ajustada à noite corrente (cai para `Velocidade` sem gabarito)
    Curva,
    /// metade dos sorteios por cada um (conjunto: mais robusto que qualquer um dos dois sozinho)
    Mista,
}

#[derive(Clone, Debug)]
pub struct Parametros {
    /// meia-vida da média móvel da velocidade (s)
    pub meia_vida_s: f64,
    /// janela da dispersão da velocidade e do ajuste aos gabaritos (s)
    pub janela_s: i64,
    /// só prever com pelo menos esta fração dos aptos apurada
    pub min_apurado: f64,
    /// e com pelo menos esta duração de série desde o primeiro BU (s)
    pub min_serie_s: i64,
    /// encolhimento das proporções do estrato para as da circunscrição (em votos nominais)
    pub k_votos: f64,
    /// encolhimento dos votos nominais por apto do estrato para o da circunscrição (em aptos)
    pub k_aptos: f64,
    /// sorteios do Monte Carlo por rodada
    pub sorteios: usize,
    pub semente: u64,
    /// desvio da deriva comum das proporções (fração dos votos nominais; calibrado nas noites anteriores)
    pub deriva: f64,
    /// desvio (em log) do expoente de chegada de cada estrato: ordem de chegada perturbada
    pub ordem: f64,
    /// erro relativo do tempo que falta (log-normal), somado ao do ajuste
    pub erro_rel: f64,
    /// aumento do erro relativo com poucas seções por chegar: `erro_rel · (1 + erro_poucas/√(restantes + 1))`
    pub erro_poucas: f64,
    /// faixa P10–P90 maior que isto aparece como "ainda incerto" (s)
    pub max_faixa_s: i64,
    pub tempo: Tempo,
    /// parada da chegada dos BUs: ritmo dos últimos `parada_janela_s` abaixo de `parada_razao` × o ritmo dos
    /// `parada_ref_s` anteriores (no país inteiro) suspende a previsão; volta com `parada_retorno` × aquele ritmo
    pub parada_janela_s: i64,
    pub parada_ref_s: i64,
    pub parada_razao: f64,
    pub parada_retorno: f64,
}

impl Default for Parametros {
    fn default() -> Self {
        Parametros { meia_vida_s: 600.0, janela_s: 1800, min_apurado: 0.05, min_serie_s: 600, k_votos: 2000.0, k_aptos: 5000.0,
            sorteios: 200, semente: 0x5eed_0008, deriva: 0.01, ordem: 0.35, erro_rel: 0.4, erro_poucas: 6.0, max_faixa_s: 3 * 3600, tempo: Tempo::Mista,
            parada_janela_s: 300, parada_ref_s: 1800, parada_razao: 0.1, parada_retorno: 0.3 }
    }
}

// ------------------------------------------------------------------------------------------------ sorteios

/// Gerador SplitMix64 (determinístico, sem dependência externa).
#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    pub fn novo(semente: u64) -> Rng { Rng(semente ^ 0x9E37_79B9_7F4A_7C15) }
    pub fn u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// uniforme em [0, 1)
    pub fn unif(&mut self) -> f64 { (self.u64() >> 11) as f64 / (1u64 << 53) as f64 }
    /// normal padrão (Box–Muller)
    pub fn normal(&mut self) -> f64 {
        let u1 = self.unif().max(1e-300);
        (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * self.unif()).cos()
    }
    pub fn indice(&mut self, n: usize) -> usize { (self.unif() * n as f64) as usize % n.max(1) }
}

/// Semente de uma rodada: a do parâmetro misturada à unidade e ao instante (mesma entrada → mesma saída).
fn semente(base: u64, unidade: usize, agora: i64) -> u64 {
    let mut r = Rng::novo(base ^ (unidade as u64).wrapping_mul(0xD6E8_FEB8_6659_FD93) ^ (agora as u64).rotate_left(29));
    r.u64()
}

/// Percentil `p` (0..1) por interpolação linear; ordena `v`.
pub fn percentil(v: &mut [f64], p: f64) -> f64 {
    if v.is_empty() { return f64::NAN; }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let x = p.clamp(0.0, 1.0) * (v.len() - 1) as f64;
    let (i, f) = (x.floor() as usize, x - x.floor());
    if i + 1 < v.len() { v[i] + f * (v[i + 1] - v[i]) } else { v[i] }
}

// ------------------------------------------------------------------------------------------------ série de avanço

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ponto {
    pub t: i64,
    /// eleitores aptos das seções apuradas
    pub aptos: u64,
    /// seções apuradas (com BU)
    pub secoes: u64,
}

/// Série (instante, aptos apurados, seções apuradas) de uma abrangência, no relógio de quem chama (no replay, o
/// simulado). Guarda as últimas 3 h e a média móvel da velocidade.
#[derive(Clone, Debug, Default)]
pub struct Serie {
    pub pontos: Vec<Ponto>,
    /// primeiro instante com algo apurado
    pub primeiro: Option<i64>,
    /// velocidade (aptos/s), média móvel exponencial
    pub ema: Option<f64>,
}

const GUARDA_S: i64 = 3 * 3600;

impl Serie {
    pub fn registrar(&mut self, t: i64, aptos: u64, secoes: u64, meia_vida_s: f64) {
        if let Some(u) = self.pontos.last_mut() {
            if t < u.t { return; }
            if t == u.t { u.aptos = aptos; u.secoes = secoes; return; }
            if aptos > 0 || self.primeiro.is_some() {
                let dt = (t - u.t) as f64;
                let taxa = (aptos as f64 - u.aptos as f64).max(0.0) / dt;
                let a = 1.0 - 0.5f64.powf(dt / meia_vida_s.max(1.0));
                self.ema = Some(match self.ema { Some(e) => e + a * (taxa - e), None => taxa });
            }
        }
        if aptos > 0 && self.primeiro.is_none() { self.primeiro = Some(t); }
        self.pontos.push(Ponto { t, aptos, secoes });
        let corte = t - GUARDA_S;
        if self.pontos.len() > 2 && self.pontos[0].t < corte {
            let i = self.pontos.partition_point(|p| p.t < corte).min(self.pontos.len() - 2);
            self.pontos.drain(..i);
        }
    }

    pub fn ultimo(&self) -> Option<Ponto> { self.pontos.last().copied() }

    /// Taxas (aptos/s) em intervalos de pelo menos 1 min dentro da janela que termina no último ponto.
    pub fn taxas(&self, janela_s: i64) -> Vec<f64> {
        let Some(u) = self.pontos.last() else { return vec![] };
        let ini = u.t - janela_s;
        let mut out = vec![];
        let mut ant: Option<Ponto> = None;
        for p in self.pontos.iter().filter(|p| p.t >= ini) {
            match ant {
                None => ant = Some(*p),
                Some(a) if p.t - a.t >= 60 => {
                    out.push((p.aptos as f64 - a.aptos as f64).max(0.0) / (p.t - a.t) as f64);
                    ant = Some(*p);
                }
                _ => {}
            }
        }
        out
    }

    /// Pontos da janela como (t, fração) com o denominador dado (aptos ou seções).
    fn janela(&self, janela_s: i64, aptos: bool, total: f64) -> Vec<(i64, f64)> {
        let Some(u) = self.pontos.last() else { return vec![] };
        let ini = u.t - janela_s;
        self.pontos.iter().filter(|p| p.t >= ini)
            .map(|p| (p.t, (if aptos { p.aptos } else { p.secoes }) as f64 / total.max(1.0)))
            .collect()
    }
}

// ------------------------------------------------------------------------------------------------ gabaritos

/// Pontos da fração apurada em que as curvas são guardadas: de 1 em 1 % até 90 %, de 0,5 em 0,5 % até 99 % e de
/// 0,1 em 0,1 % até 100 % (a cauda é onde o fim da apuração se decide).
pub fn grade() -> Vec<f64> {
    let mut g: Vec<f64> = (0..=90).map(|i| i as f64 / 100.0).collect();
    g.extend((1..=18).map(|i| 0.90 + i as f64 * 0.005));
    g.extend((1..=10).map(|i| 0.99 + i as f64 * 0.001));
    g
}

/// Curva de avanço de uma classe de abrangência numa noite: `τ(F)` = segundos desde o primeiro BU da noite até a
/// fração `F` (da [`grade`]) dos aptos (`aptos`) e das seções (`secoes`), normalizadas pelo que chegou até o fim.
/// É a mediana, ponto a ponto, das curvas das abrangências da classe.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Curva {
    pub noite: String,
    pub classe: String,
    pub unidades: u32,
    pub aptos: Vec<f64>,
    pub secoes: Vec<f64>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Gabaritos {
    pub grade: Vec<f64>,
    pub curvas: Vec<Curva>,
}

/// Gabaritos embutidos (as três noites que temos: 2024 1º e 2º turnos e 2026 1º turno; `uv-motor previsao
/// --gerar-gabaritos`).
pub const GABARITOS_EMBUTIDOS: &str = include_str!("gabaritos.json");

impl Gabaritos {
    pub fn embutidos() -> Gabaritos { serde_json::from_str(GABARITOS_EMBUTIDOS).unwrap_or_default() }
    pub fn ler(p: &std::path::Path) -> anyhow::Result<Gabaritos> { Ok(serde_json::from_str(&std::fs::read_to_string(p)?)?) }
    /// Junta vários arquivos (a grade tem de ser a mesma).
    pub fn juntar(lista: Vec<Gabaritos>) -> Gabaritos {
        let mut g = Gabaritos { grade: grade(), curvas: vec![] };
        for x in lista { if x.grade.len() == g.grade.len() { g.curvas.extend(x.curvas); } }
        g
    }
}

/// Classe da abrangência para escolher os gabaritos: Brasil, UF ou município por porte do eleitorado.
pub fn classe(abr: &Abrangencia, eleitorado: u64) -> &'static str {
    match abr {
        Abrangencia::Brasil => "br",
        Abrangencia::Uf(_) => "uf",
        Abrangencia::Municipio(..) if eleitorado >= 1_000_000 => "mun-g",
        Abrangencia::Municipio(..) if eleitorado >= 200_000 => "mun-m",
        Abrangencia::Municipio(..) if eleitorado >= 20_000 => "mun-p",
        Abrangencia::Municipio(..) => "mun-pp",
    }
}

/// Classes vizinhas, em ordem de preferência, quando a classe não tem gabarito.
fn vizinhas(c: &str) -> &'static [&'static str] {
    match c {
        "br" => &["br", "uf", "mun-g"],
        "uf" => &["uf", "br", "mun-g"],
        "mun-g" => &["mun-g", "mun-m", "uf"],
        "mun-m" => &["mun-m", "mun-g", "mun-p"],
        "mun-p" => &["mun-p", "mun-m", "mun-pp"],
        _ => &["mun-pp", "mun-p", "mun-m"],
    }
}

/// `τ(F)` de uma série crescente (instante, fração já normalizada): o instante (interpolado) em que a fração alcança
/// cada ponto da grade, em segundos desde `inicio`.
pub fn tau_da_serie(pts: &[(i64, f64)], inicio: i64, grade: &[f64]) -> Option<Vec<f64>> {
    let ultimo = pts.last()?;
    let mut out = Vec::with_capacity(grade.len());
    let mut j = 0;
    for &g in grade {
        while j < pts.len() && pts[j].1 < g - 1e-12 { j += 1; }
        let t = if j == pts.len() {
            ultimo.0 as f64
        } else if j == 0 || pts[j].1 <= pts[j - 1].1 {
            pts[j].0 as f64
        } else {
            let (a, b) = (pts[j - 1], pts[j]);
            a.0 as f64 + (g - a.1) / (b.1 - a.1) * (b.0 - a.0) as f64
        };
        out.push(t - inicio as f64);
    }
    // monótona (arredondamentos)
    for i in 1..out.len() { if out[i] < out[i - 1] { out[i] = out[i - 1]; } }
    Some(out)
}

/// Mediana ponto a ponto de várias curvas `τ` (mesma grade).
pub fn mediana_curvas(cs: &[Vec<f64>]) -> Vec<f64> {
    if cs.is_empty() { return vec![]; }
    let n = cs[0].len();
    let mut out: Vec<f64> = (0..n).map(|i| { let mut v: Vec<f64> = cs.iter().map(|c| c[i]).collect(); percentil(&mut v, 0.5) }).collect();
    for i in 1..out.len() { if out[i] < out[i - 1] { out[i] = out[i - 1]; } }
    out
}

/// `τ` de uma curva numa fração qualquer (interpolação linear na grade).
fn tau(curva: &[f64], grade: &[f64], f: f64) -> f64 {
    let f = f.clamp(0.0, 1.0);
    let i = grade.partition_point(|&g| g < f);
    if i == 0 { return curva[0]; }
    if i >= grade.len() { return curva[curva.len() - 1]; }
    let (g0, g1) = (grade[i - 1], grade[i]);
    curva[i - 1] + (f - g0) / (g1 - g0) * (curva[i] - curva[i - 1])
}

/// Ajuste de um gabarito à curva corrente: `t ≈ a + b·τ(F)` nos pontos da janela.
#[derive(Clone, Copy, Debug)]
pub struct Ajuste {
    pub a: f64,
    pub b: f64,
    /// desvio dos resíduos (s)
    pub sigma: f64,
}

/// Ajusta deslocamento e escala (mínimos quadrados de `t` em `τ(F)`). Com pouca variação de `τ` na janela, só o
/// deslocamento (escala 1). Escala limitada a [0,25; 4].
pub fn ajustar(curva: &[f64], grade: &[f64], obs: &[(i64, f64)]) -> Option<Ajuste> {
    if obs.is_empty() || curva.len() != grade.len() { return None; }
    let xs: Vec<f64> = obs.iter().map(|o| tau(curva, grade, o.1)).collect();
    let ys: Vec<f64> = obs.iter().map(|o| o.0 as f64).collect();
    let n = xs.len() as f64;
    let (mx, my) = (xs.iter().sum::<f64>() / n, ys.iter().sum::<f64>() / n);
    let sxx: f64 = xs.iter().map(|x| (x - mx).powi(2)).sum();
    let sxy: f64 = xs.iter().zip(&ys).map(|(x, y)| (x - mx) * (y - my)).sum();
    let faixa = xs.iter().cloned().fold(f64::MIN, f64::max) - xs.iter().cloned().fold(f64::MAX, f64::min);
    let b = if xs.len() >= 3 && faixa >= 120.0 && sxx > 0.0 { (sxy / sxx).clamp(0.25, 4.0) } else { 1.0 };
    let a = my - b * mx;
    let ss: f64 = xs.iter().zip(&ys).map(|(x, y)| (y - a - b * x).powi(2)).sum();
    Some(Ajuste { a, b, sigma: (ss / (n - 2.0).max(1.0)).sqrt() })
}

// ------------------------------------------------------------------------------------------------ resultado

/// Faixa de horário (segundos no relógio de quem chama): P10, P50, P90.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Faixa {
    pub p10: i64,
    pub p50: i64,
    pub p90: i64,
    /// P90 − P10 maior que [`Parametros::max_faixa_s`]: mostrar como "ainda incerto"
    pub incerta: bool,
    /// B-052: horário pelo ritmo atual (o que falta ÷ a velocidade média móvel), como o ETA de um navegador de trânsito:
    /// sempre que há ritmo, mesmo com a faixa larga demais; `None` sem ritmo (nada entrando)
    pub eta: Option<i64>,
}

impl Faixa {
    fn de(v: &mut [f64], max_faixa_s: i64) -> Option<Faixa> {
        if v.is_empty() { return None; }
        let (p10, p50, p90) = (percentil(v, 0.1).round() as i64, percentil(v, 0.5).round() as i64, percentil(v, 0.9).round() as i64);
        Some(Faixa { p10, p50, p90, incerta: p90 - p10 > max_faixa_s, eta: None })
    }
    pub fn contem(&self, t: i64) -> bool { self.p10 <= t && t <= self.p90 }
}

/// Previsão do "matematicamente".
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct PrevMat {
    /// o que deve sair ("matematicamente_eleito" ou "matematicamente_2t"): o mais frequente nos sorteios
    pub tipo: &'static str,
    pub faixa: Faixa,
    /// probabilidade de se definir antes do fim da apuração — com pelo menos 1 % dos aptos ainda por apurar (perto de
    /// 0 = "deve ir até o fim")
    pub prob_antes_fim: f64,
    /// fração dos aptos apurada em que deve sair (mediana dos sorteios)
    pub apurado_p50: f64,
}

/// Uma previsão para uma abrangência (tópico), num instante.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Previsao {
    pub instante: i64,
    /// fração dos aptos e das seções já apurados
    pub apurado: f64,
    pub secoes: f64,
    /// velocidade (aptos por minuto, média móvel)
    pub aptos_min: f64,
    /// "insuficiente" (pouco apurado ou série curta), "prevista", "definida" (já saiu o matematicamente; `mat` vazio) ou
    /// "suspensa" (a chegada dos BUs parou: as faixas são as da última previsão normal, adiadas pelo tempo parado)
    pub situacao: &'static str,
    pub mat: Option<PrevMat>,
    pub fim99: Option<Faixa>,
    pub fim100: Option<Faixa>,
    /// desde quando a chegada dos BUs está parada (só em "suspensa")
    pub parada: Option<i64>,
}

/// Parada da chegada dos BUs no país: desde quando e o ritmo de antes (BUs/s), que define a volta.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Parada {
    pub desde: i64,
    pub ritmo_antes: f64,
}

/// Ritmo (unidades/s) da série entre os instantes `de` e `ate` (pontos mais próximos antes de cada um).
fn ritmo(s: &Serie, de: i64, ate: i64) -> Option<f64> {
    let p = &s.pontos;
    let a = p.partition_point(|x| x.t <= de).checked_sub(1)?;
    let b = p.partition_point(|x| x.t <= ate).checked_sub(1)?;
    (p[b].t > p[a].t).then(|| (p[b].aptos as f64 - p[a].aptos as f64).max(0.0) / (p[b].t - p[a].t) as f64)
}

/// Detecta a parada (e a volta) na série global de BUs aplicados (`aptos` = BUs). `fracao` = BUs ÷ seções esperadas:
/// fora de 5–97 % não há parada (começo da noite e cauda lenta são normais).
pub fn detectar_parada(s: &Serie, agora: i64, fracao: f64, atual: Option<Parada>, par: &Parametros) -> Option<Parada> {
    let j = par.parada_janela_s;
    let r_agora = ritmo(s, agora - j, agora)?;
    if !(0.05..0.97).contains(&fracao) { return None; }
    match atual {
        Some(p) => (r_agora < par.parada_retorno * p.ritmo_antes).then_some(p),
        None => {
            let r_antes = ritmo(s, agora - j - par.parada_ref_s, agora - j)?;
            (r_antes > 0.0 && r_agora < par.parada_razao * r_antes).then_some(Parada { desde: agora - j, ritmo_antes: r_antes })
        }
    }
}

// ------------------------------------------------------------------------------------------------ projeção

/// Um estrato no instante: eleitorado (conhecido ou estimado), aptos apurados e votos por candidato da circunscrição.
#[derive(Clone, Debug, Default)]
pub struct EstratoVivo {
    pub eleitorado: f64,
    pub apurado: u64,
    pub votos: Vec<u64>,
}

/// Entrada da previsão do "matematicamente" de uma circunscrição.
pub struct EntradaMat<'a> {
    pub circ: &'a Circunscricao,
    /// votos e aptos apurados da circunscrição (= soma dos estratos)
    pub votos: &'a [u64],
    pub apurado: u64,
    pub estratos: &'a [EstratoVivo],
}

/// Parâmetros já derivados de cada estrato (proporções encolhidas e votos nominais por apto).
struct Base {
    resto: Vec<f64>,
    razao: Vec<f64>,
    /// por estrato, proporções e desvio de amostragem de cada candidato
    p: Vec<Vec<f64>>,
    dp: Vec<Vec<f64>>,
}

fn base(e: &EntradaMat, par: &Parametros) -> Base {
    let n = e.votos.len();
    let nominal: f64 = e.votos.iter().sum::<u64>() as f64;
    let rho_c = if e.apurado > 0 { nominal / e.apurado as f64 } else { 0.0 };
    let pc: Vec<f64> = e.votos.iter().map(|&v| if nominal > 0.0 { v as f64 / nominal } else { 1.0 / n.max(1) as f64 }).collect();
    let mut b = Base { resto: vec![], razao: vec![], p: vec![], dp: vec![] };
    for s in e.estratos {
        let ne: f64 = s.votos.iter().sum::<u64>() as f64;
        b.resto.push((s.eleitorado - s.apurado as f64).max(0.0));
        b.razao.push((ne + par.k_aptos * rho_c) / (s.apurado as f64 + par.k_aptos));
        let p: Vec<f64> = (0..n).map(|i| (s.votos.get(i).copied().unwrap_or(0) as f64 + par.k_votos * pc[i]) / (ne + par.k_votos)).collect();
        b.dp.push(p.iter().map(|&x| (x * (1.0 - x) / (ne + par.k_votos)).sqrt()).collect());
        b.p.push(p);
    }
    b
}

/// Primeiro ponto (fração `f` do restante, cada estrato entregando `f^γ` do seu) em que a circunscrição fica decidida:
/// (aptos apurados nesse ponto, estado, antes do fim). `None`: não se decide nem com tudo apurado (empate sem data…).
fn definicao(e: &EntradaMat, x: &[Vec<f64>], resto: &[f64], gama: &[f64], acc: &mut Vec<f64>, votos: &mut Vec<u64>) -> Option<(f64, MatCirc, bool)> {
    let elei = e.circ.eleitorado as f64;
    let em = |f: f64, acc: &mut Vec<f64>, votos: &mut Vec<u64>| -> (MatCirc, f64) {
        acc.clear();
        acc.extend(e.votos.iter().map(|&v| v as f64));
        let mut ap = e.apurado as f64;
        for (s, xs) in x.iter().enumerate() {
            let w = if f >= 1.0 { 1.0 } else if f <= 0.0 { 0.0 } else { f.powf(gama[s]) };
            if w == 0.0 { continue; }
            ap += w * resto[s];
            for (i, &v) in xs.iter().enumerate() { acc[i] += w * v; }
        }
        votos.clear();
        votos.extend(acc.iter().map(|v| v.round().max(0.0) as u64));
        let r = (elei - ap).max(0.0).round() as u64;
        (avaliar(e.circ, votos, r).circ, ap)
    };
    let (c0, a0) = em(0.0, acc, votos);
    if c0 != MatCirc::Apurando { return Some((a0, c0, true)); }
    let (c1, a1) = em(1.0, acc, votos);
    if c1 == MatCirc::Apurando { return None; }
    let (mut lo, mut hi, mut chi, mut ahi) = (0.0f64, 1.0f64, c1, a1);
    for _ in 0..14 {
        let m = 0.5 * (lo + hi);
        let (c, a) = em(m, acc, votos);
        if c != MatCirc::Apurando { hi = m; chi = c; ahi = a; } else { lo = m; }
    }
    // "antes do fim": com pelo menos 1 % dos aptos ainda por apurar
    Some((ahi, chi, ahi < 0.99 * a1))
}

/// Previsão do "matematicamente" por Monte Carlo. `horario(fração dos aptos, rng)` converte a fração da circunscrição
/// em que ela se define num horário (sorteado). `None` se a circunscrição já está decidida ou sem eleitorado.
pub fn prever_mat(e: &EntradaMat, par: &Parametros, rng: &mut Rng, horario: &mut dyn FnMut(f64, &mut Rng) -> f64) -> Option<PrevMat> {
    if e.circ.eleitorado == 0 || avaliar(e.circ, e.votos, e.circ.eleitorado.saturating_sub(e.apurado)).circ != MatCirc::Apurando { return None; }
    let b = base(e, par);
    let n = e.votos.len();
    let ns = e.estratos.len();
    let elei = e.circ.eleitorado as f64;
    let (mut acc, mut votos) = (Vec::with_capacity(n), Vec::with_capacity(n));
    let mut x: Vec<Vec<f64>> = vec![vec![0.0; n]; ns];
    let mut gama = vec![1.0; ns];
    let mut tempos = Vec::with_capacity(par.sorteios);
    let mut fracs = Vec::with_capacity(par.sorteios);
    let (mut antes, mut eleito, mut segundo) = (0usize, 0usize, 0usize);
    let mut delta = vec![0.0; n];
    for _ in 0..par.sorteios.max(1) {
        for d in delta.iter_mut() { *d = par.deriva * rng.normal(); }
        for s in 0..ns {
            gama[s] = (par.ordem * rng.normal()).exp();
            let razao = b.razao[s] * (0.02 * rng.normal()).exp();
            let mut soma = 0.0;
            for i in 0..n {
                let q = (b.p[s][i] + delta[i] + b.dp[s][i] * rng.normal()).max(0.0);
                x[s][i] = q;
                soma += q;
            }
            let k = if soma > 0.0 { b.resto[s] * razao / soma } else { 0.0 };
            for v in x[s].iter_mut() { *v *= k; }
        }
        let (frac, tipo, antes_fim) = match definicao(e, &x, &b.resto, &gama, &mut acc, &mut votos) {
            Some((ap, c, a)) => (ap / elei, Some(c), a),
            None => (1.0, None, false),
        };
        if antes_fim { antes += 1; }
        match tipo { Some(MatCirc::Eleito) => eleito += 1, Some(MatCirc::SegundoTurno) => segundo += 1, _ => {} }
        fracs.push(frac.min(1.0));
        tempos.push(horario(frac.min(1.0), rng));
    }
    let faixa = Faixa::de(&mut tempos, par.max_faixa_s)?;
    let tipo = if segundo > eleito { MatCirc::SegundoTurno } else { MatCirc::Eleito };
    Some(PrevMat { tipo: tipo.texto(), faixa, prob_antes_fim: antes as f64 / par.sorteios.max(1) as f64, apurado_p50: percentil(&mut fracs, 0.5) })
}

// ------------------------------------------------------------------------------------------------ relógio

/// Converte fração apurada (dos aptos ou das seções) em horário, para uma abrangência num instante: pela curva dos
/// gabaritos ajustada à noite (ancorada no ponto atual) ou pela velocidade; os dois com o erro relativo do tempo que
/// falta.
pub struct Relogio<'a> {
    agora: i64,
    frac_aptos: f64,
    frac_secoes: f64,
    /// aptos/s e desvio (log) da velocidade na janela
    ema: f64,
    sigma_v: f64,
    eleitorado: f64,
    grade: &'a [f64],
    aptos: Vec<(&'a [f64], Ajuste)>,
    secoes: Vec<(&'a [f64], Ajuste)>,
    erro_rel: f64,
    modo: Tempo,
}

/// Teto do que se projeta adiante (s): além disso a estimativa não diz nada.
const TETO_S: f64 = 24.0 * 3600.0;

impl Relogio<'_> {
    fn resto(&self, r: f64, rng: &mut Rng) -> f64 { self.agora as f64 + (r.max(0.0) * (self.erro_rel * rng.normal()).exp()).min(TETO_S) }

    fn por_velocidade(&self, df: f64, rng: &mut Rng) -> f64 {
        let v = self.ema * (self.sigma_v * rng.normal()).exp();
        if v <= 1e-9 { TETO_S } else { df * self.eleitorado / v }
    }

    fn por_curva(&self, ajs: &[(&[f64], Ajuste)], de: f64, ate: f64, rng: &mut Rng) -> f64 {
        let (c, a) = ajs[rng.indice(ajs.len())];
        a.b * (tau(c, self.grade, ate) - tau(c, self.grade, de)) + a.sigma * 0.5 * rng.normal()
    }

    /// Curva ou velocidade neste sorteio.
    fn usa_curva(&self, ajs: &[(&[f64], Ajuste)], rng: &mut Rng) -> bool {
        !ajs.is_empty() && match self.modo { Tempo::Curva => true, Tempo::Velocidade => false, Tempo::Mista => rng.unif() < 0.5 }
    }

    /// Horário (sorteado) em que os aptos apurados chegam à fração `f`.
    pub fn ate_aptos(&self, f: f64, rng: &mut Rng) -> f64 {
        if f <= self.frac_aptos { return self.agora as f64; }
        let r = if self.usa_curva(&self.aptos, rng) { self.por_curva(&self.aptos, self.frac_aptos, f, rng) } else { self.por_velocidade(f - self.frac_aptos, rng) };
        self.resto(r, rng)
    }

    /// B-052: horário em que os aptos chegam à fração `f` no ritmo atual (sem sorteio); `None` sem ritmo.
    pub fn eta_aptos(&self, f: f64) -> Option<i64> { self.eta(f - self.frac_aptos) }

    /// B-052: horário em que as seções chegam à fração `g` no ritmo atual (as seções acompanham os aptos).
    pub fn eta_secoes(&self, g: f64) -> Option<i64> { self.eta(g - self.frac_secoes) }

    fn eta(&self, df: f64) -> Option<i64> {
        if df <= 0.0 { return Some(self.agora); }
        (self.ema > 1e-9).then(|| self.agora + (df * self.eleitorado / self.ema).min(TETO_S).round() as i64)
    }

    /// Horário (sorteado) em que as seções apuradas chegam à fração `g` (pela velocidade, as seções acompanham os aptos).
    pub fn ate_secoes(&self, g: f64, rng: &mut Rng) -> f64 {
        if g <= self.frac_secoes { return self.agora as f64; }
        let r = if self.usa_curva(&self.secoes, rng) { self.por_curva(&self.secoes, self.frac_secoes, g, rng) } else { self.por_velocidade(g - self.frac_secoes, rng) };
        self.resto(r, rng)
    }
}

// ------------------------------------------------------------------------------------------------ previsor

struct EstratoDef {
    /// (índice da região em `Estado::aptos`, início em `Estado::tally`, candidato local → circunscrição)
    regioes: Vec<(usize, usize, Vec<u32>)>,
    munis: Vec<usize>,
    /// eleitorado do estrato, quando publicado (ex.: Presidente por UF)
    eleitorado: Option<f64>,
    secoes: u64,
}

/// Uma abrangência acompanhada (um tópico): a série de avanço, a última previsão e o histórico das faixas do
/// "matematicamente".
pub struct Unidade {
    pub cargo: u8,
    pub abr: Abrangencia,
    /// a abrangência é a circunscrição do cargo (só ela tem "matematicamente")
    pub circ: bool,
    pub classe: &'static str,
    pub eleitorado: u64,
    /// seções esperadas (com BU): as do catálogo menos agregadas e não instaladas
    pub secoes: u64,
    /// (município, região em `Estado::aptos`)
    regioes: Vec<(usize, usize)>,
    estratos: Vec<EstratoDef>,
    curvas: Vec<usize>,
    pub serie: Serie,
    /// (instante, faixa) de cada previsão do "matematicamente" emitida
    pub historico: Vec<(i64, Faixa)>,
    /// instante em que saiu o "matematicamente"
    pub definida: Option<i64>,
    /// instantes em que as seções chegaram a 99 % e a 100 %, e do último aumento das seções
    pub chegou: [Option<i64>; 2],
    pub ultimo_aumento: Option<i64>,
    pub ultima: Option<Previsao>,
    /// a última previsão normal ("prevista"), base das adiadas durante uma parada
    normal: Option<Previsao>,
}

/// Previsão confirmada (ou não) depois que o "matematicamente" saiu.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Confirmacao {
    /// quando saiu
    pub saiu: i64,
    /// a última faixa prevista antes de sair, e se o horário real caiu nela
    pub faixa: Faixa,
    pub dentro: bool,
    /// desde quando todas as faixas previstas continham o horário real (None se a última não continha)
    pub desde: Option<i64>,
}

pub struct Previsor {
    pub par: Parametros,
    pub gab: Gabaritos,
    pub unidades: Vec<Unidade>,
    indice: HashMap<(u8, Abrangencia), usize>,
    /// threads da rodada (≥ 1)
    pub threads: usize,
    /// BUs aplicados no país ao longo da noite (para detectar a parada da chegada) e as seções esperadas no catálogo
    global: Serie,
    total_secoes: u64,
    /// parada em curso da chegada dos BUs (previsões "suspensas", adiadas)
    pub parada: Option<Parada>,
    /// paradas da noite: (desde, até)
    pub paradas: Vec<(i64, i64)>,
}

impl Previsor {
    /// Monta as unidades dos tópicos `(cargo, abrangência)`. `excluidas`: seções que não terão BU (agregadas, não
    /// instaladas). `excluir_noite`: não usar os gabaritos desta noite (backtest sem olhar a própria noite).
    /// Não guarda referências: o serviço o usa numa thread própria, com o estado (atômicos) e os candidatos.
    pub fn novo(cat: &Catalogo, cad: &Candidatos, topicos: &[(u8, Abrangencia)], excluidas: &HashSet<u64>,
                gab: Gabaritos, excluir_noite: Option<&str>, par: Parametros) -> Previsor {
        let nc = cat.nc();
        let mut sm = vec![0u64; cat.muni_cod.len()];
        for &k in cat.secoes.keys() {
            if excluidas.contains(&k) { continue; }
            if let Some(m) = cat.muni((k >> 32) as u32) { sm[m as usize] += 1; }
        }
        let mut unidades = vec![];
        for (cargo, abr) in topicos {
            let Some(ci) = cat.ci(*cargo) else { continue };
            let munis: Vec<usize> = match abr {
                Abrangencia::Brasil => (0..cat.muni_cod.len()).collect(),
                Abrangencia::Uf(u) => match cat.ufs.iter().position(|x| x == u) {
                    Some(ui) => (0..cat.muni_cod.len()).filter(|&m| cat.muni_uf[m] as usize == ui).collect(),
                    None => vec![],
                },
                Abrangencia::Municipio(_, mu) => cat.muni(*mu).map(|m| m as usize).into_iter().collect(),
            };
            let regioes: Vec<(usize, usize)> = munis.iter().map(|&m| (m, m * nc + ci)).filter(|&(_, k)| cat.base[k] != AUSENTE).collect();
            if regioes.is_empty() { continue; }
            let circ = cad.circunscricao(*cargo, abr);
            let secoes: u64 = regioes.iter().map(|&(m, _)| sm[m]).sum();
            let eleitorado = match circ { Some(c) => c.eleitorado, None => cad.eleitorado_abr.get(&(*cargo, abr.clone())).copied().unwrap_or(0) };
            let mut estratos: Vec<EstratoDef> = vec![];
            if let Some(c) = circ {
                // candidato local da região → índice em `c.candidatos` (como no `Acompanhamento`)
                let idx: HashMap<u32, u32> = c.candidatos.iter().enumerate().map(|(i, x)| (x.numero, i as u32)).collect();
                let mut chave: HashMap<usize, usize> = HashMap::new();
                for &(m, k) in &regioes {
                    let base = cat.base[k] as usize;
                    let mapa: Vec<u32> = cat.candidatos(m, ci).iter().map(|x| idx.get(x).copied().unwrap_or(u32::MAX)).collect();
                    let (id, conhecido) = match abr {
                        Abrangencia::Brasil => {
                            let u = cat.muni_uf[m] as usize;
                            (u, cad.eleitorado_abr.get(&(*cargo, Abrangencia::Uf(cat.ufs[u]))).map(|&x| x as f64))
                        }
                        Abrangencia::Uf(u) => (m, cad.eleitorado_abr.get(&(*cargo, Abrangencia::Municipio(*u, cat.muni_cod[m]))).map(|&x| x as f64)),
                        Abrangencia::Municipio(..) => (0, Some(eleitorado as f64)),
                    };
                    let s = *chave.entry(id).or_insert_with(|| { estratos.push(EstratoDef { regioes: vec![], munis: vec![], eleitorado: conhecido.filter(|&x| x > 0.0), secoes: 0 }); estratos.len() - 1 });
                    let e = &mut estratos[s];
                    e.regioes.push((k, base, mapa));
                    if !e.munis.contains(&m) { e.munis.push(m); e.secoes += sm[m]; }
                }
            }
            let circ = circ.is_some();
            let classe = classe(abr, eleitorado);
            let mut curvas = vec![];
            for c in vizinhas(classe) {
                curvas = gab.curvas.iter().enumerate().filter(|(_, x)| x.classe == *c && Some(x.noite.as_str()) != excluir_noite).map(|(i, _)| i).collect();
                if !curvas.is_empty() { break; }
            }
            unidades.push(Unidade { cargo: *cargo, abr: abr.clone(), circ, classe, eleitorado, secoes, regioes, estratos, curvas, serie: Serie::default(),
                historico: vec![], definida: None, chegou: [None, None], ultimo_aumento: None, ultima: None, normal: None });
        }
        let indice = unidades.iter().enumerate().map(|(i, u)| ((u.cargo, u.abr.clone()), i)).collect();
        Previsor { par, gab, unidades, indice, threads: 1, global: Serie::default(), total_secoes: sm.iter().sum(), parada: None, paradas: vec![] }
    }

    /// Índice da unidade do tópico.
    pub fn indice(&self, cargo: u8, abr: &Abrangencia) -> Option<usize> { self.indice.get(&(cargo, abr.clone())).copied() }

    /// O "matematicamente" da circunscrição saiu em `em` (evento do motor): a previsão dela congela.
    pub fn definir(&mut self, cargo: u8, abr: &Abrangencia, em: i64) {
        if let Some(i) = self.indice(cargo, abr) { let u = &mut self.unidades[i]; if u.circ && u.definida.is_none() { u.definida = Some(em); } }
    }

    /// Marca as circunscrições já decididas no acompanhamento (backtest e retomada).
    pub fn definir_pelo(&mut self, ac: &Acompanhamento) {
        for c in &ac.circs {
            if let (true, Some((_, em))) = (c.mat_circ != MatCirc::Apurando, c.circ_em) { self.definir(c.cargo, c.abr(), em); }
        }
    }

    /// Lê o avanço de cada unidade no estado e registra na série (barato: só somas por município).
    pub fn registrar(&mut self, est: &Estado, agora: i64) {
        let mv = self.par.meia_vida_s;
        // parada da chegada dos BUs no país
        let bus = est.bus.load(Relaxed);
        self.global.registrar(agora, bus, bus, mv);
        let fr = bus as f64 / self.total_secoes.max(1) as f64;
        let nova = detectar_parada(&self.global, agora, fr, self.parada, &self.par);
        if let (Some(p), None) = (self.parada, nova) { self.paradas.push((p.desde, agora)); }
        self.parada = nova;
        for u in &mut self.unidades {
            let aptos: u64 = u.regioes.iter().map(|&(_, k)| est.aptos[k].load(Relaxed) as u64).sum();
            let secoes: u64 = u.regioes.iter().map(|&(m, _)| est.secoes[m].load(Relaxed) as u64).sum();
            if u.serie.ultimo().is_some_and(|p| secoes > p.secoes) { u.ultimo_aumento = Some(agora); }
            u.serie.registrar(agora, aptos, secoes, mv);
            if u.secoes > 0 {
                if u.chegou[0].is_none() && secoes as f64 >= 0.99 * u.secoes as f64 { u.chegou[0] = Some(agora); }
                if u.chegou[1].is_none() && secoes >= u.secoes { u.chegou[1] = Some(agora); }
            }
        }
    }

    /// Roda a previsão das unidades `quais` (todas se `None`) no instante `agora` (depois de [`Previsor::registrar`]).
    /// Devolve os índices das unidades com previsão nova (em `Unidade::ultima`).
    pub fn prever(&mut self, cad: &Candidatos, est: &Estado, agora: i64, quais: Option<&[usize]>) -> Vec<usize> {
        if let Some(pa) = self.parada { return self.adiar(pa, agora); }
        let mut lista: Vec<usize> = match quais { Some(q) => q.to_vec(), None => (0..self.unidades.len()).collect() };
        // depois de uma parada, as previsões adiadas são refeitas mesmo sem BU novo no tópico
        let ja: HashSet<usize> = lista.iter().copied().collect();
        lista.extend((0..self.unidades.len()).filter(|i| !ja.contains(i) && self.unidades[*i].ultima.as_ref().is_some_and(|p| p.situacao == "suspensa")));
        let threads = self.threads.max(1).min(lista.len().max(1));
        let este = &*self;
        let res: Vec<(usize, Previsao)> = if threads <= 1 {
            lista.iter().map(|&i| (i, este.calcular(i, cad, est, agora))).collect()
        } else {
            let partes: Vec<&[usize]> = lista.chunks(lista.len().div_ceil(threads)).collect();
            std::thread::scope(|s| {
                let hs: Vec<_> = partes.into_iter().map(|p| s.spawn(move || p.iter().map(|&i| (i, este.calcular(i, cad, est, agora))).collect::<Vec<_>>())).collect();
                hs.into_iter().flat_map(|h| h.join().unwrap_or_default()).collect()
            })
        };
        let mut out = vec![];
        for (i, p) in res {
            let u = &mut self.unidades[i];
            if let Some(m) = &p.mat { u.historico.push((agora, m.faixa)); }
            if p.situacao == "prevista" { u.normal = Some(p.clone()); }
            u.ultima = Some(p);
            out.push(i);
        }
        out
    }

    /// Há rodada a fazer mesmo sem tópico que mudou: parada em curso, ou previsões adiadas a refazer depois dela.
    pub fn rodada_pendente(&self) -> bool {
        self.parada.is_some() || self.unidades.iter().any(|u| u.ultima.as_ref().is_some_and(|p| p.situacao == "suspensa"))
    }

    /// Durante uma parada da chegada dos BUs: em vez de prever com um ritmo quase zero, cada tópico com previsão normal
    /// recebe a última, **adiada** pelo tempo passado desde ela ("suspensa", com o início da parada). O que já
    /// aconteceu (matematicamente, 99 %, 100 %) não é adiado.
    fn adiar(&mut self, pa: Parada, agora: i64) -> Vec<usize> {
        let mut out = vec![];
        for (i, u) in self.unidades.iter_mut().enumerate() {
            let Some(b) = &u.normal else { continue };
            let d = (agora - b.instante).max(0);
            let mover = |f: &Faixa| Faixa { p10: f.p10 + d, p50: f.p50 + d, p90: f.p90 + d, incerta: f.incerta, eta: f.eta.map(|t| t + d) };
            let ult = u.serie.ultimo();
            let mat = if u.definida.is_some() { None } else { b.mat.map(|m| PrevMat { faixa: mover(&m.faixa), ..m }) };
            let p = Previsao {
                instante: agora,
                apurado: ult.filter(|_| u.eleitorado > 0).map_or(b.apurado, |x| x.aptos as f64 / u.eleitorado as f64),
                secoes: ult.filter(|_| u.secoes > 0).map_or(b.secoes, |x| x.secoes as f64 / u.secoes as f64),
                aptos_min: u.serie.ema.unwrap_or(0.0) * 60.0,
                situacao: if u.definida.is_some() && u.circ { "definida" } else { "suspensa" },
                mat,
                fim99: if u.chegou[0].is_some() { None } else { b.fim99.as_ref().map(mover) },
                fim100: if u.chegou[1].is_some() { None } else { b.fim100.as_ref().map(mover) },
                parada: Some(pa.desde),
            };
            // nada mais a prever neste tópico (já decidido e já com 100 %)
            if p.mat.is_none() && p.fim99.is_none() && p.fim100.is_none() { continue; }
            if let Some(m) = &p.mat { u.historico.push((agora, m.faixa)); }
            u.ultima = Some(p);
            out.push(i);
        }
        out
    }

    /// A previsão de uma unidade no instante (só leitura; roda em paralelo).
    fn calcular(&self, i: usize, cad: &Candidatos, est: &Estado, agora: i64) -> Previsao {
        let u = &self.unidades[i];
        let par = &self.par;
        let ult = u.serie.ultimo().unwrap_or(Ponto { t: agora, aptos: 0, secoes: 0 });
        let elei = if u.eleitorado > 0 { u.eleitorado as f64 } else { 0.0 };
        let fs = if u.secoes > 0 { ult.secoes as f64 / u.secoes as f64 } else { 0.0 };
        let fa = if elei > 0.0 { ult.aptos as f64 / elei } else { fs };
        let mut p = Previsao { instante: agora, apurado: fa, secoes: fs, aptos_min: u.serie.ema.unwrap_or(0.0) * 60.0, situacao: "insuficiente", mat: None, fim99: None, fim100: None, parada: None };
        let definida = u.definida.is_some();
        if definida { p.situacao = "definida"; }
        let curta = u.serie.primeiro.is_none_or(|t0| agora - t0 < par.min_serie_s);
        if fa < par.min_apurado || curta || u.serie.ema.is_none() || fs >= 1.0 { return p; }
        // relógio: ajuste de cada gabarito à janela, velocidade e dispersão
        let grade = &self.gab.grade[..];
        let obs_a = u.serie.janela(par.janela_s, true, if elei > 0.0 { elei } else { u.secoes as f64 });
        let obs_s = u.serie.janela(par.janela_s, false, u.secoes as f64);
        // a curva dos aptos só serve ao "matematicamente" (circunscrição ainda em aberto)
        let com_mat = u.circ && !definida;
        let aptos: Vec<(&[f64], Ajuste)> = if com_mat { u.curvas.iter().filter_map(|&c| { let x = &self.gab.curvas[c].aptos[..]; ajustar(x, grade, &obs_a).map(|a| (x, a)) }).collect() } else { vec![] };
        let secoes: Vec<(&[f64], Ajuste)> = u.curvas.iter().filter_map(|&c| { let x = &self.gab.curvas[c].secoes[..]; ajustar(x, grade, &obs_s).map(|a| (x, a)) }).collect();
        let mut taxas = u.serie.taxas(par.janela_s);
        let sigma_v = if taxas.len() >= 3 {
            let (p10, p90) = (percentil(&mut taxas, 0.1), percentil(&mut taxas, 0.9));
            if p10 > 0.0 { ((p90 / p10).ln() / 2.563).clamp(0.05, 1.0) } else { 0.6 }
        } else { 0.5 };
        // com poucas seções por chegar, o fim depende de uma ou outra seção atrasada: o erro relativo cresce
        let restam = u.secoes.saturating_sub(ult.secoes) as f64;
        let erro_rel = par.erro_rel * (1.0 + par.erro_poucas / (restam + 1.0).sqrt());
        let rel = Relogio { agora, frac_aptos: fa, frac_secoes: fs, ema: u.serie.ema.unwrap_or(0.0), sigma_v, eleitorado: if elei > 0.0 { elei } else { u.secoes as f64 },
            grade, aptos, secoes, erro_rel, modo: par.tempo };
        let mut rng = Rng::novo(semente(par.semente, i, agora));
        p.situacao = if definida { "definida" } else { "prevista" };
        // o fim da apuração é barato e vale para todo tópico (milhares de municípios): menos sorteios que o "matematicamente"
        let n = (par.sorteios / if u.circ { 2 } else { 4 }).max(32);
        let com_eta = |f: Option<Faixa>, eta: Option<i64>| f.map(|f| Faixa { eta, ..f });
        if fs < 0.99 { let mut v: Vec<f64> = (0..n).map(|_| rel.ate_secoes(0.99, &mut rng)).collect(); p.fim99 = com_eta(Faixa::de(&mut v, par.max_faixa_s), rel.eta_secoes(0.99)); }
        let mut v: Vec<f64> = (0..n).map(|_| rel.ate_secoes(1.0, &mut rng)).collect();
        p.fim100 = com_eta(Faixa::de(&mut v, par.max_faixa_s), rel.eta_secoes(1.0));
        if let (Some(circ), false) = (cad.circunscricao(u.cargo, &u.abr).filter(|_| u.circ), definida) {
            let nv = circ.candidatos.len();
            let mut estratos: Vec<EstratoVivo> = u.estratos.iter().map(|e| {
                let mut s = EstratoVivo { eleitorado: 0.0, apurado: 0, votos: vec![0; nv] };
                for (k, base, mapa) in &e.regioes {
                    s.apurado += est.aptos[*k].load(Relaxed) as u64;
                    for (j, &ix) in mapa.iter().enumerate() { if ix != u32::MAX { s.votos[ix as usize] += est.tally[base + j].load(Relaxed) as u64; } }
                }
                s
            }).collect();
            let secoes_ap: Vec<u64> = u.estratos.iter().map(|e| e.munis.iter().map(|&m| est.secoes[m].load(Relaxed) as u64).sum()).collect();
            estimar_eleitorado(&mut estratos, &u.estratos, &secoes_ap, elei);
            let estratos = juntar_pequenos(estratos, MAX_ESTRATOS);
            let mut votos = vec![0u64; nv];
            let mut apurado = 0;
            for s in &estratos { apurado += s.apurado; for (a, b) in votos.iter_mut().zip(&s.votos) { *a += b; } }
            let e = EntradaMat { circ, votos: &votos, apurado, estratos: &estratos };
            p.mat = prever_mat(&e, par, &mut rng, &mut |f, r| rel.ate_aptos(f, r));
            // no ritmo atual, até a fração dos aptos em que ele deve sair (a mediana dos sorteios)
            if let Some(m) = p.mat.as_mut() { m.faixa.eta = rel.eta_aptos(m.apurado_p50); }
        }
        p
    }

    /// Depois que o "matematicamente" saiu: a última faixa prevista e desde quando as faixas acertavam.
    pub fn confirmacao(&self, i: usize) -> Option<Confirmacao> {
        let u = &self.unidades[i];
        let saiu = u.definida?;
        let antes: Vec<&(i64, Faixa)> = u.historico.iter().filter(|(t, _)| *t < saiu).collect();
        let &(_, faixa) = *antes.last()?;
        let dentro = faixa.contem(saiu);
        let mut desde = None;
        for (t, f) in antes.iter().rev() { if f.contem(saiu) { desde = Some(*t); } else { break; } }
        Some(Confirmacao { saiu, faixa, dentro, desde })
    }
}

/// Estratos usados no Monte Carlo: os de maior restante ficam; os demais viram um só (custo: Minas tem 853 municípios).
pub const MAX_ESTRATOS: usize = 24;

/// Mantém os `max − 1` estratos com mais aptos por apurar e junta os outros num só (votos, apurado e eleitorado somados).
fn juntar_pequenos(mut v: Vec<EstratoVivo>, max: usize) -> Vec<EstratoVivo> {
    if v.len() <= max || max < 2 { return v; }
    v.sort_by(|a, b| (b.eleitorado - b.apurado as f64).partial_cmp(&(a.eleitorado - a.apurado as f64)).unwrap_or(std::cmp::Ordering::Equal));
    let resto = v.split_off(max - 1);
    let mut j = EstratoVivo { eleitorado: 0.0, apurado: 0, votos: vec![0; resto[0].votos.len()] };
    for s in resto {
        j.eleitorado += s.eleitorado;
        j.apurado += s.apurado;
        for (a, b) in j.votos.iter_mut().zip(&s.votos) { *a += b; }
    }
    v.push(j);
    v
}

/// Eleitorado de cada estrato: o publicado quando há; senão seções esperadas × aptos por seção observados (encolhidos
/// para a média da circunscrição), com o restante reescalado para a soma bater com o eleitorado da circunscrição.
fn estimar_eleitorado(vivos: &mut [EstratoVivo], defs: &[EstratoDef], secoes_ap: &[u64], elei_circ: f64) {
    const K_SECOES: f64 = 20.0;
    let (ap_tot, sec_tot): (f64, f64) = (vivos.iter().map(|s| s.apurado as f64).sum(), secoes_ap.iter().map(|&x| x as f64).sum());
    let secoes_esp: f64 = defs.iter().map(|d| d.secoes as f64).sum();
    let mu_c = if sec_tot > 0.0 { ap_tot / sec_tot } else if secoes_esp > 0.0 { elei_circ / secoes_esp } else { 0.0 };
    let mut conhecido = 0.0;
    let (mut resto_desc, mut ap_desc) = (0.0, 0.0);
    for (s, (v, d)) in vivos.iter_mut().zip(defs).enumerate() {
        match d.eleitorado {
            Some(x) => { v.eleitorado = x.max(v.apurado as f64); conhecido += v.eleitorado; }
            None => {
                let mu = (v.apurado as f64 + K_SECOES * mu_c) / (secoes_ap[s] as f64 + K_SECOES);
                v.eleitorado = (d.secoes as f64 * mu).max(v.apurado as f64);
                resto_desc += v.eleitorado - v.apurado as f64;
                ap_desc += v.apurado as f64;
            }
        }
    }
    let alvo = elei_circ - conhecido - ap_desc;
    if resto_desc > 0.0 && alvo > 0.0 {
        let k = alvo / resto_desc;
        for (v, d) in vivos.iter_mut().zip(defs) {
            if d.eleitorado.is_none() { v.eleitorado = v.apurado as f64 + (v.eleitorado - v.apurado as f64) * k; }
        }
    }
}

#[cfg(test)]
mod testes {
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
}
