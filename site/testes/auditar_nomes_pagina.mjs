// Auditoria dos nomes na página, contra o site no ar (ou UV_URL): para cada pleito com série (ou os passados na linha de
// comando) e para a página ao vivo, todo município do mapa das seções (o "Último BU") e todo candidato da tela tem nome?
// Sem nome a página mostra o código ("Sp71072") ou o número do candidato. Complementa o
// `aquisicao/scripts/auditar_nomes.py`, que confere os arquivos da série.
//   cd site && node testes/auditar_nomes_pagina.mjs [ao-vivo] [pleitos…]
import { chromium } from 'playwright';

const URL0 = process.env.UV_URL || 'https://urnavelox.com.br';
const DADOS = process.env.UV_DADOS || 'https://dados.urnavelox.com.br/consulta';
let alvos = process.argv.slice(2);
if (!alvos.length) {
  const ind = await fetch(`${DADOS}/indice.json`).then((r) => r.json());
  alvos = ['ao-vivo', ...ind.pleitos.map((p) => String(p.pleito))];
}
const nav = await chromium.launch();
let ruins = 0;
for (const a of alvos) {
  const pg = await nav.newPage({ viewport: { width: 1366, height: 900 } });
  await pg.goto(a === 'ao-vivo' ? `${URL0}/` : `${URL0}/#/p/${a}`);
  // o mapa só carrega à vista (IntersectionObserver): rola até ele
  await pg.waitForSelector('section.mapa', { timeout: 30000 }).then((e) => e.scrollIntoViewIfNeeded()).catch(() => {});
  const ok = await pg.waitForFunction(() => window.__uv?.mapa?.carregado && [...(window.__uv.res || [])].some(([, r]) => r?.cand), null, { timeout: 60000 })
    .then(() => true).catch(() => false);
  await pg.waitForTimeout(1500);
  const r = await pg.evaluate(() => {
    const S = window.__uv, m = S.mapa, N = S.N;
    const semMun = new Set();
    if (m?.carregado) for (let i = 0; i < m.munCod.length; i++) {
      const cod = `${m.munUf[i]}${String(m.munCod[i]).padStart(5, '0')}`;
      if (m.munUf[i] === 'zz') continue;
      if (!N.n.municipios?.[cod]) semMun.add(cod);
    }
    const semCand = [];
    for (const [t, res] of S.res) {
      if (!res?.cand) continue;
      for (const k of res.cand) { const c = N.candidato(res.circ || res.abr, res.cargo, k.n); if (!c?.nome || /^\d+$/.test(String(c.nome))) semCand.push(`${t}:${k.n}`); }
    }
    const txt = document.body.innerText;
    const codigos = [...new Set(txt.match(/\b[A-Za-z]{2}\d{5}\b/g) || [])];
    return { tela: S.tela?.topico, pleito: S.consulta?.pleito ?? S.estado?.pleito, municipiosMapa: m?.munCod?.length ?? 0, semMun: [...semMun], semCand, codigos };
  });
  const prob = r.semMun.length + r.semCand.length + r.codigos.length;
  if (prob || !ok) ruins++;
  console.log(`${a.padStart(7)} (pleito ${r.pleito}, ${r.tela}): ${ok ? '' : 'NÃO ABRIU · '}municípios do mapa ${r.municipiosMapa}, sem nome ${r.semMun.length}${r.semMun.length ? ` (${r.semMun.slice(0, 8).join(' ')}…)` : ''}`
    + ` · candidatos sem nome ${r.semCand.length}${r.semCand.length ? ` (${r.semCand.slice(0, 6).join(' ')})` : ''} · códigos na tela ${r.codigos.length}${r.codigos.length ? ` (${r.codigos.slice(0, 6).join(' ')})` : ''}${prob || !ok ? '' : '  ok'}`);
  await pg.close();
}
await nav.close();
console.log(ruins ? `\n${ruins} com problema` : '\ntodos ok');
process.exit(ruins ? 1 : 0);
