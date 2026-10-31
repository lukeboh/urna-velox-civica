//! Estado da totalização (ADR-002/003/004): um vetor denso de `AtomicU32`, uma região por (município, cargo),
//! UF e Brasil por soma; o agregador de cada thread recebe os votos do leitor de BU e incrementa o vetor.

use crate::bu::{ler_bu, Sink};
use crate::catalogo::{nome_cargo, Catalogo, AUSENTE, EXTRA, N_LEGENDA, OFF_BRANCO, OFF_COMP, OFF_NULO, OFF_SEMCAND, OFF_VNT};
use crate::pacote::EmMemoria;
use std::collections::HashMap;
use std::fmt::Write as _;
use std::sync::atomic::{AtomicU32, AtomicU64, AtomicUsize, Ordering::Relaxed};
use std::sync::Mutex;

/// Registro de alerta de votável desconhecido: número votado que não constava no catálogo pré-eleição.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlertaVnt {
    pub cargo: u8,
    pub muni_cod: u32,
    pub numero: u32,
    pub votos: u64,
}

pub struct Estado {
    pub tally: Vec<AtomicU32>,
    /// eleitores aptos das seções já totalizadas, por região (município × cargo, índice `m * nc + ci`): o
    /// eleitorado já apurado, para o "matematicamente eleito"
    pub aptos: Vec<AtomicU32>,
    /// seções totalizadas (com BU aplicado) por município (índice no catálogo)
    pub secoes: Vec<AtomicU32>,
    vistos: Vec<AtomicU64>,
    pub bus: AtomicU64,
    pub cargos_invalidos: AtomicU64,
    pub desconhecidos: AtomicU64,
    pub duplicados: AtomicU64,
    pub fora_catalogo: AtomicU64,
    pub divergentes: AtomicU64,
    pub erros: AtomicU64,
    /// Alertas de votáveis desconhecidos agregados: (cargo, muni_cod, numero) -> quantidade
    pub alertas_vnt: Mutex<HashMap<(u8, u32, u32), u64>>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Aplicado {
    Ok,
    Duplicado,
    ForaDoCadastro,
    Erro,
    /// retirada de uma seção que não tem BU aplicado
    Ausente,
}

/// Sentido da agregação: somar um BU ao estado ou retirá-lo (o mesmo caminho, com `fetch_sub`), para que um BU
/// substituído ou uma seção anulada depois de totalizada voltem o estado exatamente ao que seria sem eles (ADR-019).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sinal { Somar, Retirar }

impl Estado {
    pub fn novo(cat: &Catalogo) -> Estado {
        Estado {
            tally: (0..cat.slots).map(|_| AtomicU32::new(0)).collect(),
            aptos: (0..cat.muni_cod.len() * cat.nc()).map(|_| AtomicU32::new(0)).collect(),
            secoes: (0..cat.muni_cod.len()).map(|_| AtomicU32::new(0)).collect(),
            vistos: (0..cat.secoes.len() / 64 + 1).map(|_| AtomicU64::new(0)).collect(),
            bus: AtomicU64::new(0), cargos_invalidos: AtomicU64::new(0), desconhecidos: AtomicU64::new(0),
            duplicados: AtomicU64::new(0), fora_catalogo: AtomicU64::new(0), divergentes: AtomicU64::new(0), erros: AtomicU64::new(0),
            alertas_vnt: Mutex::new(HashMap::new()),
        }
    }

    /// Alertas de votáveis desconhecidos registrados durante a totalização.
    pub fn lista_alertas_vnt(&self) -> Vec<AlertaVnt> {
        let mut v: Vec<AlertaVnt> = self.alertas_vnt.lock().unwrap().iter().map(|(&(c, m, n), &q)| {
            AlertaVnt { cargo: c, muni_cod: m, numero: n, votos: q }
        }).collect();
        v.sort_by_key(|a| (a.cargo, a.muni_cod, a.numero));
        v
    }

    /// Aplica um BU. `chave` vem do endereço/nome do arquivo e é conferida com a identificação lida no BU.
    /// A seção é anulada (votos como nulos) se estiver em `cat.anuladas`.
    pub fn aplicar(&self, cat: &Catalogo, chave: u64, bu: &[u8], ag: &mut Agregador) -> Aplicado {
        self.executar(cat, chave, bu, ag, Sinal::Somar, None)
    }

