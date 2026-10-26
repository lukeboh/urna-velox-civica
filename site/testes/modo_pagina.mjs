// Selo do modo (REPLAY / aguardando / AO VIVO), contagem para a próxima eleição ao vivo, acesso à playlist e aviso
// do dia — B-022, B-023 e M-004 (docs/BUGS.md). Com Playwright/Chromium e **nenhum servidor**: a página de `dist/`
// é servida por rotas do próprio Playwright, o WebSocket é falso (`routeWebSocket`) e a playlist é um JSON fixo.
//   cd site && node testes/modo_pagina.mjs [--saida /tmp/uv-modo-fotos]
// Cenários (a 1366, 390 e 360 px, relógio fixo em America/Sao_Paulo):
//   replay  — serviço com rótulo de replay (ciclo diário): selo "REPLAY" em âmbar; contagem à vista na faixa e no
//             letreiro; o aviso abre sozinho na 1ª visita do dia, com a próxima eleição ao vivo e o link da playlist.
//   espera  — vitrine do 3221 (sem rótulo, nenhum BU): o selo não diz "ao vivo" e leva à playlist (B-022).
//   ao vivo — domingo depois das 17h, apuração andando: selo "AO VIVO", sem contagem nem aviso.
//   dia local — 1ª visita às 10h e volta às 22h30 do mesmo dia: o aviso não abre de novo (B-023).
import { chromium } from 'playwright';
import { mkdirSync, readFileSync, existsSync } from 'node:fs';
import { dirname, join, extname } from 'node:path';
import { fileURLToPath } from 'node:url';

const arg = (k, d) => { const i = process.argv.indexOf(`--${k}`); return i > 0 ? process.argv[i + 1] : d; };
const saida = arg('saida', '/tmp/uv-modo-fotos');
mkdirSync(saida, { recursive: true });
const DIST = join(dirname(fileURLToPath(import.meta.url)), '..', 'dist');
const ORIGEM = 'http://uv.teste';
const falhas = [];
const conferir = (ok, o) => { console.log(`${ok ? 'ok   ' : 'FALHA'} ${o}`); if (!ok) falhas.push(o); };
// M-012 (24/10/2026): o selo pode estar escondido (atributo hidden no #conexao) até o fim da totalização do 2º turno;
// aí as conferências de "selo à vista" e de "tocar no selo" são puladas (o texto e o data-modo continuam conferidos)
const SELO_ESCONDIDO = /<button[^>]*id="conexao"[^>]*\shidden[\s>]/.test(readFileSync(join(DIST, 'index.html'), 'utf8'));
if (SELO_ESCONDIDO) console.log('aviso: selo escondido (M-012): conferências de selo à vista e do toque no selo puladas');

const TIPOS = { '.js': 'text/javascript', '.css': 'text/css', '.svg': 'image/svg+xml', '.woff2': 'font/woff2', '.html': 'text/html', '.txt': 'text/plain', '.xml': 'application/xml' };
const INDEX = readFileSync(join(DIST, 'index.html'), 'utf8').replaceAll('{{versao}}', 'teste');

const PROXIMA = { eleicao: '2026-10-25', nome: '2º turno das Eleições Gerais 2026', inicio: '2026-10-25T17:00:00-03:00', fim: '2026-10-26T08:00:00-03:00' };
const ROT_REPLAY = '1º turno de 2024 (eleição de 06/10/2024) · em tempo real, no ritmo em que os BUs chegaram ao TSE';

function playlist(agora, modoNoAr) {
  const aoVivo = agora >= PROXIMA.inicio && agora < PROXIMA.fim;
  const susp = { eleicao: '2026-10-25', nome: PROXIMA.nome, de: '2026-10-24T00:00:00-03:00', ate: '2026-10-29T00:00:00-03:00' };
  const dentro = agora >= susp.de && agora < susp.ate;
  return {
    agora, suspensao: dentro ? { ...susp, retoma_em: susp.ate } : null, proxima_suspensao: dentro ? null : susp,
    proxima_ao_vivo: { ...PROXIMA, ao_vivo: aoVivo },
    no_ar: modoNoAr ? { item: modoNoAr === 'espera' ? '3221-espera' : '452-cadencia', modo: modoNoAr, pleito: modoNoAr === 'espera' ? '3221' : '452' } : null,
    fila: [], opcoes: [], meu_voto: null, total_votos: 0, tocados: [],
  };
}

