// Teste do B-026 (acima do `--max-clientes`, a página não mostra a apuração): com Playwright/Chromium e um servidor
// falso **de verdade** (node:http, para o navegador usar o cache HTTP e ver a recusa do WebSocket como em produção):
// o `/ws` responde 503 "limite de clientes" e as rotas REST do protocolo (docs/PROTOCOLO_WS.md §6) respondem.
//   cd site && node testes/ws_recusado.mjs [--saida /tmp/uv-ws-recusado]
// Intervalos curtos no teste (`window.__uvConexao`): consulta a cada 1,5 s (o real é 60 s) e nova tentativa do
// WebSocket a cada 5 s (o real, ~2 min). Confere, a 1366 e a 390 px:
//   - o resultado do tópico da tela aparece (seq e candidatos no painel) e é atualizado quando o servidor muda;
//   - o estado da apuração chega (selo AO VIVO) e o aviso discreto "atualizando a cada minuto (muitos acessos)" aparece;
//   - só o tópico da tela e o estado são consultados (nenhum outro tópico, nem eventos/marcos), com If-None-Match
//     (304 sem corpo) e sem consultas mais frequentes que o intervalo (−20 %);
//   - o WebSocket é tentado de novo, sem martelar o servidor; quando ele aceita, a página volta ao tempo real, as
//     consultas param e o aviso some.
// E: com a base das consultas (`<meta name="uv-dados">`) fora do ar, a página consulta o próprio domínio.
import { chromium } from 'playwright';
import { createServer } from 'node:http';
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, existsSync } from 'node:fs';
import { dirname, join, extname } from 'node:path';
import { fileURLToPath } from 'node:url';

const arg = (k, d) => { const i = process.argv.indexOf(`--${k}`); return i > 0 ? process.argv[i + 1] : d; };
const saida = arg('saida', '/tmp/uv-ws-recusado');
mkdirSync(saida, { recursive: true });
const DIST = join(dirname(fileURLToPath(import.meta.url)), '..', 'dist');
const TIPOS = { '.js': 'text/javascript', '.css': 'text/css', '.svg': 'image/svg+xml', '.woff2': 'font/woff2', '.html': 'text/html', '.txt': 'text/plain', '.xml': 'application/xml' };
const CONSULTA_MS = 1500, NOVO_WS_MS = 5000;
const falhas = [];
const conferir = (ok, o) => { console.log(`${ok ? 'ok   ' : 'FALHA'} ${o}`); if (!ok) falhas.push(o); };

// ---------------------------------------------------------------- dados falsos (2º turno, Presidente)
const NOMES = { pleito: '545', ufs: { sp: 'SÃO PAULO' }, municipios: {}, eleicoes: [{ codigo: 545, turno: 2, cargos: [{ cargo: 1, vagas: 1, consulta: false, abrangencias: ['br', 'sp'],
  circunscricoes: [{ abr: 'br', eleitorado: 155000000, candidatos: [
    { n: 13, nome_urna: 'CANDIDATA A', nome: 'CANDIDATA A', partido: 'PT', vices: [] },
    { n: 22, nome_urna: 'CANDIDATO B', nome: 'CANDIDATO B', partido: 'PL', vices: [] }] }] }] }] };
