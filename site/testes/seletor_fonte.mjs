// Teste da melhoria do seletor de fonte dos números (docs/BUGS.md, M-015), com Playwright/Chromium e **nenhum
// servidor** (rotas e WebSocket falsos). Confere, do celular de 320 px ao computador de 1440 px:
//   - o controle diz o que é: rótulo "De onde vêm os números", botão ⓘ, nomes "Urna Velox · Comparar · TSE oficial" e uma
//     frase de uma linha embaixo, diferente em cada modo; sem nenhum texto cortado (reticência) em nenhum modo;
//   - o estado "TSE sem totais" (antes das 17h) explicado, em vez do "arquivo das 19:19 · há 10 h" que parecia defeito;
//   - o ⓘ abre a folha com a explicação e um botão por fonte (escolhe e fecha), e a URL leva `?comparar=`;
//   - a dica da primeira visita (destaque no controle e a frase "Toque em Comparar…"), uma vez por navegador;
//   - a medição de uso (POST sem corpo em /api/v1/uso/<evento>, só no log do Caddy);
//   - NADA PULA: a linha das fontes tem a mesma altura e o mesmo lugar em todos os modos e o CLS não sobe;
//   - sem rolagem horizontal.
//   cd site && node testes/seletor_fonte.mjs [--saida /tmp/uv-seletor]
import { chromium } from 'playwright';
import { mkdirSync, readFileSync, existsSync } from 'node:fs';
import { dirname, join, extname } from 'node:path';
import { fileURLToPath } from 'node:url';

const arg = (k, d) => { const i = process.argv.indexOf(`--${k}`); return i > 0 ? process.argv[i + 1] : d; };
const saida = arg('saida', join(process.env.TEMP || '/tmp', 'uv-seletor'));
mkdirSync(saida, { recursive: true });
const AQUI = dirname(fileURLToPath(import.meta.url));
const DIST = arg('dist', join(AQUI, '..', 'dist')); // --dist <pasta>: outra versão do site (ex.: a da main, para comparar o CLS)
const ORIGEM = 'http://uv.teste';
const TIPOS = { '.js': 'text/javascript', '.css': 'text/css', '.svg': 'image/svg+xml', '.woff2': 'font/woff2' };
const INDEX = readFileSync(join(DIST, 'index.html'), 'utf8').replaceAll('{{versao}}', 'teste').replace(/<meta name="uv-dados" content="[^"]*">/, '<meta name="uv-dados" content="">');
const NOMES = readFileSync(join(AQUI, 'fixtures', 'nomes_2t_presidente.json'), 'utf8');
const falhas = [];
const conferir = (ok, o) => { console.log(`${ok ? 'ok   ' : 'FALHA'} ${o}`); if (!ok) falhas.push(o); };

const AGORA = '2026-10-25T10:30:00';
const estado = { tipo: 'estado', versao: 'teste', estado: 'acompanhando', encerrada: false, pleito: 3221, relogio_site: '25/10/2026 10:30:00',
  relogio_sistema: { agora: `${AGORA}.000-03:00`, noite0: null, sistema0: null, acelerar: 1.0 },
  secoes: { catalogo: 517179, publicadas_cs: 0, aplicadas: 171, nao_instaladas: 0, pendentes: 0 }, ultimo_bu: { seq: 171, site: '25/10/2026 10:30:03', sistema: `${AGORA}.000-03:00` },
  pausa_429: { ativa: false, retomada_em: null, pausas: 0 }, conferencias: 0, alertas: {}, clientes: 1 };
const resultado = { v: 1, eleicao: 6258, turno: 2, cargo: 1, abr: 'br', circ: 'br', seq: 171, ultimo_bu: `${AGORA}-03:00`, totalizado_em: `${AGORA}.100-03:00`, encerrada: false,
  secoes: { total: 499248, apuradas: 171 }, eleitorado: 158745502, apurado: 119850, comparecimento: 51402, vb: 1306, vn: 1134, vnt: 0, validos: 48962, legenda: 0,
  estado: 'apurando', cand: [{ n: 22, v: 33665, pct: 68.76, dvt: 'Válido', st: '', mat: null }, { n: 13, v: 15297, pct: 31.24, dvt: 'Válido', st: '', mat: null }] };
