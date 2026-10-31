//! Quem participa da partição agora, e com quanto (ADR-028 §2–§4).
//!
//! A partir do `federacao.json`, dos *heartbeats* lidos do storage e do relógio do storage:
//!
//! 1. descarta nós desconhecidos, mortos (sem *heartbeat* há mais de `morte_s`), em pausa por 429 e com IP de saída
//!    medido diferente do declarado (esses não coletam);
//! 2. agrupa os restantes pelo **IP de saída medido**: um grupo divide o `teto_coleta_rps` do IP igualmente;
//! 3. peso de cada nó = ⌊vazão / `unidade_peso_rps`⌋, no mínimo 1 — a partição fica proporcional à capacidade.
//!
//! Função pura: mesmos *heartbeats* e mesmo instante → mesmo resultado em qualquer nó.

use super::config::Federacao;
use super::particao::{Participante, Particao};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::time::{Duration, SystemTime};

/// *Heartbeat* de um nó, gravado em `nos/<id>.json` a cada `batimento_s`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Batimento {
    pub no: String,
    /// IP de saída medido pelo eco da federação (o que a CDN vê); vazio se a medida falhou.
    pub ip_saida_medido: String,
    /// Em pausa por HTTP 429 até este instante (segundos Unix, relógio do storage); 0 = sem pausa.
    #[serde(default)]
    pub pausa_ate: u64,
    /// Seq do último BU que o nó gravou no storage (diagnóstico).
    #[serde(default)]
    pub seq_bus: u64,
    /// Versão do Urna Velox do nó.
    #[serde(default)]
    pub versao: String,
    /// Sobe a cada batida: o conteúdo muda sempre, e a cópia que os pares guardam só envelhece se o nó parar.
    #[serde(default)]
    pub contador: u64,
    /// O nó acabou de (re)entrar e ainda está puxando dos pares os lotes que eles publicaram: fica fora da partição
    /// até alcançá-los, para não pedir ao TSE o que os outros coletaram enquanto ele estava fora (`2.0.0-alpha.10`).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub sincronizando: bool,
}

/// *Heartbeat* lido do storage, com o instante em que o storage o recebeu.
#[derive(Clone, Debug)]
pub struct BatimentoLido {
    pub batimento: Batimento,
    pub recebido: SystemTime,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Motivo {
    /// Não está no `federacao.json` (o *heartbeat* é ignorado).
    Desconhecido,
    /// Sem *heartbeat* há mais de `morte_s` (ou nunca publicou).
    Morto,
    /// Em pausa por HTTP 429 no seu IP.
    EmPausa,
    /// IP de saída medido diferente do declarado (ou não medido): não coleta.
    IpDivergente { declarado: String, medido: String },
    /// `"coleta": false` no `federacao.json`: só recebe dos pares (ex.: segundo servidor do mesmo órgão).
    SoRecebe,
    /// Acabou de (re)entrar e ainda está alcançando os lotes dos pares.
    Sincronizando,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NoAtivo {
    pub id: String,
    pub ip_saida: String,
    /// Nós que dividem este IP de saída (inclui este).
    pub nos_no_ip: usize,
    /// Vazão de coleta deste nó, req/s.
    pub rps: f64,
    pub peso: u32,
}

#[derive(Clone, Debug)]
pub struct EstadoFederacao {
    pub ativos: Vec<NoAtivo>,
    pub fora: Vec<(String, Motivo)>,
    pub particao: Particao,
}

impl EstadoFederacao {
    /// Vazão somada da federação (soma por IP de saída distinto).
    pub fn rps_total(&self) -> f64 { self.ativos.iter().map(|n| n.rps).sum() }
    pub fn ativo(&self, id: &str) -> Option<&NoAtivo> { self.ativos.iter().find(|n| n.id == id) }
}

fn segundos_unix(t: SystemTime) -> u64 {
    t.duration_since(SystemTime::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

pub fn calcular(cfg: &Federacao, batimentos: &[BatimentoLido], agora: SystemTime) -> EstadoFederacao {
    let mut fora: Vec<(String, Motivo)> = Vec::new();
    // último heartbeat de cada nó conhecido
    let mut ultimo: BTreeMap<&str, &BatimentoLido> = BTreeMap::new();
    for b in batimentos {
        if cfg.no(&b.batimento.no).is_none() {
            fora.push((b.batimento.no.clone(), Motivo::Desconhecido));
            continue;
        }
        let e = ultimo.entry(cfg.no(&b.batimento.no).unwrap().id.as_str()).or_insert(b);
        if b.recebido > e.recebido { *e = b; }
    }
    let morte = Duration::from_secs(cfg.morte_s);
    let agora_s = segundos_unix(agora);
    // nós aptos, agrupados por IP medido
    let mut por_ip: BTreeMap<String, Vec<&str>> = BTreeMap::new();
    for n in &cfg.nos {
        let Some(b) = ultimo.get(n.id.as_str()) else { fora.push((n.id.clone(), Motivo::Morto)); continue };
        let idade = agora.duration_since(b.recebido).unwrap_or(Duration::ZERO);
        if idade > morte { fora.push((n.id.clone(), Motivo::Morto)); continue; }
        if !n.coleta { fora.push((n.id.clone(), Motivo::SoRecebe)); continue; }
        if b.batimento.sincronizando { fora.push((n.id.clone(), Motivo::Sincronizando)); continue; }
        if b.batimento.pausa_ate > agora_s { fora.push((n.id.clone(), Motivo::EmPausa)); continue; }
        let medido = b.batimento.ip_saida_medido.trim();
        let ip_igual = match (medido.parse::<std::net::IpAddr>(), n.ip_saida.parse::<std::net::IpAddr>()) {
            (Ok(a), Ok(d)) => a == d,
            _ => false,
        };
        if !ip_igual {
            fora.push((n.id.clone(), Motivo::IpDivergente { declarado: n.ip_saida.clone(), medido: medido.to_string() }));
            continue;
        }
        por_ip.entry(medido.parse::<std::net::IpAddr>().unwrap().to_string()).or_default().push(n.id.as_str());
    }
    let mut ativos = Vec::new();
    for (ip, ids) in &por_ip {
        // o IP divide o menor teto entre os nós dele (o limite do IP é um só)
        let teto_ip = ids.iter().map(|id| cfg.no(id).unwrap().teto_rps).fold(f64::INFINITY, f64::min);
        let rps_ip = cfg.teto_coleta_rps.min(teto_ip);
        let rps = rps_ip / ids.len() as f64;
        let peso = ((rps / cfg.unidade_peso_rps).floor() as u32).max(1);
        for id in ids {
            ativos.push(NoAtivo { id: id.to_string(), ip_saida: ip.clone(), nos_no_ip: ids.len(), rps, peso });
        }
    }
    ativos.sort_by(|a, b| a.id.cmp(&b.id));
    fora.sort();
    let particao = Particao::nova(&ativos.iter().map(|n| Participante { id: n.id.clone(), peso: n.peso }).collect::<Vec<_>>());
    EstadoFederacao { ativos, fora, particao }
}

#[cfg(test)]
mod testes;
