// Multipleito (1.1): consulta de um pleito encerrado pela série estática (serie.js), com o slider de cada visitante.
// Com Playwright/Chromium e **nenhum servidor**: página de `dist/` e a série real do suplementar de Governador de RR
// (pleito 3237; `testes/fixtures/consulta/3237/serie`, gerada pelo `uv-serie` de um replay do acervo, lado do TSE
// reconstruído pela hora de recebimento).
//   cd site && node testes/consulta_pagina.mjs [--saida /tmp/uv-consulta-fotos]
// Cenários (1366 px claro e 390 px escuro):
//   abre    — `#/p/3237` abre no fim da noite, parado: o resultado é a última versão da série (a publicação final), o
//             slider à vista, o lado do TSE é a reconstrução, o mapa com todas as seções acesas, sem WebSocket;
//   volta   — o slider vai ao meio da noite: o resultado, o mapa e o letreiro voltam àquele instante;
//   toca    — tocar a 300× faz a apuração andar (seq cresce);
//   painel  — no painel dos pleitos, o 3237 (encerrado com série) tem "Ver esta eleição" (botão principal, "na tela") para `#/p/3237`;
//   rota    — a rota guarda o pleito (`#/p/3237/rr/c0003`) e a troca de abrangência continua na consulta;
//   CLS < 0,01 em tudo.
import { chromium } from 'playwright';
import { mkdirSync, readFileSync, existsSync } from 'node:fs';
import { dirname, join, extname } from 'node:path';
import { fileURLToPath } from 'node:url';

const arg = (k, d) => { const i = process.argv.indexOf(`--${k}`); return i > 0 ? process.argv[i + 1] : d; };
const saida = arg('saida', '/tmp/uv-consulta-fotos');
mkdirSync(saida, { recursive: true });
const AQUI = dirname(fileURLToPath(import.meta.url));
const DIST = join(AQUI, '..', 'dist');
const FIX = join(AQUI, 'fixtures', 'consulta');
const ORIGEM = 'http://uv.teste';
const falhas = [];
const conferir = (ok, o) => { console.log(`${ok ? 'ok   ' : 'FALHA'} ${o}`); if (!ok) falhas.push(o); };
const TIPOS = { '.js': 'text/javascript', '.css': 'text/css', '.svg': 'image/svg+xml', '.woff2': 'font/woff2', '.html': 'text/html', '.json': 'application/json' };
const INDEX = readFileSync(join(DIST, 'index.html'), 'utf8').replaceAll('{{versao}}', 'teste');
const SERIE = join(FIX, '3237', 'serie');
const IND = JSON.parse(readFileSync(join(SERIE, 'indice.json'), 'utf8'));
const TOP = JSON.parse(readFileSync(join(SERIE, 't', 'rr-c0003.json'), 'utf8'));
const ultima = TOP.versoes.at(-1)[1];
/** versão em vigor no instante t (a última com hora ≤ t) */
const versaoEm = (t) => [...TOP.versoes].reverse().find(([tv]) => (tv ?? -Infinity) <= t)?.[1];

const PLEITOS = readFileSync(join(AQUI, '..', '..', 'deploy', 'oci', 'testes', 'fixtures', 'pleitos-2026-10-29.json'), 'utf8');

const nav = await chromium.launch(process.env.UV_CHROMIUM ? { executablePath: process.env.UV_CHROMIUM } : {});

async function abrir(largura, borda = true, indice = null) {
  const movel = largura < 640;
  const ctx = await nav.newContext({
    ...(movel ? { viewport: { width: largura, height: 780 }, deviceScaleFactor: 2, isMobile: true, hasTouch: true, colorScheme: 'dark' }
      : { viewport: { width: largura, height: 900 }, colorScheme: 'light' }),
    timezoneId: 'America/Sao_Paulo', locale: 'pt-BR',
  });
  let ws = 0;
  const dados = [];
  await ctx.addInitScript(() => { window.WebSocket = class { constructor() { window.__wsAbertos = (window.__wsAbertos || 0) + 1; } }; });
  if (indice) await ctx.addInitScript(() => { globalThis.__uvSerie = { atualizarMs: 500 }; });
  // a borda (meta uv-dados): serve as séries como o domínio dos dados; com `borda: false`, fora do ar (a página volta ao
  // próprio domínio)
  await ctx.route('https://dados.urnavelox.com.br/**', async (r) => {
    const p = new URL(r.request().url()).pathname;
    if (!borda) return r.abort();
    dados.push(p);
    if (p === '/consulta/indice.json') return r.fulfill({ status: 200, contentType: 'application/json', headers: { 'access-control-allow-origin': '*' }, body: JSON.stringify({ pleitos: [{ pleito: 3237 }] }) });
    const f = join(FIX, p.slice(10));
    if (p.startsWith('/consulta/') && existsSync(f)) return r.fulfill({ status: 200, contentType: 'application/json', headers: { 'access-control-allow-origin': '*' }, body: readFileSync(f) });
    return r.fulfill({ status: 404, headers: { 'access-control-allow-origin': '*' }, body: '{}' });
  });
  await ctx.route(`${ORIGEM}/**`, async (r) => {
    const p = new URL(r.request().url()).pathname;
    if (p === '/' || p === '/index.html') return r.fulfill({ status: 200, contentType: 'text/html; charset=utf-8', body: INDEX });
    if (p.startsWith('/static/')) {
      const f = join(DIST, p.slice(8));
      if (existsSync(f)) return r.fulfill({ status: 200, contentType: TIPOS[extname(f)] || 'application/octet-stream', body: readFileSync(f) });
    }
    if (indice && p === '/consulta/3237/serie/indice.json') return r.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(indice()) });
    if (p.startsWith('/consulta/')) {
      const f = join(FIX, p.slice(10));
      if (existsSync(f)) return r.fulfill({ status: 200, contentType: 'application/json', body: readFileSync(f) });
    }
    if (p === '/consulta/indice.json') return r.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ pleitos: [{ pleito: 3237 }] }) });
    if (p === '/api/v1/pleitos') return r.fulfill({ status: 200, contentType: 'application/json', body: PLEITOS });
    if (p === '/ws') ws++;
    return r.fulfill({ status: 404, contentType: 'application/json', body: '{"erro":"nao_encontrado"}' });
  });
  const pg = await ctx.newPage();
  pg.on('pageerror', (e) => conferir(false, `${largura}px: erro na página: ${e.message}`));
  return { ctx, pg, ws: () => ws, dados };
}

