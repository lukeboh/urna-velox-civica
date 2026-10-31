//! Leitura e edição do `no.env` de um nó (`/etc/urna-velox/no.env`, lido pelo `uv-iniciar.sh`).
//!
//! O arquivo é **lido pelo bash** (`source`), então um valor mal formado viraria comando. Por isso só se editam as chaves
//! da lista [`CAMPOS`] marcadas como editáveis, cada valor passa pela regra do campo e por uma lista fechada de
//! caracteres ([`seguro`]: sem espaço, aspas, `$`, crase, `;`, `|`, `&`, `<`, `>`, parênteses), e o resto do arquivo
//! (comentários, chaves desconhecidas, `EXTRA`) é mantido como está. O `uv-painel-acao` repete a conferência antes de
//! instalar o arquivo.

use anyhow::{bail, Result};
use serde_json::{json, Value};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Tipo { Texto, Numero, Ufs, Caminho, Url, Opcao(&'static [&'static str]) }

pub struct Campo {
    pub chave: &'static str,
    pub rotulo: &'static str,
    pub tipo: Tipo,
    pub editavel: bool,
    /// muda o que o nó coleta ou de onde: pede a confirmação "CONFIRMO"
    pub sensivel: bool,
    /// vazio tira a linha do arquivo (opção desligada)
    pub opcional: bool,
    pub ajuda: &'static str,
}

const fn c(chave: &'static str, rotulo: &'static str, tipo: Tipo, editavel: bool, sensivel: bool, opcional: bool, ajuda: &'static str) -> Campo {
    Campo { chave, rotulo, tipo, editavel, sensivel, opcional, ajuda }
}

/// Teto por órgão (por IP de saída) contra o TSE (`CLAUDE.md`); o `uv-servico` também garante.
pub const TETO_RPS: f64 = 100.0;

pub const CAMPOS: &[Campo] = &[
    c("PLEITO", "Pleito", Tipo::Texto, true, true, true, "código do pleito (3221 = 2º turno de 2026); vazio = o ciclo escolhe (multipleito)"),
    c("CICLO", "Ciclo", Tipo::Texto, true, true, false, "diretório do ciclo no site, ex.: ele2026"),
    c("AMBIENTE", "Ambiente", Tipo::Opcao(&["oficial", "simulado"]), true, true, false, "oficial | simulado"),
    c("BASE", "Servidor de resultados", Tipo::Url, true, true, false, "de onde o nó coleta; o TSE pede a confirmação \"CONFIRMO TSE\""),
    c("RPS", "Requisições por segundo", Tipo::Numero, true, true, false, "na borda: no máximo 100 por órgão (coletas longas: 95); no modo espelho, até o teto do espelho"),
    c("ACESSO", "Acesso ao servidor", Tipo::Opcao(&["borda", "interno"]), true, true, true,
        "borda = borda pública do TSE (limite de 100 req/s, sempre); interno = servidor espelho dos arquivos, fora da borda pública (teto abaixo); vazio = borda"),
    c("TETO_RPS_INTERNO", "Teto no modo espelho", Tipo::Numero, true, true, true, "req/s; 0 ou vazio = sem limite (só com ACESSO=interno)"),
    c("TRAVA_PP", "Trava: máximo à frente do TSE", Tipo::Numero, true, false, true,
        "pontos percentuais de seções totalizadas à frente do TSE (Presidente, Brasil): 0 = nunca à frente, 100 = sem limite; vazio = desligada; vale sem reiniciar (M-020)"),
    c("UFS", "Só estas UFs", Tipo::Ufs, true, false, true, "por vírgula (vazio = todas)"),
    c("ACELERACAO", "Aceleração local", Tipo::Numero, true, false, true, "só com servidor local (simulador); vazio = 1×"),
    c("CANDIDATOS", "candidatos.json", Tipo::Caminho, true, false, true, "vazio = /srv/urna-velox/preparo/candidatos.json"),
    c("ORDEM", "Ordem de coleta", Tipo::Caminho, true, false, true, "métricas com recebido_tse (simulados)"),
    c("NO_ID", "Id do nó", Tipo::Texto, false, false, false, "entra no hash da partição: não muda durante uma eleição"),
    c("FED_PORTA", "Porta da federação", Tipo::Numero, false, false, false, ""),
    c("ECO_IP", "Eco do IP de saída", Tipo::Url, false, false, true, ""),
    c("HTTP_LOCAL", "Difusão (HTTP local)", Tipo::Texto, false, false, true, ""),
    c("IP_LOCAL", "IP local de saída", Tipo::Texto, false, false, true, ""),
    c("EXTRA", "Argumentos extras", Tipo::Texto, false, false, true, "só pela implantação (vão direto para a linha de comando)"),
];

pub fn campo(chave: &str) -> Option<&'static Campo> { CAMPOS.iter().find(|c| c.chave == chave) }

