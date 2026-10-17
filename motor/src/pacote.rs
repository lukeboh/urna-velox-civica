//! Pacote de BUs (`.uvpk`): a forma como a aquisição entrega os BUs ao núcleo.
//!
//! Um arquivo por UF, só de acréscimo (append-only), para não criar 500 mil arquivos soltos
//! no disco (ADR-010). Cada registro tem um cabeçalho fixo de 52 bytes seguido do BU tal como
//! veio do TSE (bytes intocados, para que hash e assinaturas possam ser verificados depois):
//!
//! ```text
//! arquivo : "UVPK" versão:u8=1 0 0 0           (8 bytes)
//! registro: tamanho_bu:u32  pleito:u32  municipio:u32  zona:u16  secao:u16
//!           uf:[u8;2]  situacao:u8  origem:u8  sha256_vsc:[u8;32]  bu:[u8; tamanho_bu]
//! ```
//! inteiros em little-endian. `situacao`: 1 = "Totalizado" no aux.json, 0 = outra.
//! `origem`: 1 = API de resultados (aux+bu), 2 = zip de dados abertos.
//! Um registro incompleto no fim do arquivo (queda no meio da escrita) é descartado na abertura.

use std::collections::HashSet;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

pub const MAGICO: &[u8; 8] = b"UVPK\x01\0\0\0";
pub const CAB: usize = 52;

#[derive(Clone, Debug)]
pub struct Registro {
    pub pleito: u32,
    pub uf: [u8; 2],
    pub municipio: u32,
    pub zona: u16,
    pub secao: u16,
    pub situacao: u8,
    pub origem: u8,
    pub sha256_vsc: [u8; 32],
}

impl Registro {
    pub fn chave(&self) -> u64 { crate::chave(self.municipio, self.zona, self.secao) }
    fn cabecalho(&self, n: usize) -> [u8; CAB] {
        let mut h = [0u8; CAB];
        h[0..4].copy_from_slice(&(n as u32).to_le_bytes());
        h[4..8].copy_from_slice(&self.pleito.to_le_bytes());
        h[8..12].copy_from_slice(&self.municipio.to_le_bytes());
        h[12..14].copy_from_slice(&self.zona.to_le_bytes());
        h[14..16].copy_from_slice(&self.secao.to_le_bytes());
        h[16..18].copy_from_slice(&self.uf);
        h[18] = self.situacao;
        h[19] = self.origem;
        h[20..52].copy_from_slice(&self.sha256_vsc);
        h
    }
    fn de_cabecalho(h: &[u8]) -> (usize, Registro) {
        let u32le = |i: usize| u32::from_le_bytes(h[i..i + 4].try_into().unwrap());
        let u16le = |i: usize| u16::from_le_bytes(h[i..i + 2].try_into().unwrap());
        (u32le(0) as usize, Registro {
            pleito: u32le(4), municipio: u32le(8), zona: u16le(12), secao: u16le(14),
            uf: [h[16], h[17]], situacao: h[18], origem: h[19], sha256_vsc: h[20..52].try_into().unwrap(),
        })
    }
}

/// Escritor de pacotes: um arquivo por UF dentro de `dir`.
pub struct Escritor { dir: PathBuf, abertos: Vec<([u8; 2], BufWriter<File>, u64)>, pub registros: u64, pub bytes: u64 }

