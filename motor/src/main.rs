//! `uv-motor` — binário do motor de totalização (crate `urna-velox`). Todos os executáveis do projeto começam
//! com `uv-` (`uv-motor`, `uv-coletor`).

mod backtest;

use clap::{Parser, Subcommand};
use std::fmt::Write as _;
use std::path::PathBuf;
use urna_velox::candidatos::Candidatos;
use urna_velox::catalogo::Catalogo;
use urna_velox::estado::totalizar_lote;
use urna_velox::resultado::{Momento, Publicador};
use std::sync::atomic::Ordering::Relaxed;
use urna_velox::matematica::MatCirc;
use urna_velox::metricas::{hhmm, segundos, Metricas};
use urna_velox::noite::{antecedencias, estatistica, reproduzir};
use urna_velox::oficial::Oficial;
use urna_velox::pacote::EmMemoria;

#[derive(Parser)]
#[command(name = "uv-motor", version, about = "Urna Velox — motor de totalização eleitoral")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(clap::Args, Clone)]
struct Entrada {
    /// Pasta dos pacotes .uvpk
    #[arg(long)]
    dados: PathBuf,
    /// Catálogo (`uv-coletor catalogo`); sem ele, o catálogo é montado com os próprios BUs ou pelo --candidatos
    #[arg(long)]
    catalogo: Option<PathBuf>,
    /// Arquivo de candidatos pré-eleição (candidatos.json)
    #[arg(long)]
    candidatos: Option<PathBuf>,
    /// Pasta dos arquivos oficiais (-u.json): os candidatos (destinação, nascimento, eleitorado, turno, vagas)
    #[arg(long)]
    oficial: Option<PathBuf>,
    /// CSVs de métricas da coleta, por vírgula: seções anuladas (votos viram nulos) e não instaladas
    #[arg(long, default_value = "")]
    metricas: String,
    /// Vagas por cargo, ex.: 5=2 (sem catálogo)
    #[arg(long, default_value = "")]
    vagas: String,
    #[arg(long, default_value_t = 1)]
    threads: usize,
}

struct Carregado { mem: EmMemoria, cat: Catalogo, met: Metricas, ofs: Vec<Oficial>, cad: Option<Candidatos> }

fn carregar(e: &Entrada) -> anyhow::Result<Carregado> {
    let t0 = std::time::Instant::now();
    let mem = EmMemoria::carregar(&e.dados, None)?;
    let met = Metricas::ler_lista(&e.metricas)?;
    let (cad_prep, arq_prep) = match &e.candidatos {
        Some(p) => {
            let (c, a) = Candidatos::ler(p)?;
            (Some(c), Some(a))
        }
        None => (None, None),
    };
    let mut cat = match (&e.catalogo, &arq_prep) {
        (Some(p), _) => Catalogo::carregar(p)?,
        (None, Some(arq)) => Catalogo::de_arquivo_candidatos(arq)?,
        (None, None) => Catalogo::dos_pacotes(&mem, &vagas(&e.vagas)).0,
    };
    cat.usar_anuladas(&met);
    let ofs = match &e.oficial { Some(p) => Oficial::ler_pasta(p)?, None => vec![] };
    let cad = if cad_prep.is_some() {
        cad_prep
    } else if e.oficial.is_some() {
        Some(Candidatos::dos_oficiais(&ofs))
    } else {
        None
    };
    eprintln!("{} BUs ({:.0} MB) em {}, {} municípios, {} cargos, estado de {:.1} MB, {} seções anuladas, {} não instaladas{}; carga em {:.2} s",
        mem.itens.len(), mem.bytes() as f64 / 1048576.0, e.dados.display(), cat.muni_cod.len(), cat.cargos.len(), cat.memoria_estado_mb(),
        cat.anuladas.len(), met.nao_instaladas.len(), cad.as_ref().map(|c| format!(", {} circunscrições nos candidatos", c.n_circunscricoes())).unwrap_or_default(),
        t0.elapsed().as_secs_f64());
    Ok(Carregado { mem, cat, met, ofs, cad })
}

