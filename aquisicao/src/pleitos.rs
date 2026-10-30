//! Multipleito (etapa 1.1, `ROADMAP.md`): descoberta dos pleitos publicados no `ele-c.json` do TSE, sem ninguém
//! configurar o pleito.
//!
//! O `ele-c.json` (`<base>/<amb>/comum/config/ele-c.json`) lista todos os pleitos que o site de resultados conhece —
//! as eleições ordinárias de 2024 e 2026, os suplementares e as consultas populares — com a data, as eleições de cada
//! um, os cargos e as abrangências (UF e, nos municipais, os municípios). Daqui sai o `pleitos.json`, que a página
//! (`/api/v1/pleitos`), a playlist de replays e o ciclo dos nós leem no lugar das listas fixas de antes.
//!
//! Cada pleito é classificado pela data, na hora de Brasília: **agendado** (antes do dia da eleição), **em curso** (do
//! início do dia da eleição até as 8 h do dia seguinte, o fim da noite da apuração, como na playlist) e **encerrado**.
//! O motor continua com um pleito por vez (decisão do Luciano, 29/10/2026): esta lista não totaliza nada.

use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Horas depois do início do dia da eleição em que a noite da apuração termina (8 h do dia seguinte).
pub const FIM_APURACAO_H: i64 = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Situacao { Agendado, EmCurso, Encerrado }

impl Situacao {
    pub fn texto(self) -> &'static str {
        match self { Situacao::Agendado => "agendado", Situacao::EmCurso => "em curso", Situacao::Encerrado => "encerrado" }
    }
}

/// Tipo de uma eleição, pelo nome no `ele-c.json` ("Eleição Ordinária …", "Eleição Suplementar - …",
/// "Consulta Popular - …").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Tipo { Ordinaria, Suplementar, Consulta, Outra }

fn tipo_de(nome: &str) -> Tipo {
    let n = nome.trim();
    if n.starts_with("Eleição Ordinária") { Tipo::Ordinaria }
    else if n.starts_with("Eleição Suplementar") { Tipo::Suplementar }
    else if n.starts_with("Consulta") { Tipo::Consulta }
    else { Tipo::Outra }
}

#[derive(Clone, Debug, Serialize)]
pub struct Cargo { pub codigo: u8, pub nome: String }

#[derive(Clone, Debug, Serialize)]
pub struct Eleicao {
    pub codigo: u32,
    /// nome no `ele-c.json`
    pub nome: String,
    /// o lugar, nos suplementares e consultas ("Paranhos (MS)"); vazio nas ordinárias
    pub local: String,
    pub turno: u8,
    pub tipo: Tipo,
    /// cargos majoritários e consultas (os proporcionais ficam para a `2.0`)
    pub cargos: Vec<Cargo>,
    /// abrangências do `ele-c.json`, em minúsculas ("br", "ms"…)
    pub ufs: Vec<String>,
    /// municípios (código do TSE) quando o `ele-c.json` os lista
    pub municipios: Vec<u32>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Pleito {
    pub codigo: u32,
    pub ciclo: String,
    /// "dd/mm/aaaa"
    pub data: String,
    /// "aaaa-mm-dd"
    pub data_iso: String,
    pub ano: i64,
    pub turno: u8,
    pub situacao: Situacao,
    /// nome curto, o da playlist ("1º turno de 2024", "Suplementar de Paranhos (MS)")
    pub nome: String,
    /// nome completo ("Eleições Municipais 2024 · 1º turno", "Eleição suplementar · Paranhos (MS)")
    pub titulo: String,
    /// eleições além das ordinárias no mesmo pleito (suplementares e consultas do mesmo dia)
    pub complementares: usize,
    pub eleicoes: Vec<Eleicao>,
    /// todas as abrangências do pleito
    pub ufs: Vec<String>,
    /// acervo pronto para o replay (`<acervo>/<código>/.completo`)
    pub replay: bool,
    /// o `acervo.json` do acervo pronto (`montar_acervo.sh`): seções e duração da cadência, para a agenda da playlist
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acervo: Option<Value>,
}

#[derive(Clone, Debug, Default)]
pub struct Opcoes {
    /// pasta dos acervos (`<acervo>/<pleito>/.completo` = pronto para o replay)
    pub acervo: Option<PathBuf>,
    /// horas depois do início do dia da eleição em que o pleito deixa de estar em curso (0 = [`FIM_APURACAO_H`])
    pub fim_apuracao_h: i64,
}

fn txt(v: &Value, k: &str) -> String {
    match &v[k] { Value::String(s) => s.trim().to_string(), Value::Number(n) => n.to_string(), _ => String::new() }
}

/// Início do dia (00:00, hora de Brasília, escala de `urna_velox::metricas::segundos`) de "dd/mm/aaaa".
pub fn inicio_do_dia(data: &str) -> Option<i64> { urna_velox::metricas::segundos(&format!("{} 00:00:00", data.trim())) }

/// Situação de um pleito com a eleição em `data` ("dd/mm/aaaa") no instante `agora`.
pub fn classificar(data: &str, agora: i64, fim_apuracao_h: i64) -> Option<Situacao> {
    let d = inicio_do_dia(data)?;
    let fim = d + if fim_apuracao_h > 0 { fim_apuracao_h } else { FIM_APURACAO_H } * 3600;
    Some(if agora < d { Situacao::Agendado } else if agora < fim { Situacao::EmCurso } else { Situacao::Encerrado })
}

/// Nome da UF pela sigla (minúsculas), para não repetir o lugar ("Suplementar de Roraima", não "Roraima (RR)").
pub fn nome_uf(uf: &str) -> Option<&'static str> {
    const N: [(&str, &str); 28] = [
        ("ac", "Acre"), ("al", "Alagoas"), ("am", "Amazonas"), ("ap", "Amapá"), ("ba", "Bahia"), ("ce", "Ceará"),
        ("df", "Distrito Federal"), ("es", "Espírito Santo"), ("go", "Goiás"), ("ma", "Maranhão"), ("mg", "Minas Gerais"),
        ("ms", "Mato Grosso do Sul"), ("mt", "Mato Grosso"), ("pa", "Pará"), ("pb", "Paraíba"), ("pe", "Pernambuco"),
        ("pi", "Piauí"), ("pr", "Paraná"), ("rj", "Rio de Janeiro"), ("rn", "Rio Grande do Norte"), ("ro", "Rondônia"),
        ("rr", "Roraima"), ("rs", "Rio Grande do Sul"), ("sc", "Santa Catarina"), ("se", "Sergipe"), ("sp", "São Paulo"),
        ("to", "Tocantins"), ("zz", "Exterior"),
    ];
    N.iter().find(|x| x.0 == uf).map(|x| x.1)
}

