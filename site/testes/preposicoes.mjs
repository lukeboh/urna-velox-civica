// Teste do B-042 (docs/BUGS.md): a preposição antes do nome do lugar nos textos da página ("Governador na Bahia",
// "Primeira seção totalizada no Rio de Janeiro", "se decide no Brasil"), nunca "em Bahia" / "em Rio de Janeiro".
// Sem navegador: importa src/textos.js direto (esbuild).
//   cd site && node testes/preposicoes.mjs
import { build } from 'esbuild';

const raiz = new URL('..', import.meta.url).pathname;
const js = (await build({ stdin: { contents: "export { Nomes, itemMarco } from './src/textos.js';", resolveDir: raiz }, bundle: true, format: 'esm', write: false })).outputFiles[0].text;
const { Nomes, itemMarco } = await import(`data:text/javascript;base64,${Buffer.from(js).toString('base64')}`);

const UFS = { ac: 'ACRE', al: 'ALAGOAS', am: 'AMAZONAS', ap: 'AMAPÁ', ba: 'BAHIA', ce: 'CEARÁ', df: 'DISTRITO FEDERAL', es: 'ESPÍRITO SANTO',
  go: 'GOIÁS', ma: 'MARANHÃO', mg: 'MINAS GERAIS', ms: 'MATO GROSSO DO SUL', mt: 'MATO GROSSO', pa: 'PARÁ', pb: 'PARAÍBA', pe: 'PERNAMBUCO',
  pi: 'PIAUÍ', pr: 'PARANÁ', rj: 'RIO DE JANEIRO', rn: 'RIO GRANDE DO NORTE', rs: 'RIO GRANDE DO SUL', ro: 'RONDÔNIA', rr: 'RORAIMA',
  sc: 'SANTA CATARINA', se: 'SERGIPE', sp: 'SÃO PAULO', to: 'TOCANTINS', zz: 'EXTERIOR' };
const CERTO = { ac: 'no Acre', al: 'em Alagoas', am: 'no Amazonas', ap: 'no Amapá', ba: 'na Bahia', ce: 'no Ceará', df: 'no Distrito Federal',
  es: 'no Espírito Santo', go: 'em Goiás', ma: 'no Maranhão', mg: 'em Minas Gerais', ms: 'em Mato Grosso do Sul', mt: 'em Mato Grosso',
  pa: 'no Pará', pb: 'na Paraíba', pe: 'em Pernambuco', pi: 'no Piauí', pr: 'no Paraná', rj: 'no Rio de Janeiro', rn: 'no Rio Grande do Norte',
  rs: 'no Rio Grande do Sul', ro: 'em Rondônia', rr: 'em Roraima', sc: 'em Santa Catarina', se: 'em Sergipe', sp: 'em São Paulo',
  to: 'no Tocantins', zz: 'no Exterior' };
const N = new Nomes({ ufs: UFS, municipios: { ac01120: 'BUJARI' }, eleicoes: [] });
const falhas = [];
const conferir = (ok, o) => { if (!ok) { console.log(`FALHA ${o}`); falhas.push(o); } };

for (const [u, certo] of Object.entries(CERTO)) {
  const d = N.disputa(u, 3);
  conferir(d === `Governador ${certo}`, `disputa(${u}): "${d}" (esperado "Governador ${certo}")`);
  const it = itemMarco(N, { classe: 'primeira_secao', abrangencia: u, cargo: null, seq: 1, horario: '2026-10-04 17:00:01', uf: u, municipio: 1120, municipio_nome: 'BUJARI', zona: 1, secao: 2 });
  conferir(it.texto.startsWith(`Primeira seção totalizada ${certo}:`), `primeira seção (${u}): "${it.texto}"`);
}
conferir(N.disputa('ac01120', 11) === 'Prefeito de Bujari (AC)', `disputa municipal: "${N.disputa('ac01120', 11)}"`);
conferir(typeof N.em === 'function' && N.em('br') === 'no Brasil' && N.em('ac01120') === 'em Bujari (AC)', `em(): "${N.em?.('br')}", "${N.em?.('ac01120')}"`);

console.log(falhas.length ? `preposicoes: ${falhas.length} falha(s)` : 'preposicoes: tudo certo');
process.exit(falhas.length ? 1 : 0);
