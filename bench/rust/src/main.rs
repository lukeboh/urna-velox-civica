//! urnabench — benchmark de totalização do Urna Velox.
//!
//! Subcomandos:
//!   gen   --kind geral|municipal --n <BUs distintos> [--nosig] --out <dir>
//!         Gera um corpus sintético de BUs em BER (estrutura inspirada no bu.asn1 do TSE)
//!         e o layout denso de contadores (layout_<kind>.csv).
//!   run   --dir <dir> --kind geral|municipal --variant <v> --threads <t> --bus <n>
//!         Totaliza <n> BUs (o corpus é percorrido ciclicamente; cada iteração é uma seção distinta).
//!         variantes: manual-dense | manual-dashmap | rasn-dense | manual-dense-sha512
//!
//! Saída de `run`: uma linha CSV
//!   lang,variant,kind,threads,bus,seconds,bus_per_s,consolidate_ms,peak_rss_mb,corpus_mb,invalid
use dashmap::DashMap;
use rand::{rngs::StdRng, Rng, SeedableRng};
use rasn::prelude::*;
use std::sync::atomic::{AtomicU32, AtomicU64, AtomicUsize, Ordering::Relaxed};
use std::time::Instant;

// ---------------------------------------------------------------- esquema ASN.1 (sintético)
#[derive(AsnType, Encode, Decode, Debug, Clone)]
#[rasn(automatic_tags)]
pub struct Cabecalho { pub data_geracao: OctetString, pub id_enc: u32 }

#[derive(AsnType, Encode, Decode, Debug, Clone)]
#[rasn(automatic_tags)]
pub struct IdSecao { pub municipio: u32, pub zona: u16, pub local: u16, pub secao: u16 }

#[derive(AsnType, Encode, Decode, Debug, Clone)]
#[rasn(automatic_tags)]
pub struct Envelope { pub cabecalho: Cabecalho, pub fase: u8, pub tipo_envelope: u8, pub identificacao: IdSecao, pub conteudo: OctetString }

#[derive(AsnType, Encode, Decode, Debug, Clone)]
#[rasn(automatic_tags)]
pub struct Urna { pub tipo: u8, pub versao: OctetString, pub numero_serie: u32, pub codigo_carga: OctetString }

#[derive(AsnType, Encode, Decode, Debug, Clone)]
#[rasn(automatic_tags)]
pub struct IdVotavel { pub partido: u16, pub codigo: u32 }

#[derive(AsnType, Encode, Decode, Debug, Clone)]
#[rasn(automatic_tags)]
pub struct VotoVotavel { pub tipo_voto: u8, pub quantidade: u32, pub identificacao: Option<IdVotavel>, pub assinatura: OctetString }

#[derive(AsnType, Encode, Decode, Debug, Clone)]
#[rasn(automatic_tags)]
pub struct TotalCargo { pub codigo_cargo: u8, pub ordem_impressao: u8, pub votos: Vec<VotoVotavel> }

#[derive(AsnType, Encode, Decode, Debug, Clone)]
#[rasn(automatic_tags)]
pub struct ResultadoVotacao { pub tipo_cargo: u8, pub qtd_comparecimento: u32, pub totais: Vec<TotalCargo> }

#[derive(AsnType, Encode, Decode, Debug, Clone)]
#[rasn(automatic_tags)]
pub struct ResultadoEleicao { pub id_eleicao: u32, pub qtd_aptos: u32, pub resultados: Vec<ResultadoVotacao> }

#[derive(AsnType, Encode, Decode, Debug, Clone)]
#[rasn(automatic_tags)]
pub struct BoletimUrna {
    pub cabecalho: Cabecalho, pub fase: u8, pub urna: Urna, pub identificacao_secao: IdSecao,
    pub data_hora_emissao: OctetString, pub qtd_lib_codigo: u32, pub qtd_comp_biometrico: u32,
    pub resultados_por_eleicao: Vec<ResultadoEleicao>, pub chave_assinatura: OctetString,
}

