// Mapa das seções e cartões que se maximizam e se reorganizam (sem servidor real: a página de `dist/` servida por
// rotas do Playwright, REST e WebSocket falsos com o protocolo §4.8). Confere, medindo:
//   * o mapa carrega (/static/mapa.json, /api/v1/mapa/secoes, /api/v1/mapa/estado) e conta as seções acesas da
//     fotografia; as mensagens `mapa` do WebSocket acendem as novas (de/n contíguos), a de outra época relê a fotografia;
//   * o exterior: seções da UF zz contadas à parte, cidades sem posição listadas em `semPosicao`;
//   * a vista Globo e a cor "Horário" ficam guardadas (uv-mapa); o palco não muda de tamanho ao trocar;
//   * maximizar: o cartão cobre a tela, o painel das UFs aparece, Esc volta ao lugar com a mesma caixa;
//   * reorganizar: setas na alça mudam a ordem e ela fica guardada (uv-cartoes) depois de recarregar; "voltar" ao padrão;
//   * nada pula (CLS 0) e nenhum erro no console; capturas em 1366 e 390 px nos dois temas e do cartão maximizado.
//   cd site && node testes/mapa_pagina.mjs [--saida /tmp/uv-mapa]
import { chromium } from 'playwright';
import { mkdirSync, readFileSync, existsSync } from 'node:fs';
import { dirname, join, extname } from 'node:path';
import { fileURLToPath } from 'node:url';

const arg = (k, d) => { const i = process.argv.indexOf(`--${k}`); return i > 0 ? process.argv[i + 1] : d; };
const saida = arg('saida', '/tmp/uv-mapa');
mkdirSync(saida, { recursive: true });
const DIST = join(dirname(fileURLToPath(import.meta.url)), '..', 'dist');
const ORIGEM = 'http://uv.teste';
const INDEX = readFileSync(join(DIST, 'index.html'), 'utf8').replaceAll('{{versao}}', 'teste')
  .replace(/<meta name="uv-dados" content="[^"]*">/, '<meta name="uv-dados" content="">');
const GEO = JSON.parse(readFileSync(join(DIST, 'mapa.json'), 'utf8'));
const falhas = [];
let oks = 0;
const conferir = (nome, ok, o = '') => { if (ok) { oks++; return; } console.log(`FALHA ${nome} ${o}`); falhas.push(`${nome} ${o}`); };

