//! Amostras do nó para o painel: a cada `intervalo` lê a `saude.json` do `uv-servico` e o `/proc` (CPU, memória,
//! carga, disco, processos) e guarda uma série curta em memória (o painel não grava nada no disco do nó além da
//! auditoria). As taxas (BUs/s, BUs trocados/s) saem da diferença entre duas amostras.
//!
//! Lê só arquivos: não abre conexão com o `uv-servico` nem tira nada dele (ver `docs/PAINEL.md`).

use serde_json::{json, Map, Value};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};

/// Campos de cada amostra, na ordem das colunas devolvidas à página.
pub const COLUNAS: &[&str] = &[
    "bus_s", "baixados_s", "req_s", "teto_req_s", "http429", "fila", "pendentes", "aplicadas",
    "rss_mb", "cpu_pct", "sis_cpu_pct", "sis_mem_usada_mb", "sis_mem_total_mb", "carga1", "disco_livre_gb",
    "fed_recebidos_s", "fed_publicados_s", "fed_lotes_s", "fed_servidas_s", "fed_buscas_erro", "ws_clientes", "ws_msgs_s", "ws_kb_s",
    "idade_saude_s",
];

/// Contadores (crescem; a taxa é a diferença por segundo) → coluna da taxa.
const TAXAS: &[(&str, &str)] = &[
    ("aplicadas", "bus_s"), ("bu_baixados", "baixados_s"), ("fed_recebidos", "fed_recebidos_s"),
    ("fed_publicados", "fed_publicados_s"), ("fed_lotes", "fed_lotes_s"), ("fed_servidas", "fed_servidas_s"),
];

pub struct Amostrador {
    /// candidatas: a `saude.json` do serviço de produção e a do ciclo de simulações; vale a gravada por último
    saudes: Vec<PathBuf>,
    disco: PathBuf,
    pub serie: VecDeque<(u64, Vec<Option<f64>>)>,
    cap: usize,
    ant_cont: Option<(u64, Map<String, Value>)>,
    ant_cpu: Option<(u64, u64)>,
    /// a última `saude.json` lida inteira (para o detalhe na página)
    pub saude_atual: Option<Value>,
    pub sistema_atual: Value,
}

fn num(v: &Value) -> Option<f64> { v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse().ok())) }

/// Saída do serviço do ciclo diário de simulações (`uv-ciclo-no.service`, docs/CICLO_DIARIO.md).
pub const SAIDA_CICLO: &str = "/srv/urna-velox/ciclo/atual/saida";

pub fn agora_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

impl Amostrador {
    pub fn novo(saida: &Path, cap: usize) -> Amostrador {
        Amostrador { saudes: vec![saida.join("saude.json"), PathBuf::from(SAIDA_CICLO).join("saude.json")], disco: saida.to_path_buf(),
            serie: VecDeque::with_capacity(cap), cap,
            ant_cont: None, ant_cpu: None, saude_atual: None, sistema_atual: json!({}) }
    }

    /// Uma amostra agora (`t` em ms).
    pub fn amostrar(&mut self, t: u64) {
        let saude = self.saude_atual_caminho();
        let txt = std::fs::read_to_string(&saude).ok();
        let idade = std::fs::metadata(&saude).ok().and_then(|m| m.modified().ok())
            .and_then(|m| std::time::SystemTime::now().duration_since(m).ok()).map(|d| d.as_secs_f64());
        let s = txt.and_then(|t| serde_json::from_str::<Value>(&t).ok());
        let sis = sistema(&self.disco, &mut self.ant_cpu);
        let linha = self.registrar(t, s.as_ref(), &sis, idade);
        self.serie.push_back((t, linha));
        while self.serie.len() > self.cap { self.serie.pop_front(); }
        self.saude_atual = s;
        self.sistema_atual = sis;
    }

    /// A `saude.json` gravada por último entre as candidatas (produção ou ciclo de simulações).
    pub fn saude_atual_caminho(&self) -> PathBuf {
        let mtime = |p: &PathBuf| std::fs::metadata(p).and_then(|m| m.modified()).ok();
        self.saudes.iter().max_by_key(|p| mtime(p)).cloned().unwrap_or_else(|| self.saudes[0].clone())
    }