// ---------------------------------------------------------------- mundo sintético
// (UF, municípios, cadeiras de deputado federal)
const UFS: [(&str, u32, u32); 27] = [
    ("AC", 22, 8), ("AL", 102, 9), ("AP", 16, 8), ("AM", 62, 8), ("BA", 417, 39), ("CE", 184, 22),
    ("DF", 1, 8), ("ES", 78, 10), ("GO", 246, 17), ("MA", 217, 18), ("MT", 142, 8), ("MS", 79, 8),
    ("MG", 853, 53), ("PA", 144, 17), ("PB", 223, 12), ("PR", 399, 30), ("PE", 185, 25), ("PI", 224, 10),
    ("RJ", 92, 46), ("RN", 167, 8), ("RS", 497, 31), ("RO", 52, 8), ("RR", 15, 8), ("SC", 295, 16),
    ("SP", 645, 70), ("SE", 75, 8), ("TO", 139, 8),
];
const N_PARTIDOS: u32 = 31; // códigos de partido 10..=40
const SECOES_TOTAL: f64 = 500_000.0;

struct Muni { uf: usize, secoes: u32, cands: Vec<u32> } // cands por índice de cargo

fn code_base(kind: &str, ci: usize) -> u32 {
    if kind == "geral" { [10, 10, 100, 1000, 10000][ci] } else { [10, 10000][ci] }
}
fn cargo_codes(kind: &str, uf: &str) -> Vec<u8> {
    if kind == "geral" { vec![1, 3, 5, 6, if uf == "DF" { 8 } else { 7 }] } else { vec![11, 13] }
}
fn cargo_index(code: u8) -> usize { match code { 1 | 11 => 0, 3 | 13 => 1, 5 => 2, 6 => 3, 7 | 8 => 4, _ => 99 } }
/// vagas por cargo no cenário (2026: renovação de 2/3 do Senado → 2 votos por eleitor)
fn vagas(code: u8) -> u32 { if code == 5 { 2 } else { 1 } }

fn world(kind: &str) -> Vec<Muni> {
    let mut r = StdRng::seed_from_u64(2026);
    let mut ms = vec![];
    for (ui, &(uf, nm, fed)) in UFS.iter().enumerate() {
        let est = if uf == "DF" { 24 } else if fed <= 12 { 3 * fed } else { 36 + fed - 12 };
        let fed_c = (20.7 * fed as f64).round() as u32;
        let est_c = (15.3 * est as f64).round() as u32;
        for _ in 0..nm {
            let u: f64 = r.gen_range(0.0005..1.0);
            let w = u.powf(-1.0 / 1.1);
            ms.push(Muni { uf: ui, secoes: 0, cands: vec![], }); let m = ms.last_mut().unwrap();
            m.secoes = (w * 1000.0) as u32; // peso provisório
            m.cands = if kind == "geral" { vec![11, 10, 12, fed_c, est_c] } else { vec![r.gen_range(2..=5), 0] };
        }
    }
    let tot: f64 = ms.iter().map(|m| m.secoes as f64).sum();
    let maxs = ms.iter().map(|m| m.secoes).max().unwrap() as f64;
    for m in ms.iter_mut() {
        let rel = m.secoes as f64 / maxs;
        m.secoes = ((m.secoes as f64 / tot) * SECOES_TOTAL).round().max(3.0) as u32;
        if kind != "geral" { m.cands[1] = (40.0 + 960.0 * rel.sqrt()).round() as u32; }
    }
    ms
}

fn region_size(cands: u32) -> u32 { cands + N_PARTIDOS + 3 } // nominais, legendas, branco, nulo, comparecimento

