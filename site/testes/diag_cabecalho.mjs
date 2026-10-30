// Diagnóstico do cabeçalho (melhoria do cabeçalho, etapa 1.1; docs/handoffs/PROMPT_CABECALHO_1.1.md): mede, **sem
// mudar nada**, a altura de cada parte do topo — marca/selo/tema, escolha da abrangência, faixa, cartão de identificação,
// pílulas de cargo — e onde começa o primeiro número, a 360, 390, 768 e 1366 px, nos dois temas, ao vivo, no replay e
// na consulta. Lista também o que aparece repetido e o que parece botão e não é. Com Playwright/Chromium e **nenhum
// servidor** (página de `dist/`, WebSocket falso; a consulta usa a série de `testes/fixtures/consulta/3237`).
//   cd site && node testes/diag_cabecalho.mjs [--saida /tmp/uv-cabecalho] [--json /tmp/uv-cabecalho/medidas.json]
import { chromium } from 'playwright';
import { mkdirSync, readFileSync, writeFileSync, existsSync } from 'node:fs';
import { dirname, join, extname } from 'node:path';
import { fileURLToPath } from 'node:url';

const arg = (k, d) => { const i = process.argv.indexOf(`--${k}`); return i > 0 ? process.argv[i + 1] : d; };
const saida = arg('saida', '/tmp/uv-cabecalho');
const jsonSaida = arg('json', join(saida, 'medidas.json'));
mkdirSync(saida, { recursive: true });
const AQUI = dirname(fileURLToPath(import.meta.url));
const DIST = join(AQUI, '..', 'dist');
const FIX = join(AQUI, 'fixtures', 'consulta');
const ORIGEM = 'http://uv.teste';
const TIPOS = { '.js': 'text/javascript', '.css': 'text/css', '.svg': 'image/svg+xml', '.woff2': 'font/woff2', '.json': 'application/json' };
const INDEX = readFileSync(join(DIST, 'index.html'), 'utf8').replaceAll('{{versao}}', 'teste')
  .replace(/<meta name="uv-dados" content="[^"]*">/, '<meta name="uv-dados" content="">');
const PLEITOS = readFileSync(join(AQUI, '..', '..', 'deploy', 'oci', 'testes', 'fixtures', 'pleitos-2026-10-29.json'), 'utf8');

// ---------------------------------------------------------------- dados falsos (1º turno de 2026, SP)
const cands = (L) => L.map(([n, nome_urna, partido]) => ({ n, nome_urna, nome: nome_urna, partido, vices: [{ tp: 'v', nome_urna: `VICE DE ${nome_urna.split(' ')[0]}`, partido }] }));
const PRES = [[22, 'FLAVIO BOLSONARO', 'PL'], [13, 'LULA', 'PT'], [70, 'ESCRITOR AUGUSTO CURY', 'AVANTE'], [55, 'RONALDO CAIADO', 'PSD'],
  [14, 'RENAN SANTOS', 'MISSÃO'], [30, 'ROMEU ZEMA', 'NOVO'], [27, 'CONSTITUINTE EYMAEL', 'DC'], [80, 'SAMARA MARTINS', 'UP']];
