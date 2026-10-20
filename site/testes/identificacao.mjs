// Teste da M-001 (identificação da eleição sempre à vista): abre a página numa disputa durante um replay, a 1366 e a
// 390 px, e confere que
//   - o cartão mostra a eleição, o turno marcado, o local e o cargo da tela;
//   - rolado o painel, o cartão continua à vista, preso no alto (topo ≤ 12 px) numa linha só, com o turno e o local;
//   - o botão recolhe o cartão, e a escolha sobrevive a um recarregamento; o botão reabre;
//   - nada pula: CLS do carregamento ao fim < 0,01 (o lugar do cartão é reservado no index.html).
// Grava capturas (topo, rolado, recolhido) em --saida.
//   node testes/identificacao.mjs --url http://127.0.0.1:18080/ --hash '#/sp71072/c0011' --saida /tmp/x [--espera 30]
import { chromium } from 'playwright';
import { mkdirSync, writeFileSync } from 'node:fs';

const arg = (k, d) => { const i = process.argv.indexOf(`--${k}`); return i > 0 ? process.argv[i + 1] : d; };
const url = arg('url', 'http://127.0.0.1:18080/');
const hash = arg('hash', '#/br/c0001');
const saida = arg('saida', '/tmp/uv-identificacao');
const espera = +arg('espera', '30');
mkdirSync(saida, { recursive: true });

const CLS = () => {
  window.__cls = 0;
  new PerformanceObserver((l) => { for (const e of l.getEntries()) if (!e.hadRecentInput) window.__cls += e.value; })
    .observe({ type: 'layout-shift', buffered: true });
};
const ler = (pag) => pag.evaluate(() => {
  // o que está à vista: o cartão no topo; rolado, a barra de uma linha presa no alto
  const preso = document.querySelector('#ident').classList.contains('preso');
  const c = document.querySelector(preso ? '#ident-barra' : '#ident-cartao');
  const r = c.getBoundingClientRect();
  const vis = (s) => {
    const e = c.querySelector(s);
    if (!e || !e.getClientRects().length || getComputedStyle(e).visibility === 'hidden' || +getComputedStyle(c).opacity < 0.9) return false;
    const q = e.getBoundingClientRect();
    return q.width > 0 && q.top >= 0 && q.bottom <= innerHeight;
  };
  const txt = (s) => c.querySelector(s)?.textContent ?? '';
  return {
    top: Math.round(r.top), alt: Math.round(r.height), larg: Math.round(r.width), fixo: preso,
    recolhida: document.documentElement.classList.contains('ident-recolhida'),
    eleicao: txt('.id-eleicao'), tipo: document.querySelector('#ident-cartao .id-tipo').textContent,
    turno: [...c.querySelectorAll('.id-seg i.sel')].map((i) => i.textContent).join(),
    local: txt('.id-local'), cargo: txt('.id-cargo'),
    vis: { eleicao: vis('.id-eleicao'), turno: vis('.id-seg'), local: vis('.id-local'), cargo: vis('.id-cargo') },
    titulo: document.title, cls: +(window.__cls || 0).toFixed(4),
  };
});

const nav = await chromium.launch();
// deixa a apuração andar um pouco antes (painel com resultados, página comprida o bastante para rolar)
const p0 = await nav.newPage();
await p0.goto(url + hash);
await p0.waitForTimeout(espera * 1000);
await p0.close();
const erros = [];
const rel = {};
for (const [w, h, movel] of [[1366, 900, false], [390, 844, true]]) {
  const ctx = await nav.newContext({ viewport: { width: w, height: h }, deviceScaleFactor: movel ? 2 : 1, isMobile: movel, hasTouch: movel });
  await ctx.addInitScript(CLS);
  const pag = await ctx.newPage();
  const e = (m) => erros.push(`${w}px: ${m}`);
  await pag.goto(url + hash);
  await pag.waitForSelector('.corrida', { timeout: 30000 });
  await pag.waitForTimeout(3000);
  const topo = await ler(pag);
  await pag.screenshot({ path: `${saida}/ident-${w}-1-topo.png` });
  if (!/^Eleições 20\d\d$/.test(topo.eleicao)) e(`eleição "${topo.eleicao}"`);
  if (!['Gerais', 'Municipais'].includes(topo.tipo)) e(`tipo "${topo.tipo}"`);
  if (!['1º', '2º'].includes(topo.turno)) e(`turno marcado "${topo.turno}"`);
  if (!topo.local || !topo.cargo) e(`local "${topo.local}" / cargo "${topo.cargo}" vazios`);
  if (!Object.values(topo.vis).every(Boolean)) e(`no topo, algo escondido: ${JSON.stringify(topo.vis)}`);
  if (!topo.titulo.includes(topo.local)) e(`título "${topo.titulo}" sem o local`);
  const alto = await pag.evaluate(() => document.documentElement.scrollHeight - innerHeight);
  if (alto < 300) e(`página curta demais para rolar (${alto} px)`);
  await pag.evaluate(() => window.scrollTo(0, Math.min(900, document.documentElement.scrollHeight)));
  await pag.waitForTimeout(800);
  const rol = await ler(pag);
  await pag.screenshot({ path: `${saida}/ident-${w}-2-rolado.png` });
  if (!rol.fixo || rol.top < 0 || rol.top > 12) e(`rolado: cartão não ficou preso no alto (top ${rol.top}, fixo ${rol.fixo})`);
  if (rol.alt > 48) e(`rolado: cartão com ${rol.alt} px de altura (uma linha: ≤ 48)`);
  if (!rol.vis.turno || !rol.vis.local) e(`rolado: turno/local escondidos ${JSON.stringify(rol.vis)}`);
  const recolher = () => pag.locator('#ident .id-recolher:visible').first().click();
  await recolher();
  await pag.waitForTimeout(400);
  const rec = await ler(pag);
  await pag.screenshot({ path: `${saida}/ident-${w}-3-recolhido.png` });
  if (!rec.recolhida || rec.larg > 48 || rec.vis.local) e(`recolher: ${JSON.stringify(rec)}`);
  await pag.reload();
  await pag.waitForSelector('.corrida', { timeout: 30000 });
  await pag.waitForTimeout(1500);
  const rec2 = await ler(pag);
  if (!rec2.recolhida || rec2.vis.local) e('recolhido não sobreviveu ao recarregamento');
  if (rec2.cls >= 0.01) e(`CLS recolhido ${rec2.cls}`);
  await recolher();
  await pag.waitForTimeout(400);
  const abr = await ler(pag);
  if (abr.recolhida || !abr.vis.local) e('o botão não reabriu o cartão');
  if (topo.cls >= 0.01 || rol.cls >= 0.01) e(`CLS ${topo.cls} no topo, ${rol.cls} rolado`);
  rel[w] = { topo, rolado: rol, recolhido: rec, recarregado: rec2, reaberto: abr };
  console.log(`${w}px: ${topo.eleicao} · ${topo.tipo} · ${topo.turno} turno · ${topo.local} · ${topo.cargo} · rolado top ${rol.top} alt ${rol.alt} · CLS ${topo.cls}/${rol.cls}/${rec2.cls}`);
  await ctx.close();
}
await nav.close();
writeFileSync(`${saida}/identificacao.json`, JSON.stringify(rel, null, 1));
if (erros.length) { console.error('FALHOU\n  - ' + erros.join('\n  - ')); process.exit(1); }
console.log('ok');
