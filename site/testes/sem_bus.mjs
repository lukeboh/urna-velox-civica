// M-019 (Luciano, 25/10/2026, 17h42): aviso no topo quando o TSE não publica BU novo há 10 min depois das 17h —
// "TSE sem novos BUs desde 17h11 · retomamos automaticamente quando voltarem" — e que **some sozinho** no primeiro BU
// novo. Com Playwright/Chromium e nenhum servidor (rotas e WebSocket falsos, como o antes_17h.mjs).
//   cd site && node testes/sem_bus.mjs [--saida /tmp/uv-sem-bus]
// Cenários (1366 e 360 px):
//   parado   — 25/10 17:40, último BU 17:11:45: o chip à vista com "desde 17h11" (no desktop, com o "retomamos…");
//              chega um estado com BU de 17:40:05: o chip some sozinho (em até 3 s).
//   recente  — 25/10 17:15, último BU 17:11:45 (4 min): nada.
//   cedo     — 25/10 03:10, último BU 01:49 (antes das 17h): nada.
import { chromium } from 'playwright';
import { mkdirSync, readFileSync, existsSync } from 'node:fs';
import { dirname, join, extname } from 'node:path';
import { fileURLToPath } from 'node:url';

const arg = (k, d) => { const i = process.argv.indexOf(`--${k}`); return i > 0 ? process.argv[i + 1] : d; };
const saida = arg('saida', '/tmp/uv-sem-bus');
mkdirSync(saida, { recursive: true });
const DIST = join(dirname(fileURLToPath(import.meta.url)), '..', 'dist');
const ORIGEM = 'http://uv.teste';
const TIPOS = { '.js': 'text/javascript', '.css': 'text/css', '.svg': 'image/svg+xml', '.woff2': 'font/woff2' };
const INDEX = readFileSync(join(DIST, 'index.html'), 'utf8').replaceAll('{{versao}}', 'teste').replace(/<meta name="uv-dados" content="[^"]*">/, '<meta name="uv-dados" content="">');
const falhas = [];
const conferir = (ok, o) => { console.log(`${ok ? 'ok   ' : 'FALHA'} ${o}`); if (!ok) falhas.push(o); };

const PROXIMA = { eleicao: '2026-10-25', nome: '2º turno das Eleições Gerais 2026', inicio: '2026-10-25T17:00:00-03:00', fim: '2026-10-26T08:00:00-03:00' };
const playlist = (agora) => ({ agora, suspensao: null, proxima_suspensao: null, proxima_ao_vivo: { ...PROXIMA, ao_vivo: agora >= PROXIMA.inicio }, no_ar: null, fila: [],
  opcoes: [], meu_voto: null, total_votos: 0, tocados: [] });
const site = (iso) => { const [d, h] = iso.split('T'); const [a, m, dd] = d.split('-'); return `${dd}/${m}/${a} ${h.slice(0, 8)}`; };
const estado = (agora, ultimo, seq) => ({ tipo: 'estado', versao: 'teste', estado: 'acompanhando', encerrada: false, pleito: 3221, relogio_site: site(agora),
  relogio_sistema: { agora: `${agora}.000-03:00`, noite0: null, sistema0: null, acelerar: 1.0 },
  secoes: { catalogo: 499248, publicadas_cs: seq, aplicadas: seq, nao_instaladas: 0, pendentes: 0 }, ultimo_bu: { seq, site: site(ultimo), sistema: null },
  pausa_429: { ativa: false, retomada_em: null, pausas: 0 }, conferencias: 0, alertas: {}, clientes: 1 });
const PRIMEIRA = { tipo: 'marco', n: 1, classe: 'primeira_secao', seq: 1, horario: '2026-10-25 01:49:24', abrangencia: 'br', cargo: null, topico: null,
  uf: 'zz', municipio: 30805, municipio_nome: 'WELLINGTON', zona: 1, secao: 1088 };

