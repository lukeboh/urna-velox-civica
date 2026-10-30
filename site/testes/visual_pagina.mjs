// Revisão visual da página (B-029 a B-046, docs/BUGS.md): com Playwright/Chromium e **nenhum servidor** (a página de
// `dist/` servida por rotas do Playwright, WebSocket falso), abre cenários com dados de tamanho real em várias larguras
// e nos dois temas e confere a geometria do que está desenhado — nada de olho: cada conferência mede caixas.
//   cd site && node testes/visual_pagina.mjs [--saida /tmp/uv-visual] [--so B-032]
// Cenários: Presidente/Brasil apurando (8 candidatos, nomes longos, replay a 500× com a chegada parada) e Prefeito de
// um município pequeno com o eleito definido; modos "Urna Velox" e "Ambos" (comparação com o TSE).
// Conferências (cada uma leva o número do bug):
//   B-029/B-030 separadores do hodômetro ("." de milhar, "," decimal, ":" das horas) na mesma linha de base dos dígitos;
//   B-031 o sinal da diferença colado no número; B-032 o selo da posição no TSE sem cobrir os números do Urna Velox;
//   B-036 o rótulo "UV − TSE" numa linha só; B-043 o nome do candidato à vista ao lado da pílula "eleito às …";
//   B-034 a caixa dos eleitores que faltam (e o número dela) dentro do cartão "Apuração";
//   B-035 título e subtítulo da corrida inteiros a partir de 961 px; B-044 o horário "mais provável" inteiro no cartão
//   da previsão; B-045 nenhum cartão passa da margem direita da página e nenhum texto passa da área de conteúdo do
//   cartão; B-047 nenhum hodômetro desenha fora do cartão (o "faltam" ainda sem valor mostrava zeros fora da caixa);
//   B-033 o ícone ☰ do letreiro centrado no botão e com tamanho de ícone; B-037 a pílula "Termos de uso" opaca (o
//   texto que passa por baixo não se mistura com o dela); B-038 "pausado" fora da área dos itens do letreiro;
//   B-039 no celular, sem selo, o controle da comparação logo abaixo do subtítulo (e o selo, quando aparece, sem
//   cobrir o título); B-040 com a barra de identificação presa no alto, a faixa acima dela não mostra o conteúdo que
//   rola por baixo (duas capturas da faixa em rolagens diferentes saem iguais); B-041 o número de "Seções apuradas"
//   alinhado ao rótulo e o de "BUs/min" centrado no chip.
import { chromium } from 'playwright';
import { mkdirSync, readFileSync, existsSync } from 'node:fs';
import { dirname, join, extname } from 'node:path';
import { fileURLToPath } from 'node:url';

const arg = (k, d) => { const i = process.argv.indexOf(`--${k}`); return i > 0 ? process.argv[i + 1] : d; };
const saida = arg('saida', '/tmp/uv-visual');
const so = arg('so', null);
mkdirSync(saida, { recursive: true });
const DIST = join(dirname(fileURLToPath(import.meta.url)), '..', 'dist');
const ORIGEM = 'http://uv.teste';
const TIPOS = { '.js': 'text/javascript', '.css': 'text/css', '.svg': 'image/svg+xml', '.woff2': 'font/woff2' };
const INDEX = readFileSync(join(DIST, 'index.html'), 'utf8').replaceAll('{{versao}}', 'teste')
  .replace(/<meta name="uv-dados" content="[^"]*">/, '<meta name="uv-dados" content="">');
const falhas = [];
let oks = 0;
const conferir = (bug, ok, o) => {
  if (so && bug !== so) return;
  if (ok) { oks++; return; }
  console.log(`FALHA ${bug} ${o}`);
  falhas.push(`${bug} ${o}`);
};

// ---------------------------------------------------------------- dados falsos
const cands = (L) => L.map(([n, nome_urna, partido]) => ({ n, nome_urna, nome: nome_urna, partido, vices: [{ tp: 'v', nome_urna: `VICE DE ${nome_urna.split(' ')[0]}`, partido }] }));
const PRES = [[22, 'FLAVIO BOLSONARO', 'PL'], [13, 'LULA', 'PT'], [70, 'ESCRITOR AUGUSTO CURY', 'AVANTE'], [55, 'RONALDO CAIADO', 'PSD'],
  [14, 'RENAN SANTOS', 'MISSÃO'], [30, 'ROMEU ZEMA', 'NOVO'], [27, 'CONSTITUINTE EYMAEL', 'DC'], [80, 'SAMARA MARTINS', 'UP']];