const esperar = (pg, f, arg, ms = 8000) => pg.waitForFunction(f, arg, { timeout: ms }).then(() => true).catch(() => false);

for (const [largura, borda] of [[1366, true], [390, false]]) {
  const { ctx, pg, dados } = await abrir(largura, borda);
  // com a comparação ligada (Urna Velox e TSE lado a lado), como o visitante escolhe
  await pg.goto(`${ORIGEM}/?comparar=ambos#/p/3237`);
  // abre: no fim da noite, parado
  const fim = await esperar(pg, (s) => window.__uv?.res?.get('rr-c0003')?.seq === s, ultima.seq);
  conferir(fim, `${largura}px abre: resultado = última versão da série (seq ${ultima.seq})`);
  conferir(await pg.locator('section.player').isVisible(), `${largura}px abre: slider à vista`);
  conferir(/^#\/p\/3237\/rr\/c0003$/.test(await pg.evaluate(() => location.hash)) || (await pg.evaluate(() => location.hash)) === '#/p/3237',
    `${largura}px abre: rota da consulta (${await pg.evaluate(() => location.hash)})`);
  const ap = await pg.evaluate(() => window.__uv.res.get('rr-c0003')?.secoes?.apuradas);
  conferir(ap === ultima.secoes.apuradas, `${largura}px abre: seções apuradas ${ap} = ${ultima.secoes.apuradas}`);
  conferir(await pg.evaluate(() => window.__wsAbertos || 0) === 0, `${largura}px abre: nenhum WebSocket`);
  // B-087: o suplementar de Governador de RR não é "Municipais"
  // M-025: a barra diz a eleição inteira ("Eleição suplementar · Roraima") e a data do pleito
  const nomeEl = () => document.querySelector('#eleicao .el-nome')?.textContent;
  const tipo = await esperar(pg, () => /suplementar/i.test(document.querySelector('#eleicao .el-nome')?.textContent || ''))
    ? await pg.evaluate(nomeEl) : await pg.evaluate(nomeEl);
  conferir(/suplementar/i.test(tipo || ''), `${largura}px abre: a barra diz "${tipo}" (suplementar de Governador)`);
  const data = await pg.evaluate(() => document.querySelector('#eleicao .el-dd')?.textContent);
  conferir(data === '21/06/2026', `${largura}px abre: a data do pleito na barra (${data})`);
  conferir(borda ? dados.some((p) => p.startsWith('/consulta/3237/serie/t/')) : dados.length === 0,
    `${largura}px abre: ${borda ? 'as séries vêm da borda (dados.urnavelox.com.br)' : 'borda fora do ar: as séries vêm do próprio domínio'} (${dados.length} pedidos à borda)`);
  conferir(await esperar(pg, () => window.__uv.tse.get('rr-c0003')?.dados?.reconstrucao === undefined && window.__uv.tse.get('rr-c0003')?.dados != null),
    `${largura}px abre: lado do TSE no fim = o oficial (sem a marca de reconstrução)`);
  await pg.screenshot({ path: join(saida, `consulta-${largura}-fim.png`) });
  // volta: meio da noite
  const meio = IND.instantes[Math.floor(IND.instantes.length / 2)] + 59;
  const vm = versaoEm(meio);
  await pg.evaluate((t) => window.__uv.player.ir(t), meio);
  const voltou = await esperar(pg, (s) => window.__uv.res.get('rr-c0003')?.seq === s, vm.seq);
  conferir(voltou, `${largura}px volta: resultado no meio da noite (seq ${vm.seq}, ${vm.secoes.apuradas} seções)`);
  const tse = await pg.evaluate(() => window.__uv.tse.get('rr-c0003')?.dados);
  conferir(tse?.reconstrucao === true, `${largura}px volta: lado do TSE marcado como reconstrução no meio da noite`);
  const marcosDepois = await pg.evaluate((t) => [...window.__uv.marcos.values()].filter((m) => m.horario && Date.parse(m.horario.replace(' ', 'T') + 'Z') / 1000 > t).length, meio);
  conferir(marcosDepois === 0, `${largura}px volta: nenhum marco depois do instante (${marcosDepois})`);
  await pg.screenshot({ path: join(saida, `consulta-${largura}-meio.png`) });
  // toca a 300×
  const s0 = await pg.evaluate(() => window.__uv.res.get('rr-c0003')?.seq);
  await pg.evaluate(() => window.__uv.player.velocidade(300));
  const andou = await esperar(pg, (a) => (window.__uv.res.get('rr-c0003')?.seq || 0) > a, s0, 10000);
  conferir(andou, `${largura}px toca: a apuração anda a 300× (seq de ${s0} para ${await pg.evaluate(() => window.__uv.res.get('rr-c0003')?.seq)})`);
  await pg.evaluate(() => window.__uv.player.velocidade(0));
  // painel: o pleito com série tem o link da consulta
  await esperar(pg, () => !!window.__uv.pleitos && window.__uv.consultas?.size > 0);
  await pg.evaluate(() => window.__uv.painelPleitos.abrir());
  const href = await pg.locator('#pleitos [data-pleito="3237"] a.pl-consulta').getAttribute('href').catch(() => null);
  conferir(href === '/#/p/3237', `${largura}px painel: "Ver esta eleição" no 3237 (${href})`);
  const t3237 = await pg.locator('#pleitos [data-pleito="3237"]').textContent();
  conferir(/Ver esta eleição/.test(t3237) && /na tela/.test(t3237) && await pg.locator('#pleitos [data-pleito="3237"] a.pl-acao.primaria').count() === 1,
    `${largura}px painel: o 3237 "na tela" com o botão principal "Ver esta eleição"`);
  const outros = await pg.locator('#pleitos a.pl-consulta').count();
  conferir(outros === 1, `${largura}px painel: só os pleitos com série têm o link (${outros})`);
  await pg.evaluate(() => window.__uv.painelPleitos.fechar());
  // rota: trocar de abrangência continua na consulta (o RR só tem o próprio RR; a rota já tem o pleito)
  await pg.evaluate(() => { location.hash = '#/p/3237/rr/c0003'; });
  await pg.waitForTimeout(400);
  conferir(await pg.evaluate(() => !!window.__uv.consulta && window.__uv.tela?.topico === 'rr-c0003'), `${largura}px rota: #/p/3237/rr/c0003 na consulta`);
  const cls = await pg.evaluate(() => window.__uv.cls);
  conferir(cls < 0.01, `${largura}px CLS ${cls.toFixed(4)} < 0,01`);
  await ctx.close();
}