    /// Aplica um BU dizendo se a seção está anulada (situação lida do `aux.json` no momento da coleta), sem
    /// consultar `cat.anuladas`.
    pub fn aplicar_secao(&self, cat: &Catalogo, chave: u64, bu: &[u8], anulada: bool, ag: &mut Agregador) -> Aplicado {
        self.executar(cat, chave, bu, ag, Sinal::Somar, Some(anulada))
    }

    /// Retira um BU já aplicado: os mesmos contadores do [`Estado::aplicar_secao`], com sinal negativo (votos,
    /// comparecimento, `aptos`, `secoes`, `bus`, votáveis desconhecidos e cargos inválidos). `anulada` tem de ser a
    /// mesma da aplicação. Aplicar e retirar o mesmo BU volta o estado exatamente ao anterior.
    pub fn retirar(&self, cat: &Catalogo, chave: u64, bu: &[u8], anulada: bool, ag: &mut Agregador) -> Aplicado {
        self.executar(cat, chave, bu, ag, Sinal::Retirar, Some(anulada))
    }

    /// Troca o BU de uma seção: retira `antigo` (com a anulação com que foi aplicado) e aplica `novo`. Serve ao
    /// reenvio com conteúdo diferente e à anulação depois de totalizada (o mesmo BU, de `false` para `true`).
    pub fn substituir(&self, cat: &Catalogo, chave: u64, antigo: (&[u8], bool), novo: (&[u8], bool), ag: &mut Agregador) -> Aplicado {
        match self.retirar(cat, chave, antigo.0, antigo.1, ag) {
            Aplicado::Ok => self.aplicar_secao(cat, chave, novo.0, novo.1, ag),
            outro => outro,
        }
    }

    /// Se a seção (`chave`) tem BU aplicado.
    pub fn aplicada(&self, cat: &Catalogo, chave: u64) -> bool {
        cat.secoes.get(&chave).is_some_and(|&ord| self.vistos[ord as usize / 64].load(Relaxed) & (1u64 << (ord % 64)) != 0)
    }

    fn executar(&self, cat: &Catalogo, chave: u64, bu: &[u8], ag: &mut Agregador, sinal: Sinal, anulada: Option<bool>) -> Aplicado {
        let Some(&ord) = cat.secoes.get(&chave) else {
            if sinal == Sinal::Somar { self.fora_catalogo.fetch_add(1, Relaxed); }
            return Aplicado::ForaDoCadastro;
        };
        let bit = 1u64 << (ord % 64);
        let w = &self.vistos[ord as usize / 64];
        match sinal {
            Sinal::Somar => if w.fetch_or(bit, Relaxed) & bit != 0 { self.duplicados.fetch_add(1, Relaxed); return Aplicado::Duplicado; },
            Sinal::Retirar => if w.load(Relaxed) & bit == 0 { return Aplicado::Ausente; },
        }
        ag.chave_esperada = chave;
        ag.sinal = sinal;
        ag.anulada_forcada = anulada;
        let r = ler_bu(bu, ag);
        ag.sinal = Sinal::Somar;
        ag.anulada_forcada = None;
        match r {
            Ok(()) => {
                let m = cat.muni((chave >> 32) as u32);
                match sinal {
                    Sinal::Somar => {
                        self.bus.fetch_add(1, Relaxed);
                        if let Some(m) = m { self.secoes[m as usize].fetch_add(1, Relaxed); }
                    }
                    Sinal::Retirar => {
                        w.fetch_and(!bit, Relaxed);
                        self.bus.fetch_sub(1, Relaxed);
                        if let Some(m) = m { self.secoes[m as usize].fetch_sub(1, Relaxed); }
                    }
                }
                Aplicado::Ok
            }
            Err(_) => { self.erros.fetch_add(1, Relaxed); Aplicado::Erro }
        }
    }

    pub fn consolidar_uf(&self, cat: &Catalogo) -> Vec<Vec<u64>> {
        let nc = cat.nc();
        let mut uf: Vec<Vec<u64>> = (0..cat.ufs.len() * nc).map(|k| vec![0; cat.cand_cod[k].len() + EXTRA as usize]).collect();
        for m in 0..cat.muni_cod.len() {
            for c in 0..nc {
                let k = m * nc + c;
                if cat.base[k] == AUSENTE { continue; }
                self.somar_na_uf(cat, m, c, &mut uf[cat.muni_uf[m] as usize * nc + c]);
            }
        }
        uf
    }