    /// Monta a linha da amostra a partir da saude.json e do sistema (separado para os testes).
    pub fn registrar(&mut self, t: u64, s: Option<&Value>, sis: &Value, idade_saude: Option<f64>) -> Vec<Option<f64>> {
        let mut v: Map<String, Value> = Map::new();
        let mut por = |k: &str, x: Option<f64>| { if let Some(x) = x { v.insert(k.into(), json!(x)); } };
        if let Some(s) = s {
            por("aplicadas", num(&s["secoes"]["aplicadas"]));
            por("bu_baixados", num(&s["rede"]["bu_baixados"]));
            por("req_s", num(&s["rede"]["req_s_atual"]));
            por("teto_req_s", num(&s["rede"]["teto_req_s"]));
            por("http429", num(&s["rede"]["http429"]));
            por("fila", num(&s["fila"]));
            por("pendentes", num(&s["secoes"]["pendentes"]));
            por("rss_mb", num(&s["recursos"]["rss_mb"]));
            por("cpu_pct", num(&s["recursos"]["cpu_pct_atual"]));
            let f = &s["federacao"];
            por("fed_recebidos", num(&f["recebidos"]));
            por("fed_publicados", num(&f["publicados"]));
            por("fed_lotes", num(&f["sincronia"]["lotes_copiados"]));
            por("fed_servidas", num(&f["sincronia"]["servidas"]));
            por("fed_buscas_erro", num(&f["sincronia"]["buscas_erro"]));
            let d = &s["difusao"];
            por("ws_clientes", num(&d["clientes"]));
            por("ws_msgs_s", num(&d["mensagens_s"]));
            por("ws_kb_s", num(&d["bytes_s"]).map(|b| (b / 102.4).round() / 10.0));
        }
        por("sis_cpu_pct", num(&sis["cpu_pct"]));
        por("sis_mem_usada_mb", num(&sis["memoria_mb"]["usada"]));
        por("sis_mem_total_mb", num(&sis["memoria_mb"]["total"]));
        por("carga1", num(&sis["carga"][0]));
        por("disco_livre_gb", num(&sis["disco_livre_gb"]));
        por("idade_saude_s", idade_saude.map(|x| (x * 10.0).round() / 10.0));
        // taxas: diferença dos contadores sobre o intervalo (contador que voltou = o serviço reiniciou: taxa sem valor)
        if let Some((t0, ant)) = &self.ant_cont {
            let dt = (t.saturating_sub(*t0)) as f64 / 1000.0;
            for (cont, taxa) in TAXAS {
                if let (Some(a), Some(b)) = (ant.get(*cont).and_then(num), v.get(*cont).and_then(num)) {
                    if dt > 0.0 && b >= a { v.insert((*taxa).into(), json!(((b - a) / dt * 10.0).round() / 10.0)); }
                }
            }
        }
        self.ant_cont = Some((t, v.clone()));
        COLUNAS.iter().map(|c| v.get(*c).and_then(num)).collect()
    }

    /// Série a partir de `desde` (ms), em colunas: `{"t": [...], "bus_s": [...], ...}`.
    pub fn colunas(&self, desde: u64) -> Value {
        let it: Vec<&(u64, Vec<Option<f64>>)> = self.serie.iter().filter(|(t, _)| *t > desde).collect();
        let mut o = Map::new();
        o.insert("t".into(), json!(it.iter().map(|(t, _)| *t).collect::<Vec<_>>()));
        for (i, c) in COLUNAS.iter().enumerate() {
            o.insert((*c).into(), json!(it.iter().map(|(_, l)| l[i]).collect::<Vec<_>>()));
        }
        Value::Object(o)
    }

    /// Resumo do estado atual (o que a página mostra nos cartões).
    pub fn resumo(&self) -> Value {
        let ult = self.serie.back().map(|(t, l)| {
            let m: Map<String, Value> = COLUNAS.iter().zip(l).map(|(c, x)| ((*c).to_string(), json!(x))).collect();
            json!({"t": t, "valores": m})
        });
        let s = self.saude_atual.as_ref();
        let g = |p: &[&str]| -> Value {
            let mut v = match s { Some(v) => v, None => return Value::Null };
            for k in p { v = &v[*k]; }
            v.clone()
        };
        let alertas = g(&["alertas"]);
        let alertas = match alertas { Value::Array(a) => json!(a.iter().rev().take(8).cloned().collect::<Vec<_>>()), o => o };
        json!({
            "ultima": ult,
            "servico": {"versao": g(&["versao"]), "estado": g(&["estado"]), "agora": g(&["agora"]), "ativo_s": g(&["ativo_s"]),
                "pleito": g(&["pleito"]), "base": g(&["base"]), "relogio_site": g(&["relogio_site"]),
                "secoes": g(&["secoes"]), "ultimo_bu": g(&["ultimo_bu"]), "apuracao": g(&["apuracao"]),
                "matematicamente": g(&["matematicamente"]), "motor": g(&["motor"]), "simulacao": g(&["simulacao"]),
                "pausa_429_s": g(&["rede", "pausa_restante_s"]), "retomada_em": g(&["rede", "retomada_em"]),
                "rede": g(&["rede"]), "latencias_ms": g(&["latencias_ms"])},
            "federacao": g(&["federacao"]),
            "alertas": alertas,
            "sistema": self.sistema_atual,
        })
    }
}

