//! Conferência da totalização com os arquivos oficiais de divulgação (o mesmo que `comparar_oficial.py` e
//! `comparar_municipal.py`): para cada `-u.json`, os votos de cada candidato (`vap`), brancos (`vb`), nulos
//! (`vn`), nulos técnicos (`vnt` = votos em números sem candidato apto), comparecimento (`e.c`) e seções
//! totalizadas (`s.st` = BUs + seções não instaladas) e, com um [`Candidatos`], a **situação** de cada candidato
//! (`st`) calculada pelo motor ([`situacao_final`]) com os votos da circunscrição do cargo (Presidente: Brasil;
//! os arquivos das UFs repetem a situação nacional).

use crate::candidatos::{circunscricao_de, Candidatos};
use crate::catalogo::Catalogo;
use crate::estado::Estado;
use crate::metricas::Metricas;
use crate::oficial::{Abrangencia, Oficial};
use crate::pacote::EmMemoria;
use crate::resultado::{totais, Totais};
use crate::situacao::{situacao_final, votos_dos_candidatos, Situacao};
use std::collections::{BTreeMap, HashMap, HashSet};

/// Divergências de um arquivo oficial.
#[derive(Debug, Clone)]
pub struct Divergencia {
    pub arquivo: String,
    pub cargo: u8,
    pub difs: Vec<String>,
}

#[derive(Debug, Default)]
pub struct Relatorio {
    /// arquivos conferidos
    pub arquivos: usize,
    pub identicos: usize,
    pub divergentes: Vec<Divergencia>,
    /// por cargo: (arquivos idênticos, arquivos com divergência)
    pub por_cargo: BTreeMap<u8, (usize, usize)>,
    /// arquivos com a totalização oficial não finalizada (`tf` = n)
    pub nao_finalizados: Vec<String>,
    /// arquivos sem abrangência reconhecível no nome
    pub ignorados: Vec<String>,
    /// situação dos candidatos, por cargo: (idênticas, divergentes); só arquivos com a totalização finalizada
    pub situacoes: BTreeMap<u8, (usize, usize)>,
}

impl Relatorio {
    pub fn ok(&self) -> bool { self.divergentes.is_empty() && self.arquivos > 0 }
    pub fn resumo(&self) -> String {
        let mut s = format!("{} arquivos oficiais conferidos: {} idênticos, {} com divergência", self.arquivos, self.identicos, self.divergentes.len());
        for (c, (ok, dif)) in &self.por_cargo { s += &format!("; cargo {c}: {ok}/{}", ok + dif); }
        if !self.situacoes.is_empty() {
            let (ok, dif) = self.situacoes.values().fold((0, 0), |a, x| (a.0 + x.0, a.1 + x.1));
            s += &format!(" | situação: {ok} de {} candidatos idênticas", ok + dif);
            for (c, (ok, dif)) in &self.situacoes { s += &format!("; cargo {c}: {ok}/{}", ok + dif); }
        }
        s
    }
    pub fn detalhes(&self, max: usize) -> String {
        let mut s = String::new();
        for d in self.divergentes.iter().take(max) {
            s += &format!("{} (cargo {}):\n", d.arquivo, d.cargo);
            for x in &d.difs { s += &format!("  * {x}\n"); }
        }
        s
    }
}

/// Seções com BU nos pacotes, por (UF, município), sem repetir seção.
pub fn secoes_com_bu(mem: &EmMemoria) -> HashMap<([u8; 2], u32), u64> {
    let mut vistas = HashSet::new();
    let mut out = HashMap::new();
    for (r, _, _, _) in &mem.itens {
        if vistas.insert((r.uf, r.chave())) { *out.entry((r.uf, r.municipio)).or_default() += 1; }
    }
    out
}

/// Seções totalizadas numa abrangência: BUs + não instaladas.
pub fn secoes_totalizadas(abr: &Abrangencia, com_bu: &HashMap<([u8; 2], u32), u64>, nao_inst: &HashMap<([u8; 2], u32), u64>) -> u64 {
    let f = |(uf, mu): &([u8; 2], u32)| match abr {
        Abrangencia::Brasil => true,
        Abrangencia::Uf(u) => uf == u,
        Abrangencia::Municipio(u, m) => uf == u && mu == m,
    };
    com_bu.iter().chain(nao_inst.iter()).filter(|(k, _)| f(k)).map(|(_, q)| *q).sum()
}

