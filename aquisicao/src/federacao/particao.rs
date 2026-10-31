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
mod testes;
