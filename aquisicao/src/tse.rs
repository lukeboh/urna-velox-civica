//! Endereços e formatos JSON do site oficial de resultados do TSE
//! (<https://resultados.tse.jus.br/oficial/app/index.html>).
//!
//! Hierarquia usada pela aquisição de BUs brutos (pleito = conjunto de eleições do mesmo dia;
//! um BU contém os resultados de todas as eleições do pleito naquela seção):
//!
//! ```text
//! <base>/<amb>/comum/config/ele-c.json                                   pleitos e eleições
//! <base>/<amb>/<ciclo>/arquivo-urna/<pleito>/config/<uf>/<uf>-p<pleito:06>-cs.json   seções da UF
//! <base>/<amb>/<ciclo>/arquivo-urna/<pleito>/dados/<uf>/<mun:05>/<zona:04>/<secao:04>/
//!        p<pleito:06>-<uf>-m<mun:05>-z<zona:04>-s<secao:04>-aux.json      arquivos da seção
//!        <hash>/o0<pleito:05><uf><mun:05><zona:04><secao:04>-bu.dat       BU (envelope BER)
//! ```
//! `<hash>` é o SHA-256 (base64, e depois em hexadecimal) do arquivo de assinaturas `…-vota.vsc`.

use serde::Deserialize;

pub const UFS: [&str; 28] = [
    "ac", "al", "am", "ap", "ba", "ce", "df", "es", "go", "ma", "mg", "ms", "mt", "pa", "pb", "pe",
    "pi", "pr", "rj", "rn", "ro", "rr", "rs", "sc", "se", "sp", "to", "zz",
];

#[derive(Clone, Debug)]
pub struct Alvo {
    pub base: String,
    pub ambiente: String,
    pub ciclo: String,
    pub pleito: u32,
}

impl Alvo {
    pub fn url_ele_c(&self) -> String { format!("{}/{}/comum/config/ele-c.json", self.base, self.ambiente) }
    fn raiz(&self) -> String { format!("{}/{}/{}/arquivo-urna/{}", self.base, self.ambiente, self.ciclo, self.pleito) }
    pub fn url_cs(&self, uf: &str) -> String { format!("{}/config/{uf}/{uf}-p{:06}-cs.json", self.raiz(), self.pleito) }
    pub fn dir_secao(&self, uf: &str, mu: u32, zn: u16, se: u16) -> String {
        format!("{}/dados/{uf}/{mu:05}/{zn:04}/{se:04}", self.raiz())
    }
    pub fn url_aux(&self, uf: &str, mu: u32, zn: u16, se: u16) -> String {
        format!("{}/p{:06}-{uf}-m{mu:05}-z{zn:04}-s{se:04}-aux.json", self.dir_secao(uf, mu, zn, se), self.pleito)
    }
    pub fn url_arquivo(&self, uf: &str, mu: u32, zn: u16, se: u16, hash: &str, nome: &str) -> String {
        format!("{}/{hash}/{nome}", self.dir_secao(uf, mu, zn, se))
    }
    pub fn url_mun_cm(&self, eleicao: u32) -> String {
        format!("{}/{}/{}/{eleicao}/config/mun-e{:06}-cm.json", self.base, self.ambiente, self.ciclo, eleicao)
    }
    pub fn url_u_json(&self, eleicao: u32, abr: &str, cargo: u8, subpasta: &str) -> String {
        format!("{}/{}/{}/{eleicao}/dados/{subpasta}/{}-c{:04}-e{:06}-u.json", self.base, self.ambiente, self.ciclo, abr, cargo, eleicao)
    }
    /// Pacote de dados abertos da UF (todos os arquivos de urna em um zip), publicado após a eleição.
    pub fn url_zip_dados_abertos(&self, uf: &str, turno: u8) -> String {
        let ano = &self.ciclo[3..];
        format!("https://cdn.tse.jus.br/estatistica/sead/eleicoes/eleicoes{ano}/arqurnatot/bu_imgbu_logjez_rdv_vscmr_{ano}_{turno}t_{}.zip", uf.to_uppercase())
    }
}

