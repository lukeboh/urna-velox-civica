// B-089 e M-027 (30/10/2026, Luciano, celular): o letreiro de marcos do rodapé.
//  * B-089: tocar numa notícia abre o detalhe e pausa o letreiro ("pausado"); depois que o detalhe some — pelo ✕, pelo
//    Esc ou por um toque fora —, o letreiro tem de voltar a andar sozinho;
//  * M-027: o texto não é cortado seco junto ao botão de menu: o trilho esmaece nas bordas (mask-image com gradiente).
// Sem servidor: a consulta do 3237 (série de testes/fixtures/consulta/3237), que enche o letreiro com os marcos da noite;
// celular de 390 px com toque, no Chromium e no WebKit (UV_MOTOR=webkit).
//   cd site && node testes/letreiro_celular.mjs
import { chromium, webkit } from 'playwright';
import { readFileSync, existsSync } from 'node:fs';
import { dirname, join, extname } from 'node:path';
import { fileURLToPath } from 'node:url';

const AQUI = dirname(fileURLToPath(import.meta.url));
const DIST = join(AQUI, '..', 'dist');
const FIX = join(AQUI, 'fixtures', 'consulta');
const ORIGEM = 'http://uv.teste';
const TIPOS = { '.js': 'text/javascript', '.css': 'text/css', '.svg': 'image/svg+xml', '.woff2': 'font/woff2', '.json': 'application/json' };
const INDEX = readFileSync(join(DIST, 'index.html'), 'utf8').replaceAll('{{versao}}', 'teste')
  .replace(/<meta name="uv-dados" content="[^"]*">/, '<meta name="uv-dados" content="">');
const falhas = [];
const conferir = (ok, o) => { console.log(`${ok ? 'ok   ' : 'FALHA'} ${o}`); if (!ok) falhas.push(o); };

