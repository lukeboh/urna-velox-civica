// Teste da M-003 (horários do Urna Velox no relógio do sistema, separados dos do TSE): abre a página numa disputa
// durante um replay acelerado, a 1366 e a 390 px, e confere que
//   - o cartão da apuração diz "Totalizado pelo Urna Velox às" com a hora do relógio do sistema (a de agora, não a da
//     noite da eleição) e, embaixo, "último BU no TSE" com a hora e a data da eleição original;
//   - a âncora chegou (`relogio_sistema`) e a previsão vem no relógio do sistema;
//   - os pontos da curva estão no relógio do sistema;
//   - nada pula: CLS < 0,01.
// Grava capturas em --saida.
//   node testes/horarios_pagina.mjs --url http://127.0.0.1:18080/ --hash '#/ac/c0003' --saida /tmp/x [--espera 40]
import { chromium } from 'playwright';
import { mkdirSync, writeFileSync } from 'node:fs';

const arg = (k, d) => { const i = process.argv.indexOf(`--${k}`); return i > 0 ? process.argv[i + 1] : d; };
const url = arg('url', 'http://127.0.0.1:18080/');
const hash = arg('hash', '#/br/c0001');
const saida = arg('saida', '/tmp/uv-horarios');
const espera = +arg('espera', '40');
mkdirSync(saida, { recursive: true });

// agora, na escala de segundos() da página (horário civil de Brasília lido como UTC)
const agoraBr = () => Date.now() / 1000 - 3 * 3600;
const seg = (hms) => { const m = /(\d{2}):(\d{2}):(\d{2})/.exec(hms || ''); return m ? +m[1] * 3600 + +m[2] * 60 + +m[3] : null; };
const doDia = (s) => ((s % 86400) + 86400) % 86400;

const falhas = [];
const navegador = await chromium.launch();
for (const [larg, alt, nome] of [[1366, 900, 'desktop'], [390, 844, 'celular']]) {
  const ctx = await navegador.newContext({ viewport: { width: larg, height: alt }, deviceScaleFactor: 1, ...(larg < 500 ? { isMobile: true, hasTouch: true } : {}) });
  const pag = await ctx.newPage();
  await pag.addInitScript(() => {
    window.__cls = 0;
    new PerformanceObserver((l) => { for (const e of l.getEntries()) if (!e.hadRecentInput) window.__cls += e.value; }).observe({ type: 'layout-shift', buffered: true });
  });
  await pag.goto(url + hash);
  await pag.waitForTimeout(espera * 1000);
  const v = await pag.evaluate(() => {
    const S = window.__uv;
    const num = document.querySelector('.apuracao .numeros > div:nth-child(2)');
    const pts = S.ui?.curva?.pontos || [];
    const p = S.prev.get(S.ui?.topico) || null;
    return {
      rot: num?.querySelector('.rot')?.textContent || '', val: num?.querySelector('.val')?.textContent.replace(/\s+/g, '') || '',
      tse: num?.querySelector('.hora-tse')?.textContent || '', anc: S.anc, pts: pts.length, t0: pts[0]?.t ?? null, t1: pts[pts.length - 1]?.t ?? null,
      prev: p ? { relogio: p.relogio, instante: p.instante } : null, cls: window.__cls,
      // selo e pílulas do "matematicamente" (… às hh:mm:ss)
      selos: [S.ui?.selo?.textContent || '', ...[...document.querySelectorAll('.corrida .pilula')].map((x) => x.textContent)].filter((x) => / às \d{2}:\d{2}:\d{2}/.test(x)),
    };
  });
  const agora = agoraBr();
  const hv = seg(v.val);
  const linha = `${nome}: "${v.rot}" ${v.val} · "${v.tse}" · âncora ${JSON.stringify(v.anc)} · curva ${v.pts} pontos · previsão ${JSON.stringify(v.prev)} · selos ${JSON.stringify(v.selos)} · CLS ${v.cls.toFixed(4)}`;
  console.log(linha);
  const f = [];
  if (v.rot !== 'Totalizado pelo Urna Velox às') f.push(`rótulo "${v.rot}"`);
  if (hv == null || Math.abs(hv - doDia(agora)) > 600) f.push(`hora do Urna Velox ${v.val} longe do relógio do sistema (${new Date(agora * 1000).toISOString().slice(11, 19)})`);
  if (!/^último BU no TSE: \d{2}:\d{2}:\d{2} de \d{2}\/\d{2}$/.test(v.tse)) f.push(`linha do TSE "${v.tse}"`);
  if (!v.anc || !(v.anc.x > 1)) f.push('sem âncora do simulado');
  if (v.pts < 5 || v.t1 == null || Math.abs(v.t1 - agora) > 600) f.push(`curva fora do relógio do sistema (${v.pts} pontos, último ${v.t1})`);
  if (v.prev && v.prev.relogio !== 'sistema') f.push(`previsão no relógio "${v.prev.relogio}"`);
  if (v.cls >= 0.01) f.push(`CLS ${v.cls.toFixed(4)}`);
  for (const s of v.selos) {
    const h = seg(/ às (\d{2}:\d{2}:\d{2})/.exec(s)[1]);
    if (Math.abs(h - doDia(agora)) > 600) f.push(`"${s}" fora do relógio do sistema`);
  }
  for (const x of f) falhas.push(`${nome}: ${x}`);
  await pag.screenshot({ path: `${saida}/horarios-${nome}.png` });
  await pag.locator('.apuracao').screenshot({ path: `${saida}/apuracao-${nome}.png` }).catch(() => {});
  writeFileSync(`${saida}/horarios-${nome}.json`, JSON.stringify(v, null, 1));
  await ctx.close();
}
await navegador.close();
if (falhas.length) { console.log('FALHOU\n  - ' + falhas.join('\n  - ')); process.exit(1); }
console.log('ok');
