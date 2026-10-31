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
//!        <hash>/o<pleito:05><uf><mun:05><zona:04><secao:04>-bu.dat        BU (envelope BER)
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
    /// modelos de caminho (padrão: os de hoje; `--rotas` troca, ver [`Rotas`])
    pub rotas: std::sync::Arc<Rotas>,
}

impl Alvo {
    /// Alvo com os caminhos padrão do site do TSE.
    pub fn novo(base: &str, ambiente: &str, ciclo: &str, pleito: u32) -> Alvo {
        Alvo { base: base.trim_end_matches('/').to_string(), ambiente: ambiente.into(), ciclo: ciclo.into(), pleito, rotas: Default::default() }
    }

    /// Variáveis comuns a todos os modelos; `extra` resolve as do arquivo (seção, hash, eleição...).
    fn montar(&self, modelo: &str, extra: &dyn Fn(&str, &mut String) -> bool) -> String {
        use std::fmt::Write;
        montar_modelo(modelo, &|nome: &str, o: &mut String| {
            match nome {
                "base" => o.push_str(&self.base),
                "base_arquivos" => o.push_str(self.rotas.base_arquivos.as_deref().map(|b| b.trim_end_matches('/')).unwrap_or(&self.base)),
                "amb" => o.push_str(&self.ambiente),
                "ciclo" => o.push_str(&self.ciclo),
                "pleito" => { let _ = write!(o, "{}", self.pleito); }
                "pleito06" => { let _ = write!(o, "{:06}", self.pleito); }
                _ => return extra(nome, o),
            }
            true
        })
    }

    fn secao(&self, modelo: &str, uf: &str, mu: u32, zn: u16, se: u16, extra: &dyn Fn(&str, &mut String) -> bool) -> String {
        use std::fmt::Write;
        self.montar(modelo, &|nome: &str, o: &mut String| {
            match nome {
                "uf" => o.push_str(uf),
                "mun05" => { let _ = write!(o, "{mu:05}"); }
                "zona04" => { let _ = write!(o, "{zn:04}"); }
                "secao04" => { let _ = write!(o, "{se:04}"); }
                _ => return extra(nome, o),
            }
            true
        })
    }

    pub fn url_ele_c(&self) -> String { self.montar(&self.rotas.ele_c, &|_, _| false) }
    pub fn url_cs(&self, uf: &str) -> String {
        self.montar(&self.rotas.cs, &|n: &str, o: &mut String| { if n == "uf" { o.push_str(uf); true } else { false } })
    }
    pub fn url_aux(&self, uf: &str, mu: u32, zn: u16, se: u16) -> String { self.secao(&self.rotas.aux, uf, mu, zn, se, &|_, _| false) }
    pub fn url_arquivo(&self, uf: &str, mu: u32, zn: u16, se: u16, hash: &str, nome: &str) -> String {
        self.secao(&self.rotas.arquivo, uf, mu, zn, se, &|n: &str, o: &mut String| {
            match n { "hash" => o.push_str(hash), "nome" => o.push_str(nome), _ => return false }
            true
        })
    }
    pub fn url_mun_cm(&self, eleicao: u32) -> String {
        use std::fmt::Write;
        self.montar(&self.rotas.mun_cm, &|n: &str, o: &mut String| {
            match n { "eleicao" => { let _ = write!(o, "{eleicao}"); } "eleicao06" => { let _ = write!(o, "{eleicao:06}"); } _ => return false }
            true
        })
    }
    pub fn url_u_json(&self, eleicao: u32, abr: &str, cargo: u8, subpasta: &str) -> String {
        use std::fmt::Write;
        self.montar(&self.rotas.u_json, &|n: &str, o: &mut String| {
            match n {
                "eleicao" => { let _ = write!(o, "{eleicao}"); }
                "eleicao06" => { let _ = write!(o, "{eleicao:06}"); }
                "abr" => o.push_str(abr),
                "cargo04" => { let _ = write!(o, "{cargo:04}"); }
                "subpasta" => o.push_str(subpasta),
                _ => return false,
            }
            true
        })
    }
    /// Pacote de dados abertos da UF (todos os arquivos de urna em um zip), publicado após a eleição.
    pub fn url_zip_dados_abertos(&self, uf: &str, turno: u8) -> String {
        let ano = &self.ciclo[3..];
        format!("https://cdn.tse.jus.br/estatistica/sead/eleicoes/eleicoes{ano}/arqurnatot/bu_imgbu_logjez_rdv_vscmr_{ano}_{turno}t_{}.zip", uf.to_uppercase())
    }
}

