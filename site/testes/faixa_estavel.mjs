// B-077 e o seletor de pleito (multipleito 1.1): a faixa do topo não mexe no que está à vista. Com Playwright/Chromium e
// **nenhum servidor** (página de `dist/`, WebSocket falso).
//   cd site && node testes/faixa_estavel.mjs
// Cenários (1366, 390 e 360 px):
//   ritmo    — mostrar e esconder a "próxima eleição AO VIVO" (promo) e o aviso "parado" não muda a posição do chip do
//              ritmo (B-077: ele pulava de x = 36 para x = 649 e de volta);
//   seletor  — desde a M-025 (30/10/2026) o seletor de pleito é a eleição da barra #eleicao, que abre a lista dos
//              pleitos; a faixa começa pelo ritmo e não passa da largura da tela.
import { chromium } from 'playwright';
import { readFileSync, existsSync } from 'node:fs';
import { dirname, join, extname } from 'node:path';
import { fileURLToPath } from 'node:url';

const AQUI = dirname(fileURLToPath(import.meta.url));
const DIST = join(AQUI, '..', 'dist');
const ORIGEM = 'http://uv.teste';
const falhas = [];
const conferir = (ok, o) => { console.log(`${ok ? 'ok   ' : 'FALHA'} ${o}`); if (!ok) falhas.push(o); };
const TIPOS = { '.js': 'text/javascript', '.css': 'text/css', '.svg': 'image/svg+xml', '.woff2': 'font/woff2', '.json': 'application/json' };
const INDEX = readFileSync(join(DIST, 'index.html'), 'utf8').replaceAll('{{versao}}', 'teste');
const PLEITOS = readFileSync(join(AQUI, '..', '..', 'deploy', 'oci', 'testes', 'fixtures', 'pleitos-2026-10-29.json'), 'utf8');
const nav = await chromium.launch(process.env.UV_CHROMIUM ? { executablePath: process.env.UV_CHROMIUM } : {});

for (const largura of [1366, 390, 360]) {
  const movel = largura < 640;
  const ctx = await nav.newContext({ viewport: { width: largura, height: movel ? 780 : 900 }, ...(movel ? { isMobile: true, hasTouch: true, deviceScaleFactor: 2 } : {}),
    timezoneId: 'America/Sao_Paulo', locale: 'pt-BR' });
  await ctx.addInitScript(() => { window.WebSocket = class { constructor() { } close() { } }; });
  await ctx.route('https://dados.urnavelox.com.br/**', (r) => r.fulfill({ status: 404, headers: { 'access-control-allow-origin': '*' }, body: '{}' }));
  await ctx.route(`${ORIGEM}/**`, async (r) => {
    const p = new URL(r.request().url()).pathname;
    if (p === '/' || p === '/index.html') return r.fulfill({ status: 200, contentType: 'text/html; charset=utf-8', body: INDEX });
    if (p.startsWith('/static/')) {
      const f = join(DIST, p.slice(8));
      if (existsSync(f)) return r.fulfill({ status: 200, contentType: TIPOS[extname(f)] || 'application/octet-stream', body: readFileSync(f) });
    }
    if (p === '/api/v1/pleitos') return r.fulfill({ status: 200, contentType: 'application/json', body: PLEITOS });
    return r.fulfill({ status: 404, contentType: 'application/json', body: '{"erro":"nao_encontrado"}' });
  });
  const pg = await ctx.newPage();
  pg.on('pageerror', (e) => conferir(false, `${largura}px: erro na página: ${e.message}`));
  await pg.goto(`${ORIGEM}/`);
  await pg.waitForSelector('#faixa .chip.ritmo');
  await pg.waitForTimeout(500);
  const x = () => pg.evaluate(() => Math.round(document.querySelector('#faixa .chip.ritmo').getBoundingClientRect().x));
  const x0 = await x();
  // o que a página faz quando a próxima eleição ao vivo é anunciada e quando o TSE para de publicar
  // no celular o aviso de TSE parado toma o lugar do ritmo (de propósito): lá só a promo
  await pg.evaluate((comParado) => {
    const f = document.querySelector('#faixa');
    const promo = f.querySelector('.chip.promo'), parado = f.querySelector('.chip.parado');
    promo.querySelector('.nome').textContent = '2º turno das Eleições Municipais 2028';
    promo.classList.remove('oculto');
    if (!comParado) return;
    parado.querySelector('span:nth-child(2)').textContent = 'TSE sem BU novo há 12 min';
    parado.classList.remove('oculto');
  }, !movel);
  await pg.waitForTimeout(200);
  const x1 = await x();
  await pg.evaluate(() => { const f = document.querySelector('#faixa'); f.querySelector('.chip.promo').classList.add('oculto'); f.querySelector('.chip.parado').classList.add('oculto'); });
  await pg.waitForTimeout(200);
  const x2 = await x();
  conferir(x0 === x1 && x1 === x2, `${largura}px ritmo: o chip não sai do lugar (x = ${x0}, ${x1}, ${x2})`);
  // seletor de pleito: desde a M-025 é a eleição da barra (#eleicao); a faixa fica só com o que muda na noite
  conferir(await pg.locator('#faixa .chip.pleito').count() === 0, `${largura}px seletor: a faixa não tem mais o chip do pleito (M-025)`);
  const primeiro = await pg.evaluate(() => [...document.querySelector('#faixa').children].find((c) => getComputedStyle(c).display !== 'none')?.classList.contains('ritmo'));
  conferir(primeiro, `${largura}px a faixa começa pelo ritmo`);
  const sel = pg.locator('#eleicao .el-pleito');
  await pg.waitForFunction(() => !!window.__uv.pleitos, null, { timeout: 5000 }).catch(() => {});
  await sel.click();
  conferir(await pg.locator('#pleitos.aberta').count() === 1, `${largura}px seletor: a eleição da barra abre a lista dos pleitos`);
  const larg = await pg.evaluate(() => document.documentElement.scrollWidth);
  conferir(larg <= largura, `${largura}px a página não rola de lado (${larg})`);
  const cls = await pg.evaluate(() => window.__uv.cls);
  conferir(cls < 0.01, `${largura}px CLS ${cls.toFixed(4)} < 0,01`);
  await ctx.close();
}
await nav.close();
console.log(falhas.length ? `\n${falhas.length} falha(s)` : '\ntodos os cenários passaram');
process.exit(falhas.length ? 1 : 0);
