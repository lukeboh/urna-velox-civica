// Chip da próxima eleição ao vivo na faixa (Luciano, 24/10/2026: "quero que o texto da próxima eleição fique rodando
// igual um letreiro se necessário, mas é que quero avisar que a partir da 01h da madruga já podem chegar BUs do
// exterior"). Com Playwright/Chromium e **nenhum servidor** (rotas e WebSocket falsos).
//   cd site && node testes/promo_letreiro.mjs [--saida /tmp/uv-promo-letreiro]
// Confere, de 320 a 1920 px:
//   * o aviso "BUs do exterior a partir da 01h" no chip (antes da 01h) e "BUs do exterior já podem chegar" (da 01h até
//     o primeiro BU), também no letreiro e no aviso que o chip abre;
//   * o chip dentro da faixa, sem cobrir o BUs/min nem passar da margem;
//   * texto que não cabe: rola como um letreiro (anda sozinho, sem cortar o selo AO VIVO); texto que cabe: parado;
//   * com "reduzir movimento" no sistema, nada anda.
import { chromium } from 'playwright';
import { mkdirSync, readFileSync, existsSync } from 'node:fs';
import { dirname, join, extname } from 'node:path';
import { fileURLToPath } from 'node:url';

const arg = (k, d) => { const i = process.argv.indexOf(`--${k}`); return i > 0 ? process.argv[i + 1] : d; };
const saida = arg('saida', '/tmp/uv-promo-letreiro');
mkdirSync(saida, { recursive: true });
const DIST = join(dirname(fileURLToPath(import.meta.url)), '..', 'dist');
const ORIGEM = 'http://uv.teste';
const TIPOS = { '.js': 'text/javascript', '.css': 'text/css', '.svg': 'image/svg+xml', '.woff2': 'font/woff2' };
const INDEX = readFileSync(join(DIST, 'index.html'), 'utf8').replaceAll('{{versao}}', 'teste').replace(/<meta name="uv-dados" content="[^"]*">/, '<meta name="uv-dados" content="">');
const falhas = [];
const conferir = (ok, o) => { console.log(`${ok ? 'ok   ' : 'FALHA'} ${o}`); if (!ok) falhas.push(o); };

const PROXIMA = { eleicao: '2026-10-25', nome: '2º turno das Eleições Gerais 2026', inicio: '2026-10-25T17:00:00-03:00', fim: '2026-10-26T08:00:00-03:00' };
const playlist = (agora) => ({ agora: `${agora}-03:00`, suspensao: null, proxima_suspensao: null, proxima_ao_vivo: { ...PROXIMA, ao_vivo: false },
  no_ar: null, fila: [], opcoes: [], meu_voto: null, total_votos: 0, tocados: [] });
const site = (iso) => { const [d, h] = iso.split('T'); const [a, m, dd] = d.split('-'); return `${dd}/${m}/${a} ${h.slice(0, 8)}`; };
const estado = (agora) => ({ tipo: 'estado', versao: 'teste', estado: 'acompanhando', encerrada: false, pleito: 3221, relogio_site: site(agora),
  relogio_sistema: { agora: `${agora}.000-03:00`, noite0: null, sistema0: null, acelerar: 1.0 },
  secoes: { catalogo: 517179, publicadas_cs: 0, aplicadas: 0, nao_instaladas: 0, pendentes: 0 }, ultimo_bu: { seq: 0, site: null, sistema: null },
  pausa_429: { ativa: false, retomada_em: null, pausas: 0 }, conferencias: 0, alertas: {}, clientes: 1 });

const nav = await chromium.launch();
async function abrir(largura, agora, { reduzir = false } = {}) {
  const movel = largura < 640;
  const ctx = await nav.newContext({ ...(movel ? { viewport: { width: largura, height: 780 }, deviceScaleFactor: 2, isMobile: true, hasTouch: true }
    : { viewport: { width: largura, height: 900 } }), timezoneId: 'America/Sao_Paulo', locale: 'pt-BR', reducedMotion: reduzir ? 'reduce' : 'no-preference' });
  await ctx.clock.setFixedTime(new Date(`${agora}-03:00`));
  await ctx.addInitScript(() => { try { localStorage.setItem('uv-aviso-dia', '2099-01-01'); } catch { /* sem armazenamento */ } });
  await ctx.route(`${ORIGEM}/**`, (r) => {
    const p = new URL(r.request().url()).pathname;
    if (p === '/' || p === '/index.html') return r.fulfill({ status: 200, contentType: 'text/html; charset=utf-8', body: INDEX });
    if (p.startsWith('/static/')) { const f = join(DIST, p.slice(8)); if (existsSync(f)) return r.fulfill({ status: 200, contentType: TIPOS[extname(f)] || 'application/octet-stream', body: readFileSync(f) }); }
    if (p === '/api/v1/nomes') return r.fulfill({ status: 200, contentType: 'application/json', body: 'null' });
    if (p === '/api/v1/topicos') return r.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
    if (p === '/api/v1/playlist') return r.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(playlist(agora)) });
    return r.fulfill({ status: 404, contentType: 'application/json', body: '{"erro":"nao_encontrado"}' });
  });
  await ctx.routeWebSocket(`${ORIGEM.replace('http', 'ws')}/ws`, (ws) => { ws.send(JSON.stringify(estado(agora))); ws.onMessage(() => {}); });
  const p = await ctx.newPage();
  await p.goto(`${ORIGEM}/`);
  await p.waitForTimeout(2500);
  return { ctx, p, tag: `${movel ? 'celular' : 'desktop'}-${largura}${reduzir ? ' (reduzir movimento)' : ''}` };
}
const desloc = (p) => p.$eval('.faixa .chip.promo', (c) => {
  const el = c.querySelector('.promo-trilho');
  return el ? new DOMMatrixReadOnly(getComputedStyle(el).transform).m41 : null;
});