const nav = await chromium.launch();
async function abrir(largura, agora, ultimo) {
  const movel = largura < 640;
  const ctx = await nav.newContext({ ...(movel ? { viewport: { width: largura, height: 780 }, deviceScaleFactor: 2, isMobile: true, hasTouch: true }
    : { viewport: { width: largura, height: 900 } }), timezoneId: 'America/Sao_Paulo', locale: 'pt-BR' });
  await ctx.clock.setFixedTime(new Date(`${agora}-03:00`));
  await ctx.addInitScript(() => { try { localStorage.setItem('uv-aviso-dia', '2099-01-01'); } catch { /* sem armazenamento */ } });
  await ctx.route(`${ORIGEM}/**`, (r) => {
    const p = new URL(r.request().url()).pathname;
    if (p === '/' || p === '/index.html') return r.fulfill({ status: 200, contentType: 'text/html; charset=utf-8', body: INDEX });
    if (p.startsWith('/static/')) { const f = join(DIST, p.slice(8)); if (existsSync(f)) return r.fulfill({ status: 200, contentType: TIPOS[extname(f)] || 'application/octet-stream', body: readFileSync(f) }); }
    if (p === '/api/v1/nomes') return r.fulfill({ status: 200, contentType: 'application/json', body: 'null' });
    if (p === '/api/v1/topicos') return r.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
    if (p === '/api/v1/playlist') return r.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(playlist(`${agora}-03:00`)) });
    return r.fulfill({ status: 404, contentType: 'application/json', body: '{"erro":"nao_encontrado"}' });
  });
  await ctx.routeWebSocket(`${ORIGEM.replace('http', 'ws')}/ws`, (ws) => {
    ctx.ws = ws;
    ws.send(JSON.stringify(estado(agora, ultimo, 1636)));
    ws.onMessage((txt) => { let m; try { m = JSON.parse(txt); } catch { return; } if (m.op === 'marcos') ws.send(JSON.stringify(PRIMEIRA)); });
  });
  const p = await ctx.newPage();
  await p.goto(`${ORIGEM}/`);
  await p.waitForTimeout(2500);
  return { ctx, p, tag: movel ? `celular-${largura}` : 'desktop' };
}
const visivel = (p) => p.locator('.faixa .chip.parado').first().isVisible().catch(() => false);
const texto = async (p) => ((await p.locator('.faixa .chip.parado').first().textContent().catch(() => '')) || '').replace(/\s+/g, ' ').trim();

for (const largura of [1366, 360]) {
  {
    const { ctx, p, tag } = await abrir(largura, '2026-10-25T17:40:00', '2026-10-25T17:11:45');
    conferir(await visivel(p), `${tag} · parado: aviso à vista`);
    const t = await texto(p);
    if (largura > 640) {
      conferir(/TSE sem novos BUs há \d+ min \(desde 17h11\)/.test(t), `${tag} · parado: "há X min (desde 17h11)" ("${t}")`);
    } else {
      conferir(/TSE sem novos BUs há \d+ min/.test(t), `${tag} · parado: "há X min" ("${t}")`);
    }
    if (largura > 640) conferir(/retomamos automaticamente quando voltarem/.test(t), `${tag} · parado: "retomamos automaticamente quando voltarem"`);
    const c = await p.locator('.faixa .chip.parado').first().boundingBox().catch(() => null);
    conferir(!!c && c.x >= 0 && c.x + c.width <= largura, `${tag} · parado: chip dentro da tela${c ? ` (${Math.round(c.x)}–${Math.round(c.x + c.width)})` : ''}`);
    const fora = await p.$$eval('.faixa > *', (xs, L) => xs.filter((x) => { const b = x.getBoundingClientRect(); return b.width > 0 && b.right > L + 0.5; }).map((x) => x.className), largura);
    conferir(!fora.length, `${tag} · parado: nada da faixa sai da tela${fora.length ? ` (${fora.join(', ')})` : ''}`);
    await p.screenshot({ path: `${saida}/parado-${tag}.png` });
    // BU novo: o aviso some sozinho
    ctx.ws.send(JSON.stringify(estado('2026-10-25T17:40:06', '2026-10-25T17:40:05', 1700)));
    await p.waitForTimeout(3000);
    conferir(!(await visivel(p)), `${tag} · parado → BU novo: o aviso some sozinho`);
    await ctx.close();
  }
  for (const [nome, agora, ultimo] of [['recente (4 min)', '2026-10-25T17:15:00', '2026-10-25T17:11:45'], ['cedo (antes das 17h)', '2026-10-25T03:10:00', '2026-10-25T01:49:24']]) {
    const { ctx, p, tag } = await abrir(largura, agora, ultimo);
    conferir(!(await visivel(p)), `${tag} · ${nome}: sem o aviso`);
    await ctx.close();
  }
}

await nav.close();
console.log(falhas.length ? `sem_bus: ${falhas.length} falha(s)` : 'sem_bus: tudo certo');
process.exit(falhas.length ? 1 : 0);