let seq = 120000;
function resultado(topico) {
  const ap = Math.min(470000, Math.round(seq * 0.9));
  const a = 10 * seq + 1234, b = 9 * seq + 4321, val = a + b;
  return JSON.stringify({ v: 1, eleicao: 545, turno: 2, cargo: 1, abr: topico.slice(0, 2), circ: 'br', seq, ultimo_bu: '2026-10-25T18:30:00-03:00',
    totalizado_em: '2026-10-25T18:30:01.000-03:00', encerrada: false, secoes: { total: 470000, apuradas: ap }, eleitorado: 155000000, apurado: 30 * seq,
    comparecimento: 22 * seq, vb: seq, vn: 2 * seq, vnt: 0, validos: val, legenda: 0, estado: 'apurando',
    cand: [{ n: 13, v: a, pct: Math.round((10000 * a) / val) / 100, dvt: 'Válido', st: null, mat: null },
      { n: 22, v: b, pct: Math.round((10000 * b) / val) / 100, dvt: 'Válido', st: null, mat: null }] }) + '\n';
}
const estado = () => ({ versao: 'teste', estado: 'acompanhando', encerrada: false, pleito: 545, relogio_site: '25/10/2026 18:30:00',
  relogio_sistema: { agora: new Date().toISOString(), noite0: null, sistema0: null, acelerar: 1.0 },
  secoes: { catalogo: 470000, publicadas_cs: seq, aplicadas: seq, nao_instaladas: 0, pendentes: 0 },
  ultimo_bu: { seq, site: null, sistema: null }, pausa_429: { ativa: false, retomada_em: null, pausas: 0 }, conferencias: 0, alertas: {}, clientes: 5000 });

// ---------------------------------------------------------------- servidor falso
let aceitarWs = false;
let metaDados = '';                 // conteúdo do <meta name="uv-dados"> servido no index.html
const pedidos = [];                 // { t, caminho, inm, status }
const tentativasWs = [];            // instantes das tentativas de WebSocket
const abertos = new Set();
const INDEX = () => readFileSync(join(DIST, 'index.html'), 'utf8').replaceAll('{{versao}}', 'teste')
  .replace(/<meta name="uv-dados" content="[^"]*">/, `<meta name="uv-dados" content="${metaDados}">`);
const json = (r, status, corpo, cab = {}) => { r.writeHead(status, { 'Content-Type': 'application/json; charset=utf-8', 'Access-Control-Allow-Origin': '*', ...cab }); r.end(corpo); };