    /// Soma a região (município `m`, cargo `ci`) no vetor da UF (layout da lista da UF + extras).
    /// Nos cargos municipais, cada candidato do município vai para a sua posição na lista da UF.
    pub fn somar_na_uf(&self, cat: &Catalogo, m: usize, ci: usize, v: &mut [u64]) {
        let nc = cat.nc();
        let k = m * nc + ci;
        if cat.base[k] == AUSENTE { return; }
        let b = cat.base[k] as usize;
        if cat.municipal[cat.cargos[ci] as usize] {
            let (n, nu) = (cat.ncand[k] as usize, cat.cand_cod[cat.muni_uf[m] as usize * nc + ci].len());
            for (j, &iu) in cat.mcand_uf[k].iter().enumerate() { v[iu as usize] += self.tally[b + j].load(Relaxed) as u64; }
            for x in 0..EXTRA as usize { v[nu + x] += self.tally[b + n + x].load(Relaxed) as u64; }
        } else {
            for j in 0..(cat.ncand[k] + EXTRA) as usize { v[j] += self.tally[b + j].load(Relaxed) as u64; }
        }
    }

    /// Soma de verificação independente da ordem de chegada dos BUs.
    pub fn checksum(&self) -> u64 {
        self.tally.iter().enumerate().fold(0u64, |a, (i, x)| a.wrapping_add((i as u64 + 1).wrapping_mul(x.load(Relaxed) as u64)))
    }
}

/// Agregador de uma thread: recebe os votos do leitor e incrementa o vetor denso.
pub struct Agregador<'a> {
    cat: &'a Catalogo,
    est: &'a Estado,
    pub chave_esperada: u64,
    muni: u32,
    muni_cod: u32,
    uf: usize,
    cargo_atual: u8,
    base: u32,
    n: u32,
    tab: &'a [u32],
    /// cargo municipal: lista ordenada dos candidatos do município (busca binária) em vez de `tab`
    lista: &'a [u32],
    municipal: bool,
    /// seção anulada: todos os votos vão para os nulos
    anulada: bool,
    /// eleitores aptos da eleição em leitura (`qtdEleitoresAptos`)
    aptos_eleicao: u32,
    comp: u32,
    vag: u32,
    soma: u32,
    ativo: bool,
    /// somar ou retirar (ver [`Sinal`]); `Estado::retirar` liga e desliga
    sinal: Sinal,
    /// anulação da seção dada por quem aplica (`Estado::aplicar_secao`/`retirar`); `None` = `cat.anuladas`
    anulada_forcada: Option<bool>,
}

impl<'a> Agregador<'a> {
    pub fn novo(cat: &'a Catalogo, est: &'a Estado) -> Self {
        Agregador {
            cat, est, chave_esperada: 0, muni: AUSENTE, muni_cod: 0, uf: 0, cargo_atual: 0,
            base: 0, n: 0, tab: &[], lista: &[], municipal: false, anulada: false,
            aptos_eleicao: 0, comp: 0, vag: 1, soma: 0, ativo: false, sinal: Sinal::Somar, anulada_forcada: None,
        }
    }

    /// Soma (ou retira) `q` do contador `i` do vetor denso.
    #[inline(always)]
    fn add(&self, v: &AtomicU32, q: u32) {
        match self.sinal { Sinal::Somar => { v.fetch_add(q, Relaxed); } Sinal::Retirar => { v.fetch_sub(q, Relaxed); } }
    }

    /// Conta (ou desconta) uma ocorrência num contador de conferência.
    #[inline]
    fn contar(&self, c: &AtomicU64) {
        match self.sinal { Sinal::Somar => { c.fetch_add(1, Relaxed); } Sinal::Retirar => { c.fetch_sub(1, Relaxed); } }
    }

    #[cold]
    fn registrar_vnt(&self, codigo: u32, qtd: u32) {
        self.contar(&self.est.desconhecidos);
        let mut m = self.est.alertas_vnt.lock().unwrap();
        let chave = (self.cargo_atual, self.muni_cod, codigo);
        match self.sinal {
            Sinal::Somar => *m.entry(chave).or_default() += qtd as u64,
            Sinal::Retirar => {
                if let Some(x) = m.get_mut(&chave) {
                    *x = x.saturating_sub(qtd as u64);
                    if *x == 0 { m.remove(&chave); }
                }
            }
        }
    }
}