// ---------------------------------------------------------------- dados falsos: 6 seções por município + exterior
const UFS_TSE = ['ac', 'sp', 'zz'];
const MUNIS = [];
for (let i = 0; i < GEO.mun.length; i += 3 * 7) MUNIS.push(GEO.mun[i]);   // ~800 municípios do Brasil
const EXT = [[29629, 'LISBOA'], [29700, 'TOQUIO'], [29801, 'NOVA YORK'], [29902, 'CIDADE INVENTADA']];
const chaves = [];
for (const c of MUNIS) for (let s = 1; s <= 6; s++) chaves.push([c, 'sp', 1 + (s > 3), s]);
for (const [c] of EXT) for (let s = 1; s <= 5; s++) chaves.push([c, 'zz', 1, 3000 + s]);
chaves.sort((a, b) => a[0] - b[0] || a[2] - b[2] || a[3] - b[3]);
function lista() {
  const ufs = ['sp', 'zz'], m = [];
  let i = 0;
  while (i < chaves.length) {
    const c = chaves[i][0];
    let j = i;
    while (j < chaves.length && chaves[j][0] === c) j++;
    const zonas = [];
    for (const k of chaves.slice(i, j)) { const z = zonas.at(-1); if (z && z[0] === k[2]) z[1].push(k[3]); else zonas.push([k[2], [k[3]]]); }
    m.push(c, ufs.indexOf(chaves[i][1]), zonas.length);
    for (const [z, ss] of zonas) { m.push(z, ss.length); ss.forEach((s, q) => m.push(q ? s - ss[q - 1] : s)); }
    i = j;
  }
  return { v: 1, n: chaves.length, ufs, m };
}
const N = chaves.length;
const ACESAS0 = Math.floor(N * 0.4);
let EPOCA = 111;
// B-064: seções que se revelaram agregadas na noite (fora do total; `ret` da fotografia)
let RET = [];
// fotografia: as primeiras 40 % (exterior inclusive, que vem primeiro na ordem da chave) acesas, faixa 171 (17h)
function estadoMapa() {
  const h = new Uint8Array(N);
  for (let i = 0; i < ACESAS0; i++) h[i] = 171;
  return { v: 1, epoca: EPOCA, total: N - RET.length, n: ACESAS0, fora: 0, ret: RET, h: Buffer.from(h).toString('base64') };
}
const NOMES = {
  pleito: '3220', ufs: { sp: 'SÃO PAULO', zz: 'EXTERIOR' },
  municipios: Object.fromEntries(EXT.map(([c, n]) => [`zz${c}`, n])),
  eleicoes: [{ codigo: 544, turno: 2, cargos: [{ cargo: 1, vagas: 1, consulta: false, abrangencias: ['br'], circunscricoes: [{ abr: 'br', eleitorado: 1000, candidatos: [
    { n: 10, nome_urna: 'CANDIDATA A', nome: 'CANDIDATA A', partido: 'PT', vices: [] }, { n: 20, nome_urna: 'CANDIDATO B', nome: 'CANDIDATO B', partido: 'PL', vices: [] }] }] }] }],
};
const RES = { v: 1, eleicao: 544, turno: 2, cargo: 1, abr: 'br', circ: 'br', seq: 400, ultimo_bu: '2026-10-25T18:05:08-03:00', totalizado_em: '2026-10-25T18:05:09.000-03:00',
  encerrada: false, secoes: { total: N, apuradas: ACESAS0 }, eleitorado: 1000, apurado: 400, comparecimento: 320, vb: 5, vn: 10, vnt: 0, validos: 305, legenda: 0, estado: 'apurando',
  cand: [{ n: 10, v: 160, pct: 52.46, dvt: 'Válido', st: '' }, { n: 20, v: 145, pct: 47.54, dvt: 'Válido', st: '' }] };
const ESTADO = { tipo: 'estado', versao: 'teste', estado: 'acompanhando', encerrada: false, pleito: 3220, relogio_site: '25/10/2026 18:05:08',
  secoes: { catalogo: N, publicadas_cs: 400, aplicadas: 400, nao_instaladas: 0, pendentes: 0 }, ultimo_bu: { seq: 400, site: null, sistema: null },
  pausa_429: { ativa: false, retomada_em: null, pausas: 0 }, conferencias: 0, alertas: {}, clientes: 1 };

