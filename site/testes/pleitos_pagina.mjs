// Multipleito (1.1): a página mostra todos os pleitos do TSE (`GET /api/v1/pleitos`), com a situação de cada um, o
// que está no ar em destaque e o caminho para a playlist. Com Playwright/Chromium e **nenhum servidor** (como o
// modo_pagina.mjs): página de `dist/`, WebSocket falso e o pleitos.json da fixture de deploy/oci/testes (gerado pelo
// `uv-coletor pleitos` do ele-c.json real de 29/10/2026), com um suplementar agendado e um em curso acrescentados.
//   cd site && node testes/pleitos_pagina.mjs [--saida /tmp/uv-pleitos-fotos]
// Cenários (a 1366, 390 e 360 px; relógio fixo em 30/11/2026, 15h, America/Sao_Paulo):
//   lista   — replay do suplementar de Paranhos (3080) no ar: o aviso do selo leva aos pleitos; grupos em curso (1),
//             agendados (1) e encerrados (43); "no ar" no 3080; "faltam 6 dias" no agendado; link da playlist nos que
//             têm acervo e "replay em preparo" nos outros; filtro; painel inteiro na tela, sem rolagem lateral.
//   sem     — serviço sem --pleitos (404): o aviso não tem a linha "Pleitos".
import { chromium } from 'playwright';
import { mkdirSync, readFileSync, existsSync } from 'node:fs';
import { dirname, join, extname } from 'node:path';
import { fileURLToPath } from 'node:url';

const arg = (k, d) => { const i = process.argv.indexOf(`--${k}`); return i > 0 ? process.argv[i + 1] : d; };
const saida = arg('saida', '/tmp/uv-pleitos-fotos');
mkdirSync(saida, { recursive: true });
const AQUI = dirname(fileURLToPath(import.meta.url));
const DIST = join(AQUI, '..', 'dist');
const ORIGEM = 'http://uv.teste';
const falhas = [];
const conferir = (ok, o) => { console.log(`${ok ? 'ok   ' : 'FALHA'} ${o}`); if (!ok) falhas.push(o); };
const TIPOS = { '.js': 'text/javascript', '.css': 'text/css', '.svg': 'image/svg+xml', '.woff2': 'font/woff2', '.html': 'text/html' };
const INDEX = readFileSync(join(DIST, 'index.html'), 'utf8').replaceAll('{{versao}}', 'teste');
const AGORA = '2026-11-30T15:00:00-03:00';

const suplementar = (codigo, data, iso, local, situacao) => ({
  codigo, ciclo: 'ele2024', data, data_iso: iso, ano: +iso.slice(0, 4), turno: 1, situacao, nome: `Suplementar de ${local}`,
  titulo: `Eleição suplementar · ${local}`, complementares: 0, ufs: ['sp'], replay: false,
  eleicoes: [{ codigo: codigo + 3000, nome: 'Eleição Suplementar - x', local, turno: 1, tipo: 'suplementar', cargos: [{ codigo: 11, nome: 'Prefeito' }], ufs: ['sp'], municipios: [1] }],
});
const PLEITOS = (() => {
  const j = JSON.parse(readFileSync(join(AQUI, '..', '..', 'deploy', 'oci', 'testes', 'fixtures', 'pleitos-2026-10-29.json'), 'utf8'));
  j.pleitos.unshift(suplementar(3250, '06/12/2026', '2026-12-06', 'Vila Nova (SP)', 'agendado'),
    suplementar(3249, '30/11/2026', '2026-11-30', 'Lagoa Seca (SP)', 'em_curso'));
  j.contagem = { ...j.contagem, agendado: 1, em_curso: 1 };
  return j;
})();
const ROT = 'Suplementar de Paranhos (MS) (eleição de 06/04/2025) · mesma ordem de chegada dos BUs, acelerado';
const PLAYLIST = { agora: AGORA, suspensao: null, proxima_suspensao: null, proxima_ao_vivo: null,
  no_ar: { item: '3080-rapido', modo: 'rapido', pleito: '3080' }, fila: [], opcoes: [], meu_voto: null, total_votos: 0, tocados: [] };
const ESTADO = {
  tipo: 'estado', versao: 'teste', estado: 'acompanhando', encerrada: false, pleito: 3080, relogio_site: '06/04/2025 17:30:00',
  relogio_sistema: { agora: AGORA, noite0: null, sistema0: null, acelerar: 500 },
  secoes: { catalogo: 32, publicadas_cs: 10, aplicadas: 10, nao_instaladas: 0, pendentes: 0 },
  ultimo_bu: { seq: 10, site: null, sistema: null }, pausa_429: { ativa: false, retomada_em: null, pausas: 0 },
  conferencias: 0, alertas: {}, clientes: 1, simulacao: { rotulo: ROT },
};

