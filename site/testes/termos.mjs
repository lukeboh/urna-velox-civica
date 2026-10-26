// Termos de uso e privacidade na página: desde a 0.9.19 (Luciano, 24/10/2026: "Pode ficar sempre ao fundo, não precisa
// ficar sempre visível. Atrapalha a visualização. Deixe para ser visível só quando o usuário rolar até lá embaixo. E aí
// pode estar centralizado") a linha fica no fim da página, no fluxo: com a página rolada para o alto ela não está na
// tela; rolada até o fim, os links ficam à vista, centralizados, sem cobrir o letreiro nem o player do replay.
// Apontam para /termos e /privacidade (no próprio site) e não alargam a página. Desktop e celular, temas escuro e claro,
// com e sem replay (o conteúdo é esticado no teste para a página rolar).
//   node testes/termos.mjs [--url http://127.0.0.1:18080/]   (sem --url: serve o dist/ local, sem dados)
import { chromium } from 'playwright';
import { createServer } from 'node:http';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const arg = (k) => { const i = process.argv.indexOf(`--${k}`); return i > 0 ? process.argv[i + 1] : null; };
const LICENCA = '/termos';
let url = arg('url');
let srv = null;
if (!url) {
  const dist = fileURLToPath(new URL('../dist/', import.meta.url));
  const tipos = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.svg': 'image/svg+xml', '.woff2': 'font/woff2' };
  srv = createServer((q, r) => {
    const p = q.url.split('?')[0];
    const f = p === '/' ? 'index.html' : p === '/privacidade' ? 'privacidade.html' : p === '/termos' ? 'termos.html' : p.replace(/^\/static\//, '');
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
      // conteúdo alto (como numa apuração com muitos candidatos): a página rola
      await p.evaluate(() => { const d = document.createElement('div'); d.id = 'alto-teste'; d.style.height = '2400px'; document.querySelector('#painel').append(d); });
      // rola a janela e o body (o body é quem rola nesta página)
      const rolar = (y) => p.evaluate((y) => { const alvo = y < 0 ? 1e7 : y; window.scrollTo(0, alvo); document.body.scrollTop = alvo; document.documentElement.scrollTop = alvo; }, y);
      await rolar(0);
      await p.waitForTimeout(200);
      const noAlto = await p.locator('#termos').boundingBox();
      conferir(noAlto && noAlto.y >= h, `${rot}: com a página no alto, os termos não estão na tela (y ${noAlto && Math.round(noAlto.y)} de ${h})`);
      await rolar(-1);
      await p.waitForTimeout(300);
      const a = p.locator('#termos a[rel~="license"]');
      const caixa = await a.boundingBox();
      const cx = await p.locator('#termos .termos-caixa').boundingBox();
      // (o centro da área útil: no desktop a barra de rolagem fica dentro do body, que é quem rola)
      const util = await p.evaluate(() => document.body.clientWidth);
      conferir(cx && Math.abs(cx.x + cx.width / 2 - util / 2) <= 2, `${rot}: rolada até o fim, a linha está centralizada (centro ${cx && Math.round(cx.x + cx.width / 2)} de ${util / 2})`);
      conferir(await a.isVisible() && caixa && caixa.y >= 0 && caixa.y + caixa.height <= h && caixa.x >= 0 && caixa.x + caixa.width <= w, `${rot}: link visível dentro da tela`);
      conferir(await a.getAttribute('href') === LICENCA && (await a.getAttribute('rel')).includes('license'), `${rot}: aponta para a licença`);
      conferir(/Termos de uso/.test(await a.innerText()), `${rot}: texto "Termos de uso"`);
      conferir(!cruza(caixa, await p.locator('#feed').boundingBox()), `${rot}: não cobre o letreiro`);
      const pv = p.locator('#termos a.termos-priv');
      const cpv = await pv.boundingBox();
      conferir(await pv.isVisible() && cpv && cpv.x >= 0 && cpv.x + cpv.width <= w && cpv.y + cpv.height <= h, `${rot}: link "Privacidade" visível dentro da tela`);
      conferir(await pv.getAttribute('href') === '/privacidade' && !cruza(cpv, await p.locator('#feed').boundingBox()), `${rot}: "Privacidade" aponta para /privacidade e não cobre o letreiro`);
      if (replay) {
        await p.evaluate(() => { const d = document.createElement('div'); d.className = 'player'; d.id = 'player-teste'; document.body.append(d); });
        conferir(!cruza(caixa, await p.locator('#player-teste').boundingBox()) && !cruza(cpv, await p.locator('#player-teste').boundingBox()), `${rot}: não cobre o player do replay`);
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
