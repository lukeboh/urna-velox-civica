// Teste do aviso de versão nova (docs/BUGS.md, M-016), com Playwright/Chromium e **nenhum servidor** (rotas e WebSocket
// falsos). A página não recarrega sozinha quando o servidor troca de versão; a mensagem `estado` traz a versão do servidor
// e, se for MAIOR que a da página, aparece "Nova versão disponível · Atualizar". Confere, de 320 a 1440 px:
//   - versão igual, MENOR (um nó ainda não trocado, numa implantação) ou ilegível: nenhum aviso;
//   - versão maior: aviso à vista, inteiro na tela, texto sem corte, acima da barra do feed, sem cobrir o título;
//   - NADA PULA: o aviso é fixo (fora do fluxo): as caixas do título, do seletor e dos cartões não se movem e o CLS não sobe;
//   - ✕ dispensa o aviso daquela versão (não volta com a mesma) e uma versão ainda maior o traz de volta;
//   - "Atualizar" recarrega a página; a medição de uso (versao-aviso / versao-atualizar / versao-dispensar) sai como POST vazio.
//   cd site && node testes/aviso_versao.mjs [--saida pasta]
import { chromium } from 'playwright';
import { mkdirSync, readFileSync, existsSync } from 'node:fs';
import { dirname, join, extname } from 'node:path';
import { fileURLToPath } from 'node:url';

const arg = (k, d) => { const i = process.argv.indexOf(`--${k}`); return i > 0 ? process.argv[i + 1] : d; };
const saida = arg('saida', join(process.env.TEMP || '/tmp', 'uv-aviso-versao'));
mkdirSync(saida, { recursive: true });
const AQUI = dirname(fileURLToPath(import.meta.url));
const DIST = join(AQUI, '..', 'dist');
const ORIGEM = 'http://uv.teste';
const CARREGADA = '0.9.25'; // a versão com que a página "foi carregada" (o ?v= do app.js)
const TIPOS = { '.js': 'text/javascript', '.css': 'text/css', '.svg': 'image/svg+xml', '.woff2': 'font/woff2' };
const INDEX = readFileSync(join(DIST, 'index.html'), 'utf8').replaceAll('{{versao}}', CARREGADA).replace(/<meta name="uv-dados" content="[^"]*">/, '<meta name="uv-dados" content="">');
const NOMES = readFileSync(join(AQUI, 'fixtures', 'nomes_2t_presidente.json'), 'utf8');
const falhas = [];
const conferir = (ok, o) => { console.log(`${ok ? 'ok   ' : 'FALHA'} ${o}`); if (!ok) falhas.push(o); };

const AGORA = '2026-10-25T10:30:00';
const estado = (versao) => ({ tipo: 'estado', versao, estado: 'acompanhando', encerrada: false, pleito: 3221, relogio_site: '25/10/2026 10:30:00',
  relogio_sistema: { agora: `${AGORA}.000-03:00`, noite0: null, sistema0: null, acelerar: 1.0 },
  secoes: { catalogo: 517179, publicadas_cs: 0, aplicadas: 171, nao_instaladas: 0, pendentes: 0 }, ultimo_bu: { seq: 171, site: '25/10/2026 10:30:03', sistema: `${AGORA}.000-03:00` },
  pausa_429: { ativa: false, retomada_em: null, pausas: 0 }, conferencias: 0, alertas: {}, clientes: 1 });
const resultado = { v: 1, eleicao: 6258, turno: 2, cargo: 1, abr: 'br', circ: 'br', seq: 171, ultimo_bu: `${AGORA}-03:00`, totalizado_em: `${AGORA}.100-03:00`, encerrada: false,
  secoes: { total: 499248, apuradas: 171 }, eleitorado: 158745502, apurado: 119850, comparecimento: 51402, vb: 1306, vn: 1134, vnt: 0, validos: 48962, legenda: 0,
  estado: 'apurando', cand: [{ n: 22, v: 33665, pct: 68.76, dvt: 'Válido', st: '', mat: null }, { n: 13, v: 15297, pct: 31.24, dvt: 'Válido', st: '', mat: null }] };

