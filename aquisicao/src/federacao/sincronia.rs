//! Sincronização direta entre os nós (P2P, **sem storage compartilhado**; ADR-028, revisão de 14/10).
//!
//! Cada nó guarda no seu diretório local (o [`Armazem`] do nó) os próprios *heartbeats* e lotes **e as cópias dos
//! dos outros**. Duas tarefas:
//!
//! * [`servir`]: HTTP só de leitura em `endereco` do nó (`GET /fed/v1/nos/<nó>.json`, `GET /fed/v1/bus/<nó>/ultimo`,
//!   `GET /fed/v1/bus/<nó>/<seq>.uvtr`), com `ETag`/`If-None-Match`. Serve o que tiver, de qualquer origem.
//! * [`sincronizar`]: para cada outro nó (a **origem**), busca o *heartbeat* e os lotes novos primeiro na própria
//!   origem; se ela não responde, num dos outros pares, que repassam a cópia que têm. Assim, se A não alcança C mas B
//!   alcança, os BUs de C chegam a A por B.
//!
//! O resto do nó ([`super::no`]) não muda: o consumidor lê os lotes dos outros do diretório local, como lia do
//! storage. A vida de um nó é medida localmente: o *heartbeat* muda a cada batida (contador), e a cópia local só é
//! regravada quando muda — se a origem morre, nenhuma fonte tem cópia nova e a cópia local envelhece até `morte_s`.
//!
//! Sem autenticação entre nós por enquanto (etapa C: lotes e *heartbeats* assinados com Ed25519, chaves no
//! `federacao.json`): um par só consegue servir o que tem, e o BU recebido é conferido pelo SHA-256 declarado.

use super::armazem::{validar_chave, Lido};
use super::no::Federado;
use crate::servico::Comum;
use anyhow::{bail, Context, Result};
use bytes::Bytes;
use http_body_util::Full;
use hyper::{Request, Response, StatusCode};
use serde_json::json;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use std::sync::Arc;
use std::time::Duration;

/// Contadores da sincronização.
#[derive(Default)]
pub struct ContSinc {
    pub servidas: AtomicU64,
    pub servidas_304: AtomicU64,
    pub buscas: AtomicU64,
    pub buscas_erro: AtomicU64,
    pub lotes_copiados: AtomicU64,
    pub por_repasse: AtomicU64,
}

/// Só estes prefixos saem pelo HTTP da federação (nada do resto do disco do nó).
fn chave_servivel(c: &str) -> bool { (c.starts_with("nos/") || c.starts_with("bus/")) && validar_chave(c).is_ok() }

fn resposta(status: u16, corpo: Bytes, etag: Option<String>) -> Response<Full<Bytes>> {
    let mut b = Response::builder().status(StatusCode::from_u16(status).unwrap()).header("cache-control", "no-cache");
    if let Some(e) = etag { b = b.header("etag", format!("\"{e}\"")); }
    b.body(Full::new(corpo)).unwrap()
}

fn atender(f: &Federado, path: &str, inm: Option<&str>) -> Response<Full<Bytes>> {
    let Some(chave) = path.strip_prefix("/fed/v1/") else { return resposta(404, Bytes::new(), None) };
    if !chave_servivel(chave) { return resposta(404, Bytes::new(), None); }
    let inm = inm.map(|s| s.trim_matches('"'));
    match f.armazem().ler(chave, inm) {
        Ok(Lido::Novo(o)) => { f.sinc.servidas.fetch_add(1, Relaxed); resposta(200, Bytes::from(o.bytes), Some(o.versao.etag)) }
        Ok(Lido::NaoMudou) => { f.sinc.servidas_304.fetch_add(1, Relaxed); resposta(304, Bytes::new(), inm.map(String::from)) }
        Ok(Lido::NaoExiste) => resposta(404, Bytes::new(), None),
        Err(_) => resposta(500, Bytes::new(), None),
    }
}