// ------------------------------------------------------------------ ele-c.json
#[derive(Deserialize, Debug)]
pub struct EleC { pub pl: Vec<Pleito> }
#[derive(Deserialize, Debug)]
pub struct Pleito { pub cd: String, pub c: String, pub dt: String, pub e: Vec<Eleicao> }
#[derive(Deserialize, Debug)]
pub struct Eleicao { pub cd: String, pub nm: String, #[serde(default)] pub t: String, #[serde(default)] pub tp: String }

// ------------------------------------------------------------------ <uf>-p<pleito>-cs.json
#[derive(Deserialize, Debug)]
pub struct ConfigSecoes {
    #[serde(default)] pub dg: String,
    #[serde(default)] pub hg: String,
    pub abr: Vec<AbrUf>,
}
#[derive(Deserialize, Debug)]
pub struct AbrUf { pub cd: String, #[serde(default)] pub mu: Vec<CsMunicipio> }
#[derive(Deserialize, Debug)]
pub struct CsMunicipio { pub cd: String, #[serde(default)] pub nm: String, #[serde(default)] pub zon: Vec<CsZona> }
#[derive(Deserialize, Debug)]
pub struct CsZona { pub cd: String, #[serde(default)] pub sec: Vec<CsSecao> }
#[derive(Deserialize, Debug)]
pub struct CsSecao {
    pub ns: String,
    #[serde(default)] pub da: String,
    #[serde(default)] pub ha: String,
    /// seção principal de uma seção **agregada** (sem urna nem BU próprio: os votos vêm no BU da principal)
    #[serde(default)] pub nsp: String,
}

/// Seção eleitoral a coletar.
#[derive(Clone, Debug)]
pub struct Secao {
    pub uf: [u8; 2],
    pub municipio: u32,
    pub zona: u16,
    pub secao: u16,
    /// data/hora de disponibilização informada no cs.json ("dd/mm/aaaa hh:mm:ss")
    pub disponivel: String,
}
impl Secao {
    pub fn uf_str(&self) -> &str { std::str::from_utf8(&self.uf).unwrap_or("??") }
    pub fn chave(&self) -> u64 { chave(self.municipio, self.zona, self.secao) }
}
pub use urna_velox::chave;

impl ConfigSecoes {
    /// Chaves ([`chave`]) das seções agregadas (com `nsp`) da UF.
    pub fn agregadas(&self) -> std::collections::HashSet<u64> {
        let mut v = std::collections::HashSet::new();
        for a in &self.abr {
            for m in &a.mu {
                let mu: u32 = m.cd.parse().unwrap_or(0);
                for z in &m.zon {
                    let zn: u16 = z.cd.parse().unwrap_or(0);
                    for s in &z.sec { if !s.nsp.trim().is_empty() { v.insert(chave(mu, zn, s.ns.parse().unwrap_or(0))); } }
                }
            }
        }
        v
    }
    pub fn secoes(&self, uf: &str) -> Vec<Secao> {
        let ufb: [u8; 2] = uf.as_bytes()[..2].try_into().unwrap();
        let mut v = vec![];
        for a in &self.abr {
            for m in &a.mu {
                let mu: u32 = m.cd.parse().unwrap_or(0);
                for z in &m.zon {
                    let zn: u16 = z.cd.parse().unwrap_or(0);
                    for s in &z.sec {
                        v.push(Secao { uf: ufb, municipio: mu, zona: zn, secao: s.ns.parse().unwrap_or(0), disponivel: format!("{} {}", s.da, s.ha) });
                    }
                }
            }
        }
        v
    }
    pub fn municipios(&self) -> Vec<(u32, String)> {
        self.abr.iter().flat_map(|a| a.mu.iter().map(|m| (m.cd.parse().unwrap_or(0), m.nm.clone()))).collect()
    }
}

// ------------------------------------------------------------------ ...-aux.json
#[derive(Deserialize, Debug)]
pub struct Aux { #[serde(default)] pub st: String, #[serde(default)] pub hashes: Vec<AuxHash> }
#[derive(Deserialize, Debug)]
pub struct AuxHash {
    pub hash: String,
    #[serde(default)] pub dr: String,
    #[serde(default)] pub hr: String,
    #[serde(default)] pub st: String,
    #[serde(default)] pub arq: Vec<AuxArquivo>,
}
#[derive(Deserialize, Debug)]
pub struct AuxArquivo { pub nm: String, pub tp: String }

impl Aux {
    /// Escolhe o conjunto de arquivos a usar: o último com situação "Totalizado";
    /// sem nenhum totalizado, o último que tenha BU. Devolve (hash, nome do BU, situação, recebido em).
    ///
    /// O BU pode ser o da urna (`tp = "bu"`, `…-bu.dat`) ou, quando a seção foi apurada pelo Sistema de
    /// Apuração (urna com defeito, votação em cédula), o `tp = "busa"` (`…-busa.dat`, assinado em `-sa.vsc`).
    pub fn escolher_bu(&self) -> Option<(&str, &str, &str, String)> {
        let e_bu = |a: &AuxArquivo| a.tp == "bu" || a.tp == "busa";
        let tem_bu = |h: &&AuxHash| h.arq.iter().any(e_bu);
        let h = self.hashes.iter().rev().filter(tem_bu).find(|h| h.st.starts_with("Totalizad"))
            .or_else(|| self.hashes.iter().rev().find(tem_bu))?;
        let nm = h.arq.iter().find(|a| a.tp == "bu").or_else(|| h.arq.iter().find(|a| a.tp == "busa"))?.nm.as_str();
        Some((h.hash.as_str(), nm, h.st.as_str(), format!("{} {}", h.dr, h.hr)))
    }
}

/// O hash do diretório é o texto base64 do SHA-256 do `.vsc`, escrito em hexadecimal.
/// Devolve os 32 bytes do SHA-256 (ou zeros se o formato for outro).
pub fn sha256_do_hash_dir(h: &str) -> [u8; 32] {
    use base64::Engine;
    let mut out = [0u8; 32];
    let txt: Vec<u8> = (0..h.len() / 2).filter_map(|i| u8::from_str_radix(&h[2 * i..2 * i + 2], 16).ok()).collect();
    if let Ok(raw) = base64::engine::general_purpose::STANDARD.decode(&txt) {
        if raw.len() == 32 { out.copy_from_slice(&raw); }
    }
    out
}
pub fn hash_dir_do_sha256(s: &[u8; 32]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(s).bytes().map(|b| format!("{b:02x}")).collect()
}

/// Interpreta o nome de um arquivo de urna: `o03220df9701200150300-bu.dat` → (pleito, uf, mun, zona, seção).
pub fn chave_do_nome(nome: &str) -> Option<(u32, [u8; 2], u32, u16, u16)> {
    let base = nome.rsplit('/').next()?;
    let b = base.as_bytes();
    if b.len() < 21 || b[0] != b'o' { return None; }
    let pleito = base.get(1..6)?.parse().ok()?;
    let uf = [b[6], b[7]];
    let mu = base.get(8..13)?.parse().ok()?;
    let zn = base.get(13..17)?.parse().ok()?;
    let se = base.get(17..21)?.parse().ok()?;
    Some((pleito, uf, mu, zn, se))
}

#[cfg(test)]
mod testes {
    use super::*;
    #[test]
    fn nomes_e_hashes() {
        assert_eq!(chave_do_nome("o03220df9701200150300-bu.dat"), Some((3220, *b"df", 97012, 15, 300)));
        let h = "35307a474c3238496d3479377954436274424e44546471474f6477656a4539704a676730765342684b58733d";
        let s = sha256_do_hash_dir(h);
        assert_eq!(s[0], 0xe7);
        assert_eq!(hash_dir_do_sha256(&s), h);
        let a = Alvo { base: "https://resultados.tse.jus.br".into(), ambiente: "oficial".into(), ciclo: "ele2026".into(), pleito: 3220 };
        assert_eq!(a.url_aux("df", 97012, 15, 300), "https://resultados.tse.jus.br/oficial/ele2026/arquivo-urna/3220/dados/df/97012/0015/0300/p003220-df-m97012-z0015-s0300-aux.json");
    }

