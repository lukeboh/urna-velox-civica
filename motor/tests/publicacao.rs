//! Publicação em pasta e conferência dos arquivos gravados, publicação sem candidatos, reprodução da noite com a
//! antecedência sobre o TSE e leitura dos pacotes — sobre os fixtures.

mod comum;
use comum::{fixtures, totalizar};
use std::sync::atomic::Ordering::Relaxed;
use urna_velox::acompanhamento::Acompanhamento;
use urna_velox::candidatos::Candidatos;
use urna_velox::conferencia::conferir_pasta_publicacoes;
use urna_velox::noite::{antecedencias, estatistica, fuso, reproduzir};
use urna_velox::oficial::{Abrangencia, Oficial};
use urna_velox::resultado::{Momento, Publicador};

fn temp(nome: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("uv-pub-{}-{nome}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

#[test]
fn gravar_e_conferir_a_pasta() {
    let d = fixtures().join("2024-1t");
    let c = totalizar(&d.join("dados"), &[d.join("metricas.csv")], &[], 2);
    let ofs = Oficial::ler_pasta(&d.join("oficial")).unwrap();
    let cad = Candidatos::dos_oficiais(&ofs);
    let p = Publicador::novo(&c.cat, Some(&cad), Some(&c.met));
    let dir = temp("1t");
    let n = p.gravar_tudo(&c.est, None, Momento { encerrada: true, seq: c.est.bus.load(Relaxed), ultimo_bu: None }, false, &dir).unwrap();
    assert_eq!(n, 10);
    let r = conferir_pasta_publicacoes(&dir, &ofs);
    assert!(r.ok(), "{}\n{}", r.resumo(), r.detalhes(10));
    // uma publicação não encerrada não serve como resultado final
    let p2 = Publicador::novo(&c.cat, Some(&cad), Some(&c.met));
    p2.gravar_tudo(&c.est, None, Momento { encerrada: false, seq: 1, ultimo_bu: None }, true, &dir).unwrap();
    let r = conferir_pasta_publicacoes(&dir, &ofs);
    assert!(!r.ok() && r.divergentes.iter().all(|x| x.difs.iter().any(|d| d == "publicação não encerrada")));
    // sem a pasta
    let r = conferir_pasta_publicacoes(&dir.join("nao-existe"), &ofs);
    assert_eq!(r.divergentes.len(), 10);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn publicacao_sem_cadastro() {
    let d = fixtures().join("2024-2t");
    let c = totalizar(&d.join("dados"), &[d.join("metricas.csv")], &[], 1);
    let p = Publicador::novo(&c.cat, None, None);
    assert_eq!(p.cargos(), vec![11]);
    assert_eq!(p.abrangencias(11, false), vec![Abrangencia::Municipio(*b"ba", 34134)]);
    assert_eq!(p.abrangencias(1, false), vec![]);
    let v = p.valor(&c.est, None, &Abrangencia::Municipio(*b"ba", 34134), 11, Momento::default()).unwrap();
    let cand = v["cand"].as_array().unwrap();
    assert_eq!((cand.len(), cand[0]["n"].as_u64(), cand[0]["dvt"].is_null(), v["eleicao"].as_u64()), (2, Some(13), true, Some(0)));
    assert_eq!(v["validos"].as_u64(), Some(cand.iter().map(|x| x["v"].as_u64().unwrap()).sum()));
    // UF (soma dos municípios) e Brasil
    let uf = p.valor(&c.est, None, &Abrangencia::Uf(*b"ba"), 11, Momento::default()).unwrap();
    let br = p.totais(&c.est, &Abrangencia::Brasil, c.cat.ci(11).unwrap()).unwrap();
    assert_eq!(uf["comparecimento"].as_u64(), Some(br.comparecimento));
    assert_eq!(p.nome_arquivo(&Abrangencia::Uf(*b"ba"), 11), "ba-c0011-e000000.json");
}

#[test]
fn noite_e_antecedencia_sobre_o_tse() {
    let d = fixtures().join("2024-2t");
    let c = totalizar(&d.join("dados"), &[d.join("metricas.csv")], &[], 1);
    let ofs = Oficial::ler_pasta(&d.join("oficial")).unwrap();
    let cad = Candidatos::dos_oficiais(&ofs);
    let (_est, ac, md) = reproduzir(&c.cat, &cad, &c.mem, &c.met.recebido, Some(&[11]));
    assert_eq!((md.bus, md.sem_horario, ac.revogacoes()), (540, 0, 0));
    let a = antecedencias(&ac, &ofs, "27/10/2024", 0.002);
    assert_eq!(a.len(), 1);
    let x = &a[0];
    assert!(x.confere && !x.retotalizado);
    // BA 34134: matematicamente eleito antes do último BU, e os dois antes da totalização do TSE
    let (mat, fim, tse) = (x.mat.unwrap(), x.fim.unwrap(), x.tse_brasilia.unwrap() as f64);
    assert!(mat < fim && fim < tse, "{mat} {fim} {tse}");
    assert_eq!(estatistica(&[3.0, 1.0, 2.0, -1.0]), "{\"n\":4,\"min\":-1,\"p50\":1.5,\"max\":3,\"media\":1,\"negativos\":1}");
    assert_eq!(estatistica(&[]), "{}");
    assert_eq!((fuso(b"ac"), fuso(b"mt"), fuso(b"sp")), (-2, -1, 0));
    // eventos: candidato e circunscrição, na mesma sequência
    assert_eq!(ac.eventos.len(), 2);
    assert_eq!(ac.eventos[0].seq, ac.eventos[1].seq);
    // reavaliar tudo de novo não muda nada nem gera eventos
    let mut ac2 = Acompanhamento::novo(&c.cat, &cad, None);
    ac2.tudo_sujo();
    ac2.atualizar(&c.est, 540, 0);
    assert_eq!((ac2.circs[0].restante, ac2.eventos.len(), ac2.circs[0].mat_circ), (0, 2, ac.circs[0].mat_circ));
    ac2.tudo_sujo();
    assert_eq!(ac2.atualizar(&c.est, 541, 1), 0);
}

#[test]
fn pacotes_varrer_e_retomar() {
    let p = fixtures().join("2024-2t").join("dados").join("ba.uvpk");
    let (regs, tam) = urna_velox::pacote::varrer(&p).unwrap();
    assert_eq!((regs.len() as u64, tam), (540, std::fs::metadata(&p).unwrap().len()));
    let ja = urna_velox::pacote::ja_coletadas(&fixtures().join("2024-2t").join("dados"));
    assert_eq!(ja.len(), 540);
    // escritor: retoma um pacote com registro incompleto no fim, descartando-o
    let dir = temp("pk");
    std::fs::create_dir_all(&dir).unwrap();
    let mut b = std::fs::read(&p).unwrap();
    b.truncate(b.len() - 10);
    std::fs::write(dir.join("ba.uvpk"), &b).unwrap();
    let mut e = urna_velox::pacote::Escritor::novo(&dir).unwrap();
    e.gravar(&regs[0], &[0x30, 0x00]).unwrap();
    e.fechar().unwrap();
    let (r2, _) = urna_velox::pacote::varrer(&dir.join("ba.uvpk")).unwrap();
    assert_eq!(r2.len(), 540); // 539 íntegros + o novo
    let mem = urna_velox::pacote::EmMemoria::carregar(&dir, Some(&["ba".to_string()])).unwrap();
    assert_eq!((mem.itens.len(), mem.bu(539)), (540, &[0x30u8, 0x00][..]));
    let _ = std::fs::remove_dir_all(&dir);
}