/// Compara os totais do Urna Velox com um arquivo oficial. `secoes`: seções totalizadas pelo Urna Velox (BUs +
/// não instaladas), ou `None` para não conferir `s.st`. Devolve as divergências (vazio = idêntico).
pub fn conferir_votos(t: &Totais, of: &Oficial, secoes: Option<u64>) -> Vec<String> {
    let mut difs = vec![];
    let lista: HashSet<u32> = of.cand.iter().map(|c| c.numero).collect();
    for c in &of.cand {
        let q = t.votos(c.numero);
        if q != c.vap { difs.push(format!("candidato {} {} ({}): oficial {}, BUs {q} (dif {:+})", c.numero, c.nome, c.dvt, c.vap, q as i64 - c.vap as i64)); }
    }
    // votos digitados em número sem candidato apto: nominais no BU, nulos técnicos (vnt) na divulgação
    let sobra: u64 = t.vnt + t.nominais.iter().filter(|(n, _)| !lista.contains(n)).map(|(_, q)| q).sum::<u64>();
    for (rot, of_v, nos) in [("brancos (vb)", of.vb, t.branco), ("nulos (vn)", of.vn, t.nulo),
                             ("nulos técnicos (vnt) = votos em números sem candidato apto", of.vnt, sobra),
                             ("comparecimento (e.c)", of.comparecimento, t.comparecimento)] {
        if of_v != nos { difs.push(format!("{rot}: oficial {of_v}, BUs {nos} (dif {:+})", nos as i64 - of_v as i64)); }
    }
    if t.legenda > 0 || t.sem_candidato > 0 { difs.push(format!("atenção: legenda {}, sem_candidato {} nos BUs", t.legenda, t.sem_candidato)); }
    if let Some(s) = secoes {
        if s != of.secoes_totalizadas { difs.push(format!("seções totalizadas (s.st): oficial {}, Urna Velox {s} (BUs + não instaladas)", of.secoes_totalizadas)); }
    }
    difs
}

/// Situação calculada pelo motor para os candidatos de um arquivo oficial × campo `st`. Devolve
/// (idênticas, divergências). `cache`: situação por (cargo, circunscrição), calculada uma vez.
pub fn conferir_situacao(cat: &Catalogo, est: &Estado, cad: &Candidatos, of: &Oficial, cache: &mut HashMap<(u8, Abrangencia), Option<HashMap<u32, Situacao>>>) -> (usize, Vec<String>) {
    let Some(abr) = &of.abr else { return (0, vec![]) };
    let circ_abr = circunscricao_de(of.cargo, abr);
    let sit = cache.entry((of.cargo, circ_abr.clone())).or_insert_with(|| {
        let circ = cad.circunscricao(of.cargo, &circ_abr)?;
        let t = totais(cat, est, &circ_abr, of.cargo).unwrap_or_default();
        let st = situacao_final(circ, &votos_dos_candidatos(circ, &t));
        Some(circ.candidatos.iter().zip(st).map(|(c, s)| (c.numero, s)).collect())
    });
    let Some(sit) = sit else { return (0, vec![format!("circunscrição {} do cargo {} fora do catálogo", circ_abr.codigo(), of.cargo)]) };
    let (mut ok, mut difs) = (0, vec![]);
    for c in &of.cand {
        let nosso = sit.get(&c.numero).map_or("(fora do catálogo)", |s| s.texto());
        if nosso == c.st { ok += 1; } else { difs.push(format!("situação do candidato {} {}: oficial '{}', Urna Velox '{nosso}'", c.numero, c.nome, c.st)); }
    }
    (ok, difs)
}

/// Confere todos os arquivos oficiais com o estado totalizado. `mem`: os BUs totalizados (para contar as
/// seções); `met`: seções não instaladas; `cad`: se houver, confere também a situação dos candidatos.
pub fn conferir(cat: &Catalogo, est: &Estado, mem: &EmMemoria, oficiais: &[Oficial], met: &Metricas, cad: Option<&Candidatos>) -> Relatorio {
    let com_bu = secoes_com_bu(mem);
    let nao_inst = met.nao_instaladas_por_municipio();
    let mut cache = HashMap::new();
    let mut r = Relatorio::default();
    for of in oficiais {
        let nome = of.arquivo.file_name().map(|x| x.to_string_lossy().into_owned()).unwrap_or_default();
        let Some(abr) = &of.abr else { r.ignorados.push(nome); continue };
        let t = totais(cat, est, abr, of.cargo).unwrap_or_default();
        let mut difs = conferir_votos(&t, of, Some(secoes_totalizadas(abr, &com_bu, &nao_inst)));
        if !of.finalizada { r.nao_finalizados.push(nome.clone()); }
        if let (Some(cad), true) = (cad, of.finalizada) {
            let (ok, d) = conferir_situacao(cat, est, cad, of, &mut cache);
            let s = r.situacoes.entry(of.cargo).or_default();
            s.0 += ok;
            s.1 += d.len();
            difs.extend(d);
        }
        r.arquivos += 1;
        let e = r.por_cargo.entry(of.cargo).or_default();
        if difs.is_empty() { r.identicos += 1; e.0 += 1; } else { e.1 += 1; r.divergentes.push(Divergencia { arquivo: nome, cargo: of.cargo, difs }); }
    }
    r
}

