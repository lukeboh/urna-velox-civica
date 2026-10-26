// Teste do B-014 (celular trava e esquenta): abre a página como um celular (390 px, toque, CPU 4× mais lenta pelo
// CDP) numa disputa e mede, durante a apuração, o que chega pelo WebSocket (mensagens e bytes por minuto, por tipo e
// os tópicos de resultado distintos) e o tempo da thread principal em tarefas longas (> 50 ms).
//   node testes/carga_pagina.mjs --url http://127.0.0.1:18080/ --hash '#/sp71072/c0011' --saida /tmp/x [--limite 900]
//        [--teto-kb-min 1500] [--teto-topicos 3] [--teto-ocupado 0.25] [--mapa]   (--mapa: com o cartão do mapa à vista)
// Sai quando a apuração da tela encerra (ou no limite, em segundos). Falha (código 1) se a página recebe resultado de
// mais tópicos que o teto (a tela e a circunscrição dela), se algum minuto passa do teto de KB, ou se a thread
// principal fica ocupada em tarefas longas mais que a fração do teto. Grava <saida>/carga.json.
import { chromium } from 'playwright';
import { mkdirSync, writeFileSync } from 'node:fs';

const arg = (k, d) => { const i = process.argv.indexOf(`--${k}`); return i > 0 ? process.argv[i + 1] : d; };
const url = arg('url', 'http://127.0.0.1:18080/');
const hash = arg('hash', '#/sp71072/c0011');
const saida = arg('saida', '/tmp/uv-carga-pagina');
const limite = +arg('limite', '900');
const tetoKb = +arg('teto-kb-min', '1500');
const tetoTop = +arg('teto-topicos', '3');
const tetoOcup = +arg('teto-ocupado', '0.25');
mkdirSync(saida, { recursive: true });

const nav = await chromium.launch();
const ctx = await nav.newContext({ viewport: { width: 390, height: 844 }, deviceScaleFactor: 3, isMobile: true, hasTouch: true });
const pag = await ctx.newPage();
const cdp = await ctx.newCDPSession(pag);
await cdp.send('Network.enable');
await cdp.send('Emulation.setCPUThrottlingRate', { rate: 4 });
const t0 = Date.now();
const porMin = new Map();   // minuto → { n, bytes }
const porTipo = {};
const topicos = new Set();
let n = 0, bytes = 0;
cdp.on('Network.webSocketFrameReceived', (e) => {
  const p = e.response?.payloadData || '';
  const b = Buffer.byteLength(p);
  n++; bytes += b;
  const m = Math.floor((Date.now() - t0) / 60000);
  const x = porMin.get(m) || { n: 0, bytes: 0 };
  x.n++; x.bytes += b; porMin.set(m, x);
  const tipo = /"tipo":"([a-z_]+)"/.exec(p)?.[1] || '?';
  porTipo[tipo] = porTipo[tipo] || { n: 0, bytes: 0 };
  porTipo[tipo].n++; porTipo[tipo].bytes += b;
  if (tipo === 'atualizacao' || tipo === 'instantaneo') { const t = /"topico":"([^"]+)"/.exec(p)?.[1]; if (t) topicos.add(t); }
});
await pag.addInitScript(() => {
  window.__ocupado = 0;
  try { new PerformanceObserver((l) => { for (const e of l.getEntries()) window.__ocupado += e.duration; }).observe({ type: 'longtask', buffered: true }); } catch { /* sem longtask */ }
});
await pag.goto(url + hash);
// --mapa (M-017): com o cartão do mapa das seções à vista (ele só pede as mensagens `mapa` e desenha quando aparece)
if (process.argv.includes('--mapa')) {
  await pag.waitForFunction(() => window.__uv?.mapa?.el?.isConnected, null, { timeout: 60_000 });
  await pag.evaluate(() => window.__uv.mapa.el.scrollIntoView({ block: 'center' }));
  await pag.waitForFunction(() => window.__uv.mapa.resumo().carregado, null, { timeout: 120_000 }).catch(() => {});
}
let encerrada = false;
while ((Date.now() - t0) / 1000 < limite) {
  await pag.waitForTimeout(2000);
  encerrada = await pag.evaluate(() => { const u = window.__uv; const r = u?.tela && u.res.get(u.tela.topico); return !!r?.encerrada; }).catch(() => false);
  if (encerrada) break;
}
const dur = (Date.now() - t0) / 1000;
const ocupado = await pag.evaluate(() => window.__ocupado).catch(() => 0);
// capturas do fim (celular e tela larga), para conferir a página
await pag.screenshot({ path: `${saida}/fim-390.png`, fullPage: true }).catch(() => {});
await pag.setViewportSize({ width: 1366, height: 900 }).catch(() => {});
await pag.waitForTimeout(800);
await pag.screenshot({ path: `${saida}/fim-1366.png`, fullPage: true }).catch(() => {});
await nav.close();
const minutos = [...porMin.entries()].sort((a, b) => a[0] - b[0]).map(([m, x]) => ({ minuto: m, msgs: x.n, kb: Math.round(x.bytes / 1024) }));
const maxKb = Math.max(0, ...minutos.map((x) => x.kb));
const r = { url: url + hash, segundos: Math.round(dur), encerrada, msgs: n, kb: Math.round(bytes / 1024), kb_min_max: maxKb, topicos_resultado: topicos.size,
  ocupado_s: +(ocupado / 1000).toFixed(1), ocupado_frac: +(ocupado / 1000 / dur).toFixed(3), por_tipo: porTipo, por_minuto: minutos, topicos: [...topicos].slice(0, 20) };
writeFileSync(`${saida}/carga.json`, JSON.stringify(r, null, 1));
const erros = [];
if (!encerrada) erros.push(`a apuração da tela não encerrou em ${limite} s`);
if (topicos.size > tetoTop) erros.push(`recebeu resultado de ${topicos.size} tópicos (teto ${tetoTop}: a tela e a circunscrição dela)`);
if (maxKb > tetoKb) erros.push(`${maxKb} KB num minuto (teto ${tetoKb})`);
if (r.ocupado_frac > tetoOcup) erros.push(`thread principal ${(100 * r.ocupado_frac).toFixed(0)} % do tempo em tarefas longas (teto ${(100 * tetoOcup).toFixed(0)} %)`);
console.log(`carga_pagina: ${r.segundos} s, ${n} mensagens, ${r.kb} KB (máx. ${maxKb} KB/min), ${topicos.size} tópicos de resultado, tarefas longas ${r.ocupado_s} s (${(100 * r.ocupado_frac).toFixed(0)} %)`);
console.log('por tipo:', JSON.stringify(Object.fromEntries(Object.entries(porTipo).map(([k, v]) => [k, `${v.n} msgs, ${Math.round(v.bytes / 1024)} KB`]))));
if (erros.length) { console.error('FALHOU\n  - ' + erros.join('\n  - ')); process.exit(1); }
console.log('ok');