fn gen(kind: &str, n: usize, sig: bool, out: &str) {
    std::fs::create_dir_all(out).unwrap();
    let ms = world(kind);
    // layout denso
    let mut layout = String::new();
    let mut base = 0u64;
    let nc = ms[0].cands.len();
    let mut rows = vec![];
    for (i, m) in ms.iter().enumerate() {
        let mut row = format!("{},{}", i, m.uf);
        for c in 0..nc { row += &format!(",{},{}", base, m.cands[c]); base += region_size(m.cands[c]) as u64; }
        rows.push(row);
    }
    layout += &format!("{},{},{},{}\n", kind, ms.len(), nc, base);
    for r in rows { layout += &r; layout += "\n"; }
    std::fs::write(format!("{out}/layout_{kind}.csv"), layout).unwrap();
    let secs: u64 = ms.iter().map(|m| m.secoes as u64).sum();
    let cands_tot: u64 = if kind == "geral" {
        // candidatos distintos (federal + estadual) por UF + majoritários
        UFS.iter().enumerate().map(|(ui, _)| { let m = ms.iter().find(|m| m.uf == ui).unwrap(); (m.cands[3] + m.cands[4] + 10 + 12) as u64 }).sum::<u64>() + 11
    } else { ms.iter().map(|m| (m.cands[0] + m.cands[1]) as u64).sum() };
    eprintln!("[gen] {kind}: municipios={} secoes={} candidatos~{} slots_densos={} ({:.1} MB em u32)", ms.len(), secs, cands_tot, base, base as f64 * 4.0 / 1048576.0);

    // amostragem de município proporcional ao nº de seções
    let cum: Vec<u64> = ms.iter().scan(0u64, |a, m| { *a += m.secoes as u64; Some(*a) }).collect();
    let mut r = StdRng::seed_from_u64(42);
    let mut blob: Vec<u8> = Vec::with_capacity(n * 8000);
    blob.extend_from_slice(&(n as u32).to_le_bytes());
    let mut tot_bytes = 0usize; let mut tot_vot = 0usize;
    for i in 0..n {
        let x = r.gen_range(0..*cum.last().unwrap());
        let mi = cum.partition_point(|&c| c <= x);
        let m = &ms[mi];
        let uf = UFS[m.uf].0;
        let aptos: u32 = r.gen_range(200..420);
        let comp: u32 = (aptos as f64 * r.gen_range(0.72..0.86)) as u32;
        let codes = cargo_codes(kind, uf);
        let mut eleicoes: Vec<(u32, Vec<u8>)> = if kind == "geral" { vec![(1001, vec![codes[0], codes[3]]), (1002, vec![codes[1], codes[2], codes[4]])] } else { vec![(2001, codes.clone())] };
        let mut res_el = vec![];
        for (id_el, cargos) in eleicoes.drain(..) {
            let mut totais = vec![];
            for (ord, &cc) in cargos.iter().enumerate() {
                let ci = cargo_index(cc);
                let ncand = m.cands[ci];
                let prop = cc == 6 || cc == 7 || cc == 8 || cc == 13;
                let votos_tot = comp * vagas(cc);
                let branco = (votos_tot as f64 * r.gen_range(0.01..0.05)) as u32;
                let nulo = (votos_tot as f64 * r.gen_range(0.02..0.06)) as u32;
                let validos = votos_tot - branco - nulo;
                // votáveis presentes nesta seção
                let mut ids: Vec<(u8, u16, u32)> = vec![]; // (tipo, partido, codigo)
                let k_nom = if prop { let lo = if kind == "geral" { 80 } else { 40 }; let hi = if kind == "geral" { 170 } else { 150 }; (r.gen_range(lo..hi) as u32).min(ncand) } else { ncand };
                let mut chosen = std::collections::BTreeSet::new();
                while (chosen.len() as u32) < k_nom { chosen.insert(r.gen_range(0..ncand)); }
                for k in chosen { ids.push((1, (10 + k % N_PARTIDOS) as u16, code_base(kind, ci) + k)); }
                if prop { let nl = if kind == "geral" { 25 } else { 20 }; for p in 0..nl { ids.push((4, (10 + p) as u16, 0)); } }
                let mut q = vec![0u32; ids.len()];
                let take = (validos as usize).min(ids.len());
                ids.truncate(take); q.truncate(take);
                for j in 0..take { q[j] = 1; }
                for _ in take as u32..validos { let j = r.gen_range(0..take); q[j] += 1; }
                let mut votos: Vec<VotoVotavel> = ids.iter().zip(q.iter()).map(|(&(t, p, c), &qq)| VotoVotavel {
                    tipo_voto: t, quantidade: qq,
                    identificacao: Some(IdVotavel { partido: p, codigo: if t == 1 { c } else { p as u32 } }),
                    assinatura: if sig { (0..32).map(|_| r.gen::<u8>()).collect::<Vec<u8>>().into() } else { OctetString::from(vec![]) },
                }).collect();
                votos.push(VotoVotavel { tipo_voto: 2, quantidade: branco, identificacao: None, assinatura: if sig { vec![7u8; 32].into() } else { OctetString::from(vec![]) } });
                votos.push(VotoVotavel { tipo_voto: 3, quantidade: nulo, identificacao: None, assinatura: if sig { vec![9u8; 32].into() } else { OctetString::from(vec![]) } });
                tot_vot += votos.len();
                totais.push(ResultadoVotacao { tipo_cargo: if prop { 2 } else { 1 }, qtd_comparecimento: comp,
                    totais: vec![TotalCargo { codigo_cargo: cc, ordem_impressao: ord as u8, votos }] });
            }
            res_el.push(ResultadoEleicao { id_eleicao: id_el, qtd_aptos: aptos, resultados: totais });
        }
        let ids = IdSecao { municipio: mi as u32, zona: (mi % 400) as u16 + 1, local: 1000 + (i % 300) as u16, secao: (i % 900) as u16 + 1 };
        let bu = BoletimUrna {
            cabecalho: Cabecalho { data_geracao: b"20261004170512".to_vec().into(), id_enc: i as u32 },
            fase: 3,
            urna: Urna { tipo: 1, versao: b"10.2.0.1 - sintetica".to_vec().into(), numero_serie: 2_000_000 + i as u32, codigo_carga: vec![0x5a; 24].into() },
            identificacao_secao: ids.clone(), data_hora_emissao: b"20261004170500".to_vec().into(),
            qtd_lib_codigo: r.gen_range(0..5), qtd_comp_biometrico: comp - 3,
            resultados_por_eleicao: res_el, chave_assinatura: vec![0xab; 32].into(),
        };
        let inner = rasn::ber::encode(&bu).unwrap();
        let env = Envelope { cabecalho: Cabecalho { data_geracao: b"20261004170512".to_vec().into(), id_enc: i as u32 }, fase: 3, tipo_envelope: 1, identificacao: ids, conteudo: inner.into() };
        let bytes = rasn::ber::encode(&env).unwrap();
        tot_bytes += bytes.len();
        blob.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
        blob.extend_from_slice(&bytes);
    }
    let fname = format!("{out}/corpus_{kind}{}.bin", if sig { "" } else { "_nosig" });
    std::fs::write(&fname, &blob).unwrap();
    eprintln!("[gen] {fname}: {n} BUs, {:.1} KB/BU médio, {:.0} votáveis/BU, total {:.1} MB", tot_bytes as f64 / n as f64 / 1024.0, tot_vot as f64 / n as f64, tot_bytes as f64 / 1048576.0);
}

