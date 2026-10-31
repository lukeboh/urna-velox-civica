//! **Série de consulta** (etapa 1.1, multipleito; `docs/propostas/PROPOSTA_SERIE_CONSULTA.md`): a base do slider de cada
//! visitante. Duas camadas:
//!
//! * [`Acervo`] — no `uv-servico`, na thread de gravação: **toda versão de todo arquivo publicado** (regra do Luciano,
//!   29/10/2026) vai, só por acréscimo, para `<pasta>/versoes-<partida>.jsonl.gz` (padrão `<saida>/serie`), uma linha
//!   por versão — `{"t":<hora da noite, s>,"s":<relógio do sistema, ms>,"a":"<arquivo>","j":<JSON publicado>}` —, e as
//!   seções que acenderam em cada lote — `{"s":…,"sec":[[chave, hora], …]}`. Gzip contínuo com descarga a cada lote do
//!   gravador: legível até a última descarga mesmo depois de uma queda. Cada partida abre um arquivo novo.
//! * [`montar`] — o `uv-serie`: lê o acervo e grava os estáticos de consulta do pleito (`<destino>/<pleito>/serie/`):
//!   `indice.json`, `t/<tópico>.json` (a última versão de cada minuto da noite; a última da lista é a publicação final,
//!   byte a byte), `mapa-secoes.json` + `mapa.json` (minuto em que cada seção acendeu), `marcos.json`, `eventos.json`,
//!   `previsoes.json`; cada um também em `.json.gz`. Confere a última versão de cada tópico com `<saida>/publicacao/`.
//!
//! O eixo é a **hora da noite** (`ultimo_bu`: escala de [`urna_velox::metricas::segundos`], hora de Brasília), não a do
//! sistema: a mesma noite dá a mesma série ao vivo, no replay a 500× e na retotalização.

use std::collections::{BTreeMap, HashMap};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use flate2::{read::MultiGzDecoder, write::GzEncoder, Compression};
use serde_json::{json, Value};

/// Nome da pasta padrão do acervo de versões dentro da saída do `uv-servico`.
pub const PASTA: &str = "serie";

/// Acervo de versões do `uv-servico` (só acréscimo).
pub struct Acervo {
    gz: Option<GzEncoder<BufWriter<std::fs::File>>>,
    pub caminho: PathBuf,
    pub versoes: u64,
    pub bytes: u64,
}

impl Acervo {
    /// Abre um arquivo novo na pasta (`versoes-<ms da partida>-<n>.jsonl.gz`; nunca reescreve um que já existe).
    pub fn abrir(pasta: &Path) -> std::io::Result<Acervo> {
        std::fs::create_dir_all(pasta)?;
        let ms = crate::relogio::agora_ms();
        let mut n = 0;
        let (f, caminho) = loop {
            let p = pasta.join(format!("versoes-{ms:013}-{n}.jsonl.gz"));
            match std::fs::OpenOptions::new().create_new(true).write(true).open(&p) {
                Ok(f) => break (f, p),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists && n < 1000 => n += 1,
                Err(e) => return Err(e),
            }
        };
        Ok(Acervo { gz: Some(GzEncoder::new(BufWriter::new(f), Compression::new(6))), caminho, versoes: 0, bytes: 0 })
    }

    fn escrever(&mut self, s: &str) -> std::io::Result<()> {
        match self.gz.as_mut() { Some(g) => g.write_all(s.as_bytes()), None => Ok(()) }?;
        self.bytes += s.len() as u64;
        Ok(())
    }

    /// Uma versão de um arquivo publicado. `t`: hora da noite do BU mais recente (None antes do primeiro BU).
    pub fn versao(&mut self, t: Option<i64>, sis_ms: i64, arquivo: &str, json: &str) -> std::io::Result<()> {
        let t = t.map_or_else(|| "null".to_string(), |x| x.to_string());
        let a = serde_json::to_string(arquivo).unwrap_or_else(|_| "\"\"".into());
        // o JSON publicado é de uma linha (byte a byte no "j"); se um dia não for, vai como texto ("js")
        let linha = if json.contains('\n') || json.trim().is_empty() {
            format!("{{\"t\":{t},\"s\":{sis_ms},\"a\":{a},\"js\":{}}}\n", serde_json::to_string(json).unwrap_or_default())
        } else {
            format!("{{\"t\":{t},\"s\":{sis_ms},\"a\":{a},\"j\":{json}}}\n")
        };
        self.escrever(&linha)?;
        self.versoes += 1;
        Ok(())
    }

