//! Relógio do sistema (M-003): os horários do Urna Velox são os do relógio da máquina — a data e a hora reais em que
//! ele totalizou —, separados dos horários do TSE (o `Date` do site, o horário de recebimento dos BUs, a geração dos
//! arquivos de divulgação).
//!
//! Para comparar as duas apurações, a **âncora** liga as duas escalas: no primeiro horário visto do site (`noite0`,
//! horário do TSE) o relógio do sistema marcava `sis0`; com a aceleração `acelerar` do simulado (1 na noite real), um
//! instante `t` do TSE equivale a `sis0 + (t − noite0) / acelerar` no relógio do sistema, e um instante do sistema
//! equivale a `noite0 + (s − sis0) · acelerar` na noite do TSE. Na noite real (aceleração 1) a âncora só desconta a
//! diferença entre os dois relógios (segundos).
//!
//! A previsão (`crate::previsao`) e a curva (`crate::difusao`) andam pelo relógio do sistema; a previsão faz as contas na
//! escala da noite (o modelo e os gabaritos são de uma noite de apuração) e converte para o relógio do sistema ao
//! publicar.

use serde_json::{json, Value};
use std::sync::Mutex;

/// Agora, em milissegundos na hora de Brasília (escala de `urna_velox::metricas::segundos` × 1000).
pub fn agora_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0) - 3 * 3600 * 1000
}

/// "2026-10-18T20:41:03.512-03:00" de um instante em ms (hora de Brasília).
pub fn iso_ms(ms: i64) -> String { urna_velox::metricas::iso_brasilia_ms(ms) }

/// "2026-10-18T20:41:03.512-03:00" (ou sem os milissegundos, ou "2026-10-18 20:41:03") → ms; None se não for horário.
pub fn ms_iso(s: &str) -> Option<i64> {
    let b = s.as_bytes();
    if b.len() < 19 { return None; }
    let seg = urna_velox::metricas::segundos(&format!("{}/{}/{} {}", s.get(8..10)?, s.get(5..7)?, s.get(0..4)?, s.get(11..19)?))?;
    let frac = if b.get(19) == Some(&b'.') { s.get(20..23).and_then(|x| x.parse::<i64>().ok()).unwrap_or(0) } else { 0 };
    Some(seg * 1000 + frac)
}

/// A ligação entre a noite do TSE e o relógio do sistema.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ancora {
    /// instante da noite (TSE, segundos) que corresponde a `sis0_ms`
    pub noite0: f64,
    /// instante do sistema (ms, hora de Brasília)
    pub sis0_ms: i64,
    /// segundos da noite por segundo do sistema (1 na noite real; 500 no modo rápido)
    pub acelerar: f64,
}

impl Ancora {
    /// Instante do sistema (ms) → instante equivalente da noite (s).
    pub fn noite(&self, sis_ms: i64) -> f64 { self.noite0 + (sis_ms - self.sis0_ms) as f64 / 1000.0 * self.acelerar }
    /// Instante da noite (s) → instante equivalente no relógio do sistema (ms).
    pub fn sistema_ms(&self, noite: f64) -> i64 { self.sis0_ms + ((noite - self.noite0) / self.acelerar * 1000.0).round() as i64 }
    /// Instante da noite (s) → instante equivalente no relógio do sistema (s).
    pub fn sistema(&self, noite: i64) -> i64 { self.sistema_ms(noite as f64).div_euclid(1000) }
}

/// Relógio do sistema do serviço, com a âncora (ver o módulo).
pub struct Relogio { ancora: Mutex<Option<Ancora>>, acelerar: Mutex<f64> }

impl Relogio {
    pub fn novo(acelerar: f64) -> Relogio { Relogio { ancora: Mutex::new(None), acelerar: Mutex::new(acelerar.max(1.0)) } }

    /// Um horário do site (TSE ou simulador, segundos): o primeiro fixa a âncora.
    pub fn ver_site(&self, noite: i64) { self.ver_site_em(noite, agora_ms()) }
    pub fn ver_site_em(&self, noite: i64, sis_ms: i64) {
        if noite <= 0 { return; }
        let mut a = self.ancora.lock().unwrap();
        if a.is_none() { *a = Some(Ancora { noite0: noite as f64, sis0_ms: sis_ms, acelerar: *self.acelerar.lock().unwrap() }); }
    }

