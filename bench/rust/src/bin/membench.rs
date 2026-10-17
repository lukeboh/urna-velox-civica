//! membench — mede bytes de heap vivos (alocador contador) de modelos de estado do Urna Velox.
//! Uso: membench [n_tuplas]   (DUMP=wl.csv membench n  -> grava a carga usada por Mem.java e mem.mjs)
//! "0 shards" = DashMap::new(), isto é, o padrão da crate: (núcleos lógicos × 4) arredondado p/ potência de 2.
use dashmap::DashMap;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::alloc::{GlobalAlloc, Layout, System};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering::Relaxed};

struct Counting;
static LIVE: AtomicUsize = AtomicUsize::new(0);
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 { LIVE.fetch_add(l.size(), Relaxed); System.alloc(l) }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) { LIVE.fetch_sub(l.size(), Relaxed); System.dealloc(p, l) }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        LIVE.fetch_add(n, Relaxed); LIVE.fetch_sub(l.size(), Relaxed); System.realloc(p, l, n)
    }
}
#[global_allocator]
static A: Counting = Counting;
fn live() -> usize { LIVE.load(Relaxed) }
fn mb(b: usize) -> f64 { b as f64 / 1_048_576.0 }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct SectionId { election_id: u32, uf_id: u8, municipality: u32, zone: u16, section: u16 }

struct Tally {
    nominal: DashMap<u32, u64>,
    party: DashMap<u16, u64>,
    _b: AtomicU64, _n: AtomicU64, _e: AtomicU64, _v: AtomicU64,
}

/// (municipio, cargo, n_candidatos, n_legendas)
fn workload(target: usize, seed: u64) -> Vec<(u32, u8, usize, usize)> {
    let mut r = StdRng::seed_from_u64(seed);
    let munis = 5570u32;
    // tamanho relativo do municipio: cauda pesada (Pareto alfa ~1.1)
    let w: Vec<f64> = (0..munis).map(|_| { let u: f64 = r.gen_range(0.0001..1.0); u.powf(-1.0 / 1.1) }).collect();
    let mut out = vec![];
    let mut prop_raw = vec![];
    for m in 0..munis {
        for (c, k) in [(1u8, 11usize), (3, 10), (5, 12)] { out.push((m, c, k, 0)); }
        for c in [6u8, 7] { prop_raw.push((m, c, w[m as usize])); }
    }
    let fixed: usize = out.iter().map(|x| x.2).sum::<usize>() + munis as usize * 2 * 30;
    let remaining = target.saturating_sub(fixed) as f64;
    // calibra escala para somar ~remaining, com piso 40 e teto 2000 candidatos
    let (mut lo, mut hi) = (0.0f64, 1e6f64);
    for _ in 0..60 {
        let s = (lo + hi) / 2.0;
        let tot: f64 = prop_raw.iter().map(|x| (40.0 + s * x.2).min(2000.0)).sum();
        if tot > remaining { hi = s } else { lo = s }
    }
    for (m, c, wi) in prop_raw { out.push((m, c, (40.0 + lo * wi).min(2000.0) as usize, 30)); }
    out
}