    /// Seções que acenderam (chave `município << 32 | zona << 16 | seção`, hora da noite do BU).
    pub fn secoes(&mut self, sis_ms: i64, v: &[(u64, i64)]) -> std::io::Result<()> {
        if v.is_empty() { return Ok(()); }
        let mut s = String::with_capacity(v.len() * 24 + 32);
        s.push_str(&format!("{{\"s\":{sis_ms},\"sec\":["));
        for (i, (k, h)) in v.iter().enumerate() {
            if i > 0 { s.push(','); }
            s.push_str(&format!("[{k},{h}]"));
        }
        s.push_str("]}\n");
        self.escrever(&s)
    }

    /// Descarga (sync flush do gzip): o que foi escrito até aqui fica legível no disco.
    pub fn descarregar(&mut self) -> std::io::Result<()> {
        match self.gz.as_mut() { Some(g) => g.flush(), None => Ok(()) }
    }

    /// Fecha o gzip (rodapé). Depois disso o acervo não escreve mais nada.
    pub fn fechar(&mut self) -> std::io::Result<()> {
        if let Some(g) = self.gz.take() { let mut w = g.finish()?; w.flush()?; }
        Ok(())
    }
}

impl Drop for Acervo {
    fn drop(&mut self) { let _ = self.fechar(); }
}

/// Série de um tópico em montagem: a última versão de cada passo (minuto) da noite.
#[derive(Default)]
struct SerieTopico {
    arquivo: String,
    /// passo (`t / passo_s`; `i64::MIN` = antes do primeiro BU) → (t, JSON publicado)
    pontos: BTreeMap<i64, (Option<i64>, String)>,
    versoes_vistas: u64,
}

impl SerieTopico {
    /// Uma versão nova. Se a hora voltou (replay reposicionado, retotalização), o que vinha depois dela é descartado:
    /// fica a passagem mais recente da noite, coerente do começo ao fim.
    fn por(&mut self, b: i64, t: Option<i64>, j: String) {
        if let Some((&ult, _)) = self.pontos.iter().next_back() {
            if b < ult { let _ = self.pontos.split_off(&(b + 1)); }
        }
        self.pontos.insert(b, (t, j));
        self.versoes_vistas += 1;
    }
}

/// O que sai da leitura do acervo.
#[derive(Default)]
struct Lido {
    topicos: BTreeMap<String, SerieTopico>,
    /// chave da seção → menor hora da noite em que acendeu
    secoes: HashMap<u64, i64>,
    arquivos: usize,
    linhas: u64,
    /// arquivos que terminaram no meio (queda): lidos até a última linha inteira
    truncados: Vec<String>,
    t_min: Option<i64>,
    t_max: Option<i64>,
}

impl Lido {
    fn hora(&mut self, t: i64) {
        self.t_min = Some(self.t_min.map_or(t, |x| x.min(t)));
        self.t_max = Some(self.t_max.map_or(t, |x| x.max(t)));
    }
}

/// Tópico de um arquivo publicado (`br-c0001-e006257.json` → `br-c0001`); sem eleição no nome (formato antigo), o nome
/// sem a extensão.
fn topico(arquivo: &str) -> String {
    crate::difusao::topico_de(arquivo).map(str::to_string).unwrap_or_else(|| arquivo.trim_end_matches(".json").to_string())
}

/// Os arquivos do acervo, na ordem das partidas.
fn arquivos_do_acervo(pasta: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(pasta)
        .map_err(|e| anyhow::anyhow!("acervo de versões {}: {e}", pasta.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("versoes-") && n.ends_with(".jsonl.gz")))
        .collect();
    v.sort();
    Ok(v)
}