// UV_CHROMIUM: outro executável do Chromium (ex.: um ambiente com outra versão do Playwright instalada)
const nav = await chromium.launch(process.env.UV_CHROMIUM ? { executablePath: process.env.UV_CHROMIUM } : {});

async function abrir(largura, comPleitos) {
  const movel = largura < 640;
  const ctx = await nav.newContext({
    ...(movel ? { viewport: { width: largura, height: 780 }, deviceScaleFactor: 2, isMobile: true, hasTouch: true, colorScheme: 'dark' }
      : { viewport: { width: largura, height: 900 }, colorScheme: 'light' }),
    timezoneId: 'America/Sao_Paulo', locale: 'pt-BR',
  });
  await ctx.clock.setFixedTime(new Date(AGORA));
  // o aviso não abre sozinho (já visto hoje): o teste abre pelo selo
  await ctx.addInitScript(() => { try { localStorage.setItem('uv-aviso-dia', '2026-11-30'); } catch { /* */ } });
  // a borda (meta uv-dados) sem séries de consulta: o teste não sai para a rede
  await ctx.route('https://dados.urnavelox.com.br/**', (r) => r.fulfill({ status: 404, headers: { 'access-control-allow-origin': '*' }, body: '{}' }));
  await ctx.route(`${ORIGEM}/**`, async (r) => {
    const p = new URL(r.request().url()).pathname;
    if (p === '/' || p === '/index.html') return r.fulfill({ status: 200, contentType: 'text/html; charset=utf-8', body: INDEX });
    if (p.startsWith('/static/')) {
      const f = join(DIST, p.slice(8));
      if (existsSync(f)) return r.fulfill({ status: 200, contentType: TIPOS[extname(f)] || 'application/octet-stream', body: readFileSync(f) });
    }
    if (p === '/api/v1/nomes') return r.fulfill({ status: 200, contentType: 'application/json', body: 'null' });
    if (p === '/api/v1/topicos') return r.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
    if (p === '/api/v1/playlist') return r.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(PLAYLIST) });
    if (p === '/api/v1/pleitos' && comPleitos) return r.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(PLEITOS) });
    return r.fulfill({ status: 404, contentType: 'application/json', body: '{"erro":"nao_encontrado"}' });
  });
  await ctx.routeWebSocket(`${ORIGEM.replace('http', 'ws')}/ws`, (ws) => { ws.onMessage(() => {}); ws.send(JSON.stringify(ESTADO)); });
  const p = await ctx.newPage();
  await p.goto(`${ORIGEM}/`);
  await p.waitForTimeout(1500);
  return { ctx, p, tag: movel ? `celular-${largura}` : 'desktop' };
}

const texto = async (p, sel) => ((await p.locator(sel).first().textContent().catch(() => '')) || '').replace(/\s+/g, ' ').trim();

