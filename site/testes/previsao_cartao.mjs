// Teste do B-052 (a previsão fica constantemente "em horário ainda incerto") e do B-054 (relógio "00:00:0" cortado ao
// lado do "ainda incerto"): o cartão "Previsão" (`src/previsao.js`) sozinho numa página em branco, com Playwright/Chromium
// e nenhum servidor, a 390 px (celular) e a 1366 px.
//   cd site && node testes/previsao_cartao.mjs [--saida /tmp/uv-previsao-cartao]
// Cenário do modo rápido (simulado a 500×, horários com segundos): a faixa P10–P90 do "matematicamente" e a do fim da
// apuração são largas demais (`incerta`), mas a mensagem traz o horário pelo ritmo atual (`eta`). O cartão tem de:
//   - dar um horário ("por volta de 10:14:05"), nunca só "em horário ainda incerto";
//   - mostrar esse horário no relógio "mais provável" (nunca 00:00:00);
//   - no fim da apuração, horário para os 99 % e os 100 %;
//   - com a faixa estreita, continuar "entre … e …" com o P50 no relógio;
//   - sem horário nenhum (chegada parada, sem ritmo): nenhum relógio "mais provável" (nem 00:00:00 esmaecido);
//   - os dígitos do relógio inteiros dentro da área útil do cartão (B-054).
import { chromium } from 'playwright';
import { mkdirSync, readFileSync, existsSync } from 'node:fs';
import { dirname, join, extname } from 'node:path';
import { fileURLToPath } from 'node:url';

const arg = (k, d) => { const i = process.argv.indexOf(`--${k}`); return i > 0 ? process.argv[i + 1] : d; };
const saida = arg('saida', '/tmp/uv-previsao-cartao');
mkdirSync(saida, { recursive: true });
const RAIZ = join(dirname(fileURLToPath(import.meta.url)), '..');
const ORIGEM = 'http://uv.teste';
const TIPOS = { '.js': 'text/javascript', '.css': 'text/css', '.woff2': 'font/woff2' };
const falhas = [];
const conferir = (ok, o) => { console.log(`${ok ? 'ok   ' : 'FALHA'} ${o}`); if (!ok) falhas.push(o); };

const PAGINA = `<!doctype html><html lang="pt-BR" data-tema="escuro"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1"><link rel="stylesheet" href="/dist/estilo.css"></head>
<body><main id="raiz" style="padding:16px"></main></body></html>`;

// mensagens `previsao` (PROTOCOLO_WS §4.7) como o servidor manda no simulado a 500× (relógio do sistema)
const base = { topico: 'ac-c0003', abrangencia: 'ac', cargo: 3, circunscricao: 'ac', estimativa: true, relogio: 'sistema', instante: '2026-10-22 10:06:17',
  situacao: 'prevista', parada: null, base: { apurado: 0.53, secoes: 0.53, aptos_min: 1200 }, chegou: { fim99: null, fim100: null }, definida: null, confirmacao: null,
  historico: [] };
const LARGA = {
  ...base,
  mat: { p10: '2026-10-22 10:06:40', p50: '2026-10-22 10:09:00', p90: '2026-10-22 10:21:00', incerta: true, eta: '2026-10-22 10:12:30', tipo: 'matematicamente_2t', prob_antes_fim: 1, apurado_p50: 0.93 },
  fim99: { p10: '2026-10-22 10:07:00', p50: '2026-10-22 10:11:00', p90: '2026-10-22 10:25:00', incerta: true, eta: '2026-10-22 10:13:20' },
  fim100: { p10: '2026-10-22 10:07:30', p50: '2026-10-22 10:12:00', p90: '2026-10-22 10:30:00', incerta: true, eta: '2026-10-22 10:14:05' },
};
const ESTREITA = { ...LARGA, instante: '2026-10-22 10:08:17',
  mat: { ...LARGA.mat, p10: '2026-10-22 10:11:50', p50: '2026-10-22 10:12:20', p90: '2026-10-22 10:12:50', incerta: false } };
const SEM_RITMO = { ...LARGA, instante: '2026-10-22 10:09:17', mat: { ...LARGA.mat, eta: null }, fim99: { ...LARGA.fim99, eta: null }, fim100: { ...LARGA.fim100, eta: null } };