/// Lê o acervo de versões inteiro, por fluxo (a memória é a da série montada, não a do acervo).
fn ler_acervo(pasta: &Path, passo_s: i64) -> anyhow::Result<Lido> {
    let mut l = Lido::default();
    for p in arquivos_do_acervo(pasta)? {
        l.arquivos += 1;
        let mut r = BufReader::new(MultiGzDecoder::new(std::fs::File::open(&p)?));
        let mut linha = String::new();
        loop {
            linha.clear();
            match r.read_line(&mut linha) {
                Ok(0) => break,
                Ok(_) if !linha.ends_with('\n') => { l.truncados.push(p.display().to_string()); break; }
                Ok(_) => {}
                Err(_) => { l.truncados.push(p.display().to_string()); break; }
            }
            let s = linha.trim_end_matches('\n');
            l.linhas += 1;
            if let Some(i) = s.find(",\"j\":") {
                // versão com o JSON byte a byte: o cabeçalho é interpretado, o "j" vai como está
                let cab: Value = serde_json::from_str(&format!("{}}}", &s[..i])).map_err(|e| anyhow::anyhow!("{}: linha {}: {e}", p.display(), l.linhas))?;
                let j = s[i + 5..s.len() - 1].to_string();
                versao(&mut l, &cab, j, passo_s);
            } else {
                let v: Value = serde_json::from_str(s).map_err(|e| anyhow::anyhow!("{}: linha {}: {e}", p.display(), l.linhas))?;
                if let Some(sec) = v["sec"].as_array() {
                    for x in sec {
                        let (Some(k), Some(h)) = (x[0].as_u64(), x[1].as_i64()) else { continue };
                        l.hora(h);
                        let e = l.secoes.entry(k).or_insert(h);
                        if h < *e { *e = h; }
                    }
                } else if let Some(js) = v["js"].as_str() {
                    let js = js.to_string();
                    versao(&mut l, &v, js, passo_s);
                }
            }
        }
    }
    Ok(l)
}

fn versao(l: &mut Lido, cab: &Value, j: String, passo_s: i64) {
    let Some(a) = cab["a"].as_str() else { return };
    let t = cab["t"].as_i64();
    if let Some(t) = t { l.hora(t); }
    let b = t.map_or(i64::MIN, |t| t.div_euclid(passo_s));
    let s = l.topicos.entry(topico(a)).or_default();
    if s.arquivo.is_empty() { s.arquivo = a.to_string(); }
    s.por(b, t, j);
}

/// Grava `<dir>/<nome>.json` e `<dir>/<nome>.json.gz` (a CDN serve o pré-comprimido). Devolve (bruto, comprimido).
fn gravar(dir: &Path, nome: &str, corpo: &str) -> anyhow::Result<(u64, u64)> {
    let p = dir.join(format!("{nome}.json"));
    if let Some(d) = p.parent() { std::fs::create_dir_all(d)?; }
    std::fs::write(&p, corpo)?;
    let mut g = GzEncoder::new(Vec::new(), Compression::best());
    g.write_all(corpo.as_bytes())?;
    let gz = g.finish()?;
    std::fs::write(dir.join(format!("{nome}.json.gz")), &gz)?;
    Ok((corpo.len() as u64, gz.len() as u64))
}

/// "aaaa-mm-dd hh:mm:ss" (ou ISO com "T") → segundos na escala de `metricas::segundos`; 1970 = sem hora.
fn hora_iso(s: &str) -> Option<i64> {
    let s = s.trim();
    if s.len() < 19 || s.starts_with("1970") { return None; }
    let (d, h) = (&s[..10], &s[11..19]);
    let p: Vec<&str> = d.split('-').collect();
    if p.len() != 3 { return None; }
    urna_velox::metricas::segundos(&format!("{}/{}/{} {h}", p[2], p[1], p[0]))
}

/// Um `.csv` da saída (`marcos.csv`, `eventos.csv`, `previsoes.csv`) → `{"colunas": [..., "t"], "linhas": [[...], ...]}`,
/// com `t` = a hora da coluna `col_hora` na escala da série (null quando não há). Linhas `#` (comentários) ficam de fora.
fn csv_json(p: &Path, col_hora: &str) -> anyhow::Result<Option<Value>> {
    let Ok(f) = std::fs::File::open(p) else { return Ok(None) };
    let mut it = BufReader::new(f).lines();
    let Some(cab) = it.next().transpose()? else { return Ok(None) };
    let colunas: Vec<String> = cab.split(',').map(str::to_string).collect();
    let ih = colunas.iter().position(|c| c == col_hora);
    let mut linhas = vec![];
    for x in it {
        let x = x?;
        if x.starts_with('#') || x.trim().is_empty() { continue; }
        let campos: Vec<&str> = x.split(',').collect();
        let t = ih.and_then(|i| campos.get(i)).and_then(|h| hora_iso(h));
        let mut v: Vec<Value> = campos.iter().map(|c| Value::String(c.to_string())).collect();
        v.push(t.map_or(Value::Null, Value::from));
        linhas.push(Value::Array(v));
    }
    let mut cols = colunas;
    cols.push("t".into());
    Ok(Some(json!({"colunas": cols, "linhas": linhas})))
}

