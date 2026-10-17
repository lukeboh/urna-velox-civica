// Ensaio da comparação Urna Velox × TSE (etapa 7), com Playwright/Chromium (sem interface):
//   node testes/comparacao.mjs --url http://127.0.0.1:18080/ --saida /home/dev/uvreal/ensaios/etapa7-X/pagina \
//        --oficial /home/dev/uvreal/oficial --hashes '#/br/c0001,#/sp/c0003' [--abas 10] [--alternar 20] [--limite 3600] [--minimo 0]
// Abre N abas em modos variados — "Ambos", "TSE" e alternando entre os três a cada `--alternar` s —, umas no
// computador (1440×900) e duas no celular (360×780), cada uma numa tela da lista `--hashes` (em rodízio). Acompanha
// até a apuração encerrar e o arquivo do TSE da tela fechar (100 % das seções) em todas; então põe todas em "Ambos",
// espera as animações assentarem e lê **da tela** as duas colunas (Urna Velox e TSE: votos, %, posição) e confere com o
// `-u.json` oficial da tela; guarda o CLS de cada aba (layout-shift sem input recente), o marco "TSE confirmou" e
// confere que nenhuma requisição sai da origem e que não há rolagem horizontal. `--minimo` (s): acompanha pelo menos
// esse tempo mesmo com tudo pronto (contra o site real, com o 3220 já encerrado; `--oficial` é opcional).
import { chromium } from 'playwright';
import { mkdirSync, writeFileSync, appendFileSync, readFileSync } from 'node:fs';

const arg = (k, d) => { const i = process.argv.indexOf(`--${k}`); return i > 0 ? process.argv[i + 1] : d; };
const url = arg('url', 'http://127.0.0.1:18080/');
const saida = arg('saida', '/tmp/uv-comparacao');
const oficial = arg('oficial', '');
const hashes = arg('hashes', '#/br/c0001').split(',');
const nAbas = +arg('abas', '10');
const alternar = +arg('alternar', '20');
const limite = +arg('limite', '3600');
const minimo = +arg('minimo', '0');
mkdirSync(saida, { recursive: true });
const log = (o) => { const l = JSON.stringify({ t: new Date().toISOString(), ...o }); console.log(l); appendFileSync(`${saida}/comparacao.jsonl`, l + '\n'); };

