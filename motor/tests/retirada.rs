//! Retirar e substituir BUs no estado (etapa 3, item 1): aplicar e retirar volta exatamente ao estado zero;
//! substituir A por B é o mesmo que aplicar só B; anular depois de totalizada é o mesmo que aplicar a seção já
//! anulada; e o acompanhamento do "matematicamente" acompanha as retiradas (revogação com motivo). Fixtures
//! versionadas, sem `UV_DADOS`.

mod comum;
use comum::{fixtures, pacote_de_soltos};
use std::sync::atomic::Ordering::Relaxed;
use urna_velox::acompanhamento::Acompanhamento;
use urna_velox::candidatos::Candidatos;
use urna_velox::catalogo::Catalogo;
use urna_velox::estado::{Agregador, Aplicado, Estado};
use urna_velox::matematica::MatCirc;
use urna_velox::metricas::Metricas;
use urna_velox::oficial::Oficial;
use urna_velox::pacote::EmMemoria;

/// Todos os contadores do estado, para comparar dois estados.
fn foto(est: &Estado) -> (Vec<u32>, Vec<u32>, Vec<u32>, [u64; 7], u64, Vec<urna_velox::estado::AlertaVnt>) {
    let v = |x: &[std::sync::atomic::AtomicU32]| x.iter().map(|a| a.load(Relaxed)).collect::<Vec<_>>();
    (v(&est.tally), v(&est.aptos), v(&est.secoes),
     [est.bus.load(Relaxed), est.cargos_invalidos.load(Relaxed), est.desconhecidos.load(Relaxed), est.duplicados.load(Relaxed),
      est.fora_catalogo.load(Relaxed), est.divergentes.load(Relaxed), est.erros.load(Relaxed)],
     est.checksum(), est.lista_alertas_vnt())
}

fn zerado(est: &Estado) -> bool {
    let (t, a, s, c, ck, al) = foto(est);
    t.iter().all(|&x| x == 0) && a.iter().all(|&x| x == 0) && s.iter().all(|&x| x == 0) && c == [0; 7] && ck == 0 && al.is_empty()
}

fn conjuntos() -> Vec<(String, EmMemoria, Catalogo)> {
    let mut v = vec![];
    for nome in ["2024-1t", "2024-2t"] {
        let d = fixtures().join(nome);
        let mem = EmMemoria::carregar(&d.join("dados"), None).unwrap();
        let met = Metricas::ler(&[&d.join("metricas.csv")]).unwrap();
        let (mut cat, ileg) = Catalogo::dos_pacotes(&mem, &[]);
        assert_eq!(ileg, 0);
        cat.usar_anuladas(&met);
        v.push((nome.to_string(), mem, cat));
    }
    // 2026: BUs soltos (Presidente, Governador, Senador, Deputados; um deles do Sistema de Apuração)
    let soltos: Vec<_> = ["o03220df9701200010003-bu.dat", "o03220mg5007500470041-busa.dat"].iter().map(|n| fixtures().join("bu").join(n)).collect();
    let (_dir, mem) = pacote_de_soltos("retirada", &soltos);
    let (cat, _) = Catalogo::dos_pacotes(&mem, &[(5, 2)]);
    v.push(("2026".into(), mem, cat));
    v
}