// pleito em curso: a série é refeita durante a noite (índice com em_curso); o slider se estende quando o índice avança
// e o botão "ao vivo" volta à página em tempo real
{
  const corte = IND.instantes[Math.floor(IND.instantes.length / 2)];
  let servido = { ...IND, em_curso: true, fim: corte, instantes: IND.instantes.filter((t) => t <= corte) };
  const { ctx, pg } = await abrir(1366, false, () => servido);
  await pg.goto(`${ORIGEM}/#/p/3237`);
  const v0 = versaoEm(corte);
  conferir(await esperar(pg, (s) => window.__uv?.res?.get('rr-c0003')?.seq === s, v0.seq),
    `em curso abre: no último minuto consolidado (seq ${v0.seq})`);
  conferir(await pg.locator('section.player button.pl-aovivo').isVisible(), 'em curso: botão "● ao vivo" à vista');
  const max0 = await pg.evaluate(() => Number(document.querySelector('section.player input[type=range]')?.max));
  servido = { ...IND, em_curso: true };
  conferir(await esperar(pg, (m) => Number(document.querySelector('section.player input[type=range]')?.max) > m, max0),
    'em curso: o slider se estende quando a noite avança');
  conferir(await esperar(pg, (s) => window.__uv?.res?.get('rr-c0003')?.seq === s, ultima.seq),
    `em curso: parado no fim, acompanha a noite (seq ${ultima.seq})`);
  await pg.locator('section.player button.pl-aovivo').click();
  await pg.waitForTimeout(300);
  const h = await pg.evaluate(() => location.hash);
  conferir(/^#\/rr\/c0003$/.test(h), `em curso: "ao vivo" leva à página em tempo real (${h})`);
  await ctx.close();
}
// encerrado: sem o botão
{
  const { ctx, pg } = await abrir(1366, false);
  await pg.goto(`${ORIGEM}/#/p/3237`);
  await esperar(pg, (s) => window.__uv?.res?.get('rr-c0003')?.seq === s, ultima.seq);
  conferir(await pg.locator('section.player button.pl-aovivo').count() === 0, 'encerrado: sem o botão "ao vivo"');
  await ctx.close();
}

await nav.close();
console.log(falhas.length ? `\n${falhas.length} falha(s)` : '\ntodos os cenários passaram');
process.exit(falhas.length ? 1 : 0);
