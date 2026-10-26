// Teste do B-053 (o chip de ritmo chega a 200–300 mil BUs por minuto, claramente irreal): com Playwright/Chromium e
// **nenhum servidor** (página de `dist/` por rotas do Playwright, WebSocket falso) e o relógio da página controlado
// (`clock.install`). O serviço falso aplica 700 BUs/s (42.000 BUs/min) o tempo todo e manda o `estado` a cada 2 s.
//   cd site && node testes/ritmo_pagina.mjs
// Cenários (a 390 px):
//   contínuo  — 80 s de `estado` a cada 2 s: o chip mostra ~42 mil BUs/min.
//   pausa     — a página passa 90 s sem mensagem nenhuma (celular em segundo plano, WebSocket caído e reconectando) e
//               então recebe o `estado` com os BUs aplicados nesse meio-tempo: o chip continua ~42 mil (o salto do seq
//               é dos 90 s do servidor, não dos poucos segundos da página).
//   sessão nova — o replay recomeça (outro `sistema0`, seq do zero): o chip não fica negativo nem preso no salto.
import { chromium } from 'playwright';
import { readFileSync, existsSync } from 'node:fs';
import { dirname, join, extname } from 'node:path';
import { fileURLToPath } from 'node:url';

const DIST = join(dirname(fileURLToPath(import.meta.url)), '..', 'dist');
const ORIGEM = 'http://uv.teste';
const TIPOS = { '.js': 'text/javascript', '.css': 'text/css', '.svg': 'image/svg+xml', '.woff2': 'font/woff2', '.html': 'text/html', '.txt': 'text/plain', '.xml': 'application/xml' };
const INDEX = readFileSync(join(DIST, 'index.html'), 'utf8').replaceAll('{{versao}}', 'teste');
const falhas = [];
const conferir = (ok, o) => { console.log(`${ok ? 'ok   ' : 'FALHA'} ${o}`); if (!ok) falhas.push(o); };

const T0 = Date.parse('2026-10-21T10:00:00-03:00');
const POR_S = 700;   // BUs aplicados por segundo no serviço falso (42.000/min)
const iso = (ms) => new Date(ms - 3 * 3600e3).toISOString().replace('Z', '-03:00');

function estado(seq, agoraMs, sistema0) {
  return JSON.stringify({
    tipo: 'estado', versao: 'teste', estado: 'acompanhando', encerrada: false, pleito: 3220, relogio_site: '04/10/2026 19:03:08',
    relogio_sistema: { agora: iso(agoraMs), noite0: '2026-10-04T17:08:05-03:00', sistema0: iso(sistema0), acelerar: 1.0 },
    secoes: { catalogo: 517179, publicadas_cs: seq, aplicadas: seq, nao_instaladas: 0, pendentes: 0 },
    ultimo_bu: { seq, site: '04/10/2026 19:03:08', sistema: iso(agoraMs) }, pausa_429: { ativa: false, retomada_em: null, pausas: 0 },
    conferencias: 0, alertas: {}, clientes: 1, simulacao: { rotulo: '1º turno de 2026 · mesma ordem, acelerado' },
  });
}

const nav = await chromium.launch();
const ctx = await nav.newContext({ viewport: { width: 390, height: 780 }, deviceScaleFactor: 2, isMobile: true, hasTouch: true,
  timezoneId: 'America/Sao_Paulo', locale: 'pt-BR' });
await ctx.clock.install({ time: new Date(T0) });
await ctx.route(`${ORIGEM}/**`, async (r) => {
  const p = new URL(r.request().url()).pathname;
  if (p === '/' || p === '/index.html') return r.fulfill({ status: 200, contentType: 'text/html; charset=utf-8', body: INDEX });
  if (p.startsWith('/static/')) {
    const f = join(DIST, p.slice(8));
    if (existsSync(f)) return r.fulfill({ status: 200, contentType: TIPOS[extname(f)] || 'application/octet-stream', body: readFileSync(f) });
  }
  if (p === '/api/v1/nomes') return r.fulfill({ status: 200, contentType: 'application/json', body: 'null' });
  if (p === '/api/v1/topicos') return r.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
  return r.fulfill({ status: 404, contentType: 'application/json', body: '{"erro":"nao_encontrado"}' });
});
let sock = null;
await ctx.routeWebSocket(`${ORIGEM.replace('http', 'ws')}/ws`, (ws) => { sock = ws; ws.onMessage(() => { /* inscrições */ }); });
const p = await ctx.newPage();
await p.goto(`${ORIGEM}/`);
for (let i = 0; i < 50 && !sock; i++) await p.clock.runFor(100);
conferir(!!sock, 'a página abriu o WebSocket');

// relógio do serviço (ms) e BUs aplicados; a página e o serviço andam juntos, salvo na pausa
let srv = T0, seq = 120000, sistema0 = T0 - 3600e3;
const passo = async (ms, enviar = true) => {
  srv += ms; seq += Math.round((POR_S * ms) / 1000);
  if (enviar) sock.send(estado(seq, srv, sistema0));
  await p.clock.runFor(ms);
};
const chip = async () => {
  const t = (await p.locator('.faixa .chip.ritmo .sr').first().textContent().catch(() => '')) || '';
  return Number(t.replace(/\D/g, '')) || 0;
};
const perto = (v, alvo, tol) => Math.abs(v - alvo) <= alvo * tol;
const ESPERADO = POR_S * 60;

// ---------------------------------------------------------------- contínuo
for (let i = 0; i < 40; i++) await passo(2000);
let v = await chip();
conferir(perto(v, ESPERADO, 0.15), `contínuo: chip ${v.toLocaleString('pt-BR')} BUs/min (serviço a ${ESPERADO.toLocaleString('pt-BR')})`);

// ---------------------------------------------------------------- pausa de 90 s sem mensagens, depois o salto
srv += 90000; seq += POR_S * 90;
await p.clock.runFor(90000);
let maior = 0;
for (let i = 0; i < 10; i++) { await passo(2000); maior = Math.max(maior, await chip()); }
conferir(perto(maior, ESPERADO, 0.2), `pausa de 90 s: o maior valor do chip nos 20 s seguintes é ${maior.toLocaleString('pt-BR')} BUs/min (serviço a ${ESPERADO.toLocaleString('pt-BR')})`);

// ---------------------------------------------------------------- sessão nova do replay (seq recomeça)
sistema0 = srv; seq = 0;
let menor = Infinity; maior = 0;
for (let i = 0; i < 40; i++) { await passo(2000); const c = await chip(); menor = Math.min(menor, c); maior = Math.max(maior, c); }
v = await chip();
conferir(maior <= ESPERADO * 1.2, `sessão nova: o chip não salta (maior ${maior.toLocaleString('pt-BR')})`);
conferir(perto(v, ESPERADO, 0.15), `sessão nova: depois de 80 s o chip volta a ${v.toLocaleString('pt-BR')} BUs/min`);

await nav.close();
console.log(falhas.length ? `\n${falhas.length} FALHA(S)` : '\ntudo ok');
process.exit(falhas.length ? 1 : 0);