/// "Eleição Suplementar - Paranhos" + ["ms"] → "Paranhos (MS)".
fn local_de(nome: &str, ufs: &[String]) -> String {
    let resto = preposicoes(nome.split_once(" - ").map(|x| x.1).unwrap_or(nome).trim());
    let resto = resto.as_str();
    let ufs: Vec<&String> = ufs.iter().filter(|u| u.as_str() != "br").collect();
    // "Consulta Popular - Mato Grosso" em MT: o nome já diz o lugar
    match ufs.as_slice() {
        [uf] if !resto.is_empty() && !nome_uf(uf).is_some_and(|n| n.eq_ignore_ascii_case(resto)) => format!("{resto} ({})", uf.to_uppercase()),
        _ => resto.to_string(),
    }
}

/// "Amparo Do Serra" → "Amparo do Serra": o TSE escreve as preposições com maiúscula nos nomes de município.
fn preposicoes(s: &str) -> String {
    s.split(' ').enumerate().map(|(i, p)| match p {
        "Do" | "Da" | "De" | "Dos" | "Das" | "E" if i > 0 => p.to_lowercase(),
        _ => p.to_string(),
    }).collect::<Vec<_>>().join(" ")
}

/// "A", "A e B", "A, B e C".
fn juntar(v: &[String]) -> String {
    match v.len() {
        0 => String::new(),
        1 => v[0].clone(),
        n => format!("{} e {}", v[..n - 1].join(", "), v[n - 1]),
    }
}

fn eleicao_de(el: &Value) -> Eleicao {
    let nome = txt(el, "nm");
    let mut ufs = BTreeSet::new();
    let mut municipios = BTreeSet::new();
    let mut cargos: Vec<Cargo> = vec![];
    for a in el["abr"].as_array().into_iter().flatten() {
        let uf = txt(a, "cd").to_lowercase();
        if !uf.is_empty() { ufs.insert(uf); }
        for m in a["mu"].as_array().into_iter().flatten() { if let Ok(c) = txt(m, "cd").parse() { municipios.insert(c); } }
        for c in a["cp"].as_array().into_iter().flatten() {
            if txt(c, "tp") == "2" { continue; } // proporcional: fora do escopo (2.0)
            let Ok(codigo) = txt(c, "cd").parse::<u8>() else { continue };
            if !cargos.iter().any(|x| x.codigo == codigo) { cargos.push(Cargo { codigo, nome: txt(c, "ds") }); }
        }
    }
    cargos.sort_by_key(|c| c.codigo);
    let ufs: Vec<String> = ufs.into_iter().collect();
    let tipo = tipo_de(&nome);
    let local = if tipo == Tipo::Ordinaria { String::new() } else { local_de(&nome, &ufs) };
    Eleicao { codigo: txt(el, "cd").parse().unwrap_or(0), turno: txt(el, "t").parse().unwrap_or(1), tipo, local, nome, cargos, ufs, municipios: municipios.into_iter().collect() }
}