// ---------------------------------------------------------------- leitura do corpus e layout
struct Layout { kind: String, nc: usize, slots: usize, base: Vec<u32>, cnt: Vec<u32>, muni_uf: Vec<u8> }
fn load_layout(dir: &str, kind: &str) -> Layout {
    let s = std::fs::read_to_string(format!("{dir}/layout_{kind}.csv")).unwrap();
    let mut it = s.lines();
    let h: Vec<&str> = it.next().unwrap().split(',').collect();
    let nm: usize = h[1].parse().unwrap(); let nc: usize = h[2].parse().unwrap(); let slots: usize = h[3].parse().unwrap();
    let (mut base, mut cnt, mut muni_uf) = (vec![0; nm * nc], vec![0; nm * nc], vec![0u8; nm]);
    for (i, l) in it.enumerate() {
        let v: Vec<u64> = l.split(',').map(|x| x.parse().unwrap()).collect();
        muni_uf[i] = v[1] as u8;
        for c in 0..nc { base[i * nc + c] = v[2 + 2 * c] as u32; cnt[i * nc + c] = v[3 + 2 * c] as u32; }
    }
    Layout { kind: kind.into(), nc, slots, base, cnt, muni_uf }
}
fn load_corpus(path: &str) -> (Vec<u8>, Vec<(usize, usize)>) {
    let b = std::fs::read(path).unwrap();
    let n = u32::from_le_bytes(b[0..4].try_into().unwrap()) as usize;
    let mut idx = Vec::with_capacity(n); let mut p = 4;
    for _ in 0..n { let l = u32::from_le_bytes(b[p..p + 4].try_into().unwrap()) as usize; p += 4; idx.push((p, p + l)); p += l; }
    (b, idx)
}

// ---------------------------------------------------------------- parser BER manual (zero-copy)
#[inline(always)]
fn tlv(b: &[u8], p: usize) -> (u8, usize, usize) {
    let tag = b[p];
    let l0 = b[p + 1];
    if l0 < 0x80 { (tag, p + 2, p + 2 + l0 as usize) } else {
        let n = (l0 & 0x7f) as usize; let mut l = 0usize;
        for k in 0..n { l = (l << 8) | b[p + 2 + k] as usize; }
        (tag, p + 2 + n, p + 2 + n + l)
    }
}
#[inline(always)]
fn uint(b: &[u8], s: usize, e: usize) -> u32 { let mut v = 0u32; for k in s..e { v = (v << 8) | b[k] as u32; } v }

