// depuração de desempenho da consulta: rastro do Chromium (devtools.timeline) tocando a 300×, somado por evento, e o
// tempo de `receber` por tipo de mensagem.   [UV_CPU=4] node testes/traco_consulta.mjs <pleito> [segundos]
import { chromium } from 'playwright';
import { readFileSync, existsSync } from 'node:fs';
import { dirname, join, extname } from 'node:path';
import { fileURLToPath } from 'node:url';
const DIST = join(dirname(fileURLToPath(import.meta.url)), '..', 'dist');
const URL0 = process.env.UV_URL || 'https://urnavelox.com.br';
const [p = '452', seg = '6'] = process.argv.slice(2);
const nav = await chromium.launch();
const ctx = await nav.newContext({ viewport: { width: 360, height: 760 }, isMobile: true });
// UV_DIST=1: a página desta cópia (site/dist) com os dados do site no ar
if (process.env.UV_DIST) await ctx.route('**/static/**', (r) => { const f = join(DIST, new URL(r.request().url()).pathname.slice(8));
  return existsSync(f) ? r.fulfill({ status: 200, contentType: extname(f) === '.css' ? 'text/css' : 'text/javascript', body: readFileSync(f) }) : r.continue(); });
const pg = await ctx.newPage();
await pg.goto(`${URL0}/#/p/${p}`);
await pg.waitForFunction(() => window.__uv?.consulta && [...(window.__uv.res || [])].some(([, r]) => r?.secoes), null, { timeout: 30000 });
await pg.waitForTimeout(1500);
await pg.evaluate(() => {
  const c = window.__uv.con; const orig = c.aoReceber; window.__por = {};
  c.aoReceber = (m) => { const a = performance.now(); orig(m); const k = m.tipo + (m.classe ? ':' + m.classe : ''); const x = (window.__por[k] ||= [0, 0]); x[0]++; x[1] += performance.now() - a; };
  const pl = window.__uv.player; pl.ir(pl.info.inicio);
});
const cdp = await ctx.newCDPSession(pg);
if (process.env.UV_CPU) await cdp.send('Emulation.setCPUThrottlingRate', { rate: Number(process.env.UV_CPU) });
const evs = [];
cdp.on('Tracing.dataCollected', (d) => evs.push(...d.value));
const fim = new Promise((r) => cdp.once('Tracing.tracingComplete', r));
await cdp.send('Tracing.start', { categories: 'devtools.timeline,disabled-by-default-devtools.timeline', transferMode: 'ReportEvents' });
await pg.evaluate(() => window.__uv.player.velocidade(300));
await pg.waitForTimeout(Number(seg) * 1000);
await pg.evaluate(() => window.__uv.player.velocidade(0));
await cdp.send('Tracing.end');
await fim;
const soma = new Map();
for (const e of evs) if (e.ph === 'X' && e.dur) soma.set(e.name, (soma.get(e.name) || [0, 0, 0]).map((v, i) => v + [1, e.dur / 1000, 0][i]).map((v, i) => (i === 2 ? Math.max(soma.get(e.name)?.[2] || 0, e.dur / 1000) : v)));
console.log(`${p}: ${seg} s tocando a 300×`);
for (const [k, [n, ms, mx]] of [...soma].sort((a, b) => b[1][1] - a[1][1]).slice(0, 18)) console.log(`${ms.toFixed(0).padStart(7)} ms  ${String(n).padStart(6)}×  maior ${mx.toFixed(0).padStart(5)} ms  ${k}`);
// as chamadas de JS de cada tarefa (FunctionCall), por função de entrada
const fc = new Map();
for (const e of evs) if (e.ph === 'X' && e.name === 'FunctionCall' && e.dur) {
  const d = e.args?.data || {}; const k = `${d.functionName || '?'} ${String(d.url || '').split('/').pop()}:${d.lineNumber ?? ''}`;
  const x = fc.get(k) || [0, 0, 0]; x[0]++; x[1] += e.dur / 1000; x[2] = Math.max(x[2], e.dur / 1000); fc.set(k, x);
}
console.log('\nFunctionCall por entrada:');
for (const [k, [n, ms, mx]] of [...fc].sort((a, b) => b[1][1] - a[1][1]).slice(0, 12)) console.log(`${ms.toFixed(0).padStart(7)} ms  ${String(n).padStart(6)}×  maior ${mx.toFixed(0).padStart(5)} ms  ${k}`);
const por = await pg.evaluate(() => window.__por);
console.log('\nreceber por tipo:');
for (const [k, [n, ms]] of Object.entries(por).sort((a, b) => b[1][1] - a[1][1]).slice(0, 12)) console.log(`${ms.toFixed(0).padStart(7)} ms  ${String(n).padStart(6)}×  ${k}`);
await nav.close();
