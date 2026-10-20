//! **Página de resultados embutida** (etapa 5, site de exemplo): os arquivos de `site/dist/` (gerados por
//! `site/build.mjs` e versionados no repositório) entram no binário com `include_bytes!` e são servidos pela camada
//! HTTP do `uv-servico` (`--http`) em `GET /` e `GET /static/<arquivo>`, a partir de uma **tabela fixa**: nenhum caminho
//! vindo do cliente chega ao disco (como o REST). A página só usa o protocolo público (`/ws`, `/api/v1/*`).
//!
//! Cabeçalhos: `Content-Security-Policy` restrita (`default-src 'self'; connect-src 'self'` — nada externo: sem CDN,
//! fonte ou rastreamento), `X-Content-Type-Options: nosniff`, `Referrer-Policy: no-referrer`. O `index.html` vai com
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
];

const INDEX: &str = include_str!("../../site/dist/index.html");
const ROBOTS: &str = include_str!("../../site/dist/robots.txt");
const SITEMAP: &str = include_str!("../../site/dist/sitemap.xml");

/// Conteúdo do `robots.txt` para SEO e rastreadores.
pub fn robots() -> &'static str { ROBOTS }

/// Conteúdo do `sitemap.xml` para indexação canônica.
pub fn sitemap() -> &'static str { SITEMAP }

/// Política de segurança da página: só a própria origem (o WebSocket da mesma origem entra em `connect-src 'self'`).
pub const CSP: &str = "default-src 'self'; connect-src 'self'; img-src 'self' data:; base-uri 'none'; form-action 'none'; frame-ancestors 'none'";

/// O `index.html` com a versão do binário no lugar de `{{versao}}` (montado uma vez).
pub fn index() -> Bytes {
    static I: OnceLock<Bytes> = OnceLock::new();
    I.get_or_init(|| Bytes::from(INDEX.replace("{{versao}}", urna_velox::VERSAO))).clone()
}

/// O estático de um caminho exato da tabela (`None` = 404).
pub fn estatico(caminho: &str) -> Option<&'static Recurso> { ESTATICOS.iter().find(|r| r.caminho == caminho) }

#[cfg(test)]
mod testes {
    use super::*;
    #[test]
    fn tabela_fixa() {
        assert!(estatico("/static/app.js").is_some());
        for c in ["/static/../Cargo.toml", "/static/", "/static/app.js/", "/static/APP.JS", "/etc/passwd", "/static/app.js%00"] {
            assert!(estatico(c).is_none(), "{c}");
        }
        let i = index();
        let s = std::str::from_utf8(&i).unwrap();
        assert!(!s.contains("{{versao}}") && s.contains(urna_velox::VERSAO));
        // a página não cita nada fora da própria origem
        for r in ESTATICOS.iter().filter(|r| r.tipo.starts_with("text/")) {
            // o espaço de nomes do SVG não é um recurso
            let t = std::str::from_utf8(r.corpo).unwrap().replace("http://www.w3.org/", "");
            assert!(!t.contains("http://") && !t.contains("https://") && !t.contains("//fonts."), "{} cita recurso externo", r.caminho);
        }
        // só os links de navegação para a licença na cópia pública (termos de uso): não são carregados pela página
        let s = s.replace("https://github.com/lukeboh/urna-velox-civica/blob/main/docs/LICENCIAMENTO.md", "")
            .replace("https://github.com/lukeboh/urna-velox-civica/blob/main/LICENSE", "");
        assert!(!s.contains("https://") && !s.contains("http://"), "a página cita endereço externo além da licença");
        assert!(s.contains("Termos de uso"), "termos de uso na página");
        let rob = robots();
        assert!(rob.contains("User-agent: *") && rob.contains("urnavelox.com.br"));
        let sm = sitemap();
        assert!(sm.contains("<urlset") && sm.contains("https://urnavelox.com.br/"));
    }
}