/// Votos extraídos de um BU, entregues ao agregador sem alocação.
trait Sink { fn secao(&mut self, muni: u32, zona: u16, secao: u16); fn cargo(&mut self, code: u8, comp: u32); fn voto(&mut self, tipo: u8, qtd: u32, partido: u16, codigo: u32); fn fim_cargo(&mut self); }

fn parse_manual<S: Sink>(b: &[u8], sink: &mut S) {
    // Envelope
    let (_, s, e) = tlv(b, 0);
    let mut p = s; let (mut cs, mut ce) = (0, 0);
    while p < e { let (t, vs, ve) = tlv(b, p); if t == 0x84 { cs = vs; ce = ve; } p = ve; }
    let bu = &b[cs..ce];
    let (_, s, e) = tlv(bu, 0);
    let mut p = s;
    while p < e {
        let (t, vs, ve) = tlv(bu, p);
        match t {
            0xA3 => { // identificacao_secao
                let (mut mu, mut zo, mut se) = (0, 0, 0); let mut q = vs;
                while q < ve { let (t2, a, z) = tlv(bu, q); match t2 { 0x80 => mu = uint(bu, a, z), 0x81 => zo = uint(bu, a, z), 0x83 => se = uint(bu, a, z), _ => {} } q = z; }
                sink.secao(mu, zo as u16, se as u16);
            }
            0xA7 => { // resultados_por_eleicao: SEQUENCE OF ResultadoEleicao
                let mut q = vs;
                while q < ve { let (_, a, z) = tlv(bu, q); // ResultadoEleicao
                    let mut r = a;
                    while r < z { let (t3, a3, z3) = tlv(bu, r);
                        if t3 == 0xA2 { let mut rv = a3; // SEQUENCE OF ResultadoVotacao
                            while rv < z3 { let (_, a4, z4) = tlv(bu, rv); let mut comp = 0; let mut x = a4;
                                while x < z4 { let (t5, a5, z5) = tlv(bu, x);
                                    match t5 { 0x81 => comp = uint(bu, a5, z5), 0xA2 => { let mut tc = a5; // SEQUENCE OF TotalCargo
                                        while tc < z5 { let (_, a6, z6) = tlv(bu, tc); let mut y = a6;
                                            while y < z6 { let (t7, a7, z7) = tlv(bu, y);
                                                match t7 { 0x80 => sink.cargo(uint(bu, a7, z7) as u8, comp), 0xA2 => { let mut vv = a7;
                                                    while vv < z7 { let (_, a8, z8) = tlv(bu, vv); let (mut tp, mut qt, mut pa, mut co) = (0u8, 0u32, 0u16, 0u32); let mut w = a8;
                                                        while w < z8 { let (t9, a9, z9) = tlv(bu, w);
                                                            match t9 { 0x80 => tp = uint(bu, a9, z9) as u8, 0x81 => qt = uint(bu, a9, z9), 0xA2 => { let mut iv = a9;
                                                                while iv < z9 { let (ta, aa, za) = tlv(bu, iv); if ta == 0x80 { pa = uint(bu, aa, za) as u16 } else if ta == 0x81 { co = uint(bu, aa, za) } iv = za; } } _ => {} }
                                                            w = z9; }
                                                        sink.voto(tp, qt, pa, co); vv = z8; } sink.fim_cargo(); } _ => {} }
                                                y = z7; }
                                            tc = z6; } } _ => {} }
                                    x = z5; }
                                rv = z4; } }
                        r = z3; }
                    q = z; }
            }
            _ => {}
        }
        p = ve;
    }
}

fn parse_rasn<S: Sink>(b: &[u8], sink: &mut S) {
    let env: Envelope = rasn::ber::decode(b).unwrap();
    let bu: BoletimUrna = rasn::ber::decode(&env.conteudo).unwrap();
    let s = &bu.identificacao_secao; sink.secao(s.municipio, s.zona, s.secao);
    for el in &bu.resultados_por_eleicao { for rv in &el.resultados { for tc in &rv.totais {
        sink.cargo(tc.codigo_cargo, rv.qtd_comparecimento);
        for v in &tc.votos { let (p, c) = v.identificacao.as_ref().map(|i| (i.partido, i.codigo)).unwrap_or((0, 0)); sink.voto(v.tipo_voto, v.quantidade, p, c); }
        sink.fim_cargo();
    } } }
}

