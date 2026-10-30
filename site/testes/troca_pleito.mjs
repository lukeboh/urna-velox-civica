// B-088: a playlist troca o pleito no ar (a cada meia hora, ou no voto) e a página aberta continuava com os nomes, os
// tópicos e o mapa do pleito anterior: municípios como "Ap06092" e candidatos pelo número. Com Playwright/Chromium e
// **nenhum servidor** (rotas e WebSocket falsos): a página abre no 2º turno de 2026 (3221, Presidente), o "serviço" passa
// a totalizar o suplementar de Oiapoque (3226, Prefeito) e a conexão cai e volta; a página tem de mostrar os nomes do
// pleito novo.
//   cd site && node testes/troca_pleito.mjs
import { chromium } from 'playwright';
import { readFileSync, existsSync } from 'node:fs';
import { dirname, join, extname } from 'node:path';
import { fileURLToPath } from 'node:url';

const AQUI = dirname(fileURLToPath(import.meta.url));
const DIST = join(AQUI, '..', 'dist');
const ORIGEM = 'http://uv.teste';
const falhas = [];
const conferir = (ok, o) => { console.log(`${ok ? 'ok   ' : 'FALHA'} ${o}`); if (!ok) falhas.push(o); };
const TIPOS = { '.js': 'text/javascript', '.css': 'text/css', '.svg': 'image/svg+xml', '.woff2': 'font/woff2' };
const INDEX = readFileSync(join(DIST, 'index.html'), 'utf8').replaceAll('{{versao}}', 'teste').replace(/<meta name="uv-dados" content="[^"]*">/, '<meta name="uv-dados" content="">');
const AGORA = '2026-10-26T02:30:00';
const base = (pleito) => ({ tipo: 'estado', versao: 'teste', estado: 'encerrada', encerrada: true, pleito, relogio_site: '26/10/2026 02:30:00',
  relogio_sistema: { agora: `${AGORA}.000-03:00`, noite0: null, sistema0: null, acelerar: 1.0 },
  secoes: { catalogo: 1000, publicadas_cs: 1000, aplicadas: 1000, nao_instaladas: 0, pendentes: 0 }, ultimo_bu: { seq: 1000, site: '26/10/2026 02:00:00', sistema: `${AGORA}.000-03:00` },
  pausa_429: { ativa: false, retomada_em: null, pausas: 0 }, conferencias: 1, alertas: {}, clientes: 1 });
const res = (eleicao, cargo, abr, cand, turno = 1) => ({ v: 1, eleicao, turno, cargo, abr, circ: abr, seq: 1000, ultimo_bu: `${AGORA}-03:00`, totalizado_em: `${AGORA}.100-03:00`, encerrada: true,
  secoes: { total: 88, apuradas: 88 }, eleitorado: 23054, apurado: 23054, comparecimento: 14417, vb: 87, vn: 105, vnt: 0, validos: 14225, legenda: 0, estado: 'definido',
  cand: cand.map(([n, v, pct, st]) => ({ n, v, pct, dvt: 'Válido', st, mat: null })) });
const P = {
  3221: { nomes: readFileSync(join(AQUI, 'fixtures', 'nomes_2t_presidente.json'), 'utf8'), topico: 'br-c0001', arquivo: 'br-c0001-e006258.json',
    res: res(6258, 1, 'br', [[22, 60000, 60, 'Eleito'], [13, 40000, 40, 'Não eleito']], 2) },
  3226: { nomes: JSON.stringify({ pleito: 3226, ufs: { ap: 'AMAPÁ' }, municipios: { ap06092: 'OIAPOQUE' }, eleicoes: [{ codigo: 6266, turno: 1, cargos: [{ cargo: 11, vagas: 1, consulta: false,
    abrangencias: ['ap06092'], circunscricoes: [{ abr: 'ap06092', eleitorado: 23054, candidatos: [{ n: 12, nome_urna: 'FULANO DE TAL', nome: 'FULANO DE TAL', partido: 'PDT', vices: [] },
      { n: 11, nome_urna: 'BELTRANO SILVA', nome: 'BELTRANO SILVA', partido: 'PP', vices: [] }] }] }] }] }),
  topico: 'ap06092-c0011', arquivo: 'ap06092-c0011-e006266.json', res: res(6266, 11, 'ap06092', [[12, 7036, 49.46, 'Eleito'], [11, 6527, 45.88, 'Não eleito']]) },
};
let atual = 3221;
const conexoes = [];

const nav = await chromium.launch(process.env.UV_CHROMIUM ? { executablePath: process.env.UV_CHROMIUM } : {});
const ctx = await nav.newContext({ viewport: { width: 1366, height: 900 }, timezoneId: 'America/Sao_Paulo', locale: 'pt-BR' });
await ctx.clock.setFixedTime(new Date(`${AGORA}-03:00`));
await ctx.addInitScript(() => { try { localStorage.setItem('uv-aviso-dia', '2099-01-01'); } catch { /* */ } });
await ctx.route(`${ORIGEM}/**`, (r) => {
  const p = new URL(r.request().url()).pathname;
  if (p === '/' || p === '/index.html') return r.fulfill({ status: 200, contentType: 'text/html; charset=utf-8', body: INDEX });
  if (p.startsWith('/static/')) { const f = join(DIST, p.slice(8)); if (existsSync(f)) return r.fulfill({ status: 200, contentType: TIPOS[extname(f)] || 'application/octet-stream', body: readFileSync(f) }); }
  if (p === '/api/v1/nomes') return r.fulfill({ status: 200, contentType: 'application/json', body: P[atual].nomes });
  if (p === '/api/v1/topicos') return r.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify([{ topico: P[atual].topico, arquivo: P[atual].arquivo, seq: 1000 }]) });
  if (p === '/api/v1/estado') return r.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(base(atual)) });
  return r.fulfill({ status: 404, contentType: 'application/json', body: '{"erro":"nao_encontrado"}' });
});
await ctx.routeWebSocket(`${ORIGEM.replace('http', 'ws')}/ws`, (ws) => {
  conexoes.push(ws);
  ws.onMessage(() => {});
  const x = P[atual];
  ws.send(JSON.stringify(base(atual)));
  ws.send(JSON.stringify({ tipo: 'resultado', topico: x.topico, seq: 1000, txt: JSON.stringify({ resultado: x.res }) }));
});
const pg = await ctx.newPage();
pg.on('pageerror', (e) => conferir(false, `erro na página: ${e.message}`));
await pg.goto(`${ORIGEM}/`);
const viu = (re, ms = 15000) => pg.waitForFunction((s) => new RegExp(s).test(document.body.innerText), re.source, { timeout: ms }).then(() => true).catch(() => false);
conferir(await viu(/LULA|BOLSONARO|Presidente/), 'abre no pleito 3221 (Presidente)');
// a playlist troca o pleito: o serviço reinicia com o 3226 e a conexão cai
atual = 3226;
for (const w of conexoes) await w.close().catch(() => {});
const nomes = await viu(/FULANO DE TAL/, 40000);
const txt = await pg.evaluate(() => document.body.innerText);
conferir(nomes, 'depois da troca, o candidato do pleito novo com o nome ("FULANO DE TAL")');
conferir(/Oiapoque/.test(txt) && !/\bAp06092\b/i.test(txt), `depois da troca, o município com o nome ("Oiapoque"), sem o código${/\bAp06092\b/i.test(txt) ? ' (apareceu "Ap06092")' : ''}`);
await nav.close();
console.log(falhas.length ? `\n${falhas.length} falha(s)` : '\ntodos os cenários passaram');
process.exit(falhas.length ? 1 : 0);