/// Opções do [`montar`] (o `uv-serie`).
pub struct Opcoes {
    /// saída do `uv-servico` (`publicacao/`, `marcos.csv`, `eventos.csv`, `previsoes.csv`)
    pub saida: PathBuf,
    /// pasta do acervo de versões (padrão `<saida>/serie`)
    pub acervo: Option<PathBuf>,
    /// `candidatos.json` do pleito (o mapa das seções); sem ele, sem mapa
    pub candidatos: Option<PathBuf>,
    /// pasta final da série (ex.: `<raiz>/<pleito>/serie`); é recriada
    pub destino: PathBuf,
    /// passo da série em segundos da noite (60 = uma versão por minuto)
    pub passo_s: i64,
    /// lado do TSE (`tse/<tópico>.json`, §3.2 do handoff da 1.1); None = sem
    pub tse: Option<FonteTse>,
    /// os `-u.json` oficiais finais (conferência do lado do TSE; obrigatório na reconstrução)
    pub oficial: Option<PathBuf>,
    /// pleito em curso (a noite ao vivo, série refeita a cada poucos minutos): a conferência com a publicação final é
    /// informada mas não impede a gravação (a publicação anda enquanto se lê); a série leva `em_curso` e o marcador
    /// `EM_CURSO` (o `uv-servico` serve com cache curto)
    pub em_curso: bool,
}