const PCT_P = [50.34, 41.5, 2.95, 2.39, 2.31, 0.29, 0.12, 0.1];
const PREF = [[12, 'PADEIRO', 'PDT'], [20, 'ROBERTINHO DA SAÚDE', 'PODE'], [15, 'JOÃO DO POVO', 'MDB']];
const PCT_M = [49.79, 48.9, 1.31];
const ELEIT_BR = 158745502, SEC_BR = 499248, ELEIT_M = 11829, SEC_M = 41;
const NOMES = {
  pleito: '3220', ufs: { ac: 'ACRE', sp: 'SÃO PAULO' }, municipios: { ac01120: 'BUJARI' },
  eleicoes: [
    { codigo: 544, turno: 1, cargos: [{ cargo: 1, vagas: 1, consulta: false, abrangencias: ['br', 'ac', 'sp'], circunscricoes: [{ abr: 'br', eleitorado: ELEIT_BR, candidatos: cands(PRES) }] }] },
    { codigo: 619, turno: 1, cargos: [{ cargo: 11, vagas: 1, consulta: false, abrangencias: ['ac01120'], circunscricoes: [{ abr: 'ac01120', eleitorado: ELEIT_M, candidatos: cands(PREF) }] }] },
  ],
};
const TOPICOS = ['br-c0001', 'ac-c0001', 'sp-c0001', 'ac01120-c0011'];
function resultado(topico) {
  const municipal = topico.startsWith('ac01120');
  const L = municipal ? PREF : PRES, P = municipal ? PCT_M : PCT_P;
  const eleit = municipal ? ELEIT_M : ELEIT_BR, total = municipal ? SEC_M : SEC_BR, frac = municipal ? 1 : 0.1823;
  const apurado = Math.round(eleit * frac), comp = Math.round(apurado * 0.79), vb = Math.round(comp * 0.017), vn = Math.round(comp * 0.031), val = comp - vb - vn;
  return {
    v: 1, eleicao: municipal ? 619 : 544, turno: 1, cargo: municipal ? 11 : 1, abr: topico.split('-')[0], circ: municipal ? 'ac01120' : 'br', seq: 275920,
    // replay: o último BU é da noite da eleição, a totalização é de hoje (a tela mostra "de 04/10")
    ultimo_bu: municipal ? '2024-10-06T18:47:30-03:00' : '2026-10-04T19:05:08-03:00', totalizado_em: '2026-10-22T10:06:29.000-03:00',
    encerrada: false, secoes: { total, apuradas: Math.round(total * frac) }, eleitorado: eleit, apurado, comparecimento: comp, vb, vn, vnt: 0, validos: val,
    legenda: 0, estado: municipal ? 'matematicamente_eleito' : 'apurando',
    cand: L.map(([n], i) => ({ n, v: Math.round((val * P[i]) / 100), pct: P[i], dvt: 'Válido', st: '',
      mat: municipal && i === 0 ? { estado: 'eleito', em: '2024-10-06 16:33:51' } : null })),
  };
}
const estado = () => ({
  tipo: 'estado', versao: 'teste', estado: 'acompanhando', encerrada: false, pleito: 3220, relogio_site: '04/10/2026 19:05:08',
  relogio_sistema: { agora: '2026-10-22T10:06:29.000-03:00', noite0: '2026-10-04T17:00:00-03:00', sistema0: '2026-10-22T09:51:20.000-03:00', acelerar: 500.0 },
  secoes: { catalogo: SEC_BR, publicadas_cs: 275920, aplicadas: 275920, nao_instaladas: 0, pendentes: 0 }, ultimo_bu: { seq: 275920, site: null, sistema: null },
  pausa_429: { ativa: false, retomada_em: null, pausas: 0 }, conferencias: 0, alertas: {}, clientes: 3, simulacao: { rotulo: '1º turno de 2026 · replay' },
});
const T0 = Date.UTC(2026, 9, 22, 9, 51, 20) / 1000;
const previsao = (topico) => ({
  tipo: 'previsao', instantaneo: true, topico, abrangencia: topico.split('-')[0], cargo: 1, circunscricao: topico === 'br-c0001', estimativa: true,
  instante: '2026-10-22 10:06:20', situacao: 'suspensa', parada: { desde: '2026-10-22 10:06:20', minutos: 1 }, base: { apurado: 0.55, secoes: 0.55, aptos_min: 1e6 },
  mat: { tipo: 'matematicamente_2t', p10: '2026-10-22 10:06:40', p50: '2026-10-22 10:07:30', p90: '2026-10-22 13:20:00', incerta: true, prob_antes_fim: 0.6, apurado_p50: 0.8 },
  fim99: { p10: '2026-10-22 10:07:00', p50: '2026-10-22 10:08:00', p90: '2026-10-22 13:00:00', incerta: true }, fim100: null,
  chegou: { fim99: null, fim100: null }, definida: null, confirmacao: null,
  historico: Array.from({ length: 30 }, (_, i) => [T0 + i * 30, T0 + 600 + i * 20, T0 + 900 + i * 20, T0 + 1300 + i * 25]),
});
const tse = (topico) => {
  const r = resultado(topico);
  return { tipo: 'tse', instantaneo: true, topico: `tse:${topico}`, ver: 3, estado: 'ok', motivo: null, arquivo: `${topico}-u.json`, buscado: '22/10/2026 10:06:10', novo_em: '22/10/2026 10:05:40',
    dados: { geracao: '04/10/2026 18:40:15', totalizacao: '04/10/2026 18:40:13', andamento: 'p', md: r.estado === 'matematicamente_eleito' ? 'e' : 'n', finalizada: false,
      secoes: { total: r.secoes.total, totalizadas: r.secoes.apuradas, nao_instaladas: 0 }, eleitorado: r.eleitorado, comparecimento: r.comparecimento, abstencao: 0,
      vb: r.vb, vn: r.vn, vnt: 0, validos: r.validos,
      cand: r.cand.map((c, i) => ({ n: c.n, v: Math.round(c.v * 0.937), pct: c.pct, pos: i + 1, e: r.estado === 'matematicamente_eleito' && i === 0, st: '', dvt: 'Válido' })) } };
};