// ------------------------------------------------------------------ sistema (/proc)

fn ler(p: &str) -> Option<String> { std::fs::read_to_string(p).ok() }

fn cpu_total() -> Option<(u64, u64)> {
    let l = ler("/proc/stat")?;
    let v: Vec<u64> = l.lines().next()?.split_whitespace().skip(1).filter_map(|x| x.parse().ok()).collect();
    if v.len() < 4 { return None; }
    Some((v.iter().sum(), v[3] + v.get(4).copied().unwrap_or(0)))
}

fn memoria() -> Value {
    let mut m = Map::new();
    for l in ler("/proc/meminfo").unwrap_or_default().lines() {
        let (k, v) = l.split_once(':').unwrap_or(("", ""));
        let mb = v.split_whitespace().next().and_then(|x| x.parse::<u64>().ok()).map(|k| k / 1024);
        if let Some(mb) = mb {
            match k { "MemTotal" => { m.insert("total".into(), json!(mb)); } "MemAvailable" => { m.insert("disponivel".into(), json!(mb)); }
                "SwapTotal" => { m.insert("swap_total".into(), json!(mb)); } "SwapFree" => { m.insert("swap_livre".into(), json!(mb)); } _ => {} }
        }
    }
    if let (Some(t), Some(d)) = (m.get("total").and_then(Value::as_u64), m.get("disponivel").and_then(Value::as_u64)) {
        m.insert("usada".into(), json!(t.saturating_sub(d)));
    }
    Value::Object(m)
}

/// Processos que interessam (RSS e pico em MB, CPU acumulada em s, threads).
fn processos() -> Value {
    let mut r = Map::new();
    let tick = unsafe { libc::sysconf(libc::_SC_CLK_TCK) }.max(1) as f64;
    let Ok(rd) = std::fs::read_dir("/proc") else { return Value::Object(r) };
    for e in rd.flatten() {
        let pid = e.file_name().to_string_lossy().into_owned();
        if !pid.bytes().all(|b| b.is_ascii_digit()) { continue; }
        let nome = ler(&format!("/proc/{pid}/comm")).unwrap_or_default().trim().to_string();
        if !matches!(nome.as_str(), "uv-servico" | "uv-simulador" | "uv-painel" | "caddy") { continue; }
        let st = ler(&format!("/proc/{pid}/status")).unwrap_or_default();
        let campo = |k: &str| st.lines().find(|l| l.starts_with(k)).and_then(|l| l.split_whitespace().nth(1)).and_then(|x| x.parse::<u64>().ok());
        let s = ler(&format!("/proc/{pid}/stat")).unwrap_or_default();
        let s: Vec<&str> = s.rsplit_once(')').map(|x| x.1).unwrap_or("").split_whitespace().collect();
        let cpu = (s.len() > 12).then(|| (s[11].parse::<u64>().unwrap_or(0) + s[12].parse::<u64>().unwrap_or(0)) as f64 / tick);
        r.insert(nome, json!({"pid": pid.parse::<u64>().unwrap_or(0), "rss_mb": campo("VmRSS:").map(|k| k / 1024),
            "pico_rss_mb": campo("VmHWM:").map(|k| k / 1024), "cpu_s": cpu, "threads": campo("Threads:")}));
    }
    Value::Object(r)
}

fn disco_livre_gb(p: &Path) -> Option<f64> {
    let c = std::ffi::CString::new(p.as_os_str().to_string_lossy().as_bytes()).ok()?;
    let mut s: libc::statvfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statvfs(c.as_ptr(), &mut s) } != 0 { return None; }
    Some(((s.f_bavail as f64 * s.f_frsize as f64) / 1e9 * 100.0).round() / 100.0)
}

