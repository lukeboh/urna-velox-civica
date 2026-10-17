//! Totais por abrangência, lidos do estado: município (a região do próprio estado), UF (soma dos municípios) e
//! Brasil (soma das UFs, por número de candidato).

use crate::catalogo::{Catalogo, AUSENTE, EXTRA, N_LEGENDA, OFF_BRANCO, OFF_COMP, OFF_NULO, OFF_SEMCAND, OFF_VNT};
use crate::estado::Estado;
use crate::oficial::Abrangencia;
use std::collections::BTreeMap;
use std::sync::atomic::Ordering::Relaxed;

/// Totais de um cargo numa abrangência.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Totais {
    /// votos nominais por número digitado (inclui números sem candidato apto, que viram nulos técnicos)
    pub nominais: BTreeMap<u32, u64>,
    pub legenda: u64,
    pub branco: u64,
    pub nulo: u64,
    pub comparecimento: u64,
    pub sem_candidato: u64,
    /// votos em números sem candidato no catálogo pré-eleição (nulos técnicos registrados em OFF_VNT)
    pub vnt: u64,
}

impl Totais {
    /// `v` no layout da região: [candidatos `cods`…, legendas (100), branco, nulo, comparecimento, sem_candidato, vnt]
    pub fn de_vetor(cods: &[u32], v: &[u64]) -> Totais {
        let n = cods.len();
        let mut t = Totais {
            legenda: v[n..n + N_LEGENDA as usize].iter().sum(),
            branco: v[n + OFF_BRANCO as usize],
            nulo: v[n + OFF_NULO as usize],
            comparecimento: v[n + OFF_COMP as usize],
            sem_candidato: v[n + OFF_SEMCAND as usize],
            vnt: v[n + OFF_VNT as usize],
            ..Default::default()
        };
        for (i, &c) in cods.iter().enumerate() { if v[i] > 0 { *t.nominais.entry(c).or_default() += v[i]; } }
        t
    }
    pub fn somar(&mut self, o: &Totais) {
        for (c, q) in &o.nominais { *self.nominais.entry(*c).or_default() += q; }
        self.legenda += o.legenda;
        self.branco += o.branco;
        self.nulo += o.nulo;
        self.comparecimento += o.comparecimento;
        self.sem_candidato += o.sem_candidato;
        self.vnt += o.vnt;
    }
    pub fn votos(&self, numero: u32) -> u64 { self.nominais.get(&numero).copied().unwrap_or(0) }
}

/// Totais da região (município `m`, cargo `ci`); `None` se o cargo não tem região no município.
pub fn totais_municipio(cat: &Catalogo, est: &Estado, m: usize, ci: usize) -> Option<Totais> {
    let k = m * cat.nc() + ci;
    if cat.base[k] == AUSENTE { return None; }
    let (b, n) = (cat.base[k] as usize, (cat.ncand[k] + EXTRA) as usize);
    let v: Vec<u64> = (0..n).map(|j| est.tally[b + j].load(Relaxed) as u64).collect();
    Some(Totais::de_vetor(cat.candidatos(m, ci), &v))
}

/// Totais da UF `u` (índice em `cat.ufs`) no cargo `ci`; `None` se nenhum município da UF tem o cargo.
pub fn totais_uf(cat: &Catalogo, est: &Estado, u: usize, ci: usize) -> Option<Totais> {
    let ms: Vec<usize> = (0..cat.muni_cod.len()).filter(|&m| cat.muni_uf[m] as usize == u).collect();
    totais_lista(cat, est, u, ci, &ms)
}

/// Totais da UF `u` somando só os municípios `munis` (todos da UF `u`).
pub fn totais_lista(cat: &Catalogo, est: &Estado, u: usize, ci: usize, munis: &[usize]) -> Option<Totais> {
    let nc = cat.nc();
    let cods = &cat.cand_cod[u * nc + ci];
    let mut v = vec![0u64; cods.len() + EXTRA as usize];
    let mut algum = false;
    for &m in munis {
        if cat.base[m * nc + ci] == AUSENTE { continue; }
        est.somar_na_uf(cat, m, ci, &mut v);
        algum = true;
    }
    algum.then(|| Totais::de_vetor(cods, &v))
}