/// Nome curto e título de um pleito a partir das eleições.
fn nomes(eleicoes: &[Eleicao], ano: i64, turno: u8) -> (String, String, usize) {
    let ordinarias: Vec<&Eleicao> = eleicoes.iter().filter(|e| e.tipo == Tipo::Ordinaria).collect();
    let outras: Vec<&Eleicao> = eleicoes.iter().filter(|e| e.tipo != Tipo::Ordinaria).collect();
    if !ordinarias.is_empty() {
        let geral = ordinarias.iter().any(|e| e.nome.contains("Federal") || e.nome.contains("Estadual"));
        let tipo = if geral { "Eleições Gerais" } else { "Eleições Municipais" };
        return (format!("{turno}º turno de {ano}"), format!("{tipo} {ano} · {turno}º turno"), outras.len());
    }
    let locais: Vec<String> = outras.iter().map(|e| e.local.clone()).filter(|l| !l.is_empty()).collect();
    let sup = outras.iter().filter(|e| e.tipo == Tipo::Suplementar).count();
    let con = outras.iter().filter(|e| e.tipo == Tipo::Consulta).count();
    let t2 = if turno > 1 { format!(" · {turno}º turno") } else { String::new() };
    let (curto, longo) = match (sup, con, outras.len()) {
        (1, 0, 1) => ("Suplementar de", "Eleição suplementar"),
        (_, 0, _) => ("Suplementares de", "Eleições suplementares"),
        (0, 1, 1) => ("Consulta de", "Consulta popular"),
        (0, _, _) => ("Consultas de", "Consultas populares"),
        _ => ("Suplementares e consultas de", "Eleições suplementares e consultas"),
    };
    let l = juntar(&locais);
    (format!("{curto} {l}{t2}"), format!("{longo} · {l}{t2}"), 0)
}

/// Os pleitos de um `ele-c.json`, classificados no instante `agora` (hora de Brasília), do mais recente para o mais
/// antigo (data, e então código).
pub fn descobrir(ele_c: &[u8], agora: i64, op: &Opcoes) -> anyhow::Result<Vec<Pleito>> {
    let v: Value = serde_json::from_slice(ele_c).map_err(|e| anyhow::anyhow!("ele-c.json ilegível: {e}"))?;
    let pl = v["pl"].as_array().ok_or_else(|| anyhow::anyhow!("ele-c.json sem a lista de pleitos (pl)"))?;
    let mut lista = vec![];
    for p in pl {
        let Ok(codigo) = txt(p, "cd").parse::<u32>() else { continue };
        let data = txt(p, "dt");
        let Some(situacao) = classificar(&data, agora, op.fim_apuracao_h) else { continue };
        let (aa, mm, dd) = { let (a, m, d, ..) = urna_velox::metricas::civil(inicio_do_dia(&data).unwrap_or(0)); (a, m, d) };
        let eleicoes: Vec<Eleicao> = p["e"].as_array().into_iter().flatten().map(eleicao_de).collect();
        let turno = eleicoes.iter().filter(|e| e.tipo == Tipo::Ordinaria).map(|e| e.turno).max()
            .or_else(|| eleicoes.iter().map(|e| e.turno).max()).unwrap_or(1);
        let (nome, titulo, complementares) = nomes(&eleicoes, aa, turno);
        let ufs: BTreeSet<String> = eleicoes.iter().flat_map(|e| e.ufs.iter().cloned()).collect();
        let replay = op.acervo.as_deref().is_some_and(|a| acervo_pronto(a, codigo));
        let acervo = if replay { op.acervo.as_deref().and_then(|a| info_acervo(a, codigo)) } else { None };
        lista.push(Pleito {
            codigo, ciclo: txt(p, "c"), data_iso: format!("{aa:04}-{mm:02}-{dd:02}"), data, ano: aa, turno, situacao, nome, titulo,
            complementares, eleicoes, ufs: ufs.into_iter().collect(), replay, acervo,
        });
    }
    lista.sort_by(|a, b| b.data_iso.cmp(&a.data_iso).then(b.codigo.cmp(&a.codigo)));
    Ok(lista)
}