function estado({ rotulo, aplicadas, agora }) {
  return {
    tipo: 'estado', versao: 'teste', estado: 'acompanhando', encerrada: false, pleito: rotulo ? 452 : 3221,
    relogio_site: rotulo ? '06/10/2024 18:30:00' : '01/01/1970 00:00:00',
    relogio_sistema: { agora, noite0: null, sistema0: null, acelerar: 1.0 },
    secoes: { catalogo: 500000, publicadas_cs: aplicadas, aplicadas, nao_instaladas: 0, pendentes: 0 },
    ultimo_bu: { seq: aplicadas, site: null, sistema: null }, pausa_429: { ativa: false, retomada_em: null, pausas: 0 },
    conferencias: 0, alertas: {}, clientes: 1, ...(rotulo ? { simulacao: { rotulo } } : {}),
  };
}

const nav = await chromium.launch();

/** Um contexto com o relógio fixo em `agora` (ISO com fuso) e as rotas falsas do cenário. */
async function abrir(largura, { agora, rotulo, aplicadas, noAr, plFalhas = 0 }, storage) {
  let falhasPl = plFalhas;   // B-024: as primeiras buscas da playlist respondem 503 (o Caddy do B, antes da 0.8.40)
  const movel = largura < 640;
  const ctx = await nav.newContext({
    ...(movel ? { viewport: { width: largura, height: 780 }, deviceScaleFactor: 2, isMobile: true, hasTouch: true, colorScheme: 'dark' }
      : { viewport: { width: largura, height: 900 }, colorScheme: 'light' }),
    timezoneId: 'America/Sao_Paulo', locale: 'pt-BR', ...(storage ? { storageState: storage } : {}),
  });
  await ctx.clock.setFixedTime(new Date(agora));
  await ctx.route(`${ORIGEM}/**`, async (r) => {
    const u = new URL(r.request().url());
    const p = u.pathname;
    if (p === '/' || p === '/index.html') return r.fulfill({ status: 200, contentType: 'text/html; charset=utf-8', body: INDEX });
    if (p.startsWith('/static/')) {
      const f = join(DIST, p.slice(8));
      if (existsSync(f)) return r.fulfill({ status: 200, contentType: TIPOS[extname(f)] || 'application/octet-stream', body: readFileSync(f) });
    }
    if (p === '/api/v1/nomes') return r.fulfill({ status: 200, contentType: 'application/json', body: 'null' });
    if (p === '/api/v1/topicos') return r.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
    if (p === '/api/v1/playlist' && falhasPl > 0) { falhasPl--; return r.fulfill({ status: 503, contentType: 'application/json', body: '{"erro":"servico_indisponivel","tentar_em_s":10}' }); }
    if (p === '/api/v1/playlist') return r.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(playlist(agora, noAr)) });
    return r.fulfill({ status: 404, contentType: 'application/json', body: '{"erro":"nao_encontrado"}' });
  });
  await ctx.routeWebSocket(`${ORIGEM.replace('http', 'ws')}/ws`, (ws) => {
    ws.onMessage(() => { /* inscrições: nada a responder */ });
    ws.send(JSON.stringify(estado({ rotulo, aplicadas, agora })));
  });
  const p = await ctx.newPage();
  await p.goto(`${ORIGEM}/`);
  return { ctx, p, movel, tag: movel ? `celular-${largura}` : 'desktop' };
}

const visivel = (p, sel, timeout = 4000) => p.waitForSelector(sel, { state: 'visible', timeout }).then(() => true).catch(() => false);
const texto = async (p, sel) => ((await p.locator(sel).first().textContent().catch(() => '')) || '').replace(/\s+/g, ' ').trim();
const folhaAberta = (p) => p.isVisible('#folha.aberta');

/** O chip inteiro à vista, dentro da margem lateral de 16 px e sem texto cortado. */
async function inteiro(p, sel, tag, nome) {
  const larg = p.viewportSize().width;
  const c = await p.locator(sel).first().boundingBox().catch(() => null);
  const cortado = c ? await p.locator(sel).first().evaluate((e) => e.scrollWidth > e.clientWidth + 1) : false;
  conferir(!!c && c.width > 0 && c.x >= 16 - 0.5 && c.x + c.width <= larg - 16 + 0.5 && !cortado,
    `${tag}: ${nome} inteiro à vista (${c ? `${Math.round(c.x)}–${Math.round(c.x + c.width)} de ${larg} px` : 'oculto'}${cortado ? ', texto cortado' : ''})`);
}

