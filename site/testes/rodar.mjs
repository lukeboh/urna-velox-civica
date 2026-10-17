// Ensaio automatizado da página (etapa 5), com Playwright/Chromium (sem interface):
//   node testes/rodar.mjs --url http://127.0.0.1:18080/ --saida /home/dev/uvreal/etapa5/X [--hash '#/br/c0001']
//        [--movel] [--fotos 30] [--limite 3600] [--conferir todos|nenhum] [--ate encerrada|tempo]
// Acompanha a apuração sem recarregar; mede CLS (layout-shift sem input recente), quadros por segundo, tarefas
// longas (> 50 ms), memória do JS, mensagens e quedas de conexão; guarda fotos; confere que nenhuma requisição sai da
// origem; no fim (apuração encerrada) passa por todos os tópicos e lê os números **da tela** (texto dos hodômetros)
// para a conferência com os oficiais (testes/conferir.py).
import { chromium } from 'playwright';
import { mkdirSync, writeFileSync, appendFileSync } from 'node:fs';

const arg = (k, d) => { const i = process.argv.indexOf(`--${k}`); return i > 0 ? process.argv[i + 1] : d; };
const tem = (k) => process.argv.includes(`--${k}`);
const url = arg('url', 'http://127.0.0.1:18080/');
const saida = arg('saida', '/tmp/uv-pagina');
const hash = arg('hash', '');
const movel = tem('movel');
const fotos = +arg('fotos', '0');
const limite = +arg('limite', '3600');
const conferir = arg('conferir', 'todos');
const ate = arg('ate', 'encerrada');
mkdirSync(saida, { recursive: true });
const log = (o) => { const l = JSON.stringify({ t: new Date().toISOString(), ...o }); console.log(l); appendFileSync(`${saida}/pagina.jsonl`, l + '\n'); };

const navegador = await chromium.launch({ args: ['--enable-precise-memory-info'] });
const ctx = await navegador.newContext({ ...(movel
  ? { viewport: { width: 360, height: 780 }, deviceScaleFactor: 2, isMobile: true, hasTouch: true }
  : { viewport: { width: 1440, height: 900 }, deviceScaleFactor: 1 }), reducedMotion: tem('reduzido') ? 'reduce' : 'no-preference' });
const pagina = await ctx.newPage();
if (tem('claro')) await pagina.addInitScript(() => { try { localStorage.setItem('uv-tema', 'claro'); } catch { /* */ } });
const origem = new URL(url).origin;
const externas = [];
const erros = [];
pagina.on('request', (r) => { const u = r.url(); if (!u.startsWith(origem) && !u.startsWith('data:')) externas.push(u); });
pagina.on('websocket', (w) => { if (!w.url().startsWith(origem.replace(/^http/, 'ws'))) externas.push(w.url()); });
pagina.on('pageerror', (e) => { erros.push(String(e)); log({ erro_pagina: String(e) }); });
pagina.on('console', (m) => { if (m.type() === 'error') { erros.push(m.text()); log({ console: m.text() }); } });
let recargas = 0;
pagina.on('framenavigated', (f) => { if (f === pagina.mainFrame() && !f.url().includes('#')) recargas++; });

await pagina.addInitScript(() => {
  window.__teste = { quadros: 0, longas: 0, maiorLonga: 0 };
  const conta = () => { window.__teste.quadros++; requestAnimationFrame(conta); };
  requestAnimationFrame(conta);
  try {
    new PerformanceObserver((l) => { for (const e of l.getEntries()) { window.__teste.longas++; window.__teste.maiorLonga = Math.max(window.__teste.maiorLonga, e.duration); } })
      .observe({ type: 'longtask', buffered: true });
  } catch { /* sem longtask */ }
});