// ---------------------------------------------------------------- página
const nav = await chromium.launch();
async function abrir({ largura, tema, hash, comparar }) {
  const movel = largura < 640;
  const ctx = await nav.newContext({ viewport: { width: largura, height: movel ? 844 : 900 }, deviceScaleFactor: movel ? 2 : 1, isMobile: movel, hasTouch: movel,
    timezoneId: 'America/Sao_Paulo', locale: 'pt-BR', reducedMotion: 'reduce' });
  await ctx.addInitScript(([c, t]) => {
    try { localStorage.setItem('uv-tema', t); localStorage.setItem('uv-comparar', c); localStorage.setItem('uv-aviso-dia', '2099-01-01'); } catch { /* sem armazenamento */ }
  }, [comparar, tema]);
  await ctx.route(`${ORIGEM}/**`, async (r) => {
    const p = new URL(r.request().url()).pathname;
    if (p === '/' || p === '/index.html') return r.fulfill({ status: 200, contentType: 'text/html; charset=utf-8', body: INDEX });
    if (p.startsWith('/static/')) {
      const f = join(DIST, p.slice(8));
      if (existsSync(f)) return r.fulfill({ status: 200, contentType: TIPOS[extname(f)] || 'application/octet-stream', body: readFileSync(f) });
    }
    if (p === '/api/v1/nomes') return r.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(NOMES) });
    if (p === '/api/v1/topicos') return r.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(TOPICOS.map((t) => ({ topico: t, arquivo: `${t}.json`, seq: 275920 }))) });
    return r.fulfill({ status: 404, contentType: 'application/json', body: '{"erro":"nao_encontrado"}' });
  });
  await ctx.routeWebSocket(`${ORIGEM.replace('http', 'ws')}/ws`, (ws) => {
    ws.send(JSON.stringify(estado()));
    ws.onMessage((txt) => {
      let m;
      try { m = JSON.parse(txt); } catch { return; }
      if (m.op !== 'inscrever') return;
      for (const t of m.topicos || []) {
        if (t.startsWith('tse:')) { if (TOPICOS.includes(t.slice(4))) ws.send(JSON.stringify(tse(t.slice(4)))); continue; }
        if (!TOPICOS.includes(t)) continue;
        const r = resultado(t);
        ws.send(JSON.stringify({ tipo: 'instantaneo', topico: t, seq: r.seq, secoes_apuradas: r.secoes.apuradas, ver: 9, t_bu: null, t_envio: 0, resultado: r }));
        ws.send(JSON.stringify(previsao(t)));
      }
    });
  });
  const p = await ctx.newPage();
  await p.goto(`${ORIGEM}/${hash}`);
  await p.waitForSelector('.cand', { timeout: 10000 });
  await p.waitForTimeout(1500);
  // o aviso do replay (1ª visita do dia) cobre a lista: fecha
  if (await p.isVisible('#folha.aberta')) { await p.click('#folha-fechar'); await p.waitForTimeout(400); }
  return { ctx, p };
}