impl Escritor {
    pub fn novo(dir: &Path) -> std::io::Result<Self> {
        std::fs::create_dir_all(dir)?;
        Ok(Escritor { dir: dir.to_path_buf(), abertos: vec![], registros: 0, bytes: 0 })
    }
    pub fn caminho(dir: &Path, uf: &[u8; 2]) -> PathBuf { dir.join(format!("{}.uvpk", std::str::from_utf8(uf).unwrap_or("xx"))) }
    /// (arquivo, tamanho atual) da UF, abrindo-o se preciso (um registro incompleto no fim é descartado).
    fn arquivo(&mut self, uf: [u8; 2]) -> std::io::Result<(&mut BufWriter<File>, &mut u64)> {
        let i = match self.abertos.iter().position(|(u, _, _)| *u == uf) {
            Some(i) => i,
            None => {
                let p = Self::caminho(&self.dir, &uf);
                let valido = if p.exists() { varrer(&p)?.1 } else { 0 };
                let mut f = OpenOptions::new().create(true).read(true).write(true).truncate(false).open(&p)?;
                let tam = if valido == 0 { f.set_len(0)?; f.write_all(MAGICO)?; 8 } else { f.set_len(valido)?; f.seek(SeekFrom::End(0))?; valido };
                self.abertos.push((uf, BufWriter::with_capacity(1 << 20, f), tam));
                self.abertos.len() - 1
            }
        };
        let (_, w, t) = &mut self.abertos[i];
        Ok((w, t))
    }
    pub fn gravar(&mut self, r: &Registro, bu: &[u8]) -> std::io::Result<()> { self.gravar_pos(r, bu).map(|_| ()) }
    /// Grava e devolve a posição dos bytes do BU no arquivo da UF (para reler o BU depois, ex.: o diário de
    /// seções do `uv-servico`).
    pub fn gravar_pos(&mut self, r: &Registro, bu: &[u8]) -> std::io::Result<u64> {
        let (w, tam) = self.arquivo(r.uf)?;
        w.write_all(&r.cabecalho(bu.len()))?;
        w.write_all(bu)?;
        let pos = *tam + CAB as u64;
        *tam += (CAB + bu.len()) as u64;
        self.registros += 1;
        self.bytes += (CAB + bu.len()) as u64;
        Ok(pos)
    }
    /// Esvazia os buffers no sistema operacional (sobrevive à morte do processo); `sincronizar` também força o disco.
    pub fn descarregar(&mut self, sincronizar: bool) -> std::io::Result<()> {
        for (_, w, _) in self.abertos.iter_mut() { w.flush()?; if sincronizar { w.get_ref().sync_data()?; } }
        Ok(())
    }
    /// Esvazia os buffers no sistema operacional e devolve cópias dos descritores dos arquivos abertos, para forçar o
    /// disco noutra thread (`sync_data` numa cópia sincroniza o mesmo arquivo) sem parar quem grava.
    pub fn descarregar_e_copiar(&mut self) -> std::io::Result<Vec<File>> {
        let mut v = Vec::with_capacity(self.abertos.len());
        for (_, w, _) in self.abertos.iter_mut() { w.flush()?; v.push(w.get_ref().try_clone()?); }
        Ok(v)
    }
    pub fn fechar(&mut self) -> std::io::Result<()> {
        self.descarregar(true)?;
        self.abertos.clear();
        Ok(())
    }
}

/// Registros de um pacote com a posição e o tamanho dos bytes de cada BU no arquivo.
pub fn indice(p: &Path) -> std::io::Result<Vec<(Registro, u64, u32)>> {
    let mut f = File::open(p)?;
    let len = f.metadata()?.len();
    let mut m = [0u8; 8];
    if len < 8 || f.read_exact(&mut m).is_err() || &m != MAGICO { return Ok(vec![]); }
    let mut pos = 8u64;
    let mut v = vec![];
    let mut h = [0u8; CAB];
    // o arquivo já está na posição 8 (depois do cabeçalho); pula os BUs sem descartar o buffer
    let mut rd = std::io::BufReader::with_capacity(1 << 20, f);
    while pos + CAB as u64 <= len {
        rd.read_exact(&mut h)?;
        let (n, r) = Registro::de_cabecalho(&h);
        if pos + (CAB + n) as u64 > len { break; }
        v.push((r, pos + CAB as u64, n as u32));
        rd.seek_relative(n as i64)?;
        pos += (CAB + n) as u64;
    }
    Ok(v)
}

