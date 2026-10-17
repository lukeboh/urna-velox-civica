// Teste do hodômetro (0.5.0): cada dígito gira sempre no mesmo sentido — para a frente nos números comuns (também
// quando o valor diminui) e para trás nos contadores regressivos —, termina no dígito certo e a coluna nunca sai da
// faixa. Amostra a posição de cada coluna a cada quadro (Chromium do Playwright) e "desenrola" os saltos de 10/20
// posições que o hodômetro faz ao trazer a coluna de volta (o mesmo dígito em outra cópia, invisível).
//   cd /mnt/d/git/lukeboh/urna-velox/site && node testes/hodometro.mjs
import { build } from 'esbuild';
import { readFileSync } from 'node:fs';
import { chromium } from 'playwright';

const raiz = new URL('..', import.meta.url).pathname;
const js = (await build({ stdin: { contents: "import { Hodometro } from './src/hodometro.js'; window.Hodometro = Hodometro;", resolveDir: raiz },
  bundle: true, format: 'iife', write: false })).outputFiles[0].text;
const css = readFileSync(`${raiz}src/estilo.css`, 'utf8');

const nav = await chromium.launch();
const pag = await nav.newPage();
await pag.setContent(`<!doctype html><html><head><style>${css}</style></head><body style="font-size:40px"><div id="r"></div></body></html>`);
await pag.addScriptTag({ content: js });

// sequências: [nome, regressivo, inteiros, casas, valores, intervalo entre valores em ms]
const casos = [
  ['sobe devagar', false, 4, 0, [0, 7, 12, 19, 25, 99, 100, 101, 1234], 500],
  ['desce (um % que cai) gira para a frente', false, 3, 2, [45.37, 45.12, 44.98, 30.01, 29.99, 0.5], 500],
  ['regressivo desce', true, 6, 0, [250000, 249321, 180000, 99999, 12345, 9876, 10, 1, 0], 500],
  ['regressivo que sobe (troca de disputa) gira para trás', true, 4, 0, [10, 2000, 1500, 9999], 500],
  ['rajada (versões a cada quadro)', false, 7, 0, Array.from({ length: 300 }, (_, i) => i * 3517), 16],
  ['rajada regressiva', true, 7, 0, Array.from({ length: 300 }, (_, i) => 9999999 - i * 3517), 16],
];

let falhas = 0;
for (const [nome, regressivo, inteiros, casas, valores, dt] of casos) {
  const r = await pag.evaluate(async ({ regressivo, inteiros, casas, valores, dt }) => {
    const raiz = document.getElementById('r');
    raiz.textContent = '';
    const h = new window.Hodometro({ inteiros, casas, regressivo });
    raiz.append(h.el);
    const cols = [...h.el.querySelectorAll('.col')];
    const pos = (c) => { const t = getComputedStyle(c).transform; return t === 'none' ? 0 : -new DOMMatrixReadOnly(t).m42 / (c.offsetHeight / 30); };
    const s = regressivo ? -1 : 1;
    const ult = cols.map(pos), soma = cols.map(() => 0);
    let contra = 0, fora = 0, maior = 0;
    const amostra = () => cols.forEach((c, i) => {
      const p = pos(c);
      if (p < -0.01 || p > 29.01) fora++;
      let dp = p - ult[i];
      // salto de volta da coluna: múltiplo de 10 no sentido contrário, de uma vez
      const k = Math.round(dp / 10);
      if (k !== 0 && Math.abs(dp - 10 * k) < 0.02 && Math.sign(k) === -s) dp -= 10 * k;
      if (s * dp < -0.01) { contra++; maior = Math.max(maior, -s * dp); }
      soma[i] += s * dp;
      ult[i] = p;
    });
    let parar = false;
    const quadro = () => { amostra(); if (!parar) requestAnimationFrame(quadro); };
    requestAnimationFrame(quadro);
    for (const v of valores) { h.definir(v); await new Promise((ok) => setTimeout(ok, dt)); }
    await new Promise((ok) => setTimeout(ok, 1500));
    parar = true;
    amostra();
    // dígito que cada coluna mostra no fim × o texto esperado
    const mostrado = cols.map((c) => ((Math.round(pos(c)) % 10) + 10) % 10).join('');
    const v = valores.at(-1);
    let [int, dec = ''] = v.toFixed(casas).split('.');
    const esperado = (int.padStart(inteiros, '0') + dec);
    const vazios = [...h.el.querySelectorAll('.dg')].map((d) => d.classList.contains('vazio'));
    const visivel = (txt) => [...txt].map((ch, i) => (vazios[i] ? '_' : ch)).join('');
    return { contra, maior, fora, mostrado: visivel(mostrado), esperado: visivel(esperado), texto: h.texto.textContent };
  }, { regressivo, inteiros, casas, valores, dt });
  const ok = r.contra === 0 && r.fora === 0 && r.mostrado === r.esperado;
  if (!ok) falhas++;
  console.log(`${ok ? 'ok  ' : 'FALHA'} ${nome}: contra o sentido ${r.contra} (maior ${r.maior.toFixed(3)}), fora da coluna ${r.fora}, tela ${r.mostrado} esperado ${r.esperado} (${r.texto})`);
}
// enxuto (0.6.13): os dígitos que aparecem ou somem à esquerda não mexem em nada por layout (CLS 0) e, assentado, o
// primeiro dígito visível fica na borda do hodômetro (à esquerda; no centro, `centro: true`, o visível fica centrado)
for (const centro of [false, true]) {
  const r = await pag.evaluate(async (centro) => {
    const raiz = document.getElementById('r');
    raiz.textContent = '';
    raiz.style.cssText = centro ? 'display:grid;place-items:center;width:900px' : '';
    let cls = 0;
    const ob = new PerformanceObserver((l) => { for (const e of l.getEntries()) cls += e.value; });
    ob.observe({ type: 'layout-shift', buffered: false });
    const h = new window.Hodometro({ inteiros: 9, regressivo: false, enxuto: true, centro });
    raiz.append(h.el);
    const erros = [];
    for (const v of [5, 12345, 9, 1234567, 88, 0, 999999999, 7]) {
      h.definir(v);
      await new Promise((ok) => setTimeout(ok, 700));
      const box = h.el.getBoundingClientRect();
      const vis = [...h.el.querySelectorAll('.dg, .sp')].filter((x) => !x.classList.contains('vazio'));
      const a = vis[0].getBoundingClientRect().left, b = vis.at(-1).getBoundingClientRect().right;
      const desvio = centro ? (a + b) / 2 - (box.left + box.right) / 2 : a - box.left;
      if (Math.abs(desvio) > 1) erros.push(`${v}: ${desvio.toFixed(1)} px`);
    }
    await new Promise((ok) => setTimeout(ok, 300));
    ob.disconnect();
    return { cls, erros };
  }, centro);
  const ok = r.cls === 0 && r.erros.length === 0;
  if (!ok) falhas++;
  console.log(`${ok ? 'ok  ' : 'FALHA'} enxuto${centro ? ' centrado' : ''}: CLS ${r.cls}, posição ${r.erros.length ? r.erros.join('; ') : 'certa'}`);
}
await nav.close();
console.log(falhas ? `${falhas} caso(s) com falha` : 'hodômetro: todos os casos ok');
process.exit(falhas ? 1 : 0);
