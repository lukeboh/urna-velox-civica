// depuração de desempenho da consulta: perfil de CPU (CDP) de uma ação, com as funções de maior tempo próprio.
//   node testes/perfil_consulta.mjs <pleito> [ir|tocar]      (UV_URL = outro endereço)
import { chromium } from 'playwright';
const URL0 = process.env.UV_URL || 'https://urnavelox.com.br';
const [p = '452', acao = 'ir'] = process.argv.slice(2);
const nav = await chromium.launch();
const ctx = await nav.newContext({ viewport: { width: 360, height: 760 }, isMobile: true });
const pg = await ctx.newPage();
await pg.goto(`${URL0}/#/p/${p}`);
await pg.waitForFunction(() => window.__uv?.consulta && [...(window.__uv.res || [])].some(([, r]) => r?.secoes), null, { timeout: 30000 });
await pg.waitForTimeout(1500);
const cdp = await ctx.newCDPSession(pg);
await cdp.send('Profiler.enable');
await cdp.send('Profiler.setSamplingInterval', { interval: 500 });
await cdp.send('Profiler.start');
const t0 = Date.now();
if (acao === 'ir') await pg.evaluate(() => { const pl = window.__uv.player; pl.ir(pl.info.inicio); });
else await pg.evaluate(() => { const pl = window.__uv.player; pl.velocidade(300); });
await pg.waitForTimeout(acao === 'ir' ? 500 : 8000);
await pg.evaluate(() => 0);   // espera a tarefa em curso acabar
const dt = Date.now() - t0;
const { profile } = await cdp.send('Profiler.stop');
const self = new Map();
const dts = profile.timeDeltas; const ids = profile.samples;
const porId = new Map(profile.nodes.map((n) => [n.id, n]));
for (let i = 0; i < ids.length; i++) {
  const n = porId.get(ids[i]); const f = n.callFrame;
  const k = `${f.functionName || '(anônima)'} ${f.url.split('/').pop()}:${f.lineNumber + 1}`;
  self.set(k, (self.get(k) || 0) + (dts[i] || 0) / 1000);
}
console.log(`${p} ${acao}: ${dt} ms de relógio`);
for (const [k, v] of [...self].sort((a, b) => b[1] - a[1]).slice(0, 25)) console.log(`${v.toFixed(0).padStart(7)} ms  ${k}`);
await nav.close();