/// Acervo de um pleito pronto para o replay (`<acervo>/<pleito>/.completo`, o mesmo teste do `uv-ciclo.sh`).
pub fn acervo_pronto(acervo: &Path, pleito: u32) -> bool { acervo.join(pleito.to_string()).join(".completo").is_file() }

/// Do `<acervo>/<pleito>/acervo.json`, só o que a agenda usa (`secoes`, `tempo_cadencia_s`, `montado`); os acervos
/// montados à mão (452, 453, 3220, 3221) não têm o arquivo.
fn info_acervo(acervo: &Path, pleito: u32) -> Option<Value> {
    let v: Value = serde_json::from_slice(&std::fs::read(acervo.join(pleito.to_string()).join("acervo.json")).ok()?).ok()?;
    let mut o = serde_json::Map::new();
    for k in ["secoes", "tempo_cadencia_s", "montado"] { if !v[k].is_null() { o.insert(k.into(), v[k].clone()); } }
    Some(Value::Object(o))
}

/// O `pleitos.json`: a lista e a contagem por situação, com o instante da classificação e a geração do `ele-c.json`.
pub fn json(lista: &[Pleito], agora: i64, ele_c: &[u8]) -> Value {
    let fonte: Value = serde_json::from_slice(ele_c).unwrap_or(Value::Null);
    let n = |s: Situacao| lista.iter().filter(|p| p.situacao == s).count();
    serde_json::json!({
        "versao": 1,
        "gerado": crate::texto_hora(agora),
        "ele_c": { "dg": txt(&fonte, "dg"), "hg": txt(&fonte, "hg") },
        "contagem": { "agendado": n(Situacao::Agendado), "em_curso": n(Situacao::EmCurso), "encerrado": n(Situacao::Encerrado),
                      "replay": lista.iter().filter(|p| p.replay).count() },
        "pleitos": lista,
    })
}

/// Grava `conteudo` em `destino` sem deixar um arquivo pela metade (temporário ao lado + `rename`).
pub fn gravar_atomico(destino: &Path, conteudo: &[u8]) -> std::io::Result<()> {
    if let Some(d) = destino.parent() { if !d.as_os_str().is_empty() { std::fs::create_dir_all(d)?; } }
    let tmp = destino.with_extension(format!("tmp{}", std::process::id()));
    std::fs::write(&tmp, conteudo)?;
    std::fs::rename(&tmp, destino)
}

#[cfg(test)]
mod testes {
    use super::*;

    const ELE_C: &[u8] = include_bytes!("../tests/fixtures/ele-c-2026-10-29.json");

    fn t(s: &str) -> i64 { urna_velox::metricas::segundos(s).unwrap() }
    fn achar(l: &[Pleito], c: u32) -> &Pleito { l.iter().find(|p| p.codigo == c).unwrap() }

    #[test]
    fn classificacao_pela_data() {
        let d = "25/10/2026";
        assert_eq!(classificar(d, t("24/10/2026 23:59:59"), 0), Some(Situacao::Agendado));
        assert_eq!(classificar(d, t("25/10/2026 00:00:00"), 0), Some(Situacao::EmCurso));
        assert_eq!(classificar(d, t("26/10/2026 07:59:59"), 0), Some(Situacao::EmCurso));
        assert_eq!(classificar(d, t("26/10/2026 08:00:00"), 0), Some(Situacao::Encerrado));
        assert_eq!(classificar(d, t("25/10/2026 12:00:00"), 6), Some(Situacao::Encerrado), "fim da apuração configurável");
        assert_eq!(classificar("xx", 0, 0), None);
    }