// arquivo do TSE: sem totais (o de ontem às 19:19) ou com totais
const tse = (totalizadas) => ({ tipo: 'tse', instantaneo: true, topico: 'tse:br-c0001', ver: 1, estado: 'ok', motivo: null, arquivo: 'br-c0001-e006258-u.json',
  buscado: '25/10/2026 10:30:01', novo_em: '24/10/2026 19:19:13',
  dados: { geracao: totalizadas ? '25/10/2026 10:29:10' : '24/10/2026 19:19:13', totalizacao: '25/10/2026 10:29:08', andamento: totalizadas ? 'p' : 'n', md: 'n', finalizada: false,
    secoes: { total: 499248, totalizadas, nao_instaladas: 0 }, eleitorado: 158745502, comparecimento: totalizadas ? 50000 : 0, abstencao: 0, vb: 0, vn: 0, vnt: 0, validos: totalizadas ? 48000 : 0,
    cand: [{ n: 22, v: totalizadas ? 33000 : 0, pct: totalizadas ? 68.75 : 0, pos: 1, e: false, st: '', dvt: 'Válido' }, { n: 13, v: totalizadas ? 15000 : 0, pct: totalizadas ? 31.25 : 0, pos: 2, e: false, st: '', dvt: 'Válido' }] },
  t_envio: 1792917000000000 });

const nav = await chromium.launch();
async function abrir(largura, { tema = 'dark', totalizadas = 0, ctx: ctxPronto = null, hash = '#/br/c0001', query = '' } = {}) {
  const movel = largura < 640;
  const usos = [];
  const ctx = ctxPronto || await nav.newContext({ ...(movel ? { viewport: { width: largura, height: 780 }, deviceScaleFactor: 2, isMobile: true, hasTouch: true }
    : { viewport: { width: largura, height: 900 } }), timezoneId: 'America/Sao_Paulo', locale: 'pt-BR', colorScheme: tema });
  if (!ctxPronto) {
    await ctx.clock.setFixedTime(new Date(`${AGORA}-03:00`));
    await ctx.addInitScript(() => { try { localStorage.setItem('uv-aviso-dia', '2099-01-01'); } catch { /* sem armazenamento */ } });
    if (tema === 'light') await ctx.addInitScript(() => { try { localStorage.setItem('uv-tema', 'claro'); } catch { /* sem armazenamento */ } });
    await ctx.route(`${ORIGEM}/**`, (r) => {
      const u = new URL(r.request().url()), p = u.pathname;
      if (r.request().method() === 'POST' && p.startsWith('/api/v1/uso/')) { usos.push(p.slice(12)); return r.fulfill({ status: 404, contentType: 'application/json', body: '{"erro":"nao_encontrado"}' }); }
      if (p === '/' || p === '/index.html') return r.fulfill({ status: 200, contentType: 'text/html; charset=utf-8', body: INDEX });
      if (p.startsWith('/static/')) { const f = join(DIST, p.slice(8)); if (existsSync(f)) return r.fulfill({ status: 200, contentType: TIPOS[extname(f)] || 'application/octet-stream', body: readFileSync(f) }); }
      if (p === '/api/v1/nomes') return r.fulfill({ status: 200, contentType: 'application/json', body: NOMES });
      if (p === '/api/v1/topicos') return r.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify([{ topico: 'br-c0001', arquivo: 'br-c0001-e006258.json', seq: 171 }]) });
      if (p === '/api/v1/playlist') return r.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ agora: `${AGORA}-03:00`, suspensao: null, proxima_suspensao: null, proxima_ao_vivo: null, no_ar: null, fila: [], opcoes: [], meu_voto: null, total_votos: 0, tocados: [] }) });
      return r.fulfill({ status: 404, contentType: 'application/json', body: '{"erro":"nao_encontrado"}' });
    });
    await ctx.routeWebSocket(`${ORIGEM.replace('http', 'ws')}/ws`, (ws) => {
      ws.send(JSON.stringify(estado));
      ws.send(JSON.stringify({ tipo: 'resultado', topico: 'br-c0001', seq: 171, txt: JSON.stringify({ resultado }) }));
      ws.onMessage((msg) => { if (String(msg).includes('tse:br-c0001')) ws.send(JSON.stringify(tse(ctx.__totalizadas ?? 0))); });
    });
  }
  ctx.__totalizadas = totalizadas;
  ctx.__usos = ctx.__usos || usos;
  const p = await ctx.newPage();
  await p.addInitScript(() => { try { window.__clsT = 0; new PerformanceObserver((l) => { for (const e of l.getEntries()) if (!e.hadRecentInput) window.__clsT += e.value; }).observe({ type: 'layout-shift', buffered: true }); } catch { /* sem layout-shift */ } });
  await p.goto(`${ORIGEM}/${query}${hash}`);
  p.on('console', (m) => { if (m.type() === 'error') console.log('console.error:', m.text().slice(0, 200)); });
  p.on('pageerror', (e) => console.log('pageerror:', String(e).slice(0, 300)));
  try { await p.waitForSelector('.fontes .comparar', { timeout: 15000 }); } catch (e) {
    console.log('DIAG', JSON.stringify(await p.evaluate(() => { const S = window.__uv; return { tela: S?.tela?.topico ?? null, ui: S?.ui ? Object.keys(S.ui).slice(0, 3) : null, res: S ? [...(S.res.mapa?.keys?.() ?? [])].slice(0, 3) : null, msgs: S?.msgs, conexao: S?.conexao, hash: location.hash, texto: document.body.innerText.slice(0, 200) }; })));
    throw e;
  }
  await p.waitForTimeout(800);
  p.__cls0 = await p.evaluate(() => window.__clsT || 0); // CLS do carregamento (topo e rodapé; já existia na main)
  return { ctx, p, usos: ctx.__usos, movel };
}