/// Compara uma publicação do Urna Velox (JSON de `docs/FORMATO_RESULTADO.md`) com o arquivo oficial: votos de
/// cada candidato, `vb`, `vn`, `vnt`, comparecimento, seções totalizadas e, se a totalização oficial foi
/// finalizada, a situação (`st`) de cada candidato. Devolve (situações idênticas, divergências).
pub fn conferir_publicacao(p: &serde_json::Value, of: &Oficial) -> (usize, Vec<String>) {
    let n = |v: &serde_json::Value| v.as_u64().unwrap_or(0);
    let mut difs = vec![];
    let mut sit_ok = 0;
    let cands: HashMap<u64, &serde_json::Value> = p["cand"].as_array().map(|a| a.iter().map(|c| (n(&c["n"]), c)).collect()).unwrap_or_default();
    for c in &of.cand {
        let nosso = cands.get(&(c.numero as u64));
        let v = nosso.map_or(0, |x| n(&x["v"]));
        if v != c.vap { difs.push(format!("candidato {} {}: oficial {}, publicação {v}", c.numero, c.nome, c.vap)); }
        if of.finalizada {
            let st = nosso.and_then(|x| x["st"].as_str()).unwrap_or("(ausente)");
            if st == c.st { sit_ok += 1; } else { difs.push(format!("situação do candidato {} {}: oficial '{}', publicação '{st}'", c.numero, c.nome, c.st)); }
        }
    }
    for (rot, of_v, k) in [("brancos (vb)", of.vb, "vb"), ("nulos (vn)", of.vn, "vn"), ("nulos técnicos (vnt)", of.vnt, "vnt"), ("comparecimento (e.c)", of.comparecimento, "comparecimento")] {
        if n(&p[k]) != of_v { difs.push(format!("{rot}: oficial {of_v}, publicação {}", n(&p[k]))); }
    }
    if n(&p["secoes"]["apuradas"]) != of.secoes_totalizadas { difs.push(format!("seções totalizadas: oficial {}, publicação {}", of.secoes_totalizadas, n(&p["secoes"]["apuradas"]))); }
    if p["v"].as_u64() != Some(crate::resultado::VERSAO_FORMATO as u64) { difs.push("campo v (versão do formato) ausente ou diferente".into()); }
    (sit_ok, difs)
}

/// Gera a publicação encerrada de cada arquivo oficial e a confere ([`conferir_publicacao`]).
pub fn conferir_publicacoes(pubr: &crate::resultado::Publicador, est: &Estado, oficiais: &[Oficial], mo: crate::resultado::Momento) -> Relatorio {
    let mut r = Relatorio::default();
    pubr.limpar_cache();
    for of in oficiais {
        let nome = of.arquivo.file_name().map(|x| x.to_string_lossy().into_owned()).unwrap_or_default();
        let Some(abr) = &of.abr else { r.ignorados.push(nome); continue };
        let (ok, difs) = match pubr.valor(est, None, abr, of.cargo, mo) {
            Some(v) => conferir_publicacao(&v, of),
            None => (0, vec!["cargo ou abrangência sem publicação".to_string()]),
        };
        if !of.finalizada { r.nao_finalizados.push(nome.clone()); }
        r.arquivos += 1;
        let s = r.situacoes.entry(of.cargo).or_default();
        s.0 += ok;
        s.1 += difs.iter().filter(|d| d.starts_with("situação")).count();
        let e = r.por_cargo.entry(of.cargo).or_default();
        if difs.is_empty() { r.identicos += 1; e.0 += 1; } else { e.1 += 1; r.divergentes.push(Divergencia { arquivo: nome, cargo: of.cargo, difs }); }
    }
    r
}

/// Confere arquivos de publicação já gravados (`<abr>-c<cargo>-e<eleição>.json`, ex.: os do replay) com os
/// arquivos oficiais correspondentes.
pub fn conferir_pasta_publicacoes(dir: &std::path::Path, oficiais: &[Oficial]) -> Relatorio {
    let mut r = Relatorio::default();
    for of in oficiais {
        let nome = of.arquivo.file_name().map(|x| x.to_string_lossy().into_owned()).unwrap_or_default();
        let Some(abr) = &of.abr else { r.ignorados.push(nome); continue };
        let p = dir.join(format!("{}-c{:04}-e{:06}.json", abr.codigo(), of.cargo, of.eleicao));
        let (ok, difs) = match std::fs::read(&p).ok().and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok()) {
            Some(v) => {
                let (ok, mut d) = conferir_publicacao(&v, of);
                if v["encerrada"].as_bool() != Some(true) { d.push("publicação não encerrada".into()); }
                (ok, d)
            }
            None => (0, vec![format!("{} ausente ou ilegível", p.display())]),
        };
        if !of.finalizada { r.nao_finalizados.push(nome.clone()); }
        r.arquivos += 1;
        let s = r.situacoes.entry(of.cargo).or_default();
        s.0 += ok;
        s.1 += difs.iter().filter(|d| d.starts_with("situação")).count();
        let e = r.por_cargo.entry(of.cargo).or_default();
        if difs.is_empty() { r.identicos += 1; e.0 += 1; } else { e.1 += 1; r.divergentes.push(Divergencia { arquivo: nome, cargo: of.cargo, difs }); }
    }
    r
}