const GOV = [[10, 'TARCISIO', 'REPUBLICANOS'], [40, 'TABATA AMARAL', 'PSB'], [50, 'PAULO PIRES', 'PSOL'], [30, 'MARIA DO NOVO', 'NOVO']];
const SEN = [[111, 'GUILHERME DERRITE', 'PL'], [131, 'MARINA SILVA', 'REDE'], [400, 'MARCIO FRANCA', 'PSB'], [555, 'CANDIDATO QUATRO', 'PSD']];
const PCT = { 1: [50.34, 41.5, 2.95, 2.39, 2.31, 0.29, 0.12, 0.1], 3: [52.1, 38.2, 6.3, 3.4], 5: [33.3, 31.2, 20.1, 15.4] };
const LIST = { 1: PRES, 3: GOV, 5: SEN };
const ELEIT = { br: 158745502, sp: 34400000 }, SEC = { br: 499248, sp: 101200 };
const NOMES = {
  pleito: '3220', ufs: { sp: 'SÃO PAULO', rj: 'RIO DE JANEIRO' }, municipios: {},
  eleicoes: [
    { codigo: 6257, turno: 1, cargos: [{ cargo: 1, vagas: 1, consulta: false, abrangencias: ['br', 'sp', 'rj'], circunscricoes: [{ abr: 'br', eleitorado: ELEIT.br, candidatos: cands(PRES) }] }] },
    { codigo: 6259, turno: 1, cargos: [
      { cargo: 3, vagas: 1, consulta: false, abrangencias: ['sp'], circunscricoes: [{ abr: 'sp', eleitorado: ELEIT.sp, candidatos: cands(GOV) }] },
      { cargo: 5, vagas: 2, consulta: false, abrangencias: ['sp'], circunscricoes: [{ abr: 'sp', eleitorado: ELEIT.sp, candidatos: cands(SEN) }] },
    ] },
  ],
};
const TOPICOS = ['br-c0001', 'sp-c0001', 'rj-c0001', 'sp-c0003', 'sp-c0005'];
function resultado(topico) {
  const [a, c] = topico.split('-'); const cargo = +c.slice(1);
  const L = LIST[cargo], P = PCT[cargo];
  const eleit = a === 'br' ? ELEIT.br : ELEIT.sp, total = a === 'br' ? SEC.br : SEC.sp, frac = 0.1823;
  const apurado = Math.round(eleit * frac), comp = Math.round(apurado * 0.79), vb = Math.round(comp * 0.017), vn = Math.round(comp * 0.031), val = comp - vb - vn;
  return {
    v: 1, eleicao: cargo === 1 ? 6257 : 6259, turno: 1, cargo, abr: a, circ: cargo === 1 ? 'br' : 'sp', seq: 91000,
    ultimo_bu: '2026-10-04T19:05:08-03:00', totalizado_em: '2026-10-04T19:05:09.000-03:00',
    encerrada: false, secoes: { total, apuradas: Math.round(total * frac) }, eleitorado: eleit, apurado, comparecimento: comp, vb, vn, vnt: 0, validos: val,
    legenda: 0, estado: 'apurando',
    cand: L.map(([n], i) => ({ n, v: Math.round((val * P[i]) / 100), pct: P[i], dvt: 'Válido', st: '', mat: null })),
  };
}
const ROT_REPLAY = '1º turno de 2026 (eleição de 04/10/2026) · em tempo real, no ritmo em que os BUs chegaram ao TSE';
const estado = (modo, agora) => ({
  tipo: 'estado', versao: 'teste', estado: 'acompanhando', encerrada: false, pleito: 3220, relogio_site: '04/10/2026 19:05:08',
  relogio_sistema: { agora, noite0: null, sistema0: null, acelerar: 1.0 },
  secoes: { catalogo: SEC.br, publicadas_cs: 91000, aplicadas: 91000, nao_instaladas: 0, pendentes: 0 }, ultimo_bu: { seq: 91000, site: null, sistema: null },
  pausa_429: { ativa: false, retomada_em: null, pausas: 0 }, conferencias: 0, alertas: {}, clientes: 3, ...(modo === 'replay' ? { simulacao: { rotulo: ROT_REPLAY } } : {}),
});
const PROXIMA = { eleicao: '2026-10-25', nome: '2º turno das Eleições Gerais 2026', inicio: '2026-10-25T17:00:00-03:00', fim: '2026-10-26T08:00:00-03:00' };
const playlist = (agora, modo) => ({
  agora, suspensao: null, proxima_suspensao: null, proxima_ao_vivo: { ...PROXIMA, ao_vivo: modo === 'aovivo' },
  no_ar: modo === 'replay' ? { item: '3220-cadencia', modo: 'cadencia', pleito: '3220' } : null,
  fila: [], opcoes: [], meu_voto: null, total_votos: 0, tocados: [],
});
const AGORA = { aovivo: '2026-10-25T18:00:00-03:00', replay: '2026-10-20T15:00:00-03:00', consulta: '2026-10-30T13:00:00-03:00' };