const pedidosDados = [];          // a "base das consultas" (outra origem, como o dados.urnavelox.com.br)
const atender = (lista) => (q, r) => {
  const p = new URL(q.url, 'http://x').pathname;
  const reg = { t: Date.now(), caminho: p, metodo: q.method, inm: q.headers['if-none-match'] || null, status: 200 };
  if (p.startsWith('/api/') || q.method === 'OPTIONS') lista.push(reg);
  if (q.method === 'OPTIONS') { reg.status = 405; r.writeHead(405); return r.end(); }   // o site dados não atende preflight
  if (p === '/' || p === '/index.html') { r.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8', 'Cache-Control': 'no-cache' }); return r.end(INDEX()); }
  if (p.startsWith('/static/')) {
    const f = join(DIST, p.slice(8));
    if (existsSync(f)) { r.writeHead(200, { 'Content-Type': TIPOS[extname(f)] || 'application/octet-stream' }); return r.end(readFileSync(f)); }
  }
  if (p === '/api/v1/nomes') return json(r, 200, JSON.stringify(NOMES), { 'Cache-Control': 'public, max-age=60' });
  if (p === '/api/v1/topicos') return json(r, 200, JSON.stringify(['br-c0001', 'sp-c0001'].map((t) => ({ topico: t, arquivo: `${t}-e000545.json`, seq }))));
  if (p === '/api/v1/estado') return json(r, 200, JSON.stringify(estado()), { 'Cache-Control': 'no-cache' });
  const m = /^\/api\/v1\/resultado\/([a-z0-9-]+)$/.exec(p);
  if (m && ['br-c0001', 'sp-c0001'].includes(m[1])) {
    const etag = `"${seq}"`;
    const cab = { ETag: etag, 'Cache-Control': 'public, max-age=1' };
    if (reg.inm && reg.inm.split(',').some((x) => x.trim() === etag)) { reg.status = 304; r.writeHead(304, { 'Access-Control-Allow-Origin': '*', ...cab }); return r.end(); }
    return json(r, 200, resultado(m[1]), cab);
  }
  reg.status = 404;
  return json(r, 404, '{"erro":"nao_encontrado"}');
};
const srv = createServer(atender(pedidos));
const srvDados = createServer(atender(pedidosDados));
await new Promise((ok) => srvDados.listen(0, '127.0.0.1', ok));
const ORIGEM_DADOS = `http://127.0.0.1:${srvDados.address().port}`;
/** Um quadro de texto do servidor (sem máscara). */
function quadro(s, txt) {
  const b = Buffer.from(txt);
  const h = b.length < 126 ? Buffer.from([0x81, b.length]) : Buffer.from([0x81, 126, b.length >> 8, b.length & 255]);
  s.write(Buffer.concat([h, b]));
}
srv.on('upgrade', (q, s) => {
  tentativasWs.push(Date.now());
  s.on('error', () => {});
  if (!aceitarWs) {
    // como o uv-servico acima do --max-clientes (difusao.rs): 503 no handshake
    const corpo = '{"erro":"limite de clientes"}';
    return s.end(`HTTP/1.1 503 Service Unavailable\r\nContent-Type: application/json\r\nContent-Length: ${Buffer.byteLength(corpo)}\r\nConnection: close\r\n\r\n${corpo}`);
  }
  const k = createHash('sha1').update(q.headers['sec-websocket-key'] + '258EAFA5-E914-47DA-95CA-C5AB0DC85B11').digest('base64');
  s.write(`HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: ${k}\r\n\r\n`);
  abertos.add(s);
  s.on('close', () => abertos.delete(s));
  s.on('data', () => { /* inscrições: o teste manda o instantâneo da tela de uma vez */ });
  quadro(s, JSON.stringify({ tipo: 'estado', ...estado() }));
  quadro(s, `{"tipo":"instantaneo","topico":"br-c0001","seq":${seq},"secoes_apuradas":1,"ver":1,"t_bu":null,"t_envio":0,"resultado":${resultado('br-c0001')}}`);
});
await new Promise((ok) => srv.listen(0, '127.0.0.1', ok));
const ORIGEM = `http://127.0.0.1:${srv.address().port}`;

// ---------------------------------------------------------------- página
const nav = await chromium.launch();
const ehResultado = (x) => x.caminho.startsWith('/api/v1/resultado/');
const ehEstado = (x) => x.caminho === '/api/v1/estado';
const intervalos = (ts) => ts.slice(1).map((t, i) => t - ts[i]);

async function abrir(largura) {
  const movel = largura < 640;
  const ctx = await nav.newContext(movel
    ? { viewport: { width: largura, height: 780 }, deviceScaleFactor: 2, isMobile: true, hasTouch: true, colorScheme: 'dark' }
    : { viewport: { width: largura, height: 900 }, colorScheme: 'light' });
  await ctx.addInitScript(([c, w]) => { window.__uvConexao = { consultaMs: c, novoWsMs: w }; }, [CONSULTA_MS, NOVO_WS_MS]);
  const p = await ctx.newPage();
  const erros = [];
  p.on('pageerror', (e) => erros.push(String(e)));
  await p.goto(`${ORIGEM}/#/br/c0001`);
  return { ctx, p, erros, tag: movel ? `celular-${largura}` : 'desktop' };
}

/** A tela mostra o resultado com esse seq (no estado da página e no painel, com os dois candidatos). */
const viu = (p, s, ms) => p.waitForFunction((x) => window.__uv?.res?.seq('br-c0001') === x && document.querySelectorAll('#painel li.cand').length === 2, s, { timeout: ms })
  .then(() => true).catch(() => false);

/** O aviso discreto do modo HTTP, inteiro à vista ({ visivel, caixa }). */
async function aviso(p) {
  const loc = p.getByText('atualizando a cada minuto', { exact: false }).first();
  const visivel = await loc.isVisible().catch(() => false);
  const caixa = visivel ? await loc.boundingBox() : null;
  return { visivel, caixa, inteiro: !!caixa && caixa.x >= 0 && caixa.x + caixa.width <= p.viewportSize().width + 0.5 };
}

for (const largura of [1366, 390]) {
  aceitarWs = false; metaDados = '';
  const i0 = pedidos.length, w0 = tentativasWs.length;
  const { ctx, p, erros, tag } = await abrir(largura);
  const seq1 = seq;
  conferir(await viu(p, seq1, 20000), `${tag}: com o /ws recusando (503), o resultado da tela aparece (seq ${seq1}, 2 candidatos no painel)`);
  seq += 517;
  conferir(await viu(p, seq, 3 * CONSULTA_MS + 2000), `${tag}: o resultado é atualizado pela consulta (seq ${seq})`);
  const st = await p.evaluate(() => ({ conexao: window.__uv.conexao, estado: !!window.__uv.estado, selo: document.querySelector('#conexao')?.textContent || '' }));
  conferir(st.estado && /AO VIVO/.test(st.selo), `${tag}: o estado da apuração chega por consulta (selo "${st.selo}", conexão "${st.conexao}")`);
  const av = await aviso(p);
  conferir(av.visivel && av.inteiro, `${tag}: aviso "atualizando a cada minuto (muitos acessos)" inteiro à vista (${av.caixa ? `${Math.round(av.caixa.x)}–${Math.round(av.caixa.x + av.caixa.width)} px` : 'não aparece'})`);
  await p.screenshot({ path: `${saida}/modo-http-${tag}.png` });
  await p.waitForTimeout(4 * CONSULTA_MS);

  const ped = pedidos.slice(i0);
  const res = ped.filter(ehResultado), est = ped.filter(ehEstado);
  const quais = [...new Set(res.map((x) => x.caminho))];
  conferir(res.length >= 3 && quais.length === 1 && quais[0] === '/api/v1/resultado/br-c0001', `${tag}: consulta só o tópico da tela (${res.length} consultas: ${quais.join(', ') || 'nenhuma'})`);
  conferir(est.length >= 3, `${tag}: consulta o estado da apuração (${est.length} consultas)`);
  conferir(!ped.some((x) => /\/api\/v1\/(eventos|marcos)/.test(x.caminho)), `${tag}: não consulta eventos nem marcos`);
  conferir(res.some((x) => x.inm) && res.some((x) => x.status === 304), `${tag}: If-None-Match (${res.filter((x) => x.inm).length} com, ${res.filter((x) => x.status === 304).length} respondidas com 304)`);
  const minRes = Math.min(...intervalos(res.map((x) => x.t)));
  conferir(res.length >= 2 && minRes >= 0.8 * CONSULTA_MS - 100, `${tag}: nenhuma consulta antes do intervalo − 20 % (menor intervalo ${minRes} ms; mínimo ${0.8 * CONSULTA_MS} ms)`);
  const ws = tentativasWs.slice(w0);
  const depois = intervalos(ws).slice(2);   // as 3 primeiras são a reconexão normal (1, 2 s), antes do modo HTTP
  conferir(ws.length >= 4 && depois.every((d) => d >= 0.8 * NOVO_WS_MS - 200), `${tag}: tenta o WebSocket de novo sem martelar (${ws.length} tentativas; intervalos depois do modo HTTP: ${depois.join(', ') || '—'} ms)`);

  // o servidor volta a aceitar: a página volta ao tempo real, as consultas param e o aviso some
  aceitarWs = true;
  const voltou = await p.waitForFunction(() => window.__uv.conexao === 'vivo', null, { timeout: 1.2 * NOVO_WS_MS + 3000 }).then(() => true).catch(() => false);
  conferir(voltou, `${tag}: quando o /ws aceita, a página volta ao tempo real`);
  const n0 = pedidos.filter((x) => ehResultado(x) || ehEstado(x)).length;
  await p.waitForTimeout(3 * CONSULTA_MS);
  const n1 = pedidos.filter((x) => ehResultado(x) || ehEstado(x)).length;
  conferir(voltou && n1 === n0, `${tag}: de volta ao WebSocket, as consultas param (${n1 - n0} depois de voltar)`);
  conferir(voltou && !(await aviso(p)).visivel, `${tag}: de volta ao WebSocket, o aviso some`);
  conferir(erros.length === 0, `${tag}: sem erro na página${erros.length ? ` (${erros.slice(0, 3).join(' | ')})` : ''}`);
  await ctx.close();
  for (const s of abertos) s.destroy();
}

// ---------------------------------------------------------------- base das consultas fora do ar → o próprio domínio
{
  aceitarWs = false; metaDados = 'http://127.0.0.1:9';
  const i0 = pedidos.length;
  const { ctx, p } = await abrir(390);
  conferir(await viu(p, seq, 20000), `base fora do ar (${metaDados}): o resultado aparece, consultado no próprio domínio`);
  conferir(pedidos.slice(i0).some(ehResultado), 'base fora do ar: as consultas chegam ao próprio domínio');
  await ctx.close();
}

// ---------------------------------------------------------------- base das consultas no ar (passo 3: o subdomínio dados)
{
  aceitarWs = false; metaDados = ORIGEM_DADOS;
  const i0 = pedidos.length, d0 = pedidosDados.length;
  const { ctx, p } = await abrir(390);
  const s1 = seq;
  conferir(await viu(p, s1, 20000), `base no ar (${ORIGEM_DADOS}): o resultado aparece, consultado na base`);
  seq += 211;
  conferir(await viu(p, seq, 3 * CONSULTA_MS + 2000), 'base no ar: o resultado é atualizado pela base');
  const dad = pedidosDados.slice(d0);
  conferir(dad.some(ehResultado) && dad.some(ehEstado), `base no ar: resultado e estado consultados na base (${dad.filter(ehResultado).length} + ${dad.filter(ehEstado).length})`);
  conferir(!dad.some((x) => x.metodo === 'OPTIONS'), `base no ar: sem preflight CORS (${dad.filter((x) => x.metodo === 'OPTIONS').length} OPTIONS)`);
  conferir(!pedidos.slice(i0).some((x) => ehResultado(x) || ehEstado(x)), 'base no ar: nenhuma consulta ao próprio domínio');
  await ctx.close();
}

// ---------------------------------------------------------------- ?modo=http: o modo HTTP à mão (conferir do celular)
{
  aceitarWs = true; metaDados = '';
  const i0 = pedidos.length, w0 = tentativasWs.length;
  const ctx = await nav.newContext({ viewport: { width: 390, height: 780 }, deviceScaleFactor: 2, isMobile: true, hasTouch: true });
  const p = await ctx.newPage();
  await p.goto(`${ORIGEM}/?modo=http#/br/c0001`);
  const entrou = await p.waitForFunction(() => window.__uv?.conexao === 'http', null, { timeout: 15000 }).then(() => true).catch(() => false);
  conferir(entrou, '?modo=http: com o /ws aceitando, a página entra no modo HTTP');
  conferir(await viu(p, seq, 10000), '?modo=http: o resultado aparece pelas consultas');
  conferir(pedidos.slice(i0).some(ehResultado), '?modo=http: consulta o resultado no servidor');
  conferir(tentativasWs.length === w0, `?modo=http: nenhuma conexão WebSocket aberta (${tentativasWs.length - w0})`);
  conferir((await aviso(p)).visivel, '?modo=http: aviso à vista');
  await p.screenshot({ path: `${saida}/modo-http-forcado.png` });
  await ctx.close();
}

await nav.close();
for (const s of abertos) s.destroy();
srv.close();
srvDados.close();
console.log(falhas.length ? `ws_recusado: ${falhas.length} falha(s)` : 'ws_recusado: tudo certo');
process.exit(falhas.length ? 1 : 0);
