//! "TSE simulado" do `uv-simulador` (etapa 7): os arquivos de resultado (`-u.json`) **progressivos**, como o site de
//! divulgação do TSE os publica durante a noite, para provar a comparação Urna Velox × TSE sem o TSE.
//!
//! Cada seção com BU entra na totalização do "TSE" com o atraso da divulgação oficial depois da disponibilização: um
//! atraso por seção, sorteado (semente fixa) de uma log-normal com mediana e p90 de `Opcoes` (padrão 120 s e 360 s, valores
//! ilustrativos). As seções não instaladas
//! contam desde o começo (o `aux.json` do simulador já as diz "Nao instalada"). Os arquivos são regenerados a cada
//! `passo` da noite: um pedido no instante `t` recebe o arquivo gerado em `⌊t / passo⌋ · passo`. Depois da última
//! divulgação, o arquivo é o oficial, byte a byte (a conferência desta etapa: no fim, TSE = oficial).
//!
//! O arquivo progressivo é o oficial final com os números trocados: votos e % de cada candidato (`vap`, `pvap`,
//! `pvapn`), ordem (`seq`), eleito/2º turno (`e`, do "matematicamente" calculado pelo motor do Urna Velox sobre os BUs
//! já divulgados — o `md` também), seções, eleitorado, comparecimento e votos (`s`, `e`, `v`), geração (`dg`, `hg`),
//! totalização em andamento (`tf` = "n", `and` = "p", sem situação final `st`). Os totais vêm do mesmo motor das
//! conferências (o `Publicador`), numa thread própria (`uv-tse-simulado`), que avança o estado conforme o relógio pede
//! e recomeça do zero se o relógio voltar (controle do replay).

use bytes::Bytes;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use urna_velox::acompanhamento::Acompanhamento;
use urna_velox::candidatos::Candidatos;
use urna_velox::catalogo::Catalogo;
use urna_velox::estado::{Agregador, Estado};
use urna_velox::oficial::{nome_arquivo, Abrangencia};
use urna_velox::pacote::EmMemoria;
use urna_velox::resultado::{Momento, Publicador};

pub type ChaveUf = ([u8; 2], u64);

/// Opções (`uv-simulador --tse-simulado <candidatos.json>`).
#[derive(Clone, Debug)]
pub struct Opcoes {
    pub candidatos: PathBuf,
    /// segundos da noite entre duas gerações dos arquivos
    pub passo_s: i64,
    /// mediana e p90 do atraso disponibilização → divulgação (s)
    pub p50_s: f64,
    pub p90_s: f64,
    pub semente: u64,
}
impl Opcoes {
    pub fn padrao(candidatos: PathBuf) -> Opcoes { Opcoes { candidatos, passo_s: 15, p50_s: 120.0, p90_s: 360.0, semente: 7 } }
}

/// Uma seção para o TSE simulado: chave, disponibilização (None = nunca), não instalada.
pub struct SecTse { pub k: ChaveUf, pub disp: Option<i64>, pub nao_instalada: bool }

struct Pedido { nome: String, t: i64, resp: crossbeam_channel::Sender<Option<(Bytes, String)>> }

/// O TSE simulado: uma thread que totaliza e gera os arquivos; [`TseSimulado::arquivo`] pede um.
pub struct TseSimulado {
    tx: crossbeam_channel::Sender<Pedido>,
    /// última divulgação: daí em diante, os oficiais byte a byte
    pub t_final: i64,
    /// atraso por seção (p50, p90, máx.), para o resumo
    pub atraso: (f64, f64, f64),
}

/// Atraso da seção: log-normal com a mediana e o p90 pedidos (z da normal por Box–Muller, semente fixa).
fn atraso(op: &Opcoes, k: &ChaveUf) -> f64 {
    let h = |x: u64| { let mut z = x.wrapping_add(0x9E3779B97F4A7C15); z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9); z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB); z ^ (z >> 31) };
    let base = h(op.semente ^ (u64::from(k.0[0]) << 56) ^ (u64::from(k.0[1]) << 48) ^ k.1);
    let u1 = ((h(base) >> 11) as f64 + 0.5) / (1u64 << 53) as f64;
    let u2 = ((h(base ^ 0x5555) >> 11) as f64) / (1u64 << 53) as f64;
    let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
    let sigma = (op.p90_s / op.p50_s).ln() / 1.281_551_565_5;
    (op.p50_s.ln() + sigma * z).exp().min(3600.0)
}

