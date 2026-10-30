// depuração: abre uma consulta no ar e mostra o que a página tem (node testes/sonda_consulta.mjs <pleito>)
import { chromium } from 'playwright';
const URL0 = process.env.UV_URL || 'https://urnavelox.com.br';
const nav = await chromium.launch();
const pg = await nav.newPage({ viewport: { width: 360, height: 760 } });
pg.on('pageerror', (e) => console.log('pageerror:', e.message));
pg.on('response', (r) => { if (r.url().includes('/consulta/') && r.status() !== 200) console.log('resp', r.status(), r.url()); });
await pg.goto(`${URL0}/#/p/${process.argv[2] || '3226'}`);
await pg.waitForTimeout(8000);
console.log(await pg.evaluate(() => JSON.stringify({ hash: location.hash, consulta: !!window.__uv?.consulta, tela: window.__uv?.tela,
  res: [...(window.__uv?.res || [])].map(([t, r]) => `${t}:${r?.secoes?.apuradas}/${r?.secoes?.total}`).slice(0, 10),
  mapa: window.__uv?.mapa ? { total: window.__uv.mapa.total ?? window.__uv.con?.s?.mapa?.total, vista: window.__uv.mapa.vista } : null,
  dom: document.querySelectorAll('*').length, feed: document.querySelectorAll('#feed .item, .feed .item').length,
  canvas: [...document.querySelectorAll('canvas')].map((c) => `${c.className || c.id}:${c.width}x${c.height}`),
  animacoes: document.getAnimations().length,
  amostra: (() => { const r = [...(window.__uv?.res || [])][0]?.[1]; return r ? JSON.stringify(r).slice(0, 300) : null; })() })));
await nav.close();
