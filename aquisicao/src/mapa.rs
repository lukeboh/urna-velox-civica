//! **Mapa das seções** (cartão da página, desde a `0.9.19`): cada seção totalizada acende um ponto no mapa. O servidor
//! não sabe nada de geografia: dá a cada seção do catálogo um **índice** (a posição na lista ordenada pela chave
//! município/zona/seção), conta quais já foram aplicadas e em que faixa de horário, e entrega isso à página por três
//! caminhos (`docs/arquitetura/PROTOCOLO_WS.md` §4.8):
//!
//! * `GET /api/v1/mapa/secoes` — a lista das seções (fixa por processo, com `ETag`): para cada município, o código,
//!   a UF e as seções de cada zona (primeira seção e diferenças), na ordem dos índices;
//! * `GET /api/v1/mapa/estado` — uma fotografia: `n` (quantas seções acesas até aqui, na ordem em que acenderam),
//!   `epoca` (identifica o processo: outro processo = outra numeração) e `h`, um byte por índice em base64
//!   (0 = ainda não; 1 a 240 = faixa de 6 min do horário do BU no TSE: 1 = 00:00–00:06, 171 = 17:00–17:06);
//! * `{"op":"mapa","desde":n}` no WebSocket — as seções que acenderam depois de `n`, em mensagens `mapa` com
//!   `[índice, faixa, …]`, no máximo uma a cada [`INTERVALO_MS`] por conexão.
//!
//! A seção acende na primeira aplicação do BU (substituição e anulação não mudam o ponto). Na retomada, as seções do
//! diário entram de uma vez, com a faixa da disponibilização. Seção fora do catálogo (descoberta na noite) não tem
//! índice e fica de fora do mapa (contada em `fora`). Nada aqui expõe pessoa: só município, zona e seção.

use base64::Engine as _;
use bytes::Bytes;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use std::sync::{Mutex, RwLock};
use std::time::{Duration, Instant};
use tokio::sync::watch;

/// Intervalo mínimo entre duas mensagens `mapa` para a mesma conexão.
pub const INTERVALO_MS: u64 = 250;
/// No máximo tantos pares `[índice, faixa]` por mensagem `mapa` (o histórico de quem chega atrasado vai em várias).
pub const MAX_POR_MSG: usize = 8192;
/// A fotografia (`GET /api/v1/mapa/estado`) é refeita no máximo a cada tantos ms (base64 de ~500 KB).
const FOTO_MS: u64 = 1000;

/// As seções do mapa: `(uf, chave)` de cada seção do catálogo, **fora as agregadas** do `candidatos.json` (sem urna nem
/// BU próprio: os votos vêm no BU da principal). É o mesmo total de seções da página (`Publicador`, B-064); as que se
/// revelam agregadas na noite saem depois, por [`Mapa::retirar`].
pub fn chaves_do_catalogo(cat: &urna_velox::catalogo::Catalogo, cad: &urna_velox::candidatos::Candidatos) -> Vec<([u8; 2], u64)> {
    cat.secoes.keys().filter(|k| !cad.agregadas.contains(k))
        .filter_map(|&k| cat.muni((k >> 32) as u32).map(|mi| (cat.ufs[cat.muni_uf[mi as usize] as usize], k))).collect()
}

/// Faixa de horário de um BU (escala de `urna_velox::metricas::segundos`): 1 a 240, de 6 em 6 minutos do dia.
pub fn faixa(hora: i64) -> u8 { (hora.rem_euclid(86_400) / 360 + 1).min(240) as u8 }

struct Est {
    /// faixa de cada índice (0 = apagada)
    hora: Vec<u8>,
    /// (índice, faixa) na ordem em que acenderam
    ordem: Vec<(u32, u8)>,
    /// seções sem índice (fora do catálogo)
    fora: u64,
    /// índices retirados do total: seções que se revelaram agregadas na noite (B-064)
    ret: Vec<u32>,
    retirada: Vec<bool>,
}