impl TseSimulado {
    /// `oficiais`: nome do arquivo → caminho do oficial final (o molde e o arquivo do fim da noite).
    pub fn iniciar(op: Opcoes, dados: PathBuf, ufs: Option<Vec<String>>, secs: Vec<SecTse>, oficiais: HashMap<String, PathBuf>) -> anyhow::Result<TseSimulado> {
        // divulgação de cada seção com BU: disponibilização + atraso
        let mut ev: Vec<(i64, ChaveUf)> = secs.iter().filter(|s| !s.nao_instalada).filter_map(|s| s.disp.map(|d| (d + atraso(&op, &s.k).round() as i64, s.k))).collect();
        ev.sort_unstable();
        let mut ats: Vec<f64> = secs.iter().filter(|s| s.disp.is_some() && !s.nao_instalada).map(|s| atraso(&op, &s.k)).collect();
        ats.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let q = |p: f64| ats.get(((ats.len() as f64 - 1.0) * p).round() as usize).copied().unwrap_or(0.0);
        let at = (q(0.5), q(0.9), ats.last().copied().unwrap_or(0.0));
        let t_final = ev.last().map(|x| x.0).unwrap_or(0);
        let ni: Vec<ChaveUf> = secs.iter().filter(|s| s.nao_instalada).map(|s| s.k).collect();
        let (tx, rx) = crossbeam_channel::unbounded::<Pedido>();
        let (ptx, prx) = std::sync::mpsc::channel::<anyhow::Result<()>>();
        std::thread::Builder::new().name("uv-tse-simulado".into()).spawn(move || {
            if let Err(e) = laco(op, dados, ufs, ev, ni, oficiais, rx, &ptx) { let _ = ptx.send(Err(e)); }
        })?;
        prx.recv().map_err(|_| anyhow::anyhow!("TSE simulado: a thread não partiu"))??;
        Ok(TseSimulado { tx, t_final, atraso: at })
    }

    /// O arquivo `nome` no instante `t` (corpo e `ETag`); None = não existe (nome desconhecido).
    pub fn arquivo(&self, nome: &str, t: i64) -> Option<(Bytes, String)> {
        let (rtx, rrx) = crossbeam_channel::bounded(1);
        self.tx.send(Pedido { nome: nome.to_string(), t, resp: rtx }).ok()?;
        rrx.recv().ok().flatten()
    }
}

