// Sonda no chip de BUs/min e janela de telemetria (sonda.js, telemetria.js), com Playwright/Chromium e **nenhum
// servidor** (página de `dist/` por rotas do Playwright, WebSocket falso, relógio da página controlado).
//   cd site && node testes/telemetria.mjs            (FOTOS=<pasta> grava capturas da janela e do chip)
// Primeiro a lógica (sonda.js, em Node); depois, a 1366 e a 390 px, nos dois temas:
//   pedido   — a página pede `op: sonda` ao conectar.
//   estável  — com sondas a cada 30 s, o chip alterna BUs/min ↔ "✓ estável · 4 ms" ("✓ 4 ms" no celular), dentro da
//              caixa do chip, sem pulo na tela (CLS 0).
//   janela   — tocar no chip abre a telemetria: 4 velocímetros (ritmo, atraso da sonda, BU recebido → publicado do
//              `estado`, última confirmação), ponteiros girados pelo valor, dentro da tela; Esc e tocar fora fecham.
//   parado   — sem sonda nova por mais que o limite (2,5 × o intervalo), o chip fica fixo em "⚠ sem sonda há …" e a
//              janela avisa; a sonda voltando, volta a "estável".
//   antigo   — servidor sem sonda: o chip nunca alterna.
import { chromium } from 'playwright';
import { readFileSync, existsSync, mkdirSync } from 'node:fs';
import { dirname, join, extname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { Telemetria, textoChip, textoMs } from '../src/sonda.js';

const DIST = join(dirname(fileURLToPath(import.meta.url)), '..', 'dist');
const ORIGEM = 'http://uv.teste';
const TIPOS = { '.js': 'text/javascript', '.css': 'text/css', '.svg': 'image/svg+xml', '.woff2': 'font/woff2', '.html': 'text/html', '.txt': 'text/plain', '.xml': 'application/xml' };
const INDEX = readFileSync(join(DIST, 'index.html'), 'utf8').replaceAll('{{versao}}', 'teste');
const FOTOS = process.env.FOTOS;
if (FOTOS) mkdirSync(FOTOS, { recursive: true });
const falhas = [];
const conferir = (ok, o) => { console.log(`${ok ? 'ok   ' : 'FALHA'} ${o}`); if (!ok) falhas.push(o); };

// ---------------------------------------------------------------- lógica (Node)
{
  const t = new Telemetria();
  const s = (id, inj, extra = {}) => ({ tipo: 'sonda', id, sessao: 7, local: true, origem: null, t_injecao: inj * 1000, t_difusao: inj * 1000 + 4200, t_envio: inj * 1000 + 4500, ...extra });
  conferir(t.situacao(0) === 'desconhecida' && textoChip(t, 0) === null, 'lógica: sem nada, situação desconhecida e nada no chip');
  t.conectou(0);
  conferir(t.situacao(200000) === 'desconhecida', 'lógica: servidor que nunca mandou sonda não é acusado');
  conferir(t.receber(s(4, 1000, { instantaneo: true }), 10) === false && t.suportada, 'lógica: a de inscrição só prova o suporte');
  conferir(t.receber(s(5, 30000), 100) && t.situacao(200) === 'estavel', 'lógica: sonda nova → estável');
  conferir(t.atrasoMs() === 4.2 && t.envioMs() === 4.5, `lógica: atraso injeção → difusão ${t.atrasoMs()} ms`);
  t.receber(s(6, 60000), 30100);
  t.receber(s(8, 90000), 60100);
  conferir(t.pulos === 1 && t.recebidas === 3, `lógica: uma sonda pulada (${t.pulos})`);
  conferir(t.intervaloMs() === 30000 && t.limiteMs() === 75000, `lógica: intervalo ${t.intervaloMs()} ms, limite ${t.limiteMs()} ms`);
  conferir(t.receber(s(8, 90000), 60200) === false, 'lógica: repetida ignorada');
  conferir(t.situacao(60100 + 75000) === 'estavel' && t.situacao(60100 + 75001) === 'atrasada', 'lógica: atrasada só depois do limite');
  conferir(t.situacao(60100 + 80000, false) === 'desconhecida', 'lógica: sem conexão não acusa');
  conferir(textoChip(t, 60100 + 80000).longo === '⚠ sem confirmação há 1,3 min'.replace('1,3 min', textoMs(80000)), `lógica: chip parado "${textoChip(t, 60100 + 80000).longo}"`);
  conferir(t.receber({ ...s(1, 1, { local: false, origem: 'no-b' }) }, 1) === false && t.remotas.get('no-b') === 1, 'lógica: sonda de outro nó só conta como eco');
  conferir(t.receber(s(1, 100000, { sessao: 9 }), 70000) && t.pulos === 1, 'lógica: serviço reiniciado (sessão nova) não conta pulo');
  conferir([0.4, 37.4, 1234, 12345, 130000].map(textoMs).join(' | ') === '<1 ms | 37 ms | 1,2 s | 12 s | 2 min', 'lógica: textoMs');
  conferir(t.receber({ tipo: 'sonda', id: 'x' }, 0) === false && t.receber(null, 0) === false, 'lógica: mensagens ilegíveis');
}

// ---------------------------------------------------------------- página
const T0 = Date.parse('2026-10-25T19:00:00-03:00');
const iso = (ms) => new Date(ms - 3 * 3600e3).toISOString().replace('Z', '-03:00');
function estado(seq, agoraMs) {
  return JSON.stringify({
    tipo: 'estado', versao: 'teste', estado: 'acompanhando', encerrada: false, pleito: 3221, relogio_site: '25/10/2026 19:00:00',
    relogio_sistema: { agora: iso(agoraMs), noite0: '2026-10-25T17:00:00-03:00', sistema0: iso(T0 - 3600e3), acelerar: 1.0 },
    secoes: { catalogo: 472000, publicadas_cs: seq, aplicadas: seq, nao_instaladas: 0, pendentes: 0 },
    ultimo_bu: { seq, site: '25/10/2026 19:00:00', sistema: iso(agoraMs) }, pausa_429: { ativa: false, retomada_em: null, pausas: 0 },
    conferencias: 0, alertas: {}, clientes: 1, latencia_bu_ms: { p50: 180.5, p99: 2400, n: 5000 },
  });
}
const sonda = (id, agoraMs, inst = false) => JSON.stringify({ tipo: 'sonda', id, sessao: 1, origem: null, local: true, t_injecao: (agoraMs - 5) * 1000,
  t_motor: (agoraMs - 4) * 1000, t_gravacao: (agoraMs - 2) * 1000, t_difusao: (agoraMs - 1) * 1000 + 200, t_envio: agoraMs * 1000, seq: 1, ms: { motor: 1, gravacao: 3 }, instantaneo: inst });

async function cenario(largura, tema, comSonda) {
  const celular = largura < 640;
  // UV_CHROMIUM: outro Chromium (ex.: o já instalado na máquina, quando a versão do Playwright do site não tem o seu)
  const nav = await chromium.launch(process.env.UV_CHROMIUM ? { executablePath: process.env.UV_CHROMIUM } : {});
  const ctx = await nav.newContext({ viewport: { width: largura, height: celular ? 780 : 860 }, deviceScaleFactor: 1, isMobile: celular, hasTouch: celular,
    timezoneId: 'America/Sao_Paulo', locale: 'pt-BR' });
  await ctx.addInitScript((t) => { try { localStorage.setItem('uv-tema', t); } catch { /* */ } }, tema);
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
  const pedidos = [];
  await ctx.routeWebSocket(`${ORIGEM.replace('http', 'ws')}/ws`, (ws) => { sock = ws; ws.onMessage((m) => pedidos.push(String(m))); });
  const p = await ctx.newPage();
  const erros = [];
  p.on('pageerror', (e) => erros.push(String(e)));
  await p.goto(`${ORIGEM}/`);
  for (let i = 0; i < 50 && !sock; i++) await p.clock.runFor(100);
  const nome = `${largura}px ${tema}${comSonda ? '' : ' (servidor sem sonda)'}`;
  conferir(!!sock, `${nome}: a página abriu o WebSocket`);
  await p.clock.runFor(300);
  conferir(pedidos.some((x) => JSON.parse(x).op === 'sonda'), `${nome}: a página pede op: sonda`);
  if (!comSonda) sock.send(JSON.stringify({ tipo: 'erro', motivo: 'op_desconhecida', detalhe: 'ops: inscrever, cancelar, eventos, marcos' }));

  let agora = T0, seq = 100000, id = 0;
  const passo = async (ms, sondas = comSonda) => {
    for (let k = 0; k < ms; k += 1000) {
      agora += 1000; seq += 700;
      if (Math.round((agora - T0) / 1000) % 2 === 0) sock.send(estado(seq, agora));
      if (sondas && Math.round((agora - T0) / 1000) % 30 === 0) sock.send(sonda(++id, agora));
      await p.clock.runFor(1000);
    }
  };
  if (comSonda) sock.send(sonda(0, agora - 20000, true)); // a de inscrição (antiga)
  await passo(62000);
  const chip = p.locator('.faixa .chip.ritmo');
  // M-023: durante a apuração ativa (seq crescendo, ritmo > 0), o chip não alterna para a sonda (operação já indica ok)
  const ativos = new Set();
  for (let i = 0; i < 40; i++) {
    const m = await chip.evaluate((c) => c.classList.contains('mostra-sonda'));
    ativos.add(m);
    agora += 250; if (i % 4 === 3) { seq += 700; sock.send(estado(seq, agora)); if (Math.round((agora - T0) / 1000) % 30 === 0 && comSonda) sock.send(sonda(++id, agora)); }
    await p.clock.runFor(250);
  }
  if (comSonda) {
    conferir(!ativos.has(true), `${nome}: durante apuração ativa, o chip não alterna (operação já indica ok)`);
    await chip.click();
    await p.clock.runFor(1200);
    await p.waitForTimeout(700);
    const vrAtivo = await p.evaluate(() => {
      const v = document.querySelector('#telemetria .vel');
      return { val: v?.querySelector('.vel-valor')?.textContent, rot: v?.querySelector('.ponteiro')?.style?.transform };
    });
    conferir(/^\d{2}\.\d{3}$/.test(vrAtivo.val) && vrAtivo.rot !== 'rotate(-90.00deg)', `${nome}: ritmo ativo na telemetria ${vrAtivo.val} (${vrAtivo.rot})`);
    await chip.click();
    await p.clock.runFor(1200);
    await p.waitForTimeout(700);
  }
  // agora simula sistema ocioso (esperando BU, seq constante por 70 s para zerar a janela deslizante de 60 s): o ritmo vai a 0 e o checkup alterna
  for (let k = 0; k < 70000; k += 1000) {
    agora += 1000;
    if (Math.round((agora - T0) / 1000) % 2 === 0) sock.send(estado(seq, agora));
    if (comSonda && Math.round((agora - T0) / 1000) % 30 === 0) sock.send(sonda(++id, agora));
    await p.clock.runFor(1000);
  }
  // alternância na ociosidade: amostra 18 s a cada 250 ms
  const vistos = new Set();
  let dentro = true, txt = '';
  for (let i = 0; i < 72; i++) {
    const r = await chip.evaluate((c) => {
      const l = c.querySelector('.ritmo-sonda'), vis = [...l.children].find((x) => getComputedStyle(x).display !== 'none');
      return { mostra: c.classList.contains('mostra-sonda'), sit: c.dataset.sonda || '', txt: vis?.textContent || '', cabe: !vis || vis.getBoundingClientRect().width <= c.clientWidth + 0.5,
        rotulo: c.getAttribute('aria-label') };
    });
    vistos.add(r.mostra);
    if (r.mostra) { txt = r.txt; dentro &&= r.cabe; }
    agora += 250; if (i % 4 === 3) { if (Math.round((agora - T0) / 1000) % 30 === 0 && comSonda) sock.send(sonda(++id, agora)); }
    await p.clock.runFor(250);
  }
  if (comSonda) {
    conferir(vistos.has(true) && vistos.has(false), `${nome}: ocioso, o chip alterna BUs/min e o checkup (${[...vistos].join('/')})`);
    const esperado = celular ? '✓ ativo' : '✓ checkup ok · ativo';
    conferir(txt === esperado, `${nome}: texto da sonda no chip "${txt}" (esperado "${esperado}")`);
    conferir(dentro, `${nome}: o texto da sonda cabe no chip`);
    if (FOTOS) { await chip.evaluate((c) => c.classList.add('mostra-sonda')); await p.waitForTimeout(600); await p.screenshot({ path: join(FOTOS, `chip-${largura}-${tema}.png`), clip: { x: 0, y: 0, width: largura, height: 140 } }); }
    // janela
    await chip.click();
    await p.clock.runFor(1200);
    await p.waitForTimeout(700); // as transições de CSS andam no tempo real (o relógio falso não as controla)
    const j = await p.evaluate(() => {
      const t = document.querySelector('#telemetria');
      const r = t.getBoundingClientRect();
      const vels = [...t.querySelectorAll('.vel')].map((v) => ({ val: v.querySelector('.vel-valor').textContent, cap: v.querySelector('figcaption').textContent,
        rot: v.querySelector('.ponteiro').style.transform, cor: v.dataset.cor }));
      return { aberta: t.classList.contains('aberta'), sit: t.dataset.situacao, estado: t.querySelector('.tele-estado').textContent, vels,
        naTela: r.left >= 0 && r.right <= innerWidth + 0.5 && r.top >= 0 && r.bottom <= innerHeight + 0.5, larg: r.width,
        caixa: [r.left, r.top, r.right, r.bottom].map(Math.round), chipFim: Math.round(document.querySelector('.faixa .chip.ritmo').getBoundingClientRect().bottom), linhas: t.querySelector('.tele-linhas').textContent };
    });
    conferir(j.aberta && j.sit === 'estavel', `${nome}: tocar no chip abre a telemetria (${j.sit})`);
    conferir(/^✓ Sistema estável — sonda nº \d+ confirmada há/.test(j.estado), `${nome}: estado "${j.estado}"`);
    conferir(j.vels.length === 4, `${nome}: 4 velocímetros (${j.vels.map((v) => v.cap).join(', ')})`);
    const [vr, va, vb, vi] = j.vels;
    conferir(vr.val === '0' && /^rotate\(-90(\.0+)?deg\)$/.test(vr.rot), `${nome}: ritmo em repouso ${vr.val} (${vr.rot})`);
    conferir(va.val === '4 ms' && va.cor === 'ok', `${nome}: atraso de processamento ${va.val} (${va.cor})`);
    conferir(vb.val === '181 ms' && vb.cor === 'ok', `${nome}: BU recebido → publicado ${vb.val} (do estado)`);
    conferir(/s$/.test(vi.val) && vi.cor === 'ok', `${nome}: última confirmação ${vi.val}`);
    conferir(j.naTela, `${nome}: a janela cabe na tela (${j.caixa})`);
    conferir(j.caixa[1] >= j.chipFim, `${nome}: a janela abre abaixo do chip, sem cobri-lo (janela ${j.caixa[1]}, chip até ${j.chipFim})`);
    conferir(j.linhas.includes('2,4 s') && j.linhas.includes('30 s'), `${nome}: linhas com o p99 e o intervalo`);
    if (FOTOS) await p.screenshot({ path: join(FOTOS, `janela-${largura}-${tema}.png`) });
    await p.keyboard.press('Escape');
    await p.clock.runFor(500);
    conferir(!(await p.evaluate(() => document.querySelector('#telemetria').classList.contains('aberta'))), `${nome}: Esc fecha`);
    // parado: 100 s sem sonda nova
    await passo(100000, false);
    const r = await chip.evaluate((c) => ({ sit: c.dataset.sonda, mostra: c.classList.contains('mostra-sonda'), txt: c.querySelector('.ritmo-sonda').textContent }));
    conferir(r.sit === 'atrasada' && r.mostra && r.txt.includes('⚠'), `${nome}: sem sonda há 100 s → chip fixo em "${r.txt}"`);
    await chip.click();
    await p.clock.runFor(1200);
    await p.waitForTimeout(700);
    const e2 = await p.evaluate(() => { const t = document.querySelector('#telemetria'), v = t.querySelectorAll('.vel');
      return [t.dataset.situacao, t.querySelector('.tele-estado').textContent, v[3].dataset.cor, v[1].querySelector('.vel-valor').textContent, v[1].dataset.cor]; });
    conferir(e2[0] === 'atrasada' && e2[1].startsWith('⚠') && e2[2] === 'erro', `${nome}: janela avisa "${e2[1]}" (última confirmação ${e2[2]})`);
    conferir(e2[3].startsWith('> ') && e2[4] === 'erro', `${nome}: com o caminho parado, o atraso de processamento mostra "${e2[3]}" (${e2[4]}), não a última sonda`);
    if (FOTOS) await p.screenshot({ path: join(FOTOS, `janela-parado-${largura}-${tema}.png`) });
    await p.mouse.click(5, (celular ? 780 : 860) - 5);
    await p.clock.runFor(500);
    conferir(!(await p.evaluate(() => document.querySelector('#telemetria').classList.contains('aberta'))), `${nome}: tocar fora fecha`);
    await passo(31000);
    conferir((await chip.evaluate((c) => c.dataset.sonda)) === 'estavel', `${nome}: a sonda voltou → estável`);
  } else {
    conferir(!vistos.has(true), `${nome}: sem sonda no servidor, o chip não alterna`);
    await chip.click();
    await p.clock.runFor(800);
    const e = await p.evaluate(() => document.querySelector('#telemetria .tele-estado').textContent);
    conferir(e === 'Este servidor não envia sondas.', `${nome}: a janela diz "${e}"`);
  }
  const cls = await p.evaluate(() => window.__uv.cls);
  conferir(cls === 0, `${nome}: CLS ${cls}`);
  conferir(!erros.length, `${nome}: sem erro de JavaScript ${erros.join(' | ')}`);
  await nav.close();
}

await cenario(1366, 'escuro', true);
await cenario(390, 'claro', true);
await cenario(390, 'escuro', false);
console.log(falhas.length ? `\n${falhas.length} FALHA(S)` : '\ntudo ok');
process.exit(falhas.length ? 1 : 0);