#[test]
fn aplicar_e_retirar_volta_a_zero() {
    for (nome, mem, cat) in conjuntos() {
        let est = Estado::novo(&cat);
        let mut ag = Agregador::novo(&cat, &est);
        let anulada = |k: u64| cat.anuladas.contains(&k);
        for i in 0..mem.itens.len() {
            let k = mem.itens[i].0.chave();
            assert_eq!(est.aplicar_secao(&cat, k, mem.bu(i), anulada(k), &mut ag), Aplicado::Ok, "{nome}");
        }
        assert!(est.bus.load(Relaxed) > 0 && est.checksum() > 0, "{nome}");
        // retira em ordem inversa à da aplicação
        for i in (0..mem.itens.len()).rev() {
            let k = mem.itens[i].0.chave();
            assert!(est.aplicada(&cat, k));
            assert_eq!(est.retirar(&cat, k, mem.bu(i), anulada(k), &mut ag), Aplicado::Ok, "{nome}");
            assert!(!est.aplicada(&cat, k));
        }
        assert!(zerado(&est), "{nome}: estado não voltou a zero: {:?}", &foto(&est).3);
        // retirar de novo: a seção não tem BU
        let k = mem.itens[0].0.chave();
        assert_eq!(est.retirar(&cat, k, mem.bu(0), anulada(k), &mut ag), Aplicado::Ausente);
        assert!(zerado(&est), "{nome}");
        // e o estado serve de novo: aplicar tudo outra vez dá o mesmo da totalização em lote
        for i in 0..mem.itens.len() { est.aplicar(&cat, mem.itens[i].0.chave(), mem.bu(i), &mut ag); }
        let (lote, _) = urna_velox::estado::totalizar_lote(&cat, &mem, 2);
        assert_eq!(foto(&est), foto(&lote), "{nome}");
    }
}

#[test]
fn substituir_a_por_b_e_aplicar_so_b() {
    for (nome, mem, cat) in conjuntos() {
        // A e B: dois BUs do mesmo município; B entra no lugar de A (mesma seção)
        let n = mem.itens.len();
        let Some((ia, ib)) = (0..n).flat_map(|a| (0..n).map(move |b| (a, b)))
            .find(|&(a, b)| a != b && mem.itens[a].0.municipio == mem.itens[b].0.municipio) else { continue };
        let ka = mem.itens[ia].0.chave();
        let resto: Vec<usize> = (0..n).filter(|&i| i != ia && i != ib).collect();
        // estado 1: tudo, depois A → B
        let e1 = Estado::novo(&cat);
        let mut ag1 = Agregador::novo(&cat, &e1);
        for &i in &resto { e1.aplicar_secao(&cat, mem.itens[i].0.chave(), mem.bu(i), false, &mut ag1); }
        assert_eq!(e1.aplicar_secao(&cat, ka, mem.bu(ia), false, &mut ag1), Aplicado::Ok);
        assert_eq!(e1.substituir(&cat, ka, (mem.bu(ia), false), (mem.bu(ib), false), &mut ag1), Aplicado::Ok);
        // estado 2: tudo e só B na seção de A
        let e2 = Estado::novo(&cat);
        let mut ag2 = Agregador::novo(&cat, &e2);
        for &i in &resto { e2.aplicar_secao(&cat, mem.itens[i].0.chave(), mem.bu(i), false, &mut ag2); }
        assert_eq!(e2.aplicar_secao(&cat, ka, mem.bu(ib), false, &mut ag2), Aplicado::Ok);
        assert_eq!(foto(&e1), foto(&e2), "{nome}: substituir A por B ≠ aplicar só B");
        assert_ne!(e1.checksum(), 0);
    }
}

#[test]
fn anular_depois_e_anular_desde_o_inicio() {
    for (nome, mem, cat) in conjuntos() {
        for alvo in [0, mem.itens.len() / 2, mem.itens.len() - 1] {
            let k = mem.itens[alvo].0.chave();
            let e1 = Estado::novo(&cat);
            let mut ag1 = Agregador::novo(&cat, &e1);
            for i in 0..mem.itens.len() { e1.aplicar_secao(&cat, mem.itens[i].0.chave(), mem.bu(i), false, &mut ag1); }
            let antes = e1.checksum();
            assert_eq!(e1.substituir(&cat, k, (mem.bu(alvo), false), (mem.bu(alvo), true), &mut ag1), Aplicado::Ok);
            assert_ne!(e1.checksum(), antes, "{nome}: anular não mudou nada");
            let e2 = Estado::novo(&cat);
            let mut ag2 = Agregador::novo(&cat, &e2);
            for i in 0..mem.itens.len() { e2.aplicar_secao(&cat, mem.itens[i].0.chave(), mem.bu(i), i == alvo, &mut ag2); }
            assert_eq!(foto(&e1), foto(&e2), "{nome}: anular depois ≠ anular desde o início");
            // desfazer a anulação volta ao estado sem ela
            assert_eq!(e1.substituir(&cat, k, (mem.bu(alvo), true), (mem.bu(alvo), false), &mut ag1), Aplicado::Ok);
            assert_eq!(e1.checksum(), antes);
        }
    }
}