// ---------------------------------------------------------------- medidas (no navegador)
/** Tudo o que as conferências precisam, medido de uma vez. */
function medir() {
  const R = (e) => { const b = e.getBoundingClientRect(); return { l: b.left, r: b.right, t: b.top, b: b.bottom, w: b.width, h: b.height }; };
  const visivel = (e) => { if (!e) return false; const cs = getComputedStyle(e); const b = e.getBoundingClientRect(); return cs.display !== 'none' && cs.visibility !== 'hidden' && +cs.opacity > 0.05 && b.width > 0 && b.height > 0; };
  /** caixa do que aparece de um hodômetro: dígitos, separadores e sufixo com opacidade (os zeros à esquerda são invisíveis) */
  const tinta = (e) => {
    if (!e || !visivel(e)) return null;
    const ps = [...e.querySelectorAll('.dg, .sp, .suf')].filter((x) => visivel(x) && x.textContent.trim()).map(R);
    if (!ps.length) return R(e);
    return { l: Math.min(...ps.map((x) => x.l)), r: Math.max(...ps.map((x) => x.r)), t: Math.min(...ps.map((x) => x.t)), b: Math.max(...ps.map((x) => x.b)) };
  };
  // linha de base: um marcador de altura zero no fim do texto fica com o fundo na linha de base
  const base = (e) => { const m = document.createElement('span'); m.style.cssText = 'display:inline-block;width:0;height:0'; e.append(m); const y = m.getBoundingClientRect().bottom; m.remove(); return y; };
  const W = document.documentElement.clientWidth;
  const h1 = document.querySelector('.corrida h1'), sub = document.querySelector('.corrida .sub');
  const out = { W, rolagem: document.documentElement.scrollWidth, cands: [], seps: [], cartoes: [], faltam: null, cab: null, prev: null };
  for (const li of document.querySelectorAll('.cand')) {
    const q = (s) => li.querySelector(s);
    const b = q('.nome b'), pil = q('.pilula');
    const c = { n: li.dataset.n, nome: b.textContent, nomeW: b.clientWidth, nomeTotal: b.scrollWidth, pilula: visivel(pil) ? pil.textContent : '' };
    const partes = { nome: visivel(b) ? R(b) : null, pilula: visivel(pil) ? R(pil) : null, pct: tinta(q('.pct .hod')), qtd: tinta(q('.qtd .hod')),
      pos: visivel(q('.pos-tse')) && q('.pos-tse').textContent ? R(q('.pos-tse')) : null, tsePct: tinta(q('.tse-pct .hod')), tseQtd: tinta(q('.tse-qtd .hod')),
      rotDif: visivel(q('.dif-rot')) ? R(q('.dif-rot')) : null, sinal: visivel(q('.sinal')) && q('.sinal').textContent ? R(q('.sinal')) : null, dif: tinta(q('.dif-val .hod')) };
    c.partes = partes;
    const rot = q('.dif-rot');
    c.rotLinhas = visivel(rot) ? Math.round(rot.getBoundingClientRect().height / parseFloat(getComputedStyle(rot).lineHeight || getComputedStyle(rot).fontSize)) : 0;
    c.rotAlt = visivel(rot) ? rot.getBoundingClientRect().height : 0;
    c.rotFonte = visivel(rot) ? parseFloat(getComputedStyle(rot).fontSize) : 0;
    c.difFonte = visivel(q('.dif-val')) ? parseFloat(getComputedStyle(q('.dif-val')).fontSize) : 0;
    out.cands.push(c);
  }
  // separadores de todos os hodômetros à vista: linha de base do separador × a do dígito mais próximo (à esquerda)
  for (const h of document.querySelectorAll('.hod')) {
    if (!visivel(h) || h.closest('.trilho, .hist, .folha')) continue;
    for (const s of h.querySelectorAll('.sp')) {
      if (!visivel(s) || s.classList.contains('vazio')) continue;
      let d = s.previousElementSibling;
      while (d && !d.classList.contains('dg')) d = d.previousElementSibling;
      if (!d) continue;
      const col = d.querySelector('.col');
      // o dígito à vista na coluna: o filho cuja caixa está dentro da janela do dígito
      const jan = d.getBoundingClientRect();
      const fil = [...col.children].find((x) => { const b = x.getBoundingClientRect(); return Math.abs(b.top - jan.top) < 2; });
      if (!fil) continue;
      const onde = h.closest('[class]')?.className;
      out.seps.push({ ch: s.textContent, dif: base(s) - base(fil), fonte: parseFloat(getComputedStyle(s).fontSize), onde: `${h.parentElement.className || h.parentElement.tagName}`.slice(0, 30) });
    }
  }
  const painel = document.querySelector('.painel');
  const pp = R(painel), cs = getComputedStyle(painel);
  const limite = pp.r - parseFloat(cs.paddingRight);
  for (const c of document.querySelectorAll('.painel .corrida, .painel .lado > section')) {
    // o que está escrito no cartão (texto e hodômetros) fica dentro da área de conteúdo dele
    const cc = R(c), ccs = getComputedStyle(c), dentro = { l: cc.l + parseFloat(ccs.paddingLeft) - 1, r: cc.r - parseFloat(ccs.paddingRight) + 1 };
    const fora = [];
    for (const h of c.querySelectorAll('.hod')) { const t = tinta(h); if (t && (t.l < dentro.l || t.r > dentro.r)) fora.push(`hodômetro em ${h.parentElement.className} (${Math.round(t.l)}–${Math.round(t.r)})`); }
    for (const e of c.querySelectorAll('*')) {
      if (e.closest('.hod, svg') || e.children.length || !e.textContent.trim() || !visivel(e)) continue;
      const r = document.createRange(); r.selectNodeContents(e); const b = r.getBoundingClientRect();
      // texto cortado por reticência / overflow: o que importa é a caixa do elemento
      const cx = getComputedStyle(e).overflow !== 'visible' ? R(e) : { l: b.left, r: b.right };
      if (cx.l < dentro.l || cx.r > dentro.r) fora.push(`"${e.textContent.trim().slice(0, 40)}" (${Math.round(cx.l)}–${Math.round(cx.r)})`);
    }
    out.cartoes.push({ nome: c.className, r: cc.r, limite, dentro, fora });
  }
  const ap = document.querySelector('.apuracao'), fa = document.querySelector('.faltam');
  if (ap && fa) out.faltam = { cartao: R(ap), caixa: R(fa), num: tinta(fa.querySelector('.hod')) };
  const fA = document.querySelector('.f-a');
  out.cab = { h1: h1 && [h1.textContent, h1.scrollWidth, h1.clientWidth], sub: sub && [sub.textContent, sub.scrollWidth, sub.clientWidth],
    fA: fA && visivel(fA) ? [fA.textContent, fA.scrollWidth, fA.clientWidth] : null };
  const pv = document.querySelector('.previsao'), ph = pv?.querySelector('.pv-hod');
  if (pv && ph) {
    const caixa = R(pv), padR = parseFloat(getComputedStyle(pv).paddingRight);
    out.prev = { cartao: caixa, limite: caixa.r - padR, hod: R(ph.querySelector('.hod')), txt: R(pv.querySelector('.pv-txt')) };
  }
  // B-033: o ícone do botão do histórico
  const hb = document.querySelector('#feed-hist'), hi = hb?.querySelector('svg');
  out.menu = hb ? { botao: R(hb), icone: hi ? R(hi) : null } : null;
  // B-037: fundo da pílula dos termos (alfa da cor computada)
  const ta = document.querySelector('.termos-caixa'); // desde a 0.9.19 (M-013) a pílula é a caixa, não o link
  if (ta && visivel(ta)) {
    const c = getComputedStyle(ta).backgroundColor, m = /\/\s*([\d.]+)\s*\)$/.exec(c) || /rgba\([^)]*,\s*([\d.]+)\)$/.exec(c);
    out.termos = { cor: c, alfa: m ? +m[1] : (/^rgb\(/.test(c) || /^color\([^/]*\)$/.test(c) ? 1 : 0) };
  }
  // B-039: no celular, distância do subtítulo (texto) ao controle da comparação; selo × título
  const cab = document.querySelector('.corrida-cab'), fontes = document.querySelector('.fontes'), selo = document.querySelector('.corrida .selo');
  if (cab && fontes && sub) {
    const r = document.createRange(); r.selectNodeContents(sub); const sb = r.getBoundingClientRect();
    out.cab39 = { subB: sb.bottom, subVis: +getComputedStyle(sub).opacity, fontesT: R(fontes).t, selo: selo?.classList.contains('ligado') ? R(selo) : null, h1: R(h1), sub: { l: sb.left, r: sb.right, t: sb.top, b: sb.bottom } };
  }
  // B-041: "Seções apuradas" (número × rótulo) e o chip de BUs/min (conteúdo centrado)
  const rotSec = [...document.querySelectorAll('.numeros .rot')].find((e) => /Seções/.test(e.textContent));
  const valSec = rotSec?.nextElementSibling?.querySelector('.hod');
  if (rotSec && valSec) out.secoes = { rot: R(rotSec).l, num: tinta(valSec)?.l };
  const chip = document.querySelector('.faixa .chip.ritmo');
  if (chip && visivel(chip)) {
    const t = tinta(chip.querySelector('.hod')), sp = [...chip.querySelectorAll('span')].filter((e) => !e.closest('.hod') && e.textContent.trim()).pop();
    const r = document.createRange(); r.selectNodeContents(sp); const tb = r.getBoundingClientRect(), cb = R(chip), cs2 = getComputedStyle(chip);
    out.chip = { esq: t.l - (cb.l + parseFloat(cs2.borderLeftWidth)), dir: (cb.r - parseFloat(cs2.borderRightWidth)) - tb.right };
  }
  return out;
}