// ---------------------------------------------------------------- página
const nav = await chromium.launch(process.env.UV_CHROMIUM ? { executablePath: process.env.UV_CHROMIUM } : {});
async function abrir({ largura, tema, modo, recolhida }) {
  const movel = largura < 640;
  const ctx = await nav.newContext({ viewport: { width: largura, height: movel ? 844 : (largura < 1000 ? 1024 : 900) }, deviceScaleFactor: movel ? 2 : 1,
    isMobile: movel, hasTouch: movel, timezoneId: 'America/Sao_Paulo', locale: 'pt-BR', reducedMotion: 'reduce' });
  await ctx.clock.setFixedTime(new Date(AGORA[modo]));
  await ctx.addInitScript(([t, r]) => {
    try { localStorage.setItem('uv-tema', t); localStorage.setItem('uv-aviso-dia', '2099-01-01'); if (r) localStorage.setItem('uv-ident', 'recolhida'); } catch { /* sem armazenamento */ }
  }, [tema, recolhida]);
  if (modo === 'consulta') await ctx.addInitScript(() => { window.WebSocket = class { constructor() { } close() { } }; });
  await ctx.route('https://dados.urnavelox.com.br/**', (r) => r.abort());
  await ctx.route(`${ORIGEM}/**`, async (r) => {
    const p = new URL(r.request().url()).pathname;
    if (p === '/' || p === '/index.html') return r.fulfill({ status: 200, contentType: 'text/html; charset=utf-8', body: INDEX });
    if (p.startsWith('/static/')) {
      const f = join(DIST, p.slice(8));
      if (existsSync(f)) return r.fulfill({ status: 200, contentType: TIPOS[extname(f)] || 'application/octet-stream', body: readFileSync(f) });
    }
    if (p === '/api/v1/pleitos') return r.fulfill({ status: 200, contentType: 'application/json', body: PLEITOS });
    if (modo === 'consulta') {
      if (p === '/consulta/indice.json') return r.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ pleitos: [{ pleito: 3237 }] }) });
      if (p.startsWith('/consulta/')) {
        const f = join(FIX, p.slice(10));
        if (existsSync(f)) return r.fulfill({ status: 200, contentType: 'application/json', body: readFileSync(f) });
      }
    } else {
      if (p === '/api/v1/nomes') return r.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(NOMES) });
      if (p === '/api/v1/topicos') return r.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(TOPICOS.map((t) => ({ topico: t, arquivo: `${t}.json`, seq: 91000 }))) });
      if (p === '/api/v1/playlist') return r.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(playlist(AGORA[modo], modo)) });
    }
    return r.fulfill({ status: 404, contentType: 'application/json', body: '{"erro":"nao_encontrado"}' });
  });
  if (modo !== 'consulta') {
    await ctx.routeWebSocket(`${ORIGEM.replace('http', 'ws')}/ws`, (ws) => {
      ws.send(JSON.stringify(estado(modo, AGORA[modo])));
      ws.onMessage((txt) => {
        let m; try { m = JSON.parse(txt); } catch { return; }
        if (m.op !== 'inscrever') return;
        for (const t of m.topicos || []) {
          if (!TOPICOS.includes(t)) continue;
          const r = resultado(t);
          ws.send(JSON.stringify({ tipo: 'instantaneo', topico: t, seq: r.seq, secoes_apuradas: r.secoes.apuradas, ver: 9, t_bu: null, t_envio: 0, resultado: r }));
        }
      });
    });
  }
  const p = await ctx.newPage();
  const erros = [];
  p.on('pageerror', (e) => erros.push(e.message));
  await p.goto(`${ORIGEM}/${modo === 'consulta' ? '#/p/3237' : '#/sp/c0001'}`);
  await p.waitForSelector('.cand', { timeout: 15000 });
  await p.waitForTimeout(1800);
  if (await p.isVisible('#folha.aberta')) { await p.click('#folha-fechar'); await p.waitForTimeout(400); }
  return { ctx, p, erros };
}