// ---------------------------------------------------------------- agregadores
struct Dense<'a> { lay: &'a Layout, tally: &'a [AtomicU32], muni: u32, base: u32, cnt: u32, cbase: u32, comp: u32, vag: u32, soma: u32, invalid: u64 }
impl<'a> Sink for Dense<'a> {
    fn secao(&mut self, muni: u32, _: u16, _: u16) { self.muni = muni; }
    fn cargo(&mut self, code: u8, comp: u32) {
        let ci = cargo_index(code); let k = self.muni as usize * self.lay.nc + ci;
        self.base = self.lay.base[k]; self.cnt = self.lay.cnt[k]; self.cbase = code_base(&self.lay.kind, ci);
        self.comp = comp; self.vag = vagas(code); self.soma = 0;
        self.tally[(self.base + self.cnt + N_PARTIDOS + 2) as usize].fetch_add(comp, Relaxed);
    }
    #[inline(always)]
    fn voto(&mut self, tipo: u8, qtd: u32, partido: u16, codigo: u32) {
        let off = match tipo { 1 => codigo - self.cbase, 4 => self.cnt + (partido as u32 - 10), 2 => self.cnt + N_PARTIDOS, _ => self.cnt + N_PARTIDOS + 1 };
        self.tally[(self.base + off) as usize].fetch_add(qtd, Relaxed);
        self.soma += qtd;
    }
    fn fim_cargo(&mut self) { if self.soma != self.comp * self.vag { self.invalid += 1; } }
}

#[derive(Default)]
struct TallyD { nominal: DashMap<u32, u64>, party: DashMap<u16, u64>, blank: AtomicU64, null: AtomicU64, comp: AtomicU64 }
#[derive(Default)]
struct StateD { national: DashMap<u8, TallyD>, state: DashMap<(u8, u8), TallyD>, muni: DashMap<(u32, u8), TallyD> }
/// Desenho do ARCHITECTURE.md original: DashMap aninhado em 3 níveis, propagação a cada voto.
struct Dash<'a> { st: &'a StateD, lay: &'a Layout, muni: u32, uf: u8, code: u8, buf: Vec<(u8, u32, u16, u32)>, comp: u32 }
impl<'a> Sink for Dash<'a> {
    fn secao(&mut self, muni: u32, _: u16, _: u16) { self.muni = muni; self.uf = self.lay.muni_uf[muni as usize]; }
    fn cargo(&mut self, code: u8, comp: u32) { self.code = code; self.comp = comp; self.buf.clear(); }
    fn voto(&mut self, tipo: u8, qtd: u32, partido: u16, codigo: u32) { self.buf.push((tipo, qtd, partido, codigo)); }
    fn fim_cargo(&mut self) {
        let apply = |t: &TallyD, buf: &[(u8, u32, u16, u32)], comp: u32| {
            t.comp.fetch_add(comp as u64, Relaxed);
            for &(tp, q, p, c) in buf { match tp {
                1 => *t.nominal.entry(c).or_insert(0) += q as u64, 4 => *t.party.entry(p).or_insert(0) += q as u64,
                2 => { t.blank.fetch_add(q as u64, Relaxed); } _ => { t.null.fetch_add(q as u64, Relaxed); } } }
        };
        apply(&self.st.muni.entry((self.muni, self.code)).or_default(), &self.buf, self.comp);
        apply(&self.st.state.entry((self.uf, self.code)).or_default(), &self.buf, self.comp);
        if self.code == 1 { apply(&self.st.national.entry(self.code).or_default(), &self.buf, self.comp); }
    }
}

fn peak_rss_mb() -> f64 {
    std::fs::read_to_string("/proc/self/status").ok().and_then(|s| s.lines().find(|l| l.starts_with("VmHWM")).map(|l| l.split_whitespace().nth(1).unwrap().parse::<f64>().unwrap() / 1024.0)).unwrap_or(-1.0)
}

