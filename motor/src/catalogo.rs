//! Catálogo da totalização: o "layout calculável antes da eleição" (ADR-002/003/004).
//!
//! O **catálogo** faz o papel do "layout calculável antes da eleição": municípios e seções
//! (do `cs.json`) e candidatos por circunscrição: por (UF, cargo) nos cargos de eleição geral e por
//! (município, cargo) nos de circunscrição municipal (Prefeito, Vereador, consultas — ver
//! [`cargo_municipal`]). Em produção os candidatos viriam do registro de candidaturas; nos testes o
//! catálogo é montado a partir dos próprios BUs coletados (`uv-coletor catalogo`), em etapa medida à parte.

use crate::bu::Sink;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fmt::Write as _;

/// Slots extras de cada região, depois dos candidatos: legendas (partido 0..99) e 4 totais.
pub const N_LEGENDA: u32 = 100;
pub const OFF_BRANCO: u32 = N_LEGENDA;
pub const OFF_NULO: u32 = N_LEGENDA + 1;
pub const OFF_COMP: u32 = N_LEGENDA + 2;
pub const OFF_SEMCAND: u32 = N_LEGENDA + 3;
pub const OFF_VNT: u32 = N_LEGENDA + 4;
pub const EXTRA: u32 = N_LEGENDA + 5;
pub(crate) const AUSENTE: u32 = u32::MAX;

pub fn nome_cargo(c: u8) -> String {
    match c {
        1 => "Presidente", 3 => "Governador", 5 => "Senador", 6 => "Dep. Federal", 7 => "Dep. Estadual",
        8 => "Dep. Distrital", 11 => "Prefeito", 13 => "Vereador", _ => return format!("Cargo/consulta {c}"),
    }.to_string()
}

/// Cargo cuja circunscrição é o município: Prefeito (11), Vereador (13) e os cargos/consultas livres
/// (25..99). Os candidatos (ou opções) desses cargos são listados por município: o mesmo número
/// (ex.: 15) é um candidato diferente em cada município.
pub fn cargo_municipal(c: u8) -> bool { c == 11 || c == 13 || c >= 25 }

pub struct Catalogo {
    pub pleito: u32,
    pub ufs: Vec<[u8; 2]>,
    pub muni_cod: Vec<u32>,
    pub muni_uf: Vec<u8>,
    pub(crate) muni_ord: Vec<u32>,
    pub cargos: Vec<u8>,
    pub(crate) cargo_idx: [u8; 256],
    pub vagas: [u8; 256],
    /// por (uf, cargo): códigos dos candidatos, em ordem, e tabela direta código → índice.
    /// Nos cargos municipais é a união das listas dos municípios da UF (usada para consolidar por UF).
    pub cand_cod: Vec<Vec<u32>>,
    pub(crate) cand_tab: Vec<Vec<u32>>,
    /// por (município, cargo), só nos cargos municipais: códigos em ordem (busca binária) e, para cada
    /// um, o índice na lista da UF. Vazio nos demais cargos.
    pub mcand_cod: Vec<Vec<u32>>,
    pub(crate) mcand_uf: Vec<Vec<u32>>,
    /// cargos com listas por município (ver `montar`)
    pub municipal: [bool; 256],
    /// por (município, cargo): início da região no vetor e nº de candidatos
    pub base: Vec<u32>,
    pub ncand: Vec<u32>,
    pub slots: usize,
    pub secoes: HashMap<u64, u32>,
    /// Cargos presentes nos BUs mas deixados de fora de propósito (`restringir`): o agregador os pula
    /// sem contar como "votável desconhecido".
    pub ignorado: [bool; 256],
    /// Seções com votação **anulada** pela Junta Eleitoral (aux.json com situação "Anulada"; o BU publicado
    /// fica "Excluído"). A seção continua totalizada e com comparecimento, mas todos os seus votos contam
    /// como nulos, como na divulgação oficial (campo `vsan`).
    pub anuladas: std::collections::HashSet<u64>,
}

impl Catalogo {
    pub fn nc(&self) -> usize { self.cargos.len() }
    #[inline(always)]
    pub fn muni(&self, cod: u32) -> Option<u32> {
        self.muni_ord.get(cod as usize).copied().filter(|&o| o != AUSENTE)
    }
    #[inline(always)]
    pub fn ci(&self, cargo: u8) -> Option<usize> {
        let i = self.cargo_idx[cargo as usize];
        (i != 255).then_some(i as usize)
    }
    pub fn memoria_estado_mb(&self) -> f64 { self.slots as f64 * 4.0 / 1048576.0 }

