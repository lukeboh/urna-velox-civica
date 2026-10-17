//! Partição das seções entre os nós vivos: *Rendezvous Hashing* (HRW) **ponderado e só com inteiros** (ADR-022,
//! ADR-028).
//!
//! Um nó de peso `w` concorre com `w` identidades virtuais `(id, 0) … (id, w-1)`; a seção fica com o nó dono da
//! identidade de maior nota. Nota = `mistura(h_secao ^ h_virtual)`, onde
//!
//! * `h_secao   = mistura(fnv1a64(municipio u32 LE ‖ zona u16 LE ‖ secao u16 LE))`;
//! * `h_virtual = mistura(fnv1a64(bytes do id ‖ 0x00 ‖ indice u32 LE))`;
//! * `mistura` = finalização do *splitmix64* (constantes de Steele, Lea e Flood).
//!
//! Empate de nota (probabilidade ~2⁻⁶⁴): ganha o id lexicograficamente menor. Tudo inteiro e especificado aqui,
//! para que nós em sistemas diferentes cheguem à mesma partição (`testes::valores_fixos` trava os números).

/// FNV-1a de 64 bits.
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// Finalização do *splitmix64*.
pub fn mistura(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9e37_79b9_7f4a_7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// Hash de uma seção (chave natural: município, zona, seção).
pub fn hash_secao(municipio: u32, zona: u16, secao: u16) -> u64 {
    let mut b = [0u8; 8];
    b[..4].copy_from_slice(&municipio.to_le_bytes());
    b[4..6].copy_from_slice(&zona.to_le_bytes());
    b[6..].copy_from_slice(&secao.to_le_bytes());
    mistura(fnv1a64(&b))
}

/// Hash da identidade virtual `indice` do nó `id`.
pub fn hash_virtual(id: &str, indice: u32) -> u64 {
    let mut b = Vec::with_capacity(id.len() + 5);
    b.extend_from_slice(id.as_bytes());
    b.push(0);
    b.extend_from_slice(&indice.to_le_bytes());
    mistura(fnv1a64(&b))
}

/// Um nó que participa da partição, com o seu peso (≥ 1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Participante {
    pub id: String,
    pub peso: u32,
}

/// Partição pronta para consulta: as identidades virtuais já com hash, em ordem de id (o empate usa essa ordem).
#[derive(Clone, Debug)]
pub struct Particao {
    ids: Vec<String>,
    /// (hash da identidade virtual, índice do nó em `ids`)
    virtuais: Vec<(u64, u32)>,
}

impl Particao {
    /// Monta a partição. Pesos 0 contam como 1; ids repetidos ficam com a primeira ocorrência.
    pub fn nova(participantes: &[Participante]) -> Particao {
        let mut ps: Vec<&Participante> = participantes.iter().collect();
        ps.sort_by(|a, b| a.id.cmp(&b.id));
        ps.dedup_by(|a, b| a.id == b.id);
        let mut ids = Vec::with_capacity(ps.len());
        let mut virtuais = Vec::new();
        for (i, p) in ps.iter().enumerate() {
            ids.push(p.id.clone());
            for v in 0..p.peso.max(1) {
                virtuais.push((hash_virtual(&p.id, v), i as u32));
            }
        }
        Particao { ids, virtuais }
    }

    pub fn vazia(&self) -> bool { self.ids.is_empty() }
    pub fn ids(&self) -> &[String] { &self.ids }

    /// Índice (em `ids()`) do nó responsável pela seção; `None` sem participantes.
    pub fn indice_dono(&self, municipio: u32, zona: u16, secao: u16) -> Option<usize> {
        let hs = hash_secao(municipio, zona, secao);
        let mut melhor: Option<(u64, u32)> = None;
        for &(hv, i) in &self.virtuais {
            let nota = mistura(hs ^ hv);
            melhor = match melhor {
                None => Some((nota, i)),
                Some((n, j)) if nota > n || (nota == n && i < j) => Some((nota, i)),
                m => m,
            };
        }
        melhor.map(|(_, i)| i as usize)
    }

