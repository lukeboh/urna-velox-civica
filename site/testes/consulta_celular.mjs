// B-083, B-084 e B-085: a consulta no celular (375 px), no Chromium e no WebKit (o motor do Safari; precisa de
// `npx playwright install webkit` e das bibliotecas do sistema: `sudo npx playwright install-deps webkit`). **Nenhum
// servidor**: a série do suplementar de RR (testes/fixtures/consulta/3237), com a noite esticada para três dias no
// cenário das marcas.
//   cd site && node testes/consulta_celular.mjs
// Cenários:
//   B-085 — o número de "seções acesas" do mapa fica dentro do cartão e começa onde começa o rótulo, depois de voltar
//           no tempo, tocar e ir ao fim (no WebKit o número ficava deslocado para a esquerda, fora do cartão);
//   B-083 — numa noite de três dias, as marcas de hora acima do slider não encostam umas nas outras;
//   B-084 — o selo diz "CONSULTA" inteiro (sem a seta da playlist) e a faixa não mostra o ritmo (BUs/min);
//   B-086 — voltar ao início da noite apaga as luzes do mapa (ficava o mapa do fim, todo aceso, com "19 seções").
import * as pw from 'playwright';
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
const IND = JSON.parse(readFileSync(join(FIX, '3237', 'serie', 'indice.json'), 'utf8'));
const PLEITOS = readFileSync(join(AQUI, '..', '..', 'deploy', 'oci', 'testes', 'fixtures', 'pleitos-2026-10-29.json'), 'utf8');
const esperar = (pg, f, arg, ms = 15000) => pg.waitForFunction(f, arg, { timeout: ms }).then(() => true).catch(() => false);

async function abrir(nav, nome, indice) {
  const ctx = await nav.newContext({ viewport: { width: 375, height: 812 }, deviceScaleFactor: 3, hasTouch: true,
    ...(nome === 'firefox' ? {} : { isMobile: true }), colorScheme: 'dark', timezoneId: 'America/Sao_Paulo', locale: 'pt-BR' });
  await ctx.addInitScript(() => { window.WebSocket = class { constructor() { } close() { } }; });
  await ctx.route('https://dados.urnavelox.com.br/**', (r) => r.abort());
  await ctx.route(`${ORIGEM}/**`, async (r) => {
    const p = new URL(r.request().url()).pathname;
    if (p === '/' || p === '/index.html') return r.fulfill({ status: 200, contentType: 'text/html; charset=utf-8', body: INDEX });
    if (p.startsWith('/static/')) {
      const f = join(DIST, p.slice(8));
      if (existsSync(f)) return r.fulfill({ status: 200, contentType: TIPOS[extname(f)] || 'application/octet-stream', body: readFileSync(f) });
    }
    if (indice && p === '/consulta/3237/serie/indice.json') return r.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(indice) });
    if (p === '/consulta/indice.json') return r.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ pleitos: [{ pleito: 3237 }] }) });
    if (p.startsWith('/consulta/')) {
      const f = join(FIX, p.slice(10));
      if (existsSync(f)) return r.fulfill({ status: 200, contentType: 'application/json', body: readFileSync(f) });
    }
    if (p === '/api/v1/pleitos') return r.fulfill({ status: 200, contentType: 'application/json', body: PLEITOS });
    return r.fulfill({ status: 404, contentType: 'application/json', body: '{"erro":"nao_encontrado"}' });
  });
  const pg = await ctx.newPage();
  pg.on('pageerror', (e) => conferir(false, `${nome}: erro na página: ${e.message}`));
  await pg.goto(`${ORIGEM}/#/p/3237`);
  await esperar(pg, () => window.__uv?.consulta && window.__uv.res?.get('rr-c0003')?.secoes && window.__uv.mapa?.carregado);
  return { ctx, pg };
}

/** posição dos dígitos à vista do número de seções acesas × o cartão e o rótulo */
const numero = (pg) => pg.evaluate(() => {
  const n = document.querySelector('.mp-n'); const v = n.querySelector('.vis');
  const vis = [...v.children].filter((x) => !x.classList.contains('vazio'));
  const esq = Math.min(...vis.map((x) => x.getBoundingClientRect().left));
  const dir = Math.max(...vis.map((x) => x.getBoundingClientRect().right));
  const cartao = n.closest('section').getBoundingClientRect(), rot = document.querySelector('.mp-rot').getBoundingClientRect();
  return { esq, dir, cartao: [cartao.left, cartao.right], rot: rot.left, texto: n.querySelector('.sr')?.textContent };
});

