//! Endereços do site de resultados no painel: ver os caminhos que o nó usa, **descobrir** onde estão os arquivos a
//! partir de uma URL colada pelo operador e gravar os caminhos novos (`rotas_tse.json` + `BASE` do `no.env`).
//!
//! A descoberta só acontece quando o operador pede, faz no máximo [`MAX_REQ`] requisições, uma a cada
//! [`ESPACO_MS`] ms (≈ 3 req/s, que somadas às do `uv-servico` ficam abaixo dos 100 req/s do órgão), para na primeira
//! resposta 429 e não roda enquanto o serviço estiver em pausa por 429 (`CLAUDE.md`, "Rede e servidores do TSE").
//!
//! Passos: (1) acha a base procurando o `ele-c.json` em prefixos da URL; (2) escolhe o pleito; (3) lê o `cs.json` de
//! uma UF pequena; (4) numa seção com BU publicado, lê o `aux.json`, monta o endereço do BU e o baixa; (5) com
//! `base_arquivos`, baixa o mesmo BU também pela base e compara os bytes.

use crate::tse::{self, Alvo, Rotas};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub const MAX_REQ: usize = 20;
pub const ESPACO_MS: u64 = 350;

/// Arquivo de caminhos do nó (ao lado do `no.env`).
pub fn arquivo(no_env: &std::path::Path) -> std::path::PathBuf { no_env.with_file_name("rotas_tse.json") }

/// Caminhos gravados no nó (padrão se o arquivo não existe) e se o arquivo é válido.
pub fn ler(no_env: &std::path::Path) -> (Rotas, Option<String>) {
    let p = arquivo(no_env);
    if !p.exists() { return (Rotas::default(), None); }
    match Rotas::ler(&p) { Ok(r) => (r, None), Err(e) => (Rotas::default(), Some(e.to_string())) }
}

/// Exemplos de endereço para a página (uma seção qualquer de SP).
pub fn exemplos(base: &str, amb: &str, ciclo: &str, pleito: u32, r: &Rotas) -> Value {
    let a = Alvo { rotas: Arc::new(r.clone()), ..Alvo::novo(base, amb, ciclo, pleito) };
    json!({"ele_c": a.url_ele_c(), "cs": a.url_cs("sp"), "aux": a.url_aux("sp", 71072, 1, 9),
        "arquivo": a.url_arquivo("sp", 71072, 1, 9, "<hash>", &format!("o{pleito:05}sp7107200010009-bu.dat"))})
}

pub fn estado(no_env: &std::path::Path, saude: &Value) -> Value {
    let v = super::config_no::ler(&std::fs::read_to_string(no_env).unwrap_or_default());
    let g = |k: &str, d: &str| v.get(k).cloned().unwrap_or_else(|| d.to_string());
    let (base, amb, ciclo) = (g("BASE", "https://resultados.tse.jus.br"), g("AMBIENTE", "oficial"), g("CICLO", "ele2026"));
    let pleito: u32 = g("PLEITO", "0").parse().unwrap_or(0);
    let (r, erro) = ler(no_env);
    let no_servico = if saude["versao"].is_null() { Value::Null } else if saude["rotas"].is_null() { json!(Rotas::default()) } else { saude["rotas"].clone() };
    json!({"arquivo": arquivo(no_env), "existe": arquivo(no_env).exists(), "erro": erro, "padrao": r.padrao(), "rotas": r,
        "modelos_padrao": Rotas::default(), "base": base, "ambiente": amb, "ciclo": ciclo, "pleito": pleito,
        "base_servico": saude["base"], "rotas_servico": no_servico,
        "exemplos": exemplos(&base, &amb, &ciclo, pleito, &r)})
}

// ------------------------------------------------------------------ descoberta

struct Sonda {
    http: reqwest::Client,
    passos: Vec<Value>,
    n: usize,
    ult: Option<Instant>,
    parou: Option<String>,
}

impl Sonda {
    fn passo(&mut self, ok: bool, titulo: &str, detalhe: String, url: Option<&str>) {
        self.passos.push(json!({"ok": ok, "passo": titulo, "detalhe": detalhe, "url": url}));
    }

