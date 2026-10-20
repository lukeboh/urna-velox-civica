//! **Urna Velox** — motor de totalização eleitoral.
//!
//! Lê os Boletins de Urna (BUs) publicados pelo TSE (BER/ASN.1), totaliza num estado denso em memória
//! e consolida por abrangência (município, UF, Brasil). A aquisição dos BUs fica no crate `uv-aquisicao`
//! (binário `uv-coletor`), que entrega os BUs a este motor em pacotes `.uvpk` ou diretamente.
//!
//! Módulos:
//! * [`bu`]: leitor BER do BU (`-bu.dat` e `-busa.dat`, 2024 e 2026), sem alocação, sem pânico;
//! * [`pacote`]: pacotes `.uvpk` (um por UF, só de acréscimo) e leitura mapeada em memória;
//! * [`catalogo`]: municípios, seções, cargos e candidatos por circunscrição (o layout do estado);
//! * [`estado`]: o vetor denso de contadores, o agregador e a totalização em lote;
//! * [`resultado`]: totais por abrangência (município, UF, Brasil);
//! * [`metricas`]: seções anuladas, não instaladas e horários de recebimento (métricas da coleta);
//! * [`oficial`]: arquivos oficiais de divulgação (`-u.json`);
//! * [`candidatos`]: o que a situação precisa e não está nos BUs (destinação, nascimento, eleitorado, turno, vagas);
//! * [`situacao`]: situação final dos candidatos (eleito, 2º turno, não eleito);
//! * [`matematica`]: matematicamente eleito / matematicamente 2º turno;
//! * [`acompanhamento`]: reavaliação por lote das circunscrições e eventos (sequência e horário);
//! * [`noite`]: reprodução da noite em ordem de chegada dos BUs;
//! * [`previsao`]: previsão (estimativa com intervalo) do horário do "matematicamente" e do fim da apuração;
//! * [`conferencia`]: conferência da totalização e da situação com os arquivos oficiais.

pub mod acompanhamento;
pub mod bu;
pub mod candidatos;
pub mod catalogo;
pub mod conferencia;
pub mod estado;
pub mod matematica;
pub mod metricas;
pub mod noite;
pub mod oficial;
pub mod pacote;
pub mod previsao;
pub mod resultado;
pub mod situacao;

/// Chave de uma seção: (município, zona, seção) num `u64`.
#[inline(always)]
pub fn chave(mu: u32, zn: u16, se: u16) -> u64 { ((mu as u64) << 32) | ((zn as u64) << 16) | se as u64 }

/// Versão do motor (a mesma do arquivo `VERSION` do repositório).
pub const VERSAO: &str = env!("CARGO_PKG_VERSION");