/// Lê `tam` bytes na posição `pos` de um arquivo (o BU de um registro de [`indice`]).
pub fn ler_bu_em(p: &Path, pos: u64, tam: u32) -> std::io::Result<Vec<u8>> {
    let mut f = File::open(p)?;
    f.seek(SeekFrom::Start(pos))?;
    let mut b = vec![0u8; tam as usize];
    f.read_exact(&mut b)?;
    Ok(b)
}

/// Lê os cabeçalhos de um pacote. Devolve (registros, tamanho válido em bytes).
pub fn varrer(p: &Path) -> std::io::Result<(Vec<Registro>, u64)> {
    let mut f = File::open(p)?;
    let len = f.metadata()?.len();
    let mut m = [0u8; 8];
    if len < 8 || f.read_exact(&mut m).is_err() || &m != MAGICO { return Ok((vec![], 0)); }
    let mut pos = 8u64;
    let mut v = vec![];
    let mut h = [0u8; CAB];
    while pos + CAB as u64 <= len {
        f.seek(SeekFrom::Start(pos))?;
        f.read_exact(&mut h)?;
        let (n, r) = Registro::de_cabecalho(&h);
        if pos + (CAB + n) as u64 > len { break; }
        v.push(r);
        pos += (CAB + n) as u64;
    }
    Ok((v, pos))
}

/// Chaves (município, zona, seção) já presentes nos pacotes de `dir` — usado para retomar uma coleta.
pub fn ja_coletadas(dir: &Path) -> HashSet<u64> {
    let mut s = HashSet::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            if e.path().extension().is_some_and(|x| x == "uvpk") {
                if let Ok((v, _)) = varrer(&e.path()) { s.extend(v.iter().map(|r| r.chave())); }
            }
        }
    }
    s
}

/// Pacotes mapeados em memória (mmap), com índice dos BUs: entrada do totalizador.
/// O mapeamento evita copiar gigabytes para o heap: as páginas ficam no cache do sistema
/// operacional e não contam no orçamento de memória do motor (ADR-000/ADR-010).
pub struct EmMemoria { pub blobs: Vec<memmap2::Mmap>, pub itens: Vec<(Registro, u32, usize, usize)> }

impl EmMemoria {
    pub fn carregar(dir: &Path, ufs: Option<&[String]>) -> std::io::Result<Self> {
        let mut caminhos: Vec<PathBuf> = std::fs::read_dir(dir)?.flatten().map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "uvpk"))
            .filter(|p| ufs.is_none_or(|u| u.iter().any(|x| p.file_stem().is_some_and(|s| s == x.as_str()))))
            .collect();
        caminhos.sort();
        let mut blobs = vec![];
        let mut itens = vec![];
        for p in caminhos {
            let f = File::open(&p)?;
            // SAFETY: os pacotes só recebem acréscimos; o totalizador não roda junto com um escritor no mesmo arquivo.
            let b = unsafe { memmap2::Mmap::map(&f)? };
            let _ = b.advise(memmap2::Advice::WillNeed);
            if b.len() < 8 || &b[..8] != MAGICO { continue; }
            let bi = blobs.len() as u32;
            let mut pos = 8;
            while pos + CAB <= b.len() {
                let (n, r) = Registro::de_cabecalho(&b[pos..pos + CAB]);
                if pos + CAB + n > b.len() { break; }
                itens.push((r, bi, pos + CAB, pos + CAB + n));
                pos += CAB + n;
            }
            blobs.push(b);
        }
        Ok(EmMemoria { blobs, itens })
    }
    pub fn bu(&self, i: usize) -> &[u8] { let (_, b, s, e) = &self.itens[i]; &self.blobs[*b as usize][*s..*e] }
    pub fn bytes(&self) -> usize { self.blobs.iter().map(|b| b.len()).sum() }
}