pub struct Mapa {
    indice: HashMap<u64, u32>,
    /// corpo e `ETag` de `GET /api/v1/mapa/secoes`
    pub secoes: Bytes,
    pub etag: String,
    /// identifica o processo (milissegundos da partida)
    pub epoca: u64,
    st: RwLock<Est>,
    /// nº de seções acesas (as conexões com `op: mapa` esperam por ele)
    pub n: watch::Sender<usize>,
    foto: Mutex<Option<(usize, Instant, Bytes)>>,
    pub fotos: AtomicU64,
}

impl Mapa {
    /// A lista das seções do catálogo: `(uf, chave)` com a chave `município << 32 | zona << 16 | seção`.
    pub fn novo(mut chaves: Vec<([u8; 2], u64)>) -> Mapa {
        chaves.sort_by_key(|x| x.1);
        chaves.dedup_by_key(|x| x.1);
        let indice: HashMap<u64, u32> = chaves.iter().enumerate().map(|(i, x)| (x.1, i as u32)).collect();
        // corpo: {"v":1,"n":N,"ufs":["ac",…],"m":[cod, uf, nzonas, zona, nsecoes, s0, d1, d2, …, cod, …]}
        let mut ufs: Vec<[u8; 2]> = chaves.iter().map(|x| x.0).collect();
        ufs.sort();
        ufs.dedup();
        let mut m: Vec<u64> = vec![];
        let mut i = 0;
        while i < chaves.len() {
            let mu = chaves[i].1 >> 32;
            let uf = ufs.binary_search(&chaves[i].0).unwrap_or(0) as u64;
            let mut j = i;
            while j < chaves.len() && chaves[j].1 >> 32 == mu { j += 1; }
            // zonas do município
            let mut zonas: Vec<(u64, Vec<u64>)> = vec![];
            for x in &chaves[i..j] {
                let (z, s) = ((x.1 >> 16) & 0xffff, x.1 & 0xffff);
                if zonas.last().is_some_and(|l| l.0 == z) { if let Some(l) = zonas.last_mut() { l.1.push(s); } } else { zonas.push((z, vec![s])); }
            }
            m.extend([mu, uf, zonas.len() as u64]);
            for (z, v) in zonas {
                m.extend([z, v.len() as u64]);
                let mut ant = 0;
                for (k, s) in v.into_iter().enumerate() { m.push(if k == 0 { s } else { s - ant }); ant = s; }
            }
            i = j;
        }
        let ufs_txt: Vec<String> = ufs.iter().map(|u| String::from_utf8_lossy(u).into_owned()).collect();
        let corpo = format!("{{\"v\":1,\"n\":{},\"ufs\":{},\"m\":[{}]}}", chaves.len(), serde_json::to_string(&ufs_txt).unwrap_or_else(|_| "[]".into()),
            m.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(","));
        let etag = {
            use sha2::Digest;
            let h = sha2::Sha256::digest(corpo.as_bytes());
            format!("\"{}\"", h.iter().take(8).map(|b| format!("{b:02x}")).collect::<String>())
        };
        let n = chaves.len();
        Mapa {
            indice, secoes: Bytes::from(corpo), etag,
            epoca: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0),
            st: RwLock::new(Est { hora: vec![0; n], ordem: Vec::with_capacity(n), fora: 0, ret: vec![], retirada: vec![false; n] }),
            n: watch::Sender::new(0), foto: Mutex::new(None), fotos: AtomicU64::new(0),
        }
    }

    /// Índice de uma seção (chave `município << 32 | zona << 16 | seção`) na lista do mapa; None = fora do catálogo.
    pub fn indice_de(&self, k: u64) -> Option<u32> { self.indice.get(&k).copied() }

    /// Nº de seções do catálogo (índices).
    pub fn total(&self) -> usize { self.indice.len() }

    /// Seção que se revelou agregada na noite (`nsp` no `cs.json` ou sondada sem `aux.json`; B-051): sai do total do
    /// mapa, como sai do total da página (B-064). O índice continua na lista (fixa por processo); a fotografia leva os
    /// retirados em `ret` e as mensagens `mapa` o `total`. Devolve `true` se saiu agora (não sai se já acendeu).
    pub fn retirar(&self, k: u64) -> bool {
        let Some(&i) = self.indice.get(&k) else { return false };
        let mut st = self.st.write().unwrap();
        if st.hora[i as usize] != 0 || st.retirada[i as usize] { return false; }
        st.retirada[i as usize] = true;
        st.ret.push(i);
        true
    }

    /// Seções aplicadas (chave, horário do BU): acende as que ainda estavam apagadas. Devolve quantas acenderam.
    pub fn aplicar(&self, v: &[(u64, i64)]) -> usize {
        if v.is_empty() { return 0; }
        let mut st = self.st.write().unwrap();
        let mut novas = 0;
        for &(k, hora) in v {
            let Some(&i) = self.indice.get(&k) else { st.fora += 1; continue };
            if st.hora[i as usize] != 0 { continue; }
            // retirada (agregada na noite) que veio com BU: volta ao total, como na página (total nunca abaixo das apuradas)
            if st.retirada[i as usize] { st.retirada[i as usize] = false; st.ret.retain(|&x| x != i); }
            let f = faixa(hora);
            st.hora[i as usize] = f;
            st.ordem.push((i, f));
            novas += 1;
        }
        let n = st.ordem.len();
        drop(st);
        if novas > 0 { self.n.send_replace(n); }
        novas
    }

    /// Nº de seções acesas e de seções fora do catálogo.
    pub fn contagem(&self) -> (usize, u64) { let st = self.st.read().unwrap(); (st.ordem.len(), st.fora) }

    /// Mensagens `mapa` com as seções que acenderam de `desde` até agora (vazio = nada novo). Devolve também o novo `n`.
    pub fn mensagens(&self, desde: usize) -> (usize, Vec<String>) {
        let st = self.st.read().unwrap();
        let n = st.ordem.len();
        if desde >= n { return (n, vec![]); }
        let mut msgs = vec![];
        let mut de = desde;
        while de < n {
            let ate = (de + MAX_POR_MSG).min(n);
            let mut s = String::with_capacity((ate - de) * 10 + 80);
            s.push_str(&format!("{{\"tipo\":\"mapa\",\"epoca\":{},\"de\":{de},\"n\":{ate},\"total\":{},\"s\":[", self.epoca, st.hora.len() - st.ret.len()));
            for (j, (i, f)) in st.ordem[de..ate].iter().enumerate() {
                if j > 0 { s.push(','); }
                s.push_str(&i.to_string());
                s.push(',');
                s.push_str(&f.to_string());
            }
            s.push_str("]}");
            msgs.push(s);
            de = ate;
        }
        (n, msgs)
    }

    /// Corpo de `GET /api/v1/mapa/estado` (refeito no máximo a cada [`FOTO_MS`]).
    pub fn estado(&self) -> Bytes {
        let mut f = self.foto.lock().unwrap();
        // a fotografia muda quando acende seção ou quando sai uma agregada da noite (B-064)
        let chave = |st: &Est| st.ordem.len() | st.ret.len() << 32;
        let agora = chave(&self.st.read().unwrap());
        if let Some((c, t, b)) = f.as_ref() {
            if *c == agora || t.elapsed() < Duration::from_millis(FOTO_MS) { return b.clone(); }
        }
        let st = self.st.read().unwrap();
        let ret = st.ret.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(",");
        let corpo = format!("{{\"v\":1,\"epoca\":{},\"total\":{},\"n\":{},\"fora\":{},\"ret\":[{ret}],\"h\":\"{}\"}}", self.epoca, st.hora.len() - st.ret.len(),
            st.ordem.len(), st.fora, base64::engine::general_purpose::STANDARD.encode(&st.hora));
        let b = Bytes::from(corpo);
        *f = Some((chave(&st), Instant::now(), b.clone()));
        self.fotos.fetch_add(1, Relaxed);
        b
    }
}

#[cfg(test)]
mod testes;