    #[test]
    fn ele_c_real_de_29_10_2026() {
        let l = descobrir(ELE_C, t("29/10/2026 12:00:00"), &Opcoes::default()).unwrap();
        assert_eq!(l.len(), 43, "pleitos no ele-c.json de 29/10/2026");
        assert!(l.iter().all(|p| p.situacao == Situacao::Encerrado), "nenhum pleito futuro publicado em 29/10/2026");
        assert_eq!(l[0].codigo, 3221, "o mais recente primeiro");
        let p = achar(&l, 3221);
        assert_eq!((p.nome.as_str(), p.titulo.as_str(), p.turno, p.ano), ("2º turno de 2026", "Eleições Gerais 2026 · 2º turno", 2, 2026));
        assert_eq!(p.data_iso, "2026-10-25");
        assert_eq!(p.complementares, 10, "suplementares e consultas do mesmo dia");
        let p = achar(&l, 3220);
        assert_eq!(p.nome, "1º turno de 2026");
        let cargos: Vec<u8> = p.eleicoes.iter().flat_map(|e| e.cargos.iter().map(|c| c.codigo)).collect();
        assert_eq!(cargos, vec![1, 3, 5, 25], "proporcionais (6, 7, 8) de fora");
        let p = achar(&l, 452);
        assert_eq!((p.nome.as_str(), p.titulo.as_str()), ("1º turno de 2024", "Eleições Municipais 2024 · 1º turno"));
        assert_eq!(achar(&l, 453).nome, "2º turno de 2024");
        let p = achar(&l, 3080);
        assert_eq!((p.nome.as_str(), p.titulo.as_str(), p.ano), ("Suplementar de Paranhos (MS)", "Eleição suplementar · Paranhos (MS)", 2025));
        assert_eq!(p.eleicoes[0].municipios, vec![91847]);
        assert_eq!(p.eleicoes[0].cargos[0].nome, "Prefeito");
        assert_eq!(achar(&l, 3040).nome, "Suplementares de Neves Paulista (SP) e Eldorado (SP)");
        let p = achar(&l, 3237);
        assert_eq!((p.nome.as_str(), p.ufs.clone()), ("Suplementar de Roraima", vec!["rr".to_string()]), "suplementar estadual: o nome já é a UF");
        assert_eq!(p.eleicoes[0].cargos[0].codigo, 3);
        assert_eq!(achar(&l, 3235).nome, "Suplementar de Amparo do Serra (MG)", "preposição em minúscula");
        assert_eq!(achar(&l, 3211).nome, "Suplementar de Cruzeiro do Iguaçu (PR)");
    }

    #[test]
    fn agendado_e_em_curso() {
        // o mesmo arquivo visto em 01/10/2026 (antes do 1º turno) e na noite do 2º turno
        let l = descobrir(ELE_C, t("01/10/2026 10:00:00"), &Opcoes::default()).unwrap();
        assert_eq!(achar(&l, 3220).situacao, Situacao::Agendado);
        assert_eq!(achar(&l, 3221).situacao, Situacao::Agendado);
        assert_eq!(achar(&l, 452).situacao, Situacao::Encerrado);
        let l = descobrir(ELE_C, t("25/10/2026 19:00:00"), &Opcoes::default()).unwrap();
        assert_eq!(achar(&l, 3221).situacao, Situacao::EmCurso);
        assert_eq!(achar(&l, 3220).situacao, Situacao::Encerrado);
    }

    #[test]
    fn acervo_marca_o_replay_e_json() {
        let dir = std::env::temp_dir().join(format!("uv-pleitos-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("453")).unwrap();
        std::fs::write(dir.join("453/.completo"), "").unwrap();
        std::fs::write(dir.join("453/acervo.json"), r#"{"pleito":453,"secoes":97392,"tempo_cadencia_s":7200,"ciclo":"ele2024"}"#).unwrap();
        std::fs::create_dir_all(dir.join("452")).unwrap(); // sem .completo: não serve
        let agora = t("29/10/2026 12:00:00");
        let l = descobrir(ELE_C, agora, &Opcoes { acervo: Some(dir.clone()), ..Default::default() }).unwrap();
        assert!(achar(&l, 453).replay);
        assert!(!achar(&l, 452).replay);
        assert_eq!(achar(&l, 453).acervo, Some(serde_json::json!({"secoes": 97392, "tempo_cadencia_s": 7200})));
        assert_eq!(achar(&l, 452).acervo, None);
        let j = json(&l, agora, ELE_C);
        assert_eq!(j["contagem"]["encerrado"], 43);
        assert_eq!(j["contagem"]["replay"], 1);
        assert_eq!(j["ele_c"]["dg"], "18/10/2026");
        assert_eq!(j["pleitos"][0]["situacao"], "encerrado");
        assert_eq!(j["pleitos"][0]["eleicoes"][0]["tipo"], "suplementar");
        let arq = dir.join("saida/pleitos.json");
        gravar_atomico(&arq, serde_json::to_string(&j).unwrap().as_bytes()).unwrap();
        assert!(arq.is_file());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ele_c_invalido() {
        assert!(descobrir(b"{}", 0, &Opcoes::default()).is_err());
        assert!(descobrir(b"nada", 0, &Opcoes::default()).is_err());
    }
}