/** pixels acesos na camada das luzes do mapa (alfa > 0) */
const luzes = (pg) => pg.evaluate(() => {
  const c = document.querySelector('canvas.mp-acc'); if (!c?.width) return -1;
  const d = c.getContext('2d').getImageData(0, 0, c.width, c.height).data; let n = 0;
  for (let i = 3; i < d.length; i += 4) if (d[i] > 8) n++;
  return n;
});

const motores = [['chromium', pw.chromium], ['webkit', pw.webkit]];
for (const [nome, motor] of motores) {
  let nav;
  try { nav = await motor.launch(); } catch (e) { conferir(nome !== 'webkit', `${nome}: não abriu (${String(e.message).split('\n')[0]})`); continue; }
  // B-085
  {
    const { ctx, pg } = await abrir(nav, nome);
    const passos = [
      ['fim', () => {}],
      ['início', () => { const pl = window.__uv.player; pl.ir(pl.info.inicio + 20 * 60); }],
      ['tocou', () => window.__uv.player.velocidade(300)],
      ['fim de novo', () => { const pl = window.__uv.player; pl.velocidade(0); pl.ir(pl.info.fim); }],
    ];
    let luzFim = 0;
    for (const [rot, f] of passos) {
      await pg.evaluate(f);
      await pg.waitForTimeout(rot === 'tocou' ? 3000 : 1500);
      // B-086: as luzes do mapa acompanham o instante (no início, bem menos que no fim)
      const lz = await luzes(pg);
      if (rot === 'fim') luzFim = lz;
      if (rot === 'início') conferir(lz >= 0 && lz < luzFim * 0.3, `${nome} B-086 início: ${lz} pixels acesos no mapa (no fim, ${luzFim})`);
      const m = await numero(pg);
      conferir(m.esq >= m.cartao[0] + 4 && m.dir <= m.cartao[1] - 4 && Math.abs(m.esq - m.rot) <= 2,
        `${nome} B-085 ${rot}: número ${m.texto} em x=${m.esq.toFixed(0)}..${m.dir.toFixed(0)}, rótulo em ${m.rot.toFixed(0)}, cartão ${m.cartao.map((x) => x.toFixed(0)).join('..')}`);
    }
    // B-084
    const selo = await pg.evaluate(() => { const t = document.querySelector('#conexao .txt'); const pl = document.querySelector('#conexao .pl');
      return { txt: t.textContent, cabe: t.scrollWidth <= t.clientWidth + 1, seta: pl && getComputedStyle(pl).display !== 'none' }; });
    conferir(selo.txt === 'CONSULTA' && selo.cabe && !selo.seta, `${nome} B-084: selo "${selo.txt}" ${selo.cabe ? 'inteiro' : 'cortado'}${selo.seta ? ', com a seta da playlist' : ''}`);
    const ritmo = await pg.evaluate(() => { const r = document.querySelector('#faixa .chip.ritmo'); return r ? getComputedStyle(r).display !== 'none' && r.getBoundingClientRect().width > 0 : false; });
    conferir(!ritmo, `${nome} B-084: sem o chip do ritmo (BUs/min) na consulta`);
    await ctx.close();
  }
  // B-083: noite de três dias
  {
    const { ctx, pg } = await abrir(nav, nome, { ...IND, fim: IND.inicio + 3 * 86400 });
    await pg.waitForTimeout(800);
    const m = await pg.evaluate(() => [...document.querySelectorAll('.player .pl-marcas span')].map((s) => { const r = s.getBoundingClientRect(); return [s.textContent, r.left, r.right]; }));
    let enc = 0;
    for (let i = 1; i < m.length; i++) if (m[i][1] < m[i - 1][2] + 2) enc++;
    conferir(m.length >= 3 && enc === 0, `${nome} B-083: ${m.length} marcas, ${enc} encostadas (${m.map((x) => x[0]).join(' ')})`);
    await ctx.close();
  }
  await nav.close();
}
console.log(falhas.length ? `\n${falhas.length} falha(s)` : '\ntodos os cenários passaram');
process.exit(falhas.length ? 1 : 0);