const textos = (p) => p.evaluate(() => ({
  rot: document.querySelector('.f-rot')?.textContent, x: document.querySelector('.f-x')?.textContent, a: document.querySelector('.f-a')?.textContent,
  b: document.querySelector('.f-b')?.textContent, est: document.querySelector('.f-est')?.textContent,
  botoes: [...document.querySelectorAll('.comparar button')].map((b) => b.textContent),
  marcado: [...document.querySelectorAll('.comparar button')].find((b) => b.getAttribute('aria-checked') === 'true')?.dataset.modo,
}));
// nenhum texto da linha cortado por reticência: o conteúdo cabe na caixa
const cortados = (p) => p.evaluate(() => [...document.querySelectorAll('.f-rot, .f-x, .f-a, .f-b, .f-est.ligado, .comparar button')]
  .filter((e) => e.scrollWidth > e.clientWidth + 1).map((e) => `${e.className || e.dataset.modo}: "${e.textContent}" (${e.scrollWidth} > ${e.clientWidth})`));
const caixa = (p) => p.evaluate(() => {
  const r = (s) => { const b = document.querySelector(s)?.getBoundingClientRect(); return b ? [Math.round(b.x), Math.round(b.y + scrollY), Math.round(b.width), Math.round(b.height)] : null; };
  return { fontes: r('.fontes'), h1: r('.corrida h1'), cab: r('.f-cab'), ctl: r('.fontes .comparar'), txt: r('.f-txt') };
});
// depois do clique o ponteiro sai de cima do cartão: a inclinação sob o ponteiro (cardTatil, só transform) mexe nas caixas por frações de pixel
const clicar = async (p, modo) => { await p.click(`.comparar button[data-modo="${modo}"]`); await p.mouse.move(0, 0); await p.waitForTimeout(800); };