/// Troca cada `{nome}` do modelo pelo que `f` escreve; um nome que `f` não conhece fica como está (a validação de
/// [`Rotas::validar`] recusa esses modelos antes de chegarem aqui).
pub fn montar_modelo(modelo: &str, f: &dyn Fn(&str, &mut String) -> bool) -> String {
    let mut o = String::with_capacity(modelo.len() + 64);
    let mut r = modelo;
    while let Some(i) = r.find('{') {
        o.push_str(&r[..i]);
        let resto = &r[i + 1..];
        match resto.find('}') {
            Some(j) => {
                let nome = &resto[..j];
                if !f(nome, &mut o) { o.push('{'); o.push_str(nome); o.push('}'); }
                r = &resto[j + 1..];
            }
            None => { o.push_str(&r[i..]); r = ""; }
        }
    }
    o.push_str(r);
    o
}

// ------------------------------------------------------------------ rotas_tse.json

pub const ROTA_ELE_C: &str = "{base}/{amb}/comum/config/ele-c.json";
pub const ROTA_CS: &str = "{base}/{amb}/{ciclo}/arquivo-urna/{pleito}/config/{uf}/{uf}-p{pleito06}-cs.json";
pub const ROTA_AUX: &str = "{base}/{amb}/{ciclo}/arquivo-urna/{pleito}/dados/{uf}/{mun05}/{zona04}/{secao04}/p{pleito06}-{uf}-m{mun05}-z{zona04}-s{secao04}-aux.json";
pub const ROTA_ARQUIVO: &str = "{base_arquivos}/{amb}/{ciclo}/arquivo-urna/{pleito}/dados/{uf}/{mun05}/{zona04}/{secao04}/{hash}/{nome}";
pub const ROTA_MUN_CM: &str = "{base}/{amb}/{ciclo}/{eleicao}/config/mun-e{eleicao06}-cm.json";
pub const ROTA_U_JSON: &str = "{base}/{amb}/{ciclo}/{eleicao}/dados/{subpasta}/{abr}-c{cargo04}-e{eleicao06}-u.json";

fn r_ele_c() -> String { ROTA_ELE_C.into() }
fn r_cs() -> String { ROTA_CS.into() }
fn r_aux() -> String { ROTA_AUX.into() }
fn r_arquivo() -> String { ROTA_ARQUIVO.into() }
fn r_mun_cm() -> String { ROTA_MUN_CM.into() }
fn r_u_json() -> String { ROTA_U_JSON.into() }

/// Modelos de caminho do site de resultados (`/etc/urna-velox/rotas_tse.json`, `uv-servico --rotas`, painel dos
/// nós). Servem para seguir o TSE se ele mudar a organização dos arquivos, e para buscar os arquivos de urna (BUs) de
/// outro endereço (`base_arquivos`) sem mudar de onde vêm as listas de seções.
///
/// Variáveis: `{base}` e `{base_arquivos}` (padrão: a própria base), `{amb}`, `{ciclo}`, `{pleito}`, `{pleito06}`;
/// por seção `{uf}`, `{mun05}`, `{zona04}`, `{secao04}`; no arquivo `{hash}` e `{nome}`; nos resultados `{eleicao}`,
/// `{eleicao06}`, `{abr}`, `{cargo04}`, `{subpasta}`.
#[derive(Clone, Debug, PartialEq, serde::Serialize, Deserialize)]
pub struct Rotas {
    /// Outro servidor para os arquivos de urna (`{base_arquivos}`); vazio = o mesmo da base.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_arquivos: Option<String>,
    #[serde(default = "r_ele_c")]
    pub ele_c: String,
    #[serde(default = "r_cs")]
    pub cs: String,
    #[serde(default = "r_aux")]
    pub aux: String,
    #[serde(default = "r_arquivo")]
    pub arquivo: String,
    #[serde(default = "r_mun_cm")]
    pub mun_cm: String,
    #[serde(default = "r_u_json")]
    pub u_json: String,
}

impl Default for Rotas {
    fn default() -> Rotas {
        Rotas { base_arquivos: None, ele_c: r_ele_c(), cs: r_cs(), aux: r_aux(), arquivo: r_arquivo(), mun_cm: r_mun_cm(), u_json: r_u_json() }
    }
}

