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
mod testes {
    use super::*;

    fn cfg() -> Federacao {
        Federacao::de_json(r#"{"versao":1,"nos":[
            {"id":"a","orgao":"A","ip_saida":"10.0.0.1"},
            {"id":"b","orgao":"B","ip_saida":"10.0.0.2"},
            {"id":"c","orgao":"C","ip_saida":"10.0.0.2"},
            {"id":"d","orgao":"D","ip_saida":"10.0.0.4"}]}"#).unwrap()
    }

    fn bat(no: &str, ip: &str, ha_s: u64, agora: SystemTime) -> BatimentoLido {
        BatimentoLido {
            batimento: Batimento { no: no.into(), ip_saida_medido: ip.into(), pausa_ate: 0, seq_bus: 0, versao: String::new(), contador: 0, sincronizando: false },
            recebido: agora - Duration::from_secs(ha_s),
        }
    }

    #[test]
    fn grupos_por_ip_e_pesos() {
        let agora = SystemTime::UNIX_EPOCH + Duration::from_secs(2_000_000_000);
        let bs = [bat("a", "10.0.0.1", 3, agora), bat("b", "10.0.0.2", 5, agora), bat("c", "10.0.0.2", 1, agora), bat("d", "10.0.0.4", 9, agora)];
        let e = calcular(&cfg(), &bs, agora);
        assert!(e.fora.is_empty(), "{:?}", e.fora);
        let a = e.ativo("a").unwrap();
        assert_eq!((a.nos_no_ip, a.rps, a.peso), (1, 95.0, 19));
        let b = e.ativo("b").unwrap();
        assert_eq!((b.nos_no_ip, b.rps, b.peso), (2, 47.5, 9));
        assert_eq!(e.ativo("c").unwrap().peso, 9);
        assert_eq!(e.rps_total(), 95.0 * 3.0);
    }

    #[test]
    fn mortos_pausados_divergentes_e_desconhecidos_ficam_fora() {
        let agora = SystemTime::UNIX_EPOCH + Duration::from_secs(2_000_000_000);
        let mut pausado = bat("b", "10.0.0.2", 2, agora);
        pausado.batimento.pausa_ate = 2_000_000_000 + 300;
        let bs = [
            bat("a", "10.0.0.1", 31, agora),       // morto (> 30 s)
            pausado,                                 // em pausa por 429
            bat("c", "200.0.0.9", 2, agora),         // saiu por outro IP (NAT não declarado)
            bat("x", "10.0.0.7", 1, agora),          // não está na federação
            // d nunca publicou
        ];
        let e = calcular(&cfg(), &bs, agora);
        assert!(e.ativos.is_empty());
        assert!(e.particao.vazia());
        let motivos: Vec<(&str, &Motivo)> = e.fora.iter().map(|(i, m)| (i.as_str(), m)).collect();
        assert!(motivos.contains(&("a", &Motivo::Morto)));
        assert!(motivos.contains(&("d", &Motivo::Morto)));
        assert!(motivos.contains(&("b", &Motivo::EmPausa)));
        assert!(motivos.contains(&("x", &Motivo::Desconhecido)));
        assert!(motivos.contains(&("c", &Motivo::IpDivergente { declarado: "10.0.0.2".into(), medido: "200.0.0.9".into() })));
    }

    #[test]
    fn pausa_de_um_ip_redistribui_e_o_outro_no_do_ip_ganha_capacidade() {
        let agora = SystemTime::UNIX_EPOCH + Duration::from_secs(2_000_000_000);
        // c em pausa: b fica sozinho no IP 10.0.0.2 e volta a ter os 95 req/s do IP
        let mut c = bat("c", "10.0.0.2", 1, agora);
        c.batimento.pausa_ate = 2_000_000_000 + 600;
        let e = calcular(&cfg(), &[bat("a", "10.0.0.1", 1, agora), bat("b", "10.0.0.2", 1, agora), c], agora);
        assert_eq!(e.ativo("b").unwrap().rps, 95.0);
        assert!(e.ativo("c").is_none());
        // nenhuma seção fica com quem está fora
        for m in 0..2000u32 { assert_ne!(e.particao.dono(m, 1, 1), Some("c")); }
    }

    #[test]
    fn no_que_so_recebe_fica_fora_da_particao() {
        let agora = SystemTime::UNIX_EPOCH + Duration::from_secs(2_000_000_000);
        let mut c = cfg();
        c.nos[3].coleta = false;
        let e = calcular(&c, &[bat("a", "10.0.0.1", 1, agora), bat("d", "10.0.0.4", 1, agora)], agora);
        assert!(e.ativo("d").is_none());
        assert!(e.fora.contains(&("d".to_string(), Motivo::SoRecebe)));
        for m in 0..2000u32 { assert_eq!(e.particao.dono(m, 1, 1), Some("a")); }
    }

    #[test]
    fn no_sincronizando_fica_fora_ate_alcancar_os_pares() {
        let agora = SystemTime::UNIX_EPOCH + Duration::from_secs(2_000_000_000);
        let mut d = bat("d", "10.0.0.4", 1, agora);
        d.batimento.sincronizando = true;
        let e = calcular(&cfg(), &[bat("a", "10.0.0.1", 1, agora), d.clone()], agora);
        assert!(e.ativo("d").is_none());
        assert!(e.fora.contains(&("d".to_string(), Motivo::Sincronizando)));
        for m in 0..2000u32 { assert_eq!(e.particao.dono(m, 1, 1), Some("a")); }
        // alcançou: volta à partição
        d.batimento.sincronizando = false;
        assert!(calcular(&cfg(), &[bat("a", "10.0.0.1", 1, agora), d], agora).ativo("d").is_some());
        // o campo é opcional no JSON (heartbeats de versões anteriores)
        let j = r#"{"no":"a","ip_saida_medido":"10.0.0.1"}"#;
        assert!(!serde_json::from_str::<Batimento>(j).unwrap().sincronizando);
    }

    #[test]
    fn heartbeat_mais_recente_vale() {
        let agora = SystemTime::UNIX_EPOCH + Duration::from_secs(2_000_000_000);
        let mut velho = bat("a", "10.0.0.1", 20, agora);
        velho.batimento.pausa_ate = 2_000_000_000 + 600;
        let e = calcular(&cfg(), &[velho, bat("a", "10.0.0.1", 2, agora)], agora);
        assert!(e.ativo("a").is_some());
    }
}