const cruza = (a, b, tol = 0.5) => a && b && a.l < b.r - tol && b.l < a.r - tol && a.t < b.b - tol && b.t < a.b - tol;

function conferirTudo(m, tag, { largura, comparar }) {
  // B-043: o nome à vista ao lado da pílula (pelo menos o nome inteiro, ou 56 px dele)
  for (const c of m.cands) if (c.pilula) conferir('B-043', c.nomeW >= Math.min(c.nomeTotal, 56) - 0.5, `${tag}: nome de ${c.n} some ao lado da pílula "${c.pilula}" (${c.nomeW} de ${c.nomeTotal} px à vista)`);
  // B-032: nada da comparação por cima de outra coisa da linha
  if (comparar === 'ambos') {
    for (const c of m.cands) {
      const ps = Object.entries(c.partes).filter(([, v]) => v);
      for (let i = 0; i < ps.length; i++) for (let j = i + 1; j < ps.length; j++) {
        const [na, a] = ps[i], [nb, b] = ps[j];
        if (!cruza(a, b)) continue;
        const bug = [na, nb].includes('pos') ? 'B-032' : [na, nb].includes('sinal') || [na, nb].includes('rotDif') ? 'B-031' : 'B-032';
        conferir(bug, false, `${tag}: candidato ${c.n}: "${na}" cruza "${nb}" (${Math.round(a.l)}–${Math.round(a.r)} × ${Math.round(b.l)}–${Math.round(b.r)} px)`);
      }
      // B-031: o sinal colado no número (no máximo um quarto da fonte entre os dois)
      const { sinal, dif } = c.partes;
      if (sinal && dif) conferir('B-031', dif.l - sinal.r <= c.difFonte * 0.25 + 0.5 && Math.abs((sinal.t + sinal.b) / 2 - (dif.t + dif.b) / 2) <= c.difFonte * 0.3,
        `${tag}: candidato ${c.n}: "+" a ${Math.round(dif.l - sinal.r)} px do número (fonte ${c.difFonte} px)`);
      // B-036: o rótulo "UV − TSE" numa linha
      if (c.rotAlt) conferir('B-036', c.rotAlt <= c.rotFonte * 1.6, `${tag}: candidato ${c.n}: rótulo "UV − TSE" em mais de uma linha (${Math.round(c.rotAlt)} px de altura, fonte ${c.rotFonte} px)`);
    }
  }
  // B-029/B-030: separadores na linha de base dos dígitos (tolerância de 4% da fonte)
  const piores = new Map();
  for (const s of m.seps) {
    const bug = s.ch === ':' ? 'B-030' : 'B-029';
    if (Math.abs(s.dif) > Math.max(0.5, s.fonte * 0.04)) {
      const k = `${bug}${s.ch}`;
      if (!piores.has(k) || Math.abs(piores.get(k).dif) < Math.abs(s.dif)) piores.set(k, s);
    }
  }
  for (const [k, s] of piores) conferir(k.slice(0, 5), false, `${tag}: "${s.ch}" fora da linha de base dos dígitos (${s.dif.toFixed(1)} px abaixo, fonte ${s.fonte} px, em ${s.onde})`);
  if (!m.seps.length) conferir('B-029', false, `${tag}: nenhum separador medido`);
  // B-045: cartões dentro da margem direita da página; nada de rolagem horizontal
  conferir('B-045', m.rolagem <= m.W, `${tag}: rolagem horizontal (${m.rolagem} > ${m.W} px)`);
  for (const c of m.cartoes) {
    conferir('B-045', c.r <= c.limite + 0.5, `${tag}: cartão "${c.nome}" passa da margem direita (${Math.round(c.r)} > ${Math.round(c.limite)} px)`);
    for (const f of c.fora) conferir(f.startsWith('hodômetro') ? 'B-047' : 'B-045', false, `${tag}: no cartão "${c.nome}" (conteúdo ${Math.round(c.dentro.l)}–${Math.round(c.dentro.r)} px), fora: ${f}`);
  }
  // B-034: a caixa dos eleitores que faltam e o número dela dentro do cartão "Apuração"
  if (m.faltam) {
    const { cartao, caixa, num } = m.faltam;
    conferir('B-034', caixa.b <= cartao.b - 8 && caixa.r <= cartao.r - 8, `${tag}: caixa "Eleitores que faltam" fora do cartão (embaixo ${Math.round(caixa.b)} × ${Math.round(cartao.b)} px)`);
    if (num) conferir('B-034', num.b <= caixa.b - 2 && num.r <= caixa.r, `${tag}: número dos eleitores que faltam cortado pela caixa (embaixo ${Math.round(num.b)} × ${Math.round(caixa.b)} px)`);
  }
  // B-035: título e subtítulo inteiros a partir de 961 px (abaixo disso o título quebra em duas linhas)
  if (largura >= 961 && m.cab) {
    for (const [k, v] of Object.entries(m.cab)) if (v) conferir('B-035', v[1] <= v[2] + 1, `${tag}: ${k} "${v[0].slice(0, 40)}" truncado (${v[1]} > ${v[2]} px)`);
  }
  // B-044: o horário "mais provável" inteiro dentro do cartão da previsão, sem encostar no texto ao lado
  if (m.prev) {
    const { hod, limite, txt } = m.prev;
    conferir('B-044', hod.r <= limite + 0.5, `${tag}: horário da previsão passa da borda do cartão (${Math.round(hod.r)} > ${Math.round(limite)} px)`);
    conferir('B-044', !cruza(hod, txt), `${tag}: horário da previsão por cima do texto da faixa`);
  }
}

