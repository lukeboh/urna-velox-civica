// Ensaio do mapa das seções (M-017) contra um uv-servico de verdade (o simulador da noite, aquisicao/scripts):
// abre a página, espera o cartão carregar e, a cada --passo segundos, compara o que o cartão conta com a saúde do
// serviço (saude.json: secoes.aplicadas e difusao.mapa). Pode recarregar a página no meio (--recarregar-em s) e
// aguentar um reinício do serviço (troca de época: o cartão relê a fotografia). Grava <saida>/ensaio_mapa.json e capturas.
//   node testes/ensaio_mapa.mjs --url http://127.0.0.1:18080/ --saude ~/uvreal/simulacao-site/saida/saude.json \
//        --saida /tmp/uv-ensaio-mapa [--duracao 600] [--passo 10] [--recarregar-em 120] [--modo http] [--esperar-fim]
// Falha (código 1) se, com o serviço parado de acender por um passo, o cartão não bater com o mapa do serviço.
import { chromium } from 'playwright';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const arg = (k, d) => { const i = process.argv.indexOf(`--${k}`); return i > 0 ? process.argv[i + 1] : d; };
const tem = (k) => process.argv.includes(`--${k}`);
const URL0 = arg('url', 'http://127.0.0.1:18080/');
const SAUDE = arg('saude');
const SAIDA = arg('saida', '/tmp/uv-ensaio-mapa');
const DUR = +arg('duracao', 600), PASSO = +arg('passo', 10), RECARREGAR = +arg('recarregar-em', 0);
const MODO = arg('modo', '');
mkdirSync(SAIDA, { recursive: true });
const url = MODO ? `${URL0}${URL0.includes('?') ? '&' : '?'}modo=${MODO}` : URL0;

const saude = () => { try { return JSON.parse(readFileSync(SAUDE, 'utf8')); } catch { return null; } };
const nav = await chromium.launch();
const ctx = await nav.newContext({ viewport: { width: 1366, height: 900 } });
const pg = await ctx.newPage();
const erros = [];
pg.on('pageerror', (e) => erros.push(String(e)));
let bytesMapa = 0, msgsMapa = 0;
pg.on('websocket', (ws) => ws.on('framereceived', (f) => {
  const s = typeof f.payload === 'string' ? f.payload : '';
  if (s.startsWith('{"tipo":"mapa"')) { msgsMapa++; bytesMapa += s.length; }
}));
let bytesGeo = 0;
pg.on('response', async (r) => { if (r.url().includes('/static/mapa.json')) { const h = await r.allHeaders(); bytesGeo = +(h['content-length'] || 0); console.log(`mapa.json: ${h['content-encoding'] || 'sem compressão'}, ${bytesGeo} bytes`); } });

async function abrir() {
  await pg.goto(url, { waitUntil: 'domcontentloaded' });
  const t0 = Date.now();
  // o cartão só busca e desenha quando está na tela
  await pg.waitForFunction(() => window.__uv?.mapa?.el?.isConnected, null, { timeout: 60_000 });
  await pg.evaluate(() => window.__uv.mapa.el.scrollIntoView({ block: 'center' }));
  await pg.waitForFunction(() => window.__uv?.mapa?.resumo?.().carregado, null, { timeout: 120_000 });
  return Date.now() - t0;
}
const res = { url, inicio: new Date().toISOString(), carga_ms: [await abrir()], amostras: [], recargas: 0, epocas: new Set() };
console.log(`cartão carregado em ${res.carga_ms[0]} ms`);
const t0 = Date.now();
let recarregou = false, ultimo = null, falhas = [];
while ((Date.now() - t0) / 1000 < DUR) {
  await pg.waitForTimeout(PASSO * 1000);
  if (RECARREGAR && !recarregou && (Date.now() - t0) / 1000 >= RECARREGAR) {
    recarregou = true; res.recargas++;
    const ms = await abrir(); res.carga_ms.push(ms);
    console.log(`recarregada: cartão de volta em ${ms} ms`);
    await pg.screenshot({ path: join(SAIDA, 'depois-da-recarga.png') });
  }
  // a página recarregou sozinha (a barra de replay do simulador recarrega quando a sessão muda): volta ao cartão
  if (!(await pg.evaluate(() => window.__uv?.mapa?.resumo?.().carregado).catch(() => false))) {
    await pg.waitForFunction(() => window.__uv?.mapa?.el?.isConnected, null, { timeout: 60_000 });
    await pg.evaluate(() => window.__uv.mapa.el.scrollIntoView({ block: 'center' }));
    await pg.waitForFunction(() => window.__uv.mapa.resumo().carregado, null, { timeout: 120_000 });
    res.recargas_sozinha = (res.recargas_sozinha || 0) + 1;
    console.log('a página recarregou sozinha: cartão de volta');
  }
  const r = await pg.evaluate(() => window.__uv.mapa.resumo());
  const s = saude();
  const m = s?.difusao?.mapa;
  const a = { t: Math.round((Date.now() - t0) / 1000), relogio: s?.relogio_site, pagina_n: r.n, pagina_acesas: r.acesas, exterior: r.exterior,
    no_local: r.noLocal, N: r.N, servico_acesas: m?.acesas, servico_secoes: m?.secoes, fora: m?.fora_do_catalogo, aplicadas: s?.secoes?.aplicadas,
    msgs_mapa: msgsMapa, kb_mapa: Math.round(bytesMapa / 1024) };
  res.amostras.push(a);
  console.log(JSON.stringify(a));
  // parado de acender entre duas amostras: o cartão tem de bater com o serviço
  if (ultimo && m && ultimo.servico_acesas === m.acesas && m.acesas > 0) {
    if (r.n !== m.acesas) falhas.push(`t=${a.t}: cartão ${r.n} × serviço ${m.acesas}`);
    if (m.acesas + (m.fora_do_catalogo || 0) !== s.secoes.aplicadas) falhas.push(`t=${a.t}: mapa ${m.acesas}+${m.fora_do_catalogo} fora × aplicadas ${s.secoes.aplicadas}`);
  }
  ultimo = a;
  if (tem('esperar-fim') && s?.secoes?.aplicadas === s?.secoes?.catalogo) break;
}
await pg.screenshot({ path: join(SAIDA, 'fim.png') });
res.erros = erros; res.falhas = falhas; res.bytes_geo = bytesGeo; res.msgs_mapa = msgsMapa; res.kb_mapa = Math.round(bytesMapa / 1024);
delete res.epocas;
writeFileSync(join(SAIDA, 'ensaio_mapa.json'), JSON.stringify(res, null, 1));
console.log(`ensaio_mapa: ${res.amostras.length} amostras, ${falhas.length} divergência(s), ${erros.length} erro(s) na página; ${msgsMapa} mensagens mapa (${res.kb_mapa} KB)`);
for (const f of [...falhas, ...erros]) console.log('  ' + f);
await nav.close();
process.exit(falhas.length || erros.length ? 1 : 0);
