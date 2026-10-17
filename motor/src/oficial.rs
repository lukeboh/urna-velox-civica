//! Arquivos oficiais de divulgação do TSE (`<abr>-c<cargo>-e<eleição>-u.json`): o oráculo das conferências e,
//! nesta etapa, a fonte dos dados que não estão nos BUs (destinação, nascimento, eleitorado, turno, vagas).
//!
//! Abrangências: `br` (Brasil), UF (`df`, `zz` = exterior) e município (`<uf><município com 5 dígitos>`).
//! Os números vêm como texto (`"1204"`); campo ausente ou vazio vale 0.

use serde_json::Value;
use std::path::{Path, PathBuf};

/// Abrangência de um resultado.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Abrangencia {
    Brasil,
    /// UF em minúsculas (`df`; `zz` = exterior)
    Uf([u8; 2]),
    /// (UF, código do município no TSE)
    Municipio([u8; 2], u32),
}

impl Abrangencia {
    /// `br`, `df`, `ac01031`
    pub fn codigo(&self) -> String {
        match self {
            Abrangencia::Brasil => "br".into(),
            Abrangencia::Uf(u) => String::from_utf8_lossy(u).into_owned(),
            Abrangencia::Municipio(u, m) => format!("{}{m:05}", String::from_utf8_lossy(u)),
        }
    }
    /// Interpreta `br`, `df` ou `ac01031` (maiúsculas ou minúsculas).
    pub fn de_codigo(s: &str) -> Option<Abrangencia> {
        let s = s.to_ascii_lowercase();
        let b = s.as_bytes();
        match b.len() {
            2 if s == "br" => Some(Abrangencia::Brasil),
            2 if b.iter().all(u8::is_ascii_lowercase) => Some(Abrangencia::Uf([b[0], b[1]])),
            7 if b[..2].iter().all(u8::is_ascii_lowercase) => Some(Abrangencia::Municipio([b[0], b[1]], s[2..].parse().ok()?)),
            _ => None,
        }
    }
}

/// Candidato (ou opção de consulta) no arquivo oficial.
#[derive(Clone, Debug, Default)]
pub struct CandOficial {
    pub numero: u32,
    /// nome de urna (`nmu`), nome (`nm`) ou, nas consultas, a descrição da opção (`ds`)
    pub nome: String,
    /// votos apurados (`vap`)
    pub vap: u64,
    /// destinação dos votos (`dvt`): "Válido", "Anulado", "Anulado sub judice"…; nas consultas, "Válido"
    pub dvt: String,
    /// situação (`st`): "Eleito", "2º turno", "Não eleito", "" (totalização não finalizada)…
    pub st: String,
    /// data de nascimento (`dt`, dd/mm/aaaa) ou vazio
    pub nascimento: String,
    /// `seq` do arquivo; 0 se ausente. **Não é a ordem do arquivo**: nas UFs do 3220 é a ordem nacional; nos municipais
    /// de 2024, um sequencial que não vai de 1 a n (a comparação ordena pelos votos, `tse_resultados::resumo`)
    pub seq: u32,
    /// % dos votos a votáveis concorrentes (`pvapn`, ou `pvap` com vírgula); None se ausente
    pub pvap: Option<f64>,
    /// eleito ou no 2º turno (`e` = "s")
    pub eleito: bool,
}

/// Número decimal do TSE: "48,24" (`pvap`) ou "48.240000000" (`pvapn`).
fn dec(v: &Value, k: &str) -> Option<f64> {
    match v.get(k) {
        Some(Value::String(s)) if !s.trim().is_empty() => s.trim().replace(',', ".").parse().ok(),
        Some(Value::Number(n)) => n.as_f64(),
        _ => None,
    }
}
fn cand_de(c: &Value, nome: String, dvt: String) -> CandOficial {
    CandOficial { numero: num(c, "n") as u32, nome, vap: num(c, "vap"), dvt, st: txt(c, "st"), nascimento: txt(c, "dt"), seq: num(c, "seq") as u32,
        pvap: dec(c, "pvapn").or_else(|| dec(c, "pvap")), eleito: txt(c, "e") == "s" }
}