    /// Monta as tabelas a partir de municípios, candidatos e seções.
    ///
    /// `cands`: candidatos por (UF, cargo). `mcands`: candidatos por (código do município, cargo), só
    /// nos cargos municipais ([`cargo_municipal`]). Um cargo municipal sem nenhuma entrada em `mcands`
    /// (catálogo antigo) é tratado como de circunscrição estadual, como antes.
    pub fn montar(pleito: u32, ufs: Vec<[u8; 2]>, munis: Vec<(u32, u8)>, mut cands: BTreeMap<(u8, u8), BTreeSet<u32>>,
                  mcands: BTreeMap<(u32, u8), BTreeSet<u32>>, vagas_lista: &[(u8, u8)], secoes: Vec<u64>) -> Catalogo {
        let mut muni_ord = vec![AUSENTE; 100_000];
        let (mut muni_cod, mut muni_uf) = (vec![], vec![]);
        for (cod, u) in munis {
            if (cod as usize) < muni_ord.len() && muni_ord[cod as usize] == AUSENTE {
                muni_ord[cod as usize] = muni_cod.len() as u32;
                muni_cod.push(cod);
                muni_uf.push(u);
            }
        }
        // cargos municipais: a lista da UF é a união das listas dos municípios
        let mut municipal = [false; 256];
        for ((mu, c), set) in &mcands {
            if !cargo_municipal(*c) { continue; }
            let Some(&o) = muni_ord.get(*mu as usize) else { continue };
            if o == AUSENTE { continue; }
            municipal[*c as usize] = true;
            cands.entry((muni_uf[o as usize], *c)).or_default().extend(set.iter().copied());
        }
        let cargos: Vec<u8> = cands.keys().map(|k| k.1).collect::<BTreeSet<_>>().into_iter().collect();
        let mut cargo_idx = [255u8; 256];
        for (i, &c) in cargos.iter().enumerate() { cargo_idx[c as usize] = i as u8; }
        let mut vagas = [1u8; 256];
        for &(c, v) in vagas_lista { vagas[c as usize] = v; }
        let nc = cargos.len();
        let mut cand_cod = vec![vec![]; ufs.len() * nc];
        let mut cand_tab = vec![vec![]; ufs.len() * nc];
        for ((u, c), set) in &cands {
            let k = *u as usize * nc + cargo_idx[*c as usize] as usize;
            cand_cod[k] = set.iter().copied().collect();
            if municipal[*c as usize] { continue; } // busca nas listas por município; sem tabela direta
            let max = set.iter().max().copied().unwrap_or(0) as usize;
            let mut t = vec![AUSENTE; max + 1];
            for (i, &cod) in cand_cod[k].iter().enumerate() { t[cod as usize] = i as u32; }
            cand_tab[k] = t;
        }
        let mut base = vec![AUSENTE; muni_cod.len() * nc];
        let mut ncand = vec![0; muni_cod.len() * nc];
        let mut mcand_cod = vec![vec![]; muni_cod.len() * nc];
        let mut mcand_uf = vec![vec![]; muni_cod.len() * nc];
        let mut slots = 0u64;
        for m in 0..muni_cod.len() {
            for c in 0..nc {
                let k = muni_uf[m] as usize * nc + c;
                let mk = m * nc + c;
                if municipal[cargos[c] as usize] {
                    // só os municípios com lista própria têm região (os demais não têm eleição para o cargo)
                    let Some(set) = mcands.get(&(muni_cod[m], cargos[c])) else { continue };
                    mcand_cod[mk] = set.iter().copied().collect();
                    mcand_uf[mk] = mcand_cod[mk].iter().map(|x| cand_cod[k].binary_search(x).map(|i| i as u32).unwrap_or(AUSENTE)).collect();
                    base[mk] = slots as u32;
                    ncand[mk] = mcand_cod[mk].len() as u32;
                    slots += (ncand[mk] + EXTRA) as u64;
                } else if cands.contains_key(&(muni_uf[m], cargos[c])) {
                    base[mk] = slots as u32;
                    ncand[mk] = cand_cod[k].len() as u32;
                    slots += (cand_cod[k].len() as u32 + EXTRA) as u64;
                }
            }
        }
        let secoes = secoes.into_iter().enumerate().map(|(i, k)| (k, i as u32)).collect();
        Catalogo { pleito, ufs, muni_cod, muni_uf, muni_ord, cargos, cargo_idx, vagas, cand_cod, cand_tab, mcand_cod, mcand_uf, municipal,
                   base, ncand, slots: slots as usize, secoes, ignorado: [false; 256], anuladas: Default::default() }
    }

