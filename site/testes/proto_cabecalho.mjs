// Protótipos do cabeçalho (melhoria do cabeçalho, etapa 1.1; docs/handoffs/PROMPT_CABECALHO_1.1.md): a página real de
// `dist/` com os dados falsos do `diag_cabecalho.mjs`, e o topo **remontado no navegador** (CSS e DOM injetados depois
// do carregamento; nada do `src/` muda) em três propostas. Só capturas: nada funciona de verdade nos protótipos.
//   cd site && node testes/proto_cabecalho.mjs [--saida /tmp/uv-cabecalho-proto]
//   P1 barra única (eleição · turno · local · cargo, com o selo do modo dentro); P2 o topo vira a identificação (a marca
//   encolhe, a eleição sobe para a linha da marca, local + pílulas de cargo numa 2ª linha presa); P3 trilha (uma linha
//   fina "Eleição › turno › local › cargo", uma folha só para escolher tudo).
import { chromium } from 'playwright';
import { mkdirSync, readFileSync, existsSync } from 'node:fs';
import { dirname, join, extname } from 'node:path';
import { fileURLToPath } from 'node:url';

const arg = (k, d) => { const i = process.argv.indexOf(`--${k}`); return i > 0 ? process.argv[i + 1] : d; };
const saida = arg('saida', '/tmp/uv-cabecalho-proto');
mkdirSync(saida, { recursive: true });
const AQUI = dirname(fileURLToPath(import.meta.url));
const DIST = join(AQUI, '..', 'dist');
const FIX = join(AQUI, 'fixtures', 'consulta');
const ORIGEM = 'http://uv.teste';
const TIPOS = { '.js': 'text/javascript', '.css': 'text/css', '.svg': 'image/svg+xml', '.woff2': 'font/woff2', '.json': 'application/json' };
const INDEX = readFileSync(join(DIST, 'index.html'), 'utf8').replaceAll('{{versao}}', 'teste')
  .replace(/<meta name="uv-dados" content="[^"]*">/, '<meta name="uv-dados" content="">');
const PLEITOS = readFileSync(join(AQUI, '..', '..', 'deploy', 'oci', 'testes', 'fixtures', 'pleitos-2026-10-29.json'), 'utf8');

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
  return { v: 1, eleicao: cargo === 1 ? 6257 : 6259, turno: 1, cargo, abr: a, circ: cargo === 1 ? 'br' : 'sp', seq: 91000,
    ultimo_bu: '2026-10-04T19:05:08-03:00', totalizado_em: '2026-10-04T19:05:09.000-03:00', encerrada: false,
    secoes: { total, apuradas: Math.round(total * frac) }, eleitorado: eleit, apurado, comparecimento: comp, vb, vn, vnt: 0, validos: val, legenda: 0, estado: 'apurando',
    cand: L.map(([n], i) => ({ n, v: Math.round((val * P[i]) / 100), pct: P[i], dvt: 'Válido', st: '', mat: null })) };
}
const estado = (agora) => ({ tipo: 'estado', versao: 'teste', estado: 'acompanhando', encerrada: false, pleito: 3220, relogio_site: '04/10/2026 19:05:08',
  relogio_sistema: { agora, noite0: null, sistema0: null, acelerar: 1.0 }, secoes: { catalogo: SEC.br, publicadas_cs: 91000, aplicadas: 91000, nao_instaladas: 0, pendentes: 0 },
  ultimo_bu: { seq: 91000, site: null, sistema: null }, pausa_429: { ativa: false, retomada_em: null, pausas: 0 }, conferencias: 0, alertas: {}, clientes: 3 });
const PROXIMA = { eleicao: '2026-10-25', nome: '2º turno das Eleições Gerais 2026', inicio: '2026-10-25T17:00:00-03:00', fim: '2026-10-26T08:00:00-03:00' };
const AGORA = { aovivo: '2026-10-25T18:00:00-03:00', consulta: '2026-10-30T13:00:00-03:00' };

