// Diagnóstico (M-017): abre a página, mostra o cartão do mapa e registra, por 120 s, o que vai e vem no WebSocket
// (op mapa, mensagens mapa, reinícios), o modo da conexão e o resumo do cartão a cada 5 s. Usado com o uv-servico
// sendo reiniciado no meio (troca de época).  node testes/diag_mapa_retomada.mjs --url http://127.0.0.1:18082/ [--duracao 120]
import { chromium } from 'playwright';
const arg = (k, d) => { const i = process.argv.indexOf(`--${k}`); return i > 0 ? process.argv[i + 1] : d; };
const url = arg('url', 'http://127.0.0.1:18082/'), DUR = +arg('duracao', 120);
const nav = await chromium.launch();
const pg = await (await nav.newContext({ viewport: { width: 1366, height: 900 } })).newPage();
const t0 = Date.now(), t = () => ((Date.now() - t0) / 1000).toFixed(1);
pg.on('console', (m) => console.log(`${t()} console.${m.type()}: ${m.text().slice(0, 200)}`));
pg.on('pageerror', (e) => console.log(`${t()} ERRO: ${e}`));
pg.on('websocket', (ws) => {
  console.log(`${t()} ws aberto ${ws.url()}`);
  ws.on('close', () => console.log(`${t()} ws fechado`));
  ws.on('framesent', (f) => { const s = String(f.payload); if (s.includes('"op":"mapa"')) console.log(`${t()} >> ${s}`); });
  ws.on('framereceived', (f) => {
    const s = String(f.payload);
    if (s.startsWith('{"tipo":"mapa"')) { const j = JSON.parse(s); console.log(`${t()} << mapa epoca=${j.epoca} de=${j.de} n=${j.n} ${j.reiniciar ? 'REINICIAR' : ''} pares=${(j.s || []).length / 2}`); }
    else if (s.includes('"tipo":"erro"')) console.log(`${t()} << ${s.slice(0, 200)}`);
  });
});
pg.on('request', (r) => { if (r.url().includes('/mapa/')) console.log(`${t()} GET ${r.url().replace(/^https?:\/\/[^/]+/, '')}`); });
await pg.goto(url);
await pg.waitForFunction(() => window.__uv?.mapa?.el?.isConnected);
await pg.evaluate(() => window.__uv.mapa.el.scrollIntoView({ block: 'center' }));
await pg.waitForFunction(() => window.__uv.mapa.resumo().carregado, null, { timeout: 60000 });
while ((Date.now() - t0) / 1000 < DUR) {
  await pg.waitForTimeout(5000);
  const r = await pg.evaluate(() => { const m = window.__uv.mapa.resumo(); return { n: m.n, epoca: window.__uv.mapa.epoca, modo: window.__uv.con?.modo ?? window.__uv.modo, ws: window.__uv.con?.ws?.readyState }; }).catch((e) => String(e));
  console.log(`${t()} resumo ${JSON.stringify(r)}`);
}
await nav.close();