    /// Listas por município dos cargos municipais (para `restringir` e `salvar`).
    fn listas_municipais(&self, so: Option<&[u8]>) -> BTreeMap<(u32, u8), BTreeSet<u32>> {
        let nc = self.nc();
        let mut out = BTreeMap::new();
        for m in 0..self.muni_cod.len() {
            for (ci, &c) in self.cargos.iter().enumerate() {
                if !self.municipal[c as usize] || so.is_some_and(|s| !s.contains(&c)) { continue; }
                let k = m * nc + ci;
                if self.base[k] == AUSENTE { continue; }
                out.insert((self.muni_cod[m], c), self.mcand_cod[k].iter().copied().collect());
            }
        }
        out
    }

    /// Lista de candidatos da região (município, cargo): a do município nos cargos municipais, a da UF nos demais.
    pub fn candidatos(&self, m: usize, ci: usize) -> &[u32] {
        let c = self.cargos[ci];
        if self.municipal[c as usize] { &self.mcand_cod[m * self.nc() + ci] } else { &self.cand_cod[self.muni_uf[m] as usize * self.nc() + ci] }
    }

    /// Catálogo só com os cargos pedidos (ex.: `[5]` = Senador). Os demais cargos dos BUs são pulados
    /// pelo agregador, sem alarme: serve para totalizar e medir um cargo isoladamente.
    pub fn restringir(&self, so: &[u8]) -> Catalogo {
        let nc = self.nc();
        let mut cands: BTreeMap<(u8, u8), BTreeSet<u32>> = BTreeMap::new();
        for u in 0..self.ufs.len() {
            for (ci, &c) in self.cargos.iter().enumerate() {
                if !so.contains(&c) { continue; }
                let k = u * nc + ci;
                if !self.cand_tab[k].is_empty() || self.base_uf_tem(u, ci) {
                    cands.insert((u as u8, c), self.cand_cod[k].iter().copied().collect());
                }
            }
        }
        let munis = self.muni_cod.iter().zip(&self.muni_uf).map(|(&m, &u)| (m, u)).collect();
        let vagas: Vec<(u8, u8)> = self.cargos.iter().map(|&c| (c, self.vagas[c as usize])).collect();
        let mut sec: Vec<(u64, u32)> = self.secoes.iter().map(|(k, v)| (*k, *v)).collect();
        sec.sort_by_key(|x| x.1);
        let mcands = self.listas_municipais(Some(so));
        let mut c = Catalogo::montar(self.pleito, self.ufs.clone(), munis, cands, mcands, &vagas, sec.into_iter().map(|x| x.0).collect());
        for &x in &self.cargos { if !so.contains(&x) { c.ignorado[x as usize] = true; } }
        c.anuladas = self.anuladas.clone();
        c
    }

    /// Lê as seções anuladas das métricas de uma coleta pela API (colunas uf, municipio, zona, secao,
    /// situacao): o BU escolhido tem situação "Excluído" quando a seção foi anulada. Devolve quantas.
    pub fn ler_anuladas(&mut self, arqs: &str) -> anyhow::Result<usize> {
        for p in arqs.split(',').filter(|s| !s.is_empty()) {
            let txt = std::fs::read_to_string(p)?;
            let mut l = txt.lines();
            let cab: Vec<&str> = l.next().unwrap_or("").split(',').collect();
            let c = |n: &str| cab.iter().position(|x| *x == n).ok_or_else(|| anyhow::anyhow!("{p}: sem coluna {n}"));
            let (cm, cz, cs, cst) = (c("municipio")?, c("zona")?, c("secao")?, c("situacao")?);
            for x in l {
                let f: Vec<&str> = x.split(',').collect();
                if f.len() <= cst || !f[cst].starts_with("Exclu") { continue; }
                let (Ok(m), Ok(z), Ok(s)) = (f[cm].parse::<u32>(), f[cz].parse::<u16>(), f[cs].parse::<u16>()) else { continue };
                self.anuladas.insert(crate::chave(m, z, s));
            }
        }
        Ok(self.anuladas.len())
    }

