// Teste do B-050 (docs/BUGS.md) e do aviso "totalização antes das 17h" (pedido do Luciano em 23/10/2026): no Brasil a
// totalização começa oficialmente às 17h (Brasília), mas os boletins de urna do exterior são publicados pelo TSE a
// partir de ~1h da madrugada de domingo (1º turno de 2026: a primeira seção às 01:49, Wellington) e o Urna Velox
// totaliza cada BU assim que ele sai — começa antes. Com Playwright/Chromium e **nenhum servidor** (rotas e WebSocket
// falsos; relógio do serviço no `estado`).
//   cd site && node testes/antes_17h.mjs [--saida /tmp/uv-antes-17h]
// Cenários (a 1366, 390 e 360 px):
//   madrugada  — 25/10 03:10, BUs do exterior aplicados: selo AO VIVO, sem a contagem "Próxima eleição AO VIVO · faltam"
//                (B-050) nem o item dela no letreiro; chip "Oficial às 17h" inteiro à vista; tocar abre a explicação
//                (17h oficial, BUs do exterior a partir de 1h, o Urna Velox totaliza cada BU quando sai); o mesmo no
//                letreiro (item fixo). Também sem a playlist (a hora vem do primeiro BU da noite).
//   depois     — 25/10 17:05: nada do aviso.
//   virada     — 26/10 01:00 (a noite continua depois da meia-noite): nada do aviso.
//   espera     — 25/10 03:10 sem nenhum BU: a contagem continua (a noite oficial ainda não começou), sem o aviso.
import { chromium } from 'playwright';
import { mkdirSync, readFileSync, existsSync } from 'node:fs';
import { dirname, join, extname } from 'node:path';
import { fileURLToPath } from 'node:url';

const arg = (k, d) => { const i = process.argv.indexOf(`--${k}`); return i > 0 ? process.argv[i + 1] : d; };
const saida = arg('saida', '/tmp/uv-antes-17h');
mkdirSync(saida, { recursive: true });
const DIST = join(dirname(fileURLToPath(import.meta.url)), '..', 'dist');
const ORIGEM = 'http://uv.teste';
const TIPOS = { '.js': 'text/javascript', '.css': 'text/css', '.svg': 'image/svg+xml', '.woff2': 'font/woff2' };
const INDEX = readFileSync(join(DIST, 'index.html'), 'utf8').replaceAll('{{versao}}', 'teste').replace(/<meta name="uv-dados" content="[^"]*">/, '<meta name="uv-dados" content="">');
const falhas = [];
const conferir = (ok, o) => { console.log(`${ok ? 'ok   ' : 'FALHA'} ${o}`); if (!ok) falhas.push(o); };

const PROXIMA = { eleicao: '2026-10-25', nome: '2º turno das Eleições Gerais 2026', inicio: '2026-10-25T17:00:00-03:00', fim: '2026-10-26T08:00:00-03:00' };
const playlist = (agora) => ({ agora, suspensao: { eleicao: '2026-10-25', nome: PROXIMA.nome, de: '2026-10-24T00:00:00-03:00', ate: '2026-10-29T00:00:00-03:00', retoma_em: '2026-10-29T00:00:00-03:00' },
  proxima_suspensao: null, proxima_ao_vivo: { ...PROXIMA, ao_vivo: agora >= PROXIMA.inicio }, no_ar: null, fila: [], opcoes: [], meu_voto: null, total_votos: 0, tocados: [] });
const site = (iso) => { const [d, h] = iso.split('T'); const [a, m, dd] = d.split('-'); return `${dd}/${m}/${a} ${h.slice(0, 8)}`; };
const estado = (agora, aplicadas) => ({ tipo: 'estado', versao: 'teste', estado: 'acompanhando', encerrada: false, pleito: 3221, relogio_site: site(agora),
  relogio_sistema: { agora: `${agora}.000-03:00`.replace('-03:00.000', '.000'), noite0: null, sistema0: null, acelerar: 1.0 },
  secoes: { catalogo: 499248, publicadas_cs: aplicadas, aplicadas, nao_instaladas: 0, pendentes: 0 }, ultimo_bu: { seq: aplicadas, site: null, sistema: null },
  pausa_429: { ativa: false, retomada_em: null, pausas: 0 }, conferencias: 0, alertas: {}, clientes: 1 });