/// Monta a série de consulta. Devolve o resumo (JSON) e se a conferência passou: a última versão de cada tópico tem de
/// ser idêntica, byte a byte, ao arquivo em `<saida>/publicacao/` (que o `uv-servico` conferiu com os oficiais). Se não
/// passar, nada é gravado.
pub fn montar(o: &Opcoes) -> anyhow::Result<(Value, bool)> {
    anyhow::ensure!(o.passo_s > 0, "passo da série tem de ser positivo");
    let pasta = o.acervo.clone().unwrap_or_else(|| o.saida.join(PASTA));
    let lido = ler_acervo(&pasta, o.passo_s)?;
    // 1. conferência com a publicação final
    let pub_dir = o.saida.join("publicacao");
    let mut publicados: Vec<String> = std::fs::read_dir(&pub_dir)
        .map_err(|e| anyhow::anyhow!("publicação {}: {e}", pub_dir.display()))?
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter(|n| n.ends_with(".json")).collect();
    publicados.sort();
    let (mut identicos, mut divergentes, mut faltando) = (0u64, vec![], vec![]);
    for n in &publicados {
        let corpo = std::fs::read_to_string(pub_dir.join(n))?;
        match lido.topicos.get(&topico(n)).and_then(|s| s.pontos.values().next_back()) {
            Some((_, j)) if *j == corpo => identicos += 1,
            Some(_) => divergentes.push(n.clone()),
            None => faltando.push(n.clone()),
        }
    }
    let ok = o.em_curso || (!publicados.is_empty() && divergentes.is_empty() && faltando.is_empty());
    let conferencia = json!({"publicados": publicados.len(), "identicos": identicos, "divergentes": divergentes, "faltando": faltando});
    let acervo = json!({"pasta": pasta.display().to_string(), "arquivos": lido.arquivos, "linhas": lido.linhas, "truncados": lido.truncados,
        "secoes": lido.secoes.len()});
    if !ok { return Ok((json!({"ok": false, "conferencia": conferencia, "acervo": acervo}), false)); }
    // 2. a série, numa pasta nova que só toma o lugar da anterior no fim (quem lê nunca vê a série pela metade)
    let nome = o.destino.file_name().and_then(|n| n.to_str()).unwrap_or("serie").to_string();
    let nova = o.destino.with_file_name(format!("{nome}.nova"));
    let dir = &nova;
    if dir.exists() { std::fs::remove_dir_all(dir)?; }
    std::fs::create_dir_all(dir.join("t"))?;
    let (mut bruto, mut gz) = (0u64, 0u64);
    let mut soma = |x: (u64, u64)| { bruto += x.0; gz += x.1; x };
    let mut instantes = std::collections::BTreeSet::new();
    let mut topicos = serde_json::Map::new();
    let mut lista = vec![];
    for (nome, s) in &lido.topicos {
        let mut c = String::with_capacity(s.pontos.values().map(|x| x.1.len() + 16).sum::<usize>() + 128);
        c.push_str(&format!("{{\"topico\":{},\"arquivo\":{},\"passo_s\":{},\"versoes\":[", json!(nome), json!(s.arquivo), o.passo_s));
        for (i, (b, (t, j))) in s.pontos.iter().enumerate() {
            if *b != i64::MIN { instantes.insert(b * o.passo_s); }
            if i > 0 { c.push(','); }
            c.push_str(&format!("[{},{j}]", t.map_or_else(|| "null".to_string(), |x| x.to_string())));
        }
        c.push_str("]}");
        let (b, g) = soma(gravar(&dir.join("t"), nome, &c)?);
        topicos.insert(nome.clone(), json!({"arquivo": s.arquivo, "versoes": s.pontos.len(), "versoes_no_acervo": s.versoes_vistas, "bytes": b, "bytes_gz": g}));
        let seq = s.pontos.values().next_back().and_then(|(_, j)| serde_json::from_str::<Value>(j).ok()).and_then(|v| v["seq"].as_u64());
        lista.push(json!({"topico": nome, "arquivo": s.arquivo, "seq": seq}));
    }
    // a lista dos tópicos e os nomes (os mesmos corpos de GET /api/v1/topicos e /api/v1/nomes): a página da consulta
    soma(gravar(dir, "topicos", &Value::Array(lista).to_string())?);
    let inicio = lido.t_min.map(|t| t.div_euclid(60) * 60);
    // 3. mapa das seções: a lista (o mesmo corpo de GET /api/v1/mapa/secoes) e o minuto em que cada uma acendeu
    let mut pleito = Value::Null;
    let mut mapa = Value::Null;
    if let Some(cp) = &o.candidatos {
        let (_, arq) = urna_velox::candidatos::Candidatos::ler(cp).map_err(|e| anyhow::anyhow!("{}: {e}", cp.display()))?;
        pleito = json!(arq.pleito);
        let cad = urna_velox::candidatos::Candidatos::do_arquivo(&arq);
        let cat = urna_velox::catalogo::Catalogo::de_arquivo_candidatos(&arq).map_err(|e| anyhow::anyhow!("catálogo: {e}"))?;
        let m = crate::mapa::Mapa::novo(crate::mapa::chaves_do_catalogo(&cat, &cad));
        let t0 = inicio.unwrap_or(0);
        let mut min = vec![0u16; m.total()];
        let (mut acesas, mut fora) = (0u64, 0u64);
        for (&k, &h) in &lido.secoes {
            match m.indice_de(k) {
                Some(i) => { min[i as usize] = ((h - t0).div_euclid(60) + 1).clamp(1, u16::MAX as i64) as u16; acesas += 1; }
                None => fora += 1,
            }
        }
        let bytes: Vec<u8> = min.iter().flat_map(|x| x.to_le_bytes()).collect();
        use base64::Engine;
        soma(gravar(dir, "mapa-secoes", std::str::from_utf8(&m.secoes)?)?);
        soma(gravar(dir, "nomes", std::str::from_utf8(&crate::difusao::montar_nomes(&arq))?)?);
        let c = json!({"v": 1, "inicio": t0, "passo_s": 60, "total": m.total(), "acesas": acesas, "fora": fora,
            "m": base64::engine::general_purpose::STANDARD.encode(&bytes)}).to_string();
        soma(gravar(dir, "mapa", &c)?);
        mapa = json!({"total": m.total(), "acesas": acesas, "fora": fora});
    }
    // 4. marcos, eventos e previsões (as linhas dos .csv, com a hora da noite)
    let mut extras = serde_json::Map::new();
    for (nome, col) in [("marcos", "horario"), ("eventos", "horario"), ("previsoes", "instante")] {
        if let Some(v) = csv_json(&o.saida.join(format!("{nome}.csv")), col)? {
            let n = v["linhas"].as_array().map_or(0, Vec::len);
            soma(gravar(dir, nome, &v.to_string())?);
            extras.insert(nome.into(), json!(n));
        }
    }
    // 5. lado do TSE
    let inst: Vec<i64> = instantes.iter().copied().collect();
    let (tse, tb, tg) = gravar_tse(dir, o, &lido, &inst)?;
    soma((tb, tg));
    // 6. índice
    let indice = json!({"v": 1, "pleito": pleito, "gerado_por": format!("uv-serie {}", env!("CARGO_PKG_VERSION")), "passo_s": o.passo_s,
        "inicio": inicio, "fim": lido.t_max, "inicio_texto": inicio.map(crate::texto_hora), "fim_texto": lido.t_max.map(crate::texto_hora),
        "instantes": instantes.into_iter().collect::<Vec<_>>(), "topicos": topicos, "mapa": mapa, "extras": extras, "tse": tse, "em_curso": o.em_curso});
    soma(gravar(dir, "indice", &indice.to_string())?);
    if o.em_curso { std::fs::write(dir.join("EM_CURSO"), "série da noite em curso\n")?; }
    // a troca: a anterior sai, a nova entra (duas renomeações na mesma pasta)
    if o.destino.exists() {
        let velha = o.destino.with_file_name(format!("{nome}.velha"));
        if velha.exists() { std::fs::remove_dir_all(&velha)?; }
        std::fs::rename(&o.destino, &velha)?;
        std::fs::rename(dir, &o.destino)?;
        std::fs::remove_dir_all(&velha)?;
    } else {
        std::fs::rename(dir, &o.destino)?;
    }
    let dir = &o.destino;
    let resumo = json!({"ok": true, "destino": dir.display().to_string(), "conferencia": conferencia, "acervo": acervo,
        "topicos": lido.topicos.len(), "instantes": indice["instantes"].as_array().map_or(0, Vec::len), "mapa": mapa, "extras": indice["extras"], "tse": indice["tse"],
        "bytes": bruto, "bytes_gz": gz});
    Ok((resumo, true))
}