// ---------------------------------------------------------------- página
const nav = await chromium.launch();
const erros = [];
async function abrir({ largura, tema, ctx: ctxAnt }) {
  const movel = largura < 640;
  const ctx = ctxAnt || await nav.newContext({ viewport: { width: largura, height: movel ? 844 : 1000 }, deviceScaleFactor: movel ? 2 : 1, isMobile: movel, hasTouch: movel,
    timezoneId: 'America/Sao_Paulo', locale: 'pt-BR' });
  if (!ctxAnt) {
    await ctx.addInitScript((t) => { try { if (!sessionStorage.getItem('uv-teste')) { localStorage.clear(); sessionStorage.setItem('uv-teste', '1'); }
      localStorage.setItem('uv-tema', t); localStorage.setItem('uv-aviso-dia', '2099-01-01'); } catch { /* */ } }, tema);
    await ctx.route(`${ORIGEM}/**`, async (r) => {
      const p = new URL(r.request().url()).pathname;
      const json = (o) => r.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(o) });
      if (p === '/' || p === '/index.html') return r.fulfill({ status: 200, contentType: 'text/html; charset=utf-8', body: INDEX });
      if (p.startsWith('/static/')) {
        const f = join(DIST, p.slice(8));
        const tipos = { '.js': 'text/javascript', '.css': 'text/css', '.svg': 'image/svg+xml', '.woff2': 'font/woff2', '.json': 'application/json' };
        if (existsSync(f)) return r.fulfill({ status: 200, contentType: tipos[extname(f)] || 'application/octet-stream', body: readFileSync(f) });
      }
      if (p === '/api/v1/nomes') return json(NOMES);
      if (p === '/api/v1/topicos') return json([{ topico: 'br-c0001', arquivo: 'br-c0001.json', seq: 400 }]);
      if (p === '/api/v1/mapa/secoes') return json(lista());
      if (p === '/api/v1/mapa/estado') return json(estadoMapa());
      return r.fulfill({ status: 404, contentType: 'application/json', body: '{"erro":"nao_encontrado"}' });
    });
    ctx.wsMapa = [];
    await ctx.routeWebSocket(`${ORIGEM.replace('http', 'ws')}/ws`, (ws) => {
      ctx.ws = ws;
      ws.send(JSON.stringify(ESTADO));
      ws.onMessage((txt) => {
        let m;
        try { m = JSON.parse(txt); } catch { return; }
        if (m.op === 'mapa') ctx.wsMapa.push(m);
        if (m.op !== 'inscrever') return;
        for (const t of m.topicos || []) if (t === 'br-c0001') ws.send(JSON.stringify({ tipo: 'instantaneo', topico: t, seq: 400, secoes_apuradas: ACESAS0, ver: 1, t_bu: null, t_envio: 0, resultado: RES }));
      });
    });
  }
  const p = await ctx.newPage();
  // 404 de outras rotas que este teste não imita (curva, previsão, playlist) não interessam; as do mapa, sim
  p.on('console', (m) => { if (m.type() === 'error' && !m.text().startsWith('Failed to load resource')) erros.push(`${largura}/${tema}: ${m.text()}`); });
  p.on('response', (r) => { const u = new URL(r.url()); if (r.status() >= 400 && (u.pathname.startsWith('/api/v1/mapa') || u.pathname.startsWith('/static/'))) erros.push(`${largura}/${tema}: ${r.status()} ${u.pathname}`); });
  p.on('pageerror', (e) => erros.push(`${largura}/${tema}: ${e.message}`));
  await p.goto(`${ORIGEM}/#/br/c0001`);
  await p.waitForSelector('.mapa', { timeout: 10000 });
  await p.evaluate(() => document.querySelector('.mapa').scrollIntoView({ block: 'center' }));
  // o CLS conta a partir daqui (a partida da página, com o topo e a fonte, é conferida pelos outros ensaios):
  // carregar o mapa, mensagens, marcos, troca de vista e maximizar não podem deslocar nada
  await p.evaluate(() => { window.__uv.cls = 0; window.__uv.clsFontes = []; });
  await p.waitForFunction(() => window.__uv.mapa?.resumo().carregado, null, { timeout: 15000 });
  await p.waitForTimeout(800);
  return { ctx, p };
}
const resumo = (p) => p.evaluate(() => window.__uv.mapa.resumo());
const caixa = (p, s) => p.evaluate((s) => { const b = document.querySelector(s).getBoundingClientRect(); return { l: b.left, t: b.top, w: b.width, h: b.height }; }, s);
// caixa sem a inclinação tátil (a `transform` do vidro muda o getBoundingClientRect sob o ponteiro)
const lugarDe = (p, s) => p.evaluate((s) => { const e = document.querySelector(s); let t = 0, l = 0; for (let x = e; x; x = x.offsetParent) { t += x.offsetTop; l += x.offsetLeft; } return { l, t, w: e.offsetWidth, h: e.offsetHeight }; }, s);