/// Caso real: MA 07315, zona 23, seção 354 (2024, 1º turno) foi anulada depois de recebida. Totalizada sem saber da
/// anulação e anulada depois, a conferência com o arquivo oficial do município bate (os votos viraram nulos).
#[test]
fn anulacao_real_ma_depois_de_totalizada() {
    let d = fixtures().join("2024-1t");
    let mem = EmMemoria::carregar(&d.join("dados"), Some(&["ma".to_string()])).unwrap();
    let (cat, _) = Catalogo::dos_pacotes(&mem, &[]);
    let k = urna_velox::chave(7315, 23, 354);
    let i = mem.itens.iter().position(|x| x.0.chave() == k).expect("seção anulada nas fixtures");
    let est = Estado::novo(&cat);
    let mut ag = Agregador::novo(&cat, &est);
    for j in 0..mem.itens.len() { est.aplicar_secao(&cat, mem.itens[j].0.chave(), mem.bu(j), false, &mut ag); }
    let ofs: Vec<Oficial> = Oficial::ler_pasta(&d.join("oficial")).unwrap().into_iter()
        .filter(|o| o.arquivo.file_name().unwrap().to_string_lossy().starts_with("ma07315-c0011")).collect();
    assert_eq!(ofs.len(), 1);
    let cad = Candidatos::dos_oficiais(&ofs);
    let met = Metricas::default();
    let antes = urna_velox::conferencia::conferir(&cat, &est, &mem, &ofs, &met, Some(&cad));
    assert!(!antes.ok(), "sem a anulação a conferência deveria divergir");
    assert_eq!(est.substituir(&cat, k, (mem.bu(i), false), (mem.bu(i), true), &mut ag), Aplicado::Ok);
    let depois = urna_velox::conferencia::conferir(&cat, &est, &mem, &ofs, &met, Some(&cad));
    assert!(depois.ok(), "{}", depois.detalhes(10));
}

/// O acompanhamento lê as diferenças com sinal: retirar todos os BUs zera os votos e o apurado e revoga o
/// "matematicamente", com o motivo.
#[test]
fn acompanhamento_com_retirada() {
    let d = fixtures().join("2024-2t");
    let mem = EmMemoria::carregar(&d.join("dados"), None).unwrap();
    let (cat, _) = Catalogo::dos_pacotes(&mem, &[]);
    let cad = Candidatos::dos_arquivos_oficiais(&d.join("oficial")).unwrap();
    let est = Estado::novo(&cat);
    let mut ag = Agregador::novo(&cat, &est);
    let mut ac = Acompanhamento::novo(&cat, &cad, None);
    let muni = |i: usize| cat.muni(mem.itens[i].0.municipio).unwrap() as usize;
    for i in 0..mem.itens.len() { est.aplicar_secao(&cat, mem.itens[i].0.chave(), mem.bu(i), false, &mut ag); ac.bu_aplicado(muni(i)); }
    ac.atualizar(&est, mem.itens.len() as u64, 1);
    assert_eq!(ac.circs.len(), 1);
    assert_ne!(ac.circs[0].mat_circ, MatCirc::Apurando, "fixture deveria ficar decidida");
    let apurado = ac.circs[0].apurado;
    assert!(apurado > 0);
    for i in 0..mem.itens.len() {
        est.retirar(&cat, mem.itens[i].0.chave(), mem.bu(i), false, &mut ag);
        ac.bu_alterado(muni(i), "teste: BU retirado");
    }
    ac.atualizar(&est, 0, 2);
    let c = &ac.circs[0];
    assert_eq!((c.apurado, c.votos.iter().sum::<u64>(), c.mat_circ), (0, 0, MatCirc::Apurando));
    assert!(c.revogacoes > 0);
    let rev: Vec<_> = ac.eventos.iter().filter(|e| e.estado.starts_with("revogado:")).collect();
    assert!(!rev.is_empty() && rev.iter().all(|e| e.motivo.starts_with("teste: BU retirado")), "{rev:?}");
}