/// De onde vem o lado do TSE da série (`tse/<tópico>.json`).
pub enum FonteTse {
    /// parciais oficiais de divulgação: pasta com `u-*/lote-*.jsonl.gz` (uma linha por versão de cada `-u.json`:
    /// `arquivo`, `fim` = hora da geração, `conteudo`); as pastas `u-zona` ficam de fora (o Urna Velox não publica zona)
    Parciais(PathBuf),
    /// reconstrução pela hora de recebimento de cada BU no TSE (o TSE simulado do `uv-simulador`, com 1 s de atraso):
    /// `dados` = pacotes `.uvpk`, `linha_do_tempo` = `linha-do-tempo.csv` (recebimento por seção); precisa do
    /// `--oficial` (os `-u.json` finais: o molde e o fim da noite)
    Reconstrucao { dados: PathBuf, linha_do_tempo: PathBuf },
}

/// Nome do `-u.json` do TSE de um arquivo publicado (`br-c0001-e006258.json` → `br-c0001-e006258-u.json`).
fn nome_tse(arquivo: &str) -> String { format!("{}-u.json", arquivo.trim_end_matches(".json")) }

/// JSON compacto de um texto (as parciais vêm formatadas, com quebras de linha).
fn compacto(s: &str) -> Option<(String, Value)> {
    let v: Value = serde_json::from_str(s).ok()?;
    Some((v.to_string(), v))
}

#[derive(serde::Deserialize)]
struct LinhaParcial<'a> {
    #[serde(borrow)]
    arquivo: std::borrow::Cow<'a, str>,
    #[serde(borrow, default)]
    fim: Option<std::borrow::Cow<'a, str>>,
    #[serde(borrow)]
    conteudo: std::borrow::Cow<'a, str>,
}

/// Lado do TSE pelas parciais: tópico → [(t, JSON compacto)], a última versão de cada passo, em ordem. Só até `limite`
/// (a noite: as regenerações de dias depois — ordem dos candidatos, situações — não são da apuração).
fn tse_das_parciais(dir: &Path, nomes: &HashMap<String, String>, passo_s: i64, limite: i64) -> anyhow::Result<HashMap<String, Vec<(i64, String)>>> {
    let mut sub: Vec<PathBuf> = std::fs::read_dir(dir).map_err(|e| anyhow::anyhow!("parciais {}: {e}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_dir() && p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("u-") && n != "u-zona"))
        .collect();
    sub.sort();
    // tópico → passo → (t, sq na ordem de leitura, JSON)
    let mut por: HashMap<String, BTreeMap<i64, (i64, u64, String)>> = HashMap::new();
    let mut n = 0u64;
    for d in sub {
        let mut lotes: Vec<PathBuf> = std::fs::read_dir(&d)?.filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("lote-") && n.ends_with(".jsonl.gz"))).collect();
        lotes.sort();
        for l in lotes {
            let r = BufReader::new(MultiGzDecoder::new(std::fs::File::open(&l)?));
            for x in r.lines() {
                let Ok(x) = x else { break };
                let Ok(p) = serde_json::from_str::<LinhaParcial>(&x) else { continue };
                let Some(top) = nomes.get(p.arquivo.as_ref()) else { continue };
                let Some(t) = p.fim.as_deref().and_then(hora_iso) else { continue };
                if t > limite { continue; }
                let Some((j, _)) = compacto(&p.conteudo) else { continue };
                n += 1;
                let b = t.div_euclid(passo_s);
                let e = por.entry(top.clone()).or_default();
                if e.get(&b).is_none_or(|a| (a.0, a.1) <= (t, n)) { e.insert(b, (t, n, j)); }
            }
        }
    }
    Ok(por.into_iter().map(|(k, m)| (k, m.into_values().map(|(t, _, j)| (t, j)).collect())).collect())
}