#[allow(clippy::too_many_arguments)]
fn laco(op: Opcoes, dados: PathBuf, ufs: Option<Vec<String>>, ev: Vec<(i64, ChaveUf)>, ni: Vec<ChaveUf>, oficiais: HashMap<String, PathBuf>,
        rx: crossbeam_channel::Receiver<Pedido>, pronto: &std::sync::mpsc::Sender<anyhow::Result<()>>) -> anyhow::Result<()> {
    let (cad, arq) = Candidatos::ler(&op.candidatos)?;
    let cat = Catalogo::de_arquivo_candidatos(&arq)?;
    let mem = EmMemoria::carregar(&dados, ufs.as_deref())?;
    let mut bu_de: HashMap<ChaveUf, usize> = HashMap::new();
    for (i, it) in mem.itens.iter().enumerate() { bu_de.entry((it.0.uf, it.0.chave())).or_insert(i); }
    let t_final = ev.last().map(|x| x.0).unwrap_or(0);
    let _ = pronto.send(Ok(()));
    // moldes (o oficial final de cada arquivo), lidos na primeira vez
    let mut moldes: HashMap<String, (Value, Bytes)> = HashMap::new();
    let mut cache: HashMap<String, (i64, Bytes)> = HashMap::new();
    // pedido que chegou com o relógio de volta: atendido no estado recomeçado
    let mut pendente: Option<Pedido> = None;
    loop {
        // o estado do "TSE" (recomeça quando o relógio volta)
        let est = Estado::novo(&cat);
        let mut ag = Agregador::novo(&cat, &est);
        let mut pubr = Publicador::novo(&cat, Some(&cad), None);
        for &(uf, k) in &ni { pubr.nao_instalada(uf, k); }
        let mut ac = Acompanhamento::novo(&cat, &cad, None);
        let (mut i, mut g_atual, mut seq) = (0usize, i64::MIN, 0u64);
        cache.clear();
        let voltar = loop {
            let p = match pendente.take() { Some(p) => p, None => match rx.recv() { Ok(p) => p, Err(_) => return Ok(()) } };
            let Some(caminho) = oficiais.get(&p.nome) else { let _ = p.resp.send(None); continue };
            if !moldes.contains_key(&p.nome) {
                match std::fs::read(caminho) {
                    Ok(b) => match serde_json::from_slice::<Value>(&b) { Ok(v) => { moldes.insert(p.nome.clone(), (v, Bytes::from(b))); } Err(_) => { let _ = p.resp.send(None); continue } },
                    Err(_) => { let _ = p.resp.send(None); continue }
                }
            }
            if p.t >= t_final {
                let _ = p.resp.send(Some((moldes[&p.nome].1.clone(), "\"tse-final\"".into())));
                continue;
            }
            let g = p.t.div_euclid(op.passo_s) * op.passo_s;
            if g < g_atual { pendente = Some(p); break true; }
            if g > g_atual {
                // aplica as seções divulgadas até g
                let mut ultimo = None;
                while i < ev.len() && ev[i].0 <= g {
                    let k = ev[i].1;
                    if let Some(&j) = bu_de.get(&k) {
                        let (r, _, _, _) = &mem.itens[j];
                        est.aplicar(&cat, r.chave(), mem.bu(j), &mut ag);
                        if let Some(m) = cat.muni(r.municipio) { ac.bu_aplicado(m as usize); }
                        seq += 1;
                        ultimo = Some(ev[i].0);
                    }
                    i += 1;
                }
                if let Some(t) = ultimo { ac.atualizar(&est, seq, t); }
                g_atual = g;
            }
            if let Some((gc, b)) = cache.get(&p.nome) { if *gc == g { let _ = p.resp.send(Some((b.clone(), format!("\"tse-{g}\"")))); continue; } }
            let Some((abr, cargo, _)) = nome_arquivo(&p.nome) else { let _ = p.resp.send(None); continue };
            let mo = Momento { encerrada: false, seq, ultimo_bu: (seq > 0).then_some(g) };
            let corpo = pubr.valor(&est, Some(&ac), &abr, cargo, mo).map(|v| progressivo(&moldes[&p.nome].0, &v, &abr, g));
            match corpo {
                Some(c) => {
                    let b = Bytes::from(c.to_string());
                    cache.insert(p.nome.clone(), (g, b.clone()));
                    let _ = p.resp.send(Some((b, format!("\"tse-{g}\""))));
                }
                None => { let _ = p.resp.send(Some((moldes[&p.nome].1.clone(), "\"tse-molde\"".into()))); }
            }
        };
        if !voltar { return Ok(()); }
    }
}

fn p2(x: f64) -> String { format!("{:.2}", x).replace('.', ",") }
fn p9(x: f64) -> String { if x == 0.0 { "0".into() } else if (x - 100.0).abs() < 1e-12 { "100".into() } else { format!("{:.9}", x).replace('.', ",") } }
fn pc(a: u64, b: u64) -> f64 { if b == 0 { 0.0 } else { a as f64 * 100.0 / b as f64 } }
fn n(v: &Value, k: &str) -> u64 { v.get(k).and_then(|x| x.as_str().and_then(|s| s.parse().ok()).or_else(|| x.as_u64())).unwrap_or(0) }

