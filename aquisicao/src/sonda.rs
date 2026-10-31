//! **Sonda** — o padrão *Test Message* (Hohpe & Woolf, *Enterprise Integration Patterns*, cap. 11) no Urna Velox.
//!
//! A saúde (`saude.json`, *ping* do WebSocket, *heartbeat* da federação) diz que cada processo está vivo; não diz que o
//! **caminho dos BUs** anda. A sonda é uma mensagem de teste que o próprio serviço injeta a cada `--sonda-intervalo`
//! segundos no mesmo canal dos BUs e que percorre o caminho real, na mesma fila:
//!
//! ```text
//! injetor (uv-sonda-injetor) → canal do motor (FIFO, atrás dos BUs) → motor (depois do lote e da publicação)
//!   → thread de gravação (depois dos arquivos do mesmo lote) → difusão → WebSocket (`op: sonda`) → verificador
//!   └→ federação: lote do nó (registro "sonda") → consumidor do par → motor do par → … (eco no nó par)
//! ```
//!
//! As quatro peças do padrão:
//! * **gerador / injetor**: a thread `uv-sonda` do `uv-servico` ([`Sonda::nova`], `servico::executar`);
//! * **separador**: a sonda nunca entra no estado, no diário, nos pacotes nem na publicação; na difusão só vai para
//!   quem pede `{"op":"sonda"}` (a página não pede; `docs/arquitetura/PROTOCOLO_WS.md` §4.9);
//! * **verificador**: o [`Vigia`] dentro do serviço (sonda que não volta → alerta `sonda_atrasada`, bloco `sonda` da
//!   `saude.json`) e o [`Verificador`] do `uv-sonda`, cliente WebSocket de fora (o caminho inteiro, até a rede).
//!
//! A sonda **não faz nenhuma requisição ao TSE** (nasce dentro do serviço) e não muda a totalização: ver o teste
//! `sonda_nao_muda_a_publicacao` e `docs/arquitetura/SONDA.md`.
//!
//! Tudo aqui é lógica pura, com o tempo passado de fora (µs Unix), para os testes.

use crate::servico::Amostra;
use serde_json::{json, Value};
use std::collections::{BTreeMap, VecDeque};

/// Uma mensagem de teste.
#[derive(Clone, Debug, PartialEq)]
pub struct Sonda {
    /// sequencial dentro da sessão do injetor (1, 2, 3…)
    pub id: u64,
    /// identifica a execução do injetor (µs Unix da partida): o `id` recomeça quando o serviço reinicia
    pub sessao: u64,
    /// nó da federação que injetou; `None` = este serviço
    pub origem: Option<String>,
    /// µs Unix no relógio de quem injetou
    pub t_injecao: u64,
    /// sonda de um par: quando o consumidor deste nó a tirou do lote (µs, relógio deste nó)
    pub t_recebida: Option<u64>,
    /// quando a thread do motor a passou adiante (depois do lote e da publicação que estavam na frente)
    pub t_motor: Option<u64>,
    /// quando a thread de gravação a passou adiante (depois de gravar os arquivos do mesmo lote)
    pub t_gravacao: Option<u64>,
    /// BUs aplicados quando passou pelo motor
    pub seq: u64,
}

fn ms(de: u64, ate: Option<u64>) -> Value {
    match ate { Some(a) => json!(((a as f64 - de as f64) / 1e3 * 1000.0).round() / 1000.0), None => Value::Null }
}

