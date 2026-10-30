// M-025 (cabeçalho da página, etapa 1.1; proposta P2 escolhida pelo Luciano em 30/10/2026, com os ajustes dele): sem
// servidor, a página de `dist/` com dados falsos (ao vivo e replay: 1º turno de 2026, Presidente em SP; consulta: a série
// do suplementar de RR, pleito 3237), de 360 a 1920 px, nos dois temas. Confere:
//  * o topo (`header.topo`) só com o que é geral, independente da eleição: a marca e o tema (nada de selo nem de eleição);
//  * a barra da eleição (`#eleicao`), presa no alto ao rolar, com a identificação inteira da eleição ("Eleições Gerais
//    2026", "Eleição suplementar · Roraima"), o turno e a **data do pleito**, o selo do modo (CONSULTA inteiro, B-084),
//    o local e os cargos; os cargos que não cabem vão para o botão "…", e o selecionado está sempre à vista;
//  * tocar na eleição abre o painel "Escolha a eleição"; escolher um cargo no "…" troca de cargo;
//  * nada fora da tela nem por cima de outra coisa, sem rolagem horizontal, CLS < 0,01 (também ao rolar);
//  * sem a lista dos pleitos, o nome e a data vêm do resultado (fallback).
//   cd site && node testes/cabecalho.mjs [--fotos /tmp/uv-cabecalho-teste]   (UV_MOTOR=webkit para o WebKit)
import { chromium, webkit } from 'playwright';
import { mkdirSync, readFileSync, existsSync } from 'node:fs';
import { dirname, join, extname } from 'node:path';
import { fileURLToPath } from 'node:url';