/** No navegador: caixas das partes do topo, textos repetidos e o que parece botão sem ser. */
function medir() {
  const vis = (e) => { if (!e) return false; const cs = getComputedStyle(e); const b = e.getBoundingClientRect(); return cs.display !== 'none' && cs.visibility !== 'hidden' && +cs.opacity > 0.05 && b.width > 0 && b.height > 0; };
  const cx = (s) => { const e = document.querySelector(s); if (!vis(e)) return null; const b = e.getBoundingClientRect(); return { t: Math.round(b.top), h: Math.round(b.height), b: Math.round(b.bottom) }; };
  const txt = (s) => { const e = document.querySelector(s); return vis(e) ? e.textContent.replace(/\s+/g, ' ').trim() : null; };
  const partes = {
    topo: cx('header.topo'), marca: cx('.marca'), rota: cx('#rota'), selo: cx('#conexao'), tema: cx('#tema'),
    faixa: cx('#faixa'), ident: cx('#ident'), cargos: cx('#cargos'), corrida: cx('.corrida'), cand1: cx('.lista .cand'),
  };
  // primeiro número: o % do primeiro candidato (tinta dos dígitos)
  const pct = document.querySelector('.lista .cand .pct');
  partes.numero1 = pct && vis(pct) ? { t: Math.round(pct.getBoundingClientRect().top) } : null;
  const selRota = [...document.querySelectorAll('#rota .passo')].filter(vis).map((e) => e.tagName === 'SELECT' ? (e.selectedOptions[0]?.textContent || '') : e.textContent);
  const textos = {
    selo: txt('#conexao .txt'), seletorPleito: txt('#faixa .chip.pleito'), seletorPleitoTitulo: document.querySelector('#faixa .chip.pleito')?.title || null,
    identEleicao: txt('#ident-cartao .id-eleicao'), identTipo: txt('#ident-cartao .id-tipo'), identTroca: txt('#ident-cartao .id-troca'),
    identTurno: [...document.querySelectorAll('#ident-cartao .id-seg i')].map((i) => `${i.textContent}${i.classList.contains('sel') ? '*' : ''}`).join(' '),
    identLocal: txt('#ident-cartao .id-local'), identCargo: txt('#ident-cartao .id-cargo'),
    rota: selRota, cargoPilula: txt('#cargos [aria-selected="true"]'), pilulas: [...document.querySelectorAll('#cargos button')].map((b) => b.textContent),
    corridaTitulo: txt('.corrida h1'), corridaSub: txt('.corrida .sub'), faixaChips: [...document.querySelectorAll('#faixa > *')].filter(vis).map((c) => `${c.className.replace('chip ', '')}: ${c.textContent.replace(/\s+/g, ' ').trim().slice(0, 40)}`),
    titulo: document.title,
  };
  // parece botão e não é: caixa arredondada com borda ou fundo próprio, sem ser (nem estar dentro de) um controle
  const limite = (partes.corrida?.t ?? 400) + 4;
  const falsos = [];
  for (const e of document.querySelectorAll('header *, #faixa *, #faixa, #ident *, #cargos *, .corrida-cab *')) {
    if (!vis(e)) continue;
    const b = e.getBoundingClientRect();
    if (b.top > limite || b.width * b.height < 120) continue;
    const cs = getComputedStyle(e);
    const raio = parseFloat(cs.borderTopLeftRadius) || 0;
    const borda = parseFloat(cs.borderTopWidth) > 0 && !/rgba\(0, 0, 0, 0\)|transparent/.test(cs.borderTopColor);
    const fundo = !/rgba\(0, 0, 0, 0\)|transparent/.test(cs.backgroundColor) || cs.backgroundImage !== 'none';
    if (raio < 6 || !(borda || fundo)) continue;
    if (e.closest('button, a, select, input, [role="tab"], [role="button"]')) continue;
    if (e.querySelector('button, a, select, input')) continue;
    falsos.push({ el: `${e.tagName.toLowerCase()}.${[...e.classList].join('.')}`, texto: e.textContent.replace(/\s+/g, ' ').trim().slice(0, 40), caixa: [Math.round(b.left), Math.round(b.top), Math.round(b.width), Math.round(b.height)], cursor: cs.cursor });
  }
  // controles só com símbolo (sem texto que diga o que fazem)
  const cripticos = [...document.querySelectorAll('header button, #faixa button, #ident button')].filter(vis)
    .filter((b) => !/[a-zà-ú]{3}/i.test([...b.querySelectorAll('*')].filter(vis).map((x) => x.childNodes.length && [...x.childNodes].some((n) => n.nodeType === 3) ? x.textContent : '').join(' ') + [...b.childNodes].filter((n) => n.nodeType === 3).map((n) => n.textContent).join('')))
    .map((b) => ({ el: `${b.tagName.toLowerCase()}#${b.id || ''}.${[...b.classList].join('.')}`, visto: b.innerText.replace(/\s+/g, ' ').trim(), rotulo: b.getAttribute('aria-label') || b.title || '' }));
  return { W: innerWidth, H: innerHeight, rolagem: document.documentElement.scrollWidth, partes, textos, falsos, cripticos, cls: window.__uv?.cls ?? null };
}