impl Sonda {
    pub fn nova(id: u64, sessao: u64, t_injecao: u64) -> Sonda {
        Sonda { id, sessao, origem: None, t_injecao, t_recebida: None, t_motor: None, t_gravacao: None, seq: 0 }
    }
    /// Sonda que veio de um par da federação (já com o instante da chegada a este nó).
    pub fn remota(origem: &str, id: u64, sessao: u64, t_injecao: u64, t_recebida: u64) -> Sonda {
        Sonda { origem: Some(origem.to_string()), t_recebida: Some(t_recebida), ..Sonda::nova(id, sessao, t_injecao) }
    }
    pub fn local(&self) -> bool { self.origem.is_none() }
    /// Início do trecho medido neste nó: a injeção (local) ou a chegada do lote do par (remota).
    pub fn inicio(&self) -> u64 { self.t_recebida.unwrap_or(self.t_injecao) }
    /// Corpo da mensagem `sonda` (sem `tipo`, `t_difusao` e `t_envio`, que a difusão acrescenta). Os `ms` contam a partir
    /// do [`Sonda::inicio`], no relógio deste nó; `federacao_ms` (só remota) cruza os relógios dos dois nós.
    pub fn json(&self) -> Value {
        let i = self.inicio();
        let mut v = json!({"id": self.id, "sessao": self.sessao, "origem": self.origem, "local": self.local(), "t_injecao": self.t_injecao,
            "t_motor": self.t_motor, "t_gravacao": self.t_gravacao, "seq": self.seq,
            "ms": {"motor": ms(i, self.t_motor), "gravacao": ms(i, self.t_gravacao)}});
        if let Some(r) = self.t_recebida {
            v["t_recebida"] = json!(r);
            v["ms"]["federacao"] = ms(self.t_injecao, Some(r));
        }
        v
    }
    /// Lê o corpo de uma mensagem `sonda` (o inverso de [`Sonda::json`]; os campos da difusão são ignorados).
    pub fn de_json(v: &Value) -> Option<Sonda> {
        Some(Sonda {
            id: v["id"].as_u64()?, sessao: v["sessao"].as_u64().unwrap_or(0), origem: v["origem"].as_str().map(String::from),
            t_injecao: v["t_injecao"].as_u64()?, t_recebida: v["t_recebida"].as_u64(), t_motor: v["t_motor"].as_u64(),
            t_gravacao: v["t_gravacao"].as_u64(), seq: v["seq"].as_u64().unwrap_or(0),
        })
    }
}

/// Mudança do estado do [`Vigia`], para virar alerta ou registro no `servico.log`.
#[derive(Clone, Debug, PartialEq)]
pub enum Transicao {
    /// a sonda mais antiga ainda não voltou depois do limite: o caminho dos BUs parou (ou está muito atrasado)
    Atrasou { atraso_ms: f64, pendentes: usize, id: u64 },
    /// as sondas voltaram a passar
    Normalizou { duracao_ms: f64 },
}

/// Sondas pendentes guardadas no máximo (com o motor parado, uma por intervalo; além disso só conta).
const MAX_PENDENTES: usize = 10_000;

/// O que se sabe das sondas de um par da federação.
#[derive(Default, Clone, Debug)]
struct Remota { sessao: u64, ultima: u64, recebidas: u64, puladas: u64, repetidas: u64, reinicios: u64, t_ultima: u64, federacao_ms: Option<f64>, total_ms: Option<f64> }

/// Verificador dentro do serviço: acompanha as sondas injetadas até a thread de gravação devolvê-las.
pub struct Vigia {
    pub intervalo_us: u64,
    pub limite_us: u64,
    injetadas: u64,
    /// (id, t_injecao) das injetadas que ainda não voltaram, da mais antiga para a mais nova
    pendentes: VecDeque<(u64, u64)>,
    excedentes: u64,
    concluidas: u64,
    perdidas: u64,
    ultima: Option<Sonda>,
    t_ultima: Option<u64>,
    atrasada: Option<u64>,
    episodios: u64,
    lat_motor: Amostra,
    lat_gravacao: Amostra,
    lat_total: Amostra,
    remotas: BTreeMap<String, Remota>,
}

impl Vigia {
    /// `limite_s` = 0: o dobro do intervalo (no mínimo 5 s).
    pub fn novo(intervalo_s: f64, limite_s: f64) -> Vigia {
        let intervalo_us = (intervalo_s.max(0.0) * 1e6) as u64;
        let limite_us = if limite_s > 0.0 { (limite_s * 1e6) as u64 } else { (2 * intervalo_us).max(5_000_000) };
        Vigia { intervalo_us, limite_us, injetadas: 0, pendentes: VecDeque::new(), excedentes: 0, concluidas: 0, perdidas: 0, ultima: None,
            t_ultima: None, atrasada: None, episodios: 0, lat_motor: Amostra::com_janela(1024), lat_gravacao: Amostra::com_janela(1024),
            lat_total: Amostra::com_janela(1024), remotas: BTreeMap::new() }
    }

