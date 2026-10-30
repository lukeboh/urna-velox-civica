//! **Federação de órgãos** (2.0, etapa A; ADR-022 e ADR-028; plano em `docs/handoffs/HANDOFF_0.6.0.md`).
//!
//! Vários `uv-servico`, cada um de um órgão diferente e no IP de saída desse órgão, dividem as seções entre si e
//! trocam os BUs **diretamente, nó a nó** (sem storage compartilhado): cada um guarda tudo no seu diretório e
//! totaliza tudo.
//!
//! * [`config`]: o `federacao.json` (um nó por órgão, IP de saída declarado, endereço de sincronização, tetos);
//! * [`armazem`]: o diretório local de cada nó (os próprios dados e as cópias dos pares);
//! * [`sincronia`]: HTTP só de leitura para os pares e o sincronizador (direto da origem ou por repasse);
//! * [`estado`]: quem participa agora e com que peso (*heartbeats*, IP de saída medido, pausa por 429);
//! * [`particao`]: *Rendezvous Hashing* ponderado, só com inteiros;
//! * [`no`]: o nó dentro do `uv-servico` (`--federacao`): *heartbeat*, IP de saída medido, cota do limitador,
//!   seções delegadas e troca dos resultados com os pares.
//!
//! [`config`], [`estado`], [`particao`] e [`armazem`] não falam com o TSE nem com o motor: decidem **quem coleta o
//! quê**. O [`no`] e a [`sincronia`] ligam isso ao serviço.

pub mod armazem;
pub mod config;
pub mod estado;
pub mod no;
pub mod particao;
pub mod sincronia;

use anyhow::{Context, Result};
use armazem::{Armazem, Lido};
use estado::{Batimento, BatimentoLido};

/// Prefixo dos *heartbeats* no storage.
pub const PREFIXO_NOS: &str = "nos/";

pub fn chave_batimento(no: &str) -> String { format!("{PREFIXO_NOS}{no}.json") }

/// Grava o *heartbeat* do nó.
pub fn publicar_batimento(a: &dyn Armazem, b: &Batimento) -> Result<()> {
    let bytes = serde_json::to_vec(b)?;
    a.gravar(&chave_batimento(&b.no), &bytes).map(|_| ())
}

/// Lê todos os *heartbeats*. O instante de cada um é o da gravação no storage (não o relógio do nó). Arquivos
/// ilegíveis são ignorados (um nó com defeito não derruba os outros).
pub fn ler_batimentos(a: &dyn Armazem) -> Result<Vec<BatimentoLido>> {
    let mut saida = Vec::new();
    for (chave, _) in a.listar(PREFIXO_NOS).context("listando heartbeats")? {
        if !chave.ends_with(".json") { continue; }
        if let Lido::Novo(o) = a.ler(&chave, None)? {
            if let Ok(b) = serde_json::from_slice::<Batimento>(&o.bytes) {
                // o nome do arquivo manda: um nó não publica pelo outro
                if chave_batimento(&b.no) == chave {
                    saida.push(BatimentoLido { batimento: b, recebido: o.versao.modificado });
                }
            }
        }
    }
    Ok(saida)
}

/// Estado atual da federação lido do storage, pelo relógio do storage.
pub fn estado_atual(cfg: &config::Federacao, a: &dyn Armazem) -> Result<estado::EstadoFederacao> {
    let bs = ler_batimentos(a)?;
    Ok(estado::calcular(cfg, &bs, a.agora()?))
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn tres_orgaos_pelo_storage_local() {
        let d = std::env::temp_dir().join(format!("uv-federacao-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let a = armazem::ArmazemLocal::novo(&d).unwrap();
        let cfg = config::Federacao::de_json(r#"{"versao":1,"nos":[
            {"id":"no-a","orgao":"A","ip_saida":"127.0.0.2"},
            {"id":"no-b","orgao":"B","ip_saida":"127.0.0.3"},
            {"id":"no-c","orgao":"C","ip_saida":"127.0.0.4"}]}"#).unwrap();
        for (no, ip) in [("no-a", "127.0.0.2"), ("no-b", "127.0.0.3"), ("no-c", "127.0.0.4")] {
            publicar_batimento(&a, &Batimento { no: no.into(), ip_saida_medido: ip.into(), pausa_ate: 0, seq_bus: 0, versao: "t".into(), contador: 0, sincronizando: false }).unwrap();
        }
        // um nó tentando publicar o heartbeat de outro: ignorado
        a.gravar("nos/no-a.json.bak", b"{}").unwrap();
        a.gravar("nos/intruso.json", br#"{"no":"no-b","ip_saida_medido":"9.9.9.9"}"#).unwrap();
        let e = estado_atual(&cfg, &a).unwrap();
        assert_eq!(e.ativos.len(), 3, "{:?}", e.fora);
        assert_eq!(e.rps_total(), 285.0);
        // cada nó, sozinho, chega à mesma partição
        let e2 = estado_atual(&cfg, &a).unwrap();
        for m in 0..500u32 { assert_eq!(e.particao.dono(m, 1, 1), e2.particao.dono(m, 1, 1)); }
        let _ = std::fs::remove_dir_all(&d);
    }
}
