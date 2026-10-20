// Teste do B-013/B-016 (a previsão não aparece / o gráfico da previsão não é atualizado): abre a página numa disputa
// durante um replay e, a cada segundo, anota o cartão "Previsão" — a mensagem `previsao` mais nova do tópico (instante,
// situação, pontos do histórico) e o que está desenhado (texto da faixa, caminho da banda P10–P90).
//   node testes/previsao_pagina.mjs --url http://127.0.0.1:18080/ --hash '#/br/c0001' --saida /tmp/x [--limite 900]
//        [--min-redesenhos 10] [--max-parado-s 20]
// Sai quando o "matematicamente" da tela sai (ou a apuração encerra, ou no limite). Falha se, depois da primeira
// previsão e antes do "matematicamente", o gráfico foi redesenhado menos que --min-redesenhos vezes ou ficou parado
// mais que --max-parado-s segundos com mensagens novas chegando. Grava <saida>/previsao.json.
import { chromium } from 'playwright';
import { mkdirSync, writeFileSync } from 'node:fs';

const arg = (k, d) => { const i = process.argv.indexOf(`--${k}`); return i > 0 ? process.argv[i + 1] : d; };
const url = arg('url', 'http://127.0.0.1:18080/');
const hash = arg('hash', '#/br/c0001');
const saida = arg('saida', '/tmp/uv-previsao-pagina');
const limite = +arg('limite', '900');
const minRedes = +arg('min-redesenhos', '10');
const maxParado = +arg('max-parado-s', '20');
mkdirSync(saida, { recursive: true });

const nav = await chromium.launch();
const pag = await (await nav.newContext({ viewport: { width: 1366, height: 900 } })).newPage();
const cdp = await pag.context().newCDPSession(pag);
await cdp.send('Network.enable');
const t0 = Date.now();
const msgs = []; // [s, instante, situacao, n_hist, tem_mat]
cdp.on('Network.webSocketFrameReceived', (e) => {
  const p = e.response?.payloadData || '';
  if (!p.startsWith('{"tipo":"previsao"') && !p.includes('"tipo":"previsao"')) return;
  try {
    const m = JSON.parse(p);
    msgs.push([+((Date.now() - t0) / 1000).toFixed(1), m.topico, m.instante, m.situacao, (m.historico || []).length, !!m.mat]);
  } catch { /* parcial */ }
});
await pag.goto(url + hash);
const amostras = [];
let fim = '';
while ((Date.now() - t0) / 1000 < limite) {
  await pag.waitForTimeout(1000);
  const a = await pag.evaluate(() => {
    const u = window.__uv; if (!u?.tela) return null;
    const r = u.res.get(u.tela.topico);
    const tc = r ? `${r.circ}-c${String(u.tela.cargo).padStart(4, '0')}` : u.tela.topico;
    const p = u.prev.get(tc) || u.prev.get(u.tela.topico);
    const d = document.querySelector('.previsao .pv-banda')?.getAttribute('d') || '';
    return { estado: r?.estado, encerrada: !!r?.encerrada, secoes: r?.secoes?.apuradas, instante: p?.instante, situacao: p?.situacao, hist: p?.historico?.length || 0,
      definida: p?.definida || null, faixa: document.querySelector('.previsao .pv-faixa')?.textContent || '', rot: document.querySelector('.previsao .pv-rot')?.textContent || '',
      d: d.length ? `${d.length}:${d.slice(-24)}` : '' };
  }).catch(() => null);
  if (!a) continue;
  amostras.push({ s: Math.round((Date.now() - t0) / 1000), ...a });
  if (a.estado && a.estado !== 'apurando') { fim = a.estado; break; }
  if (a.encerrada) { fim = 'encerrada'; break; }
}
await pag.screenshot({ path: `${saida}/previsao-1366.png`, fullPage: true }).catch(() => {});
await nav.close();
// janela avaliada: da primeira previsão desenhada até o "matematicamente" (ou o fim)
const com = amostras.filter((a) => a.d);
const ini = com[0]?.s ?? null;
let redes = 0, parado = 0, maiorParado = 0, ultD = null, ultS = ini;
for (const a of amostras) {
  if (ini == null || a.s < ini) continue;
  if (a.d !== ultD) { redes++; ultD = a.d; ultS = a.s; } else { parado = a.s - ultS; maiorParado = Math.max(maiorParado, parado); }
}
const r = { url: url + hash, fim, amostras: amostras.length, primeira_previsao_s: ini, redesenhos: redes, maior_parado_s: maiorParado, msgs_previsao: msgs.length,
  msgs_por_topico: msgs.reduce((o, m) => { o[m[1]] = (o[m[1]] || 0) + 1; return o; }, {}), ultimas: amostras.slice(-5), linha: amostras, mensagens: msgs.slice(-200) };
writeFileSync(`${saida}/previsao.json`, JSON.stringify(r, null, 1));
console.log(`previsao_pagina: fim=${fim || 'limite'}, ${msgs.length} mensagens previsao, primeira previsão desenhada em ${ini ?? '—'} s, ${redes} redesenhos do gráfico, maior trecho parado ${maiorParado} s`);
for (const a of amostras.filter((_, i) => i % Math.max(1, Math.floor(amostras.length / 12)) === 0)) console.log(`  ${a.s}s seções ${a.secoes} · ${a.rot} ${a.faixa} · hist ${a.hist} · inst ${a.instante} · d ${a.d.split(':')[0] || '—'}`);
const erros = [];
if (ini == null) erros.push('nenhuma previsão desenhada');
else {
  if (redes < minRedes) erros.push(`o gráfico foi redesenhado ${redes} vezes (mínimo ${minRedes})`);
  if (maiorParado > maxParado) erros.push(`o gráfico ficou parado ${maiorParado} s (máximo ${maxParado})`);
}
if (erros.length) { console.error('FALHOU\n  - ' + erros.join('\n  - ')); process.exit(1); }
console.log('ok');