await pagina.goto(url + hash, { waitUntil: 'load' });
const t0 = Date.now();
let ultFoto = 0, ultQuadros = 0, ultT = Date.now(), n = 0;
const amostras = [];
for (;;) {
  await pagina.waitForTimeout(5000);
  const m = await pagina.evaluate(() => {
    const S = window.__uv;
    const res = S.tela ? S.res.get(S.tela.topico) : null;
    return { cls: S.cls, msgs: S.msgs, quedas: S.con?.quedas ?? 0, conexao: S.conexao, encerrada: !!S.estado?.encerrada, telaEncerrada: !!res?.encerrada,
      seq: res?.seq ?? null, quadros: window.__teste.quadros, longas: window.__teste.longas, maiorLonga: Math.round(window.__teste.maiorLonga),
      heap: performance.memory ? Math.round(performance.memory.usedJSHeapSize / 1e6) : null, marcos: S.marcos.size, eventos: S.eventos.size, faltam: S.faltam.size,
      feed: document.querySelectorAll('#trilho .item').length, scrollW: document.scrollingElement.scrollWidth, larg: innerWidth };
  });
  const agora = Date.now();
  m.fps = Math.round(((m.quadros - ultQuadros) * 1000) / (agora - ultT));
  ultQuadros = m.quadros; ultT = agora;
  m.s = Math.round((agora - t0) / 1000);
  amostras.push(m);
  if (n++ % 6 === 0) log(m);
  if (fotos && agora - ultFoto >= fotos * 1000) { ultFoto = agora; await pagina.screenshot({ path: `${saida}/foto_${String(m.s).padStart(5, '0')}.png` }); }
  if (ate === 'encerrada' && m.encerrada && m.telaEncerrada) break;
  if ((agora - t0) / 1000 > limite) { log({ aviso: 'limite de tempo' }); break; }
}
await pagina.waitForTimeout(3000);
await pagina.screenshot({ path: `${saida}/final.png` });
const clsAcompanhamento = await pagina.evaluate(() => window.__uv.cls);

// números da tela de cada tópico (depois de as animações assentarem)
const numeros = {};
if (conferir === 'todos') {
  const topicos = await pagina.evaluate(() => [...window.__uv.topicos].sort());
  for (const t of topicos) {
    const [a, c] = t.split('-c');
    await pagina.evaluate((h) => { location.hash = h; }, `#/${a}/c${c}`);
    await pagina.waitForTimeout(2600);
    numeros[t] = await pagina.evaluate(() => {
      const txt = (e) => e?.querySelector('.sr')?.textContent ?? null;
      const num = (s) => (s == null || s === '—' ? null : Number(s.replace(/\./g, '').replace(',', '.').replace(/[^\d.]/g, '')));
      const S = window.__uv, ui = S.ui;
      if (!ui || ui.pendente) return null;
      const cand = [...document.querySelectorAll('#painel li.cand')].map((li) => ({ n: +li.dataset.n, v: num(txt(li.querySelector('.qtd'))), pct: num(txt(li.querySelector('.pct'))),
        pos: +getComputedStyle(li).getPropertyValue('--pos') }));
      return { topico: ui.topico, seq: S.res.get(ui.topico)?.seq, cand, secoes: num(txt(ui.secAp.el.parentElement)), total: num(txt(ui.secTot.el.parentElement)),
        comparecimento: num(txt(ui.comp.h.el.parentElement)), brancos: num(txt(ui.brancos.h.el.parentElement)), nulos: num(txt(ui.nulos.h.el.parentElement)),
        selo: document.querySelector('.selo')?.textContent, estado: S.res.get(ui.topico)?.estado };
    });
  }
}
const fim = await pagina.evaluate(() => ({ cls_total: window.__uv.cls, cls_fontes: window.__uv.clsFontes || [], quedas: window.__uv.con?.quedas ?? 0, msgs: window.__uv.msgs, longas: window.__teste.longas, maiorLonga: window.__teste.maiorLonga }));
const fpsTodos = amostras.map((a) => a.fps).filter((x) => x > 0).sort((a, b) => a - b);
const resumo = { url, hash, movel, duracao_s: Math.round((Date.now() - t0) / 1000), cls_acompanhamento: clsAcompanhamento, ...fim, recargas: Math.max(0, recargas - 1), externas, erros: erros.slice(0, 20),
  fps_p5: fpsTodos[Math.floor(fpsTodos.length * 0.05)] ?? null, fps_med: fpsTodos[Math.floor(fpsTodos.length / 2)] ?? null,
  heap_mb: { ini: amostras[0]?.heap, max: Math.max(...amostras.map((a) => a.heap || 0)), fim: amostras.at(-1)?.heap },
  rolagem_horizontal: amostras.some((a) => a.scrollW > a.larg), topicos_lidos: Object.keys(numeros).length };
writeFileSync(`${saida}/numeros.json`, JSON.stringify(numeros, null, 1));
writeFileSync(`${saida}/resumo.json`, JSON.stringify(resumo, null, 1));
writeFileSync(`${saida}/amostras.json`, JSON.stringify(amostras));
log({ resumo });
await navegador.close();