    /// Formato texto simples (uma linha por fato).
    pub fn salvar(&self, p: &std::path::Path) -> std::io::Result<()> {
        let mut s = format!("UVCAT 1 {}\n", self.pleito);
        for &c in &self.cargos { if self.vagas[c as usize] != 1 { let _ = writeln!(s, "V {c} {}", self.vagas[c as usize]); } }
        for u in &self.ufs { let _ = writeln!(s, "U {}", std::str::from_utf8(u).unwrap()); }
        for (i, &m) in self.muni_cod.iter().enumerate() { let _ = writeln!(s, "M {m} {}", self.muni_uf[i]); }
        for u in 0..self.ufs.len() {
            for (ci, &c) in self.cargos.iter().enumerate() {
                let k = u * self.nc() + ci;
                if !self.cand_tab[k].is_empty() || self.base_uf_tem(u, ci) {
                    let _ = write!(s, "C {u} {c}");
                    for x in &self.cand_cod[k] { let _ = write!(s, " {x}"); }
                    s.push('\n');
                }
            }
        }
        // K <município> <cargo> <candidatos…>: listas por município dos cargos municipais
        for ((m, c), set) in self.listas_municipais(None) {
            let _ = write!(s, "K {m} {c}");
            for x in set { let _ = write!(s, " {x}"); }
            s.push('\n');
        }
        let mut sec: Vec<(&u64, &u32)> = self.secoes.iter().collect();
        sec.sort_by_key(|x| x.1);
        for (k, _) in sec { let _ = writeln!(s, "S {} {} {}", k >> 32, (k >> 16) & 0xffff, k & 0xffff); }
        std::fs::write(p, s)
    }
    fn base_uf_tem(&self, u: usize, ci: usize) -> bool {
        (0..self.muni_cod.len()).any(|m| self.muni_uf[m] as usize == u && self.base[m * self.nc() + ci] != AUSENTE)
    }

    pub fn carregar(p: &std::path::Path) -> anyhow::Result<Catalogo> {
        let s = std::fs::read_to_string(p)?;
        let mut it = s.lines();
        let h: Vec<&str> = it.next().unwrap_or("").split_whitespace().collect();
        anyhow::ensure!(h.first() == Some(&"UVCAT"), "catálogo inválido: {}", p.display());
        let pleito = h.get(2).and_then(|x| x.parse().ok()).unwrap_or(0);
        let (mut ufs, mut munis, mut cands, mut vagas, mut secoes) = (vec![], vec![], BTreeMap::new(), vec![], vec![]);
        let mut mcands = BTreeMap::new();
        for l in it {
            let mut f = l.split_whitespace();
            match f.next() {
                Some("V") => vagas.push((f.next().unwrap().parse()?, f.next().unwrap().parse()?)),
                Some("U") => ufs.push(f.next().unwrap().as_bytes()[..2].try_into()?),
                Some("M") => munis.push((f.next().unwrap().parse()?, f.next().unwrap().parse()?)),
                Some("C") => {
                    let u: u8 = f.next().unwrap().parse()?;
                    let c: u8 = f.next().unwrap().parse()?;
                    let set: BTreeSet<u32> = f.map(|x| x.parse().unwrap()).collect();
                    cands.insert((u, c), set);
                }
                Some("K") => {
                    let m: u32 = f.next().unwrap().parse()?;
                    let c: u8 = f.next().unwrap().parse()?;
                    let set: BTreeSet<u32> = f.map(|x| x.parse().unwrap()).collect();
                    mcands.insert((m, c), set);
                }
                Some("S") => {
                    let v: Vec<u64> = f.map(|x| x.parse().unwrap()).collect();
                    secoes.push(crate::chave(v[0] as u32, v[1] as u16, v[2] as u16));
                }
                _ => {}
            }
        }
        Ok(Catalogo::montar(pleito, ufs, munis, cands, mcands, &vagas, secoes))
    }
}