    pub fn injetar(&mut self, id: u64, t: u64) {
        self.injetadas += 1;
        if self.pendentes.len() < MAX_PENDENTES { self.pendentes.push_back((id, t)); } else { self.excedentes += 1; }
    }

    /// A thread de gravação devolveu a sonda `s` em `agora`. As locais tiram da fila as pendentes até o `id` dela (o
    /// caminho é FIFO: uma mais antiga que não voltou se perdeu); as dos pares só atualizam o par.
    pub fn concluir(&mut self, s: &Sonda, agora: u64) {
        let i = s.inicio();
        let d = |a: Option<u64>| a.map(|a| a.saturating_sub(i) as f64 / 1e3);
        let total = agora.saturating_sub(i) as f64 / 1e3;
        let Some(origem) = &s.origem else {
            while let Some(&(id, _)) = self.pendentes.front() {
                if id > s.id { break; }
                self.pendentes.pop_front();
                if id < s.id { self.perdidas += 1; }
            }
            self.concluidas += 1;
            if let Some(x) = d(s.t_motor) { self.lat_motor.por(x); }
            if let Some(x) = d(s.t_gravacao) { self.lat_gravacao.por(x); }
            self.lat_total.por(total);
            self.ultima = Some(s.clone());
            self.t_ultima = Some(agora);
            return;
        };
        let r = self.remotas.entry(origem.clone()).or_default();
        if r.recebidas > 0 && r.sessao != s.sessao { r.reinicios += 1; r.ultima = 0; }
        if r.recebidas > 0 && r.sessao == s.sessao && s.id <= r.ultima { r.repetidas += 1; return; }
        if r.ultima > 0 && s.id > r.ultima + 1 { r.puladas += s.id - r.ultima - 1; }
        r.sessao = s.sessao;
        r.ultima = s.id;
        r.recebidas += 1;
        r.t_ultima = agora;
        r.federacao_ms = s.t_recebida.map(|x| (x as f64 - s.t_injecao as f64) / 1e3);
        r.total_ms = Some(total);
    }

    /// Idade da sonda pendente mais antiga (0 sem pendentes).
    pub fn atraso_us(&self, agora: u64) -> u64 { self.pendentes.front().map_or(0, |&(_, t)| agora.saturating_sub(t)) }

    /// Confere o atraso e devolve a mudança de estado, se houver (uma por episódio).
    pub fn verificar(&mut self, agora: u64) -> Option<Transicao> {
        let atraso = self.atraso_us(agora);
        match self.atrasada {
            None if atraso > self.limite_us => {
                self.atrasada = Some(agora);
                self.episodios += 1;
                let id = self.pendentes.front().map_or(0, |x| x.0);
                Some(Transicao::Atrasou { atraso_ms: atraso as f64 / 1e3, pendentes: self.pendentes.len() + self.excedentes as usize, id })
            }
            Some(desde) if atraso <= self.limite_us => {
                self.atrasada = None;
                Some(Transicao::Normalizou { duracao_ms: agora.saturating_sub(desde) as f64 / 1e3 })
            }
            _ => None,
        }
    }

    pub fn atrasada(&self) -> bool { self.atrasada.is_some() }