const nav = await chromium.launch(process.env.UV_CHROMIUM ? { executablePath: process.env.UV_CHROMIUM } : {});
async function abrir({ largura, tema, modo }) {
  const movel = largura < 640;
  const ctx = await nav.newContext({ viewport: { width: largura, height: movel ? 844 : 900 }, deviceScaleFactor: movel ? 2 : 1,
    isMobile: movel, hasTouch: movel, timezoneId: 'America/Sao_Paulo', locale: 'pt-BR', reducedMotion: 'reduce' });
  await ctx.clock.setFixedTime(new Date(AGORA[modo]));
  await ctx.addInitScript((t) => { try { localStorage.setItem('uv-tema', t); localStorage.setItem('uv-aviso-dia', '2099-01-01'); } catch { /* */ } }, tema);
  if (modo === 'consulta') await ctx.addInitScript(() => { window.WebSocket = class { constructor() { } close() { } }; });
  await ctx.route('https://dados.urnavelox.com.br/**', (r) => r.abort());
  await ctx.route(`${ORIGEM}/**`, async (r) => {
    const p = new URL(r.request().url()).pathname;
    if (p === '/' || p === '/index.html') return r.fulfill({ status: 200, contentType: 'text/html; charset=utf-8', body: INDEX });
    if (p.startsWith('/static/')) { const f = join(DIST, p.slice(8)); if (existsSync(f)) return r.fulfill({ status: 200, contentType: TIPOS[extname(f)] || 'application/octet-stream', body: readFileSync(f) }); }
    if (p === '/api/v1/pleitos') return r.fulfill({ status: 200, contentType: 'application/json', body: PLEITOS });
    if (modo === 'consulta') {
      if (p === '/consulta/indice.json') return r.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ pleitos: [{ pleito: 3237 }] }) });
      if (p.startsWith('/consulta/')) { const f = join(FIX, p.slice(10)); if (existsSync(f)) return r.fulfill({ status: 200, contentType: 'application/json', body: readFileSync(f) }); }
    } else {
      if (p === '/api/v1/nomes') return r.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(NOMES) });
      if (p === '/api/v1/topicos') return r.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(TOPICOS.map((t) => ({ topico: t, arquivo: `${t}.json`, seq: 91000 }))) });
      if (p === '/api/v1/playlist') return r.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ agora: AGORA[modo], suspensao: null, proxima_suspensao: null, proxima_ao_vivo: { ...PROXIMA, ao_vivo: true }, no_ar: null, fila: [], opcoes: [], meu_voto: null, total_votos: 0, tocados: [] }) });
    }
    return r.fulfill({ status: 404, contentType: 'application/json', body: '{"erro":"nao_encontrado"}' });
  });
  if (modo !== 'consulta') {
    await ctx.routeWebSocket(`${ORIGEM.replace('http', 'ws')}/ws`, (ws) => {
      ws.send(JSON.stringify(estado(AGORA[modo])));
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
  await p.goto(`${ORIGEM}/${modo === 'consulta' ? '#/p/3237' : '#/sp/c0001'}`);
  await p.waitForSelector('.cand', { timeout: 15000 });
  await p.waitForTimeout(1800);
  if (await p.isVisible('#folha.aberta')) { await p.click('#folha-fechar'); await p.waitForTimeout(400); }
  return { ctx, p };
}

const CSS = `
html.px #rota, html.px #ident, html.px #cargos, html.px #faixa .chip.pleito { display: none !important; }
html.px .topo { display: flex; align-items: center; gap: 12px; height: 52px; }
html.px #tema { margin-left: auto; }
.px-barra #conexao, html.px3 .topo #conexao { width: 104px !important; height: 28px; padding: 0 9px; gap: 6px; font-size: 11px !important; letter-spacing: 0.06em; flex: none; }
.px-curto { display: none; }
@media (max-width: 640px) { .px-longo { display: none; } .px-curto { display: inline; } }
.px-barra { position: sticky; top: 0; z-index: 15; margin: 0 clamp(12px, 3vw, 36px); border-radius: 18px; border: 1px solid var(--borda);
  background: color-mix(in oklab, var(--fundo-2) 82%, transparent); backdrop-filter: blur(14px) saturate(1.3); -webkit-backdrop-filter: blur(14px) saturate(1.3);
  box-shadow: var(--sombra); }
.px-seg { all: unset; box-sizing: border-box; cursor: pointer; display: grid; grid-template-columns: minmax(0, 1fr) auto; grid-template-rows: 13px 22px;
  column-gap: 6px; align-content: center; padding: 0 12px; min-width: 0; height: 46px; border-radius: 12px; color: var(--texto); }
.px-seg:hover { background: var(--vidro-forte); }
.px-seg small { grid-column: 1 / -1; font-size: 10.5px; font-weight: 650; letter-spacing: 0.08em; text-transform: uppercase; color: var(--texto-3); }
.px-seg b { font-size: 16px; font-weight: 720; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; min-width: 0; }
.px-seg b .t { font-weight: 450; color: var(--texto-2); }
.px-seg i { font-style: normal; font-size: 11px; color: var(--texto-2); align-self: center; }
.px-div { width: 1px; align-self: stretch; margin: 10px 0; background: var(--borda); }
.px-pino { width: 13px; height: 13px; fill: #7da2ff; vertical-align: -1px; margin-right: 3px; }
.px-barra #conexao .pl { display: none; }
.px-turnos { display: inline-flex; border: 1px solid var(--borda); border-radius: 999px; overflow: hidden; height: 24px; align-self: center; }
.px-turnos span { display: grid; place-items: center; padding: 0 9px; font-size: 13px; font-weight: 700; color: var(--texto-2); }
.px-turnos span.sel { background: linear-gradient(135deg, #4fc3ff55, #8c6bff66); color: var(--texto); }
.px-turnos span.nao { opacity: 0.35; text-decoration: line-through; }
/* ---- P1: barra única */
.p1 { display: flex; align-items: center; gap: 4px; padding: 4px 6px; height: 56px; }
.p1 .px-modo { flex: none; padding: 0 6px; }
.p1 .px-el { flex: 1.4 1 0; } .p1 .px-tu { flex: none; display: flex; flex-direction: column; justify-content: center; align-items: flex-start; gap: 0; } .p1 .px-lo { flex: 1 1 0; } .p1 .px-ca { flex: 1 1 0; }
@media (max-width: 640px) {
  .p1 { flex-wrap: wrap; height: auto; padding: 4px; row-gap: 0; }
  .p1 .px-div { display: none; }
  .p1 .px-el { flex: 1 1 0; }
  .p1 .px-lo { flex: 1 0 55%; } .p1 .px-ca { flex: 1 0 35%; }
  .p1 .px-seg small { display: none; } .p1 .px-seg { grid-template-rows: 22px; height: 46px; }
  .p1 .px-lo, .p1 .px-ca { border-top: 1px solid var(--borda); border-radius: 0 0 12px 12px; }
}
/* ---- P2: o topo vira a identificação */
html.px2 .topo { display: none; }
.p2 { margin-top: 10px; display: grid; grid-template-columns: auto minmax(0, 1fr) auto auto; grid-template-rows: 52px 44px; align-items: center; column-gap: 8px; padding: 0 8px 0 14px; }
.p2 .px-logo { font-stretch: 125%; font-weight: 800; font-size: 17px; color: var(--texto); white-space: nowrap; text-decoration: none; }
.p2 .px-logo span { font-weight: 300; color: var(--texto-2); }
.p2 .px-logo .uv { display: none; }
.p2 .px-el { grid-column: 2; grid-row: 1; grid-template-rows: 22px; height: 40px; }
.p2 .px-el small { display: none; }
.p2 .px-lin2 { grid-column: 1 / -1; grid-row: 2; display: flex; align-items: center; gap: 6px; border-top: 1px solid var(--borda); margin: 0 -8px 0 -14px; padding: 0 10px; height: 44px; overflow: hidden; }
.p2 .px-lo { all: unset; cursor: pointer; flex: none; display: inline-flex; align-items: center; gap: 4px; height: 32px; padding: 0 12px; border-radius: 999px; border: 1px solid var(--borda); background: var(--vidro); font-weight: 650; font-size: 14px; color: var(--texto); }
.p2 .px-pils { display: flex; gap: 2px; overflow-x: auto; scrollbar-width: none; min-width: 0; mask-image: linear-gradient(90deg, #000 85%, transparent); }
.p2 .px-pils span { flex: none; height: 32px; display: grid; place-items: center; padding: 0 12px; border-radius: 999px; font-size: 14px; color: var(--texto-2); cursor: pointer; }
.p2 .px-pils span.sel { background: var(--vidro-forte); border: 1px solid var(--borda); color: var(--texto); font-weight: 650; }
.p2 #tema { width: 32px; height: 32px; }
@media (max-width: 640px) { .p2 .px-logo .lg { display: none; } .p2 .px-logo .uv { display: inline; } .p2 { padding-left: 12px; column-gap: 4px; } .p2 .px-seg { padding: 0 6px; } }
/* ---- P3: trilha */
.p3 { display: flex; align-items: center; gap: 2px; height: 44px; padding: 0 6px 0 12px; border-radius: 999px; }
.p3 .px-pt { flex: none; width: 10px; height: 10px; border-radius: 50%; margin-right: 8px; }
.p3 .px-pt.ao_vivo { background: #e0213a; box-shadow: 0 0 0 4px #e0213a33; } .p3 .px-pt.consulta, .p3 .px-pt.replay { background: #ffb020; box-shadow: 0 0 0 4px #ffb02033; }
.p3 .px-cr.lo, .p3 .px-cr.ca { flex: 0 1 auto; } .p3 .px-cr.el { flex: 0 3 auto; }
.p3 .px-cr { all: unset; cursor: pointer; white-space: nowrap; font-size: 15px; font-weight: 650; color: var(--texto); padding: 4px 6px; border-radius: 8px;
  text-decoration: underline dotted var(--texto-3); text-underline-offset: 5px; min-width: 0; overflow: hidden; text-overflow: ellipsis; }
.p3 .px-cr.leve { font-weight: 450; color: var(--texto-2); }
.p3 .px-gt { flex: none; color: var(--texto-3); font-size: 13px; }
.p3 .px-esc { flex: none; margin-left: auto; height: 32px; padding: 0 12px; border-radius: 999px; border: 1px solid var(--borda); background: var(--vidro-forte); font-size: 13px; font-weight: 650; color: var(--texto); display: inline-flex; align-items: center; gap: 4px; }
html.px3 #faixa { height: 34px; }
@media (max-width: 640px) {
  .px-barra #conexao, html.px3 .topo #conexao { width: 90px !important; padding: 0 7px; letter-spacing: 0.02em; font-size: 10.5px !important; }
  .p1 .px-tu { padding: 0 4px; } .px-turnos span { padding: 0 7px; } }
@media (max-width: 640px) { .p3 { border-radius: 16px; padding-left: 10px; } .p3 .px-pt { margin-right: 4px; } .p3 .px-cr { font-size: 14px; padding: 4px 3px; } .p3 .px-esc { padding: 0 10px; } .p3 .px-esc .longo { display: none; } .p3 .opc { display: none; } }
`;

/** No navegador: remonta o topo na proposta n, com os textos que a página já mostra. */
function aplicar([n, css]) {
  const $ = (s) => document.querySelector(s);
  const tx = (s) => ($(s)?.textContent || '').trim();
  const d = { el: tx('#ident-cartao .id-eleicao'), tipo: tx('#ident-cartao .id-tipo'), local: tx('#ident-cartao .id-local'), cargo: tx('#ident-cartao .id-cargo'),
    turno: [...document.querySelectorAll('#ident-cartao .id-seg i')].findIndex((i) => i.classList.contains('sel')) + 1,
    pils: [...document.querySelectorAll('#cargos button')].map((b) => [b.textContent, b.getAttribute('aria-selected') === 'true']),
    modo: (() => { const t = tx('#conexao .txt').toUpperCase(); return /CONSULTA/.test(t) ? 'consulta' : /REPLAY/.test(t) ? 'replay' : /AO VIVO/.test(t) ? 'ao_vivo' : $('#conexao').dataset.modo; })() };
  const suplementar = /suplementar/i.test(d.tipo);
  const st = document.createElement('style'); st.textContent = css; document.head.append(st);
  document.documentElement.classList.add('px', `px${n}`);
  const PINO = '<svg class="px-pino" viewBox="0 0 24 24"><path d="M12 2a7 7 0 0 0-7 7c0 5.2 7 13 7 13s7-7.8 7-13a7 7 0 0 0-7-7zm0 9.5A2.5 2.5 0 1 1 12 6.5a2.5 2.5 0 0 1 0 5z"/></svg>';
  const ano = d.el.replace(/\D+/g, ''), tipoC = suplementar ? 'Suplementar' : d.tipo;
  const curto = `${tipoC} ${ano}`;
  const turnos = suplementar ? '<span class="sel">turno único</span>' : `<span class="${d.turno === 1 ? 'sel' : ''}">1º</span><span class="${d.turno === 2 ? 'sel' : ''}">2º</span>`;
  const b = document.createElement('div');
  b.className = `px-barra p${n}`;
  if (n === 1) {
    b.innerHTML = `<span class="px-modo"></span>
      <button class="px-seg px-el"><small>Eleição</small><b><span class="px-longo">${d.el} <span class="t">· ${d.tipo}</span></span><span class="px-curto">${curto}</span></b><i>▾</i></button><span class="px-div"></span>
      ${suplementar ? '' : `<button class="px-seg px-tu"><small>Turno</small><span class="px-turnos">${turnos}</span></button><span class="px-div"></span>`}
      <button class="px-seg px-lo"><small>Onde</small><b>${PINO}${d.local}</b><i>▾</i></button><span class="px-div"></span>
      <button class="px-seg px-ca"><small>Cargo</small><b>${d.cargo}</b><i>▾</i></button>`;
    b.querySelector('.px-modo').append($('#conexao'));
    $('.faixa').before(b);
  } else if (n === 2) {
    b.innerHTML = `<a class="px-logo"><span class="lg">Urna <span>Velox</span></span><span class="uv">U<span>V</span></span></a>
      <button class="px-seg px-el"><b><span class="px-longo">${d.el} <span class="t">· ${d.tipo} · ${suplementar ? 'turno único' : `${d.turno}º turno`}</span></span><span class="px-curto">${curto}${suplementar ? '' : ` <span class="t">· ${d.turno}º</span>`}</span></b><i>▾</i></button>
      <span class="px-modo"></span><span class="px-tema"></span>
      <div class="px-lin2"><button class="px-lo">${PINO}${d.local} <i>▾</i></button><div class="px-pils">${d.pils.map(([t, s]) => `<span class="${s ? 'sel' : ''}">${t}</span>`).join('')}</div></div>`;
    b.querySelector('.px-modo').append($('#conexao'));
    b.querySelector('.px-tema').append($('#tema'));
    $('.faixa').before(b);
  } else {
    const rot = { ao_vivo: 'ao vivo', replay: 'replay', consulta: 'consulta' }[d.modo] || '';
    b.innerHTML = `<span class="px-pt ${d.modo}" title="${rot}"></span>
      <button class="px-cr el"><span class="px-longo">${d.el} · ${d.tipo}</span><span class="px-curto">${curto}${suplementar ? '' : ` · ${d.turno}º`}</span></button><span class="px-gt">›</span>
      ${suplementar ? '' : `<button class="px-cr leve opc">${d.turno}º turno</button><span class="px-gt opc">›</span>`}
      <button class="px-cr lo">${d.local}</button><span class="px-gt">›</span>
      <button class="px-cr ca">${d.cargo}</button>
      <span class="px-esc">⇅<span class="longo">&nbsp;trocar</span></span>`;
    // o selo continua no topo, menor; a trilha presa logo abaixo da faixa
    $('#conexao').classList.add('px-selo-p');
    $('.faixa').after(b);
  }
  // a faixa: só o que muda durante a noite (o seletor de pleito saiu)
  return d;
}

const casos = [];
for (const n of [1, 2, 3]) {
  for (const [modo, tema, largura] of [['aovivo', 'escuro', 390], ['aovivo', 'escuro', 1366], ['consulta', 'claro', 390], ['aovivo', 'claro', 360]]) {
    const { ctx, p } = await abrir({ largura, tema, modo });
    await p.evaluate(aplicar, [n, CSS]);
    await p.waitForTimeout(600);
    const nome = `P${n}-${modo}-${tema}-${largura}`;
    await p.screenshot({ path: join(saida, `${nome}.png`) });
    const m = await p.evaluate(() => {
      const r = (s) => { const e = document.querySelector(s); const b = e?.getBoundingClientRect(); return b ? Math.round(b.bottom) : null; };
      const pct = document.querySelector('.lista .cand .pct').getBoundingClientRect().top;
      const sobra = [...document.querySelectorAll('.px-barra *')].filter((e) => { const b = e.getBoundingClientRect(); return b.width && (b.right > innerWidth + 0.5 || b.left < -0.5); }).length;
      return { fimCab: Math.max(r('.px-barra'), r('#faixa')), numero1: Math.round(pct), rolagem: document.documentElement.scrollWidth, fora: sobra };
    });
    if (largura === 390 && modo === 'aovivo') {
      await p.evaluate(() => window.scrollTo(0, 700)); await p.waitForTimeout(500);
      await p.screenshot({ path: join(saida, `${nome}-rolado.png`) });
    }
    console.log(`${nome.padEnd(30)} cabeçalho até y=${m.fimCab} · 1º número em y=${m.numero1} · largura ${m.rolagem}${m.fora ? ` · ${m.fora} fora da tela` : ''}`);
    casos.push({ nome, ...m });
    await ctx.close();
  }
}
await nav.close();
console.log(`capturas em ${saida}`);