/// Lado do TSE reconstruído pela hora de recebimento de cada BU: o arquivo do TSE no fim de cada instante da série e,
/// por último, o oficial final (que o TSE simulado devolve byte a byte depois da última divulgação).
fn tse_reconstruido(candidatos: &Path, dados: &Path, ltd: &Path, oficial: &Path, nomes: &HashMap<String, String>, instantes: &[i64], passo_s: i64)
    -> anyhow::Result<HashMap<String, Vec<(i64, String)>>> {
    use crate::tse_simulado::{Opcoes as OpTse, SecTse, TseSimulado};
    let f = std::fs::File::open(ltd).map_err(|e| anyhow::anyhow!("linha do tempo {}: {e}", ltd.display()))?;
    let mut it = BufReader::new(f).lines();
    let cab: Vec<String> = it.next().transpose()?.unwrap_or_default().split(',').map(str::to_string).collect();
    let col = |n: &str| cab.iter().position(|c| c == n).ok_or_else(|| anyhow::anyhow!("linha do tempo sem a coluna {n}"));
    let (cu, cm, cz, cs, csit, crec) = (col("uf")?, col("municipio")?, col("zona")?, col("secao")?, col("situacao")?, col("recebido_tse")?);
    let mut secs = vec![];
    for x in it {
        let x = x?;
        let c: Vec<&str> = x.split(',').collect();
        if c.len() <= crec.max(csit) { continue; }
        let Ok(uf) = <[u8; 2]>::try_from(c[cu].as_bytes()) else { continue };
        let (Ok(m), Ok(z), Ok(s)) = (c[cm].parse::<u64>(), c[cz].parse::<u64>(), c[cs].parse::<u64>()) else { continue };
        let nao = c[csit].to_lowercase().starts_with("nao instalada") || c[csit].to_lowercase().starts_with("não instalada");
        secs.push(SecTse { k: (uf, m << 32 | z << 16 | s), disp: urna_velox::metricas::segundos(c[crec]), nao_instalada: nao });
    }
    let mut oficiais = HashMap::new();
    for n in nomes.keys() { let p = oficial.join(n); if p.exists() { oficiais.insert(n.clone(), p); } }
    anyhow::ensure!(!oficiais.is_empty(), "nenhum -u.json oficial dos tópicos em {}", oficial.display());
    let op = OpTse { candidatos: candidatos.to_path_buf(), passo_s, p50_s: 1.0, p90_s: 1.0, semente: 7 };
    let tse = TseSimulado::iniciar(op, dados.to_path_buf(), None, secs, oficiais.clone())?;
    let mut por: HashMap<String, Vec<(i64, String)>> = HashMap::new();
    let mut ult: HashMap<String, String> = HashMap::new();
    let mut ts: Vec<i64> = instantes.iter().map(|t| t + passo_s - 1).filter(|&t| t < tse.t_final).collect();
    ts.push(tse.t_final.max(ts.last().copied().unwrap_or(i64::MIN) + 1));
    for &t in &ts {
        for n in oficiais.keys() {
            let Some((b, _)) = tse.arquivo(n, t) else { continue };
            let Some((j, mut v)) = compacto(std::str::from_utf8(&b).unwrap_or("")) else { continue };
            // marca de reconstrução (não vai no último, o oficial, para a conferência byte a byte do conteúdo)
            let j = if t < tse.t_final { if let Some(o) = v.as_object_mut() { o.insert("reconstrucao".into(), Value::Bool(true)); } v.to_string() } else { j };
            // só quando os números mudam: o arquivo simulado troca a hora de geração e de totalização a cada passo (sem
            // isso, cada tópico ganharia uma versão por instante: 5.574 municípios × ~900 instantes no 452)
            let x: Value = serde_json::from_str(&j).unwrap_or(Value::Null);
            let mut k = crate::tse_resultados::resumo(&urna_velox::oficial::Oficial::de_json(&x));
            if let Some(o) = k.as_object_mut() { o.remove("geracao"); o.remove("totalizacao"); }
            let k = k.to_string();
            if ult.get(n) == Some(&k) { continue; }
            ult.insert(n.clone(), k);
            por.entry(nomes[n].clone()).or_default().push((t, j));
        }
    }
    Ok(por)
}

