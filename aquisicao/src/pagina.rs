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
// gerados por site/documentos.mjs a partir de docs/LICENCIAMENTO.md, LICENSE, LICENCA.pt-BR.md e docs/PRIVACIDADE.md
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
        // M-005: o endereço oficial no <link rel="canonical"> (o antigo, no DuckDNS, responde 301 para ele no Caddy)
        assert!(s.contains(r#"<link rel="canonical" href="https://urnavelox.com.br/">"#), "falta a tag canônica");
        // só o canônico (não é carregado) e a base das consultas do modo HTTP (B-026, passo 3 do plano de borda),
        // liberada na CSP (teste abaixo); termos de uso e privacidade ficam no próprio site, sem depender do GitHub
        let s = s.replace(r#"<link rel="canonical" href="https://urnavelox.com.br/">"#, "")
            .replace(r#"<meta name="uv-dados" content="https://dados.urnavelox.com.br">"#, "");
        assert!(!s.contains("https://") && !s.contains("http://"), "a página cita endereço externo");
        assert!(s.contains("Termos de uso") && s.contains(r#"href="/termos""#) && s.contains(r#"href="/privacidade""#), "termos e privacidade na página");
        let rob = robots();
        assert!(rob.contains("User-agent: *") && rob.contains("urnavelox.com.br"));
        let sm = sitemap();
        assert!(sm.contains("<urlset") && sm.contains("https://urnavelox.com.br/"));
    }

    /// Termos e privacidade no próprio site: versão preenchida, só recursos da própria origem e nenhum link para fora
    /// (o visitante lê as regras sem depender do GitHub), o mesmo texto dos arquivos do repositório e os compromissos à
    /// vista; links para as duas no sitemap e no robots.
    #[test]
    fn termos_e_privacidade_no_site() {
        let t = termos();
        let p = privacidade();
        for (nome, b) in [("termos", &t), ("privacidade", &p)] {
            let s = std::str::from_utf8(b).unwrap();
            assert!(!s.contains("{{") && s.contains(urna_velox::VERSAO), "{nome}: molde preenchido");
            for r in ["/static/documento.css", "/static/documento.js"] { assert!(estatico(r).is_some() && s.contains(r), "{nome}: {r}"); }
            let canonico = format!(r#"<link rel="canonical" href="https://urnavelox.com.br/{nome}">"#);
            assert!(s.contains(&canonico), "{nome}: canônico");
            let s = s.replace(&canonico, "");
            assert!(!s.contains(r#"href="http"#) && !s.contains(r#"src="http"#) && !s.contains(r#"href="//"#), "{nome}: link ou recurso externo");
        }
        let t = std::str::from_utf8(&t).unwrap();
        let p = std::str::from_utf8(&p).unwrap();
        // o texto integral da licença (original e tradução), as linhas "Required Notice:" e o licenciamento
        for k in ["PolyForm Noncommercial", "Required Notice: Copyright (c) 2026 Luciano Soares Bohnert", "Acceptance", "Aceitação",
                  "Uso comercial", r#"id="licenca-pt""#, r#"id="licenca-en""#, r##"href="#licenca-pt-licença-de-distribuição""##] {
            assert!(t.contains(k), "termos sem {k}");
        }
        assert!(t.contains(r#"id="licenca-pt-licença-de-distribuição""#), "âncora interna da licença");
        assert!(p.contains("Não vendemos") && p.contains("não compartilhamos"), "compromissos na página");
        for k in ["uv-tema", "uv-comparar", "uv-ident", "uv-aviso-dia"] { assert!(p.contains(k), "preferência {k} descrita"); }
        // o mesmo texto dos arquivos do repositório (gerado deles no build: dist/ regenerado depois de mudar um deles)
        let doc = include_str!("../../docs/PRIVACIDADE.md");
        for k in ["Não vendemos", "hash com sal", "Em vigor desde 24/10/2026", "luciano.bohnert@gmail.com"] { assert!(doc.contains(k) && p.contains(k), "privacidade: {k}"); }
        assert!(include_str!("../../LICENSE").contains("## Acceptance") && include_str!("../../docs/LICENCIAMENTO.md").contains("PRIVACIDADE.md"));
        assert!(sitemap().contains("https://urnavelox.com.br/privacidade") && sitemap().contains("https://urnavelox.com.br/termos"));
        assert!(robots().contains("Allow: /privacidade$") && robots().contains("Allow: /termos$"));
    }

    /// Passo 3 do plano de borda: a página consulta a base do `<meta name="uv-dados">` (outra origem, atrás da
    /// Cloudflare) no modo HTTP (B-026); com `connect-src 'self'` o navegador bloquearia e a página cairia sempre no
    /// próprio domínio, sem o cache da borda.
    #[test]
    fn csp_libera_a_base_das_consultas() {
        let i = index();
        let s = std::str::from_utf8(&i).unwrap();
        let k = r#"<meta name="uv-dados" content=""#;
        let ini = s.find(k).expect("falta o <meta name=\"uv-dados\">") + k.len();
        let base = &s[ini..ini + s[ini..].find('"').unwrap()];
        assert_eq!(base, "https://dados.urnavelox.com.br");
        let connect = CSP.split(';').map(str::trim).find(|d| d.starts_with("connect-src")).expect("sem connect-src");
        assert!(connect.split_whitespace().any(|x| x == "'self'"), "{connect}");
        assert!(connect.split_whitespace().any(|x| x == base), "a CSP não libera a base das consultas: {connect}");
        // nada além disso: o resto continua só na própria origem
        assert!(CSP.contains("default-src 'self';"), "{CSP}");
        assert_eq!(connect.split_whitespace().count(), 3, "{connect}");
    }
}