/// Põe os números da publicação `p` (formato do Urna Velox) no molde (o oficial final), como um arquivo do TSE gerado
/// no instante `g` com a totalização em andamento.
fn progressivo(molde: &Value, p: &Value, abr: &Abrangencia, g: i64) -> Value {
    let mut o = molde.clone();
    let tx = crate::texto_hora(g);
    let (dg, hg) = tx.split_once(' ').unwrap_or(("", ""));
    // candidatos: votos, %, ordem e eleito/2º turno
    let estado = p["estado"].as_str().unwrap_or("apurando");
    let mut por_n: HashMap<u64, (u64, Option<f64>, usize, bool)> = HashMap::new();
    for (pos, c) in p["cand"].as_array().map(|a| a.as_slice()).unwrap_or(&[]).iter().enumerate() {
        let num = c["n"].as_u64().unwrap_or(0);
        por_n.insert(num, (c["v"].as_u64().unwrap_or(0), c["pct"].as_f64(), pos + 1, !c["mat"].is_null()));
    }
    fn visitar(v: &mut Value, f: &mut dyn FnMut(&mut serde_json::Map<String, Value>)) {
        match v {
            Value::Object(m) => {
                if let Some(Value::Array(cs)) = m.get_mut("cand") { for c in cs.iter_mut() { if let Value::Object(cm) = c { f(cm); } } }
                for (_, x) in m.iter_mut() { visitar(x, f); }
            }
            Value::Array(a) => for x in a { visitar(x, f) },
            _ => {}
        }
    }
    let decidido = estado == "matematicamente_eleito" || estado == "matematicamente_2t";
    visitar(&mut o["carg"], &mut |c| {
        let num = c.get("n").and_then(|x| x.as_str()).and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
        let (v, pct, pos, mat) = por_n.get(&num).copied().unwrap_or((0, None, 999, false));
        c.insert("vap".into(), json!(v.to_string()));
        c.insert("pvap".into(), json!(pct.map(p2).unwrap_or_default()));
        c.insert("pvapn".into(), json!(pct.map(p9).unwrap_or_default()));
        c.insert("seq".into(), json!(pos.to_string()));
        c.insert("e".into(), json!(if decidido && mat { "s" } else { "n" }));
        c.insert("st".into(), json!(""));
    });
    let m = o.as_object_mut().unwrap();
    m.insert("dg".into(), json!(dg));
    m.insert("hg".into(), json!(hg));
    m.insert("dt".into(), json!(dg));
    m.insert("ht".into(), json!(hg));
    m.insert("tf".into(), json!("n"));
    m.insert("and".into(), json!("p"));
    m.insert("md".into(), json!(match estado { "matematicamente_eleito" => "e", "matematicamente_2t" => "s", _ => "n" }));
    m.remove("esae");
    m.remove("mnae");
    let _ = abr;
    // seções: as não instaladas do molde contam desde o começo
    let ts = n(&o["s"], "ts");
    let sni = n(&o["s"], "sni");
    let st = p["secoes"]["apuradas"].as_u64().unwrap_or(0).min(ts);
    let si = st.saturating_sub(sni);
    let s = json!({"ts": ts.to_string(), "st": st.to_string(), "pst": p2(pc(st, ts)), "pstn": p9(pc(st, ts)),
        "snt": (ts - st).to_string(), "psnt": p2(pc(ts - st, ts)), "psntn": p9(pc(ts - st, ts)),
        "si": si.to_string(), "psi": p2(pc(si, ts)), "psin": p9(pc(si, ts)), "sni": sni.to_string(), "psni": p2(pc(sni, ts)), "psnin": p9(pc(sni, ts)),
        "sa": si.to_string(), "psa": p2(pc(si, si)), "psan": p9(pc(si, si)), "sna": "0", "psna": "0,00", "psnan": "0"});
    o["s"] = s;
    // eleitorado e comparecimento
    let te = n(&o["e"], "te");
    let esni = n(&o["e"], "esni");
    let esi = p["apurado"].as_u64().unwrap_or(0);
    let est_ = (esi + esni).min(te);
    let c = p["comparecimento"].as_u64().unwrap_or(0);
    let a = esi.saturating_sub(c);
    o["e"] = json!({"te": te.to_string(), "est": est_.to_string(), "pest": p2(pc(est_, te)), "pestn": p9(pc(est_, te)),
        "esnt": (te - est_).to_string(), "pesnt": p2(pc(te - est_, te)), "pesntn": p9(pc(te - est_, te)),
        "esi": esi.to_string(), "pesi": p2(pc(esi, te)), "pesin": p9(pc(esi, te)), "esni": esni.to_string(), "pesni": p2(pc(esni, te)), "pesnin": p9(pc(esni, te)),
        "esa": esi.to_string(), "pesa": p2(pc(esi, esi)), "pesan": p9(pc(esi, esi)), "esna": "0", "pesna": "0,00", "pesnan": "0",
        "c": c.to_string(), "pc": p2(pc(c, esi)), "pcn": p9(pc(c, esi)), "a": a.to_string(), "pa": p2(pc(a, esi)), "pan": p9(pc(a, esi))});
    // votos
    let (vb, vn, vnt, vv) = (p["vb"].as_u64().unwrap_or(0), p["vn"].as_u64().unwrap_or(0), p["vnt"].as_u64().unwrap_or(0), p["validos"].as_u64().unwrap_or(0));
    let leg = p["legenda"].as_u64().unwrap_or(0);
    let tv = vb + vn + vnt + vv;
    let tvn = vn + vnt;
    let vv_obj = o["v"].as_object_mut();
    if let Some(v) = vv_obj {
        let pares: Vec<(&str, String)> = vec![("tv", tv.to_string()), ("vvc", vv.to_string()), ("pvvc", p2(pc(vv, tv))), ("pvvcn", p9(pc(vv, tv))),
            ("vv", vv.to_string()), ("pvv", p2(pc(vv, vv))), ("pvvn", p9(pc(vv, vv))), ("vnom", (vv - leg.min(vv)).to_string()),
            ("pvnom", p2(pc(vv - leg.min(vv), vv))), ("pvnomn", p9(pc(vv - leg.min(vv), vv))),
            ("vb", vb.to_string()), ("pvb", p2(pc(vb, tv))), ("pvbn", p9(pc(vb, tv))), ("tvn", tvn.to_string()), ("ptvn", p2(pc(tvn, tv))), ("ptvnn", p9(pc(tvn, tv))),
            ("vn", vn.to_string()), ("pvn", p2(pc(vn, tvn))), ("pvnn", p9(pc(vn, tvn))), ("vnt", vnt.to_string()), ("pvnt", p2(pc(vnt, tvn))), ("pvntn", p9(pc(vnt, tvn)))];
        for (k, x) in pares { v.insert(k.into(), json!(x)); }
    }
    o
}