const nav = await chromium.launch();
async function abrir(largura, versaoServidor, { tema = 'dark' } = {}) {
  const movel = largura < 640;
  const usos = [], raiz = { n: 0 };
  const sockets = [];
  const ctx = await nav.newContext({ ...(movel ? { viewport: { width: largura, height: 780 }, deviceScaleFactor: 2, isMobile: true, hasTouch: true }
    : { viewport: { width: largura, height: 900 } }), timezoneId: 'America/Sao_Paulo', locale: 'pt-BR', colorScheme: tema });
  await ctx.clock.setFixedTime(new Date(`${AGORA}-03:00`));
  await ctx.addInitScript(() => { try { localStorage.setItem('uv-aviso-dia', '2099-01-01'); localStorage.setItem('uv-dica-fonte', '1'); } catch { /* sem armazenamento */ } });
  await ctx.route(`${ORIGEM}/**`, (r) => {
    const u = new URL(r.request().url()), p = u.pathname;
    if (r.request().method() === 'POST' && p.startsWith('/api/v1/uso/')) { usos.push(p.slice(12)); return r.fulfill({ status: 404, contentType: 'application/json', body: '{"erro":"nao_encontrado"}' }); }
    if (p === '/' || p === '/index.html') { raiz.n++; return r.fulfill({ status: 200, contentType: 'text/html; charset=utf-8', body: INDEX }); }
    if (p.startsWith('/static/')) { const f = join(DIST, p.slice(8)); if (existsSync(f)) return r.fulfill({ status: 200, contentType: TIPOS[extname(f)] || 'application/octet-stream', body: readFileSync(f) }); }
    if (p === '/api/v1/nomes') return r.fulfill({ status: 200, contentType: 'application/json', body: NOMES });
    if (p === '/api/v1/topicos') return r.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify([{ topico: 'br-c0001', arquivo: 'br-c0001-e006258.json', seq: 171 }]) });
    if (p === '/api/v1/playlist') return r.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ agora: `${AGORA}-03:00`, suspensao: null, proxima_suspensao: null, proxima_ao_vivo: null, no_ar: null, fila: [], opcoes: [], meu_voto: null, total_votos: 0, tocados: [] }) });
    return r.fulfill({ status: 404, contentType: 'application/json', body: '{"erro":"nao_encontrado"}' });
  });
  await ctx.routeWebSocket(`${ORIGEM.replace('http', 'ws')}/ws`, (ws) => {
    sockets.push(ws);
    ws.send(JSON.stringify(estado(versaoServidor)));
    ws.send(JSON.stringify({ tipo: 'resultado', topico: 'br-c0001', seq: 171, txt: JSON.stringify({ resultado }) }));
    ws.onMessage(() => {});
  });
  const p = await ctx.newPage();
  await p.addInitScript(() => { try { window.__clsT = 0; new PerformanceObserver((l) => { for (const e of l.getEntries()) if (!e.hadRecentInput) window.__clsT += e.value; }).observe({ type: 'layout-shift', buffered: true }); } catch { /* sem layout-shift */ } });
  await p.goto(`${ORIGEM}/#/br/c0001`);
  await p.waitForSelector('.fontes .comparar', { timeout: 15000 });
  await p.waitForTimeout(1200);
  p.__cls0 = await p.evaluate(() => window.__clsT || 0);
  return { ctx, p, usos, raiz, enviar: (v) => sockets.forEach((s) => { try { s.send(JSON.stringify(estado(v))); } catch { /* socket de antes do reload */ } }) };
}
const visivel = (p) => p.evaluate(() => { const n = document.querySelector('.nova-versao'); return !!n && !n.hidden && getComputedStyle(n).display !== 'none'; });
const caixas = (p) => p.evaluate(() => {
  const r = (s) => { const b = document.querySelector(s)?.getBoundingClientRect(); return b ? [Math.round(b.x), Math.round(b.y + scrollY), Math.round(b.width), Math.round(b.height)] : null; };
  return { h1: r('.corrida h1'), fontes: r('.fontes'), controle: r('.fontes .comparar'), lista: r('.lista'), apuracao: r('.apuracao') };
});

// ---------------------------------------------------------------- sem aviso: igual, menor, ilegível
for (const [v, rot] of [[CARREGADA, 'igual'], ['0.9.24', 'menor (um nó ainda não trocado)'], ['teste', 'ilegível'], ['0.9.25-alpha.1', 'igual com sufixo']]) {
  const { ctx, p } = await abrir(390, v);
  conferir(!(await visivel(p)), `servidor ${v} (${rot}) × página ${CARREGADA}: nenhum aviso`);
  await ctx.close();
}