#[derive(Subcommand)]
enum Cmd {
    /// Imprime o conteúdo de um BU (-bu.dat ou -busa.dat)
    Inspecionar { arquivo: PathBuf },
    /// Totaliza em lote os BUs dos pacotes e grava o resultado de cada cargo e abrangência (JSON,
    /// docs/arquitetura/FORMATO_RESULTADO.md)
    Totalizar {
        #[command(flatten)]
        entrada: Entrada,
        /// Pasta onde gravar os resultados (um JSON por cargo e abrangência)
        #[arg(long)]
        saida: Option<PathBuf>,
        /// Também os cargos gerais (Presidente, Governador, Senador) por município
        #[arg(long)]
        municipais_gerais: bool,
        /// Repetições da totalização (medida de desempenho)
        #[arg(long, default_value_t = 1)]
        repeticoes: usize,
    },
    /// Totaliza e confere com os arquivos oficiais: votos, brancos, nulos, nulos técnicos, comparecimento, seções e
    /// a situação de cada candidato, pelo estado e pelas publicações JSON. Sai com código 1 se houver divergência
    Conferir {
        #[command(flatten)]
        entrada: Entrada,
        /// Divergências mostradas
        #[arg(long, default_value_t = 20)]
        max: usize,
    },
    /// Confere com os arquivos oficiais resultados já publicados numa pasta (ex.: a do replay); código 1 se divergir
    ConferirPublicacao {
        /// Pasta com os JSON publicados (`<abr>-c<cargo>-e<eleição>.json`)
        #[arg(long)]
        publicacao: PathBuf,
        #[arg(long)]
        oficial: PathBuf,
        /// Só os arquivos oficiais destes cargos (padrão: todos)
        #[arg(long)]
        cargos: Option<String>,
        #[arg(long, default_value_t = 20)]
        max: usize,
    },
    /// Reproduz a noite em ordem de chegada dos BUs e mede quando cada circunscrição fica matematicamente
    /// decidida, comparando com a hora de totalização dos arquivos oficiais
    Noite {
        /// Pasta dos pacotes .uvpk
        #[arg(long)]
        dados: PathBuf,
        /// Arquivo de candidatos pré-eleição (candidatos.json)
        #[arg(long)]
        candidatos: Option<PathBuf>,
        /// Pasta dos arquivos oficiais (-u.json): candidatos (destinação, nascimento, eleitorado) e hora do TSE
        #[arg(long)]
        oficial: Option<PathBuf>,
        /// CSVs com o horário de chegada de cada BU (colunas uf,municipio,zona,secao,recebido_tse), por vírgula
        #[arg(long)]
        chegadas: String,
        /// Usa a disponibilização no cs.json quando é anterior ao recebimento do aux.json (lotes recarregados)
        #[arg(long)]
        disponivel_antes: bool,
        /// CSVs de métricas com as seções anuladas (BU "Excluído")
        #[arg(long)]
        anuladas: Option<String>,
        /// Vagas por cargo, ex.: 5=2
        #[arg(long, default_value = "")]
        vagas: String,
        /// Dia da eleição (dd/mm/aaaa): arquivos oficiais de outro dia foram retotalizados (fora das estatísticas)
        #[arg(long)]
        dia: Option<String>,
        /// Latência do Urna Velox somada a cada instante (ms)
        #[arg(long, default_value_t = 0.0)]
        latencia_ms: f64,
        #[arg(long)]
        saida: PathBuf,
    },
    /// Backtest da previsão (etapa 8): reproduz a noite em ordem de chegada e, a cada `--intervalo-s` do relógio da
    /// noite, prevê o horário do "matematicamente" e do fim da apuração só com o que se sabia; no fim compara com o
    /// que aconteceu (calibração P10–P90, erro do P50, custo). Também grava os gabaritos da noite (`--gerar-gabaritos`)
    Previsao {
        #[command(flatten)]
        noite: ArgsNoite,
        /// Nome da noite (ex.: 2026-1t): os gabaritos dela não são usados na previsão (sem olhar a própria noite)
        #[arg(long)]
        nome: String,
        /// Gabaritos (JSON, por vírgula); vazio = os embutidos no motor
        #[arg(long, default_value = "")]
        gabaritos: String,
        /// Grava os gabaritos desta noite neste arquivo
        #[arg(long)]
        gerar_gabaritos: Option<PathBuf>,
        /// Classes de gabarito geradas (br, uf, mun); numa noite só de alguns municípios (2º turno municipal), "mun"
        #[arg(long, default_value = "br,uf,mun")]
        classes: String,
        /// Intervalo entre previsões, no relógio da noite (s)
        #[arg(long, default_value_t = 30)]
        intervalo_s: i64,
        /// Só estes cargos (por vírgula)
        #[arg(long)]
        cargos: Option<String>,
        /// Conversão em horário: mista (metade dos sorteios por cada), curva (gabaritos) ou velocidade
        #[arg(long, default_value = "mista")]
        tempo: String,
        #[arg(long)]
        deriva: Option<f64>,
        #[arg(long)]
        ordem: Option<f64>,
        #[arg(long)]
        erro_rel: Option<f64>,
        #[arg(long)]
        erro_poucas: Option<f64>,
        #[arg(long)]
        sorteios: Option<usize>,
        #[arg(long, default_value_t = 1)]
        threads: usize,
        /// Não roda a previsão (só gera os gabaritos: mais rápido)
        #[arg(long)]
        sem_previsao: bool,
        /// Todos os tópicos de cada cargo (UFs e municípios, só o fim da apuração), como no serviço: medida de custo
        #[arg(long)]
        todos_topicos: bool,
        #[arg(long)]
        saida: PathBuf,
    },
}

