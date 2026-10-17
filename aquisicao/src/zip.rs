//! Aquisição pelo pacote de **dados abertos** do TSE (um zip por UF com BU, RDV, log e
//! assinaturas de todas as seções), publicado depois da eleição em cdn.tse.jus.br.
//!
//! Dois modos:
//! * `faixas`: lê só o diretório central do zip com requisições HTTP Range e depois baixa
//!   apenas as entradas `-bu.dat` (≈ 8 % do pacote), uma faixa por BU;
//! * `completo`: baixa o zip inteiro para o disco e extrai os BUs localmente.

use crate::coletor::{Medida, Recebido, Rede};
use urna_velox::pacote::Registro;
use crate::tse;
use flate2::read::DeflateDecoder;
use futures::StreamExt;
use std::io::Read;
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};
use std::sync::Arc;
use std::time::Instant;

/// Entrada de BU no zip: o da urna (`-bu.dat`, `.bu` em anos anteriores) ou o do Sistema de Apuração
/// (`-busa.dat`), usado quando a seção foi apurada fora da urna.
pub fn e_bu(nome: &str) -> bool { nome.ends_with("-bu.dat") || nome.ends_with("-busa.dat") || nome.ends_with(".bu") }

#[derive(Clone, Debug)]
pub struct Entrada { pub nome: String, pub metodo: u16, pub comp: u64, pub unc: u64, pub off: u64 }

fn u16le(b: &[u8], i: usize) -> u16 { u16::from_le_bytes([b[i], b[i + 1]]) }
fn u32le(b: &[u8], i: usize) -> u32 { u32::from_le_bytes(b[i..i + 4].try_into().unwrap()) }
fn u64le(b: &[u8], i: usize) -> u64 { u64::from_le_bytes(b[i..i + 8].try_into().unwrap()) }

/// Localiza o diretório central a partir do fim do arquivo. Devolve (offset, tamanho).
pub fn achar_diretorio(fim: &[u8], tamanho_arquivo: u64) -> anyhow::Result<(u64, u64)> {
    let i = fim.windows(4).rposition(|w| w == b"PK\x05\x06").ok_or_else(|| anyhow::anyhow!("fim do diretório central não encontrado"))?;
    let (mut size, mut off) = (u32le(fim, i + 12) as u64, u32le(fim, i + 16) as u64);
    if off == 0xFFFF_FFFF || size == 0xFFFF_FFFF {
        let j = fim.windows(4).rposition(|w| w == b"PK\x06\x06").ok_or_else(|| anyhow::anyhow!("zip64 sem registro de fim"))?;
        size = u64le(fim, j + 40);
        off = u64le(fim, j + 48);
    }
    anyhow::ensure!(off + size <= tamanho_arquivo, "diretório central fora do arquivo");
    Ok((off, size))
}

/// Interpreta o diretório central (com suporte a zip64).
pub fn ler_diretorio(cd: &[u8]) -> Vec<Entrada> {
    let mut v = vec![];
    let mut p = 0;
    while p + 46 <= cd.len() && &cd[p..p + 4] == b"PK\x01\x02" {
        let metodo = u16le(cd, p + 10);
        let (mut comp, mut unc) = (u32le(cd, p + 20) as u64, u32le(cd, p + 24) as u64);
        let (nl, el, cl) = (u16le(cd, p + 28) as usize, u16le(cd, p + 30) as usize, u16le(cd, p + 32) as usize);
        let mut off = u32le(cd, p + 42) as u64;
        let nome = String::from_utf8_lossy(&cd[p + 46..p + 46 + nl]).into_owned();
        // campo extra zip64 (id 0x0001): valores na ordem unc, comp, off, só os que estão saturados
        let mut x = p + 46 + nl;
        let xe = x + el;
        while x + 4 <= xe {
            let (id, sz) = (u16le(cd, x), u16le(cd, x + 2) as usize);
            if id == 1 {
                let mut q = x + 4;
                if unc == 0xFFFF_FFFF { unc = u64le(cd, q); q += 8; }
                if comp == 0xFFFF_FFFF { comp = u64le(cd, q); q += 8; }
                if off == 0xFFFF_FFFF { off = u64le(cd, q); }
            }
            x += 4 + sz;
        }
        v.push(Entrada { nome, metodo, comp, unc, off });
        p += 46 + nl + el + cl;
    }
    v
}

/// Extrai o conteúdo de uma entrada a partir dos bytes que começam no cabeçalho local.
pub fn extrair(local: &[u8], e: &Entrada) -> anyhow::Result<Vec<u8>> {
    anyhow::ensure!(local.len() >= 30 && &local[..4] == b"PK\x03\x04", "cabeçalho local inválido em {}", e.nome);
    let ini = 30 + u16le(local, 26) as usize + u16le(local, 28) as usize;
    let fim = ini + e.comp as usize;
    anyhow::ensure!(fim <= local.len(), "faixa curta para {}", e.nome);
    let dados = &local[ini..fim];
    match e.metodo {
        0 => Ok(dados.to_vec()),
        8 => { let mut out = Vec::with_capacity(e.unc as usize); DeflateDecoder::new(dados).read_to_end(&mut out)?; Ok(out) }
        m => anyhow::bail!("método de compressão {m} não suportado ({})", e.nome),
    }
}

fn registro_do_nome(nome: &str) -> Option<Registro> {
    let (pleito, uf, mu, zn, se) = tse::chave_do_nome(nome)?;
    Some(Registro { pleito, uf, municipio: mu, zona: zn, secao: se, situacao: 1, origem: 2, sha256_vsc: [0; 32] })
}