    /// GET espaçado; `None` = não foi feito (limite, 429 anterior) ou falhou na rede (anotado nos passos).
    async fn get(&mut self, titulo: &str, url: &str) -> Option<(u16, Vec<u8>)> {
        if self.parou.is_some() { return None; }
        if self.n >= MAX_REQ {
            self.parou = Some(format!("limite de {MAX_REQ} requisições da descoberta"));
            self.passo(false, titulo, self.parou.clone().unwrap(), Some(url));
            return None;
        }
        if let Some(t) = self.ult {
            let falta = Duration::from_millis(ESPACO_MS).saturating_sub(t.elapsed());
            if !falta.is_zero() { tokio::time::sleep(falta).await; }
        }
        self.n += 1;
        self.ult = Some(Instant::now());
        let t0 = Instant::now();
        let r = async {
            let resp = self.http.get(url).send().await?;
            let st = resp.status().as_u16();
            let b = resp.bytes().await?.to_vec();
            Ok::<_, reqwest::Error>((st, b))
        }.await;
        let ms = t0.elapsed().as_millis();
        match r {
            Ok((429, _)) => {
                self.parou = Some("HTTP 429: o servidor pediu para parar. A descoberta parou; espere pelo menos 10 minutos antes de tentar de novo".into());
                self.passo(false, titulo, format!("{} ({ms} ms)", self.parou.clone().unwrap()), Some(url));
                None
            }
            Ok((st, b)) => Some((st, b)),
            Err(e) => { self.passo(false, titulo, format!("erro de rede: {e}"), Some(url)); None }
        }
    }
}

/// Bases candidatas a partir da URL colada: se o caminho tem o segmento do ambiente (`oficial`, `simulado`), a base é o
/// que vem antes dele; senão, os prefixos do caminho (do mais longo ao mais curto, até 4) e a origem.
pub fn candidatas(url: &str, ambiente: &str) -> Result<(Vec<String>, String), String> {
    let u = url.trim();
    let (esq, resto) = u.split_once("://").ok_or("a URL tem de começar por http:// ou https://")?;
    if esq != "https" && esq != "http" { return Err("a URL tem de começar por http:// ou https://".into()); }
    let resto = resto.split(['?', '#']).next().unwrap_or("");
    let (host, caminho) = resto.split_once('/').unwrap_or((resto, ""));
    if host.is_empty() || !host.bytes().all(|b| b.is_ascii_alphanumeric() || b".-:[]".contains(&b)) { return Err("endereço do servidor inválido".into()); }
    let origem = format!("{esq}://{host}");
    let segs: Vec<&str> = caminho.split('/').filter(|s| !s.is_empty() && !s.contains('.') && *s != "app").collect();
    if segs.iter().any(|s| !s.bytes().all(|b| b.is_ascii_alphanumeric() || b"-_~".contains(&b))) { return Err("caminho da URL com caracteres inesperados".into()); }
    if let Some(k) = segs.iter().position(|s| *s == "oficial" || *s == "simulado") {
        let base = std::iter::once(origem.clone()).chain(segs[..k].iter().map(|s| s.to_string())).collect::<Vec<_>>().join("/");
        return Ok((vec![base], segs[k].to_string()));
    }
    let mut v: Vec<String> = (1..=segs.len().min(4)).rev().map(|n| format!("{origem}/{}", segs[..n].join("/"))).collect();
    v.push(origem);
    v.dedup();
    Ok((v, ambiente.to_string()))
}

pub struct Pedido {
    pub url: String,
    pub ambiente: String,
    pub ciclo: String,
    pub pleito: u32,
    pub rotas: Rotas,
}