#[derive(clap::Args, Clone)]
struct ArgsNoite {
    /// Pasta dos pacotes .uvpk
    #[arg(long)]
    dados: PathBuf,
    /// Arquivo de candidatos pré-eleição (candidatos.json)
    #[arg(long)]
    candidatos: Option<PathBuf>,
    /// Pasta dos arquivos oficiais (-u.json)
    #[arg(long)]
    oficial: Option<PathBuf>,
    /// CSVs com o horário de chegada de cada BU (colunas uf,municipio,zona,secao,recebido_tse), por vírgula
    #[arg(long)]
    chegadas: String,
    /// CSVs de métricas com as seções anuladas (BU "Excluído")
    #[arg(long)]
    anuladas: Option<String>,
    /// Vagas por cargo, ex.: 5=2
    #[arg(long, default_value = "")]
    vagas: String,
}

fn vagas(s: &str) -> Vec<(u8, u8)> {
    s.split(',').filter_map(|x| { let (a, b) = x.split_once('=')?; Some((a.trim().parse().ok()?, b.trim().parse().ok()?)) }).collect()
}

/// "2024-10-27 18:40:05"
fn hms(t: f64) -> String {
    let (a, m, d, h, mi, s) = urna_velox::metricas::civil(t.floor() as i64);
    format!("{a:04}-{m:02}-{d:02} {h:02}:{mi:02}:{s:02}")
}

