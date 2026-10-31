//! Storage comum da federação (ADR-028 §6): *heartbeats*, BUs trocados e, depois, os resultados publicados.
//!
//! Interface mínima, que um *object storage* S3-compatível (Cloudflare R2) também oferece: gravar, ler com `ETag`
//! (para não baixar de novo o que não mudou), listar por prefixo e o **relógio do storage** (o `Date` das respostas),
//! que decide quem está vivo sem depender do relógio de cada nó.
//!
//! [`ArmazemLocal`] grava num diretório: serve aos testes e a vários nós na mesma máquina com o `uv-simulador`.

use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Versão de um objeto: `etag` (SHA-256 do conteúdo, hexadecimal) e instante da última gravação pelo relógio do
/// storage.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Versao {
    pub etag: String,
    pub modificado: SystemTime,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Objeto {
    pub versao: Versao,
    pub bytes: Vec<u8>,
}

pub trait Armazem: Send + Sync {
    /// Grava (substitui) o objeto; a gravação é atômica para quem lê.
    fn gravar(&self, chave: &str, bytes: &[u8]) -> Result<Versao>;
    /// Lê o objeto: `NaoExiste`, `NaoMudou` (o `etag` atual é `se_diferente_de`, como um 304) ou `Novo`.
    fn ler(&self, chave: &str, se_diferente_de: Option<&str>) -> Result<Lido>;
    /// Chaves com o prefixo, em ordem, com a versão de cada uma.
    fn listar(&self, prefixo: &str) -> Result<Vec<(String, Versao)>>;
    /// Agora, pelo relógio do storage.
    fn agora(&self) -> Result<SystemTime>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Lido {
    NaoExiste,
    NaoMudou,
    Novo(Objeto),
}

/// Chaves: segmentos `[A-Za-z0-9._-]` separados por `/`, sem `..`, sem `/` no começo ou no fim.
pub fn validar_chave(chave: &str) -> Result<()> {
    if chave.is_empty() || chave.len() > 512 || chave.starts_with('/') || chave.ends_with('/') {
        bail!("chave inválida: '{chave}'");
    }
    for seg in chave.split('/') {
        if seg.is_empty() || seg == "." || seg == ".." || !seg.chars().all(|c| c.is_ascii_alphanumeric() || "._-".contains(c)) {
            bail!("chave inválida: '{chave}'");
        }
    }
    Ok(())
}

fn etag(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

/// Storage num diretório local. Cada objeto é um arquivo; a gravação vai para um temporário e é renomeada.
pub struct ArmazemLocal {
    raiz: PathBuf,
}

impl ArmazemLocal {
    pub fn novo(raiz: impl AsRef<Path>) -> Result<ArmazemLocal> {
        let raiz = raiz.as_ref().to_path_buf();
        std::fs::create_dir_all(&raiz).with_context(|| format!("criando {}", raiz.display()))?;
        Ok(ArmazemLocal { raiz })
    }

    fn caminho(&self, chave: &str) -> Result<PathBuf> {
        validar_chave(chave)?;
        Ok(self.raiz.join(chave))
    }

    fn versao_de(caminho: &Path, bytes: &[u8]) -> Result<Versao> {
        let modificado = std::fs::metadata(caminho)?.modified()?;
        Ok(Versao { etag: etag(bytes), modificado })
    }
}

impl Armazem for ArmazemLocal {
    fn gravar(&self, chave: &str, bytes: &[u8]) -> Result<Versao> {
        let c = self.caminho(chave)?;
        if let Some(pai) = c.parent() { std::fs::create_dir_all(pai)?; }
        // temporário único por processo e gravação, no mesmo diretório (rename atômico)
        static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let tmp = c.with_file_name(format!(".{}.{}.{}.tmp", c.file_name().unwrap().to_string_lossy(), std::process::id(), n));
        std::fs::write(&tmp, bytes).with_context(|| format!("gravando {}", tmp.display()))?;
        std::fs::rename(&tmp, &c).with_context(|| format!("renomeando para {}", c.display()))?;
        ArmazemLocal::versao_de(&c, bytes)
    }

    fn ler(&self, chave: &str, se_diferente_de: Option<&str>) -> Result<Lido> {
        let c = self.caminho(chave)?;
        let bytes = match std::fs::read(&c) {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Lido::NaoExiste),
            Err(e) => return Err(e).with_context(|| format!("lendo {}", c.display())),
        };
        let versao = ArmazemLocal::versao_de(&c, &bytes)?;
        if se_diferente_de == Some(versao.etag.as_str()) { return Ok(Lido::NaoMudou); }
        Ok(Lido::Novo(Objeto { versao, bytes }))
    }

    fn listar(&self, prefixo: &str) -> Result<Vec<(String, Versao)>> {
        let mut saida = Vec::new();
        let mut pilha = vec![self.raiz.clone()];
        while let Some(d) = pilha.pop() {
            let entradas = match std::fs::read_dir(&d) {
                Ok(e) => e,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(e) => return Err(e.into()),
            };
            for e in entradas {
                let e = e?;
                let nome = e.file_name().to_string_lossy().to_string();
                if nome.starts_with('.') { continue; } // temporários
                let p = e.path();
                if e.file_type()?.is_dir() { pilha.push(p); continue; }
                let rel = p.strip_prefix(&self.raiz)?.to_string_lossy().replace('\\', "/");
                if !rel.starts_with(prefixo) { continue; }
                let bytes = std::fs::read(&p)?;
                saida.push((rel, ArmazemLocal::versao_de(&p, &bytes)?));
            }
        }
        saida.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(saida)
    }

    fn agora(&self) -> Result<SystemTime> { Ok(SystemTime::now()) }
}

#[cfg(test)]
mod testes;