const motores = process.env.UV_MOTOR === 'webkit' ? [['webkit', webkit]] : process.env.UV_MOTOR === 'chromium' ? [['chromium', chromium]] : [['chromium', chromium], ['webkit', webkit]];
for (const [nome, motor] of motores) {
  const nav = await motor.launch();
  for (const fechar of ['✕', 'Esc', 'toque fora']) {
    const tag = `${nome} · fecha pelo ${fechar}`;
    const ctx = await nav.newContext({ viewport: { width: 390, height: 844 }, deviceScaleFactor: 2, hasTouch: true, ...(nome === 'chromium' ? { isMobile: true } : {}),
      timezoneId: 'America/Sao_Paulo', locale: 'pt-BR' });
    await ctx.addInitScript(() => { window.WebSocket = class { constructor() { } close() { } }; try { localStorage.setItem('uv-aviso-dia', '2099-01-01'); } catch { /* */ } });
    await ctx.route('https://dados.urnavelox.com.br/**', (r) => r.abort());
    await ctx.route(`${ORIGEM}/**`, async (r) => {
      const p = new URL(r.request().url()).pathname;
      if (p === '/' || p === '/index.html') return r.fulfill({ status: 200, contentType: 'text/html; charset=utf-8', body: INDEX });
      if (p.startsWith('/static/')) { const f = join(DIST, p.slice(8)); if (existsSync(f)) return r.fulfill({ status: 200, contentType: TIPOS[extname(f)] || 'application/octet-stream', body: readFileSync(f) }); }
      if (p === '/consulta/indice.json') return r.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ pleitos: [{ pleito: 3237 }] }) });
      if (p.startsWith('/consulta/')) { const f = join(FIX, p.slice(10)); if (existsSync(f)) return r.fulfill({ status: 200, contentType: 'application/json', body: readFileSync(f) }); }
      return r.fulfill({ status: 404, contentType: 'application/json', body: '{"erro":"nao_encontrado"}' });
    });
    const p = await ctx.newPage();
    p.on('pageerror', (e) => conferir(false, `${tag}: erro na página: ${e.message}`));
    await p.goto(`${ORIGEM}/#/p/3237`);
    await p.waitForSelector('#feed .item', { timeout: 20000 });
    await p.waitForTimeout(1500);
    if (await p.isVisible('#folha.aberta')) { await p.click('#folha-fechar'); await p.waitForTimeout(400); }
    // M-027: esmaecimento nas bordas do trilho
    if (fechar === '✕') {
      const mask = await p.$eval('#trilho', (t) => { const c = getComputedStyle(t); return c.maskImage || c.webkitMaskImage || ''; });
      conferir(/gradient/.test(mask), `${nome} · M-027: o trilho esmaece nas bordas (mask-image: ${mask.slice(0, 80) || 'nenhuma'})`);
    }
    // anda?
    const posicao = () => p.evaluate(() => { const i = [...document.querySelectorAll('#feed .item')]; return i.map((e) => new DOMMatrixReadOnly(getComputedStyle(e).transform).m41).join(','); });
    const andando = async () => { const a = await posicao(); await p.waitForTimeout(1200); return a !== await posicao(); };
    conferir(await andando(), `${tag}: o letreiro anda antes do toque`);
    // tocar numa notícia à vista
    const alvo = await p.evaluate(() => {
      const t = document.querySelector('#trilho').getBoundingClientRect();
      // a parte à vista de uma notícia (os itens fixos são mais largos que o trilho do celular), longe das bordas
      const i = [...document.querySelectorAll('#feed .item')].map((e) => e.getBoundingClientRect())
        .map((b) => ({ l: Math.max(b.left, t.left + 40), r: Math.min(b.right, t.right - 40), y: b.top + b.height / 2 })).find((b) => b.r - b.l > 60);
      return i ? { x: (i.l + i.r) / 2, y: i.y } : null;
    });
    if (!alvo) { conferir(false, `${tag}: nenhuma notícia inteira à vista para tocar`); await ctx.close(); continue; }
    // o que está sob o dedo tem de ser a notícia (não uma camada invisível por cima do trilho)
    const sob = await p.evaluate(([x, y]) => { const e = document.elementFromPoint(x, y); return e?.closest('.item') ? 'item' : `${e?.tagName}.${e?.className}`; }, [alvo.x, alvo.y]);
    conferir(sob === 'item', `${tag}: sob o dedo está a notícia (${sob})`);
    await p.touchscreen.tap(alvo.x, alvo.y);
    // no iPhone o botão tocado guarda o foco (e um toque fora, num lugar sem foco, não o tira): o mesmo aqui
    await p.evaluate(([x]) => { for (const e of document.querySelectorAll('#feed .item')) { const b = e.getBoundingClientRect(); if (b.left <= x && b.right >= x) { e.focus({ preventScroll: true }); break; } } }, [alvo.x]);
    await p.waitForTimeout(500);
    conferir(await p.isVisible('#folha.aberta'), `${tag}: tocar na notícia abre o detalhe`);
    conferir(await p.$eval('#feed', (f) => f.dataset.pausado) === '1', `${tag}: com o detalhe aberto, o letreiro está pausado`);
    if (fechar === '✕') { const b = await p.$('#folha-fechar'); const r = await b.boundingBox(); await p.touchscreen.tap(r.x + r.width / 2, r.y + r.height / 2); }
    else if (fechar === 'Esc') await p.keyboard.press('Escape');
    else { const r = await p.locator('.corrida h1').boundingBox(); await p.touchscreen.tap(r.x + 20, r.y + r.height / 2); }
    await p.waitForTimeout(500);
    conferir(!(await p.isVisible('#folha.aberta')), `${tag}: o detalhe some`);
    // a pausa do toque dura no máximo 6 s
    await p.waitForTimeout(6800);
    const pausado = await p.$eval('#feed', (f) => f.dataset.pausado);
    conferir(pausado === '0', `${tag}: o letreiro sai do "pausado" depois que o detalhe some (data-pausado=${pausado})`);
    conferir(await andando(), `${tag}: o letreiro volta a andar`);
    await ctx.close();
  }
  await nav.close();
}
console.log(falhas.length ? `\n${falhas.length} falha(s)` : '\ntodos os cenários passaram');
process.exit(falhas.length ? 1 : 0);