// 1) carga, fotografia e mensagens ao vivo
{
  const { ctx, p } = await abrir({ largura: 1366, tema: 'escuro' });
  let r = await resumo(p);
  conferir('carga', r.N === N && r.n === ACESAS0, JSON.stringify(r));
  conferir('fotografia', r.acesas === ACESAS0, `${r.acesas} ≠ ${ACESAS0}`);
  conferir('exterior contado', r.nExt === EXT.length * 5 && r.exterior === EXT.length * 5, JSON.stringify(r));
  conferir('exterior sem posição', r.semPosicao.length === 1 && r.semPosicao[0] === 'CIDADE INVENTADA', JSON.stringify(r.semPosicao));
  conferir('pedido mapa', ctx.wsMapa.some((m) => m.desde === ACESAS0 && m.epoca === EPOCA), JSON.stringify(ctx.wsMapa));
  // contador na tela
  const txt = await p.textContent('.mp-val');
  conferir('contador', txt.replace(/\D/g, '').startsWith(String(ACESAS0)) || (await p.evaluate(() => document.querySelector('.mp-n .hod') != null)), txt);
  // ao vivo: 200 seções novas (de = n), com um par repetido e uma já acesa
  const s = [];
  for (let i = ACESAS0; i < ACESAS0 + 200; i++) s.push(i, 175);
  s.push(3, 171);
  ctx.ws.send(JSON.stringify({ tipo: 'mapa', epoca: EPOCA, de: ACESAS0, n: ACESAS0 + 201, s }));
  await p.waitForTimeout(500);
  r = await resumo(p);
  conferir('ao vivo', r.acesas === ACESAS0 + 200 && r.n === ACESAS0 + 201, JSON.stringify(r));
  await p.screenshot({ path: join(saida, 'mapa-1366-escuro.png') });
  // mensagem atrasada (já vista) não muda nada; lacuna (de > n) relê a fotografia
  ctx.ws.send(JSON.stringify({ tipo: 'mapa', epoca: EPOCA, de: 0, n: 10, s: [0, 171] }));
  await p.waitForTimeout(200);
  conferir('mensagem velha', (await resumo(p)).acesas === ACESAS0 + 200);
  EPOCA = 222;
  ctx.ws.send(JSON.stringify({ tipo: 'mapa', epoca: EPOCA, de: 0, n: 5, s: [0, 171, 1, 171] }));
  await p.waitForFunction((e) => window.__uv.mapa.epoca === e, EPOCA, { timeout: 5000 }).catch(() => {});
  r = await resumo(p);
  conferir('outra época', r.acesas === ACESAS0 && r.n === ACESAS0, JSON.stringify(r));
  // B-064: o "de N" é o total da fotografia (sem as agregadas); agregada da noite: a mensagem traz o total novo e a
  // página relê a fotografia (que diz quais saíram: o total da UF também diminui)
  const deN = async () => Number((await p.textContent('.mp-val small')).replace(/\D/g, ''));
  conferir('B-064 total da fotografia', (await deN()) === N, `${await deN()} ≠ ${N}`);
  const totSp = await p.evaluate(() => window.__uv.mapa.totUf.get('sp'));
  RET = [N - 1, N - 2];
  ctx.ws.send(JSON.stringify({ tipo: 'mapa', epoca: EPOCA, de: ACESAS0, n: ACESAS0 + 1, total: N - 2, s: [ACESAS0, 175] }));
  await p.waitForFunction((t) => window.__uv.mapa.total === t, N - 2, { timeout: 5000 }).catch(() => {});
  await p.waitForTimeout(500);
  conferir('B-064 agregada da noite', (await deN()) === N - 2, `${await deN()} ≠ ${N - 2}`);
  conferir('B-064 total da UF', (await p.evaluate(() => window.__uv.mapa.totUf.get('sp'))) === totSp - 2);
  RET = [];
  EPOCA = 223;
  ctx.ws.send(JSON.stringify({ tipo: 'mapa', epoca: EPOCA, de: 0, n: 5, total: N, s: [0, 171] }));
  await p.waitForFunction((e) => window.__uv.mapa.epoca === e, EPOCA, { timeout: 5000 }).catch(() => {});
  conferir('B-064 nova época volta ao total', (await deN()) === N);
  r = await resumo(p);
  // marco no mapa: o selo aparece
  const [mu, , z, se] = chaves[ACESAS0 - 1];
  ctx.ws.send(JSON.stringify({ tipo: 'marco', n: 1, classe: 'bu_marco', seq: 100000, valor: 100000, horario: '2026-10-25 18:00:00', abrangencia: 'br', cargo: null, topico: null,
    uf: 'sp', municipio: mu, zona: z, secao: se, t_envio: 0 }));
  await p.waitForTimeout(700);
  conferir('selo do marco', await p.evaluate(() => document.querySelector('.mp-marco').classList.contains('ligado')));
  // B-061: o quadro é desenhado com o horário do requestAnimationFrame (o começo do quadro), que com a página ocupada
  // fica antes do início de uma explosão criada nele (performance.now()): o raio do anel não pode ficar negativo
  const explosaoAdiantada = async (vista) => {
    const antes = erros.length;
    await p.evaluate(() => { const m = window.__uv.mapa, t = performance.now() + 400; m.fx.push({ i: 0, t0: t, r: 3.2, grande: true }, { i: 1, t0: t, r: 1 }); });
    const n0 = (await resumo(p)).n;
    ctx.ws.send(JSON.stringify({ tipo: 'mapa', epoca: EPOCA, de: n0, n: n0 + 1, s: [n0 + 300, 172] }));
    await p.waitForTimeout(400);
    const novos = erros.slice(antes);
    conferir(`B-061 explosão adiantada (${vista})`, !novos.length, novos.join(' | '));
  };
  await explosaoAdiantada('Brasil');
  // vista Globo: o palco não muda de tamanho; a escolha fica guardada
  const antes = await lugarDe(p, '.mp-palco');
  await p.click('.mp-modo button[data-v="globo"]');
  await p.click('.mp-modo button[data-v="hora"]');
  await p.waitForTimeout(1200);
  const depois = await lugarDe(p, '.mp-palco');
  conferir('palco fixo', Math.abs(antes.w - depois.w) < 0.5 && Math.abs(antes.h - depois.h) < 0.5, `${JSON.stringify(antes)} → ${JSON.stringify(depois)}`);
  conferir('preferência guardada', await p.evaluate(() => localStorage.getItem('uv-mapa')) === '{"vista":"globo","cor":"hora"}');
  // arrastar o globo gira (o cartão não inclina)
  const c = await caixa(p, '.mp-fx');
  const lam0 = await p.evaluate(() => window.__uv.mapa.G.lam);
  await p.mouse.move(c.l + c.w / 2, c.t + c.h / 2); await p.mouse.down(); await p.mouse.move(c.l + c.w / 2 - 200, c.t + c.h / 2, { steps: 8 }); await p.mouse.up();
  conferir('globo gira', Math.abs((await p.evaluate(() => window.__uv.mapa.G.lam)) - lam0) > 20);
  await explosaoAdiantada('globo');
  await p.waitForTimeout(600);
  await p.screenshot({ path: join(saida, 'globo-1366-escuro.png') });
  // maximizar e voltar
  const lugar = await lugarDe(p, '.mapa');
  await p.hover('.mapa'); await p.click('.mapa .cf-max');
  await p.waitForTimeout(700);
  const mx = await caixa(p, '.mapa');
  conferir('maximizado', mx.w > lugar.w * 1.3 && await p.isVisible('.mp-ufs'), `${JSON.stringify(lugar)} → ${JSON.stringify(mx)}`);
  await p.screenshot({ path: join(saida, 'mapa-maximizado.png') });
  await p.keyboard.press('Escape');
  await p.waitForTimeout(700);
  const volta = await lugarDe(p, '.mapa');
  conferir('restaurado', Math.abs(volta.w - lugar.w) < 1 && Math.abs(volta.t - lugar.t) < 1 && !(await p.isVisible('.mp-ufs')), `${JSON.stringify(lugar)} → ${JSON.stringify(volta)}`);
  // maximizar outro cartão (a corrida)
  await p.hover('.corrida'); await p.click('.corrida .cf-max'); await p.waitForTimeout(600);
  conferir('corrida maximizada', await p.evaluate(() => document.querySelector('.corrida').classList.contains('max')));
  await p.click('.cartao-veu', { position: { x: 5, y: 5 } }); await p.waitForTimeout(600);
  conferir('véu fecha', await p.evaluate(() => !document.querySelector('.corrida').classList.contains('max')));
  // reorganizar pelo teclado: o mapa vai para a coluna lateral; a ordem fica depois de recarregar
  await p.focus('.mapa .cf-alca');
  await p.keyboard.press('ArrowRight');
  await p.waitForTimeout(500);
  const ordem = await p.evaluate(() => localStorage.getItem('uv-cartoes'));
  conferir('ordem guardada', ordem === '[["corrida"],["mapa","apuracao","previsao","curva","votos"]]', ordem);
  // CLS da primeira carga (mapa, mensagens, globo, maximizar, reorganizar); a recarga começa outra conta
  const cls1 = await p.evaluate(() => window.__uv.cls);
  conferir('CLS', cls1 < 0.001, `${cls1} ${JSON.stringify(await p.evaluate(() => window.__uv.clsFontes))}`);
  await p.reload();
  await p.waitForSelector('.mapa');
  await p.waitForTimeout(800);
  const ordem2 = await p.evaluate(() => [...document.querySelectorAll('.coluna-cartoes')].map((c) => [...c.children].map((x) => x.dataset.cartao)));
  conferir('ordem depois de recarregar', JSON.stringify(ordem2) === '[["corrida"],["mapa","apuracao","previsao","curva","votos"]]', JSON.stringify(ordem2));
  await p.evaluate(() => window.__uv.cartoes.restaurarLayout());
  await p.waitForTimeout(500);
  conferir('layout padrão', await p.evaluate(() => localStorage.getItem('uv-cartoes')) === '[["corrida","mapa"],["apuracao","previsao","curva","votos"]]');
  await ctx.close();
}