/// Lê o diretório central de um zip remoto. Devolve (tamanho do arquivo, entradas, bytes lidos).
pub async fn listar_remoto(rede: &Rede, url: &str) -> anyhow::Result<(u64, Vec<Entrada>)> {
    // tamanho (HEAD), depois os últimos 64 KiB (registro de fim, inclusive zip64), depois o diretório
    let tamanho = tamanho_remoto(url).await?;
    let ini = tamanho.saturating_sub(65_536);
    let fim = rede.get(url, None, Some((ini, tamanho - 1))).await?;
    anyhow::ensure!(fim.status == 206, "o servidor não aceitou Range (HTTP {})", fim.status);
    let (off, size) = achar_diretorio(&fim.corpo, tamanho)?;
    let cd = rede.get(url, None, Some((off, off + size - 1))).await?;
    Ok((tamanho, ler_diretorio(&cd.corpo)))
}

async fn tamanho_remoto(url: &str) -> anyhow::Result<u64> {
    let c = reqwest::Client::builder().user_agent(crate::coletor::USER_AGENT).build()?;
    let r = c.head(url).send().await?;
    r.headers().get("content-length").and_then(|v| v.to_str().ok()).and_then(|v| v.parse().ok())
        .ok_or_else(|| anyhow::anyhow!("sem Content-Length"))
}

/// Modo `faixas`: baixa só as entradas `-bu.dat`, uma requisição Range por BU.
pub async fn coletar_faixas(rede: Arc<Rede>, url: String, entradas: Vec<Entrada>, concorrencia: usize,
                            tx: crossbeam_channel::Sender<Recebido>, t0: Instant) -> Vec<Medida> {
    let total = entradas.len();
    let feitas = Arc::new(AtomicUsize::new(0));
    let url = Arc::new(url);
    futures::stream::iter(entradas.into_iter().map(|e| {
        let (rede, url, tx, feitas) = (rede.clone(), url.clone(), tx.clone(), feitas.clone());
        async move {
            let mut m = Medida::default();
            if let Some(r) = registro_do_nome(&e.nome) {
                m.uf = String::from_utf8_lossy(&r.uf).into_owned();
                m.municipio = r.municipio; m.zona = r.zona; m.secao = r.secao;
            }
            // cabeçalho local = 30 + nome + extra; o extra local pode diferir do central: margem de 512 bytes
            let fim = e.off + 30 + e.nome.len() as u64 + 512 + e.comp;
            match rede.get(&url, None, Some((e.off, fim))).await {
                Ok(resp) => {
                    m.lat_bu = resp.lat;
                    m.tentativas = resp.tentativas;
                    match extrair(&resp.corpo, &e) {
                        Ok(bu) => {
                            m.bytes_bu = bu.len();
                            if let Some(reg) = registro_do_nome(&e.nome) { let _ = tx.send(Recebido { reg, bu: bu.into() }); }
                        }
                        Err(err) => m.erro = err.to_string(),
                    }
                }
                Err(err) => m.erro = err.to_string(),
            }
            m.t_fim = t0.elapsed().as_secs_f64();
            let f = feitas.fetch_add(1, Relaxed) + 1;
            if f % 2000 == 0 || f == total { eprintln!("  [{:7.1} s] {f}/{total} BUs", m.t_fim); }
            m
        }
    })).buffer_unordered(concorrencia.max(1)).collect().await
}

/// Modo `completo`: baixa o zip inteiro em `destino` (streaming). Devolve bytes gravados.
pub async fn baixar_completo(url: &str, destino: &std::path::Path) -> anyhow::Result<u64> {
    use tokio::io::AsyncWriteExt;
    let c = reqwest::Client::builder().user_agent(crate::coletor::USER_AGENT).build()?;
    let r = c.get(url).send().await?.error_for_status()?;
    let mut f = tokio::io::BufWriter::with_capacity(4 << 20, tokio::fs::File::create(destino).await?);
    let mut s = r.bytes_stream();
    let mut n = 0u64;
    let t0 = Instant::now();
    let mut prox = 5.0;
    while let Some(ch) = s.next().await {
        let ch = ch?;
        n += ch.len() as u64;
        f.write_all(&ch).await?;
        if t0.elapsed().as_secs_f64() > prox { prox += 5.0; eprintln!("  [{:7.1} s] {:.0} MB ({:.1} MB/s)", t0.elapsed().as_secs_f64(), n as f64 / 1e6, n as f64 / 1e6 / t0.elapsed().as_secs_f64()); }
    }
    f.flush().await?;
    Ok(n)
}

/// Extrai os BUs de um zip local. Devolve os BUs como `Recebido`.
pub fn extrair_local(caminho: &std::path::Path) -> anyhow::Result<Vec<Recebido>> {
    let b = std::fs::read(caminho)?;
    let ini = b.len().saturating_sub(65_536);
    let (off, size) = achar_diretorio(&b[ini..], b.len() as u64)?;
    let ents = ler_diretorio(&b[off as usize..(off + size) as usize]);
    let mut v = vec![];
    for e in ents.iter().filter(|e| e_bu(&e.nome)) {
        let bu = extrair(&b[e.off as usize..], e)?;
        if let Some(reg) = registro_do_nome(&e.nome) { v.push(Recebido { reg, bu: bu.into() }); }
    }
    Ok(v)
}