for (const largura of [1366, 390, 360]) {
  // ---------------------------------------------------------------- replay (terça, 15h)
  {
    const { ctx, p, tag } = await abrir(largura, { agora: '2026-10-20T15:00:00-03:00', rotulo: ROT_REPLAY, aplicadas: 1200, noAr: 'cadencia' });
    await p.waitForTimeout(1500);
    const selo = await texto(p, '#conexao');
    conferir(/REPLAY/.test(selo) && !/ao vivo/i.test(selo), `${tag} · replay: selo "${selo}"`);
    conferir((await p.getAttribute('#conexao', 'data-modo')) === 'replay', `${tag} · replay: selo com data-modo="replay"`);
    conferir(!(await p.locator('#conexao .txt').evaluate((e) => e.scrollWidth > e.clientWidth + 1).catch(() => true)),
      `${tag} · replay: o nome do modo inteiro no selo, sem reticência`);
    const cor = await p.locator('#conexao').evaluate((e) => getComputedStyle(e).backgroundColor).catch(() => '');
    const rgb = (cor.match(/\d+(\.\d+)?/g) || []).map(Number);
    conferir(rgb.length >= 3 && rgb[0] > 150 && rgb[1] > 90 && rgb[2] < 90, `${tag} · replay: selo em âmbar (${cor})`);
    conferir(await visivel(p, '#folha.aberta'), `${tag} · replay: o aviso abre sozinho na 1ª visita do dia`);
    await p.waitForTimeout(600);
    const folha = await texto(p, '#folha');
    conferir(/REPLAY/.test(folha), `${tag} · replay: o aviso explica o replay`);
    conferir(/AO VIVO/.test(folha) && /25\/10/.test(folha) && /17h/.test(folha), `${tag} · replay: o aviso anuncia a próxima eleição ao vivo (25/10, 17h)`);
    // M-012: com o selo escondido o aviso também não leva à playlist
    if (SELO_ESCONDIDO) conferir((await p.locator('#folha a[href="/playlist"]').count()) === 0, `${tag} · replay: com o selo escondido, o aviso não leva à playlist (M-012)`);
    else conferir(await p.isVisible('#folha a[href="/playlist"]'), `${tag} · replay: o aviso leva à playlist`);
    await p.screenshot({ path: `${saida}/replay-aviso-${tag}.png` });
    if (await folhaAberta(p)) await p.click('#folha-fechar');
    conferir(await visivel(p, '.faixa .chip.promo'), `${tag} · replay: contagem para a próxima eleição ao vivo na faixa`);
    const promo = await texto(p, '.faixa .chip.promo');
    conferir(/AO VIVO/.test(promo) && /25\/10/.test(promo) && /17h/.test(promo) && /\d+d \d{2}h/.test(promo), `${tag} · replay: contagem "${promo}"`);
    await inteiro(p, '.faixa .chip.promo', tag, 'contagem');
    await inteiro(p, '.faixa .chip.ritmo', tag, 'BUs/min');
    if (!SELO_ESCONDIDO) await inteiro(p, '#conexao', tag, 'selo');
    conferir((await p.locator('#trilho .item.promo').count()) === 1, `${tag} · replay: a próxima eleição ao vivo também no letreiro`);
    await p.screenshot({ path: `${saida}/replay-${tag}.png` });
    // o selo é o mesmo botão da playlist: abre o aviso (com o link)
    if (!SELO_ESCONDIDO) {
      await p.click('#conexao').catch(() => null);
      conferir(await visivel(p, '#folha.aberta a[href="/playlist"]', 2000), `${tag} · replay: o selo abre o aviso com a playlist`);
      if (await folhaAberta(p)) await p.click('#folha-fechar');
    }
    // a contagem também abre o aviso
    await p.click('.faixa .chip.promo').catch(() => null);
    conferir(await visivel(p, '#folha.aberta', 2000), `${tag} · replay: a contagem abre o aviso`);
    if (await folhaAberta(p)) await p.click('#folha-fechar');
    const est = await ctx.storageState();
    await ctx.close();
    // segunda visita no mesmo dia (17h): o aviso não abre sozinho
    const v2 = await abrir(largura, { agora: '2026-10-20T17:00:00-03:00', rotulo: ROT_REPLAY, aplicadas: 1300, noAr: 'cadencia' }, est);
    await v2.p.waitForTimeout(1500);
    conferir(!(await folhaAberta(v2.p)), `${tag} · replay: na 2ª visita do dia o aviso não abre sozinho`);
    await v2.ctx.close();
  }
  // ---------------------------------------------------------------- espera (vitrine do 3221) — B-022
  {
    const { ctx, p, tag } = await abrir(largura, { agora: '2026-10-20T14:40:00-03:00', rotulo: null, aplicadas: 0, noAr: 'espera' });
    await p.waitForTimeout(1500);
    const selo = await texto(p, '#conexao');
    conferir(!/ao vivo/i.test(selo), `${tag} · espera: o selo não diz "ao vivo" sem apuração ("${selo}")`);
    const txtCortado = await p.locator('#conexao .txt').evaluate((e) => e.scrollWidth > e.clientWidth + 1).catch(() => true);
    conferir(!txtCortado, `${tag} · espera: o nome do modo inteiro no selo, sem reticência`);
    conferir((await p.getAttribute('#conexao', 'data-modo')) === 'espera', `${tag} · espera: selo com data-modo="espera"`);
    if (await folhaAberta(p)) {
      conferir(/AO VIVO/.test(await texto(p, '#folha')), `${tag} · espera: o aviso do dia anuncia a próxima eleição ao vivo`);
      await p.click('#folha-fechar');
    }
    conferir(await visivel(p, '.faixa .chip.promo'), `${tag} · espera: contagem para a próxima eleição ao vivo na faixa`);
    if (!SELO_ESCONDIDO) {
      await inteiro(p, '#conexao', tag, 'selo');
      await p.click('#conexao').catch(() => null);
      conferir(await visivel(p, '#folha.aberta a[href="/playlist"]', 2000), `${tag} · espera: o selo leva à playlist (B-022)`);
    }
    await p.screenshot({ path: `${saida}/espera-${tag}.png` });
    await ctx.close();
  }
  // ---------------------------------------------------------------- ao vivo (domingo, 18h)
  {
    const { ctx, p, tag } = await abrir(largura, { agora: '2026-10-25T18:00:00-03:00', rotulo: null, aplicadas: 52000, noAr: null });
    await p.waitForTimeout(1500);
    const selo = await texto(p, '#conexao');
    conferir(/AO VIVO/.test(selo), `${tag} · ao vivo: selo "${selo}"`);
    conferir((await p.getAttribute('#conexao', 'data-modo')) === 'ao_vivo', `${tag} · ao vivo: selo com data-modo="ao_vivo"`);
    conferir(!(await p.isVisible('.faixa .chip.promo')), `${tag} · ao vivo: sem contagem (já é a eleição ao vivo)`);
    conferir(!(await folhaAberta(p)), `${tag} · ao vivo: nenhum aviso abre sozinho`);
    await p.screenshot({ path: `${saida}/ao-vivo-${tag}.png` });
    await ctx.close();
  }
}