/// Cache de totais por UF do `Publicador` (usado pelo `uv-servico`): com `invalidar_uf` a cada BU, os totais de UF e do
/// Brasil são os mesmos do cálculo sem cache, também depois de retirar um BU; sem invalidar, o cache fica velho (prova
/// de que ele é usado).
#[test]
fn publicador_cache_por_uf() {
    use urna_velox::oficial::Abrangencia;
    use urna_velox::resultado::Publicador;
    let soltos: Vec<_> = ["o03220df9701200010003-bu.dat", "o03220mg5007500470041-busa.dat"].iter().map(|n| fixtures().join("bu").join(n)).collect();
    let (_dir, mem) = pacote_de_soltos("cache-uf", &soltos);
    let (cat, _) = Catalogo::dos_pacotes(&mem, &[(5, 2)]);
    let est = Estado::novo(&cat);
    let mut ag = Agregador::novo(&cat, &est);
    let sem = Publicador::novo(&cat, None, None);
    let mut com = Publicador::novo(&cat, None, None);
    com.ativar_cache_uf();
    let abrs: Vec<Abrangencia> = std::iter::once(Abrangencia::Brasil).chain(cat.ufs.iter().map(|u| Abrangencia::Uf(*u))).collect();
    let cargos: Vec<usize> = (0..cat.nc()).collect();
    let confere = |rotulo: &str| {
        for a in &abrs { for &ci in &cargos { assert_eq!(com.totais(&est, a, ci), sem.totais(&est, a, ci), "{rotulo}: {a:?} cargo {ci}"); } }
    };
    confere("vazio");
    let uf = |i: usize| mem.itens[i].0.uf;
    for i in 0..mem.itens.len() {
        assert_eq!(est.aplicar_secao(&cat, mem.itens[i].0.chave(), mem.bu(i), false, &mut ag), Aplicado::Ok);
        com.invalidar_uf(uf(i));
        confere("aplicado");
    }
    // retirar o primeiro BU e invalidar a UF dele
    assert_eq!(est.retirar(&cat, mem.itens[0].0.chave(), mem.bu(0), false, &mut ag), Aplicado::Ok);
    com.invalidar_uf(uf(0));
    confere("retirado");
    // sem invalidar, o Brasil continua o de antes (cache velho)
    let br_antes = com.totais(&est, &Abrangencia::Brasil, 0);
    assert_eq!(est.aplicar_secao(&cat, mem.itens[0].0.chave(), mem.bu(0), false, &mut ag), Aplicado::Ok);
    let ci_com_votos = cargos.iter().copied().find(|&ci| sem.totais(&est, &Abrangencia::Uf(uf(0)), ci).is_some_and(|t| t.comparecimento > 0)).unwrap();
    assert_ne!(com.totais(&est, &Abrangencia::Uf(uf(0)), ci_com_votos), sem.totais(&est, &Abrangencia::Uf(uf(0)), ci_com_votos), "o cache não está sendo usado");
    assert_eq!(com.totais(&est, &Abrangencia::Brasil, 0), br_antes);
    com.invalidar_tudo();
    confere("invalidar_tudo");
}