fn sistema(disco: &Path, ant: &mut Option<(u64, u64)>) -> Value {
    let cpu = cpu_total();
    let pct = match (cpu, *ant) {
        (Some((t, o)), Some((t0, o0))) if t > t0 => Some(((100.0 * ((t - t0) - (o - o0).min(t - t0)) as f64 / (t - t0) as f64) * 10.0).round() / 10.0),
        _ => None,
    };
    *ant = cpu;
    let carga: Vec<f64> = ler("/proc/loadavg").unwrap_or_default().split_whitespace().take(3).filter_map(|x| x.parse().ok()).collect();
    let ncpu = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
    json!({"cpu_pct": pct, "cpus": ncpu, "carga": carga, "memoria_mb": memoria(), "disco_livre_gb": disco_livre_gb(disco),
        "processos": processos(), "hostname": ler("/proc/sys/kernel/hostname").map(|h| h.trim().to_string()),
        "kernel": ler("/proc/sys/kernel/osrelease").map(|h| h.trim().to_string())})
}

#[cfg(test)]
mod testes {
    use super::*;

    fn saude(aplicadas: u64, baixados: u64, recebidos: u64) -> Value {
        json!({"versao": "0.8.33", "estado": "coletando", "secoes": {"aplicadas": aplicadas, "pendentes": 3}, "fila": 7,
            "rede": {"bu_baixados": baixados, "req_s_atual": 94.5, "teto_req_s": 95, "http429": 0},
            "recursos": {"rss_mb": 410, "cpu_pct_atual": 55.5},
            "federacao": {"recebidos": recebidos, "publicados": 10, "sincronia": {"lotes_copiados": 2, "servidas": 4, "buscas_erro": 0}},
            "difusao": {"clientes": 12, "mensagens_s": 3.5, "bytes_s": 2048},
            "alertas": [{"tipo": "a"}, {"tipo": "b"}]})
    }

    fn col(a: &Amostrador, c: &str) -> Vec<Value> { a.colunas(0)[c].as_array().unwrap().clone() }

    #[test]
    fn taxas_entre_amostras() {
        let mut a = Amostrador::novo(Path::new("/nao/existe"), 10);
        let sis = json!({"cpu_pct": 20.0, "memoria_mb": {"usada": 1000, "total": 24000}, "carga": [1.5], "disco_livre_gb": 80.0});
        for (t, s) in [(1000, saude(100, 90, 0)), (6000, saude(600, 590, 250)), (11000, saude(50, 40, 300))] {
            let l = a.registrar(t, Some(&s), &sis, Some(1.0));
            a.serie.push_back((t, l));
        }
        assert_eq!(col(&a, "bus_s"), vec![Value::Null, json!(100.0), Value::Null], "5 s, +500; depois o contador voltou (reinício)");
        assert_eq!(col(&a, "baixados_s"), vec![Value::Null, json!(100.0), Value::Null]);
        assert_eq!(col(&a, "fed_recebidos_s"), vec![Value::Null, json!(50.0), json!(10.0)]);
        assert_eq!(col(&a, "req_s")[1], json!(94.5));
        assert_eq!(col(&a, "ws_kb_s")[0], json!(2.0));
        assert_eq!(col(&a, "sis_mem_total_mb")[0], json!(24000.0));
        assert_eq!(a.colunas(6000)["t"], json!([11000]), "só depois de `desde`");
    }

    #[test]
    fn sem_saude_ainda_amostra_o_sistema() {
        let mut a = Amostrador::novo(Path::new("/nao/existe"), 3);
        for t in 0..5u64 { a.amostrar(t * 1000); }
        assert_eq!(a.serie.len(), 3, "capacidade");
        assert!(a.saude_atual.is_none());
        let r = a.resumo();
        assert!(r["servico"]["versao"].is_null());
        assert!(r["sistema"]["memoria_mb"]["total"].as_u64().unwrap_or(0) > 0 || !Path::new("/proc/meminfo").exists());
    }

    #[test]
    fn resumo_com_alertas_mais_recentes_primeiro() {
        let mut a = Amostrador::novo(Path::new("/nao/existe"), 3);
        a.saude_atual = Some(saude(1, 1, 1));
        assert_eq!(a.resumo()["alertas"], json!([{"tipo": "b"}, {"tipo": "a"}]));
        assert_eq!(a.resumo()["servico"]["versao"], "0.8.33");
    }
}
