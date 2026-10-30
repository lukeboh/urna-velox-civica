// Teste da identificação da eleição sempre à vista (M-001; desde a M-025, 30/10/2026, a barra da eleição `#eleicao`, que
// substituiu o cartão "C" e o botão de recolher): abre a página numa disputa durante um replay, a 1366 e a 390 px, e
// confere que
//   - a barra mostra a eleição ("Eleições Gerais 2026", "Eleições Municipais 2024"…), o turno, a data do pleito, o
//     selo do modo, o local e o cargo da tela;
//   - rolado o painel, a barra continua à vista, presa no alto (topo ≤ 12 px), inteira;
//   - nada pula: CLS do carregamento ao fim < 0,01.
// Grava capturas (topo, rolado) em --saida. Sem servidor, o mesmo e mais: `node testes/cabecalho.mjs`.
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
  const c = document.querySelector('#eleicao .el-cartao');
  const r = c.getBoundingClientRect();
  const vis = (s) => {
    const e = c.querySelector(s);
    if (!e || !e.getClientRects().length || getComputedStyle(e).visibility === 'hidden') return false;
    const q = e.getBoundingClientRect();
    return q.width > 0 && q.top >= 0 && q.bottom <= innerHeight && e.scrollWidth <= e.clientWidth + 1;
  };
  const txt = (s) => c.querySelector(s)?.textContent.trim() ?? '';
  return {
    top: Math.round(r.top), alt: Math.round(r.height), presa: document.querySelector('#eleicao').classList.contains('presa'),
    eleicao: txt('.el-nome'), turno: txt('.el-turno'), data: txt('.el-dd'), selo: txt('#conexao .txt'),
    local: txt('.el-local-txt'), cargo: txt('#cargos [aria-selected="true"]'),
    vis: { eleicao: vis('.el-nome'), data: vis('.el-sub'), local: vis('.el-local-txt'), cargo: vis('#cargos [aria-selected="true"]'), selo: vis('#conexao') },
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
  if (await pag.isVisible('#folha.aberta')) { await pag.click('#folha-fechar'); await pag.waitForTimeout(400); }
  const topo = await ler(pag);
  await pag.screenshot({ path: `${saida}/ident-${w}-1-topo.png` });
  if (!/^Elei(ção|ções) .*20\d\d|suplementar/i.test(topo.eleicao)) e(`eleição "${topo.eleicao}"`);
  if (!/^[12]º turno$/.test(topo.turno)) e(`turno "${topo.turno}"`);
  if (!/^\d\d\/\d\d\/20\d\d$/.test(topo.data)) e(`data do pleito "${topo.data}"`);
  if (!topo.local || !topo.cargo || !topo.selo) e(`local "${topo.local}" / cargo "${topo.cargo}" / selo "${topo.selo}" vazios`);
  if (!Object.values(topo.vis).every(Boolean)) e(`no topo, algo escondido ou cortado: ${JSON.stringify(topo.vis)}`);
  if (!topo.titulo.includes(topo.local)) e(`título "${topo.titulo}" sem o local`);
  const alto = await pag.evaluate(() => document.documentElement.scrollHeight - innerHeight);
  if (alto < 300) e(`página curta demais para rolar (${alto} px)`);
  await pag.evaluate(() => window.scrollTo(0, Math.min(900, document.documentElement.scrollHeight)));
  await pag.waitForTimeout(800);
  const rol = await ler(pag);
  await pag.screenshot({ path: `${saida}/ident-${w}-2-rolado.png` });
  if (!rol.presa || rol.top < 0 || rol.top > 12) e(`rolado: a barra não ficou presa no alto (top ${rol.top}, presa ${rol.presa})`);
  if (rol.alt > 112) e(`rolado: barra com ${rol.alt} px de altura (≤ 112)`);
  if (!Object.values(rol.vis).every(Boolean)) e(`rolado: algo escondido ou cortado ${JSON.stringify(rol.vis)}`);
  if (topo.cls >= 0.01 || rol.cls >= 0.01) e(`CLS ${topo.cls} no topo, ${rol.cls} rolado`);
  rel[w] = { topo, rolado: rol };
  console.log(`${w}px: ${topo.eleicao} · ${topo.turno} · ${topo.data} · ${topo.selo} · ${topo.local} · ${topo.cargo} · rolado top ${rol.top} alt ${rol.alt} · CLS ${topo.cls}/${rol.cls}`);
  await ctx.close();
}
await nav.close();
writeFileSync(`${saida}/identificacao.json`, JSON.stringify(rel, null, 1));
if (erros.length) { console.error('FALHOU\n  - ' + erros.join('\n  - ')); process.exit(1); }
console.log('ok');