/// Serve o diretório do nó aos pares, até o serviço parar.
pub async fn servir(f: Arc<Federado>, com: Arc<Comum>, escuta: SocketAddr) -> Result<()> {
    use hyper::server::conn::http1;
    use hyper::service::service_fn;
    use hyper_util::rt::TokioIo;
    let l = tokio::net::TcpListener::bind(escuta).await.with_context(|| format!("federação: escutando em {escuta}"))?;
    com.registrar("info", "federacao_escuta", json!({"endereco": l.local_addr()?.to_string()}));
    // para só no `fim` do nó (não no `parar` do serviço): no encerramento, os pares ainda levam os últimos lotes
    while !f.fim.load(Relaxed) {
        let r = tokio::select! { r = l.accept() => r, _ = tokio::time::sleep(Duration::from_millis(200)) => continue };
        let Ok((s, _)) = r else { continue };
        let _ = s.set_nodelay(true);
        let f = f.clone();
        tokio::spawn(async move {
            let svc = service_fn(move |req: Request<hyper::body::Incoming>| {
                let f = f.clone();
                let path = req.uri().path().to_string();
                let inm = req.headers().get("if-none-match").and_then(|v| v.to_str().ok()).map(String::from);
                let get = req.method() == hyper::Method::GET;
                async move { Ok::<_, std::convert::Infallible>(if get { atender(&f, &path, inm.as_deref()) } else { resposta(405, Bytes::new(), None) }) }
            });
            let _ = http1::Builder::new().keep_alive(true).serve_connection(TokioIo::new(s), svc).await;
        });
    }
    Ok(())
}

/// GET num par: `Ok(Some(corpo))`, `Ok(None)` = 304 (a cópia local é a mesma), erro = 404, falha de rede etc.
async fn buscar(cli: &reqwest::Client, f: &Federado, end: &str, chave: &str) -> Result<Option<Bytes>> {
    f.sinc.buscas.fetch_add(1, Relaxed);
    let mut rq = cli.get(format!("http://{end}/fed/v1/{chave}"));
    if let Lido::Novo(o) = f.armazem().ler(chave, None)? { rq = rq.header("if-none-match", format!("\"{}\"", o.versao.etag)); }
    let r = rq.send().await?;
    match r.status().as_u16() {
        200 => Ok(Some(r.bytes().await?)),
        304 => Ok(None),
        s => bail!("{end}/{chave}: HTTP {s}"),
    }
}

fn ultimo_local(f: &Federado, origem: &str) -> u64 {
    match f.armazem().ler(&format!("bus/{origem}/ultimo"), None) {
        Ok(Lido::Novo(o)) => String::from_utf8_lossy(&o.bytes).trim().parse().unwrap_or(0),
        _ => 0,
    }
}

/// Heartbeat da origem, de uma fonte: só regrava se mudou (a idade da cópia local é o que mede a vida da origem).
async fn puxar_batimento(cli: &reqwest::Client, f: &Federado, origem: &str, fonte: &str) -> Result<()> {
    if let Some(b) = buscar(cli, f, fonte, &format!("nos/{origem}.json")).await? {
        let k = format!("nos/{origem}.json");
        let igual = matches!(f.armazem().ler(&k, None)?, Lido::Novo(o) if o.bytes == b);
        if !igual { f.armazem().gravar(&k, &b)?; }
    }
    Ok(())
}

/// Lotes por volta e por origem: com milhares a copiar (um nó que volta), as outras origens não esperam o fim
const LOTES_POR_VOLTA: u64 = 64;

