// Confere as cores por partido em todas as disputas de um candidatos.json:
//   node testes/cores.mjs /home/dev/uvreal/etapa5/preparo-2026-1t/candidatos.json
// Falha se algum partido do arquivo não tiver cor (nem pela federação) ou se duas cores de uma mesma disputa
// ficarem iguais. Imprime a disputa de presidente (ou a primeira) para inspeção.
import { readFileSync } from 'node:fs';
import { PARTIDOS, coresDaDisputa, normalizarSigla } from '../src/cores.js';

const arq = JSON.parse(readFileSync(process.argv[2], 'utf8'));
const disputas = [];
const anda = (o, cam) => {
  if (Array.isArray(o)) {
    if (o.length && o[0] && typeof o[0] === 'object' && 'numero' in o[0] && 'partido' in o[0]) disputas.push({ cam, cands: o.map((k) => ({ n: k.numero, partido: k.partido })) });
    else o.forEach((v, i) => anda(v, `${cam}[${i}]`));
  } else if (o && typeof o === 'object') for (const [k, v] of Object.entries(o)) anda(v, `${cam}.${k}`);
};
anda(arq, '');
const sem = new Set();
let iguais = 0;
for (const d of disputas) {
  const ordem = [...d.cands].sort((a, b) => a.n - b.n).map((k) => ({ n: k.n, partido: k.partido }));
  for (const k of ordem) if (!PARTIDOS[normalizarSigla(k.partido)]) sem.add(k.partido);
  const c = coresDaDisputa(ordem);
  if (new Set(c.values()).size !== c.size) { iguais++; if (iguais <= 3) console.log('cores repetidas em', d.cam); }
}
const exemplo = disputas.find((d) => d.cands.some((k) => k.n === 13) && d.cands.some((k) => k.n === 22)) || disputas[0];
const ord = [...exemplo.cands].sort((a, b) => a.n - b.n);
const c = coresDaDisputa(ord.map((k) => ({ n: k.n, partido: k.partido })));
for (const k of ord) console.log(String(k.n).padStart(3), (k.partido || '').padEnd(14), c.get(k.n));
console.log(`${disputas.length} disputas; partidos sem cor: ${[...sem].join(', ') || 'nenhum'}; disputas com cor repetida: ${iguais}`);
process.exit(sem.size || iguais ? 1 : 0);
