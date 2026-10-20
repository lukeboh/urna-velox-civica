// Teste do B-012 (dois candidatos com cores quase iguais): em todas as disputas de um ou mais candidatos.json, a
// menor distância em OKLab entre as cores de dois candidatos da mesma disputa, nos dois temas, tem de passar do
// limiar — para que linhas da curva, legenda e lista se distingam. Mostra as piores disputas.
//   node testes/cores_distancia.mjs <candidatos.json> [...] [--min 0.12] [--min-grandes 0.075]
// Limiar --min nas disputas de até 8 candidatos (todas as de 2º turno, quase todas as de Prefeito e Governador) e
// --min-grandes nas maiores (Senado, Presidente com muitos partidos da mesma cor). Saída 1 se alguma ficar abaixo.
import { readFileSync } from 'node:fs';
import { coresDaDisputa } from '../src/cores.js';

const args = process.argv.slice(2);
const opc = (k, d) => { const i = args.indexOf(k); return i >= 0 ? +args[i + 1] : d; };
const MIN = opc('--min', 0.12), MIN_G = opc('--min-grandes', 0.075);
const arquivos = args.filter((a, i) => !a.startsWith('--') && !(i > 0 && args[i - 1].startsWith('--')));

// oklch(L C h) → [L, a, b]
const lab = (s) => { const m = /oklch\(([\d.]+) ([\d.]+) ([\d.]+)\)/.exec(s); const [L, C, h] = [+m[1], +m[2], +m[3]]; return [L, C * Math.cos((h * Math.PI) / 180), C * Math.sin((h * Math.PI) / 180)]; };
const de = (a, b) => Math.hypot(a[0] - b[0], a[1] - b[1], a[2] - b[2]);
const temas = (v) => { const m = /light-dark\((oklch\([^)]*\)), (oklch\([^)]*\))\)/.exec(v); return [lab(m[1]), lab(m[2])]; };

const piores = [];
let total = 0, abaixo = 0;
for (const a of arquivos) {
  const arq = JSON.parse(readFileSync(a, 'utf8'));
  const anda = (o, cam) => {
    if (Array.isArray(o)) {
      if (o.length && o[0] && typeof o[0] === 'object' && 'numero' in o[0] && 'partido' in o[0]) {
        const cands = [...o].sort((x, y) => x.numero - y.numero).map((k) => ({ n: k.numero, partido: k.partido, nome: k.nome_urna || k.nome || '' }));
        if (cands.length < 2) return;
        total++;
        const c = coresDaDisputa(cands.map((k) => ({ n: k.n, partido: k.partido })));
        let pior = null;
        for (let i = 0; i < cands.length; i++) for (let j = i + 1; j < cands.length; j++) {
          const [ci, ei] = temas(c.get(cands[i].n)), [cj, ej] = temas(c.get(cands[j].n));
          const d = Math.min(de(ci, cj), de(ei, ej));
          if (!pior || d < pior.d) pior = { d, a: cands[i], b: cands[j] };
        }
        if (pior.d < (cands.length <= 8 ? MIN : MIN_G)) abaixo++;
        piores.push({ cam, ...pior, n: cands.length });
      } else o.forEach((v, i) => anda(v, `${cam}[${i}]`));
    } else if (o && typeof o === 'object') for (const [k, v] of Object.entries(o)) anda(v, `${cam}.${k}`);
  };
  anda(arq, a.split('/').slice(-3).join('/'));
}
piores.sort((x, y) => x.d - y.d);
for (const p of piores.slice(0, 8)) console.log(`${p.d.toFixed(3)}  ${p.a.n} ${p.a.partido} ${p.a.nome} × ${p.b.n} ${p.b.partido} ${p.b.nome}  (${p.n} candidatos; ${p.cam})`);
const ate8 = piores.filter((p) => p.n <= 8);
console.log(`cores_distancia: ${total} disputas; ${abaixo} com algum par abaixo do limiar (${MIN} até 8 candidatos, ${MIN_G} acima) em OKLab; pior ${piores[0]?.d.toFixed(3)}; pior até 8 candidatos ${ate8[0]?.d.toFixed(3)}`);
process.exit(abaixo ? 1 : 0);