/// Totais do Brasil no cargo `ci` (todas as UFs, inclusive o exterior).
pub fn totais_brasil(cat: &Catalogo, est: &Estado, ci: usize) -> Option<Totais> {
    let mut t: Option<Totais> = None;
    for u in 0..cat.ufs.len() {
        if let Some(x) = totais_uf(cat, est, u, ci) { t.get_or_insert_with(Totais::default).somar(&x); }
    }
    t
}

/// Totais de uma abrangência; `None` se o cargo não existe nela (ou a abrangência não está no catálogo).
pub fn totais(cat: &Catalogo, est: &Estado, abr: &Abrangencia, cargo: u8) -> Option<Totais> {
    let ci = cat.ci(cargo)?;
    match abr {
        Abrangencia::Brasil => totais_brasil(cat, est, ci),
        Abrangencia::Uf(uf) => totais_uf(cat, est, cat.ufs.iter().position(|x| x == uf)?, ci),
        Abrangencia::Municipio(uf, mu) => {
            let m = cat.muni(*mu)? as usize;
            if cat.ufs[cat.muni_uf[m] as usize] != *uf { return None; }
            totais_municipio(cat, est, m, ci)
        }
    }
}

// ------------------------------------------------------------------ publicação (JSON, docs/FORMATO_RESULTADO.md)

use crate::acompanhamento::Acompanhamento;
use crate::candidatos::{circunscricao_de, Candidatos};
use crate::catalogo::cargo_municipal;
use crate::metricas::Metricas;
use crate::situacao::{ordem, situacao_final, votos_dos_candidatos, Situacao};
use serde_json::{json, Value};
use std::cell::RefCell;
use std::collections::HashMap;

/// Versão do formato publicado (campo `v`).
pub const VERSAO_FORMATO: u32 = 1;

/// Em que ponto da apuração a publicação é feita.
#[derive(Clone, Copy, Debug, Default)]
pub struct Momento {
    /// totalização encerrada: a situação final (`st`) é publicada
    pub encerrada: bool,
    /// nº de BUs aplicados
    pub seq: u64,
    /// horário do último BU aplicado (segundos, hora de Brasília)
    pub ultimo_bu: Option<i64>,
}

/// Monta os resultados por (cargo, abrangência) no formato documentado em `docs/FORMATO_RESULTADO.md`.
pub struct Publicador<'a> {
    pub cat: &'a Catalogo,
    pub cad: Option<&'a Candidatos>,
    /// seções por município: do catálogo, mais as não instaladas que o catálogo não tem
    secoes_total: Vec<u64>,
    /// seções não instaladas por município (contam como totalizadas, como em `s.st`)
    nao_inst: Vec<u64>,
    /// não instaladas em municípios fora do catálogo (sem nenhum BU), por UF: entram nos totais da UF e do Brasil
    nao_inst_fora: HashMap<[u8; 2], u64>,
    munis_uf: Vec<Vec<usize>>,
    /// situação final por (cargo, circunscrição), calculada uma vez por publicação encerrada
    cache: RefCell<HashMap<(u8, Abrangencia), Vec<Situacao>>>,
    /// totais por (UF, cargo) já somados, quando quem publica avisa as UFs que mudaram ([`Publicador::ativar_cache_uf`]):
    /// o Brasil passa a custar só as UFs tocadas desde a publicação anterior
    cache_uf: Option<RefCell<HashMap<(usize, usize), Option<Totais>>>>,
}

fn arred2(x: f64) -> f64 { (x * 100.0).round() / 100.0 }