for (const [agora, quando, re] of [['2026-10-24T21:50:00', 'antes da 01h', /BUs do exterior a partir da 01h/], ['2026-10-25T01:20:00', 'depois da 01h, sem BU ainda', /BUs do exterior já podem chegar/]]) {
  for (const largura of [1920, 1366, 1024, 390, 360, 320]) {
    const { ctx, p, tag } = await abrir(largura, agora);
    const t = `${tag} · ${quando}`;
    const chip = p.locator('.faixa .chip.promo');
    conferir(await chip.isVisible(), `${t}: chip da próxima eleição à vista`);
    const txt = ((await chip.textContent()) || '').replace(/\s+/g, ' ');
    conferir(re.test(txt), `${t}: o chip avisa dos BUs do exterior ("${txt.slice(0, 140)}")`);
    const c = await chip.boundingBox(), faixa = await p.locator('#faixa').boundingBox();
    const ritmo = await p.locator('.faixa .chip.ritmo').boundingBox();
    conferir(c.x >= faixa.x && c.x + c.width <= largura - 10, `${t}: o chip dentro da tela (${Math.round(c.x)}–${Math.round(c.x + c.width)} de ${largura})`);
    conferir(!ritmo || c.x + c.width <= ritmo.x, `${t}: não cobre o BUs/min`);
    // o selo AO VIVO fica parado e inteiro, à esquerda
    const selo = await p.locator('.faixa .chip.promo .vivo').boundingBox();
    conferir(selo && selo.x >= c.x && selo.x + selo.width <= c.x + c.width, `${t}: o selo AO VIVO inteiro dentro do chip`);
    const cabe = await p.$eval('.faixa .chip.promo', (el) => { const v = el.querySelector('.promo-janela'); const a = el.querySelector('.promo-a'); return v && a ? a.scrollWidth <= v.clientWidth + 1 : null; });
    const d0 = await desloc(p); await p.waitForTimeout(1200); const d1 = await desloc(p);
    if (cabe === null) conferir(false, `${t}: chip com janela e trilho do letreiro (.promo-janela / .promo-a)`);
    else if (cabe) conferir(d0 === d1 && Math.abs(d0) < 0.5, `${t}: o texto cabe e fica parado (${d0} → ${d1})`);
    else conferir(d0 !== d1, `${t}: o texto não cabe e roda como letreiro (${d0} → ${d1})`);
    conferir(await p.evaluate(() => document.documentElement.scrollWidth) <= largura, `${t}: sem rolagem horizontal`);
    // o letreiro do rodapé e o aviso que o chip abre também avisam
    const item = (await p.locator('#trilho .item.promo').allTextContents()).join(' ');
    conferir(re.test(item), `${t}: o item do letreiro avisa ("${item.replace(/\s+/g, ' ').slice(0, 120)}")`);
    await chip.click();
    await p.waitForTimeout(500);
    const folha = ((await p.locator('#folha').textContent()) || '').replace(/\s+/g, ' ');
    conferir(/exterior/i.test(folha) && /01h|1h/.test(folha), `${t}: o aviso aberto pelo chip explica os BUs do exterior`);
    await p.screenshot({ path: join(saida, `${tag}-${agora.slice(11, 13)}h.png`) });
    await ctx.close();
  }
}
// reduzir movimento: o texto não anda (fica cortado com reticência)
for (const largura of [390, 360]) {
  const { ctx, p, tag } = await abrir(largura, '2026-10-24T21:50:00', { reduzir: true });
  const d0 = await desloc(p); await p.waitForTimeout(1200); const d1 = await desloc(p);
  conferir(d0 === d1, `${tag}: nada anda (${d0} → ${d1})`);
  await ctx.close();
}
await nav.close();
console.log(falhas.length ? `\n${falhas.length} falha(s)` : '\ntudo ok');
process.exit(falhas.length ? 1 : 0);