/// Um arquivo `-u.json` de resultado.
#[derive(Clone, Debug, Default)]
pub struct Oficial {
    pub arquivo: PathBuf,
    pub abr: Option<Abrangencia>,
    pub eleicao: u32,
    pub cargo: u8,
    /// turno (`t`)
    pub turno: u8,
    /// totalização finalizada (`tf` = "s")
    pub finalizada: bool,
    /// motivo da totalização não finalizada (`mntf`)
    pub motivo_nao_finalizada: String,
    /// data e hora da totalização (`dt`, `ht`), na hora local da UF
    pub data: String,
    pub hora: String,
    /// consulta popular (`perg`) em vez de cargo (`carg`)
    pub consulta: bool,
    /// número de vagas (`nv`), 1 se ausente
    pub vagas: u8,
    pub cand: Vec<CandOficial>,
    /// eleitorado (`e.te`), comparecimento (`e.c`)
    pub eleitorado: u64,
    pub comparecimento: u64,
    /// brancos (`v.vb`), nulos (`v.vn`), nulos técnicos (`v.vnt`)
    pub vb: u64,
    pub vn: u64,
    pub vnt: u64,
    /// seções (`s.ts`) e seções totalizadas (`s.st`)
    pub secoes: u64,
    pub secoes_totalizadas: u64,
    /// seções não instaladas (`s.sni`, contadas em `s.st`) e o eleitorado delas (`e.esni`)
    pub nao_instaladas: u64,
    pub eleitorado_nao_instaladas: u64,
    /// abstenção (`e.a`) e votos válidos (`v.vv`)
    pub abstencao: u64,
    pub validos: u64,
    /// geração do arquivo (`dg` `hg`: "dd/mm/aaaa hh:mm:ss", hora de Brasília); vazio se ausente
    pub geracao: String,
    /// andamento da totalização (`and`: "n", "p", "f") e matematicamente definido (`md`: "e", "s", "n" ou vazio)
    pub andamento: String,
    pub md: String,
}

fn num(v: &Value, k: &str) -> u64 {
    match v.get(k) {
        Some(Value::String(s)) => s.trim().parse().unwrap_or(0),
        Some(Value::Number(n)) => n.as_u64().unwrap_or(0),
        _ => 0,
    }
}
fn txt(v: &Value, k: &str) -> String { v.get(k).and_then(Value::as_str).unwrap_or("").to_string() }

/// Percorre a árvore (`agr` → `par` → `cand`, profundidade variável) atrás das listas `cand`.
fn visitar(v: &Value, out: &mut Vec<CandOficial>) {
    match v {
        Value::Object(o) => {
            if let Some(Value::Array(cs)) = o.get("cand") {
                for c in cs {
                    let nome = { let u = txt(c, "nmu"); if u.is_empty() { txt(c, "nm") } else { u } };
                    out.push(cand_de(c, nome, txt(c, "dvt")));
                }
            }
            for x in o.values() { visitar(x, out); }
        }
        Value::Array(a) => for x in a { visitar(x, out); },
        _ => {}
    }
}

/// `ac01031-c0011-e000619-u.json` → (abrangência, cargo, eleição)
pub fn nome_arquivo(nome: &str) -> Option<(Abrangencia, u8, u32)> {
    let base = nome.strip_suffix("-u.json")?;
    let mut it = base.rsplitn(3, '-');
    let e = it.next()?.strip_prefix('e')?.parse().ok()?;
    let c = it.next()?.strip_prefix('c')?.parse().ok()?;
    let abr = Abrangencia::de_codigo(it.next()?)?;
    Some((abr, c, e))
}