    /// Bloco `sonda` da `saude.json`.
    pub fn json(&self, agora: u64) -> Value {
        let lim_par = 2 * self.limite_us;
        let remotas: serde_json::Map<String, Value> = self.remotas.iter().map(|(n, r)| {
            let idade = agora.saturating_sub(r.t_ultima);
            (n.clone(), json!({"recebidas": r.recebidas, "ultima": r.ultima, "puladas": r.puladas, "repetidas": r.repetidas, "reinicios": r.reinicios,
                "idade_s": (idade as f64 / 1e5).round() / 10.0, "atrasada": idade > lim_par,
                "federacao_ms": r.federacao_ms.map(|x| (x * 1000.0).round() / 1000.0), "total_ms": r.total_ms.map(|x| (x * 1000.0).round() / 1000.0)}))
        }).collect();
        json!({
            "intervalo_s": self.intervalo_us as f64 / 1e6, "limite_s": self.limite_us as f64 / 1e6,
            "injetadas": self.injetadas, "concluidas": self.concluidas, "pendentes": self.pendentes.len() as u64 + self.excedentes,
            "perdidas": self.perdidas, "atrasada": self.atrasada(), "episodios_atraso": self.episodios,
            "atraso_s": (self.atraso_us(agora) as f64 / 1e5).round() / 10.0,
            "idade_ultima_s": self.t_ultima.map(|t| (agora.saturating_sub(t) as f64 / 1e5).round() / 10.0),
            "ultima": self.ultima.as_ref().map(Sonda::json),
            "latencias_ms": {"motor": self.lat_motor.json(), "gravacao": self.lat_gravacao.json(), "total": self.lat_total.json()},
            "remotas": remotas,
        })
    }
}

// ------------------------------------------------------------------ verificador de fora (uv-sonda)

/// Situação de um alvo no [`Verificador`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Situacao {
    /// conectado, sondas chegando dentro do limite
    Ok,
    /// conectado há menos que o limite e nenhuma sonda ainda
    Esperando,
    /// conectado, mas a última sonda (ou a conexão, se nenhuma veio) tem mais que o limite: o caminho parou
    Atrasada,
    Desconectado,
}

impl Situacao {
    pub fn texto(self) -> &'static str {
        match self { Situacao::Ok => "ok", Situacao::Esperando => "esperando", Situacao::Atrasada => "atrasada", Situacao::Desconectado => "desconectado" }
    }
}

/// Um servidor acompanhado pelo verificador.
#[derive(Default)]
struct Alvo {
    conectado: bool,
    /// início da contagem de idade: conexão (ou a última sonda nova)
    t_ref: u64,
    t_ultima: Option<u64>,
    recebidas: u64,
    sessao: u64,
    ultima: u64,
    puladas: u64,
    repetidas: u64,
    reinicios: u64,
    conexoes: u64,
    quedas: u64,
    ultima_queda: Option<String>,
    /// t_difusao − t_injecao (mesmo relógio, o do servidor)
    lat_servidor: Amostra,
    /// chegada aqui − t_injecao (relógios diferentes: só vale com os dois sincronizados)
    lat_ponta: Amostra,
    ultima_msg: Option<Value>,
    /// sondas dos pares da federação vistas por este servidor: origem → (recebidas, última id)
    remotas: BTreeMap<String, (u64, u64)>,
}

/// Verificador de fora: recebe as mensagens `sonda` de um ou mais servidores e diz, de cada um, se o caminho anda.
pub struct Verificador {
    pub limite_us: u64,
    alvos: BTreeMap<String, Alvo>,
}

impl Verificador {
    pub fn novo(limite_s: f64, alvos: &[String]) -> Verificador {
        let alvos = alvos.iter().map(|a| (a.clone(), Alvo { lat_servidor: Amostra::com_janela(1024), lat_ponta: Amostra::com_janela(1024), ..Alvo::default() })).collect();
        Verificador { limite_us: (limite_s.max(0.001) * 1e6) as u64, alvos }
    }

    fn alvo(&mut self, a: &str) -> &mut Alvo {
        self.alvos.entry(a.to_string()).or_insert_with(|| Alvo { lat_servidor: Amostra::com_janela(1024), lat_ponta: Amostra::com_janela(1024), ..Alvo::default() })
    }

    pub fn conectou(&mut self, a: &str, agora: u64) {
        let x = self.alvo(a);
        x.conectado = true;
        x.conexoes += 1;
        x.t_ref = agora;
    }

