// Teste das mudanças de 24 e 25/10/2026 no topo da página (docs/BUGS.md, M-010, M-011, M-012 e M-018), com
// Playwright/Chromium e **nenhum servidor** (rotas e WebSocket falsos):
//   M-018 — sem o chip "Anuncie Aqui" do M-011 (retirado em 25/10); o topo continua inteiro, o tema no fim da linha;
//   M-012 — o selo "EM BREVE" / Playlist (#conexao) ficou escondido até o fim da totalização do 2º turno; M-022
//           (26/10/2026) o traz de volta: o selo à vista e o aviso da próxima eleição com o link "▶ Playlist";
//   M-010 — sem o aviso "destinação dos votos presumida" na faixa (os outros avisos continuam).
//   cd site && node testes/topo_anuncie.mjs [--saida /tmp/uv-topo-anuncie]
import { chromium } from 'playwright';
import { mkdirSync, readFileSync, existsSync } from 'node:fs';
import { dirname, join, extname } from 'node:path';
import { fileURLToPath } from 'node:url';

const arg = (k, d) => { const i = process.argv.indexOf(`--${k}`); return i > 0 ? process.argv[i + 1] : d; };
const saida = arg('saida', '/tmp/uv-topo-anuncie');
mkdirSync(saida, { recursive: true });
const DIST = join(dirname(fileURLToPath(import.meta.url)), '..', 'dist');
const ORIGEM = 'http://uv.teste';
const TIPOS = { '.js': 'text/javascript', '.css': 'text/css', '.svg': 'image/svg+xml', '.woff2': 'font/woff2' };
const INDEX = readFileSync(join(DIST, 'index.html'), 'utf8').replaceAll('{{versao}}', 'teste').replace(/<meta name="uv-dados" content="[^"]*">/, '<meta name="uv-dados" content="">');
const falhas = [];
const conferir = (ok, o) => { console.log(`${ok ? 'ok   ' : 'FALHA'} ${o}`); if (!ok) falhas.push(o); };

const PROXIMA = { eleicao: '2026-10-25', nome: '2º turno das Eleições Gerais 2026', inicio: '2026-10-25T17:00:00-03:00', fim: '2026-10-26T08:00:00-03:00' };
const AGORA = '2026-10-24T20:30:00';
const playlist = { agora: `${AGORA}-03:00`, suspensao: null, proxima_suspensao: null, proxima_ao_vivo: { ...PROXIMA, ao_vivo: false },
  no_ar: null, fila: [], opcoes: [], meu_voto: null, total_votos: 0, tocados: [] };
const estado = (alertas) => ({ tipo: 'estado', versao: 'teste', estado: 'acompanhando', encerrada: false, pleito: 3221, relogio_site: '24/10/2026 20:30:00',
  relogio_sistema: { agora: `${AGORA}.000-03:00`, noite0: null, sistema0: null, acelerar: 1.0 },
  secoes: { catalogo: 517179, publicadas_cs: 0, aplicadas: 0, nao_instaladas: 0, pendentes: 0 }, ultimo_bu: { seq: 0, site: null, sistema: null },
  pausa_429: { ativa: false, retomada_em: null, pausas: 0 }, conferencias: 0, alertas, clientes: 1 });

const nav = await chromium.launch();
async function abrir(largura, alertas, tema, { primeiraVisita = false } = {}) {
  const movel = largura < 640;
  const ctx = await nav.newContext({ ...(movel ? { viewport: { width: largura, height: 780 }, deviceScaleFactor: 2, isMobile: true, hasTouch: true }
    : { viewport: { width: largura, height: 900 } }), timezoneId: 'America/Sao_Paulo', locale: 'pt-BR', colorScheme: tema });
  await ctx.clock.setFixedTime(new Date(`${AGORA}-03:00`));
  if (!primeiraVisita) await ctx.addInitScript(() => { try { localStorage.setItem('uv-aviso-dia', '2099-01-01'); } catch { /* sem armazenamento */ } });
  await ctx.route(`${ORIGEM}/**`, (r) => {
    const p = new URL(r.request().url()).pathname;
    if (p === '/' || p === '/index.html') return r.fulfill({ status: 200, contentType: 'text/html; charset=utf-8', body: INDEX });
    if (p.startsWith('/static/')) { const f = join(DIST, p.slice(8)); if (existsSync(f)) return r.fulfill({ status: 200, contentType: TIPOS[extname(f)] || 'application/octet-stream', body: readFileSync(f) }); }
    if (p === '/api/v1/nomes') return r.fulfill({ status: 200, contentType: 'application/json', body: 'null' });
    if (p === '/api/v1/topicos') return r.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
    if (p === '/api/v1/playlist') return r.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(playlist) });
    return r.fulfill({ status: 404, contentType: 'application/json', body: '{"erro":"nao_encontrado"}' });
  });
  await ctx.routeWebSocket(`${ORIGEM.replace('http', 'ws')}/ws`, (ws) => { ws.send(JSON.stringify(estado(alertas))); ws.onMessage(() => {}); });
  const p = await ctx.newPage();
  await p.goto(`${ORIGEM}/`);
  await p.waitForTimeout(2000);
  return { ctx, p, tag: `${movel ? `celular-${largura}` : `desktop-${largura}`} ${tema}` };
}

