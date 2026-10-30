// B-082 (lado da página): a curva da apuração tem um relógio só. Depois de um reinício do serviço, uma publicação com a
// hora da noite (sem `totalizado_em`) e as seguintes com o relógio do sistema iam para o mesmo eixo (19:12 da noite e
// 10:50 de agora): a curva ficava espremida na beirada. Com Playwright/Chromium e **nenhum servidor**: a classe
// `Curva` de `src/graficos.js` numa página em branco.
//   cd site && node testes/curva_relogio.mjs
import { chromium } from 'playwright';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const SRC = join(dirname(fileURLToPath(import.meta.url)), '..', 'src');
const falhas = [];
const conferir = (ok, o) => { console.log(`${ok ? 'ok   ' : 'FALHA'} ${o}`); if (!ok) falhas.push(o); };
const nav = await chromium.launch(process.env.UV_CHROMIUM ? { executablePath: process.env.UV_CHROMIUM } : {});
const pg = await nav.newPage();
await pg.route('http://uv.teste/**', (r) => {
  const p = new URL(r.request().url()).pathname;
  if (p === '/') return r.fulfill({ contentType: 'text/html', body: '<!doctype html><body></body>' });
  return r.fulfill({ contentType: 'text/javascript', body: readFileSync(join(SRC, p.slice(1))) });
});
await pg.goto('http://uv.teste/');
const r = await pg.evaluate(async () => {
  const { Curva } = await import('/graficos.js');
  const c = new Curva();
  document.body.append(c.el);
  const noite = 1791141129, sist = 1793354052;
  c.semear([{ t: noite, s: 72, c: new Map([[22, 50]]) }], 'noite');   // GET /api/v1/curva logo depois do reinício
  c.ponto(sist, 73, [[22, 50]], 'sistema');
  c.ponto(sist + 60, 74, [[22, 51]], 'sistema');
  c.ponto(noite + 30, 75, [[22, 52]], 'noite');                        // uma publicação sem totalizado_em no meio
  const a = c.pontos.map((p) => p.t);
  // sem relógio (consulta): como antes
  const d = new Curva();
  d.semear([{ t: noite, s: 1, c: new Map() }]);
  d.ponto(noite + 60, 2, []);
  return { a, d: d.pontos.length };
});
conferir(r.a.length === 2 && r.a.every((t) => t >= 1793354052), `um relógio só na curva: ${r.a.join(', ')}`);
conferir(r.d === 2, `sem relógio (consulta), a curva segue como antes (${r.d} pontos)`);
await nav.close();
console.log(falhas.length ? `\n${falhas.length} falha(s)` : '\ntodos os cenários passaram');
process.exit(falhas.length ? 1 : 0);
