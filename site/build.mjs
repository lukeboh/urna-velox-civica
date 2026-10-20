// Build da página (etapa 5): junta os módulos de src/ em dist/app.js (esbuild, sem minificar: a página é também modelo
// para quem integra) e copia index.html, estilo.css, a fonte embutida (Archivo, OFL) e o ícone. O `uv-servico`
// embute dist/ com include_bytes! (aquisicao/src/pagina.rs): depois de mudar algo aqui, rode `npm run build` e
// recompile o binário. Nada externo em tempo de execução.
import { build } from 'esbuild';
import { copyFileSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const raiz = fileURLToPath(new URL('.', import.meta.url));
const dist = path.join(raiz, 'dist');
mkdirSync(dist, { recursive: true });

await build({
  entryPoints: [path.join(raiz, 'src', 'app.js')],
  bundle: true,
  format: 'esm',
  target: ['es2022', 'chrome110', 'safari16', 'firefox115'],
  outfile: path.join(dist, 'app.js'),
  minify: false,
  legalComments: 'none',
  charset: 'utf8',
  banner: { js: '// Urna Velox — página de resultados (gerado por site/build.mjs a partir de site/src; não edite aqui)' },
});

copyFileSync(path.join(raiz, 'src', 'index.html'), path.join(dist, 'index.html'));
copyFileSync(path.join(raiz, 'src', 'estilo.css'), path.join(dist, 'estilo.css'));
copyFileSync(path.join(raiz, 'src', 'icone.svg'), path.join(dist, 'icone.svg'));
copyFileSync(path.join(raiz, 'src', 'robots.txt'), path.join(dist, 'robots.txt'));
copyFileSync(path.join(raiz, 'src', 'sitemap.xml'), path.join(dist, 'sitemap.xml'));
copyFileSync(path.join(raiz, 'node_modules', '@fontsource-variable', 'archivo', 'files', 'archivo-latin-wdth-normal.woff2'), path.join(dist, 'archivo.woff2'));
writeFileSync(path.join(dist, 'LICENCA-FONTE.txt'), readFileSync(path.join(raiz, 'node_modules', '@fontsource-variable', 'archivo', 'LICENSE'), 'utf8'));
console.log('dist/ pronto');