/// Atualiza a cópia local dos lotes da origem a partir de uma fonte (a própria origem ou um par que a repassa).
async fn puxar(cli: &reqwest::Client, f: &Federado, origem: &str, fonte: &str) -> Result<()> {
    // lotes: os que faltam, em ordem, e só então o `ultimo` local (o consumidor lê `ultimo` e depois os lotes)
    let k_ult = format!("bus/{origem}/ultimo");
    // 404 = a origem ainda não publicou nenhum lote (nada a copiar); outro erro (fonte fora do ar) passa para a próxima
    // fonte — é isso que faz o repasse funcionar quando a própria origem não responde
    let Some(b) = (match buscar(cli, f, fonte, &k_ult).await {
        Ok(x) => x,
        Err(e) if e.to_string().ends_with("HTTP 404") => None,
        Err(e) => return Err(e),
    }) else { return Ok(()) };
    let n_origem: u64 = String::from_utf8_lossy(&b).trim().parse().context("ultimo ilegível")?;
    let m = ultimo_local(f, origem);
    let n = n_origem.min(m + LOTES_POR_VOLTA);
    for seq in m + 1..=n {
        let k = format!("bus/{origem}/{seq:010}.uvtr");
        if matches!(f.armazem().ler(&k, None)?, Lido::Novo(_)) { continue; }
        let Some(lote) = buscar(cli, f, fonte, &k).await? else { bail!("{fonte}: lote {seq} de {origem} veio 304 sem cópia local") };
        super::no::decodificar(&lote).with_context(|| format!("lote {seq} de {origem} ilegível (de {fonte})"))?;
        f.armazem().gravar(&k, &lote)?;
        f.sinc.lotes_copiados.fetch_add(1, Relaxed);
        if Some(fonte) != f.endereco_de(origem).as_deref() { f.sinc.por_repasse.fetch_add(1, Relaxed); }
    }
    if n > m { f.armazem().gravar(&k_ult, n.to_string().as_bytes())?; }
    Ok(())
}

/// Puxa de cada outro nó, primeiro da própria origem, depois de qualquer par que responda.
pub async fn sincronizar(f: Arc<Federado>, com: Arc<Comum>) {
    let cli = match reqwest::Client::builder().timeout(Duration::from_secs(5)).connect_timeout(Duration::from_secs(2)).build() {
        Ok(c) => c,
        Err(e) => { com.registrar("erro", "federacao_sincronia", json!({"erro": e.to_string()})); return; }
    };
    let outros: Vec<String> = f.cfg.nos.iter().filter(|n| n.id != f.id).map(|n| n.id.clone()).collect();
    let mut falhas_avisadas: std::collections::HashSet<String> = Default::default();
    while !com.parar.load(Relaxed) {
        for origem in &outros {
            // fontes: a própria origem e, depois, os outros pares com endereço
            let mut fontes: Vec<String> = f.endereco_de(origem).into_iter().collect();
            fontes.extend(outros.iter().filter(|p| *p != origem).filter_map(|p| f.endereco_de(p)));
            let mut ok = false;
            for fonte in &fontes {
                match puxar(&cli, &f, origem, fonte).await {
                    Ok(()) => { ok = true; break; }
                    Err(_) => { f.sinc.buscas_erro.fetch_add(1, Relaxed); }
                }
            }
            if ok { falhas_avisadas.remove(origem); }
            else if falhas_avisadas.insert(origem.clone()) {
                com.registrar("aviso", "federacao_sem_fonte", json!({"origem": origem, "fontes": fontes}));
            }
        }
        tokio::time::sleep(f.troca).await;
    }
}

/// Os *heartbeats* dos pares numa tarefa só deles (`2.0.0-alpha.10`): antes vinham na mesma volta dos lotes, e um nó
/// copiando milhares de lotes (ao voltar de uma queda, ou no pico) deixava a cópia do heartbeat de outro par envelhecer
/// até parecer morto. Busca cada par a cada meio *heartbeat*, direto ou por repasse.
pub async fn vigiar_batimentos(f: Arc<Federado>, com: Arc<Comum>) {
    let cli = match reqwest::Client::builder().timeout(Duration::from_secs(2)).connect_timeout(Duration::from_secs(1)).build() {
        Ok(c) => c,
        Err(e) => { com.registrar("erro", "federacao_batimentos", json!({"erro": e.to_string()})); return; }
    };
    let outros: Vec<String> = f.cfg.nos.iter().filter(|n| n.id != f.id).map(|n| n.id.clone()).collect();
    while !com.parar.load(Relaxed) {
        for origem in &outros {
            let mut fontes: Vec<String> = f.endereco_de(origem).into_iter().collect();
            fontes.extend(outros.iter().filter(|p| *p != origem).filter_map(|p| f.endereco_de(p)));
            for fonte in &fontes {
                if puxar_batimento(&cli, &f, origem, fonte).await.is_ok() { break; }
                f.sinc.buscas_erro.fetch_add(1, Relaxed);
            }
        }
        tokio::time::sleep(f.batimento / 2).await;
    }
}
