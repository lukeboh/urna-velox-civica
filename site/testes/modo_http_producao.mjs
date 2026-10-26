// Conferência do modo HTTP (B-026) contra o site de verdade, sem mexer em nada nos nós: um Chromium (Playwright) em
// que **só este navegador** tem o WebSocket recusado (o `/ws` é fechado na hora, como um 503 do `--max-clientes`);
// o resto (página, /api/v1/resultado, /api/v1/estado) vai ao servidor real, nos intervalos reais (60 s, ~2 min).
//   cd site && node testes/modo_http_producao.mjs [--url https://urnavelox.com.br/] [--hash '#/br/c0001']
//        [--liberar 150] [--saida /tmp/uv-modo-http]
// Confere: o modo HTTP entra em poucos segundos, o resultado da tela aparece, o aviso "atualizando a cada minuto" está
// à vista, as consultas são só do tópico da tela e do estado, a ~60 s, e respondem 200/304; depois de `--liberar` s o
// WebSocket deixa de ser bloqueado e a página tem de voltar ao tempo real na nova tentativa (~2 min). Carga: uma
// consulta por minuto de um navegador só. Fotos em --saida.
import { chromium } from 'playwright';
import { mkdirSync } from 'node:fs';

const arg = (k, d) => { const i = process.argv.indexOf(`--${k}`); return i > 0 ? process.argv[i + 1] : d; };
const url = arg('url', 'https://urnavelox.com.br/');
const hash = arg('hash', '');
const liberarS = +arg('liberar', '150');
const saida = arg('saida', '/tmp/uv-modo-http');
mkdirSync(saida, { recursive: true });
const falhas = [];
const conferir = (ok, o) => { console.log(`${ok ? 'ok   ' : 'FALHA'} ${o}`); if (!ok) falhas.push(o); };
const t0 = Date.now();
const s = () => Math.round((Date.now() - t0) / 1000);

const nav = await chromium.launch();
const ctx = await nav.newContext({ viewport: { width: 390, height: 844 }, deviceScaleFactor: 2, isMobile: true, hasTouch: true });
let liberado = false, tentativas = 0;
await ctx.routeWebSocket(/\/ws$/, (ws) => {
  tentativas++;
  if (liberado) { ws.connectToServer(); return; }
  ws.close({ code: 1013, reason: 'teste: WebSocket recusado só neste navegador' });
});
const consultas = [];
const p = await ctx.newPage();
p.on('response', (r) => {
  const u = new URL(r.url());
  if (/^\/api\/v1\/(resultado|estado|eventos|marcos)/.test(u.pathname)) consultas.push({ s: s(), caminho: u.pathname, status: r.status() });
});
await p.goto(url + hash, { waitUntil: 'load' });

const entrou = await p.waitForFunction(() => window.__uv?.conexao === 'http', null, { timeout: 30000 }).then(() => true).catch(() => false);
conferir(entrou, `modo HTTP em ${s()} s (${tentativas} tentativas de WebSocket recusadas)`);
const tela = await p.waitForFunction(() => { const S = window.__uv; return S?.tela && S.res.seq(S.tela.topico) >= 0 && document.querySelectorAll('#painel li.cand').length > 0 ? S.tela.topico : null; }, null, { timeout: 30000 })
  .then((h) => h.jsonValue()).catch(() => null);
conferir(!!tela, `resultado da tela à vista (${tela || 'nada'}, seq ${tela ? await p.evaluate((t) => window.__uv.res.seq(t), tela) : '—'})`);
const aviso = await p.getByText('atualizando a cada minuto').first().isVisible().catch(() => false);
conferir(aviso, 'aviso "atualizando a cada minuto (muitos acessos)" à vista');
await p.screenshot({ path: `${saida}/modo-http.png` });

// acompanha até liberar o WebSocket
while (s() < liberarS) {
  await p.waitForTimeout(10000);
  const e = await p.evaluate(() => ({ c: window.__uv.conexao, seq: window.__uv.tela ? window.__uv.res.seq(window.__uv.tela.topico) : null }));
  console.log(`  ${s()} s: conexão ${e.c}, seq da tela ${e.seq}, ${consultas.length} consultas`);
}
const res = consultas.filter((c) => c.caminho.startsWith('/api/v1/resultado/'));
const quais = [...new Set(res.map((c) => c.caminho))];
// só a tela (e, numa abrangência menor que a disputa, o tópico da circunscrição dela): no máximo 2 tópicos
conferir(res.length >= 2 && quais.length <= 2 && quais.includes(`/api/v1/resultado/${tela}`), `consultas de resultado: ${res.length} (${quais.join(', ')}); status ${[...new Set(res.map((c) => c.status))].join('/')}`);
const ts = res.map((c) => c.s);
const inter = ts.slice(1).map((t, i) => t - ts[i]).filter((d) => d > 2);
conferir(inter.every((d) => d >= 45 && d <= 75), `intervalo entre consultas ~60 s (${inter.join(', ')} s)`);
conferir(res.every((c) => c.status === 200 || c.status === 304), 'consultas respondidas com 200 ou 304');
conferir(!consultas.some((c) => /eventos|marcos/.test(c.caminho)), 'sem consultas de eventos/marcos');

liberado = true;
console.log(`  ${s()} s: WebSocket liberado; esperando a nova tentativa (~2 min)`);
const voltou = await p.waitForFunction(() => window.__uv.conexao === 'vivo', null, { timeout: 170000 }).then(() => true).catch(() => false);
conferir(voltou, `de volta ao tempo real em ${s()} s`);
const n = consultas.length;
await p.waitForTimeout(75000);
conferir(voltou && consultas.length === n, `sem consultas depois de voltar (${consultas.length - n})`);
conferir(voltou && !(await p.getByText('atualizando a cada minuto').first().isVisible().catch(() => false)), 'aviso sumiu');
await p.screenshot({ path: `${saida}/de-volta.png` });
await nav.close();
console.log(falhas.length ? `modo_http_producao: ${falhas.length} falha(s)` : 'modo_http_producao: tudo certo');
process.exit(falhas.length ? 1 : 0);