function conferirExtras(m, tag, { largura }) {
  if (m.menu) {
    const { botao: b, icone: i } = m.menu;
    conferir('B-033', !!i && Math.abs((i.l + i.r) / 2 - (b.l + b.r) / 2) <= 0.75 && Math.abs((i.t + i.b) / 2 - (b.t + b.b) / 2) <= 0.75 && i.w >= 16,
      `${tag}: ícone ☰ do letreiro ${i ? `fora do centro ou pequeno (${(i.l + i.r) / 2 - (b.l + b.r) / 2}, ${(i.t + i.b) / 2 - (b.t + b.b) / 2} px; ${Math.round(i.w)} px)` : 'é um caractere de texto (sem desenho próprio)'}`);
  }
  if (m.termos) conferir('B-037', m.termos.alfa >= 0.9, `${tag}: pílula dos termos translúcida (${m.termos.cor})`);
  if (largura < 640 && m.cab39) {
    const c = m.cab39;
    if (!c.selo) conferir('B-039', c.fontesT - c.subB <= 24, `${tag}: ${Math.round(c.fontesT - c.subB)} px vazios entre o subtítulo e o controle da comparação`);
    else conferir('B-039', c.selo.h >= 24 && !cruza(c.selo, c.h1) && (c.subVis < 0.05 || !cruza(c.selo, c.sub)), `${tag}: o selo cobre o título ou o subtítulo, ou some (${Math.round(c.selo.h)} px de altura)`);
  }
  if (m.secoes?.num != null) conferir('B-041', Math.abs(m.secoes.num - m.secoes.rot) <= 1, `${tag}: número de "Seções apuradas" ${Math.round(m.secoes.num - m.secoes.rot)} px à direita do rótulo`);
  if (m.chip) conferir('B-041', Math.abs(m.chip.esq - m.chip.dir) <= 2, `${tag}: conteúdo do chip de BUs/min fora do centro (${Math.round(m.chip.esq)} px à esquerda, ${Math.round(m.chip.dir)} à direita)`);
}