fn main() -> anyhow::Result<()> {
    match Cli::parse().cmd {
        Cmd::Totalizar { entrada, saida, municipais_gerais, repeticoes } => {
            let c = carregar(&entrada)?;
            let mut ultimo = None;
            for _ in 0..repeticoes.max(1) {
                let (est, seg) = totalizar_lote(&c.cat, &c.mem, entrada.threads.max(1));
                let n = est.bus.load(Relaxed);
                println!("totalização: {n} BUs em {seg:.4} s com {} thread(s) = {:.3} µs por BU; checksum {}", entrada.threads.max(1), seg * 1e6 / n.max(1) as f64, est.checksum());
                ultimo = Some(est);
            }
            let est = ultimo.unwrap();
            println!("conferência: cargos inválidos {} | votáveis desconhecidos {} | duplicados {} | fora do catálogo {} | identificação divergente {} | erros de leitura {}",
                est.cargos_invalidos.load(Relaxed), est.desconhecidos.load(Relaxed), est.duplicados.load(Relaxed), est.fora_catalogo.load(Relaxed),
                est.divergentes.load(Relaxed), est.erros.load(Relaxed));
            if let Some(dir) = saida {
                let t0 = std::time::Instant::now();
                let p = Publicador::novo(&c.cat, c.cad.as_ref(), Some(&c.met));
                let n = p.gravar_tudo(&est, None, Momento { encerrada: true, seq: est.bus.load(Relaxed), ultimo_bu: None, totalizado_ms: None }, municipais_gerais, &dir)?;
                println!("{n} resultados gravados em {} ({:.2} s)", dir.display(), t0.elapsed().as_secs_f64());
            }
        }
        Cmd::Conferir { entrada, max } => {
            anyhow::ensure!(entrada.oficial.is_some(), "conferir precisa de --oficial");
            let c = carregar(&entrada)?;
            let (est, seg) = totalizar_lote(&c.cat, &c.mem, entrada.threads.max(1));
            println!("totalização: {} BUs em {seg:.4} s", est.bus.load(Relaxed));
            let r1 = urna_velox::conferencia::conferir(&c.cat, &est, &c.mem, &c.ofs, &c.met, c.cad.as_ref());
            println!("estado:     {}", r1.resumo());
            let p = Publicador::novo(&c.cat, c.cad.as_ref(), Some(&c.met));
            let r2 = urna_velox::conferencia::conferir_publicacoes(&p, &est, &c.ofs, Momento { encerrada: true, seq: est.bus.load(Relaxed), ultimo_bu: None, totalizado_ms: None });
            println!("publicação: {}", r2.resumo());
            if !r1.nao_finalizados.is_empty() { println!("totalização oficial não finalizada (situação não conferida): {}", r1.nao_finalizados.join(", ")); }
            print!("{}", r1.detalhes(max));
            print!("{}", r2.detalhes(max));
            if !(r1.ok() && r2.ok()) { std::process::exit(1); }
        }
        Cmd::Inspecionar { arquivo } => {
            let b = std::fs::read(&arquivo)?;
            let mut t = urna_velox::bu::Texto(String::new());
            urna_velox::bu::ler_bu(&b, &mut t)?;
            print!("{}", t.0);
        }
        Cmd::ConferirPublicacao { publicacao, oficial, cargos, max } => {
            let mut ofs = Oficial::ler_pasta(&oficial)?;
            if let Some(c) = cargos { let so: Vec<u8> = c.split(',').filter_map(|x| x.trim().parse().ok()).collect(); ofs.retain(|o| so.contains(&o.cargo)); }
            let r = urna_velox::conferencia::conferir_pasta_publicacoes(&publicacao, &ofs);
            println!("publicação: {}", r.resumo());
            print!("{}", r.detalhes(max));
            if !r.ok() { std::process::exit(1); }
        }
        Cmd::Previsao { noite, nome, gabaritos, gerar_gabaritos, classes, intervalo_s, cargos, tempo, deriva, ordem, erro_rel, erro_poucas, sorteios, threads, sem_previsao, todos_topicos, saida } => {
            use urna_velox::previsao::{Parametros, Tempo};
            let tempo = match tempo.as_str() { "velocidade" => Tempo::Velocidade, "curva" => Tempo::Curva, "mista" => Tempo::Mista, x => anyhow::bail!("--tempo {x}: curva, velocidade ou mista") };
            let mut par = Parametros { tempo, ..Parametros::default() };
            if let Some(x) = deriva { par.deriva = x; }
            if let Some(x) = ordem { par.ordem = x; }
            if let Some(x) = erro_rel { par.erro_rel = x; }
            if let Some(x) = erro_poucas { par.erro_poucas = x; }
            if let Some(x) = sorteios { par.sorteios = x; }
            let cargos = cargos.map(|c| c.split(',').filter_map(|x| x.trim().parse().ok()).collect());
            backtest::rodar(
                backtest::Entrada { dados: noite.dados, candidatos: noite.candidatos, oficial: noite.oficial, chegadas: noite.chegadas, anuladas: noite.anuladas, vagas: vagas(&noite.vagas) },
                backtest::Opcoes { nome, gabaritos, gerar: gerar_gabaritos, classes, intervalo_s, cargos, par, threads, sem_previsao, todos_topicos, saida },
            )?;
        }
        Cmd::Noite { dados, candidatos, oficial, chegadas, disponivel_antes, anuladas, vagas: v, dia, latencia_ms, saida } => {
            std::fs::create_dir_all(&saida)?;
            let mem = EmMemoria::carregar(&dados, None)?;
            let (cad_prep, arq_prep) = match &candidatos {
                Some(p) => {
                    let (c, a) = Candidatos::ler(p)?;
                    (Some(c), Some(a))
                }
                None => (None, None),
            };
            let (mut cat, ilegiveis) = match &arq_prep {
                Some(arq) => (Catalogo::de_arquivo_candidatos(arq)?, 0),
                None => Catalogo::dos_pacotes(&mem, &vagas(&v)),
            };
            if let Some(a) = &anuladas { cat.usar_anuladas(&Metricas::ler_lista(a)?); }
            let ofs = match &oficial { Some(p) => Oficial::ler_pasta(p)?, None => vec![] };
            let cad = if let Some(c) = cad_prep {
                c
            } else if oficial.is_some() {
                Candidatos::dos_oficiais(&ofs)
            } else {
                anyhow::bail!("é necessário informar --candidatos ou --oficial para o comando noite");
            };
            let met = Metricas::ler_lista(&chegadas)?;
            let mut ch = met.recebido.clone();
            let primeiro = ch.values().min().copied().unwrap_or(0);
            let dia = dia.unwrap_or_else(|| { let (a, m, d, ..) = urna_velox::metricas::civil(primeiro); format!("{d:02}/{m:02}/{a:04}") });
            let d0 = segundos(&format!("{dia} 00:00:00")).ok_or_else(|| anyhow::anyhow!("--dia inválido: {dia}"))?;
            let mut corrigidas = 0;
            if disponivel_antes {
                for (k, t) in ch.iter_mut() {
                    if let Some(&td) = met.disponivel.get(k) { if td < *t && (d0..d0 + 86400).contains(&td) { *t = td; corrigidas += 1; } }
                }
            }
            eprintln!("{} BUs ({ilegiveis} ilegíveis), {} circunscrições nos candidatos, {} horários de chegada ({corrigidas} pela disponibilização)", mem.itens.len(), cad.n_circunscricoes(), ch.len());
            let (_est, ac, md) = reproduzir(&cat, &cad, &mem, &ch, None);
            let ants = antecedencias(&ac, &ofs, &dia, latencia_ms / 1e3);
            // eventos
            let mut ev = String::from("seq,horario,abrangencia,cargo,candidato,estado\n");
            for e in &ac.eventos {
                let c = &ac.circs[e.circ];
                let _ = writeln!(ev, "{},{},{},{},{},{}", e.seq, hms(e.em as f64), c.abr().codigo(), c.cargo, e.numero.map(|n| n.to_string()).unwrap_or_default(), e.estado);
            }
            std::fs::write(saida.join("eventos.csv"), ev)?;
            // por circunscrição
            let mut cs = String::from("abrangencia,cargo,eleitorado,restante,estado,matematicamente_em,ultimo_bu,tse_totalizacao,tse_brasilia,retotalizado,antecedencia_mat_s,antecedencia_fim_s,confere\n");
            let (mut am, mut af) = (vec![], vec![]);
            let mut nao_confere = 0;
            for a in &ants {
                let c = &ac.circs[a.circ];
                let dm = a.tse_brasilia.zip(a.mat).map(|(t, m)| t as f64 - m);
                let df = a.tse_brasilia.zip(a.fim).map(|(t, f)| t as f64 - f);
                if !a.retotalizado { if let Some(x) = dm { am.push(x); } if let Some(x) = df { af.push(x); } }
                if !a.confere && c.mat_circ != MatCirc::Apurando { nao_confere += 1; }
                let f = |x: Option<f64>| x.map(|v| format!("{v:.0}")).unwrap_or_default();
                let _ = writeln!(cs, "{},{},{},{},{},{},{},{},{},{},{},{},{}", c.abr().codigo(), c.cargo, c.circ.eleitorado, c.restante, c.mat_circ.texto(),
                    a.mat.map(hms).unwrap_or_default(), a.fim.map(hms).unwrap_or_default(), a.tse, a.tse_brasilia.map(|t| hms(t as f64)).unwrap_or_default(),
                    a.retotalizado, f(dm), f(df), a.confere);
            }
            std::fs::write(saida.join("circunscricoes.csv"), cs)?;
            let decididas = ac.circs.iter().filter(|c| c.mat_circ != MatCirc::Apurando).count();
            let resumo = format!(concat!("{{\"bus\":{},\"sem_horario\":{},\"lotes\":{},\"avaliacoes\":{},\"us_por_bu\":{:.3},\"primeiro\":\"{}\",\"ultimo\":\"{}\",",
                "\"circunscricoes\":{},\"decididas\":{},\"revogacoes\":{},\"nao_confere\":{},\"retotalizadas\":{},\"latencia_ms\":{},",
                "\"antecedencia_mat_s\":{},\"antecedencia_fim_s\":{}}}\n"),
                md.bus, md.sem_horario, md.lotes, md.avaliacoes, md.us_por_bu(), hhmm(md.primeiro), hhmm(md.ultimo), ac.circs.len(), decididas, ac.revogacoes(),
                nao_confere, ants.iter().filter(|a| a.retotalizado).count(), latencia_ms, estatistica(&am), estatistica(&af));
            std::fs::write(saida.join("resumo.json"), &resumo)?;
            print!("{resumo}");
        }
    }
    Ok(())
}