    #[test]
    fn bu_do_sistema_de_apuracao() {
        // aux.json real (MG 50075/0047/0041, 1º turno 2026): seção apurada pelo SA, sem -bu.dat
        let j = r#"{ "st" : "Totalizada", "hashes" : [ { "hash" : "616e", "dr" : "04/10/2026", "hr" : "22:29:08", "st" : "Totalizado", "arq" : [
            { "nm" : "o03220mg5007500470041-rdv.dat", "tp" : "rdv" }, { "nm" : "o03220mg5007500470041-logsa.jez", "tp" : "logsa" },
            { "nm" : "o03220mg5007500470041-sa.vsc", "tp" : "sa" }, { "nm" : "o03220mg5007500470041-busa.dat", "tp" : "busa" } ] } ] }"#;
        let a: Aux = serde_json::from_str(j).unwrap();
        assert_eq!(a.escolher_bu().map(|x| x.1), Some("o03220mg5007500470041-busa.dat"));
        assert_eq!(chave_do_nome("o03220mg5007500470041-busa.dat"), Some((3220, *b"mg", 50075, 47, 41)));
        assert!(crate::zip::e_bu("o03220mg5007500470041-busa.dat") && !crate::zip::e_bu("o03220mg5007500470041-sa.vsc"));
    }
}
