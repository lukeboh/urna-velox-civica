//! `uv-serie` — monta a série de consulta de um pleito (etapa 1.1, multipleito: a base do slider de cada visitante) a
//! partir do acervo de versões que o `uv-servico` grava e da saída dele. Ver `uv_aquisicao::serie` e
//! `docs/propostas/PROPOSTA_SERIE_CONSULTA.md`.
//!
//! Sai com 0 se a série foi gravada (a última versão de cada tópico idêntica à publicação final), 1 se a conferência
//! recusou (nada gravado) e 2 em erro.

use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
#[command(name = "uv-serie", version, about = "Urna Velox — série de consulta de um pleito (base do slider) a partir do acervo de versões do uv-servico")]
struct Cli {
    /// Saída do uv-servico (publicacao/, marcos.csv, eventos.csv, previsoes.csv)
    #[arg(long)]
    saida: PathBuf,
    /// Pasta do acervo de versões (padrão: <saida>/serie)
    #[arg(long)]
    acervo: Option<PathBuf>,
    /// candidatos.json do pleito (para o mapa das seções; sem ele, a série sai sem mapa)
    #[arg(long)]
    candidatos: Option<PathBuf>,
    /// Pasta da série (recriada), ex.: /srv/urna-velox/consulta/3221/serie
    #[arg(long)]
    destino: PathBuf,
    /// Passo da série em segundos da noite (60 = a última versão de cada minuto)
    #[arg(long, default_value_t = 60)]
    passo_s: i64,
    /// Lado do TSE pelas parciais oficiais de divulgação: pasta com u-*/lote-*.jsonl.gz (ex.: <pleito>/tse-parciais)
    #[arg(long)]
    tse_parciais: Option<PathBuf>,
    /// Lado do TSE reconstruído pela hora de recebimento de cada BU: pasta dos pacotes .uvpk (com --linha-do-tempo e --oficial)
    #[arg(long)]
    tse_dados: Option<PathBuf>,
    /// linha-do-tempo.csv do pleito (recebimento de cada seção no TSE), para a reconstrução
    #[arg(long)]
    linha_do_tempo: Option<PathBuf>,
    /// Pasta dos -u.json oficiais finais (conferência do lado do TSE)
    #[arg(long)]
    oficial: Option<PathBuf>,
    /// Pleito em curso (a noite ao vivo): grava mesmo com a publicação andando, com o marcador EM_CURSO (cache curto)
    #[arg(long)]
    em_curso: bool,
}

fn main() -> std::process::ExitCode {
    let a = Cli::parse();
    use uv_aquisicao::serie::FonteTse;
    // as parciais oficiais quando houver; senão, a reconstrução (se pedida)
    let tse = match (a.tse_parciais, a.tse_dados, a.linha_do_tempo) {
        (Some(p), _, _) => Some(FonteTse::Parciais(p)),
        (None, Some(dados), Some(linha_do_tempo)) => Some(FonteTse::Reconstrucao { dados, linha_do_tempo }),
        _ => None,
    };
    let o = uv_aquisicao::serie::Opcoes { saida: a.saida, acervo: a.acervo, candidatos: a.candidatos, destino: a.destino, passo_s: a.passo_s,
        tse, oficial: a.oficial, em_curso: a.em_curso };
    match uv_aquisicao::serie::montar(&o) {
        Ok((r, ok)) => {
            println!("{}", serde_json::to_string_pretty(&r).unwrap_or_default());
            if ok { std::process::ExitCode::SUCCESS } else {
                eprintln!("uv-serie: a última versão não confere com a publicação final; nada foi gravado");
                std::process::ExitCode::from(1)
            }
        }
        Err(e) => { eprintln!("uv-serie: {e:#}"); std::process::ExitCode::from(2) }
    }
}
