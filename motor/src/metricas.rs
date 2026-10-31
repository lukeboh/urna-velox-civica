//! Métricas de uma coleta pela API (`uv-coletor baixar --metricas`, uma linha por seção): o que os BUs não dizem
//! e a divulgação usa — seções **anuladas** pela Junta (BU "Excluído", votos contam como nulos), seções **não
//! instaladas** (totalizadas sem BU) e o **horário de recebimento** de cada BU no TSE (`dr`/`hr` do `aux.json`,
//! para o replay da noite).
//!
//! Colunas usadas: `uf,municipio,zona,secao,situacao,recebido_tse,disponivel_cs,erro` (o campo `erro` tem as
//! vírgulas trocadas por `;`, então o CSV se lê com um `split(',')`).

use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Chave (UF, seção) — a mesma seção não se repete entre UFs, mas o exterior (`zz`) usa códigos próprios.
pub type ChaveUf = ([u8; 2], u64);

#[derive(Default, Debug)]
pub struct Metricas {
    /// seções anuladas (situação do BU "Excluído")
    pub anuladas: HashSet<u64>,
    /// seções não instaladas: totalizadas sem BU (o TSE as conta em `s.st`)
    pub nao_instaladas: HashSet<ChaveUf>,
    /// horário de recebimento do BU no TSE (segundos, ver [`segundos`]), hora de Brasília
    pub recebido: HashMap<ChaveUf, i64>,
    /// horário em que a seção apareceu como disponível no `cs.json`
    pub disponivel: HashMap<ChaveUf, i64>,
}

impl Metricas {
    /// Lê e junta vários CSVs (o último horário lido de uma seção prevalece).
    pub fn ler(arqs: &[&Path]) -> anyhow::Result<Metricas> {
        let mut m = Metricas::default();
        for p in arqs {
            let txt = std::fs::read_to_string(p).map_err(|e| anyhow::anyhow!("{}: {e}", p.display()))?;
            let mut l = txt.lines();
            let cab: Vec<&str> = l.next().unwrap_or("").split(',').collect();
            let col = |n: &str| cab.iter().position(|x| *x == n);
            let req = |n: &str| col(n).ok_or_else(|| anyhow::anyhow!("{}: sem coluna {n}", p.display()));
            let (cu, cm, cz, cs) = (req("uf")?, req("municipio")?, req("zona")?, req("secao")?);
            let (cst, crec, cdisp, cerr) = (col("situacao"), col("recebido_tse"), col("disponivel_cs"), col("erro"));
            for x in l {
                let f: Vec<&str> = x.split(',').collect();
                let campo = |c: Option<usize>| c.and_then(|i| f.get(i).copied()).unwrap_or("");
                let uf: [u8; 2] = match f.get(cu).map(|s| s.as_bytes()) { Some([a, b]) => [*a, *b], _ => continue };
                let (Some(Ok(mu)), Some(Ok(zn)), Some(Ok(se))) = (f.get(cm).map(|s| s.parse::<u32>()), f.get(cz).map(|s| s.parse::<u16>()), f.get(cs).map(|s| s.parse::<u16>())) else { continue };
                let k = crate::chave(mu, zn, se);
                if campo(cst).starts_with("Exclu") { m.anuladas.insert(k); }
                if campo(cerr).starts_with("sem BU (situação Nao instalada") { m.nao_instaladas.insert((uf, k)); }
                if let Some(t) = segundos(campo(crec)) { m.recebido.insert((uf, k), t); }
                if let Some(t) = segundos(campo(cdisp)) { m.disponivel.insert((uf, k), t); }
            }
        }
        Ok(m)
    }

    /// Lê a lista separada por vírgulas (como nas opções `--anuladas` e `--chegadas` dos binários).
    pub fn ler_lista(arqs: &str) -> anyhow::Result<Metricas> {
        let ps: Vec<&Path> = arqs.split(',').filter(|s| !s.is_empty()).map(Path::new).collect();
        Metricas::ler(&ps)
    }

    /// Seções não instaladas por (UF, município).
    pub fn nao_instaladas_por_municipio(&self) -> HashMap<([u8; 2], u32), u64> {
        let mut out = HashMap::new();
        for (uf, k) in &self.nao_instaladas { *out.entry((*uf, (k >> 32) as u32)).or_default() += 1; }
        out
    }
}

/// Dias desde 1970-01-01 (algoritmo de Howard Hinnant), para converter "dd/mm/aaaa hh:mm:ss".
fn dias_civil(a: i64, m: i64, d: i64) -> i64 {
    let a = if m <= 2 { a - 1 } else { a };
    let era = if a >= 0 { a } else { a - 399 } / 400;
    let ano_era = a - era * 400;
    let dia_ano = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let dia_era = ano_era * 365 + ano_era / 4 - ano_era / 100 + dia_ano;
    era * 146097 + dia_era - 719468
}

/// "04/10/2026 18:33:53" → segundos desde 1970 (horário local, sem fuso: só diferenças importam).
pub fn segundos(s: &str) -> Option<i64> {
    let (d, h) = s.trim().split_once(' ')?;
    let mut dp = d.split('/').map(|x| x.parse::<i64>());
    let (dd, mm, aa) = (dp.next()?.ok()?, dp.next()?.ok()?, dp.next()?.ok()?);
    let mut hp = h.split(':').map(|x| x.parse::<i64>());
    let (hh, mi, ss) = (hp.next()?.ok()?, hp.next()?.ok()?, hp.next().and_then(|x| x.ok()).unwrap_or(0));
    Some(dias_civil(aa, mm, dd) * 86400 + hh * 3600 + mi * 60 + ss)
}

/// (ano, mês, dia, hora, minuto, segundo) de um instante em segundos (inverso de [`segundos`]).
pub fn civil(t: i64) -> (i64, i64, i64, i64, i64, i64) {
    let d = t.div_euclid(86400);
    let r = t.rem_euclid(86400);
    let z = d + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let dia = doy - (153 * mp + 2) / 5 + 1;
    let mes = if mp < 10 { mp + 3 } else { mp - 9 };
    let ano = yoe + era * 400 + if mes <= 2 { 1 } else { 0 };
    (ano, mes, dia, r / 3600, r % 3600 / 60, r % 60)
}

/// "2026-10-04 23:59"
pub fn hhmm(t: i64) -> String {
    let (a, m, d, h, mi, _) = civil(t);
    format!("{a:04}-{m:02}-{d:02} {h:02}:{mi:02}")
}

/// "2024-10-27T18:40:00-03:00" (hora de Brasília)
pub fn iso_brasilia(t: i64) -> String {
    let (a, m, d, h, mi, s) = civil(t);
    format!("{a:04}-{m:02}-{d:02}T{h:02}:{mi:02}:{s:02}-03:00")
}

/// [`iso_brasilia`] de um instante em milissegundos, com os milissegundos: "2026-10-18T20:41:03.512-03:00".
pub fn iso_brasilia_ms(ms: i64) -> String {
    let (a, m, d, h, mi, s) = civil(ms.div_euclid(1000));
    format!("{a:04}-{m:02}-{d:02}T{h:02}:{mi:02}:{s:02}.{:03}-03:00", ms.rem_euclid(1000))
}

#[cfg(test)]
mod testes;