// 2) capturas: celular e tema claro
for (const [largura, tema] of [[390, 'escuro'], [390, 'claro'], [1366, 'claro']]) {
  const { ctx, p } = await abrir({ largura, tema });
  await p.waitForTimeout(600);
  const b = await caixa(p, '.mapa');
  const W = await p.evaluate(() => document.documentElement.clientWidth);
  conferir(`dentro da página ${largura} ${tema}`, b.l >= 0 && b.l + b.w <= W + 0.5, JSON.stringify(b));
  conferir(`sem rolagem lateral ${largura} ${tema}`, await p.evaluate(() => document.documentElement.scrollWidth <= document.documentElement.clientWidth));
  await p.locator('.mapa').screenshot({ path: join(saida, `mapa-${largura}-${tema}.png`) });
  const cls = await p.evaluate(() => window.__uv.cls);
  conferir(`CLS ${largura} ${tema}`, cls < 0.001, `${cls} ${JSON.stringify(await p.evaluate(() => window.__uv.clsFontes))}`);
  await ctx.close();
}

await nav.close();
for (const e of erros) conferir('console', false, e);
console.log(`mapa_pagina: ${oks} conferências ok, ${falhas.length} falha(s); capturas em ${saida}`);
process.exit(falhas.length ? 1 : 0);
