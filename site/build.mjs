// Build da página (etapa 5): junta os módulos de src/ em dist/app.js (esbuild, sem minificar: a página é também modelo
// para quem integra) e copia index.html, estilo.css, a fonte embutida (Archivo, OFL) e o ícone. O `uv-servico`
// embute dist/ com include_bytes! (aquisicao/src/pagina.rs): depois de mudar algo aqui, rode `npm run build` e
// recompile o binário. Nada externo em tempo de execução.
import { build } from 'esbuild';
import { copyFileSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';

const raiz = new URL('.', import.meta.url).pathname;
const dist = `${raiz}dist/`;
mkdirSync(dist, { recursive: true });

await build({
  entryPoints: [`${raiz}src/app.js`],
  bundle: true,
  format: 'esm',
  target: ['es2022', 'chrome110', 'safari16', 'firefox115'],
  outfile: `${dist}app.js`,
  minify: false,
  legalComments: 'none',
  charset: 'utf8',
  banner: { js: '// Urna Velox — página de resultados (gerado por site/build.mjs a partir de site/src; não edite aqui)' },
});

copyFileSync(`${raiz}src/index.html`, `${dist}index.html`);
copyFileSync(`${raiz}src/estilo.css`, `${dist}estilo.css`);
copyFileSync(`${raiz}src/icone.svg`, `${dist}icone.svg`);
copyFileSync(`${raiz}node_modules/@fontsource-variable/archivo/files/archivo-latin-wdth-normal.woff2`, `${dist}archivo.woff2`);
writeFileSync(`${dist}LICENCA-FONTE.txt`, readFileSync(`${raiz}node_modules/@fontsource-variable/archivo/LICENSE`, 'utf8'));
console.log('dist/ pronto');