/// Grava o lado do TSE (`tse/<tópico>.json`) e confere a última versão de cada tópico com o `-u.json` oficial final
/// (como JSON, não byte a byte: as parciais vêm formatadas). Devolve o bloco `tse` do índice e (bruto, comprimido).
fn gravar_tse(dir: &Path, o: &Opcoes, lido: &Lido, instantes: &[i64]) -> anyhow::Result<(Value, u64, u64)> {
    let Some(fonte) = &o.tse else { return Ok((Value::Null, 0, 0)) };
    // nome do -u.json → tópico
    let nomes: HashMap<String, String> = lido.topicos.iter().map(|(t, s)| (nome_tse(&s.arquivo), t.clone())).collect();
    let (rotulo, por, oficial) = match fonte {
        // a noite: até 6 h depois do último BU do Urna Velox
        FonteTse::Parciais(d) => ("parciais", tse_das_parciais(d, &nomes, o.passo_s, lido.t_max.map_or(i64::MAX, |t| t + 6 * 3600))?, o.oficial.clone()),
        FonteTse::Reconstrucao { dados, linha_do_tempo } => {
            let c = o.candidatos.as_deref().ok_or_else(|| anyhow::anyhow!("a reconstrução do lado do TSE precisa do --candidatos"))?;
            let of = o.oficial.as_deref().ok_or_else(|| anyhow::anyhow!("a reconstrução do lado do TSE precisa do --oficial"))?;
            ("reconstrucao", tse_reconstruido(c, dados, linha_do_tempo, of, &nomes, instantes, o.passo_s)?, Some(of.to_path_buf()))
        }
    };
    std::fs::create_dir_all(dir.join("tse"))?;
    let (mut bruto, mut gz) = (0, 0);
    let (mut identicos, mut divergentes, mut sem_oficial) = (0u64, vec![], 0u64);
    let mut topicos = serde_json::Map::new();
    let mut nomes_ord: Vec<_> = por.keys().cloned().collect();
    nomes_ord.sort();
    for top in nomes_ord {
        let v = &por[&top];
        let arquivo = nome_tse(&lido.topicos[&top].arquivo);
        let mut c = format!("{{\"topico\":{},\"arquivo\":{},\"fonte\":\"{rotulo}\",\"passo_s\":{},\"versoes\":[", json!(top), json!(arquivo), o.passo_s);
        // cada versão como o `dados` da mensagem `tse` (PROTOCOLO_WS §4.6), com a marca da reconstrução
        for (i, (t, j)) in v.iter().enumerate() {
            if i > 0 { c.push(','); }
            let x: Value = serde_json::from_str(j).unwrap_or(Value::Null);
            let mut d = crate::tse_resultados::resumo(&urna_velox::oficial::Oficial::de_json(&x));
            if x.get("reconstrucao").is_some() { d["reconstrucao"] = Value::Bool(true); }
            c.push_str(&format!("[{t},{d}]"));
        }
        c.push_str("]}");
        let (b, g) = gravar(&dir.join("tse"), &top, &c)?;
        bruto += b;
        gz += g;
        topicos.insert(top.clone(), json!(v.len()));
        // conferência com o oficial final (pasta do --oficial, ou a da saída do serviço)
        let cand = oficial.as_ref().map(|d| d.join(&arquivo)).filter(|p| p.exists())
            .or_else(|| Some(o.saida.join("oficial").join(&arquivo)).filter(|p| p.exists()));
        match cand.and_then(|p| std::fs::read_to_string(p).ok()).and_then(|s| compacto(&s)) {
            Some((_, of)) => {
                let ult: Value = v.last().and_then(|(_, j)| serde_json::from_str(j).ok()).unwrap_or(Value::Null);
                if ult == of { identicos += 1; } else { divergentes.push(arquivo); }
            }
            None => sem_oficial += 1,
        }
    }
    let bloco = json!({"fonte": rotulo, "topicos": topicos, "conferencia": {"identicos": identicos, "divergentes": divergentes, "sem_oficial": sem_oficial}});
    Ok((bloco, bruto, gz))
}

#[cfg(test)]
mod testes;