/** B-038: com o letreiro pausado, "pausado" fora da área dos itens. B-040: a faixa acima da barra presa no alto. */
async function conferirRolagem(p, tag) {
  const pausa = await p.evaluate(() => {
    const f = document.querySelector('#feed'); f.dataset.pausado = '1';
    const R = (e) => e.getBoundingClientRect();
    return new Promise((ok) => setTimeout(() => { const a = R(f.querySelector('.pausa')), t = R(f.querySelector('.trilho'));
      ok({ cruza: a.left < t.right - 0.5 && t.left < a.right - 0.5 && a.top < t.bottom && t.top < a.bottom, a: [a.left, a.right], t: [t.left, t.right] }); f.dataset.pausado = '0'; }, 500));
  });
  conferir('B-038', !pausa.cruza, `${tag}: "pausado" por cima da área dos itens do letreiro (pausado ${pausa.a.map(Math.round).join('–')} × itens ${pausa.t.map(Math.round).join('–')} px)`);
  const fotos = [];
  for (const y of [700, 780]) {
    await p.evaluate((yy) => window.scrollTo(0, yy), y);
    await p.waitForTimeout(500);
    // M-025: a barra da eleição presa no alto (antes, a barra do cartão de identificação da M-001)
    const topo = await p.evaluate(() => (document.querySelector('.eleicao.presa .el-cartao')?.getBoundingClientRect().top ?? null));
    if (topo == null || topo < 2) { fotos.push(null); continue; }
    fotos.push(await p.screenshot({ clip: { x: 0, y: 0, width: p.viewportSize().width, height: Math.floor(topo) } }));
  }
  if (fotos[0] && fotos[1]) conferir('B-040', fotos[0].equals(fotos[1]), `${tag}: a faixa acima da barra presa no alto mostra o conteúdo que rola por baixo`);
}