// ---------------------------------------------------------------- B-024: playlist fora do ar na partida (503)
{
  const { ctx, p } = await abrir(390, { agora: '2026-10-20T16:49:00-03:00', rotulo: ROT_REPLAY, aplicadas: 9000, noAr: 'cadencia', plFalhas: 2 });
  conferir(await visivel(p, '.faixa .chip.promo', 75000), 'B-024: com a playlist fora do ar na partida, a contagem aparece assim que ela volta (≤ 75 s)');
  await ctx.close();
}

// ---------------------------------------------------------------- dia local (B-023): 10h e 22h30 do mesmo dia
{
  const v1 = await abrir(1366, { agora: '2026-10-21T10:00:00-03:00', rotulo: ROT_REPLAY, aplicadas: 10, noAr: 'cadencia' });
  conferir(await visivel(v1.p, '#folha.aberta'), 'dia local: o aviso abre na 1ª visita (21/10, 10h)');
  const est = await v1.ctx.storageState();
  await v1.ctx.close();
  const v2 = await abrir(1366, { agora: '2026-10-21T22:30:00-03:00', rotulo: ROT_REPLAY, aplicadas: 20, noAr: 'cadencia' }, est);
  await v2.p.waitForTimeout(1500);
  conferir(!(await folhaAberta(v2.p)), 'dia local: às 22h30 do mesmo dia o aviso não abre de novo (B-023)');
  await v2.ctx.close();
  const v3 = await abrir(1366, { agora: '2026-10-22T08:00:00-03:00', rotulo: ROT_REPLAY, aplicadas: 30, noAr: 'cadencia' }, est);
  conferir(await visivel(v3.p, '#folha.aberta'), 'dia local: no dia seguinte (22/10, 8h) o aviso abre de novo');
  await v3.ctx.close();
}

await nav.close();
console.log(falhas.length ? `modo_pagina: ${falhas.length} falha(s)` : 'modo_pagina: tudo certo');
process.exit(falhas.length ? 1 : 0);