// ---------------------------------------------------------------- por largura
for (const largura of [320, 360, 390, 768, 1366, 1440]) {
  const tag = largura < 640 ? `celular ${largura}` : `tela ${largura}`;
  const alturaEsperada = largura >= 1200 ? 79 : 117;
  if (process.argv.includes('--so-cls')) { // só o CLS do carregamento (comparar duas versões)
    const a = await abrir(largura, { totalizadas: 0 });
    console.log(`${tag}: CLS do carregamento ${(await a.p.evaluate(() => window.__clsT)).toFixed(4)}`);
    await a.ctx.close();
    continue;
  }
  const { ctx, p, usos } = await abrir(largura, { totalizadas: 0 });
  // 1) primeira visita: rótulo, nomes, dica
  let t = await textos(p);
  conferir(t.rot === 'De onde vêm os números', `${tag}: rótulo "De onde vêm os números" ("${t.rot}")`);
  conferir(await p.locator('.f-info').isVisible(), `${tag}: botão ⓘ à vista`);
  conferir(JSON.stringify(t.botoes) === JSON.stringify(['Urna Velox', 'Comparar', 'TSE oficial']), `${tag}: botões ${JSON.stringify(t.botoes)}`);
  conferir(t.marcado === 'uv', `${tag}: começa em Urna Velox`);
  conferir(t.x === 'Toque em Comparar para conferir com o TSE', `${tag}: 1ª visita: a frase da dica ("${t.x}")`);
  conferir(await p.locator('.comparar.dica').count() === 1, `${tag}: 1ª visita: controle destacado (classe dica)`);
  conferir(usos.filter((u) => u === 'fonte-dica').length === 1, `${tag}: a dica foi medida uma vez (${JSON.stringify(usos)})`);
  const c0 = await caixa(p);
  conferir(c0.fontes[3] === alturaEsperada, `${tag}: linha das fontes com ${alturaEsperada} px de altura (${c0.fontes[3]})`);
  await p.screenshot({ path: join(saida, `${tag.replace(' ', '-')}-1-primeira-visita.png`) });
  // 2) a dica termina sozinha em 6 s e a frase do modo volta
  await p.waitForTimeout(6000);
  t = await textos(p);
  conferir(t.x === 'Boletins somados assim que o TSE os publica', `${tag}: depois da dica: frase do modo Urna Velox ("${t.x}")`);
  conferir(await p.locator('.comparar.dica').count() === 0, `${tag}: o destaque some depois de 6 s`);
  // 3) cada modo, com o TSE ainda sem totais
  for (const modo of ['ambos', 'tse', 'uv']) {
    await clicar(p, modo);
    t = await textos(p);
    const esperado = { uv: 'Boletins somados assim que o TSE os publica', ambos: 'Sem totais no TSE ainda; só o Urna Velox', tse: 'Ainda sem totais no TSE: veja o Urna Velox' }[modo];
    conferir(t.marcado === modo && t.x === esperado, `${tag}: modo ${modo}: marcado e frase "${t.x}"`);
    if (modo !== 'uv') conferir(t.b === 'O TSE ainda não começou a totalizar', `${tag}: modo ${modo}: "${t.b}" (em vez do arquivo velho)`);
    const cx = await caixa(p);
    const moveu = Object.keys(c0).filter((k) => JSON.stringify(cx[k]) !== JSON.stringify(c0[k])).map((k) => `${k}: ${JSON.stringify(c0[k])} → ${JSON.stringify(cx[k])}`);
    conferir(moveu.length === 0,
      `${tag}: modo ${modo}: nada se moveu (linha das fontes, título, rótulo, controle e texto nos mesmos lugares)${moveu.length ? ' — ' + moveu.join(' | ') : ''}`);
    const corte = await cortados(p);
    conferir(corte.length === 0, `${tag}: modo ${modo}: nenhum texto cortado${corte.length ? ' — ' + corte.join(' | ') : ''}`);
    await p.screenshot({ path: join(saida, `${tag.replace(' ', '-')}-2-${modo}-sem-totais.png`) });
  }
  conferir(JSON.stringify(usos.filter((u) => u !== 'fonte-dica')) === JSON.stringify(['fonte-ambos', 'fonte-tse', 'fonte-uv']), `${tag}: cliques medidos ${JSON.stringify(usos)}`);
  // 4) ⓘ abre a folha; "TSE oficial" na folha escolhe e fecha
  await p.click('.f-info');
  await p.waitForTimeout(500);
  const folha = await p.evaluate(() => ({ aberta: document.querySelector('#folha').classList.contains('aberta'), titulo: document.querySelector('#folha-titulo').textContent,
    dt: [...document.querySelectorAll('#folha-corpo dt')].map((x) => x.textContent), acoes: [...document.querySelectorAll('.folha-acoes button')].map((b) => b.textContent + (b.classList.contains('ativo') ? '*' : '')) }));
  conferir(folha.aberta && folha.titulo === 'De onde vêm os números', `${tag}: ⓘ abre a folha "${folha.titulo}"`);
  conferir(JSON.stringify(folha.dt) === JSON.stringify(['Urna Velox', 'TSE oficial', 'Comparar', 'Antes das 17h']), `${tag}: a folha explica as três fontes e as 17h (${folha.dt})`);
  conferir(JSON.stringify(folha.acoes) === JSON.stringify(['Urna Velox*', 'Comparar', 'TSE oficial']), `${tag}: a folha tem um botão por fonte, o atual marcado (${folha.acoes})`);
  await p.screenshot({ path: join(saida, `${tag.replace(' ', '-')}-3-folha.png`) });
  await p.click('.folha-acoes button:nth-child(3)');
  await p.waitForTimeout(700);
  t = await textos(p);
  conferir(t.marcado === 'tse' && !(await p.evaluate(() => document.querySelector('#folha').classList.contains('aberta'))), `${tag}: "TSE oficial" na folha escolhe a fonte e fecha`);
  conferir(new URL(p.url()).searchParams.get('comparar') === 'tse', `${tag}: a URL leva ?comparar=tse`);
  conferir(usos.includes('fonte-info') && usos.filter((u) => u === 'fonte-tse').length === 2, `${tag}: ⓘ e a escolha pela folha medidos (${JSON.stringify(usos)})`);
  // 5) 2ª visita (mesmo navegador): sem dica
  const p2 = (await abrir(largura, { ctx, query: '?comparar=uv' })).p;
  t = await textos(p2);
  conferir(t.x === 'Boletins somados assim que o TSE os publica' && await p2.locator('.comparar.dica').count() === 0, `${tag}: 2ª visita: sem dica ("${t.x}")`);
  // 6) com o TSE já totalizando: arquivo e porcentagem aparecem, sem corte
  ctx.__totalizadas = 1200;
  await clicar(p2, 'ambos');
  await p2.waitForTimeout(800);
  t = await textos(p2);
  conferir(/^arquivo do TSE das 10:29:10/.test(t.b || '') && t.x === 'Lado a lado: veja quem está à frente', `${tag}: com totais: "${t.x}" / "${t.b}"`);
  const corte2 = await cortados(p2);
  conferir(corte2.length === 0, `${tag}: com totais: nenhum texto cortado${corte2.length ? ' — ' + corte2.join(' | ') : ''}`);
  await p2.screenshot({ path: join(saida, `${tag.replace(' ', '-')}-4-comparar-com-totais.png`) });
  // 7) nada pulou e não há rolagem horizontal (CLS acumulado das duas páginas)
  for (const [nome, pg] of [['1ª', p], ['2ª', p2]]) {
    const cls = await pg.evaluate(() => window.__clsT || 0);
    const larg = await pg.evaluate(() => document.documentElement.scrollWidth);
    // o do carregamento (topo e rodapé nas telas largas) já existia na main; aqui vale o que o seletor causa depois: dica, modos, folha
    const depois = cls - (pg.__cls0 || 0);
    conferir(depois < 0.001, `${tag}: ${nome} página: nada pulou depois do carregamento (CLS +${depois.toFixed(4)}; no carregamento ${(pg.__cls0 || 0).toFixed(4)})`);
    if (depois >= 0.001) console.log('  deslocamentos:', JSON.stringify(await pg.evaluate(() => window.__uv.clsFontes)));
    conferir(larg <= largura, `${tag}: ${nome} página: sem rolagem horizontal (${larg} px)`);
  }
  await ctx.close();
}
// tema claro: o rótulo e o ⓘ continuam legíveis (contraste do rótulo contra o fundo do cartão)
for (const largura of [360, 1440]) {
  const { ctx, p } = await abrir(largura, { tema: 'light' });
  const c = await p.evaluate(() => {
    const lum = (rgb) => { const [r, g, b] = (rgb.match(/[\d.]+/g) || ['0', '0', '0']).slice(0, 3).map(Number).map((v) => { v /= 255; return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4; }); return 0.2126 * r + 0.7152 * g + 0.0722 * b; };
    const razao = (a, b) => { const [x, y] = [lum(a), lum(b)].sort((m, n) => n - m); return (x + 0.05) / (y + 0.05); };
    const fundo = getComputedStyle(document.body).backgroundColor;
    const itens = { rotulo: '.f-rot', frase: '.f-x', linhaA: '.f-a', info: '.f-info', botao: '.comparar button' };
    const o = { tema: document.documentElement.dataset.tema || document.documentElement.className, fundo };
    for (const [k, s] of Object.entries(itens)) { const el = document.querySelector(s); if (!el) { o[k] = ['(sem elemento)', 0]; continue; } const cor = getComputedStyle(el).color; o[k] = [cor, Number(razao(cor, fundo).toFixed(2))]; }
    return o;
  });
  console.log(`tema claro ${largura}: ${JSON.stringify(c)}`);
  conferir(c.rotulo[1] >= 4.5 && c.frase[1] >= 4.5 && c.linhaA[1] >= 4.5, `claro ${largura}: contraste do rótulo (${c.rotulo[1]}), da frase (${c.frase[1]}) e do % (${c.linhaA[1]})`);
  await p.screenshot({ path: join(saida, `claro-${largura}.png`) });
  await ctx.close();
}
await nav.close();
console.log(falhas.length ? `\n${falhas.length} falha(s)` : '\ntudo ok');
process.exit(falhas.length ? 1 : 0);