    /// A velocidade do simulado mudou (controle do replay): a âncora continua do ponto de agora, na nova velocidade.
    pub fn mudar_aceleracao(&self, x: f64) { self.mudar_aceleracao_em(x, agora_ms()) }
    pub fn mudar_aceleracao_em(&self, x: f64, sis_ms: i64) {
        let x = x.max(1.0);
        *self.acelerar.lock().unwrap() = x;
        let mut a = self.ancora.lock().unwrap();
        if let Some(an) = a.as_mut() { *an = Ancora { noite0: an.noite(sis_ms), sis0_ms: sis_ms, acelerar: x }; }
    }

    /// O simulado voltou no tempo (controle do replay): a âncora é refeita no próximo horário do site.
    pub fn reiniciar(&self) { *self.ancora.lock().unwrap() = None; }

    pub fn ancora(&self) -> Option<Ancora> { *self.ancora.lock().unwrap() }

    /// Agora, na escala da noite (s), pelo relógio do sistema; None antes do primeiro horário do site.
    pub fn noite_agora(&self) -> Option<i64> { self.ancora().map(|a| a.noite(agora_ms()).floor() as i64) }

    /// Bloco `relogio_sistema` da mensagem `estado` (`docs/arquitetura/PROTOCOLO_WS.md` §4.3).
    pub fn json(&self) -> Value {
        let a = self.ancora();
        json!({"agora": iso_ms(agora_ms()), "noite0": a.map(|a| urna_velox::metricas::iso_brasilia(a.noite0.floor() as i64)),
            "sistema0": a.map(|a| iso_ms(a.sis0_ms)), "acelerar": a.map_or(*self.acelerar.lock().unwrap(), |a| a.acelerar)})
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn iso_ida_e_volta() {
        let t = urna_velox::metricas::segundos("18/10/2026 20:41:03").unwrap() * 1000 + 512;
        assert_eq!(iso_ms(t), "2026-10-18T20:41:03.512-03:00");
        assert_eq!(ms_iso(&iso_ms(t)), Some(t));
        assert_eq!(ms_iso("2026-10-18T20:41:03-03:00"), Some(t - 512));
        assert_eq!(ms_iso("2026-10-18 20:41:03"), Some(t - 512));
        assert_eq!(ms_iso("—"), None);
    }

    #[test]
    fn ancora_converte_nos_dois_sentidos() {
        // simulado começa hoje às 07:00:00 (sistema) reproduzindo a noite de 06/10/2024 a partir das 17:00:00, a 500×
        let s0 = urna_velox::metricas::segundos("18/10/2026 07:00:00").unwrap() * 1000;
        let n0 = urna_velox::metricas::segundos("06/10/2024 17:00:00").unwrap();
        let r = Relogio::novo(500.0);
        r.ver_site_em(0, s0 - 5000);
        assert!(r.ancora().is_none(), "sem horário do site, sem âncora");
        r.ver_site_em(n0, s0);
        r.ver_site_em(n0 + 999, s0 + 1);
        let a = r.ancora().unwrap();
        assert_eq!(a, Ancora { noite0: n0 as f64, sis0_ms: s0, acelerar: 500.0 });
        // 10 h da noite do TSE (17:00 → 03:00) = 72 s do sistema
        assert_eq!(a.sistema(n0 + 36_000), s0 / 1000 + 72);
        assert_eq!(a.noite(s0 + 72_000), (n0 + 36_000) as f64);
        // a velocidade cai para 100× aos 72 s: o instante da noite continua o mesmo e o resto anda 5× mais devagar
        r.mudar_aceleracao_em(100.0, s0 + 72_000);
        let b = r.ancora().unwrap();
        assert_eq!(b.noite(s0 + 72_000), (n0 + 36_000) as f64);
        assert_eq!(b.noite(s0 + 73_000), (n0 + 36_100) as f64);
        // na noite real, a âncora só desconta a diferença de relógio
        let real = Relogio::novo(1.0);
        real.ver_site_em(n0, n0 * 1000 + 2_000);
        assert_eq!(real.ancora().unwrap().sistema(n0 + 600), n0 + 602);
        r.reiniciar();
        assert!(r.ancora().is_none());
    }
}