impl Sink for Agregador<'_> {
    fn secao(&mut self, _pl: u32, mu: u32, zn: u16, se: u16) {
        let k = crate::chave(mu, zn, se);
        if k != self.chave_esperada { self.contar(&self.est.divergentes); }
        self.anulada = match self.anulada_forcada { Some(a) => a, None => !self.cat.anuladas.is_empty() && self.cat.anuladas.contains(&k) };
        self.muni_cod = mu;
        match self.cat.muni(mu) {
            Some(m) => { self.muni = m; self.uf = self.cat.muni_uf[m as usize] as usize; }
            None => self.muni = AUSENTE,
        }
    }
    fn eleicao(&mut self, _id: u32, aptos: u32) { self.aptos_eleicao = aptos; }
    fn cargo(&mut self, cargo: u8, comp: u32) {
        self.ativo = false;
        if self.cat.ignorado[cargo as usize] { return; }
        if self.muni == AUSENTE { self.contar(&self.est.desconhecidos); return; }
        let Some(ci) = self.cat.ci(cargo) else { self.contar(&self.est.desconhecidos); return; };
        let nc = self.cat.nc();
        let k = self.muni as usize * nc + ci;
        if self.cat.base[k] == AUSENTE { self.contar(&self.est.desconhecidos); return; }
        self.cargo_atual = cargo;
        self.base = self.cat.base[k];
        self.n = self.cat.ncand[k];
        self.municipal = self.cat.municipal[cargo as usize];
        if self.municipal { self.lista = &self.cat.mcand_cod[k]; } else { self.tab = &self.cat.cand_tab[self.uf * nc + ci]; }
        self.comp = comp;
        self.vag = self.cat.vagas[cargo as usize] as u32;
        self.soma = 0;
        self.ativo = true;
        self.add(&self.est.tally[(self.base + self.n + OFF_COMP) as usize], comp);
        self.add(&self.est.aptos[k], self.aptos_eleicao);
    }
    #[inline(always)]
    fn voto(&mut self, tipo: u8, qtd: u32, partido: u16, codigo: u32) {
        if !self.ativo { return; }
        let off = match tipo {
            1 | 2 | 4 | 5 if self.anulada => self.n + OFF_NULO,
            1 if self.municipal => match self.lista.binary_search(&codigo) {
                Ok(i) => i as u32,
                Err(_) => {
                    self.registrar_vnt(codigo, qtd);
                    self.add(&self.est.tally[(self.base + self.n + OFF_VNT) as usize], qtd);
                    self.soma += qtd;
                    return;
                }
            },
            1 => match self.tab.get(codigo as usize) {
                Some(&i) if i != AUSENTE => i,
                _ => {
                    self.registrar_vnt(codigo, qtd);
                    self.add(&self.est.tally[(self.base + self.n + OFF_VNT) as usize], qtd);
                    self.soma += qtd;
                    return;
                }
            },
            4 => self.n + (partido as u32 % N_LEGENDA),
            2 => self.n + OFF_BRANCO,
            3 => self.n + OFF_NULO,
            5 => self.n + OFF_SEMCAND,
            _ => { self.contar(&self.est.desconhecidos); return; }
        };
        self.add(&self.est.tally[(self.base + off) as usize], qtd);
        self.soma += qtd;
    }
    fn fim_cargo(&mut self) {
        if self.ativo && self.soma != self.comp * self.vag { self.contar(&self.est.cargos_invalidos); }
        self.ativo = false;
    }
}

/// Totaliza em lote os BUs já em memória com `threads` threads (blocos de 64 BUs, como no benchmark sintético).
pub fn totalizar_lote(cat: &Catalogo, mem: &EmMemoria, threads: usize) -> (Estado, f64) {
    let est = Estado::novo(cat);
    let next = AtomicUsize::new(0);
    let n = mem.itens.len();
    let t0 = std::time::Instant::now();
    std::thread::scope(|sc| {
        for _ in 0..threads {
            sc.spawn(|| {
                let mut ag = Agregador::novo(cat, &est);
                loop {
                    let s = next.fetch_add(64, Relaxed);
                    if s >= n { break; }
                    for i in s..(s + 64).min(n) {
                        est.aplicar(cat, mem.itens[i].0.chave(), mem.bu(i), &mut ag);
                    }
                }
            });
        }
    });
    (est, t0.elapsed().as_secs_f64())
}

