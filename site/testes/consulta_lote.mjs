// B-081: a consulta de um pleito com muitos marcos (o 452, 1º turno de 2024, tem ~12 mil) não pode travar a página
// tocando. Com Playwright/Chromium e **nenhum servidor**: a série do suplementar de RR (testes/fixtures/consulta/3237)
// com 24 mil "primeira seção" a mais espalhadas pela noite, em seções que existem no mapa (o 452 tem ~12 mil numa
// noite 3× mais longa): cada uma acende a onda grande do mapa e entra no letreiro.
//   cd site && node testes/consulta_lote.mjs
// Critérios (CPU 4× mais lenta, como o T14): tocando a 300× por 20 s, a maior tarefa longa ≤ 500 ms e as tarefas longas ≤ 35 % do tempo; o letreiro
// fica com no máximo 70 itens e o histórico tem todos os marcos entregues até o instante.
import { chromium } from 'playwright';
import { readFileSync, existsSync } from 'node:fs';
import { dirname, join, extname } from 'node:path';
import { fileURLToPath } from 'node:url';

const AQUI = dirname(fileURLToPath(import.meta.url));
const DIST = join(AQUI, '..', 'dist');
const FIX = join(AQUI, 'fixtures', 'consulta');
const ORIGEM = 'http://uv.teste';
const falhas = [];
const conferir = (ok, o) => { console.log(`${ok ? 'ok   ' : 'FALHA'} ${o}`); if (!ok) falhas.push(o); };
const TIPOS = { '.js': 'text/javascript', '.css': 'text/css', '.svg': 'image/svg+xml', '.woff2': 'font/woff2', '.json': 'application/json' };
const INDEX = readFileSync(join(DIST, 'index.html'), 'utf8').replaceAll('{{versao}}', 'teste');
const SERIE = join(FIX, '3237', 'serie');
const IND = JSON.parse(readFileSync(join(SERIE, 'indice.json'), 'utf8'));
const MARCOS = JSON.parse(readFileSync(join(SERIE, 'marcos.json'), 'utf8'));
const N = Number(process.env.UV_MARCOS || 24000);
const col = MARCOS.colunas;
/** os marcos a mais, nas seções `sec` ([município, zona, seção] que existem no mapa: cada uma ganha a onda grande) */
function gerar(sec) {
const extra = [];
for (let i = 0; i < N; i++) {
  const t = IND.inicio + Math.floor(((IND.fim - IND.inicio) * (i + 1)) / (N + 1));
  const r = Object.fromEntries(col.map((c) => [c, '']));
  // como as primeiras seções de cada município do 452 (as que mais pesam no letreiro)
  Object.assign(r, { seq: String(100000 + i), horario: new Date(t * 1000).toISOString().slice(0, 19).replace('T', ' '), classe: 'primeira_secao',
    abrangencia: 'rr', uf: 'rr', municipio: String(sec[i % sec.length][0]), zona: String(sec[i % sec.length][1]), secao: String(sec[i % sec.length][2]), t });
  extra.push(col.map((c) => r[c]));
}
return [...MARCOS.linhas, ...extra].sort((a, b) => (a.at(-1) ?? -Infinity) - (b.at(-1) ?? -Infinity));
}
let linhas = MARCOS.linhas;
let MARCOS_GRANDE = JSON.stringify(MARCOS);