// ---------------------------------------------------------------- com aviso: por largura
for (const largura of [320, 360, 390, 768, 1366, 1440]) {
  const tag = largura < 640 ? `celular ${largura}` : `tela ${largura}`;
  const { ctx, p, usos, raiz, enviar } = await abrir(largura, CARREGADA);
  conferir(!(await visivel(p)), `${tag}: sem aviso enquanto a versão é a mesma`);
  const antes = await caixas(p);
  enviar('0.9.26'); // a troca de versão chega pelo WebSocket (o mesmo que o servidor faz a cada mensagem de estado)
  await p.waitForTimeout(900);
  conferir(await visivel(p), `${tag}: versão 0.9.26 > ${CARREGADA}: aviso à vista`);
  const t = await p.evaluate(() => { const n = document.querySelector('.nova-versao'); const tx = n.querySelector('.nv-txt'), b = n.getBoundingClientRect(), f = document.querySelector('.feed')?.getBoundingClientRect();
    return { texto: tx.textContent, botoes: [...n.querySelectorAll('button')].map((x) => x.textContent), corte: tx.scrollWidth > tx.clientWidth + 1, x: Math.round(b.x), dir: Math.round(b.right), y: Math.round(b.bottom), feedY: f ? Math.round(f.top) : null, alvos: [...n.querySelectorAll('button')].map((x) => Math.round(x.getBoundingClientRect().height)), larg: innerWidth, role: n.getAttribute('role') }; });
  conferir(t.texto === 'Nova versão disponível' && JSON.stringify(t.botoes) === JSON.stringify(['Atualizar', '✕']), `${tag}: texto e botões (${t.texto} · ${t.botoes})`);
  conferir(!t.corte, `${tag}: texto do aviso sem corte`);
  conferir(t.x >= 0 && t.dir <= t.larg, `${tag}: inteiro na tela (${t.x}–${t.dir} de ${t.larg})`);
  conferir(t.feedY == null || t.y <= t.feedY, `${tag}: acima da barra do feed (${t.y} ≤ ${t.feedY})`);
  conferir(Math.min(...t.alvos) >= 30 && t.role === 'status', `${tag}: botões de ${Math.min(...t.alvos)} px de altura e role="status"`);
  const depois = await caixas(p);
  conferir(JSON.stringify(depois) === JSON.stringify(antes), `${tag}: nada se moveu (título, seletor, lista e cartão nos mesmos lugares)`);
  const cls = (await p.evaluate(() => window.__clsT || 0)) - p.__cls0;
  conferir(cls < 0.001, `${tag}: CLS +${cls.toFixed(4)} depois do aviso`);
  conferir(usos.filter((u) => u === 'versao-aviso').length === 1, `${tag}: aviso medido uma vez (${JSON.stringify(usos)})`);
  enviar('0.9.26');
  await p.waitForTimeout(500);
  conferir(usos.filter((u) => u === 'versao-aviso').length === 1, `${tag}: outra mensagem com a mesma versão não repete a medição`);
  await p.screenshot({ path: join(saida, `${tag.replace(' ', '-')}-1-aviso.png`) });
  // ✕ dispensa essa versão; a mesma versão não traz o aviso de volta; uma maior, sim
  await p.click('.nova-versao .fechar');
  await p.waitForTimeout(400);
  conferir(!(await visivel(p)), `${tag}: ✕ esconde o aviso`);
  enviar('0.9.26');
  await p.waitForTimeout(500);
  conferir(!(await visivel(p)), `${tag}: a mesma versão dispensada não volta`);
  enviar('0.9.27');
  await p.waitForTimeout(500);
  conferir(await visivel(p), `${tag}: uma versão ainda maior (0.9.27) traz o aviso de volta`);
  // "Atualizar" recarrega a página (nova busca do index, que é no-cache)
  const n0 = raiz.n;
  await Promise.all([p.waitForNavigation({ waitUntil: 'load', timeout: 10000 }).catch(() => null), p.click('.nova-versao .atualizar')]);
  await p.waitForTimeout(800);
  conferir(raiz.n === n0 + 1, `${tag}: "Atualizar" recarrega a página (index buscado de novo: ${n0} → ${raiz.n})`);
  conferir(['versao-aviso', 'versao-dispensar', 'versao-aviso', 'versao-atualizar'].every((e) => usos.includes(e)), `${tag}: medição de uso ${JSON.stringify(usos)}`);
  const larg = await p.evaluate(() => document.documentElement.scrollWidth);
  conferir(larg <= largura, `${tag}: sem rolagem horizontal (${larg} px)`);
  await ctx.close();
}

// ---------------------------------------------------------------- tema claro: contraste do texto do aviso
for (const largura of [360, 1440]) {
  const { ctx, p, enviar } = await abrir(largura, CARREGADA, { tema: 'light' });
  await p.evaluate(() => { try { localStorage.setItem('uv-tema', 'claro'); } catch { /* */ } });
  await p.reload();
  await p.waitForSelector('.fontes .comparar');
  await p.waitForTimeout(1200);
  enviar('0.9.26');
  await p.waitForTimeout(900);
  await p.screenshot({ path: join(saida, `claro-${largura}-aviso.png`) });
  await ctx.close();
}
await nav.close();
console.log(falhas.length ? `\n${falhas.length} falha(s)` : '\ntudo ok');
process.exit(falhas.length ? 1 : 0);
