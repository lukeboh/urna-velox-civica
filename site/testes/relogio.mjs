// M-003: relógio do sistema × relógio do TSE na página (src/relogio.js e o cabeçalho das fontes da comparação).
//   node testes/relogio.mjs
// Simulado que começa hoje às 07:00:00 reproduzindo a noite de 06/10/2024 a partir das 17:00:00 a 500×, e a noite
// real (aceleração 1, relógios com 2 s de diferença).
import assert from 'node:assert/strict';
import { ancora, comparar, distancia, horaDe, segundosF, tseNoSistema } from '../src/relogio.js';

const casos = [];
const caso = (nome, f) => casos.push([nome, f]);

const sim = ancora({ noite0: '2024-10-06T17:00:00-03:00', sistema0: '2026-10-18T07:00:00.000-03:00', acelerar: 500 });
const real = ancora({ noite0: '2026-10-25T17:00:00-03:00', sistema0: '2026-10-25T17:00:02.000-03:00', acelerar: 1 });

caso('segundos com fração', () => {
  assert.ok(Math.abs(segundosF('2026-10-18T20:41:03.512-03:00') - segundosF('2026-10-18T20:41:03-03:00') - 0.512) < 1e-6);
  assert.equal(segundosF('—'), null);
});
caso('sem âncora (servidor antigo, antes do primeiro horário do site)', () => {
  assert.equal(ancora({ noite0: null, sistema0: null, acelerar: 500 }), null);
  assert.equal(tseNoSistema(null, 1000), 1000);
});
caso('TSE levado ao relógio do sistema no simulado', () => {
  // 10 h da noite do TSE (17:00 → 03:00) = 72 s do sistema a 500×
  const t = segundosF('2024-10-07T03:00:00-03:00');
  assert.equal(horaDe(tseNoSistema(sim, t)), '07:01:12');
});
caso('distância entre as apurações no simulado', () => {
  // TSE confirmou às 21:00:00 da noite (4 h depois do início = 28,8 s do sistema); Urna Velox às 07:00:20 de hoje
  const c = comparar(sim, segundosF('2026-10-18T07:00:20.000-03:00'), segundosF('2024-10-06T21:00:00-03:00'));
  assert.equal(horaDe(c.tse), '07:00:28');
  assert.ok(Math.abs(c.s - 8.8) < 1e-6, String(c.s));
  assert.equal(c.texto, '9 s antes');
  assert.ok(Math.abs(c.noite - 4400) < 1e-3, String(c.noite));
  assert.equal(distancia(c.noite), '1 h 13 min antes');
});
caso('distância na noite real só desconta a diferença dos relógios', () => {
  const c = comparar(real, segundosF('2026-10-25T20:10:00.000-03:00'), segundosF('2026-10-25T20:25:00-03:00'));
  assert.equal(c.texto, '15 min antes');
  assert.equal(distancia(-30), '30 s depois');
  assert.equal(distancia(0.2), 'no mesmo instante');
});

let falhas = 0;
for (const [nome, f] of casos) {
  try { f(); console.log(`ok ${nome}`); } catch (e) { falhas++; console.log(`FALHOU ${nome}: ${e.message}`); }
}
console.log(falhas ? `relogio: ${falhas} caso(s) falharam` : 'relogio: ok');
process.exit(falhas ? 1 : 0);