for (const largura of [1920, 1366, 390, 360, 320]) {
  for (const tema of ['dark', 'light']) {
    const { ctx, p, tag } = await abrir(largura, { destinacao_presumida: 1 }, tema);
    // M-018 (Luciano, 25/10, 13h59: "pode remover o 'anuncie aqui'. Não vou mais querer mostrá-la"): sem o chip
    // M-011 e sem o texto em lugar nenhum da página
    conferir((await p.locator('#anuncie, .anuncie').count()) === 0, `${tag}: sem o chip "Anuncie Aqui" (M-018)`);
    conferir(!/anuncie/i.test((await p.locator('body').textContent()) || ''), `${tag}: nenhum "Anuncie" na página (M-018)`);
    conferir((await p.locator('a[href^="mailto:"]').count()) === 0, `${tag}: nenhum link mailto no topo nem na página (M-018)`);
    // o topo continua inteiro: marca à esquerda, botão do tema no fim da linha, nada sobreposto
    const topo = await p.locator('header.topo').boundingBox();
    const marca = await p.locator('.marca').boundingBox(), temaB = await p.locator('#tema').boundingBox();
    const pad = await p.$eval('header.topo', (x) => parseFloat(getComputedStyle(x).paddingRight));
    conferir(temaB.y >= topo.y && temaB.y + temaB.height <= topo.y + 52 && marca.y >= topo.y && marca.y + marca.height <= topo.y + 52,
      `${tag}: marca e tema na primeira linha do topo`);
    // fim da linha = fim do conteúdo do header (o html tem scrollbar-gutter: stable: a faixa da rolagem fica reservada)
    const util = topo.x + topo.width;
    conferir(Math.abs(util - pad - (temaB.x + temaB.width)) <= 2, `${tag}: botão do tema no fim da linha (direita em ${Math.round(temaB.x + temaB.width)} de ${Math.round(util)} − ${pad})`);
    // M-025: a navegação (local, cargos) e o selo saíram do topo para a barra da eleição
    conferir(marca.x + marca.width <= temaB.x, `${tag}: marca e tema sem sobrepor`);
    conferir(await p.locator('#eleicao #conexao').isVisible(), `${tag}: selo EM BREVE / Playlist à vista, na barra da eleição (M-022, M-025)`);
    const al = (await p.locator('.faixa .chip.alerta').allTextContents()).join(' | ');
    conferir(!/destina/i.test(al), `${tag}: sem o aviso da destinação presumida (M-010)${al.trim() ? ` — "${al}"` : ''}`);
    const larg = await p.evaluate(() => document.documentElement.scrollWidth);
    conferir(larg <= largura, `${tag}: sem rolagem horizontal (${larg} px)`);
    await p.screenshot({ path: join(saida, `${tag.replace(' ', '-')}.png`) });
    await ctx.close();
  }
}
// M-022 (volta do M-012): o aviso "Próxima eleição AO VIVO" (o que abre sozinho na primeira visita do dia e ao tocar na
// contagem) com o link da playlist, junto com o selo
for (const largura of [1366, 390]) {
  const { ctx, p, tag } = await abrir(largura, { destinacao_presumida: 1 }, 'dark', { primeiraVisita: true });
  const sozinho = await p.locator('#folha.aberta').isVisible().catch(() => false);
  conferir(sozinho && /AO VIVO/.test((await p.locator('#folha').textContent()) || ''), `${tag}: na primeira visita do dia o aviso da próxima eleição abre sozinho`);
  conferir((await p.locator('#folha a[href="/playlist"]').count()) > 0, `${tag}: o aviso que abre sozinho leva à playlist (M-022)`);
  if (sozinho) await p.click('#folha-fechar');
  await p.waitForTimeout(500);
  await p.click('.faixa .chip.promo').catch(() => null);
  await p.waitForTimeout(500);
  conferir(await p.locator('#folha.aberta').isVisible().catch(() => false), `${tag}: tocar na contagem abre o aviso`);
  conferir((await p.locator('#folha a[href="/playlist"]').count()) > 0 && /Playlist/.test((await p.locator('#folha').textContent()) || ''),
    `${tag}: o aviso da contagem leva à playlist (M-022)`);
  await ctx.close();
}
// os outros avisos da faixa continuam
{
  const { ctx, p, tag } = await abrir(1366, { destinacao_presumida: 1, anulacao: 1 }, 'dark');
  const al = (await p.locator('.faixa .chip.alerta').allTextContents()).join(' | ');
  conferir(/anulada/.test(al) && !/destina/i.test(al), `${tag}: o aviso de seção anulada continua, sem o da destinação ("${al}")`);
  await ctx.close();
}
await nav.close();
console.log(falhas.length ? `\n${falhas.length} falha(s)` : '\ntudo ok');
process.exit(falhas.length ? 1 : 0);