const arg = (k, d) => { const i = process.argv.indexOf(`--${k}`); return i > 0 ? process.argv[i + 1] : d; };
const fotos = arg('fotos', '/tmp/uv-cabecalho-teste');
mkdirSync(fotos, { recursive: true });
const MOTOR = process.env.UV_MOTOR === 'webkit' ? webkit : chromium;
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
const PRES = [[22, 'FLAVIO BOLSONARO', 'PL'], [13, 'LULA', 'PT'], [70, 'ESCRITOR AUGUSTO CURY', 'AVANTE'], [55, 'RONALDO CAIADO', 'PSD']];
const GOV = [[10, 'TARCISIO', 'REPUBLICANOS'], [40, 'TABATA AMARAL', 'PSB'], [50, 'PAULO PIRES', 'PSOL']];
const SEN = [[111, 'GUILHERME DERRITE', 'PL'], [131, 'MARINA SILVA', 'REDE'], [400, 'MARCIO FRANCA', 'PSB']];
const PCT = { 1: [50.34, 41.5, 5.77, 2.39], 3: [52.1, 38.2, 9.7], 5: [40.3, 35.2, 24.5] };
const LIST = { 1: PRES, 3: GOV, 5: SEN };
const ELEIT = { br: 158745502, sp: 34400000, rj: 12800000 }, SEC = { br: 499248, sp: 101200, rj: 34000 };
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
  const eleit = ELEIT[a], total = SEC[a], frac = 0.1823;
  const apurado = Math.round(eleit * frac), comp = Math.round(apurado * 0.79), vb = Math.round(comp * 0.017), vn = Math.round(comp * 0.031), val = comp - vb - vn;
  return {
    v: 1, eleicao: cargo === 1 ? 6257 : 6259, turno: 1, cargo, abr: a, circ: cargo === 1 ? 'br' : a, seq: 91000,
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
// o que cada caso deve mostrar
const ESPERADO = {
  aovivo: { nome: 'Eleições Gerais 2026', data: '04/10/2026', turno: '1º turno', selo: 'AO VIVO', local: 'São Paulo', cargo: 'Presidente', cargos: ['Presidente', 'Governador', 'Senador'] },
  replay: { nome: 'Eleições Gerais 2026', data: '04/10/2026', turno: '1º turno', selo: 'REPLAY', local: 'São Paulo', cargo: 'Presidente', cargos: ['Presidente', 'Governador', 'Senador'] },
  consulta: { nome: 'Eleição suplementar · Roraima', data: '21/06/2026', turno: '1º turno', selo: 'CONSULTA', local: 'Roraima', cargo: 'Governador', cargos: ['Governador'] },
};

// ---------------------------------------------------------------- página
const nav = await MOTOR.launch(process.env.UV_CHROMIUM && MOTOR === chromium ? { executablePath: process.env.UV_CHROMIUM } : {});
async function abrir({ largura, tema, modo, semPleitos = false }) {
  const movel = largura < 640;
  const ctx = await nav.newContext({ viewport: { width: largura, height: movel ? 844 : (largura < 1000 ? 1024 : 900) }, deviceScaleFactor: movel ? 2 : 1,
    isMobile: movel && MOTOR === chromium, hasTouch: movel, timezoneId: 'America/Sao_Paulo', locale: 'pt-BR', reducedMotion: 'reduce' });
  await ctx.clock.setFixedTime(new Date(AGORA[modo]));
  await ctx.addInitScript((t) => { try { localStorage.setItem('uv-tema', t); localStorage.setItem('uv-aviso-dia', '2099-01-01'); } catch { /* sem armazenamento */ } }, tema);
  if (modo === 'consulta') await ctx.addInitScript(() => { window.WebSocket = class { constructor() { } close() { } }; });
  await ctx.route('https://dados.urnavelox.com.br/**', (r) => r.abort());
  await ctx.route(`${ORIGEM}/**`, async (r) => {
    const p = new URL(r.request().url()).pathname;
    if (p === '/' || p === '/index.html') return r.fulfill({ status: 200, contentType: 'text/html; charset=utf-8', body: INDEX });
    if (p.startsWith('/static/')) {
      const f = join(DIST, p.slice(8));
      if (existsSync(f)) return r.fulfill({ status: 200, contentType: TIPOS[extname(f)] || 'application/octet-stream', body: readFileSync(f) });
    }
    if (p === '/api/v1/pleitos' && !semPleitos) return r.fulfill({ status: 200, contentType: 'application/json', body: PLEITOS });
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

/** No navegador: o que o cabeçalho mostra e onde. */
function medir() {
  const vis = (e) => { if (!e) return false; const cs = getComputedStyle(e); const b = e.getBoundingClientRect(); return cs.display !== 'none' && cs.visibility !== 'hidden' && +cs.opacity > 0.05 && b.width > 0 && b.height > 0; };
  const caixa = (e) => { const b = e.getBoundingClientRect(); return { l: b.left, r: b.right, t: b.top, b: b.bottom, w: b.width, h: b.height }; };
  const txt = (s) => { const e = document.querySelector(s); return vis(e) ? e.textContent.replace(/\s+/g, ' ').trim() : null; };
  const bar = document.querySelector('#eleicao');
  const topo = document.querySelector('header.topo');
  // o topo: só o geral (marca e tema)
  const noTopo = topo ? [...topo.querySelectorAll('a, button, select, nav, [id]')].filter(vis).map((e) => e.id || e.className.split(' ')[0]) : [];
  const nome = bar?.querySelector('.el-nome');
  // texto cortado? (reticência ou maior que a caixa)
  const cortado = (e) => !!e && e.scrollWidth > e.clientWidth + 1;
  const pilulas = bar ? [...bar.querySelectorAll('#cargos [role="tab"]')] : [];
  const visiveis = pilulas.filter(vis).map((b) => b.textContent.trim());
  const sel = pilulas.find((b) => b.getAttribute('aria-selected') === 'true');
  const mais = bar?.querySelector('.el-mais');
  const maisOpc = mais && vis(mais) ? [...mais.querySelectorAll('option')].filter((o) => o.value).map((o) => o.textContent.trim()) : [];
  // controles da barra: nada fora da tela nem por cima de outro
  const ctl = bar ? [bar.querySelector('.el-pleito'), bar.querySelector('#conexao'), bar.querySelector('.el-local'), ...pilulas, mais].filter(vis) : [];
  const fora = [], sobre = [];
  for (const e of bar ? [...bar.querySelectorAll('*')].filter(vis) : []) { const b = caixa(e); if (b.l < -0.5 || b.r > innerWidth + 0.5) fora.push(e.className || e.tagName); }
  for (let i = 0; i < ctl.length; i++) for (let j = i + 1; j < ctl.length; j++) {
    const a = caixa(ctl[i]), b = caixa(ctl[j]);
    if (Math.min(a.r, b.r) - Math.max(a.l, b.l) > 1 && Math.min(a.b, b.b) - Math.max(a.t, b.t) > 1) sobre.push(`${ctl[i].className} × ${ctl[j].className}`);
  }
  const pct = document.querySelector('.lista .cand .pct');
  const fimCab = Math.max(...['header.topo', '#eleicao', '#faixa'].map((s) => document.querySelector(s)).filter(vis).map((e) => e.getBoundingClientRect().bottom));
  const selo = document.querySelector('#conexao');
  return {
    temBarra: vis(bar), noTopo, seloNaBarra: !!bar?.contains(selo), selo: txt('#conexao .txt'), seloCortado: cortado(selo?.querySelector('.txt')),
    seta: (() => { const s = bar?.querySelector('.el-pleito > .el-seta'); if (!vis(s)) return null; const b = s.getBoundingClientRect(); return { w: Math.round(b.width), h: Math.round(b.height), fonte: parseFloat(getComputedStyle(s).fontSize) }; })(),
    nome: nome?.textContent.trim() ?? null, nomeCortado: cortado(nome), nomeTitulo: bar?.querySelector('.el-pleito')?.title || '', nomeVis: vis(nome),
    turno: txt('#eleicao .el-turno'), data: txt('#eleicao .el-data'), dataCortada: cortado(bar?.querySelector('.el-sub')),
    local: txt('#eleicao .el-local-txt'), localCortado: cortado(bar?.querySelector('.el-local-txt')),
    cargoSel: sel?.textContent.trim() ?? null, cargoSelVis: vis(sel), visiveis, maisOpc, maisVis: vis(mais),
    antigos: ['#ident-cartao', '#ident-barra', '#rota', '#faixa .chip.pleito'].filter((s) => vis(document.querySelector(s))),
    fora, sobre, rolagem: document.documentElement.scrollWidth, W: innerWidth,
    barraTopo: bar ? Math.round(bar.getBoundingClientRect().top) : null, topoFim: topo ? Math.round(topo.getBoundingClientRect().bottom) : null,
    fimCab: Math.round(fimCab), numero1: pct ? Math.round(pct.getBoundingClientRect().top) : null, cls: window.__uv?.cls ?? null,
    clsFontes: (window.__uv?.clsFontes || []).filter((f) => f.v > 0.0005).map((f) => `${f.v.toFixed(4)}@${f.t}: ${f.de.join(' | ')}`),
  };
}

const falhas = [];
const conferir = (caso, ok, msg) => { if (!ok) falhas.push(`${caso}: ${msg}`); };
function avaliar(caso, m, esp, { rolado = false, largura }) {
  conferir(caso, m.temBarra, 'sem a barra da eleição (#eleicao)');
  if (!m.temBarra) return;
  if (!rolado) conferir(caso, m.noTopo.every((x) => /marca|tema/.test(x)) && m.noTopo.length >= 2, `o topo deve ter só a marca e o tema: ${m.noTopo.join(', ')}`);
  conferir(caso, m.seloNaBarra, 'o selo do modo não está na barra da eleição');
  conferir(caso, m.selo?.startsWith(esp.selo) && !m.seloCortado, `selo "${m.selo}" (esperado ${esp.selo}, inteiro)`);
  conferir(caso, m.nome === esp.nome && m.nomeVis && !m.nomeCortado, `eleição "${m.nome}"${m.nomeCortado ? ' cortada' : ''} (esperado "${esp.nome}", inteira)`);
  // M-026: a seta da escolha da eleição bem visível (no celular era um "▾" de 12 px solto)
  // (sem a lista dos pleitos não há o que escolher e a seta fica escondida, de propósito)
  if (!esp.semLista) conferir(caso, !!m.seta && m.seta.w >= 24 && m.seta.h >= 20 && m.seta.fonte >= 13, `seta da eleição pequena demais ou escondida (${JSON.stringify(m.seta)}; mínimo 24×20 px, fonte 13 px)`);
  conferir(caso, m.nomeTitulo.includes(esp.nome), `o título do botão da eleição não diz "${esp.nome}": "${m.nomeTitulo}"`);
  conferir(caso, m.turno === esp.turno, `turno "${m.turno}" (esperado "${esp.turno}")`);
  conferir(caso, !!m.data && m.data.includes(esp.data) && !m.dataCortada, `data "${m.data}" (esperado ${esp.data}, inteira)`);
  conferir(caso, m.local === esp.local && !m.localCortado, `local "${m.local}" (esperado "${esp.local}")`);
  conferir(caso, m.cargoSel === esp.cargo && m.cargoSelVis, `cargo selecionado "${m.cargoSel}" à vista (esperado ${esp.cargo})`);
  const todos = [...m.visiveis, ...m.maisOpc];
  conferir(caso, esp.cargos.every((c) => todos.includes(c)) && todos.length === esp.cargos.length, `cargos à vista ${m.visiveis.join('/')} + "…" ${m.maisOpc.join('/')} ≠ ${esp.cargos.join('/')}`);
  conferir(caso, m.maisVis === (m.maisOpc.length > 0), '"…" à vista sem cargos escondidos (ou o contrário)');
  conferir(caso, !m.antigos.length, `restos do cabeçalho antigo à vista: ${m.antigos.join(', ')}`);
  conferir(caso, !m.fora.length, `fora da tela: ${[...new Set(m.fora)].slice(0, 4).join(', ')}`);
  conferir(caso, !m.sobre.length, `por cima: ${m.sobre.slice(0, 3).join('; ')}`);
  conferir(caso, m.rolagem <= m.W, `rolagem horizontal (${m.rolagem} > ${m.W})`);
  conferir(caso, m.cls != null && m.cls < 0.01, `CLS ${m.cls}`);
  if (rolado) conferir(caso, m.barraTopo != null && m.barraTopo >= 0 && m.barraTopo <= 12 && m.topoFim <= 0, `rolada, a barra deve ficar presa no alto (topo da barra ${m.barraTopo}, fim do topo ${m.topoFim})`);
  else if (largura < 640) conferir(caso, m.fimCab <= 200, `no celular o cabeçalho deve acabar até y = 200 (hoje ${m.fimCab})`);
}

const LARGURAS = [360, 390, 768, 1366, 1920];
for (const modo of ['aovivo', 'replay', 'consulta']) {
  for (const tema of ['escuro', 'claro']) {
    for (const largura of LARGURAS) {
      const caso = `${modo}-${tema}-${largura}`;
      const { ctx, p, erros } = await abrir({ largura, tema, modo });
      const m = await p.evaluate(medir);
      avaliar(caso, m, ESPERADO[modo], { largura });
      if ([360, 390, 1366].includes(largura)) await p.screenshot({ path: join(fotos, `${caso}.png`) });
      // rolada: a barra fica presa no alto, inteira
      await p.evaluate(() => window.scrollTo(0, 700));
      await p.waitForTimeout(500);
      const r = await p.evaluate(medir);
      avaliar(`${caso} rolado`, r, ESPERADO[modo], { largura, rolado: true });
      if (tema === 'escuro' && [390, 1366].includes(largura)) await p.screenshot({ path: join(fotos, `${caso}-rolado.png`) });
      conferir(caso, !erros.length, `erros na página: ${erros.join(' | ')}`);
      if (process.env.UV_CLS_FONTES && r.clsFontes.length) console.log(`  CLS ${caso}:\n   ${r.clsFontes.join('\n   ')}`);
      console.log(`${caso.padEnd(24)} cabeçalho até y=${m.fimCab} · 1º número em y=${m.numero1} · "${m.nome}" · ${m.turno} · ${m.data} · ${m.selo} · ${m.local} · cargos ${m.visiveis.join('/')}${m.maisOpc.length ? ` + … ${m.maisOpc.join('/')}` : ''} · CLS ${m.cls?.toFixed?.(4)}/${r.cls?.toFixed?.(4)}`);
      await ctx.close();
    }
  }
}

// tocar na eleição abre a escolha da eleição; o "…" (ou a pílula) troca de cargo; o selecionado continua à vista
for (const largura of [360, 1366]) {
  const caso = `interação-${largura}`;
  const { ctx, p } = await abrir({ largura, tema: 'escuro', modo: 'aovivo' });
  p.setDefaultTimeout(4000);
  try {
  await p.click('#eleicao .el-pleito');
  await p.waitForTimeout(600);
  conferir(caso, await p.isVisible('#pleitos.aberta'), 'tocar na eleição não abriu o painel "Escolha a eleição"');
  await p.keyboard.press('Escape');
  await p.waitForTimeout(500);
  const noMais = await p.evaluate(() => [...document.querySelectorAll('#eleicao .el-mais option')].some((o) => o.textContent.trim() === 'Senador'));
  if (noMais) await p.selectOption('#eleicao .el-mais select', { label: 'Senador' });
  else await p.click('#eleicao #cargos [role="tab"]:has-text("Senador")');
  await p.waitForTimeout(1200);
  const m = await p.evaluate(medir);
  conferir(caso, (await p.evaluate(() => location.hash)) === '#/sp/c0005', `o cargo não mudou para Senador (${await p.evaluate(() => location.hash)})`);
  avaliar(`${caso} Senador`, m, { ...ESPERADO.aovivo, cargo: 'Senador' }, { largura });
  if (largura === 360) await p.screenshot({ path: join(fotos, `${caso}-senador.png`) });
  console.log(`${caso}: Senador ${noMais ? 'pelo "…"' : 'pela pílula'} · cargos ${m.visiveis.join('/')}${m.maisOpc.length ? ` + … ${m.maisOpc.join('/')}` : ''}`);
  // o local: o seletor leva ao Rio de Janeiro (Presidente)
  await p.selectOption('#eleicao .el-local select', 'rj');
  await p.waitForTimeout(1200);
  const l = await p.evaluate(medir);
  conferir(caso, (await p.evaluate(() => location.hash)) === '#/rj/c0001' && l.local === 'Rio de Janeiro', `o local não mudou para o Rio de Janeiro (${await p.evaluate(() => location.hash)}, "${l.local}")`);
  } catch (e) { conferir(caso, false, `interação: ${e.message.split('\n')[0]}`); }
  await ctx.close();
}

// sem a lista dos pleitos (serviço sem --pleitos): o nome e a data vêm do resultado
{
  const caso = 'sem-pleitos-390';
  const { ctx, p } = await abrir({ largura: 390, tema: 'escuro', modo: 'aovivo', semPleitos: true });
  const m = await p.evaluate(medir);
  avaliar(caso, m, { ...ESPERADO.aovivo, semLista: true }, { largura: 390 });
  conferir(caso, m.seta === null, 'sem a lista dos pleitos a seta da eleição deveria ficar escondida');
  console.log(`${caso}: "${m.nome}" · ${m.turno} · ${m.data}`);
  await ctx.close();
}

await nav.close();
console.log(`capturas em ${fotos}`);
if (falhas.length) {
  console.log(`\nFALHOU (${falhas.length}):\n${[...new Set(falhas)].slice(0, 60).join('\n')}`);
  process.exit(1);
}
console.log('ok');