const nav = await chromium.launch(process.env.UV_CHROMIUM ? { executablePath: process.env.UV_CHROMIUM } : {});
const ctx = await nav.newContext({ viewport: { width: 390, height: 780 }, isMobile: true, hasTouch: true, timezoneId: 'America/Sao_Paulo', locale: 'pt-BR' });
await ctx.addInitScript(() => {
  window.WebSocket = class { constructor() { } close() { } };
  window.__longas = [];
  new PerformanceObserver((l) => { for (const e of l.getEntries()) window.__longas.push([e.startTime, e.duration]); }).observe({ type: 'longtask', buffered: true });
});
await ctx.route('https://dados.urnavelox.com.br/**', (r) => r.abort());
await ctx.route(`${ORIGEM}/**`, async (r) => {
  const p = new URL(r.request().url()).pathname;
  if (p === '/' || p === '/index.html') return r.fulfill({ status: 200, contentType: 'text/html; charset=utf-8', body: INDEX });
  if (p.startsWith('/static/')) {
    const f = join(DIST, p.slice(8));
    if (existsSync(f)) return r.fulfill({ status: 200, contentType: TIPOS[extname(f)] || 'application/octet-stream', body: readFileSync(f) });
  }
  if (p === '/consulta/3237/serie/marcos.json') return r.fulfill({ status: 200, contentType: 'application/json', body: MARCOS_GRANDE });
  if (p === '/consulta/indice.json') return r.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ pleitos: [{ pleito: 3237 }] }) });
  if (p.startsWith('/consulta/')) {
    const f = join(FIX, p.slice(10));
    if (existsSync(f)) return r.fulfill({ status: 200, contentType: 'application/json', body: readFileSync(f) });
  }
  return r.fulfill({ status: 404, contentType: 'application/json', body: '{"erro":"nao_encontrado"}' });
});
const pg = await ctx.newPage();
// um celular modesto: CPU 4× mais lenta
await (await ctx.newCDPSession(pg)).send('Emulation.setCPUThrottlingRate', { rate: Number(process.env.UV_CPU || 4) });
pg.on('pageerror', (e) => conferir(false, `erro na página: ${e.message}`));
// 1ª abertura (os marcos da série): as seções do mapa, para os marcos a mais caírem em seções que existem
await pg.goto(`${ORIGEM}/#/p/3237`);
await pg.waitForFunction(() => window.__uv?.mapa?.carregado && window.__uv.mapa.N > 0, null, { timeout: 20000 }).catch(() => {});
const sec = await pg.evaluate(() => { const m = window.__uv.mapa; const r = [];
  for (let i = 0; i < m.N; i++) if (!m.ext[i]) r.push([m.munCod[m.mu[i]], m.zona[i], m.secao[i]]); return r; });
conferir(sec.length > 1000, `seções do mapa para os marcos (${sec.length})`);
linhas = gerar(sec);
MARCOS_GRANDE = JSON.stringify({ colunas: col, linhas });
await pg.goto('about:blank');
await pg.goto(`${ORIGEM}/#/p/3237`);
const abriu = await pg.waitForFunction(() => window.__uv?.consulta && window.__uv.res?.get('rr-c0003')?.secoes, null, { timeout: 20000 }).then(() => true).catch(() => false);
conferir(abriu, 'abre a consulta com os marcos a mais');
const t0 = await pg.evaluate(() => performance.now());
await pg.evaluate(() => { const pl = window.__uv.player; pl.ir(pl.info.inicio); pl.velocidade(300); });
await pg.waitForTimeout(20000);
const t1 = await pg.evaluate(() => performance.now());
const tInst = await pg.evaluate(() => { const pl = window.__uv.player; pl.velocidade(0); return pl.agora(); });
const longas = (await pg.evaluate(() => window.__longas)).filter(([s]) => s >= t0 && s <= t1);
const maior = longas.reduce((a, [, d]) => Math.max(a, d), 0);
const ocup = longas.reduce((a, [, d]) => a + d, 0) / (t1 - t0);
conferir(maior <= 500, `tocando a 300×: maior tarefa longa ${maior.toFixed(0)} ms (≤ 500)`);
console.log('      as maiores (s depois do play: ms):', [...longas].sort((x, y) => y[1] - x[1]).slice(0, 5).map(([a, d]) => `${((a - t0) / 1000).toFixed(1)}:${d.toFixed(0)}`).join(' '));
conferir(ocup <= 0.35, `tocando a 300×: tarefas longas ${(ocup * 100).toFixed(0)} % do tempo (≤ 35 %)`);
await pg.waitForTimeout(300);
const nFeed = await pg.evaluate(() => document.querySelectorAll('#feed .item, .feed .item').length);
// MAX = 70 vivos; os que estão saindo ficam até passar pela borda esquerda
conferir(nFeed > 0 && nFeed <= 90, `letreiro com ${nFeed} itens (≤ 90)`);
const esperado = linhas.filter((l) => l[2] !== 'faltam_para_mat' && (l.at(-1) ?? -Infinity) <= tInst).length;
const recebidos = await pg.evaluate(() => window.__uv.marcos.size);
conferir(recebidos >= esperado * 0.99, `a página recebeu os marcos até o instante (${recebidos} de ~${esperado}; o histórico tem todos)`);
await nav.close();
console.log(falhas.length ? `\n${falhas.length} falha(s)` : '\ntodos os cenários passaram');
process.exit(falhas.length ? 1 : 0);