/// Só letras, dígitos e `. _ : / , @ + = -`.
pub fn seguro(v: &str) -> bool {
    v.bytes().all(|b| b.is_ascii_alphanumeric() || b"._:/,@+=-".contains(&b))
}

/// Chave/valor de uma linha `CHAVE=valor` (aceita aspas simples ou duplas em volta do valor e comentário depois de `#`
/// fora das aspas, como no bash). `None` para comentários, linhas em branco e o que não for atribuição simples.
pub fn linha(l: &str) -> Option<(String, String)> {
    let t = l.trim_start();
    if t.starts_with('#') { return None; }
    let t = t.strip_prefix("export ").unwrap_or(t);
    let (k, v) = t.split_once('=')?;
    if k.is_empty() || !k.bytes().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_') { return None; }
    let v = v.trim_end();
    let v = if let Some(r) = v.strip_prefix('"') { r.split_once('"').map(|x| x.0).unwrap_or(r) }
        else if let Some(r) = v.strip_prefix('\'') { r.split_once('\'').map(|x| x.0).unwrap_or(r) }
        else { v.split_once(" #").map(|x| x.0).unwrap_or(v).trim_end() };
    Some((k.to_string(), v.to_string()))
}

/// Valores do arquivo (a última atribuição vale, como no bash).
pub fn ler(texto: &str) -> BTreeMap<String, String> {
    texto.lines().filter_map(linha).collect()
}

pub fn validar(chave: &str, valor: &str) -> Result<()> {
    let Some(c) = campo(chave) else { bail!("{chave}: chave desconhecida") };
    if !c.editavel { bail!("{chave}: não se muda pelo painel ({})", if c.ajuda.is_empty() { "só pela implantação" } else { c.ajuda }); }
    if valor.is_empty() {
        if c.opcional { return Ok(()); }
        bail!("{chave}: obrigatório");
    }
    if valor.len() > 300 || !seguro(valor) { bail!("{chave}: só letras, dígitos e . _ : / , @ + = - (sem espaços)"); }
    match c.tipo {
        Tipo::Numero => {
            let n: f64 = valor.parse().map_err(|_| anyhow::anyhow!("{chave}: número inválido"))?;
            if !n.is_finite() || n < 0.0 || (n == 0.0 && chave != "TETO_RPS_INTERNO" && chave != "TRAVA_PP") { bail!("{chave}: tem de ser maior que zero"); }
            if n > crate::coletor::SEM_LIMITE_RPS { bail!("{chave}: no máximo {} req/s", crate::coletor::SEM_LIMITE_RPS); }
            if chave == "ACELERACAO" && !(1.0..=1000.0).contains(&n) { bail!("ACELERACAO: de 1 a 1000"); }
            if chave == "TRAVA_PP" && n > 100.0 { bail!("TRAVA_PP: de 0 (nunca à frente do TSE) a 100 (sem limite)"); }
        }
        Tipo::Ufs => {
            for u in valor.split(',') {
                if u.len() != 2 || !u.bytes().all(|b| b.is_ascii_alphabetic()) { bail!("UFS: siglas de 2 letras por vírgula (ex.: sp,rj)"); }
            }
        }
        Tipo::Caminho => {
            if !valor.starts_with("/srv/urna-velox/") || valor.contains("..") { bail!("{chave}: caminho absoluto dentro de /srv/urna-velox/"); }
        }
        Tipo::Url => {
            if !(valor.starts_with("https://") || valor.starts_with("http://")) { bail!("{chave}: URL http(s)://"); }
        }
        Tipo::Opcao(ops) => {
            if !ops.contains(&valor) { bail!("{chave}: uma de {}", ops.join(", ")); }
        }
        Tipo::Texto => {
            if chave == "PLEITO" && !valor.bytes().all(|b| b.is_ascii_digit()) { bail!("PLEITO: só dígitos"); }
            if chave == "CICLO" && !valor.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit()) { bail!("CICLO: letras minúsculas e dígitos"); }
        }
    }
    Ok(())
}

/// Regras que dependem de mais de um campo, sobre o arquivo como ficaria (`base_arquivos`: a do `rotas_tse.json`):
/// * na borda (padrão), RPS acima de 100 só contra o simulador local (o `uv-servico` faz o mesmo);
/// * o modo espelho (`interno`) nunca vale para a borda pública (`resultados`/`cdn.tse.jus.br`) nem para o simulador local,
///   nem quando os BUs vêm de lá.
///
/// Recusar aqui evita gravar um `no.env` com que o serviço não sobe.
pub fn validar_conjunto(v: &BTreeMap<String, String>, base_arquivos: Option<&str>) -> Result<()> {
    let base = v.get("BASE").map(String::as_str).unwrap_or("");
    let interno = v.get("ACESSO").map(String::as_str) == Some("interno");
    let rps: f64 = v.get("RPS").and_then(|r| r.parse().ok()).unwrap_or(95.0);
    if interno {
        crate::coletor::Acesso::Interno { teto: None }.validar(&[base, base_arquivos.unwrap_or(base)])?;
    } else {
        if rps > TETO_RPS && !crate::coletor::base_local(base) {
            bail!("RPS: na borda, no máximo {TETO_RPS} req/s por órgão (acima disso o TSE responde 429). Para passar disso, ACESSO=interno (servidor espelho, fora da borda pública)");
        }
        if v.get("TETO_RPS_INTERNO").is_some_and(|t| !t.is_empty()) {
            bail!("TETO_RPS_INTERNO só vale com ACESSO=interno (deixe vazio na borda)");
        }
    }
    Ok(())
}

