// Artigos de lei na página viram link para a norma no site do TSE, numa aba nova, com âncora no artigo (normas.js;
// regra do Luciano, 29/10/2026). Duas partes:
//   tabela — sem navegador: cada artigo de ARTIGOS aponta para uma página do TSE (https://www.tse.jus.br/), com o
//            fragmento de texto `#:~:text=` igual ao início do artigo ("Art. N. …"), bem codificado;
//   página — Playwright/Chromium sem servidor (rotas e WebSocket falsos): Presidente com a apuração encerrada sem eleito
//            (estado `sem_eleito`, B-070) mostra "Sem eleito (art. 224)" com "art. 224" como link (target _blank,
//            rel noopener noreferrer, href da tabela), a 1366 e 360 px.
//   cd site && node testes/normas_links.mjs   (UV_CHROMIUM=<chromium> para outro navegador)
import { chromium } from 'playwright';
import { readFileSync, existsSync } from 'node:fs';
import { dirname, join, extname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { ARTIGOS, NORMAS, urlArtigo } from '../src/normas.js';

const AQUI = dirname(fileURLToPath(import.meta.url));
const DIST = join(AQUI, '..', 'dist');
const ORIGEM = 'http://uv.teste';
const falhas = [];
const conferir = (ok, o) => { console.log(`${ok ? 'ok   ' : 'FALHA'} ${o}`); if (!ok) falhas.push(o); };

// ---------------------------------------------------------------- tabela
for (const [chave, a] of Object.entries(ARTIGOS)) {
  const u = urlArtigo(chave);
  const [base, frag] = u.split('#:~:text=');
  conferir(!!NORMAS[a.norma] && base === NORMAS[a.norma].url && base.startsWith('https://www.tse.jus.br/legislacao/'),
    `${chave}: norma no site do TSE (${base})`);
  conferir(decodeURIComponent(frag) === a.inicio && a.inicio.startsWith(`Art. ${a.artigo}. `), `${chave}: âncora no início do artigo ("${decodeURIComponent(frag)}")`);
  conferir(!/[\s,&-]/.test(frag), `${chave}: fragmento codificado (sem espaço, vírgula, & nem hífen crus)`);
}
conferir(urlArtigo('art. 999') === null, 'artigo fora da tabela: sem link');

// ---------------------------------------------------------------- página
const TIPOS = { '.js': 'text/javascript', '.css': 'text/css', '.svg': 'image/svg+xml', '.woff2': 'font/woff2' };
const INDEX = readFileSync(join(DIST, 'index.html'), 'utf8').replaceAll('{{versao}}', 'teste').replace(/<meta name="uv-dados" content="[^"]*">/, '<meta name="uv-dados" content="">');
const NOMES = readFileSync(join(AQUI, 'fixtures', 'nomes_2t_presidente.json'), 'utf8');
const AGORA = '2026-10-26T02:30:00';
const estado = { tipo: 'estado', versao: 'teste', estado: 'encerrada', encerrada: true, pleito: 3221, relogio_site: '26/10/2026 02:30:00',
  relogio_sistema: { agora: `${AGORA}.000-03:00`, noite0: null, sistema0: null, acelerar: 1.0 },
  secoes: { catalogo: 1000, publicadas_cs: 1000, aplicadas: 1000, nao_instaladas: 0, pendentes: 0 }, ultimo_bu: { seq: 1000, site: '26/10/2026 02:00:00', sistema: `${AGORA}.000-03:00` },
  pausa_429: { ativa: false, retomada_em: null, pausas: 0 }, conferencias: 1, alertas: {}, clientes: 1 };
const resultado = { v: 1, eleicao: 6258, turno: 2, cargo: 1, abr: 'br', circ: 'br', seq: 1000, ultimo_bu: `${AGORA}-03:00`, totalizado_em: `${AGORA}.100-03:00`, encerrada: true,
  secoes: { total: 1000, apuradas: 1000 }, eleitorado: 300000, apurado: 300000, comparecimento: 250000, vb: 1000, vn: 2000, vnt: 0, validos: 90000, legenda: 0,
  estado: 'sem_eleito', cand: [{ n: 22, v: 157000, pct: null, dvt: 'Anulado sub judice', st: 'Não eleito', mat: null }, { n: 13, v: 90000, pct: 100, dvt: 'Válido', st: 'Não eleito', mat: null }] };

const nav = await chromium.launch(process.env.UV_CHROMIUM ? { executablePath: process.env.UV_CHROMIUM } : {});
for (const largura of [1366, 360]) {
  const movel = largura < 640, tag = movel ? `celular-${largura}` : 'desktop';
  const ctx = await nav.newContext({ ...(movel ? { viewport: { width: largura, height: 780 }, deviceScaleFactor: 2, isMobile: true, hasTouch: true } : { viewport: { width: largura, height: 900 } }),
    timezoneId: 'America/Sao_Paulo', locale: 'pt-BR' });
  await ctx.clock.setFixedTime(new Date(`${AGORA}-03:00`));
  await ctx.addInitScript(() => { try { localStorage.setItem('uv-aviso-dia', '2099-01-01'); } catch { /* */ } });
  await ctx.route(`${ORIGEM}/**`, (r) => {
    const p = new URL(r.request().url()).pathname;
    if (p === '/' || p === '/index.html') return r.fulfill({ status: 200, contentType: 'text/html; charset=utf-8', body: INDEX });
    if (p.startsWith('/static/')) { const f = join(DIST, p.slice(8)); if (existsSync(f)) return r.fulfill({ status: 200, contentType: TIPOS[extname(f)] || 'application/octet-stream', body: readFileSync(f) }); }
    if (p === '/api/v1/nomes') return r.fulfill({ status: 200, contentType: 'application/json', body: NOMES });
    if (p === '/api/v1/topicos') return r.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify([{ topico: 'br-c0001', arquivo: 'br-c0001-e006258.json', seq: 1000 }]) });
    return r.fulfill({ status: 404, contentType: 'application/json', body: '{"erro":"nao_encontrado"}' });
  });
  await ctx.routeWebSocket(`${ORIGEM.replace('http', 'ws')}/ws`, (ws) => {
    ws.onMessage(() => {});
    ws.send(JSON.stringify(estado));
    ws.send(JSON.stringify({ tipo: 'resultado', topico: 'br-c0001', seq: 1000, txt: JSON.stringify({ resultado }) }));
  });
  const p = await ctx.newPage();
  await p.goto(`${ORIGEM}/#/br/c0001`);
  const viu = await p.waitForSelector('.selo.ligado a.norma', { timeout: 15000 }).then(() => true).catch(() => false);
  conferir(viu, `${tag}: selo com o link do artigo`);
  if (viu) {
    const a = await p.evaluate(() => { const x = document.querySelector('.selo a.norma'); const s = x.closest('.selo');
      return { txt: s.textContent, link: x.textContent, href: x.href, target: x.target, rel: x.rel, title: x.title }; });
    conferir(a.txt === 'Sem eleito (art. 224)' && a.link === 'art. 224', `${tag}: texto "${a.txt}", link "${a.link}"`);
    conferir(a.href === urlArtigo('art. 224'), `${tag}: href = norma no TSE com âncora no art. 224`);
    conferir(a.target === '_blank' && /noopener/.test(a.rel) && /noreferrer/.test(a.rel), `${tag}: abre em aba nova (target ${a.target}, rel "${a.rel}")`);
    conferir(/Código Eleitoral/.test(a.title), `${tag}: title explica a norma`);
    const [pop] = await Promise.all([ctx.waitForEvent('page', { timeout: 5000 }).catch(() => null), p.click('.selo a.norma').catch(() => null)]);
    conferir(!!pop, `${tag}: o clique abre outra aba`);
    if (pop) await pop.close().catch(() => {});
    conferir(p.url().startsWith(ORIGEM), `${tag}: a página da apuração continua na aba dela`);
  }
  await ctx.close();
}
await nav.close();
console.log(falhas.length ? `\n${falhas.length} falha(s)` : '\ntudo ok');
process.exit(falhas.length ? 1 : 0);