fn run(dir: &str, kind: &str, variant: &str, threads: usize, bus: usize, corpus_file: Option<String>) {
    let lay = load_layout(dir, kind);
    let cf = corpus_file.unwrap_or(format!("{dir}/corpus_{kind}.bin"));
    let (blob, idx) = load_corpus(&cf);
    let corpus_mb = blob.len() as f64 / 1048576.0;
    let tally: Vec<AtomicU32> = (0..lay.slots).map(|_| AtomicU32::new(0)).collect();
    let dedup: Vec<AtomicU64> = (0..bus / 64 + 1).map(|_| AtomicU64::new(0)).collect();
    let st = StateD::default();
    let next = AtomicUsize::new(0);
    let invalid = AtomicU64::new(0);
    const CHUNK: usize = 64;
    let t0 = Instant::now();
    std::thread::scope(|sc| {
        for _ in 0..threads { sc.spawn(|| {
            let mut d = Dense { lay: &lay, tally: &tally, muni: 0, base: 0, cnt: 0, cbase: 0, comp: 0, vag: 1, soma: 0, invalid: 0 };
            let mut ds = Dash { st: &st, lay: &lay, muni: 0, uf: 0, code: 0, buf: Vec::with_capacity(256), comp: 0 };
            let mut h = [0u8; 64];
            loop {
                let s = next.fetch_add(CHUNK, Relaxed); if s >= bus { break; }
                for i in s..(s + CHUNK).min(bus) {
                    // deduplicação: 1 bit por seção (ordinal conhecido a priori)
                    let bit = 1u64 << (i % 64);
                    if dedup[i / 64].fetch_or(bit, Relaxed) & bit != 0 { continue; }
                    let (a, z) = idx[i % idx.len()]; let b = &blob[a..z];
                    match variant {
                        "manual-dense" => parse_manual(b, &mut d),
                        "manual-dense-sha512" => { use sha2::Digest; h.copy_from_slice(&sha2::Sha512::digest(b)); parse_manual(b, &mut d) }
                        "rasn-dense" => parse_rasn(b, &mut d),
                        "manual-dashmap" => parse_manual(b, &mut ds),
                        _ => panic!("variante desconhecida"),
                    }
                }
            }
            invalid.fetch_add(d.invalid, Relaxed);
            std::hint::black_box(h);
        }); }
    });
    let secs = t0.elapsed().as_secs_f64();
    // consolidação UF/Brasil a partir dos municípios (só para o modelo denso)
    let t1 = Instant::now();
    if variant != "manual-dashmap" {
        let mut uf_tot: Vec<Vec<u64>> = (0..27 * lay.nc).map(|_| vec![]).collect();
        for m in 0..lay.muni_uf.len() { for c in 0..lay.nc { let k = m * lay.nc + c; let (b, n) = (lay.base[k] as usize, region_size(lay.cnt[k]) as usize);
            let v = &mut uf_tot[lay.muni_uf[m] as usize * lay.nc + c]; if v.len() < n { v.resize(n, 0); }
            for j in 0..n { v[j] += tally[b + j].load(Relaxed) as u64; } } }
        let mut br = vec![0u64; 64];
        for u in 0..27 { for (j, x) in uf_tot[u * lay.nc].iter().enumerate() { if j < 64 { br[j] += x; } } }
        std::hint::black_box(&br);
    }
    let cons_ms = t1.elapsed().as_secs_f64() * 1000.0;
    if std::env::var("CHECK").is_ok() { let ck: u64 = tally.iter().enumerate().map(|(i, x)| (i as u64 % 1009 + 1) * x.load(Relaxed) as u64).sum(); eprintln!("checksum={ck}"); }
    println!("rust,{variant}{},{kind},{threads},{bus},{secs:.4},{:.0},{cons_ms:.1},{:.1},{corpus_mb:.1},{}",
        if cf.contains("nosig") { "-nosig" } else { "" }, bus as f64 / secs, peak_rss_mb(), invalid.load(Relaxed));
}

fn arg(a: &[String], k: &str) -> Option<String> { a.iter().position(|x| x == k).map(|i| a[i + 1].clone()) }
fn main() {
    let a: Vec<String> = std::env::args().collect();
    match a.get(1).map(|s| s.as_str()) {
        Some("gen") => gen(&arg(&a, "--kind").unwrap(), arg(&a, "--n").unwrap().parse().unwrap(), !a.contains(&"--nosig".to_string()), &arg(&a, "--out").unwrap()),
        Some("run") => run(&arg(&a, "--dir").unwrap(), &arg(&a, "--kind").unwrap(), &arg(&a, "--variant").unwrap(),
            arg(&a, "--threads").unwrap().parse().unwrap(), arg(&a, "--bus").unwrap().parse().unwrap(), arg(&a, "--corpus")),
        _ => eprintln!("uso: urnabench gen|run ..."),
    }
}