    pub fn caiu(&mut self, a: &str, motivo: &str) {
        let x = self.alvo(a);
        if x.conectado { x.quedas += 1; }
        x.conectado = false;
        x.ultima_queda = Some(motivo.to_string());
    }

    /// Uma mensagem recebida do alvo. Só as `sonda` contam; a de inscrição (`instantaneo: true`, a última que o servidor
    /// tem, possivelmente antiga) só serve de referência para a contagem dos `id`, não como prova de que o caminho anda.
    /// Devolve `true` se era uma sonda nova deste servidor.
    pub fn receber(&mut self, a: &str, m: &Value, agora: u64) -> bool {
        if m["tipo"] != "sonda" { return false; }
        let Some(s) = Sonda::de_json(m) else { return false };
        let inst = m["instantaneo"].as_bool().unwrap_or(false);
        let x = self.alvo(a);
        if let Some(o) = &s.origem {
            let e = x.remotas.entry(o.clone()).or_default();
            if !inst { e.0 += 1; }
            e.1 = s.id;
            return false;
        }
        let mesma = x.sessao == s.sessao && (x.recebidas > 0 || x.ultima > 0);
        if mesma && s.id <= x.ultima { if !inst { x.repetidas += 1; } return false; }
        if x.ultima > 0 && x.sessao != s.sessao { x.reinicios += 1; }
        if mesma && s.id > x.ultima + 1 && !inst { x.puladas += s.id - x.ultima - 1; }
        x.sessao = s.sessao;
        x.ultima = s.id;
        x.ultima_msg = Some(m.clone());
        if inst { return false; }
        x.recebidas += 1;
        x.t_ultima = Some(agora);
        x.t_ref = agora;
        if let Some(td) = m["t_difusao"].as_u64() { x.lat_servidor.por(td.saturating_sub(s.t_injecao) as f64 / 1e3); }
        x.lat_ponta.por((agora as f64 - s.t_injecao as f64) / 1e3);
        true
    }

    pub fn situacao(&self, a: &str, agora: u64) -> Situacao {
        let Some(x) = self.alvos.get(a) else { return Situacao::Desconectado };
        if !x.conectado { return Situacao::Desconectado; }
        let idade = agora.saturating_sub(x.t_ref);
        if idade <= self.limite_us { if x.t_ultima.is_some() { Situacao::Ok } else { Situacao::Esperando } } else { Situacao::Atrasada }
    }

    /// Todos os alvos com sondas chegando (para o código de saída do `uv-sonda`).
    pub fn tudo_ok(&self, agora: u64) -> bool { self.alvos.keys().all(|a| self.situacao(a, agora) == Situacao::Ok) }

    pub fn recebidas(&self, a: &str) -> u64 { self.alvos.get(a).map_or(0, |x| x.recebidas) }

    /// Relatório (o arquivo `--saida` do `uv-sonda`, lido pelo painel dos nós).
    pub fn json(&self, agora: u64) -> Value {
        let alvos: serde_json::Map<String, Value> = self.alvos.iter().map(|(n, x)| {
            let sit = self.situacao(n, agora);
            (n.clone(), json!({"situacao": sit.texto(), "conectado": x.conectado, "recebidas": x.recebidas, "ultima": x.ultima, "sessao": x.sessao,
                "puladas": x.puladas, "repetidas": x.repetidas, "reinicios": x.reinicios, "conexoes": x.conexoes, "quedas": x.quedas,
                "ultima_queda": x.ultima_queda, "idade_s": x.t_ultima.map(|t| (agora.saturating_sub(t) as f64 / 1e5).round() / 10.0),
                "latencias_ms": {"servidor": x.lat_servidor.json(), "ponta": x.lat_ponta.json()},
                "remotas": x.remotas.iter().map(|(o, (n, u))| (o.clone(), json!({"recebidas": n, "ultima": u}))).collect::<serde_json::Map<_, _>>(),
                "ultima_msg": x.ultima_msg}))
        }).collect();
        json!({"t": agora, "limite_s": self.limite_us as f64 / 1e6, "ok": self.tudo_ok(agora), "alvos": alvos})
    }
}

#[cfg(test)]
mod testes;
