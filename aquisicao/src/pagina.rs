//! **Página de resultados embutida** (etapa 5, site de exemplo): os arquivos de `site/dist/` (gerados por
//! `site/build.mjs` e versionados no repositório) entram no binário com `include_bytes!` e são servidos pela camada
//! HTTP do `uv-servico` (`--http`) em `GET /`, `GET /termos` (termos de uso e licença), `GET /privacidade` (privacidade,
//! acesso e retenção de dados) e `GET /static/<arquivo>`, a partir de uma **tabela fixa**: nenhum caminho
//! vindo do cliente chega ao disco (como o REST). A página só usa o protocolo público (`/ws`, `/api/v1/*`).
//!
//! Cabeçalhos: `Content-Security-Policy` restrita (`default-src 'self'; connect-src 'self'` + a base das consultas do
//! modo HTTP, `https://dados.urnavelox.com.br` — nada mais externo: sem CDN, fonte ou rastreamento), `X-Content-Type-Options: nosniff`, `Referrer-Policy: no-referrer`. O `index.html` vai com
//! `no-cache` e cita os estáticos com `?v=<versão>`; os estáticos, com `Cache-Control` longo (`immutable`).

use bytes::Bytes;
use std::sync::OnceLock;

/// Um arquivo embutido.
pub struct Recurso {
    /// caminho na URL (`/static/app.js`)
    pub caminho: &'static str,
    pub tipo: &'static str,
    pub corpo: &'static [u8],
}

/// Tabela fixa dos estáticos (`site/dist/`).
pub const ESTATICOS: &[Recurso] = &[
    Recurso { caminho: "/static/app.js", tipo: "text/javascript; charset=utf-8", corpo: include_bytes!("../../site/dist/app.js") },
    Recurso { caminho: "/static/estilo.css", tipo: "text/css; charset=utf-8", corpo: include_bytes!("../../site/dist/estilo.css") },
    Recurso { caminho: "/static/archivo.woff2", tipo: "font/woff2", corpo: include_bytes!("../../site/dist/archivo.woff2") },
    Recurso { caminho: "/static/icone.svg", tipo: "image/svg+xml", corpo: include_bytes!("../../site/dist/icone.svg") },
    // geografia do mapa das seções (site/geo/gerar_mapa.py): a página busca quando o cartão aparece
    Recurso { caminho: "/static/mapa.json", tipo: "application/json; charset=utf-8", corpo: include_bytes!("../../site/dist/mapa.json") },
    Recurso { caminho: "/static/documento.css", tipo: "text/css; charset=utf-8", corpo: include_bytes!("../../site/dist/documento.css") },
    Recurso { caminho: "/static/documento.js", tipo: "text/javascript; charset=utf-8", corpo: include_bytes!("../../site/dist/documento.js") },
];

const INDEX: &str = include_str!("../../site/dist/index.html");
// gerados por site/documentos.mjs a partir de docs/legal/LICENCIAMENTO.md, LICENSE, LICENCA.pt-BR.md e docs/legal/PRIVACIDADE.md
const TERMOS: &str = include_str!("../../site/dist/termos.html");
const PRIVACIDADE: &str = include_str!("../../site/dist/privacidade.html");
const ROBOTS: &str = include_str!("../../site/dist/robots.txt");
const SITEMAP: &str = include_str!("../../site/dist/sitemap.xml");

/// Conteúdo do `robots.txt` para SEO e rastreadores.
pub fn robots() -> &'static str { ROBOTS }

/// Conteúdo do `sitemap.xml` para indexação canônica.
pub fn sitemap() -> &'static str { SITEMAP }

/// Política de segurança da página: só a própria origem (o WebSocket da mesma origem entra em `connect-src 'self'`),
/// e, só para `fetch`, a base das consultas do modo HTTP (B-026, passo 3 do plano de borda: o subdomínio de leitura
/// atrás da Cloudflare, o mesmo do `<meta name="uv-dados">` do `index.html`; o teste `csp_libera_a_base_das_consultas`
/// confere que os dois batem).
pub const CSP: &str = "default-src 'self'; connect-src 'self' https://dados.urnavelox.com.br; img-src 'self' data:; base-uri 'none'; form-action 'none'; frame-ancestors 'none'";

/// O `index.html` com a versão do binário no lugar de `{{versao}}` (montado uma vez).
pub fn index() -> Bytes {
    static I: OnceLock<Bytes> = OnceLock::new();
    I.get_or_init(|| Bytes::from(INDEX.replace("{{versao}}", urna_velox::VERSAO))).clone()
}

/// A página `/termos` (termos de uso e o texto integral da licença, em português e no original) com a versão.
pub fn termos() -> Bytes {
    static T: OnceLock<Bytes> = OnceLock::new();
    T.get_or_init(|| Bytes::from(TERMOS.replace("{{versao}}", urna_velox::VERSAO))).clone()
}

/// A página `/privacidade` (privacidade, acesso e retenção de dados) com a versão no lugar de `{{versao}}`.
pub fn privacidade() -> Bytes {
    static P: OnceLock<Bytes> = OnceLock::new();
    P.get_or_init(|| Bytes::from(PRIVACIDADE.replace("{{versao}}", urna_velox::VERSAO))).clone()
}

/// O estático de um caminho exato da tabela (`None` = 404).
pub fn estatico(caminho: &str) -> Option<&'static Recurso> { ESTATICOS.iter().find(|r| r.caminho == caminho) }

/// O corpo de um estático em gzip (M-017: o `mapa.json` com os locais de votação tem ~3 MB e nada na frente do serviço
/// comprimia), calculado uma vez por processo para a tabela inteira. `None` para o que já vem comprimido (fonte).
pub fn gzip(r: &Recurso) -> Option<Bytes> {
    use std::io::Write as _;
    static Z: OnceLock<std::collections::HashMap<&'static str, Bytes>> = OnceLock::new();
    Z.get_or_init(|| {
        ESTATICOS.iter().filter(|r| !r.tipo.starts_with("font/")).filter_map(|r| {
            let mut e = flate2::write::GzEncoder::new(Vec::with_capacity(r.corpo.len() / 3), flate2::Compression::new(6));
            e.write_all(r.corpo).ok()?;
            Some((r.caminho, Bytes::from(e.finish().ok()?)))
        }).collect()
    }).get(r.caminho).cloned()
}

/// O `Accept-Encoding` aceita gzip (`gzip` ou `*`, sem `q=0`)?
pub fn aceita_gzip(v: Option<&str>) -> bool {
    v.is_some_and(|v| v.split(',').any(|t| {
        let mut p = t.split(';').map(str::trim);
        let nome = p.next().unwrap_or("");
        (nome.eq_ignore_ascii_case("gzip") || nome == "*")
            && !p.any(|q| q.strip_prefix("q=").and_then(|x| x.parse::<f32>().ok()).is_some_and(|x| x <= 0.0))
    }))
}

#[cfg(test)]
mod testes;