for (const largura of [1366, 390, 360]) {
  {
    const { ctx, p, tag } = await abrir(largura, true);
    await p.click('#conexao');
    await p.waitForSelector('#folha.aberta', { timeout: 4000 }).catch(() => null);
    const aviso = await texto(p, '#folha');
    conferir(/Pleitos/.test(aviso) && /Ver todos os pleitos/.test(aviso), `${tag} · lista: o aviso do selo tem a linha "Pleitos"`);
    conferir(/1 em curso · 1 agendado · 43 encerrados/.test(aviso), `${tag} · lista: a contagem no botão ("${aviso.match(/\d+ em curso[^A-Z]*/)?.[0] || ''}")`);
    await p.click('.pl-abrir');
    const aberto = await p.waitForSelector('#pleitos.aberta', { state: 'visible', timeout: 4000 }).then(() => true).catch(() => false);
    conferir(aberto, `${tag} · lista: o painel abre`);
    await p.waitForTimeout(600);
    conferir(!(await p.isVisible('#folha.aberta')), `${tag} · lista: o aviso fecha ao abrir o painel`);
    const grupos = await p.locator('#pleitos .pl-grupo').allTextContents();
    conferir(JSON.stringify(grupos.map((g) => g.replace(/\s+/g, ' ').trim())) === JSON.stringify(['Em curso 1', 'Agendados 1', 'Encerrados 43']),
      `${tag} · lista: grupos ${JSON.stringify(grupos)}`);
    const it = (c) => texto(p, `#pleitos .pl-item[data-pleito="${c}"]`);
    const t3080 = await it(3080);
    conferir(/no ar/.test(t3080) && /Paranhos \(MS\)/.test(t3080) && /domingo, 06\/04\/2025/.test(t3080) && /Prefeito/.test(t3080),
      `${tag} · lista: 3080 no ar, com data e cargo ("${t3080}")`);
    conferir(/Pedir na playlist/.test(t3080) && (await p.getAttribute('#pleitos .pl-item[data-pleito="3080"] a', 'href')) === '/playlist',
      `${tag} · lista: 3080 com o link da playlist`);
    conferir(/replay em preparo/.test(await it(3237)), `${tag} · lista: 3237 (sem acervo) com "replay em preparo"`);
    const t3250 = await it(3250);
    conferir(/faltam 6 dias/.test(t3250) && /domingo, 06\/12\/2026/.test(t3250), `${tag} · lista: agendado com a data e "faltam 6 dias" ("${t3250}")`);
    conferir(/em curso/.test(await it(3249)), `${tag} · lista: em curso com o selo`);
    const t3221 = await it(3221);
    conferir(/Eleições Gerais 2026 · 2º turno/.test(t3221) && /Brasil, com 10 eleições suplementares e consultas/.test(t3221) && /Presidente · Governador · Prefeito · 3 consultas populares/.test(t3221),
      `${tag} · lista: 3221 com o Brasil e as complementares ("${t3221}")`);
    const t3220 = await it(3220);
    conferir(/Presidente · Governador · Senador · Conselheiro Distrital/.test(t3220) && !/consulta/.test(t3220),
      `${tag} · lista: 3220 com o Conselheiro Distrital (cargo 25 de uma eleição ordinária, não consulta) ("${t3220}")`);
    // painel inteiro na tela e sem rolagem lateral da página
    const larg = p.viewportSize().width;
    const c = await p.locator('#pleitos').boundingBox();
    const mg = larg < 640 ? 7.5 : 11.5;   // M-024: no celular, quase a tela inteira
    conferir(!!c && c.x >= mg && c.x + c.width <= larg - mg, `${tag} · lista: painel dentro da tela (${c ? `${Math.round(c.x)}–${Math.round(c.x + c.width)} de ${larg}` : '?'})`);
    // M-024: nada por cima do painel (no celular, o letreiro e o slider ficavam na frente da parte de baixo)
    const cobre = await p.evaluate(() => { const r = document.querySelector('#pleitos').getBoundingClientRect(); const e = document.elementFromPoint(r.left + r.width / 2, r.bottom - 12);
      return e && !e.closest('#pleitos') ? (e.className || e.tagName) : ''; });
    conferir(!cobre, `${tag} · lista: nada por cima da parte de baixo do painel${cobre ? ` (${cobre})` : ''}`);
    // M-024: o título diz o que fazer
    conferir(/Escolha a eleição/.test(await texto(p, '#pleitos h2')) && /Toque numa eleição/.test(await texto(p, '#pleitos .pl-intro')), `${tag} · lista: título e instrução`);
    conferir(/No dia da eleição, a apuração aparece aqui ao vivo/.test(t3250), `${tag} · lista: o agendado diz o que vai acontecer`);
    conferir(await p.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth + 1), `${tag} · lista: sem rolagem lateral`);
    conferir(await p.locator('#pleitos .pl-lista').evaluate((e) => e.scrollHeight > e.clientHeight), `${tag} · lista: a lista rola por dentro do painel`);
    await p.screenshot({ path: join(saida, `${tag}-lista.png`) });
    // filtro
    await p.fill('#pleitos .hist-busca', 'paranhos');
    await p.waitForTimeout(200);
    const n = await p.locator('#pleitos .pl-item').count();
    conferir(n === 1, `${tag} · lista: filtro "paranhos" → ${n} pleito`);
    await p.fill('#pleitos .hist-busca', 'xyz-nada');
    conferir(/Nenhum pleito passa no filtro/.test(await texto(p, '#pleitos')), `${tag} · lista: filtro sem resultado avisa`);
    await p.keyboard.press('Escape');
    await p.waitForTimeout(500);
    conferir(!(await p.isVisible('#pleitos.aberta')), `${tag} · lista: Esc fecha`);
    // M-024/M-025: a eleição da barra abre a lista (com o "▾" à vista)
    const troca = await p.evaluate(() => { const t = document.querySelector('#eleicao .el-pleito > .el-seta'); return t && getComputedStyle(t).visibility === 'visible' && t.getBoundingClientRect().width > 0 ? t.textContent : ''; });
    conferir(/▾/.test(troca), `${tag} · barra: "${troca}" à vista na eleição`);
    await p.click('#eleicao .el-pleito');
    conferir(await p.waitForSelector('#pleitos.aberta', { state: 'visible', timeout: 4000 }).then(() => true).catch(() => false), `${tag} · barra: tocar na eleição abre a lista`);
    await ctx.close();
  }
  {
    const { ctx, p, tag } = await abrir(largura, false);
    await p.click('#conexao');
    await p.waitForSelector('#folha.aberta', { timeout: 4000 }).catch(() => null);
    conferir(!/Ver todos os pleitos/.test(await texto(p, '#folha')), `${tag} · sem: sem /api/v1/pleitos, nenhuma linha "Pleitos"`);
    await ctx.close();
  }
}
await nav.close();
console.log(falhas.length ? `\n${falhas.length} falha(s)` : '\ntudo ok');
process.exit(falhas.length ? 1 : 0);