const cenarios = [
  { nome: 'presidente', hash: '#/br/c0001' },
  { nome: 'prefeito-eleito', hash: '#/ac01120/c0011' },
];
const larguras = [360, 390, 430, 768, 961, 1024, 1100, 1180, 1200, 1280, 1366, 1920];
for (const cen of cenarios) for (const comparar of ['uv', 'ambos']) for (const largura of larguras) for (const tema of ['escuro', 'claro']) {
  if (tema === 'claro' && ![390, 1366].includes(largura)) continue;
  const tag = `${cen.nome} · ${comparar} · ${largura} px · ${tema}`;
  const { ctx, p } = await abrir({ largura, tema, hash: cen.hash, comparar });
  const m = await p.evaluate(medir);
  conferirTudo(m, tag, { largura, comparar });
  conferirExtras(m, tag, { largura });
  if (comparar === 'uv' && [390, 1366].includes(largura)) await conferirRolagem(p, tag);
  if (tema === 'escuro' && [360, 390, 1024, 1366].includes(largura)) await p.screenshot({ path: `${saida}/${cen.nome}-${comparar}-${largura}.png`, fullPage: true });
  await ctx.close();
}

await nav.close();
console.log(`visual_pagina: ${oks} conferências ok, ${falhas.length} falha(s)`);
const porBug = new Map();
for (const f of falhas) { const b = f.slice(0, 5); porBug.set(b, (porBug.get(b) || 0) + 1); }
if (porBug.size) console.log('por bug:', [...porBug].map(([b, n]) => `${b} ${n}`).join(' · '));
process.exit(falhas.length ? 1 : 0);