/// Coleta candidatos lendo BUs — substituto do registro de candidaturas nos testes: por (UF, cargo) e,
/// nos cargos municipais, também por (município, cargo).
pub struct ColetaCandidatos<'a> {
    pub uf: u8,
    pub cargo: u8,
    pub municipio: u32,
    pub cands: &'a mut BTreeMap<(u8, u8), BTreeSet<u32>>,
    pub mcands: &'a mut BTreeMap<(u32, u8), BTreeSet<u32>>,
}
impl Sink for ColetaCandidatos<'_> {
    fn secao(&mut self, _: u32, mu: u32, _: u16, _: u16) { self.municipio = mu; }
    fn cargo(&mut self, c: u8, _: u32) {
        self.cargo = c;
        self.cands.entry((self.uf, c)).or_default();
        if cargo_municipal(c) { self.mcands.entry((self.municipio, c)).or_default(); }
    }
    fn voto(&mut self, tipo: u8, _: u32, _: u16, codigo: u32) {
        if tipo != 1 { return; }
        if cargo_municipal(self.cargo) { self.mcands.entry((self.municipio, self.cargo)).or_default().insert(codigo); }
        else { self.cands.entry((self.uf, self.cargo)).or_default().insert(codigo); }
    }
    fn fim_cargo(&mut self) {}
}

/// Candidatos lidos dos BUs em memória: por (índice da UF em `ufs`, cargo) e, nos cargos municipais, por
/// (município, cargo). Devolve também o nº de BUs que não puderam ser lidos. BUs de UF fora de `ufs` são pulados.
#[allow(clippy::type_complexity)]
pub fn candidatos_dos_bus(mem: &crate::pacote::EmMemoria, ufs: &[[u8; 2]]) -> (BTreeMap<(u8, u8), BTreeSet<u32>>, BTreeMap<(u32, u8), BTreeSet<u32>>, usize) {
    let mut cands: BTreeMap<(u8, u8), BTreeSet<u32>> = BTreeMap::new();
    let mut mcands: BTreeMap<(u32, u8), BTreeSet<u32>> = BTreeMap::new();
    let mut erros = 0;
    for i in 0..mem.itens.len() {
        let Some(ui) = ufs.iter().position(|u| *u == mem.itens[i].0.uf) else { continue };
        let mut c = ColetaCandidatos { uf: ui as u8, cargo: 0, municipio: 0, cands: &mut cands, mcands: &mut mcands };
        if crate::bu::ler_bu(mem.bu(i), &mut c).is_err() { erros += 1; }
    }
    (cands, mcands, erros)
}

impl Catalogo {
    /// Catálogo montado só com os BUs em memória: UFs, municípios e seções dos próprios pacotes (sem `cs.json`;
    /// seções sem BU ficam de fora) e candidatos lidos dos BUs. Serve aos testes e às conferências; na noite da
    /// eleição o catálogo vem de antes da apuração (etapa 2). Devolve também o nº de BUs ilegíveis.
    pub fn dos_pacotes(mem: &crate::pacote::EmMemoria, vagas: &[(u8, u8)]) -> (Catalogo, usize) {
        let ufs: Vec<[u8; 2]> = mem.itens.iter().map(|x| x.0.uf).collect::<BTreeSet<_>>().into_iter().collect();
        let munis: Vec<(u32, u8)> = mem.itens.iter()
            .map(|x| (x.0.municipio, ufs.iter().position(|u| *u == x.0.uf).unwrap() as u8)).collect::<BTreeSet<_>>().into_iter().collect();
        let mut vistas = HashSet::new();
        let secoes: Vec<u64> = mem.itens.iter().map(|x| x.0.chave()).filter(|k| vistas.insert(*k)).collect();
        let (cands, mcands, erros) = candidatos_dos_bus(mem, &ufs);
        let pleito = mem.itens.first().map_or(0, |x| x.0.pleito);
        (Catalogo::montar(pleito, ufs, munis, cands, mcands, vagas, secoes), erros)
    }

    /// Seções do catálogo por município (índice no catálogo).
    pub fn secoes_por_municipio(&self) -> Vec<u32> {
        let mut v = vec![0u32; self.muni_cod.len()];
        for k in self.secoes.keys() { if let Some(m) = self.muni((k >> 32) as u32) { v[m as usize] += 1; } }
        v
    }