const PRIMEIRA = { tipo: 'marco', n: 1, classe: 'primeira_secao', seq: 1, horario: '2026-10-25 01:49:24', abrangencia: 'br', cargo: null, topico: null,
  uf: 'zz', municipio: 30805, municipio_nome: 'WELLINGTON', zona: 1, secao: 1088 };

const nav = await chromium.launch();
async function abrir(largura, { agora, aplicadas, comPlaylist = true, atrasoEstado = 0 }) {
  const movel = largura < 640;
  const ctx = await nav.newContext({ ...(movel ? { viewport: { width: largura, height: 780 }, deviceScaleFactor: 2, isMobile: true, hasTouch: true }
    : { viewport: { width: largura, height: 900 } }), timezoneId: 'America/Sao_Paulo', locale: 'pt-BR' });
  await ctx.clock.setFixedTime(new Date(`${agora}-03:00`));
  await ctx.addInitScript(() => { try { localStorage.setItem('uv-aviso-dia', '2099-01-01'); } catch { /* sem armazenamento */ } });
  await ctx.route(`${ORIGEM}/**`, (r) => {
    const p = new URL(r.request().url()).pathname;
    if (p === '/' || p === '/index.html') return r.fulfill({ status: 200, contentType: 'text/html; charset=utf-8', body: INDEX });
    if (p.startsWith('/static/')) { const f = join(DIST, p.slice(8)); if (existsSync(f)) return r.fulfill({ status: 200, contentType: TIPOS[extname(f)] || 'application/octet-stream', body: readFileSync(f) }); }
    if (p === '/api/v1/nomes') return r.fulfill({ status: 200, contentType: 'application/json', body: 'null' });
    if (p === '/api/v1/topicos') return r.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
    if (p === '/api/v1/playlist' && comPlaylist) return r.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(playlist(`${agora}-03:00`)) });
    return r.fulfill({ status: 404, contentType: 'application/json', body: '{"erro":"nao_encontrado"}' });
  });
  await ctx.routeWebSocket(`${ORIGEM.replace('http', 'ws')}/ws`, (ws) => {
    // B-062: o estado pode chegar depois da playlist (página mais pesada, rede lenta)
    if (atrasoEstado) setTimeout(() => ws.send(JSON.stringify(estado(agora, aplicadas))), atrasoEstado);
    else ws.send(JSON.stringify(estado(agora, aplicadas)));
    ws.onMessage((txt) => {
      let m;
      try { m = JSON.parse(txt); } catch { return; }
      if (m.op === 'marcos' && aplicadas > 0) ws.send(JSON.stringify(PRIMEIRA));
    });
  });
  const p = await ctx.newPage();
  await p.goto(`${ORIGEM}/`);
  await p.waitForTimeout(2500);
  return { ctx, p, tag: movel ? `celular-${largura}` : 'desktop' };
}
const visivel = (p, sel) => p.locator(sel).first().isVisible().catch(() => false);
const texto = async (p, sel) => ((await p.locator(sel).first().textContent().catch(() => '')) || '').replace(/\s+/g, ' ').trim();
const itensLetreiro = (p) => p.$$eval('#trilho .item', (xs) => xs.map((x) => x.textContent.replace(/\s+/g, ' ').trim()));

