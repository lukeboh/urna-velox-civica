// T14 da validação da 1.1 (HANDOFF_1.1.0 §4): desempenho da consulta (slider) num celular modesto. Playwright/Chromium
// com a CPU 4× mais lenta (CDP), tela de 360 px, contra o site no ar (ou UV_URL). Para cada pleito: abre a consulta,
// toca a 300× por 20 s e mede.
//   cd site && node testes/consulta_desempenho.mjs [pleitos…]        (padrão: 3237 3221 3220 452)
// Critérios (decididos em 30/10/2026, revisáveis): abre (primeiro resultado) em ≤ 10 s; tocando, as tarefas longas
// (> 50 ms) ocupam ≤ 35 % do tempo e a maior fica ≤ 1 s; memória JS ≤ 200 MB; CLS < 0,01.
import { chromium } from 'playwright';
import { readFileSync, existsSync } from 'node:fs';
import { dirname, join, extname } from 'node:path';
import { fileURLToPath } from 'node:url';
// UV_DIST=1: a página desta cópia (site/dist) com os dados do site no ar (para medir uma correção antes de implantar)
const DIST = join(dirname(fileURLToPath(import.meta.url)), '..', 'dist');
const TIPOS = { '.js': 'text/javascript', '.css': 'text/css', '.svg': 'image/svg+xml', '.woff2': 'font/woff2' };

const URL0 = process.env.UV_URL || 'https://urnavelox.com.br';
const PLEITOS = process.argv.slice(2).length ? process.argv.slice(2) : ['3237', '3221', '3220', '452'];
const falhas = [];
const conferir = (ok, o) => { console.log(`${ok ? 'ok   ' : 'FALHA'} ${o}`); if (!ok) falhas.push(o); };
const nav = await chromium.launch(process.env.UV_CHROMIUM ? { executablePath: process.env.UV_CHROMIUM } : {});
const linhas = [];

for (const p of PLEITOS) {
  const ctx = await nav.newContext({ viewport: { width: 360, height: 760 }, deviceScaleFactor: 3, isMobile: true, hasTouch: true,
    timezoneId: 'America/Sao_Paulo', locale: 'pt-BR' });
  await ctx.addInitScript(() => {
    window.__longas = [];
    try { new PerformanceObserver((l) => { for (const e of l.getEntries()) window.__longas.push([e.startTime, e.duration]); }).observe({ type: 'longtask', buffered: true }); } catch { /* sem suporte */ }
  });
  if (process.env.UV_DIST) await ctx.route('**/static/**', (r) => { const f = join(DIST, new URL(r.request().url()).pathname.slice(8));
    return existsSync(f) ? r.fulfill({ status: 200, contentType: TIPOS[extname(f)] || 'application/octet-stream', body: readFileSync(f) }) : r.continue(); });
  const pg = await ctx.newPage();
  const cdp = await ctx.newCDPSession(pg);
  await cdp.send('Emulation.setCPUThrottlingRate', { rate: 4 });
  await cdp.send('Performance.enable');
  let bytes = 0;
  pg.on('response', async (r) => { if (r.url().includes('/consulta/')) { const l = Number(r.headers()['content-length'] || 0); bytes += l; } });
  const t0 = Date.now();
  await pg.goto(`${URL0}/#/p/${p}`, { waitUntil: 'domcontentloaded' });
  const abriu = await pg.waitForFunction(() => window.__uv?.consulta && [...(window.__uv.res || [])].some(([, r]) => r?.secoes), null, { timeout: 30000 })
    .then(() => true).catch(() => false);
  const tAbre = (Date.now() - t0) / 1000;
  conferir(abriu && tAbre <= 10, `${p}: abre em ${tAbre.toFixed(1)} s (≤ 10 s, CPU 4×, 360 px)`);
  if (!abriu) { await ctx.close(); continue; }
  // toca do começo a 300× por 20 s
  const ti = await pg.evaluate(() => performance.now());
  await pg.evaluate(() => { const pl = window.__uv.player; pl.ir(pl.info?.inicio ?? 0); pl.velocidade(300); });
  await pg.waitForTimeout(20000);
  const tf = await pg.evaluate(() => performance.now());
  await pg.evaluate(() => window.__uv.player.velocidade(0));
  const longas = (await pg.evaluate(() => window.__longas)).filter(([s]) => s >= ti && s <= tf);
  const ocup = longas.reduce((a, [, d]) => a + d, 0) / (tf - ti);
  const maior = longas.reduce((a, [, d]) => Math.max(a, d), 0);
  const m = await cdp.send('Performance.getMetrics');
  const heap = (m.metrics.find((x) => x.name === 'JSHeapUsedSize')?.value || 0) / 1048576;
  const cls = await pg.evaluate(() => window.__uv.cls ?? 0);
  conferir(ocup <= 0.35, `${p}: tocando a 300×, tarefas longas ${(ocup * 100).toFixed(0)} % do tempo (≤ 35 %)`);
  conferir(maior <= 1000, `${p}: maior tarefa longa ${maior.toFixed(0)} ms (≤ 1000)`);
  if (maior > 1000) console.log('      as maiores (início em s depois do play, duração em ms):', longas.sort((x, y) => y[1] - x[1]).slice(0, 5).map(([s0, d]) => `${((s0 - ti) / 1000).toFixed(1)}s:${d.toFixed(0)}`).join(' '));
  conferir(heap <= 200, `${p}: memória JS ${heap.toFixed(0)} MB (≤ 200)`);
  conferir(cls < 0.01, `${p}: CLS ${cls.toFixed(4)} (< 0,01)`);
  linhas.push(`| ${p} | ${tAbre.toFixed(1)} s | ${(ocup * 100).toFixed(0)} % | ${maior.toFixed(0)} ms | ${heap.toFixed(0)} MB | ${(bytes / 1048576).toFixed(2)} MB | ${cls.toFixed(4)} |`);
  await ctx.close();
}
await nav.close();
console.log('\n| pleito | abre | tarefas longas tocando | maior | memória JS | baixado (consulta, comprimido) | CLS |\n|---|---|---|---|---|---|---|');
for (const l of linhas) console.log(l);
console.log(falhas.length ? `\n${falhas.length} falha(s)` : '\ntodos os cenários passaram');
process.exit(falhas.length ? 1 : 0);
