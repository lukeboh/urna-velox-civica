// Teste do B-008 (página vira download): abre uma URL como um visitante, no Chromium e no Firefox do Playwright, e
// exige que a navegação mostre uma página HTML — nunca um download — e, se pedido, que o corpo contenha um texto.
//   node testes/indisponivel.mjs <url> [texto esperado no corpo]
// Saída 0 = ok; 1 = falhou (o motivo vai para a saída de erro). Usado por deploy/oci/testes/pagina_indisponivel.sh.
// O Firefox precisa estar instalado (`npx playwright install firefox`); sem ele, o teste diz e falha.
import { chromium, firefox } from 'playwright';

const [url, esperado] = process.argv.slice(2);
if (!url) { console.error('uso: node testes/indisponivel.mjs <url> [texto esperado]'); process.exit(2); }

async function abrir(motor) {
  const nav = await motor.launch();
  const ctx = await nav.newContext({ acceptDownloads: true });
  const pag = await ctx.newPage();
  let baixou = null;
  pag.on('download', d => { baixou = d.suggestedFilename(); });
  let falha = null, resp = null;
  try {
    resp = await pag.goto(url, { waitUntil: 'load', timeout: 15000 });
  } catch (e) {
    // o navegador aborta a navegação quando a resposta vira download ("Download is starting")
    falha = String(e.message || e).split('\n')[0];
  }
  await pag.waitForTimeout(500);
  const tipo = resp ? (resp.headers()['content-type'] || '(sem Content-Type)') : '(sem resposta)';
  const status = resp ? resp.status() : '-';
  let corpo = '';
  try { corpo = await pag.evaluate(() => document.documentElement.outerHTML); } catch { /* sem documento */ }
  await nav.close();
  const erros = [];
  if (baixou !== null) erros.push(`o navegador baixou um arquivo ("${baixou}") em vez de mostrar a página`);
  if (falha) erros.push(`navegação falhou: ${falha}`);
  if (resp && !tipo.startsWith('text/html')) erros.push(`Content-Type ${tipo} (esperado text/html)`);
  if (esperado && !corpo.includes(esperado)) erros.push(`o corpo não contém "${esperado}"`);
  return { status, tipo, erros };
}

let falhou = false;
for (const [nome, motor] of [['chromium', chromium], ['firefox', firefox]]) {
  let r;
  try { r = await abrir(motor); } catch (e) { r = { status: '-', tipo: '-', erros: [`não abriu o ${nome}: ${String(e.message || e).split('\n')[0]}`] }; }
  if (r.erros.length) {
    falhou = true;
    console.error(`FALHOU ${nome} ${url} · status ${r.status} · ${r.tipo}\n  - ${r.erros.join('\n  - ')}`);
  } else console.log(`ok ${nome} ${url} · status ${r.status} · ${r.tipo}`);
}
process.exit(falhou ? 1 : 0);