const navegador = await chromium.launch();
const pc = await navegador.newContext({ viewport: { width: 1440, height: 900 } });
const cel = await navegador.newContext({ viewport: { width: 360, height: 780 }, deviceScaleFactor: 2, isMobile: true, hasTouch: true });
const origem = new URL(url).origin;
const externas = [], erros = [];
const abas = [];
for (let i = 0; i < nAbas; i++) {
  const modo = ['ambos', 'tse', 'alternar'][i % 3];
  const movel = i >= nAbas - 2;
  const hash = hashes[i % hashes.length];
  const p = await (movel ? cel : pc).newPage();
  p.on('request', (r) => { const u = r.url(); if (!u.startsWith(origem) && !u.startsWith('data:')) externas.push(u); });
  p.on('pageerror', (e) => { erros.push(`aba ${i}: ${e}`); log({ aba: i, erro_pagina: String(e) }); });
  // respostas de erro: com a URL (o console só diz "Failed to load resource"); /api/v1/replay dá 404 sem
  // --controle-replay no serviço, e a página trata (sem player)
  p.on('response', (r) => { if (r.status() >= 400 && !r.url().endsWith('/api/v1/replay')) erros.push(`aba ${i}: HTTP ${r.status()} ${r.url()}`); });
  p.on('console', (m) => { if (m.type() === 'error' && !m.text().startsWith('Failed to load resource')) { erros.push(`aba ${i}: ${m.text()}`); } });
  const q = modo === 'alternar' ? 'ambos' : modo;
  await p.goto(`${url}?comparar=${q}${hash}`, { waitUntil: 'load' });
  abas.push({ i, modo, movel, hash, p, prox: Date.now() + alternar * 1000 });
}
log({ abas: abas.map((a) => ({ i: a.i, modo: a.modo, movel: a.movel, hash: a.hash })) });
const t0 = Date.now();
const ORDEM = ['uv', 'ambos', 'tse'];
let trocas = 0;
for (;;) {
  await abas[0].p.waitForTimeout(5000);
  const estados = [];
  for (const a of abas) {
    if (a.modo === 'alternar' && Date.now() >= a.prox) {
      a.prox = Date.now() + alternar * 1000;
      const atual = await a.p.evaluate(() => window.__uv.comparar);
      const prox = ORDEM[(ORDEM.indexOf(atual) + 1) % 3];
      await a.p.click(`.comparar button[data-modo="${prox}"]`);
      trocas++;
    }
    estados.push(await a.p.evaluate(() => {
      const S = window.__uv, t = S.tela?.topico, m = t ? S.tse.get(t) : null, r = t ? S.res.get(t) : null;
      return { encerrada: !!S.estado?.encerrada, tela: t, tse: m?.estado ?? null, tse_fin: !!m?.dados?.finalizada, tse_st: m?.dados?.secoes?.totalizadas ?? null, tse_ts: m?.dados?.secoes?.total ?? null,
        uv_ap: r?.secoes?.apuradas ?? null, cls: S.cls, modo: S.comparar, scrollW: document.scrollingElement.scrollWidth, larg: innerWidth };
    }));
  }
  const s = Math.round((Date.now() - t0) / 1000);
  if (s % 30 < 5) log({ s, estados: estados.map((e) => `${e.modo}:${e.uv_ap}/${e.tse_st}/${e.tse_ts}:${e.tse}`) });
  // pronto: apuração encerrada e o arquivo do TSE da tela de cada aba final (totalização finalizada); as abas do
  // rodízio que estão em "Urna Velox" não têm TSE agora: contam pelo que já receberam
  const pronto = estados.every((e) => e.encerrada) && estados.every((e) => e.tse_fin || (e.modo === 'uv' && e.tse_st === e.tse_ts));
  if (pronto && s >= minimo) break;
  if (s > limite) { log({ aviso: 'limite de tempo' }); break; }
}
// todas em "Ambos" (clique: não conta no CLS), animações assentadas, e a leitura da tela
const resultado = [];
for (const a of abas) {
  const cls_antes = await a.p.evaluate(() => window.__uv.cls);
  if (await a.p.evaluate(() => window.__uv.comparar) !== 'ambos') await a.p.click('.comparar button[data-modo="ambos"]');
}
await abas[0].p.waitForTimeout(5000);
for (const a of abas) {
  const tela = await a.p.evaluate(() => {
    const txt = (e) => e?.querySelector('.sr')?.textContent ?? null;
    const num = (s) => (s == null || s === '—' ? null : Number(s.replace(/\./g, '').replace(',', '.').replace(/[^\d.]/g, '')));
    const S = window.__uv, ui = S.ui, t = ui?.topico;
    const cand = [...document.querySelectorAll('#painel li.cand')].map((li) => ({
      n: +li.dataset.n, pos: +getComputedStyle(li).getPropertyValue('--pos') + 1,
      uv_v: num(txt(li.querySelector('.qtd'))), uv_pct: num(txt(li.querySelector('.pct'))),
      tse_v: num(txt(li.querySelector('.tse-qtd'))), tse_pct: num(txt(li.querySelector('.tse-pct'))),
      tse_pos: parseInt(li.querySelector('.pos-tse')?.textContent || '0', 10) || null,
      dif: num(txt(li.querySelector('.dif-val'))), diverge: li.classList.contains('diverge') }));
    return { topico: t, arquivo: S.tse.get(t)?.arquivo, cand, fontes: [ui.fa.textContent, ui.fb.textContent, ui.fe.textContent], cls: S.cls, clsFontes: S.clsFontes || [],
      confirmados: [...S.confirmados.entries()], feed: [...document.querySelectorAll('#trilho .item')].map((x) => x.textContent).filter((x) => x.includes('TSE')).slice(0, 5),
      scrollW: document.scrollingElement.scrollWidth, larg: innerWidth };
  });
  await a.p.screenshot({ path: `${saida}/aba_${a.i}_${a.movel ? 'celular' : 'pc'}.png` });
  // conferência com o oficial da tela
  const dif = [];
  if (oficial && tela.arquivo) {
    const o = JSON.parse(readFileSync(`${oficial}/${tela.arquivo}`, 'utf8'));
    const of = new Map();
    const visitar = (x) => { if (Array.isArray(x)) x.forEach(visitar); else if (x && typeof x === 'object') { if (Array.isArray(x.cand)) for (const c of x.cand) of.set(+c.n, { v: +c.vap, seq: +c.seq }); Object.values(x).forEach(visitar); } };
    visitar(o.carg);
    // posição oficial = ordem pelos votos do arquivo (o seq não é a ordem: no 3220 é a nacional); empates não se conferem
    const ord = [...of.entries()].sort((a, b) => b[1].v - a[1].v);
    ord.forEach(([n, k], i) => { k.pos = i + 1; k.empate = ord.some(([m, j]) => m !== n && j.v === k.v); });
    for (const c of tela.cand) {
      const k = of.get(c.n);
      if (!k) { dif.push(`${c.n}: fora do oficial`); continue; }
      if (c.uv_v !== k.v) dif.push(`${c.n}: Urna Velox ${c.uv_v} × oficial ${k.v}`);
      if (c.tse_v !== k.v) dif.push(`${c.n}: TSE ${c.tse_v} × oficial ${k.v}`);
      if (!k.empate && c.pos !== k.pos) dif.push(`${c.n}: posição Urna Velox ${c.pos} × oficial ${k.pos}`);
      if (!k.empate && c.tse_pos !== k.pos) dif.push(`${c.n}: posição TSE ${c.tse_pos} × oficial ${k.pos}`);
      if (c.dif !== 0) dif.push(`${c.n}: diferença ${c.dif}`);
    }
  }
  resultado.push({ aba: a.i, modo: a.modo, movel: a.movel, hash: a.hash, ...tela, divergencias: dif, rolagem_horizontal: tela.scrollW > tela.larg });
}
const resumo = {
  url, abas: resultado.length, trocas, duracao_s: Math.round((Date.now() - t0) / 1000),
  conferidas: resultado.filter((r) => r.arquivo).length, identicas: resultado.filter((r) => r.arquivo && r.divergencias.length === 0).length,
  cls_max: Math.max(...resultado.map((r) => r.cls)), rolagem_horizontal: resultado.some((r) => r.rolagem_horizontal),
  confirmados: resultado.flatMap((r) => r.confirmados.map(([t, c]) => ({ aba: r.aba, t, ...c }))), externas, erros: erros.slice(0, 20),
};
writeFileSync(`${saida}/abas.json`, JSON.stringify(resultado, null, 1));
writeFileSync(`${saida}/resumo.json`, JSON.stringify(resumo, null, 1));
log({ resumo });
await navegador.close();