/// Roda a descoberta e devolve os passos e a proposta (`BASE`, `AMBIENTE`, `CICLO`, `PLEITO`, rotas).
pub async fn descobrir(p: Pedido) -> Value {
    let http = match reqwest::Client::builder().user_agent(crate::coletor::USER_AGENT).timeout(Duration::from_secs(12))
        .connect_timeout(Duration::from_secs(5)).build() {
        Ok(h) => h,
        Err(e) => return json!({"ok": false, "erro": e.to_string()}),
    };
    let mut s = Sonda { http, passos: vec![], n: 0, ult: None, parou: None };
    let (cands, amb) = match candidatas(&p.url, &p.ambiente) {
        Ok(x) => x,
        Err(e) => return json!({"ok": false, "erro": e, "passos": []}),
    };

    // 1. base: o ele-c.json
    let mut achou: Option<(String, tse::EleC)> = None;
    for c in &cands {
        if let Err(e) = p.rotas.validar(c) { s.passo(false, "Caminhos", e.to_string(), None); break; }
        let a = Alvo { rotas: Arc::new(p.rotas.clone()), ..Alvo::novo(c, &amb, &p.ciclo, p.pleito) };
        let url = a.url_ele_c();
        match s.get("Lista de pleitos (ele-c.json)", &url).await {
            Some((200, b)) => match serde_json::from_slice::<tse::EleC>(&b) {
                Ok(e) => { s.passo(true, "Lista de pleitos (ele-c.json)", format!("achada: base {c}, ambiente {amb}, {} pleito(s)", e.pl.len()), Some(&url)); achou = Some((c.clone(), e)); break; }
                Err(e) => s.passo(false, "Lista de pleitos (ele-c.json)", format!("HTTP 200, mas não é a lista de pleitos: {e}"), Some(&url)),
            },
            Some((st, _)) => s.passo(false, "Lista de pleitos (ele-c.json)", format!("HTTP {st}"), Some(&url)),
            None => {}
        }
        if s.parou.is_some() { break; }
    }
    let Some((base, elec)) = achou else {
        return json!({"ok": false, "erro": "não achei a lista de pleitos (ele-c.json) a partir desta URL. Confira a URL ou ajuste o modelo ele_c",
            "passos": s.passos, "requisicoes": s.n});
    };
    let pleitos: Vec<Value> = elec.pl.iter().map(|x| json!({"cd": x.cd, "ciclo": x.c, "data": x.dt,
        "eleicoes": x.e.iter().map(|e| format!("{} {}", e.cd, e.nm)).collect::<Vec<_>>()})).collect();

    // 2. pleito e ciclo
    let alvo_pl = elec.pl.iter().find(|x| x.cd.trim_start_matches('0') == p.pleito.to_string());
    let (pleito, ciclo) = match alvo_pl {
        Some(x) => {
            s.passo(true, "Pleito", format!("{} de {} encontrado (ciclo {})", p.pleito, x.dt, if x.c.is_empty() { &p.ciclo } else { &x.c }), None);
            (p.pleito, if x.c.is_empty() { p.ciclo.clone() } else { x.c.clone() })
        }
        None => {
            s.passo(false, "Pleito", format!("o pleito {} não está na lista deste servidor (veja a lista abaixo)", p.pleito), None);
            return json!({"ok": false, "erro": format!("pleito {} não encontrado", p.pleito), "passos": s.passos, "pleitos": pleitos,
                "requisicoes": s.n, "proposta": {"BASE": base, "AMBIENTE": amb}});
        }
    };
    let alvo = Alvo { rotas: Arc::new(p.rotas.clone()), ..Alvo::novo(&base, &amb, &ciclo, pleito) };
    let proposta = json!({"BASE": base, "AMBIENTE": amb, "CICLO": ciclo, "PLEITO": pleito.to_string(), "rotas": p.rotas});

    // 3. seções de uma UF pequena
    let mut cs: Option<(String, tse::ConfigSecoes)> = None;
    for uf in ["rr", "ap", "ac"] {
        let url = alvo.url_cs(uf);
        match s.get("Seções da UF (cs.json)", &url).await {
            Some((200, b)) => match serde_json::from_slice::<tse::ConfigSecoes>(&b) {
                Ok(c) => {
                    let total = c.secoes(uf).len();
                    s.passo(true, "Seções da UF (cs.json)", format!("{}: {total} seções", uf.to_uppercase()), Some(&url));
                    cs = Some((uf.to_string(), c));
                    break;
                }
                Err(e) => s.passo(false, "Seções da UF (cs.json)", format!("HTTP 200, formato inesperado: {e}"), Some(&url)),
            },
            Some((st, _)) => { s.passo(false, "Seções da UF (cs.json)", format!("HTTP {st}"), Some(&url)); }
            None => {}
        }
        if s.parou.is_some() { break; }
    }
    let Some((uf, c)) = cs else {
        return json!({"ok": false, "erro": "achei o servidor, mas não a lista de seções (cs.json) no caminho do modelo cs", "passos": s.passos,
            "pleitos": pleitos, "proposta": proposta, "requisicoes": s.n});
    };

    // 4. um BU de verdade: seção com BU publicado (data de disponibilização preenchida, não agregada)
    let agregadas = c.agregadas();
    let com_bu: Vec<tse::Secao> = c.secoes(&uf).into_iter().filter(|x| !x.disponivel.trim().is_empty() && !agregadas.contains(&x.chave())).take(3).collect();
    if com_bu.is_empty() {
        s.passo(true, "Arquivo de urna (BU)", format!("nenhuma seção de {} com BU publicado ainda (antes da apuração é normal): a lista de seções está no lugar; para testar o caminho dos BUs agora, use um pleito já apurado", uf.to_uppercase()), None);
        return json!({"ok": true, "bu_ok": null, "passos": s.passos, "pleitos": pleitos, "proposta": proposta, "requisicoes": s.n});
    }
    let mut bu_ok = false;
    let mut exemplo: Value = Value::Null;
    for sec in &com_bu {
        let url = alvo.url_aux(&uf, sec.municipio, sec.zona, sec.secao);
        let Some((st, b)) = s.get("Arquivos da seção (aux.json)", &url).await else { break };
        if st != 200 { s.passo(false, "Arquivos da seção (aux.json)", format!("HTTP {st}"), Some(&url)); continue; }
        let Ok(aux) = serde_json::from_slice::<tse::Aux>(&b) else { s.passo(false, "Arquivos da seção (aux.json)", "formato inesperado".into(), Some(&url)); continue };
        let Some((hash, nome, situacao, _)) = aux.escolher_bu() else { s.passo(false, "Arquivos da seção (aux.json)", "sem BU nesta seção".into(), Some(&url)); continue };
        s.passo(true, "Arquivos da seção (aux.json)", format!("seção {}/{}/{}: {nome} ({situacao})", sec.municipio, sec.zona, sec.secao), Some(&url));
        let (hash, nome) = (hash.to_string(), nome.to_string());
        let url_bu = alvo.url_arquivo(&uf, sec.municipio, sec.zona, sec.secao, &hash, &nome);
        let Some((st, bu)) = s.get("Arquivo de urna (BU)", &url_bu).await else { break };
        let valido = st == 200 && bu.len() > 64 && bu[0] == 0x30;
        let sha = crate::painel::seguranca::hex(&Sha256::digest(&bu));
        s.passo(valido, "Arquivo de urna (BU)", if valido { format!("{} bytes, envelope BER, SHA-256 {}…", bu.len(), &sha[..16]) }
            else { format!("HTTP {st}, {} bytes: não parece um BU", bu.len()) }, Some(&url_bu));
        exemplo = json!({"aux": url, "bu": url_bu, "bytes": bu.len(), "sha256": sha});
        if !valido { break; }
        bu_ok = true;
        // 5. outra base para os arquivos: o mesmo BU pela base principal tem de ser idêntico
        if p.rotas.base_arquivos.is_some() {
            let mut r0 = p.rotas.clone();
            r0.base_arquivos = None;
            let a0 = Alvo { rotas: Arc::new(r0), ..alvo.clone() };
            let url0 = a0.url_arquivo(&uf, sec.municipio, sec.zona, sec.secao, &hash, &nome);
            if url0 != url_bu {
                match s.get("Mesmo BU pela base principal", &url0).await {
                    Some((200, b0)) => {
                        let igual = b0 == bu;
                        s.passo(igual, "Mesmo BU pela base principal", if igual { "idêntico ao baixado do outro endereço".into() }
                            else { format!("DIFERENTE ({} × {} bytes): não use o outro endereço", b0.len(), bu.len()) }, Some(&url0));
                        bu_ok = igual;
                    }
                    Some((st, _)) => s.passo(false, "Mesmo BU pela base principal", format!("HTTP {st} (não deu para comparar)"), Some(&url0)),
                    None => {}
                }
            }
        }
        break;
    }
    json!({"ok": true, "bu_ok": bu_ok, "passos": s.passos, "pleitos": pleitos, "proposta": proposta, "exemplo": exemplo, "requisicoes": s.n})
}

#[cfg(test)]
mod testes;