const nav = await chromium.launch();
for (const largura of [390, 1366]) {
  const tag = largura < 640 ? `celular-${largura}` : 'desktop';
  const ctx = await nav.newContext({ viewport: { width: largura, height: 800 }, ...(largura < 640 ? { deviceScaleFactor: 2, isMobile: true, hasTouch: true } : {}), colorScheme: 'dark' });
  await ctx.route(`${ORIGEM}/**`, async (r) => {
    const p = new URL(r.request().url()).pathname;
    if (p === '/') return r.fulfill({ status: 200, contentType: 'text/html; charset=utf-8', body: PAGINA });
    const f = join(RAIZ, p);
    if ((p.startsWith('/src/') || p.startsWith('/dist/')) && existsSync(f)) return r.fulfill({ status: 200, contentType: TIPOS[extname(f)] || 'application/octet-stream', body: readFileSync(f) });
    return r.fulfill({ status: 404, body: '' });
  });
  const p = await ctx.newPage();
  await p.goto(`${ORIGEM}/`);
  await p.evaluate(async () => {
    const { CartaoPrevisao } = await import('/src/previsao.js');
    const c = new CartaoPrevisao();
    c.definirAceleracao(500);
    document.querySelector('#raiz').append(c.el);
    window.__cartao = c;
  });
  const mostrar = async (m) => {
    await p.evaluate((m) => window.__cartao.definir(m, true, 'Acre'), m);
    await p.waitForTimeout(1200); // transição do hodômetro
    return p.evaluate(() => {
      const el = document.querySelector('.previsao');
      const hod = el.querySelector('.pv-hod');
      // os dígitos de fato (a caixa .pv-hod tem largura fixa; o número pode transbordar dela) e a área útil do cartão
      const pad = parseFloat(getComputedStyle(el).paddingRight) || 0;
      const caixa = el.getBoundingClientRect(), h = hod.querySelector('.hod').getBoundingClientRect();
      const visivel = getComputedStyle(hod).visibility !== 'hidden' && parseFloat(getComputedStyle(hod).opacity) > 0;
      return { faixa: el.querySelector('.pv-faixa').textContent, nota: el.querySelector('.pv-nota').textContent, fim: el.querySelector('.pv-fim').textContent,
        relogio: visivel ? (hod.querySelector('.sr')?.textContent ?? '') : '(escondido)', dentro: h.left >= caixa.left - 0.5 && h.right <= caixa.right - pad + 0.5,
        hodDir: Math.round(h.right), caixaDir: Math.round(caixa.right - pad) };
    });
  };
  let v = await mostrar(LARGA);
  await p.screenshot({ path: `${saida}/larga-${tag}.png` });
  conferir(/por volta de 10:12:30/.test(v.faixa) && !/incerto$/.test(v.faixa), `${tag} · faixa larga: o matematicamente tem horário ("${v.faixa}")`);
  conferir(v.relogio === '10:12:30', `${tag} · faixa larga: relógio "mais provável" ${v.relogio}`);
  conferir(/99% por volta de 10:13:20/.test(v.fim) && /100% por volta de 10:14:05/.test(v.fim), `${tag} · faixa larga: fim da apuração com horário ("${v.fim}")`);
  conferir(v.dentro, `${tag} · relógio inteiro dentro do cartão (relógio até ${v.hodDir} px, cartão até ${v.caixaDir} px)`);
  v = await mostrar(ESTREITA);
  conferir(/entre 10:11:50 e 10:12:50/.test(v.faixa) && v.relogio === '10:12:20', `${tag} · faixa estreita: "${v.faixa}", relógio ${v.relogio}`);
  v = await mostrar(SEM_RITMO);
  await p.screenshot({ path: `${saida}/sem-ritmo-${tag}.png` });
  conferir(v.relogio === '(escondido)', `${tag} · sem ritmo: nenhum relógio "mais provável" (nem 00:00:00 esmaecido): ${v.relogio}`);
  await ctx.close();
}
await nav.close();
console.log(falhas.length ? `\n${falhas.length} FALHA(S)` : '\nprevisao_cartao: tudo ok');
process.exit(falhas.length ? 1 : 0);