/// O servidor é o TSE (qualquer endereço em `tse.jus.br`).
pub fn eh_tse(base: &str) -> bool {
    let h = base.split("://").nth(1).unwrap_or(base).split(['/', ':']).next().unwrap_or("").to_lowercase();
    h == "tse.jus.br" || h.ends_with(".tse.jus.br")
}

/// Confirmação exigida por um conjunto de alterações sobre o arquivo atual: `None`, `"CONFIRMO"` (campo sensível) ou
/// `"CONFIRMO TSE"` (a coleta passa a apontar para o TSE, ou continua no TSE com outro pleito/ritmo).
pub fn confirmacao_exigida(atual: &BTreeMap<String, String>, alt: &BTreeMap<String, String>) -> Option<&'static str> {
    let muda: Vec<&String> = alt.iter().filter(|(k, v)| atual.get(*k).map(String::as_str).unwrap_or("") != v.as_str()).map(|(k, _)| k).collect();
    if !muda.iter().any(|k| campo(k).is_some_and(|c| c.sensivel)) { return None; }
    let base_final = alt.get("BASE").or(atual.get("BASE")).map(String::as_str).unwrap_or("");
    if eh_tse(base_final) { Some("CONFIRMO TSE") } else { Some("CONFIRMO") }
}

/// Aplica as alterações ao texto do arquivo: troca a última linha de cada chave (as anteriores, que o bash ignoraria,
/// viram comentário), acrescenta as que não existem e tira as opcionais deixadas vazias. Valida tudo antes.
pub fn aplicar(texto: &str, alt: &BTreeMap<String, String>) -> Result<String> {
    aplicar_com(texto, alt, None)
}

/// Como [`aplicar`], conferindo também as regras entre campos ([`validar_conjunto`]) com a base dos arquivos de urna.
pub fn aplicar_com(texto: &str, alt: &BTreeMap<String, String>, base_arquivos: Option<&str>) -> Result<String> {
    for (k, v) in alt { validar(k, v)?; }
    let mut final_ = ler(texto);
    for (k, v) in alt { if v.is_empty() { final_.remove(k); } else { final_.insert(k.clone(), v.clone()); } }
    validar_conjunto(&final_, base_arquivos)?;
    let linhas: Vec<&str> = texto.lines().collect();
    let mut ultima: BTreeMap<String, usize> = BTreeMap::new();
    for (i, l) in linhas.iter().enumerate() {
        if let Some((k, _)) = linha(l) { ultima.insert(k, i); }
    }
    let mut saida: Vec<String> = Vec::with_capacity(linhas.len() + alt.len());
    for (i, l) in linhas.iter().enumerate() {
        match linha(l).filter(|(k, _)| alt.contains_key(k)) {
            Some((k, _)) if ultima.get(&k) == Some(&i) => {
                let v = &alt[&k];
                if !v.is_empty() { saida.push(format!("{k}={v}")); }
            }
            Some(_) => saida.push(format!("# (substituída pelo painel) {l}")),
            None => saida.push(l.to_string()),
        }
    }
    for (k, v) in alt {
        if !ultima.contains_key(k) && !v.is_empty() { saida.push(format!("{k}={v}")); }
    }
    let mut t = saida.join("\n");
    t.push('\n');
    Ok(t)
}

/// Descrição dos campos com o valor atual, para a página.
pub fn descrever(valores: &BTreeMap<String, String>) -> Value {
    let campos: Vec<Value> = CAMPOS.iter().map(|c| {
        let tipo = match c.tipo {
            Tipo::Texto => json!("texto"), Tipo::Numero => json!("numero"), Tipo::Ufs => json!("ufs"),
            Tipo::Caminho => json!("caminho"), Tipo::Url => json!("url"), Tipo::Opcao(o) => json!({"opcoes": o}),
        };
        json!({"chave": c.chave, "rotulo": c.rotulo, "tipo": tipo, "editavel": c.editavel, "sensivel": c.sensivel,
            "opcional": c.opcional, "ajuda": c.ajuda, "valor": valores.get(c.chave)})
    }).collect();
    let outros: BTreeMap<&String, &String> = valores.iter().filter(|(k, _)| campo(k).is_none()).collect();
    json!({"campos": campos, "outros": outros, "teto_rps": TETO_RPS})
}

#[cfg(test)]
mod testes;
