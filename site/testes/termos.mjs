// Termos de uso na página: o link está à vista (dentro da tela), aponta para a licença na cópia pública, não cobre o
// letreiro nem o player do replay e não alarga a página. Desktop e celular, temas escuro e claro, com e sem replay.
//   node testes/termos.mjs [--url http://127.0.0.1:18080/]   (sem --url: serve o dist/ local, sem dados)
import { chromium } from 'playwright';
import { createServer } from 'node:http';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const arg = (k) => { const i = process.argv.indexOf(`--${k}`); return i > 0 ? process.argv[i + 1] : null; };
const LICENCA = 'https://github.com/lukeboh/urna-velox-civica/blob/main/docs/LICENCIAMENTO.md';
let url = arg('url');
let srv = null;
if (!url) {
  const dist = fileURLToPath(new URL('../dist/', import.meta.url));
  const tipos = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.svg': 'image/svg+xml', '.woff2': 'font/woff2' };
  srv = createServer((q, r) => {
    const p = q.url.split('?')[0];
    const f = p === '/' ? 'index.html' : p.replace(/^\/static\//, '');
    let corpo;
    try { corpo = readFileSync(path.join(dist, f)); } catch { r.writeHead(404); r.end(); return; }
    r.writeHead(200, { 'content-type': tipos[path.extname(f)] || 'text/plain' }); r.end(corpo);
  }).listen(0, '127.0.0.1');
  await new Promise((ok) => srv.once('listening', ok));
  url = `http://127.0.0.1:${srv.address().port}/`;
}
const nav = await chromium.launch();
let falhas = 0;
const conferir = (ok, msg) => { console.log(`${ok ? 'ok ' : 'FALHOU'} ${msg}`); if (!ok) falhas++; };
const cruza = (a, b) => a && b && a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height;
for (const [w, h, movel] of [[1366, 768, false], [390, 844, true], [360, 780, true]]) {
  for (const tema of ['escuro', 'claro']) {
    for (const replay of [false, true]) {
      const ctx = await nav.newContext({ viewport: { width: w, height: h }, isMobile: movel, hasTouch: movel });
      const p = await ctx.newPage();
      await p.addInitScript((t) => { try { localStorage.setItem('uv-tema', t); } catch { /* */ } }, tema);
      await p.goto(url, { waitUntil: 'domcontentloaded' });
      await p.waitForTimeout(400);
      if (replay) await p.evaluate(() => document.documentElement.classList.add('com-replay'));
      const rot = `${w}px ${tema}${replay ? ' replay' : ''}`;
      const a = p.locator('#termos a');
      const caixa = await a.boundingBox();
      conferir(await a.isVisible() && caixa && caixa.y >= 0 && caixa.y + caixa.height <= h && caixa.x >= 0 && caixa.x + caixa.width <= w, `${rot}: link visível dentro da tela`);
      conferir(await a.getAttribute('href') === LICENCA && (await a.getAttribute('rel')).includes('noopener'), `${rot}: aponta para a licença`);
      conferir(/Termos de uso/.test(await a.innerText()), `${rot}: texto "Termos de uso"`);
      conferir(!cruza(caixa, await p.locator('#feed').boundingBox()), `${rot}: não cobre o letreiro`);
      if (replay) {
        await p.evaluate(() => { const d = document.createElement('div'); d.className = 'player'; d.id = 'player-teste'; document.body.append(d); });
        conferir(!cruza(caixa, await p.locator('#player-teste').boundingBox()), `${rot}: não cobre o player do replay`);
      }
      conferir(await p.evaluate(() => document.documentElement.scrollWidth) <= w, `${rot}: não alarga a página`);
      if (!replay && tema === 'escuro') await p.screenshot({ path: `/tmp/uv-termos-${w}.png` });
      await ctx.close();
    }
  }
}
await nav.close();
if (srv) srv.close();
console.log(falhas ? `termos: ${falhas} falha(s)` : 'termos: tudo certo');
process.exit(falhas ? 1 : 0);
