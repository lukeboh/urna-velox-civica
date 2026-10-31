//! `federacao.json`: quem participa da federação e com que limites (ADR-028).
//!
//! ```json
//! { "versao": 1,
//!   "nos": [ { "id": "no-a", "orgao": "Órgão A", "ip_saida": "200.1.2.3" },
//!            { "id": "no-b", "orgao": "Órgão B", "ip_saida": "189.4.5.6", "teto_rps": 100 } ],
//!   "teto_coleta_rps": 95, "unidade_peso_rps": 5, "batimento_s": 10, "morte_s": 30, "pausa_429_s": 600 }
//! ```
//!
//! Regras (recusadas na leitura): ids e órgãos únicos (**um nó por órgão**); teto de um nó acima de 100 req/s só com
//! `acordo` preenchido (autorização registrada para passar de 100 req/s; sem ela, acima disso há HTTP 429); `teto_coleta_rps` ≤ teto de cada nó. A assinatura do arquivo e
//! as chaves Ed25519 entram na etapa C.

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Limite por órgão (por IP de saída) sem autorização registrada (`CLAUDE.md`); acima disso, HTTP 429.
pub const LIMITE_POR_ORGAO_RPS: f64 = 100.0;

fn padrao_teto() -> f64 { LIMITE_POR_ORGAO_RPS }
fn padrao_teto_coleta() -> f64 { 95.0 }
fn padrao_unidade() -> f64 { 5.0 }
fn padrao_batimento() -> u64 { 10 }
fn padrao_morte() -> u64 { 30 }
fn padrao_pausa() -> u64 { 600 }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct NoConfig {
    /// Identificador estável do nó (entra no hash da partição: não mude durante uma eleição).
    pub id: String,
    /// Órgão que opera o nó; um nó por órgão.
    pub orgao: String,
    /// IP de saída declarado (o que a CDN do TSE vê). O nó só coleta se o IP medido for este.
    pub ip_saida: String,
    /// Teto do órgão, em req/s (padrão 100).
    #[serde(default = "padrao_teto")]
    pub teto_rps: f64,
    /// Autorização registrada que permite teto acima de 100 req/s (referência do documento).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acordo: Option<String>,
    /// Chave pública Ed25519 em hexadecimal (etapa C).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chave_publica: Option<String>,
    /// Endereço da sincronização entre nós (`host:porta`): onde os pares buscam os *heartbeats* e os lotes deste nó (e
    /// as cópias que ele guarda dos outros, para repasse). Necessário para quem coleta: sem ele, ninguém busca o que o
    /// nó coletou nem o *heartbeat* dele, e os pares o veem como morto.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endereco: Option<String>,
    /// `false` = o nó só recebe dos pares e totaliza, nunca pede seção ao TSE (fica fora da partição). Para um segundo
    /// servidor do **mesmo órgão**: a regra de 100 req/s é por órgão, então dois servidores do mesmo órgão não podem
    /// coletar cada um a 100 req/s (`CLAUDE.md`). Padrão: `true`.
    #[serde(default = "padrao_coleta", skip_serializing_if = "eh_verdadeiro")]
    pub coleta: bool,
}

fn padrao_coleta() -> bool { true }
fn eh_verdadeiro(b: &bool) -> bool { *b }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Federacao {
    pub versao: u32,
    pub nos: Vec<NoConfig>,
    /// Vazão usada nas coletas longas, por IP de saída (abaixo do teto, como os 95 de hoje).
    #[serde(default = "padrao_teto_coleta")]
    pub teto_coleta_rps: f64,
    /// Quantos req/s valem 1 de peso na partição.
    #[serde(default = "padrao_unidade")]
    pub unidade_peso_rps: f64,
    #[serde(default = "padrao_batimento")]
    pub batimento_s: u64,
    #[serde(default = "padrao_morte")]
    pub morte_s: u64,
    #[serde(default = "padrao_pausa")]
    pub pausa_429_s: u64,
}

impl Federacao {
    pub fn de_json(texto: &str) -> Result<Federacao> {
        let f: Federacao = serde_json::from_str(texto).context("federacao.json inválido")?;
        f.validar()?;
        Ok(f)
    }

    pub fn ler(caminho: &std::path::Path) -> Result<Federacao> {
        let t = std::fs::read_to_string(caminho).with_context(|| format!("lendo {}", caminho.display()))?;
        Federacao::de_json(&t)
    }

    pub fn no(&self, id: &str) -> Option<&NoConfig> { self.nos.iter().find(|n| n.id == id) }

    pub fn validar(&self) -> Result<()> {
        if self.versao != 1 { bail!("federacao.json: versão {} não suportada", self.versao); }
        if self.nos.is_empty() { bail!("federacao.json: nenhum nó"); }
        let (mut ids, mut orgaos) = (BTreeSet::new(), BTreeSet::new());
        for n in &self.nos {
            if n.id.is_empty() || n.id.len() > 64 || !n.id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
                bail!("nó '{}': id deve ter 1 a 64 caracteres [A-Za-z0-9_-]", n.id);
            }
            if !ids.insert(n.id.as_str()) { bail!("nó '{}' repetido", n.id); }
            let orgao = n.orgao.trim().to_lowercase();
            if orgao.is_empty() { bail!("nó '{}': órgão vazio", n.id); }
            // um nó **que coleta** por órgão; outros servidores do mesmo órgão só podem receber ("coleta": false)
            if n.coleta && !orgaos.insert(orgao) { bail!("órgão '{}' com mais de um nó que coleta (um nó por órgão; os outros com \"coleta\": false)", n.orgao); }
            if n.ip_saida.parse::<std::net::IpAddr>().is_err() { bail!("nó '{}': ip_saida '{}' inválido", n.id, n.ip_saida); }
            if !(n.teto_rps > 0.0) { bail!("nó '{}': teto_rps deve ser positivo", n.id); }
            if n.teto_rps > LIMITE_POR_ORGAO_RPS && n.acordo.as_deref().map_or(true, |a| a.trim().is_empty()) {
                bail!("nó '{}': teto de {} req/s acima de {} só com autorização registrada (campo 'acordo')",
                    n.id, n.teto_rps, LIMITE_POR_ORGAO_RPS);
            }
            if let Some(e) = &n.endereco {
                if e.parse::<std::net::SocketAddr>().is_err() && !e.contains(':') { bail!("nó '{}': endereco '{e}' deve ser host:porta", n.id); }
            }
            if self.teto_coleta_rps > n.teto_rps { bail!("teto_coleta_rps ({}) acima do teto do nó '{}' ({})", self.teto_coleta_rps, n.id, n.teto_rps); }
        }
        if !(self.teto_coleta_rps > 0.0) { bail!("teto_coleta_rps deve ser positivo"); }
        if !(self.unidade_peso_rps > 0.0) { bail!("unidade_peso_rps deve ser positiva"); }
        if self.batimento_s == 0 || self.morte_s <= self.batimento_s { bail!("morte_s ({}) deve ser maior que batimento_s ({})", self.morte_s, self.batimento_s); }
        Ok(())
    }
}

#[cfg(test)]
mod testes;