impl Oficial {
    pub fn de_json(v: &Value) -> Oficial {
        let mut o = Oficial {
            eleicao: num(v, "ele") as u32,
            turno: num(v, "t").max(1) as u8,
            finalizada: txt(v, "tf") != "n",
            motivo_nao_finalizada: txt(v, "mntf"),
            data: txt(v, "dt"),
            hora: txt(v, "ht"),
            vagas: 1,
            ..Default::default()
        };
        if let Some(p) = v.get("perg").and_then(|p| p.get(0)) {
            o.consulta = true;
            o.cargo = num(p, "cd") as u8;
            if let Some(Value::Array(rs)) = p.get("resp") {
                for r in rs {
                    let mut c = cand_de(r, txt(r, "ds"), "Válido".into());
                    c.nascimento.clear();
                    o.cand.push(c);
                }
            }
        } else if let Some(c) = v.get("carg").and_then(|c| c.get(0)) {
            o.cargo = num(c, "cd") as u8;
            o.vagas = num(c, "nv").max(1) as u8;
            visitar(c, &mut o.cand);
        }
        let (e, vv, s) = (&v["e"], &v["v"], &v["s"]);
        o.eleitorado = num(e, "te");
        o.comparecimento = num(e, "c");
        o.vb = num(vv, "vb");
        o.vn = num(vv, "vn");
        o.vnt = num(vv, "vnt");
        o.secoes = num(s, "ts");
        o.secoes_totalizadas = num(s, "st");
        o.nao_instaladas = num(s, "sni");
        o.eleitorado_nao_instaladas = num(e, "esni");
        o.abstencao = num(e, "a");
        o.validos = num(vv, "vv");
        let (dg, hg) = (txt(v, "dg"), txt(v, "hg"));
        if !dg.is_empty() { o.geracao = format!("{dg} {hg}").trim().to_string(); }
        o.andamento = txt(v, "and");
        o.md = txt(v, "md");
        o
    }

    /// Um `-u.json` já em memória (os bytes baixados pela comparação com o TSE, etapa 7): o mesmo que [`Oficial::ler`]
    /// sem o disco. `nome` é o nome do arquivo (dá abrangência, cargo e eleição).
    pub fn de_bytes(nome: &str, b: &[u8]) -> anyhow::Result<Oficial> {
        let v: Value = serde_json::from_slice(b).map_err(|e| anyhow::anyhow!("{nome}: {e}"))?;
        let mut o = Oficial::de_json(&v);
        if let Some((abr, c, e)) = nome_arquivo(nome) {
            o.abr = Some(abr);
            if o.cargo == 0 { o.cargo = c; }
            if o.eleicao == 0 { o.eleicao = e; }
        }
        Ok(o)
    }

    pub fn ler(p: &Path) -> anyhow::Result<Oficial> {
        let nome = p.file_name().and_then(|x| x.to_str()).unwrap_or("");
        let mut o = Oficial::de_bytes(nome, &std::fs::read(p)?).map_err(|e| anyhow::anyhow!("{}: {e}", p.display()))?;
        o.arquivo = p.to_path_buf();
        Ok(o)
    }