#[cfg(test)]
mod testes {
    use super::*;
    #[test]
    fn atraso_tem_a_mediana_e_o_p90_pedidos() {
        let op = Opcoes::padrao(PathBuf::new());
        let mut v: Vec<f64> = (0..20000u64).map(|i| atraso(&op, &(*b"sp", i * 7919))).collect();
        v.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let (p50, p90) = (v[10000], v[18000]);
        assert!((p50 - 120.0).abs() < 8.0, "p50 {p50}");
        assert!((p90 - 360.0).abs() < 20.0, "p90 {p90}");
        assert_eq!(atraso(&op, &(*b"am", 42)), atraso(&op, &(*b"am", 42)), "determinístico");
    }
    #[test]
    fn progressivo_troca_os_numeros_no_molde() {
        let molde: Value = serde_json::from_str(r#"{"ele":"620","t":"2","tf":"s","and":"f","esae":"n","dg":"29/09/2026","hg":"19:26:34",
            "carg":[{"cd":"11","agr":[{"par":[{"cand":[{"n":"70","seq":"1","e":"s","st":"Eleito","vap":"576171"},{"n":"22","seq":"2","e":"n","st":"Não eleito","vap":"479297"}]}]}]}],
            "s":{"ts":"10","st":"10","sni":"1"},"e":{"te":"1000","esni":"20","c":"800"},"v":{"tv":"800","vb":"10"}}"#).unwrap();
        let p = json!({"estado": "apurando", "secoes": {"total": 10, "apuradas": 5}, "apurado": 480, "comparecimento": 400, "vb": 5, "vn": 7, "vnt": 0, "validos": 388, "legenda": 0,
            "cand": [{"n": 22, "v": 200, "pct": 51.55, "mat": null}, {"n": 70, "v": 188, "pct": 48.45, "mat": null}]});
        let g = urna_velox::metricas::segundos("27/10/2024 18:00:15").unwrap();
        let o = progressivo(&molde, &p, &Abrangencia::Municipio(*b"am", 2550), g);
        let of = urna_velox::oficial::Oficial::de_json(&o);
        assert_eq!((of.geracao.as_str(), of.finalizada, of.andamento.as_str(), of.md.as_str()), ("27/10/2024 18:00:15", false, "p", "n"));
        assert_eq!((of.secoes, of.secoes_totalizadas, of.nao_instaladas, of.comparecimento, of.abstencao, of.vb, of.validos), (10, 5, 1, 400, 80, 5, 388));
        let c22 = of.cand.iter().find(|c| c.numero == 22).unwrap();
        let c70 = of.cand.iter().find(|c| c.numero == 70).unwrap();
        assert_eq!((c22.vap, c22.seq, c22.pvap, c22.eleito, c22.st.as_str()), (200, 1, Some(51.55), false, ""));
        assert_eq!((c70.vap, c70.seq), (188, 2));
        assert!(o.get("esae").is_none());
    }
}