impl<'a> Publicador<'a> {
    pub fn novo(cat: &'a Catalogo, cad: Option<&'a Candidatos>, met: Option<&Metricas>) -> Publicador<'a> {
        let mut secoes_total: Vec<u64> = cat.secoes_por_municipio().into_iter().map(u64::from).collect();
        // seções agregadas: sem urna nem BU próprio (os votos vêm no BU da principal); o TSE não as conta em `s.ts`.
        // Ficam fora do total — se uma vier com BU, `secoes` a conta (total nunca abaixo das apuradas)
        if let Some(cad) = cad {
            for &k in &cad.agregadas {
                if !cat.secoes.contains_key(&k) { continue; }
                if let Some(m) = cat.muni((k >> 32) as u32) { secoes_total[m as usize] = secoes_total[m as usize].saturating_sub(1); }
            }
        }
        let mut nao_inst = vec![0u64; cat.muni_cod.len()];
        let mut nao_inst_fora: HashMap<[u8; 2], u64> = HashMap::new();
        if let Some(met) = met {
            for (uf, k) in &met.nao_instaladas {
                let m = cat.muni((k >> 32) as u32).filter(|&m| cat.ufs[cat.muni_uf[m as usize] as usize] == *uf);
                let Some(m) = m else { *nao_inst_fora.entry(*uf).or_default() += 1; continue };
                nao_inst[m as usize] += 1;
                if !cat.secoes.contains_key(k) { secoes_total[m as usize] += 1; }
            }
        }
        let mut munis_uf = vec![vec![]; cat.ufs.len()];
        for m in 0..cat.muni_cod.len() { munis_uf[cat.muni_uf[m] as usize].push(m); }
        Publicador { cat, cad, secoes_total, nao_inst, nao_inst_fora, munis_uf, cache: RefCell::new(HashMap::new()), cache_uf: None }
    }

    /// Guarda os totais de cada (UF, cargo) entre publicações. Quem ativa **tem de** chamar
    /// [`Publicador::invalidar_uf`] a cada BU aplicado, retirado ou substituído na UF (o `uv-servico` faz isso); sem
    /// isso a publicação mostraria totais velhos.
    pub fn ativar_cache_uf(&mut self) { self.cache_uf = Some(RefCell::new(HashMap::new())); }

    /// A UF `uf` mudou: esquece os totais dela (todos os cargos).
    pub fn invalidar_uf(&self, uf: [u8; 2]) {
        let Some(c) = &self.cache_uf else { return };
        let Some(ui) = self.cat.ufs.iter().position(|x| *x == uf) else { return };
        c.borrow_mut().retain(|k, _| k.0 != ui);
    }

    /// Esquece todos os totais guardados.
    pub fn invalidar_tudo(&self) { if let Some(c) = &self.cache_uf { c.borrow_mut().clear(); } }

    /// Totais de (UF, cargo), do cache quando ativo.
    fn totais_uf_cache(&self, est: &Estado, ui: usize, ci: usize) -> Option<Totais> {
        let Some(c) = &self.cache_uf else { return totais_lista(self.cat, est, ui, ci, &self.munis_uf[ui]) };
        if let Some(t) = c.borrow().get(&(ui, ci)) { return t.clone(); }
        let t = totais_lista(self.cat, est, ui, ci, &self.munis_uf[ui]);
        c.borrow_mut().insert((ui, ci), t.clone());
        t
    }

    /// Esquece as situações calculadas (chamar antes de uma nova publicação encerrada).
    pub fn limpar_cache(&self) { self.cache.borrow_mut().clear(); }

    /// Uma seção não instalada descoberta durante a apuração (situação do `aux.json`): passa a contar como
    /// totalizada, como em `s.st`. Quem chama garante que cada seção é informada uma vez só.
    pub fn nao_instalada(&mut self, uf: [u8; 2], k: u64) {
        let cat = self.cat;
        match cat.muni((k >> 32) as u32).filter(|&m| cat.ufs[cat.muni_uf[m as usize] as usize] == uf) {
            None => *self.nao_inst_fora.entry(uf).or_default() += 1,
            Some(m) => {
                self.nao_inst[m as usize] += 1;
                if !cat.secoes.contains_key(&k) { self.secoes_total[m as usize] += 1; }
            }
        }
    }

    /// (seções, seções apuradas = BUs + não instaladas) de uma abrangência.
    pub fn secoes(&self, est: &Estado, abr: &Abrangencia) -> (u64, u64) {
        let (mut total, mut apuradas) = (0u64, 0u64);
        for m in self.munis(abr) {
            let ap = est.secoes[m].load(Relaxed) as u64 + self.nao_inst[m];
            // total nunca abaixo das apuradas (uma seção agregada que vier com BU passa a contar)
            total += self.secoes_total[m].max(ap);
            apuradas += ap;
        }
        let fora: u64 = match abr {
            Abrangencia::Brasil => self.nao_inst_fora.values().sum(),
            Abrangencia::Uf(u) => self.nao_inst_fora.get(u).copied().unwrap_or(0),
            Abrangencia::Municipio(..) => 0,
        };
        (total + fora, apuradas + fora)
    }

    fn munis(&self, abr: &Abrangencia) -> Vec<usize> {
        match abr {
            Abrangencia::Brasil => (0..self.cat.muni_cod.len()).collect(),
            Abrangencia::Uf(u) => self.cat.ufs.iter().position(|x| x == u).map(|i| self.munis_uf[i].clone()).unwrap_or_default(),
            Abrangencia::Municipio(u, mu) => self.cat.muni(*mu).map(|m| m as usize).filter(|&m| self.cat.ufs[self.cat.muni_uf[m] as usize] == *u).into_iter().collect(),
        }
    }

    /// Totais de uma abrangência usando as listas de municípios por UF já montadas.
    pub fn totais(&self, est: &Estado, abr: &Abrangencia, ci: usize) -> Option<Totais> {
        let cat = self.cat;
        match abr {
            Abrangencia::Municipio(..) => { let m = *self.munis(abr).first()?; totais_municipio(cat, est, m, ci) }
            Abrangencia::Uf(u) => { let ui = cat.ufs.iter().position(|x| x == u)?; self.totais_uf_cache(est, ui, ci) }
            Abrangencia::Brasil => {
                let mut t: Option<Totais> = None;
                for ui in 0..cat.ufs.len() {
                    if let Some(x) = self.totais_uf_cache(est, ui, ci) { t.get_or_insert_with(Totais::default).somar(&x); }
                }
                t
            }
        }
    }

    /// Cargos publicados: os dos candidatos (ou, sem candidatos, os do catálogo) que existem no catálogo.
    pub fn cargos(&self) -> Vec<u8> {
        let v = self.cad.map(|c| c.cargos()).unwrap_or_else(|| self.cat.cargos.clone());
        v.into_iter().filter(|&c| self.cat.ci(c).is_some()).collect()
    }

    /// Abrangências publicadas de um cargo: Brasil (Presidente), UFs (cargos gerais) e municípios (cargos
    /// municipais; com `municipais_gerais`, também os cargos gerais por município).
    pub fn abrangencias(&self, cargo: u8, municipais_gerais: bool) -> Vec<Abrangencia> {
        let Some(ci) = self.cat.ci(cargo) else { return vec![] };
        let nc = self.cat.nc();
        let tem = |m: usize| self.cat.base[m * nc + ci] != crate::catalogo::AUSENTE;
        let mut out = vec![];
        let municipal = cargo_municipal(cargo);
        // com os candidatos, as abrangências são as deles (as dos arquivos de divulgação)
        if let Some(v) = self.cad.and_then(|c| c.abrangencias.get(&cargo)).filter(|v| !v.is_empty()) {
            out = v.clone();
            if municipais_gerais && !municipal {
                for m in 0..self.cat.muni_cod.len() {
                    let a = Abrangencia::Municipio(self.cat.ufs[self.cat.muni_uf[m] as usize], self.cat.muni_cod[m]);
                    if tem(m) && !out.contains(&a) { out.push(a); }
                }
            }
            return out;
        }
        if !municipal {
            if cargo == 1 { out.push(Abrangencia::Brasil); }
            for (u, ms) in self.munis_uf.iter().enumerate() { if ms.iter().any(|&m| tem(m)) { out.push(Abrangencia::Uf(self.cat.ufs[u])); } }
        }
        if municipal || municipais_gerais {
            for m in 0..self.cat.muni_cod.len() { if tem(m) { out.push(Abrangencia::Municipio(self.cat.ufs[self.cat.muni_uf[m] as usize], self.cat.muni_cod[m])); } }
        }
        out
    }

    /// Nome do arquivo de uma publicação: `<abr>-c<cargo>-e<eleição>.json` (como os oficiais, sem o `-u`).
    pub fn nome_arquivo(&self, abr: &Abrangencia, cargo: u8) -> String {
        let e = self.cad.and_then(|c| c.eleicao_de(cargo, abr)).map_or(0, |x| x.0);
        format!("{}-c{cargo:04}-e{e:06}.json", abr.codigo())
    }

    /// O resultado de um cargo numa abrangência; `None` se o cargo não existe nela.
    pub fn valor(&self, est: &Estado, ac: Option<&Acompanhamento>, abr: &Abrangencia, cargo: u8, mo: Momento) -> Option<Value> {
        let cat = self.cat;
        let ci = cat.ci(cargo)?;
        let t = self.totais(est, abr, ci)?;
        let nc = cat.nc();
        let ms = self.munis(abr);
        let (mut total, mut apuradas, mut apurado) = (0u64, 0u64, 0u64);
        for &m in &ms {
            total += self.secoes_total[m];
            apuradas += est.secoes[m].load(Relaxed) as u64 + self.nao_inst[m];
            if cat.base[m * nc + ci] != AUSENTE { apurado += est.aptos[m * nc + ci].load(Relaxed) as u64; }
        }
        let fora: u64 = match abr {
            Abrangencia::Brasil => self.nao_inst_fora.values().sum(),
            Abrangencia::Uf(u) => self.nao_inst_fora.get(u).copied().unwrap_or(0),
            Abrangencia::Municipio(..) => 0,
        };
        total += fora;
        apuradas += fora;
        let circ_abr = circunscricao_de(cargo, abr);
        let circ = self.cad.and_then(|c| c.circunscricao(cargo, &circ_abr));
        let (eleicao, turno) = self.cad.and_then(|c| c.eleicao_de(cargo, abr)).unwrap_or((0, 1));
        let eleitorado = self.cad.and_then(|c| c.eleitorado_abr.get(&(cargo, abr.clone())).copied())
            .or_else(|| circ.filter(|c| c.abr == *abr).map(|c| c.eleitorado));
        let mut cand = vec![];
        let (estado, vnt, validos);
        match circ {
            Some(circ) => {
                let v = votos_dos_candidatos(circ, &t);
                let lista: std::collections::HashSet<u32> = circ.candidatos.iter().map(|c| c.numero).collect();
                vnt = t.vnt + t.nominais.iter().filter(|(n, _)| !lista.contains(n)).map(|(_, q)| *q).sum::<u64>();
                validos = (0..v.len()).filter(|&i| circ.candidatos[i].destinacao.apto()).map(|i| v[i]).sum::<u64>();
                // situação: só na publicação encerrada, calculada na circunscrição inteira
                let st: Option<Vec<Situacao>> = mo.encerrada.then(|| {
                    self.cache.borrow_mut().entry((cargo, circ_abr.clone())).or_insert_with(|| {
                        let tc = if circ_abr == *abr { t.clone() } else { self.totais(est, &circ_abr, ci).unwrap_or_default() };
                        situacao_final(circ, &votos_dos_candidatos(circ, &tc))
                    }).clone()
                });
                let acomp = ac.and_then(|a| a.circ_de(cargo, &circ_abr));
                estado = if apuradas == 0 {
                    "apurando"
                } else {
                    match (&st, acomp) {
                        (Some(st), _) if !circ.finalizada || st.iter().all(|&s| s == Situacao::Pendente) => "apurando",
                        (Some(st), _) if st.iter().any(|&s| s == Situacao::Eleito || s == Situacao::SegundoTurno) => "definido",
                        (Some(_), _) => "sem_eleito",
                        (None, Some(a)) => a.mat_circ.texto(),
                        (None, None) => "apurando",
                    }
                };
                for i in ordem(circ, &v) {
                    let c = &circ.candidatos[i];
                    let mat = acomp.and_then(|a| a.mat_em[i].filter(|_| a.mat[i] != crate::matematica::Mat::Nao)
                        .map(|(seq, em)| json!({"estado": a.mat[i].texto(), "seq": seq, "em": crate::metricas::iso_brasilia(em)})));
                    let pct = (c.destinacao.apto() && validos > 0).then(|| arred2(v[i] as f64 * 100.0 / validos as f64));
                    // enquanto "apurando", nenhuma situação (como na divulgação, que só a dá com a totalização finalizada)
                    let st_i = match &st { Some(s) if estado != "apurando" => s[i].texto(), _ => "" };
                    cand.push(json!({"n": c.numero, "v": v[i], "pct": pct, "dvt": c.destinacao.texto(),
                        "st": st_i, "mat": mat}));
                }
            }
            None => {
                // sem candidatos: todos os números votados, sem destinação nem situação
                vnt = 0;
                validos = t.nominais.values().sum();
                let mut ns: Vec<(u32, u64)> = t.nominais.iter().map(|(a, b)| (*a, *b)).collect();
                ns.sort_by_key(|&(n, q)| (std::cmp::Reverse(q), n));
                for (n, q) in ns {
                    let pct = (validos > 0).then(|| arred2(q as f64 * 100.0 / validos as f64));
                    cand.push(json!({"n": n, "v": q, "pct": pct, "dvt": Value::Null, "st": "", "mat": Value::Null}));
                }
                estado = "apurando";
            }
        }
        Some(json!({
            "v": VERSAO_FORMATO, "eleicao": eleicao, "turno": turno, "cargo": cargo, "abr": abr.codigo(), "circ": circ_abr.codigo(),
            "seq": mo.seq, "ultimo_bu": mo.ultimo_bu.map(crate::metricas::iso_brasilia), "encerrada": mo.encerrada,
            "secoes": {"total": total.max(apuradas), "apuradas": apuradas}, "eleitorado": eleitorado, "apurado": apurado,
            "comparecimento": t.comparecimento, "vb": t.branco, "vn": t.nulo, "vnt": vnt, "validos": validos,
            "legenda": t.legenda, "estado": estado, "cand": cand,
        }))
    }

    /// [`Publicador::valor`] em texto (uma linha, terminada em `\n`).
    pub fn json(&self, est: &Estado, ac: Option<&Acompanhamento>, abr: &Abrangencia, cargo: u8, mo: Momento) -> Option<String> {
        self.valor(est, ac, abr, cargo, mo).map(|v| v.to_string() + "\n")
    }

    /// Grava todas as publicações (cargos × abrangências) numa pasta. Devolve quantos arquivos.
    pub fn gravar_tudo(&self, est: &Estado, ac: Option<&Acompanhamento>, mo: Momento, municipais_gerais: bool, dir: &std::path::Path) -> std::io::Result<usize> {
        self.gravar(est, ac, mo, &self.cargos(), municipais_gerais, dir)
    }

    /// Grava as publicações de `cargos` × abrangências numa pasta. Devolve quantos arquivos.
    pub fn gravar(&self, est: &Estado, ac: Option<&Acompanhamento>, mo: Momento, cargos: &[u8], municipais_gerais: bool, dir: &std::path::Path) -> std::io::Result<usize> {
        std::fs::create_dir_all(dir)?;
        self.limpar_cache();
        let mut n = 0;
        for &cargo in cargos {
            for abr in self.abrangencias(cargo, municipais_gerais) {
                if let Some(s) = self.json(est, ac, &abr, cargo, mo) { gravar_atomico(dir, &self.nome_arquivo(&abr, cargo), &s)?; n += 1; }
            }
        }
        Ok(n)
    }
}

/// Grava `conteudo` em `dir/nome` sem que um leitor veja o arquivo pela metade (arquivo temporário + rename).
pub fn gravar_atomico(dir: &std::path::Path, nome: &str, conteudo: &str) -> std::io::Result<()> {
    let tmp = dir.join(format!(".{nome}.tmp"));
    std::fs::write(&tmp, conteudo)?;
    std::fs::rename(&tmp, dir.join(nome))
}
