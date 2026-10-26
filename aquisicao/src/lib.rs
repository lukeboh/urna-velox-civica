//! **uv-aquisicao** — a parte do Urna Velox que fala com a rede (o motor `urna-velox` não faz rede, ADR-012/014).
//!
//! Binários:
//! * `uv-coletor` (`src/main.rs`): coleta, acompanhamento, preparo dos candidatos, replay e utilitários;
//! * `uv-servico` (`src/bin/uv-servico.rs`, módulo [`servico`]): o serviço da noite — acompanha o site, coleta,
//!   totaliza em fluxo, publica, retoma depois de uma queda e confere com os oficiais no fim;
//! * `uv-simulador` (`src/bin/uv-simulador.rs`, módulo [`simulador`]): site de resultados simulado, local, com relógio
//!   acelerado e injeção de falhas, para provar o serviço sem baixar BUs do TSE;
//! * `uv-carga` (`src/bin/uv-carga.rs`): cliente de carga e de conferência da difusão em tempo real do serviço
//!   (`--http`, módulo [`difusao`], `docs/PROTOCOLO_WS.md`);
//! * `uv-painel` (`src/bin/uv-painel.rs`, módulo [`painel`]): painel dos nós — desempenho, federação e configuração,
//!   com login do Google (`docs/PAINEL.md`).

pub mod coletor;
pub mod difusao;
pub mod controle_replay;
pub mod federacao;
pub mod mapa;
pub mod marcos;
pub mod pagina;
pub mod painel;
pub mod preparo;
pub mod previsao;
pub mod relogio;
pub mod replay;
pub mod servico;
pub mod simulador;
pub mod trava;
pub mod tse;
pub mod tse_resultados;
pub mod tse_simulado;
pub mod zip;

/// Data HTTP (`Date`, RFC 7231: "Sun, 27 Oct 2024 20:01:02 GMT") → segundos na hora de Brasília (UTC−3), na mesma
/// escala de `urna_velox::metricas::segundos`.
pub fn data_http_brasilia(s: &str) -> Option<i64> {
    let p: Vec<&str> = s.split_whitespace().collect();
    if p.len() < 5 { return None; }
    let dia: i64 = p[1].parse().ok()?;
    let mes = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"].iter().position(|m| *m == p[2])? as i64 + 1;
    let ano: i64 = p[3].parse().ok()?;
    let t = urna_velox::metricas::segundos(&format!("{dia:02}/{mes:02}/{ano:04} {}", p[4]))?;
    Some(t - 3 * 3600)
}

/// Segundos na hora de Brasília (escala de `urna_velox::metricas::segundos`) → data HTTP em GMT.
pub fn data_http_de_brasilia(t: i64) -> String {
    let g = t + 3 * 3600;
    let (a, m, d, h, mi, s) = urna_velox::metricas::civil(g);
    // dia da semana: 1970-01-01 foi quinta-feira
    let dsem = ["Thu", "Fri", "Sat", "Sun", "Mon", "Tue", "Wed"][g.div_euclid(86400).rem_euclid(7) as usize];
    let mes = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"][(m - 1) as usize];
    format!("{dsem}, {d:02} {mes} {a:04} {h:02}:{mi:02}:{s:02} GMT")
}

/// Agora, na hora de Brasília (escala de `urna_velox::metricas::segundos`), pelo relógio do sistema.
pub fn agora_brasilia() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0) - 3 * 3600
}

/// "dd/mm/aaaa hh:mm:ss" de um instante (escala de `urna_velox::metricas::segundos`).
pub fn texto_hora(t: i64) -> String {
    let (a, m, d, h, mi, s) = urna_velox::metricas::civil(t);
    format!("{d:02}/{m:02}/{a:04} {h:02}:{mi:02}:{s:02}")
}

#[cfg(test)]
mod testes {
    use super::*;
    #[test]
    fn datas_http() {
        let t = urna_velox::metricas::segundos("27/10/2024 17:01:02").unwrap();
        let h = data_http_de_brasilia(t);
        assert_eq!(h, "Sun, 27 Oct 2024 20:01:02 GMT");
        assert_eq!(data_http_brasilia(&h), Some(t));
        assert_eq!(texto_hora(t), "27/10/2024 17:01:02");
        assert!(agora_brasilia() > urna_velox::metricas::segundos("01/01/2026 00:00:00").unwrap());
    }
}