const resultados = [];
for (const modo of ['aovivo', 'replay', 'consulta']) {
  for (const tema of ['escuro', 'claro']) {
    for (const largura of [360, 390, 768, 1366]) {
      const { ctx, p, erros } = await abrir({ largura, tema, modo });
      const m = await p.evaluate(medir);
      m.modo = modo; m.tema = tema; m.largura = largura; m.erros = erros;
      const nome = `${modo}-${tema}-${largura}`;
      await p.screenshot({ path: join(saida, `${nome}.png`) });
      if (tema === 'escuro' && (largura === 390 || largura === 1366)) {
        await p.evaluate(() => window.scrollTo(0, 700));
        await p.waitForTimeout(700);
        await p.screenshot({ path: join(saida, `${nome}-rolado.png`) });
        m.barra = await p.evaluate(() => { const b = document.querySelector('#ident-barra'); const r = b.getBoundingClientRect(); return { t: Math.round(r.top), h: Math.round(r.height), preso: document.querySelector('#ident').classList.contains('preso') }; });
        // a faixa e o topo continuam à vista quando a barra está presa?
        m.rolado = await p.evaluate(() => ['header.topo', '#faixa', '#ident-barra', '#cargos'].map((s) => { const r = document.querySelector(s).getBoundingClientRect(); return `${s} ${Math.round(r.top)}+${Math.round(r.height)}`; }));
      }
      resultados.push(m);
      await ctx.close();
      const P = m.partes;
      const h = (k) => P[k] ? P[k].h : '—';
      console.log(`${nome.padEnd(24)} topo ${h('topo')} · rota ${P.rota ? `${P.rota.h}@${P.rota.t}` : '—'} · faixa ${h('faixa')} · ident ${h('ident')} · cargos ${h('cargos')} · 1º número em y=${P.numero1?.t ?? '—'} (${P.numero1 ? Math.round((100 * P.numero1.t) / m.H) : '—'}% de ${m.H}) · CLS ${m.cls?.toFixed?.(4)}${erros.length ? ` · ERROS ${erros.join(' | ')}` : ''}`);
    }
  }
}
// recolhido (o botão ⌃), só para comparar
for (const largura of [390, 1366]) {
  const { ctx, p } = await abrir({ largura, tema: 'escuro', modo: 'aovivo', recolhida: true });
  const m = await p.evaluate(medir); m.modo = 'aovivo-recolhido'; m.tema = 'escuro'; m.largura = largura;
  await p.screenshot({ path: join(saida, `aovivo-recolhido-${largura}.png`) });
  resultados.push(m); await ctx.close();
  console.log(`aovivo-recolhido-${largura} ident ${m.partes.ident?.h} · 1º número em y=${m.partes.numero1?.t}`);
}
await nav.close();
writeFileSync(jsonSaida, JSON.stringify(resultados, null, 1));
console.log(`\nmedidas em ${jsonSaida}; capturas em ${saida}`);