for (const largura of [1366, 390, 360]) {
  for (const comPlaylist of [true, false]) {
    const { ctx, p, tag } = await abrir(largura, { agora: '2026-10-25T03:10:00', aplicadas: 412, comPlaylist });
    const t = `${tag} · madrugada${comPlaylist ? '' : ' (sem playlist)'}`;
    conferir(/AO VIVO/.test(await texto(p, '#conexao')), `${t}: selo AO VIVO`);
    const promo = (await visivel(p, '.faixa .chip.promo')) ? await texto(p, '.faixa .chip.promo') : '';
    conferir(!/faltam|Próxima/.test(promo), `${t}: sem a contagem "Próxima eleição AO VIVO · faltam" (B-050)${promo ? ` — "${promo}"` : ''}`);
    const letreiro = await itensLetreiro(p);
    conferir(!letreiro.some((x) => /Próxima eleição AO VIVO/.test(x)), `${t}: o letreiro não anuncia a "próxima eleição AO VIVO" (B-050)`);
    conferir(await visivel(p, '.faixa .chip.cedo'), `${t}: chip do aviso "Oficial às 17h" à vista`);
    const chip = await texto(p, '.faixa .chip.cedo');
    conferir(/17h/.test(chip), `${t}: o chip diz a hora oficial ("${chip}")`);
    const c = await p.locator('.faixa .chip.cedo').first().boundingBox().catch(() => null);
    const cortado = await p.locator('.faixa .chip.cedo').first().evaluate((e) => e.scrollWidth > e.clientWidth + 1).catch(() => true);
    conferir(!!c && c.x >= 15.5 && c.x + c.width <= largura - 15.5 && !cortado, `${t}: chip inteiro à vista${c ? ` (${Math.round(c.x)}–${Math.round(c.x + c.width)} de ${largura} px)` : ''}`);
    conferir(letreiro.some((x) => /17h/.test(x) && /exterior/i.test(x)), `${t}: o aviso também no letreiro (${letreiro.length} itens)`);
    await p.click('.faixa .chip.cedo').catch(() => null);
    await p.waitForTimeout(600);
    const folha = (await visivel(p, '#folha.aberta')) ? await texto(p, '#folha') : '';
    conferir(/oficialmente às 17h/.test(folha) && /exterior/.test(folha) && /1h da madrugada/.test(folha) && /Urna Velox totaliza cada boletim de urna assim que/.test(folha),
      `${t}: tocar no chip abre a explicação${folha ? ` ("${folha.slice(0, 110)}…")` : ' (não abriu)'}`);
    await p.screenshot({ path: `${saida}/madrugada-${tag}${comPlaylist ? '' : '-sem-playlist'}.png` });
    const cls = await p.evaluate(() => window.__uv?.cls ?? 0);
    conferir(cls < 0.001, `${t}: CLS ${cls}`);
    await ctx.close();
  }
  // B-062: o estado da noite chega 1,5 s depois da playlist — a próxima eleição não pode entrar no letreiro antes dele
  // (um item à vista marcado para sair só vai embora quando passa da borda esquerda: ficava anunciando)
  {
    const { ctx, p, tag } = await abrir(largura, { agora: '2026-10-25T03:10:00', aplicadas: 412, atrasoEstado: 1500 });
    const letreiro = await itensLetreiro(p);
    conferir(!letreiro.some((x) => /Próxima eleição AO VIVO/.test(x)), `${tag} · madrugada, estado depois da playlist: o letreiro não anuncia a "próxima eleição AO VIVO" (B-062)`);
    conferir(/AO VIVO/.test(await texto(p, '#conexao')), `${tag} · madrugada, estado depois da playlist: selo AO VIVO`);
    await ctx.close();
  }
  for (const [nome, agora] of [['depois das 17h', '2026-10-25T17:05:00'], ['virada (26/10 01:00)', '2026-10-26T01:00:00']]) {
    const { ctx, p, tag } = await abrir(largura, { agora, aplicadas: 95000 });
    conferir(!(await visivel(p, '.faixa .chip.cedo')), `${tag} · ${nome}: sem o aviso "Oficial às 17h"`);
    conferir(!(await itensLetreiro(p)).some((x) => /Oficial às 17h|antes das 17h/.test(x)), `${tag} · ${nome}: nada do aviso no letreiro`);
    await ctx.close();
  }
  {
    const { ctx, p, tag } = await abrir(largura, { agora: '2026-10-25T03:10:00', aplicadas: 0 });
    conferir(await visivel(p, '.faixa .chip.promo'), `${tag} · espera: sem BU nenhum, a contagem para as 17h continua`);
    conferir(!(await visivel(p, '.faixa .chip.cedo')), `${tag} · espera: sem o aviso (ainda não há totalização)`);
    await ctx.close();
  }
}

await nav.close();
console.log(falhas.length ? `antes_17h: ${falhas.length} falha(s)` : 'antes_17h: tudo certo');
process.exit(falhas.length ? 1 : 0);