    /// Todos os `-u.json` de uma pasta, em ordem de nome.
    pub fn ler_pasta(dir: &Path) -> anyhow::Result<Vec<Oficial>> {
        let mut ps: Vec<PathBuf> = std::fs::read_dir(dir)?.flatten().map(|e| e.path())
            .filter(|p| p.file_name().and_then(|x| x.to_str()).is_some_and(|n| n.ends_with("-u.json"))).collect();
        ps.sort();
        ps.iter().map(|p| Oficial::ler(p)).collect()
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    #[test]
    fn nomes_e_abrangencias() {
        assert_eq!(nome_arquivo("ac01031-c0011-e000619-u.json"), Some((Abrangencia::Municipio(*b"ac", 1031), 11, 619)));
        assert_eq!(nome_arquivo("br-c0001-e006257-u.json"), Some((Abrangencia::Brasil, 1, 6257)));
        assert_eq!(nome_arquivo("zz-c0001-e006257-u.json"), Some((Abrangencia::Uf(*b"zz"), 1, 6257)));
        assert_eq!(nome_arquivo("mun-e000619-cm.json"), None);
        assert_eq!(Abrangencia::Municipio(*b"ma", 7315).codigo(), "ma07315");
    }
    #[test]
    fn cargo_e_consulta() {
        let v: Value = serde_json::from_str(r#"{"ele":"633","t":"1","tf":"s","perg":[{"cd":"55","resp":[{"n":"1","ds":"SIM","st":"Eleito","vap":"4387"},{"n":"2","ds":"NÃO","st":"Não eleito","vap":"871"}]}],
            "s":{"ts":"25","st":"25"},"e":{"te":"6134","c":"5319"},"v":{"vb":"50","vn":"11","vnt":"0"}}"#).unwrap();
        let o = Oficial::de_json(&v);
        assert!(o.consulta && o.finalizada);
        assert_eq!((o.cargo, o.eleicao, o.turno, o.eleitorado, o.comparecimento, o.vb, o.vn, o.secoes_totalizadas), (55, 633, 1, 6134, 5319, 50, 11, 25));
        assert_eq!(o.cand.len(), 2);
        assert_eq!((o.cand[0].numero, o.cand[0].vap, o.cand[0].dvt.as_str()), (1, 4387, "Válido"));
        let v: Value = serde_json::from_str(r#"{"ele":"6259","t":"1","tf":"n","mntf":"Aguarde","carg":[{"cd":"5","nv":"2","agr":[{"par":[{"cand":[{"n":"277","nmu":"X","dt":"02/02/1961","dvt":"Válido","st":"","vap":"14463"}]}]}]}],"e":{},"v":{},"s":{}}"#).unwrap();
        let o = Oficial::de_json(&v);
        assert!(!o.consulta && !o.finalizada);
        assert_eq!((o.cargo, o.vagas, o.cand[0].numero, o.cand[0].nascimento.as_str()), (5, 2, 277, "02/02/1961"));
    }
    /// Campos da comparação com o TSE (etapa 7) e a leitura de memória (o mesmo resultado da leitura do arquivo).
    #[test]
    fn campos_da_comparacao_e_bytes() {
        let b = br#"{"ele":"6257","t":"1","tf":"n","and":"p","md":"s","dg":"04/10/2026","hg":"20:41:10","carg":[{"cd":"1","agr":[{"par":[{"cand":[
            {"n":"13","seq":"1","e":"s","vap":"900","pvap":"52,94","pvapn":"52.941176471"},{"n":"22","seq":"2","e":"n","vap":"800","pvap":"47,06"}]}]}]}],
            "s":{"ts":"10","st":"5","sni":"1"},"e":{"te":"2000","c":"1800","a":"150","esni":"50"},"v":{"vb":"50","vn":"40","vnt":"10","vv":"1700"}}"#;
        let o = Oficial::de_bytes("br-c0001-e006257-u.json", b).unwrap();
        assert_eq!((o.abr.clone(), o.cargo, o.eleicao), (Some(Abrangencia::Brasil), 1, 6257));
        assert_eq!((o.geracao.as_str(), o.andamento.as_str(), o.md.as_str()), ("04/10/2026 20:41:10", "p", "s"));
        assert_eq!((o.nao_instaladas, o.eleitorado_nao_instaladas, o.abstencao, o.validos), (1, 50, 150, 1700));
        assert_eq!((o.cand[0].seq, o.cand[0].eleito, o.cand[0].pvap), (1, true, Some(52.941176471)));
        assert_eq!((o.cand[1].seq, o.cand[1].eleito, o.cand[1].pvap), (2, false, Some(47.06)));
        let d = std::env::temp_dir().join(format!("uv-of-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let arq = d.join("br-c0001-e006257-u.json");
        std::fs::write(&arq, b).unwrap();
        let l = Oficial::ler(&arq).unwrap();
        let _ = std::fs::remove_dir_all(&d);
        assert_eq!((l.cand.len(), l.cand[0].vap, l.secoes_totalizadas, l.geracao), (2, 900, 5, o.geracao));
    }
}