/// Totais por UF e cargo em CSV: uf,cargo,tipo,codigo,votos (tipo: nominal|legenda|branco|nulo|comparecimento|sem_candidato).
pub fn csv_totais_uf(cat: &Catalogo, uf_tot: &[Vec<u64>]) -> String {
    let mut s = String::from("uf,cargo,tipo,codigo,votos\n");
    let nc = cat.nc();
    for u in 0..cat.ufs.len() {
        let ufs = std::str::from_utf8(&cat.ufs[u]).unwrap();
        for (ci, &c) in cat.cargos.iter().enumerate() {
            let k = u * nc + ci;
            let v = &uf_tot[k];
            let n = cat.cand_cod[k].len();
            if v.iter().all(|&x| x == 0) { continue; }
            for (i, &cod) in cat.cand_cod[k].iter().enumerate() { let _ = writeln!(s, "{ufs},{c},nominal,{cod},{}", v[i]); }
            for p in 0..N_LEGENDA as usize { if v[n + p] > 0 { let _ = writeln!(s, "{ufs},{c},legenda,{p},{}", v[n + p]); } }
            for (nm, off) in [("branco", OFF_BRANCO), ("nulo", OFF_NULO), ("comparecimento", OFF_COMP), ("sem_candidato", OFF_SEMCAND)] {
                let _ = writeln!(s, "{ufs},{c},{nm},,{}", v[n + off as usize]);
            }
        }
    }
    s
}

/// Totais por município e cargo em CSV: uf,municipio,cargo,tipo,codigo,votos (mesmos tipos de
/// `csv_totais_uf`). `cargos`: só estes; padrão: os cargos municipais. Candidatos sem votos são omitidos;
/// os totais (branco, nulo, comparecimento, sem_candidato) saem sempre que a região existe.
pub fn csv_totais_municipio(cat: &Catalogo, est: &Estado, cargos: Option<&[u8]>) -> String {
    let mut s = String::from("uf,municipio,cargo,tipo,codigo,votos\n");
    let nc = cat.nc();
    for m in 0..cat.muni_cod.len() {
        let ufs = std::str::from_utf8(&cat.ufs[cat.muni_uf[m] as usize]).unwrap();
        let mu = cat.muni_cod[m];
        for (ci, &c) in cat.cargos.iter().enumerate() {
            if !cargos.map_or(cat.municipal[c as usize], |so| so.contains(&c)) { continue; }
            let k = m * nc + ci;
            if cat.base[k] == AUSENTE { continue; }
            let (b, n) = (cat.base[k] as usize, cat.ncand[k] as usize);
            let t = |j: usize| est.tally[b + j].load(Relaxed);
            for (i, &cod) in cat.candidatos(m, ci).iter().enumerate() { if t(i) > 0 { let _ = writeln!(s, "{ufs},{mu},{c},nominal,{cod},{}", t(i)); } }
            for p in 0..N_LEGENDA as usize { if t(n + p) > 0 { let _ = writeln!(s, "{ufs},{mu},{c},legenda,{p},{}", t(n + p)); } }
            for (nm, off) in [("branco", OFF_BRANCO), ("nulo", OFF_NULO), ("comparecimento", OFF_COMP), ("sem_candidato", OFF_SEMCAND)] {
                let _ = writeln!(s, "{ufs},{mu},{c},{nm},,{}", t(n + off as usize));
            }
        }
    }
    s
}

/// Resumo legível: para cada UF e cargo, os 3 mais votados, brancos, nulos e comparecimento.
pub fn resumo(cat: &Catalogo, uf_tot: &[Vec<u64>], max_ufs: usize) -> String {
    let mut s = String::new();
    let nc = cat.nc();
    let com_votos = (0..cat.ufs.len()).filter(|&u| (0..nc).any(|ci| uf_tot[u * nc + ci].iter().any(|&x| x > 0)));
    for u in com_votos.take(max_ufs) {
        let _ = writeln!(s, "  {}:", std::str::from_utf8(&cat.ufs[u]).unwrap().to_uppercase());
        for (ci, &c) in cat.cargos.iter().enumerate() {
            let k = u * nc + ci;
            let v = &uf_tot[k];
            let n = cat.cand_cod[k].len();
            if v.iter().all(|&x| x == 0) { continue; }
            let mut top: Vec<(u64, u32)> = (0..n).map(|i| (v[i], cat.cand_cod[k][i])).collect();
            top.sort_by(|a, b| b.cmp(a));
            let tops: Vec<String> = top.iter().take(3).map(|(q, c)| format!("{c}={q}")).collect();
            let _ = writeln!(s, "    {:<15} comp={:<9} brancos={:<8} nulos={:<8} mais votados: {}", nome_cargo(c),
                v[n + OFF_COMP as usize], v[n + OFF_BRANCO as usize], v[n + OFF_NULO as usize], tops.join("  "));
        }
    }
    s
}

#[cfg(test)]
mod testes;