const VARS_COMUNS: &[&str] = &["base", "base_arquivos", "amb", "ciclo", "pleito", "pleito06"];
const VARS_SECAO: &[&str] = &["uf", "mun05", "zona04", "secao04"];
const NADA: &[&str] = &[];
const SO_UF: &[&str] = &["uf"];
const VARS_ARQUIVO: &[&str] = &["uf", "mun05", "zona04", "secao04", "hash", "nome"];
const SO_NOME: &[&str] = &["nome"];
const VARS_ELEICAO: &[&str] = &["eleicao", "eleicao06"];
const VARS_U_JSON: &[&str] = &["eleicao", "eleicao06", "abr", "cargo04", "subpasta"];
const OBRIG_U_JSON: &[&str] = &["abr", "cargo04"];

impl Rotas {
    pub fn de_json(t: &str) -> anyhow::Result<Rotas> {
        let r: Rotas = serde_json::from_str(t).map_err(|e| anyhow::anyhow!("rotas_tse.json inválido: {e}"))?;
        Ok(r)
    }

    pub fn ler(p: &std::path::Path) -> anyhow::Result<Rotas> {
        Rotas::de_json(&std::fs::read_to_string(p).map_err(|e| anyhow::anyhow!("lendo {}: {e}", p.display()))?)
    }

    pub fn padrao(&self) -> bool { *self == Rotas::default() }

    /// (nome, modelo, variáveis permitidas além das comuns, obrigatórias)
    fn modelos(&self) -> [(&'static str, &str, &'static [&'static str], &'static [&'static str]); 6] {
        [
            ("ele_c", self.ele_c.as_str(), NADA, NADA),
            ("cs", self.cs.as_str(), SO_UF, SO_UF),
            ("aux", self.aux.as_str(), VARS_SECAO, VARS_SECAO),
            ("arquivo", self.arquivo.as_str(), VARS_ARQUIVO, SO_NOME),
            ("mun_cm", self.mun_cm.as_str(), VARS_ELEICAO, NADA),
            ("u_json", self.u_json.as_str(), VARS_U_JSON, OBRIG_U_JSON),
        ]
    }

    /// Recusa modelos que levariam a coleta para fora do controle:
    /// * todo modelo começa por `{base}` (o arquivo de urna também pode começar por `{base_arquivos}`): o servidor
    ///   vem sempre da base configurada, nunca de um endereço escrito no modelo;
    /// * só variáveis conhecidas, as obrigatórias de cada modelo e caracteres de caminho (sem `?`, `#`, `..`, espaço);
    /// * `base_arquivos` é local (127.0.0.1/localhost) **se e só se** a base for: a aceleração e o ritmo de um servidor
    ///   local nunca valem contra o TSE.
    pub fn validar(&self, base: &str) -> anyhow::Result<()> {
        use anyhow::bail;
        for (nome, m, extras, obrig) in self.modelos() {
            let inicio_ok = m.starts_with("{base}/") || (nome == "arquivo" && m.starts_with("{base_arquivos}/"));
            if !inicio_ok { bail!("modelo {nome}: tem de começar por {{base}}/{}", if nome == "arquivo" { " (ou {base_arquivos}/)" } else { "" }); }
            if m.len() > 400 || m.contains("..") || !m.bytes().all(|b| b.is_ascii_alphanumeric() || b"-._~/{}".contains(&b)) {
                bail!("modelo {nome}: só letras, dígitos, - . _ ~ / e variáveis entre chaves");
            }
            let mut achadas = vec![];
            let mut r: &str = m;
            while let Some(i) = r.find('{') {
                let Some(j) = r[i..].find('}') else { bail!("modelo {nome}: chave aberta sem fechar") };
                let v = &r[i + 1..i + j];
                if !VARS_COMUNS.contains(&v) && !extras.contains(&v) { bail!("modelo {nome}: variável {{{v}}} não vale aqui"); }
                achadas.push(v);
                r = &r[i + j + 1..];
            }
            if r.contains('}') { bail!("modelo {nome}: chave fechada sem abrir"); }
            for o in obrig { if !achadas.contains(o) { bail!("modelo {nome}: falta {{{o}}}"); } }
        }
        if let Some(b) = &self.base_arquivos {
            if !(b.starts_with("https://") || b.starts_with("http://")) || b.len() > 200
                || !b.bytes().all(|c| c.is_ascii_alphanumeric() || b"-._~:/".contains(&c)) {
                bail!("base_arquivos: URL http(s):// só com letras, dígitos e - . _ ~ : /");
            }
            if crate::coletor::base_local(b) != crate::coletor::base_local(base) {
                bail!("base_arquivos ({b}) e a base ({base}) têm de ser os dois locais (simulador) ou os dois remotos");
            }
        }
        Ok(())
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
mod testes;