fn main() {
    let target: usize = std::env::args().nth(1).map(|s| s.parse().unwrap()).unwrap_or(4_500_000);
    let wl = workload(target, 42);
    let n_tuples: usize = wl.iter().map(|x| x.2 + x.3).sum();
    let n_maps = wl.len();
    println!("tuplas (municipio,cargo,votavel) = {n_tuples}  | pares (municipio,cargo) = {n_maps}");
    if let Ok(p) = std::env::var("DUMP") {
        let s: String = wl.iter().map(|x| format!("{},{},{},{}\n", x.0, x.1, x.2, x.3)).collect();
        std::fs::write(p, s).unwrap(); return;
    }
    let mut r = StdRng::seed_from_u64(7);

    // 1) Desenho do ARCHITECTURE.md: DashMap aninhado
    for shards in [0usize, 8, 64, 128] {
        let base = live();
        let mk = |s: usize| if s == 0 { DashMap::new() } else { DashMap::with_shard_amount(s) };
        let st: DashMap<(u32, u8), Tally> = if shards == 0 { DashMap::new() } else { DashMap::with_shard_amount(shards) };
        for &(m, c, k, p) in &wl {
            let t = Tally {
                nominal: mk(shards), party: if shards == 0 { DashMap::new() } else { DashMap::with_shard_amount(shards) },
                _b: AtomicU64::new(0), _n: AtomicU64::new(0), _e: AtomicU64::new(0), _v: AtomicU64::new(0),
            };
            let mut i = 0; while t.nominal.len() < k { t.nominal.insert(r.gen_range(10..99999) + i * 0, 1); i += 1; }
            for j in 0..p { t.party.insert(j as u16 + 10, 1); }
            st.insert((m, c), t);
        }
        let sh = if shards == 0 { (std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1) * 4).next_power_of_two() } else { shards };
        println!("[A] DashMap aninhado, {sh:>3} shards/mapa{}: {:>8.1} MB", if shards == 0 { " (padrão)" } else { "         " }, mb(live() - base));
        // mesmo desenho, so o custo dos mapas vazios
        drop(st);
    }
    {
        let base = live();
        let v: Vec<Tally> = (0..n_maps).map(|_| Tally { nominal: DashMap::with_shard_amount(64), party: DashMap::with_shard_amount(64),
            _b: AtomicU64::new(0), _n: AtomicU64::new(0), _e: AtomicU64::new(0), _v: AtomicU64::new(0) }).collect();
        println!("    (so os {} TallyBreakdown vazios, 64 shards): {:>8.1} MB", n_maps, mb(live() - base));
        drop(v);
    }
    // 2) Um unico hash map plano: chave u64 (municipio|cargo|votavel) -> u32
    {
        let base = live();
        let mut h: hashbrown::HashMap<u64, u32> = hashbrown::HashMap::new();
        for &(m, c, k, p) in &wl {
            let mut n = 0; while n < k + p { if h.insert(((m as u64) << 40) | ((c as u64) << 32) | r.gen_range(10..99999u64), 1).is_none() { n += 1; } }
        }
        println!("[B] hashbrown plano u64->u32             : {:>8.1} MB  (len {}, cap {})", mb(live() - base), h.len(), h.capacity());
    }
    // 3) Denso: Vec<u32> contiguo + offsets por (municipio,cargo)
    {
        let base = live();
        let offsets: Vec<u32> = wl.iter().scan(0u32, |acc, x| { let o = *acc; *acc += (x.2 + x.3) as u32; Some(o) }).collect();
        let tally: Vec<std::sync::atomic::AtomicU32> = (0..n_tuples).map(|_| std::sync::atomic::AtomicU32::new(0)).collect();
        // indice candidato->posicao por (UF,cargo): ~30 mil candidatos no total
        let idx: HashMap<(u8, u8, u32), u16> = (0..32_000u32).map(|i| (((i % 27) as u8, 6u8, i), (i % 2000) as u16)).collect();
        println!("[C] Denso Vec<AtomicU32> + offsets       : {:>8.1} MB  (indice cand. incl.)", mb(live() - base));
        drop((offsets, tally, idx));
    }
    // Deduplicacao
    {
        let base = live();
        let s: DashMap<SectionId,()> = DashMap::with_shard_amount(64);
        for i in 0..500_000u32 { s.insert(SectionId { election_id: 544, uf_id: (i % 28) as u8, municipality: i / 90, zone: (i % 400) as u16, section: (i % 997) as u16 }, ()); }
        println!("[D] DashSet<SectionId> 500k (64 shards)  : {:>8.1} MB  (size_of SectionId = {})", mb(live() - base), std::mem::size_of::<SectionId>());
        let base = live();
        let bits: Vec<u64> = vec![0; 500_000 / 64 + 1];
        println!("[D] Bitset denso 500k                    : {:>8.3} MB", mb(live() - base));
        drop(bits);
    }
    println!("size_of: Tally={} RwLock<HashMap<u32,u64>>≈ shard", std::mem::size_of::<Tally>());
}