    /// Id do nó responsável pela seção.
    pub fn dono(&self, municipio: u32, zona: u16, secao: u16) -> Option<&str> {
        self.indice_dono(municipio, zona, secao).map(|i| self.ids[i].as_str())
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn p(id: &str, peso: u32) -> Participante { Participante { id: id.into(), peso } }

    /// Seções sintéticas com a forma das reais: municípios de 5 dígitos, zonas e seções pequenas.
    fn secoes(n: usize) -> Vec<(u32, u16, u16)> {
        (0..n).map(|i| (10_000 + (i / 400) as u32 * 7, (i / 40 % 10) as u16 + 1, (i % 40) as u16 + 1)).collect()
    }

    fn contagem(pt: &Particao, ss: &[(u32, u16, u16)]) -> Vec<usize> {
        let mut c = vec![0; pt.ids().len()];
        for &(m, z, s) in ss { c[pt.indice_dono(m, z, s).unwrap()] += 1; }
        c
    }

    #[test]
    fn valores_fixos() {
        // Travam a especificação: mudar o hash muda a partição entre versões e entre nós.
        assert_eq!(fnv1a64(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a64(b"a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(mistura(0), 0xe220_a839_7b1d_cdaf);
        let pt = Particao::nova(&[p("no-a", 19), p("no-b", 19), p("no-c", 19)]);
        let donos: Vec<&str> = [(71072, 1, 1), (71072, 1, 2), (97012, 3, 104), (38490, 250, 77), (1, 1, 1)]
            .iter().map(|&(m, z, s)| pt.dono(m, z, s).unwrap()).collect();
        assert_eq!(donos, DONOS_FIXOS);
        assert_eq!(hash_secao(71072, 1, 1), HASH_SECAO_71072_1_1);
        assert_eq!(hash_virtual("no-a", 0), HASH_VIRTUAL_NO_A_0);
    }
    // Saída da primeira execução (`2.0.0-alpha.3`, Linux x86-64), mantida desde então: se estes valores mudarem,
    // nós de versões diferentes deixam de concordar sobre a partição.
    const DONOS_FIXOS: [&str; 5] = ["no-c", "no-b", "no-c", "no-a", "no-b"];
    const HASH_SECAO_71072_1_1: u64 = 8_626_954_529_163_576_250;
    const HASH_VIRTUAL_NO_A_0: u64 = 12_738_679_178_947_764_935;

    #[test]
    fn uniforme_com_pesos_iguais() {
        let ss = secoes(300_000);
        let c = contagem(&Particao::nova(&[p("no-a", 19), p("no-b", 19), p("no-c", 19)]), &ss);
        for n in c { assert!((n as f64 / 100_000.0 - 1.0).abs() < 0.02, "{n}"); }
    }

    #[test]
    fn proporcional_ao_peso() {
        let ss = secoes(300_000);
        // dois nós dividindo um IP (peso 9 cada) e um sozinho (peso 19)
        let c = contagem(&Particao::nova(&[p("a", 9), p("b", 9), p("c", 19)]), &ss);
        let t = ss.len() as f64;
        assert!((c[0] as f64 / t - 9.0 / 37.0).abs() < 0.01, "{c:?}");
        assert!((c[1] as f64 / t - 9.0 / 37.0).abs() < 0.01, "{c:?}");
        assert!((c[2] as f64 / t - 19.0 / 37.0).abs() < 0.01, "{c:?}");
    }

    #[test]
    fn saida_de_um_no_so_libera_as_dele() {
        let ss = secoes(100_000);
        let antes = Particao::nova(&[p("a", 19), p("b", 19), p("c", 19)]);
        let depois = Particao::nova(&[p("a", 19), p("b", 19)]);
        let mut movidas = 0;
        for &(m, z, s) in &ss {
            let (x, y) = (antes.dono(m, z, s).unwrap(), depois.dono(m, z, s).unwrap());
            if x != "c" { assert_eq!(x, y, "seção {m}/{z}/{s} mudou de dono sem ser do nó que saiu"); } else { movidas += 1; }
        }
        assert!((movidas as f64 / ss.len() as f64 - 1.0 / 3.0).abs() < 0.02);
    }

    #[test]
    fn entrada_de_um_no_so_toma_para_si() {
        let ss = secoes(100_000);
        let antes = Particao::nova(&[p("a", 19), p("b", 19)]);
        let depois = Particao::nova(&[p("a", 19), p("b", 19), p("d", 19)]);
        let mut tomadas = 0;
        for &(m, z, s) in &ss {
            let (x, y) = (antes.dono(m, z, s).unwrap(), depois.dono(m, z, s).unwrap());
            if x != y { assert_eq!(y, "d"); tomadas += 1; }
        }
        assert!((tomadas as f64 / ss.len() as f64 - 1.0 / 3.0).abs() < 0.02);
    }

    #[test]
    fn mudanca_de_peso_so_mexe_no_no_que_mudou() {
        // b passa a dividir o IP com um nó novo: peso 19 → 9. Seções só saem de b (para a, c ou e) ou vão para e.
        let ss = secoes(100_000);
        let antes = Particao::nova(&[p("a", 19), p("b", 19), p("c", 19)]);
        let depois = Particao::nova(&[p("a", 19), p("b", 9), p("c", 19), p("e", 9)]);
        for &(m, z, s) in &ss {
            let (x, y) = (antes.dono(m, z, s).unwrap(), depois.dono(m, z, s).unwrap());
            if x != y { assert!(x == "b" || y == "e", "{x} → {y}"); }
        }
    }

    #[test]
    fn ordem_dos_participantes_nao_importa() {
        let a = Particao::nova(&[p("a", 3), p("b", 5), p("c", 7)]);
        let b = Particao::nova(&[p("c", 7), p("a", 3), p("b", 5)]);
        for &(m, z, s) in &secoes(10_000) { assert_eq!(a.dono(m, z, s), b.dono(m, z, s)); }
    }

    #[test]
    fn sem_participantes() {
        assert!(Particao::nova(&[]).dono(1, 1, 1).is_none());
    }
}