    /// Seções anuladas a partir das métricas já lidas (ver [`Catalogo::ler_anuladas`]).
    pub fn usar_anuladas(&mut self, m: &crate::metricas::Metricas) -> usize {
        self.anuladas.extend(m.anuladas.iter().copied());
        self.anuladas.len()
    }

    /// Constrói o catálogo a partir de um `candidatos.json` (sem ler BUs): UFs, municípios e seções do arquivo e os
    /// candidatos de cada circunscrição. Cargos municipais (Prefeito, consultas) só têm região nos municípios da
    /// eleição (`municipios`, do `cm.json`), mesmo quando a circunscrição é a UF (consulta regional).
    pub fn de_arquivo_candidatos(arq: &crate::candidatos::ArquivoCandidatos) -> anyhow::Result<Catalogo> {
        use crate::oficial::Abrangencia;
        let uf2 = |s: &str| -> anyhow::Result<[u8; 2]> {
            let b = s.as_bytes();
            anyhow::ensure!(b.len() == 2 && b.iter().all(u8::is_ascii_lowercase), "UF inválida no candidatos.json: {s:?}");
            Ok([b[0], b[1]])
        };
        let mut ufs: Vec<[u8; 2]> = vec![];
        for s in arq.ufs.iter().map(String::as_str).chain(arq.municipios.iter().map(|m| m.uf.as_str())) {
            let u = uf2(s)?;
            if !ufs.contains(&u) { ufs.push(u); }
        }
        ufs.sort();
        let ui = |u: &[u8; 2]| ufs.iter().position(|x| x == u);
        let mut munis: Vec<(u32, u8)> = vec![];
        for m in &arq.municipios { munis.push((m.codigo, ui(&uf2(&m.uf)?).unwrap() as u8)); }
        let secoes: Vec<u64> = arq.secoes.iter().map(|s| crate::chave(s.municipio, s.zona, s.secao)).collect();
        let mut cands: BTreeMap<(u8, u8), BTreeSet<u32>> = BTreeMap::new();
        let mut mcands: BTreeMap<(u32, u8), BTreeSet<u32>> = BTreeMap::new();
        let mut vagas: Vec<(u8, u8)> = vec![];
        for ele in &arq.eleicoes {
            // municípios da eleição; sem a lista, todos os do arquivo
            let mut munis_el: Vec<([u8; 2], u32)> = vec![];
            for x in &ele.municipios {
                match Abrangencia::de_codigo(x) { Some(Abrangencia::Municipio(u, m)) => munis_el.push((u, m)), _ => anyhow::bail!("município inválido na eleição {}: {x:?}", ele.codigo) }
            }
            if munis_el.is_empty() { for m in &arq.municipios { munis_el.push((uf2(&m.uf)?, m.codigo)); } }
            for carg in &ele.cargos {
                vagas.push((carg.cargo, carg.vagas));
                let municipal = cargo_municipal(carg.cargo);
                for circ in &carg.circunscricoes {
                    let nums: BTreeSet<u32> = circ.candidatos.iter().map(|c| c.numero).collect();
                    let abr = Abrangencia::de_codigo(&circ.abr).ok_or_else(|| anyhow::anyhow!("abrangência inválida: {:?}", circ.abr))?;
                    let alvo = |u: &[u8; 2]| match &abr { Abrangencia::Brasil => true, Abrangencia::Uf(x) | Abrangencia::Municipio(x, _) => x == u };
                    match &abr {
                        Abrangencia::Municipio(_, m) => { mcands.entry((*m, carg.cargo)).or_default().extend(nums.iter().copied()); }
                        _ if municipal => {
                            for (u, m) in munis_el.iter().filter(|x| alvo(&x.0)) { let _ = u; mcands.entry((*m, carg.cargo)).or_default().extend(nums.iter().copied()); }
                        }
                        _ => {
                            let ufs_el: BTreeSet<[u8; 2]> = munis_el.iter().map(|x| x.0).filter(|u| alvo(u)).collect();
                            for u in ufs_el { if let Some(i) = ui(&u) { cands.entry((i as u8, carg.cargo)).or_default().extend(nums.iter().copied()); } }
                        }
                    }
                }
            }
        }
        Ok(Catalogo::montar(arq.pleito, ufs, munis, cands, mcands, &vagas, secoes))
    }
}
