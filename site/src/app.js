// Urna Velox — página de resultados (etapa 5). Acompanha a apuração só pelo protocolo público (docs/PROTOCOLO_WS.md):
// WebSocket /ws (instantâneo, atualização, evento, marco, estado) e REST /api/v1/nomes e /api/v1/topicos.
//
// Ritmo de desenho ≠ ritmo de mensagens: a 60× chegam ~100 versões/s do br-c0001; cada mensagem só guarda o último
// valor do tópico e marca "sujo"; o desenho acontece no máximo uma vez por requestAnimationFrame, e as animações em
// andamento (hodômetros, barras, posições, anel, curva) são redirecionadas para o valor novo, sem recomeçar.
// Nenhum pulo na tela: tudo muda por transform/opacity em caixas de tamanho reservado (meta CLS = 0, medida aqui
// mesmo com PerformanceObserver e exposta em window.__uv para os ensaios).

import { Conexao } from './conexao.js';
import { Hodometro } from './hodometro.js';
import { Anel, Curva } from './graficos.js';
import { Feed, Folha } from './feed.js';
import { Historico } from './historico.js';
import { Nomes, itemMarco, itemEvento, primeiros, itemConfirmou, itemOrdemDiferente } from './textos.js';
import { coresDaDisputa } from './cores.js';
import { iniciarPlayer } from './replay.js';
import { Controle, celulasTse, desenharCelulas, gravarModo, modoInicial, segundosBr, situacaoTse, textoEstado, textoFontes } from './comparacao.js';
import { abr, digitos, el, hora, mil, pct, segundos, movimentoReduzido } from './util.js';

const $ = (s) => document.querySelector(s);
const cargo4 = (c) => String(c).padStart(4, '0');

/** Último resultado de cada tópico: guarda o texto da mensagem e só interpreta o JSON quando alguém lê (get). */
class Resultados {
  constructor() { this.m = new Map(); }
  por(t, seq, txt) { this.m.set(t, { seq, txt, obj: null }); }
  seq(t) { return this.m.get(t)?.seq ?? -1; }
  has(t) { return this.m.has(t); }
  get(t) {
    const x = this.m.get(t);
    if (!x) return undefined;
    if (!x.obj) { try { x.obj = JSON.parse(x.txt).resultado; } catch { x.obj = null; } x.txt = null; }
    return x.obj;
  }
  *[Symbol.iterator]() { for (const t of this.m.keys()) yield [t, this.get(t)]; }
}

const S = {
  N: new Nomes(null),
  topicos: new Set(),       // tópicos existentes (GET /api/v1/topicos)
  res: new Resultados(),    // tópico → último resultado
  sujos: new Set(),
  eventos: new Map(),       // chave de conteúdo → evento (dedup na reconexão, §5)
  marcos: new Map(),
  faltam: new Map(),        // tópico da circunscrição → último faltam_para_mat
  estado: null,
  conexao: 'conectando',
  tela: null,               // { abr, cargo, topico }
  ui: null,                 // elementos da tela atual
  cls: 0,
  msgs: 0,
  ritmo: [],                // [instante ms, seq] do br/maior tópico (BUs por minuto)
  // comparação com o TSE (etapa 7, comparacao.js)
  comparar: modoInicial(),  // 'uv' | 'ambos' | 'tse'
  tse: new Map(),           // tópico → última mensagem `tse` (PROTOCOLO_WS §4.6)
  relogio: null,            // { t: segundos do site, em: performance.now(), x: velocidade } da última mensagem `estado`
  confirmados: new Map(),   // tópico → marco "TSE confirmou" (para os ensaios)
  ordemDesde: new Map(),    // tópico → geração (s) do 1º arquivo do TSE com a ordem diferente
};
window.__uv = S; // para os ensaios (Playwright)

// ---------------------------------------------------------------- CLS medido aqui
try {
  new PerformanceObserver((l) => {
    for (const e of l.getEntries()) {
      if (e.hadRecentInput) continue;
      S.cls += e.value;
      // de onde veio (para depurar): os primeiros 20 deslocamentos
      if ((S.clsFontes ||= []).length < 20) S.clsFontes.push({ v: e.value, t: Math.round(e.startTime), de: (e.sources || []).map((s) => (s.node?.className || s.node?.nodeName || '?') + ' ' + JSON.stringify(s.previousRect) + '→' + JSON.stringify(s.currentRect)) });
    }
  }).observe({ type: 'layout-shift', buffered: true });
} catch { /* navegador sem layout-shift */ }

// ---------------------------------------------------------------- mensagens
function receber(m) {
  S.msgs++;
  switch (m.tipo) {
    case 'resultado': {
      // nunca volta: substitui só se seq ≥ (o instantâneo da reconexão também)
      if (S.res.seq(m.topico) > m.seq) return;
      S.res.por(m.topico, m.seq, m.txt);
      S.sujos.add(m.topico);
      // `seq` é o nº de BUs aplicados (global): o maior visto dá o ritmo da apuração
      S.seqMax = Math.max(S.seqMax || 0, m.seq);
      pedirQuadro();
      break;
    }
    case 'evento': {
      const k = `${m.seq}|${m.abrangencia}|${m.cargo}|${m.candidato}|${m.estado}`;
      if (S.eventos.has(k)) return;
      S.eventos.set(k, m);
      const it = itemEvento(S.N, m);
      if (it) porNoFeed(it);
      S.primeirosSujo = true;
      if (m.classe === 'apuracao') S.estadoSujo = true;
      pedirQuadro();
      break;
    }
    case 'marco': {
      if (m.classe === 'faltam_para_mat') {
        const ant = S.faltam.get(m.topico);
        if (ant && ant.seq > m.seq) return;
        S.faltam.set(m.topico, m);
        S.faltamSujo = true;
        pedirQuadro();
        return;
      }
      const k = `${m.classe}|${m.abrangencia}|${m.cargo}|${m.seq}|${m.secao}|${m.valor}`;
      if (S.marcos.has(k)) return;
      S.marcos.set(k, m);
      const it = itemMarco(S.N, m);
      if (it) porNoFeed(it);
      if (m.classe === 'pct_secoes' && m.valor === 100 && m.abrangencia.length === 2 && !S.primeiraUf100) {
        S.primeiraUf100 = m;
        porNoFeed({ id: 'c:uf100', ic: '★', t: hora(m.horario), seq: m.seq, cat: 'andamento', texto: `Primeira UF com 100% das seções apuradas: ${S.N.uf(m.abrangencia)}`,
          detalhe: { titulo: 'Primeira UF 100% apurada', linhas: [['UF', S.N.uf(m.abrangencia)], ['Horário', hora(m.horario)], ['BUs aplicados (seq)', mil(m.seq)]] } });
      }
      break;
    }
    case 'tse': {
      const t = String(m.topico || '').slice(4);
      const ant = S.tse.get(t);
      // a versão só cresce numa sessão do serviço; o instantâneo (inscrição, reconexão) sempre vale
      if (ant && !m.instantaneo && ant.ver >= m.ver) return;
      S.tse.set(t, m);
      S.tseSujo = true;
      if (S.tela?.topico === t) S.sujos.add(t);
      verificarConfirmacao(t);
      pedirQuadro();
      break;
    }
    case 'estado':
      S.estado = m;
      S.relogio = { t: segundosBr(m.relogio_site), em: performance.now(), x: m.replay ? (m.replay.pausado ? 0 : (m.replay.acelerar || 1)) : 1 };
      S.estadoSujo = true;
      // barra do replay (uv-servico --controle-replay): sessão nova depois de voltar no tempo → recarrega
      S.player?.estadoWs(m.replay);
      pedirQuadro();
      break;
    case 'erro':
      // servidor sem a comparação (--sem-tse): o cabeçalho das fontes avisa
      if (m.motivo === 'comparacao_desligada') { S.tseDesligada = true; S.fontesSujo = true; pedirQuadro(); }
      break;
    default:
      break; // aviso (seq_pulado é normal): nada a fazer
  }
}

function estadoConexao(e, info) {
  S.conexao = e;
  S.conexaoInfo = info;
  S.estadoSujo = true;
  pedirQuadro();
}

// ---------------------------------------------------------------- feed
let feed, folha, hist;
function porNoFeed(it) {
  hist.por(it);
  const item = feed.por(it);
  if (it.destaque) destacar(item.el);
}
const destaques = [];
function destacar(e) {
  // no máximo 2 itens do feed com o brilho ao mesmo tempo, por 30 s
  if (movimentoReduzido()) return;
  e.classList.add('brilho');
  const h = el('span', 'halo');
  e.append(h);
  destaques.push([e, h]);
  while (destaques.length > 2) { const [a, b] = destaques.shift(); a.classList.remove('brilho'); b.remove(); }
  setTimeout(() => { const i = destaques.findIndex((x) => x[0] === e); if (i >= 0) { destaques.splice(i, 1); e.classList.remove('brilho'); h.remove(); } }, 30000);
}

// ---------------------------------------------------------------- tela (rota, cargos, painel)
function lerRota() {
  const m = /^#\/([a-z]{2}(?:\d{5})?)\/c(\d{4})$/.exec(location.hash);
  let a = m?.[1], c = m ? +m[2] : null;
  if (!a || !S.topicos.has(`${a}-c${cargo4(c)}`)) {
    // padrão: Presidente no Brasil; senão o primeiro tópico do cargo de menor código
    const pref = ['br-c0001', ...[...S.topicos].sort()];
    const t = pref.find((x) => S.topicos.has(x));
    if (!t) return null;
    const p = /^(.+)-c(\d{4})$/.exec(t);
    a = p[1]; c = +p[2];
  }
  return { abr: a, cargo: c, topico: `${a}-c${cargo4(c)}` };
}

function cargosDe(a) {
  return [...S.topicos].filter((t) => t.startsWith(`${a}-c`)).map((t) => +t.slice(-4)).sort((x, y) => x - y);
}

function montarRota(t) {
  const nav = $('#rota');
  const a = abr(t.abr);
  const ufs = [...new Set([...S.topicos].map((x) => x.split('-')[0]).filter((x) => x.length === 2 && x !== 'br'))].sort();
  const munis = a.uf ? [...new Set([...S.topicos].map((x) => x.split('-')[0]).filter((x) => x.length === 7 && x.startsWith(a.uf)))] : [];
  const br = el('button', 'passo', 'Brasil');
  br.type = 'button';
  br.addEventListener('click', () => irPara('br'));
  const filhos = [];
  if (S.topicos.has('br-c0001') || [...S.topicos].some((x) => x.startsWith('br-'))) filhos.push(br);
  // UFs: as que têm tópico próprio e as que só têm municípios (eleições municipais)
  const ufsTodas = [...new Set([...ufs, ...[...S.topicos].map((x) => x.split('-')[0]).filter((x) => x.length === 7).map((x) => x.slice(0, 2))])].sort();
  if (ufsTodas.length) {
    const s = el('select', 'passo');
    s.setAttribute('aria-label', 'Estado');
    s.append(new Option('Estado…', ''));
    for (const u of ufsTodas) s.append(new Option(S.N.uf(u), u, false, u === a.uf));
    s.addEventListener('change', () => s.value && irPara(s.value));
    filhos.push(s);
  }
  if (munis.length) {
    const s = el('select', 'passo');
    s.setAttribute('aria-label', 'Município');
    s.append(new Option('Município…', ''));
    munis.map((m) => [m, S.N.mun(m)]).sort((x, y) => x[1].localeCompare(y[1], 'pt-BR')).forEach(([m, n]) => s.append(new Option(n, m, false, m === t.abr)));
    s.addEventListener('change', () => s.value && irPara(s.value));
    filhos.push(s);
  }
  nav.replaceChildren(...filhos);
}

function irPara(a, c) {
  const cs = cargosDe(a);
  let cargo = c ?? (S.tela && cs.includes(S.tela.cargo) ? S.tela.cargo : cs[0]);
  if (cargo == null) {
    // UF sem tópico próprio (só municípios): vai para o primeiro município
    const m = [...S.topicos].map((x) => x.split('-')[0]).filter((x) => x.startsWith(a) && x.length === 7).sort()[0];
    if (!m) return;
    a = m; cargo = cargosDe(m)[0];
  }
  location.hash = `#/${a}/c${cargo4(cargo)}`;
}

function montarCargos(t) {
  const nav = $('#cargos');
  nav.replaceChildren(...cargosDe(t.abr).map((c) => {
    const b = el('button', null, S.N.cargo(c));
    b.type = 'button';
    b.setAttribute('role', 'tab');
    b.setAttribute('aria-selected', String(c === t.cargo));
    b.addEventListener('click', () => irPara(t.abr, c));
    return b;
  }));
}

function cardTatil(c) {
  // inclina de leve sob o ponteiro (só transform) e acende o reflexo; no toque, a mesma resposta enquanto pressiona
  if (movimentoReduzido()) return c;
  const mover = (e) => {
    const r = c.getBoundingClientRect();
    const x = (e.clientX - r.left) / r.width, y = (e.clientY - r.top) / r.height;
    c.style.setProperty('--ry', `${((x - 0.5) * 3).toFixed(2)}deg`);
    c.style.setProperty('--rx', `${((0.5 - y) * 3).toFixed(2)}deg`);
    c.style.setProperty('--mx', `${(x * 100).toFixed(1)}%`);
    c.style.setProperty('--my', `${(y * 100).toFixed(1)}%`);
    c.style.setProperty('--luz', '1');
  };
  const soltar = () => { c.style.setProperty('--rx', '0deg'); c.style.setProperty('--ry', '0deg'); c.style.setProperty('--luz', '0'); };
  c.addEventListener('pointermove', (e) => { if (e.pointerType === 'mouse' || e.buttons) mover(e); });
  c.addEventListener('pointerdown', mover);
  c.addEventListener('pointerleave', soltar);
  c.addEventListener('pointerup', (e) => { if (e.pointerType !== 'mouse') soltar(); });
  c.addEventListener('pointercancel', soltar);
  return c;
}

function stat(rot, dig, comPct) {
  const h = new Hodometro({ inteiros: dig, enxuto: true });
  const p = comPct ? new Hodometro({ inteiros: 3, casas: 2, sufixo: '%', enxuto: true }) : null;
  const caixa = el('div', null, [el('div', 'rot', rot), el('div', 'val', h.el)]);
  if (p) caixa.append(el('div', 'p', p.el));
  return { caixa, h, p };
}

function montarPainel(t) {
  const res = S.res.get(t.topico);
  const painel = $('#painel');
  if (!res) {
    painel.replaceChildren(el('div', 'carregando', 'Esperando o primeiro resultado deste cargo…'));
    S.ui = { topico: t.topico, pendente: true };
    return;
  }
  const circ = res.circ;
  const eleitorado = Math.max(res.eleitorado || 0, res.apurado || 0, 1);
  const dig = digitos(eleitorado);
  const digSec = digitos(Math.max(res.secoes?.total || 0, 1));
  // candidatos na ordem do número (cores fixas: tom da cor do partido, ver cores.js)
  const ordem = [...res.cand].sort((a, b) => a.n - b.n);
  const cores = coresDaDisputa(ordem.map((k) => ({ n: k.n, partido: S.N.candidato(circ, res.cargo, k.n).partido })));
  const linhas = new Map();
  const lista = el('ol', 'lista');
  lista.setAttribute('aria-label', 'Candidatos em ordem de votos');
  const altura = parseFloat(getComputedStyle(document.documentElement).getPropertyValue('--linha-cand')) || 76;
  lista.style.height = `${Math.max(1, ordem.length) * altura}px`;
  for (const k of ordem) {
    const info = S.N.candidato(circ, res.cargo, k.n);
    const cor = cores.get(k.n);
    const li = el('li', 'cand');
    li.style.setProperty('--c', cor);
    const pctH = new Hodometro({ inteiros: 3, casas: 2, sufixo: '%' });
    const votH = new Hodometro({ inteiros: dig, sufixo: 'votos' });
    const barra = el('i');
    const pilula = el('span', 'pilula', '');
    const vice = info.vices.find((v) => v.tp === 'v') || info.vices[0];
    const det = [info.partido, vice ? `${vice.tp === 'v' ? 'vice' : 'suplente'} ${vice.nome_urna}` : ''].filter(Boolean).join(' · ');
    const nome = el('div', 'nome', [el('b', null, info.nome), pilula]);
    // células do TSE e da diferença (comparação, etapa 7): sempre montadas; o modo só decide o que aparece
    const tc = celulasTse(dig);
    li.append(el('span', 'cor'), el('span', 'num', String(k.n)), el('div', 'quem', [nome, el('div', 'detalhe', det || ' ')]),
      el('span', 'pct', pctH.el), el('span', 'qtd', votH.el), tc.tse, tc.dif, el('span', 'barra', barra));
    li.dataset.n = k.n;
    lista.append(li);
    linhas.set(k.n, { li, pctH, votH, barra, pilula, tc, pos: -1, glow: false, diverge: false, nome: info.nome, cor });
  }
  lista.classList.add(`m-${S.comparar}`);
  const consulta = S.N.cargos.get(res.cargo)?.consulta;
  const h1 = el('h1', null, consulta ? `Consulta — ${S.N.local(t.abr)}` : (t.abr === 'br' ? `${S.N.cargo(res.cargo)} — Brasil` : `${S.N.cargo(res.cargo)} — ${S.N.local(t.abr)}`));
  const turno = res.turno ? `${res.turno}º turno` : '';
  const sub = el('p', 'sub', [turno, circ !== t.abr ? `resultado na abrangência; quem decide é ${S.N.local(circ)}` : `eleitorado de ${mil(res.eleitorado)}`].filter(Boolean).join(' · '));
  const selo = el('div', 'selo', '');
  selo.setAttribute('aria-live', 'polite');
  // fontes (comparação com o TSE): controle "Urna Velox · Ambos · TSE" e, ao lado, % de seções de cada fonte e a
  // idade do arquivo do TSE — linha de altura fixa, sempre presente (trocar de modo não mexe no que está embaixo)
  const controle = new Controle(S.comparar, mudarModo);
  const fa = el('span', 'f-a', ''), fb = el('span', 'f-b', ''), fe = el('span', 'f-est', '');
  const fontes = el('div', 'fontes', [controle.el, el('div', 'f-txt', [fa, fb]), fe]);
  const colunas = el('div', 'colunas', [el('span', 'c-uv', 'Urna Velox'), el('span', 'c-tse', 'TSE'), el('span', 'c-dif', 'Diferença')]);
  colunas.setAttribute('aria-hidden', 'true');
  const corrida = cardTatil(el('section', 'vidro corrida', [el('header', 'corrida-cab', [el('div', null, [h1, sub]), selo]), fontes, colunas, lista]));
  corrida.classList.add(`m-${S.comparar}`);

  // apuração
  const anel = new Anel();
  const pctSec = new Hodometro({ inteiros: 3, casas: 2, sufixo: '%', enxuto: true, centro: true });
  anel.centro.append(el('div', null, [pctSec.el, el('small', null, 'das seções')]));
  const secAp = new Hodometro({ inteiros: digSec }), secTot = new Hodometro({ inteiros: digSec });
  const ultBu = new Hodometro({ molde: '##:##:##' });
  // o número é o da circunscrição (Presidente numa UF: o do Brasil, até 9 dígitos), não o da abrangência da tela: com os
  // dígitos do eleitorado da UF ele estourava e aparecia 9.999.999. Enxuto: os 9 dígitos reservados não aparecem como espaço.
  const faltH = new Hodometro({ inteiros: Math.max(dig, 9), regressivo: true, enxuto: true });
  const camadaN = el('div', 'camada', [el('div', 'rot', 'Eleitores que faltam apurar para o matematicamente'), el('div', 'val', faltH.el)]);
  const camadaD = el('div', 'camada oculta decidido', '');
  const apuracao = cardTatil(el('section', 'vidro apuracao', [
    el('h2', 'cartao-titulo', 'Apuração'), anel.el,
    el('div', 'numeros', [
      el('div', null, [el('div', 'rot', 'Seções apuradas'), el('div', 'val', [secAp.el, el('span', 'de', ' de '), secTot.el])]),
      el('div', null, [el('div', 'rot', 'Horário do último BU'), el('div', 'val', ultBu.el)]),
    ]),
    el('div', 'faltam', [camadaN, camadaD]),
  ]));

  // curva
  const curva = new Curva();
  // a noite até agora (o servidor guarda um ponto por minuto de cada tópico): a curva não começa na hora em que a página abriu
  fetch(`/api/v1/curva/${t.topico}`).then((r) => (r.ok ? r.json() : null)).then((j) => {
    if (!j || S.ui?.curva !== curva) return;
    curva.semear(j.pontos.map(([tt, ap, tot, c]) => ({ t: tt, s: tot ? (100 * ap) / tot : 0, c: new Map(c) })));
    pedirQuadro();
  }).catch(() => {});
  const legenda = el('div', 'legenda');
  const cardCurva = cardTatil(el('section', 'vidro curva', [el('h2', 'cartao-titulo', 'Curva da apuração'), curva.el, legenda]));

  // votos
  const comp = stat('Comparecimento', dig, true), abst = stat('Abstenção', dig, true), brancos = stat('Brancos', dig, true), nulos = stat('Nulos', dig, true);
  const cardVotos = cardTatil(el('section', 'vidro votos', [el('h2', 'cartao-titulo', 'Votos'), comp.caixa, abst.caixa, brancos.caixa, nulos.caixa]));

  painel.replaceChildren(corrida, el('aside', 'lado', [apuracao, cardCurva, cardVotos]));
  S.ui = { topico: t.topico, circ, linhas, selo, anel, pctSec, secAp, secTot, ultBu, faltH, camadaN, camadaD, curva, legenda, comp, abst, brancos, nulos, altura, ultCurva: 0,
    corrida, lista, controle, fa, fb, fe, rotBu: apuracao.querySelector('.numeros > div:nth-child(2) .rot') };
  S.sujos.add(t.topico);
  S.faltamSujo = true;
}

function montarTela() {
  const t = lerRota();
  if (!t) return;
  S.tela = t;
  montarRota(t);
  montarCargos(t);
  montarPainel(t);
  atualizarTopicos();
  pedirQuadro();
}

// ---------------------------------------------------------------- inscrições
function atualizarTopicos() {
  // base: tudo o que é Brasil ou UF (curiosidades por UF) e as circunscrições municipais, até 190 tópicos; e a tela
  const todos = [...S.topicos];
  const base = todos.filter((t) => t.split('-')[0].length <= 2);
  const munis = todos.filter((t) => t.split('-')[0].length === 7);
  const uf = S.tela ? abr(S.tela.abr).uf : null;
  munis.sort((a, b) => (b.startsWith(uf || '--') - a.startsWith(uf || '--')) || a.localeCompare(b));
  const lista = [...base, ...munis].slice(0, 190);
  if (S.tela && !lista.includes(S.tela.topico)) lista.push(S.tela.topico);
  // comparação com o TSE: só o tópico da tela, só com "Ambos" ou "TSE" e só com a aba visível (a inscrição é o que
  // faz o servidor buscar o arquivo do TSE: aba escondida, nenhuma requisição)
  if (S.tela && S.comparar !== 'uv' && document.visibilityState !== 'hidden') lista.push(`tse:${S.tela.topico}`);
  S.con.definirTopicos(lista);
}

// ---------------------------------------------------------------- desenho (no máximo 1× por quadro)
let pedido = false, ultimoQuadro = 0;
function pedirQuadro() { if (!pedido) { pedido = true; requestAnimationFrame(quadro); } }

function quadro(agora) {
  pedido = false;
  const dt = Math.min(100, ultimoQuadro ? agora - ultimoQuadro : 16);
  ultimoQuadro = agora;
  let mexe = false;
  const ui = S.ui;
  if (ui?.pendente && S.res.has(ui.topico)) montarPainel(S.tela);
  if (S.ui && !S.ui.pendente) {
    if (S.sujos.has(S.ui.topico)) desenharTopico(agora);
    if (S.faltamSujo) desenharFaltam();
    mexe = S.ui.anel.quadro(dt) | S.ui.curva.quadro(dt);
  }
  S.sujos.clear();
  // cabeçalho das fontes: a cada mensagem do TSE e uma vez por segundo (a idade do arquivo anda)
  if (S.ui && !S.ui.pendente && (S.fontesSujo || S.tseSujo || (S.comparar !== 'uv' && agora - (S.fontesEm || 0) > 1000))) { desenharFontes(); S.fontesEm = agora; }
  S.tseSujo = false;
  if (S.estadoSujo) desenharEstado();
  if (S.faltamSujo) { feedFaltam(); S.faltamSujo = false; }
  if (agora - (S.curiosEm || 0) > 2000) { curiosidades(); S.curiosEm = agora; }
  feed.quadro(dt, agora);
  // o letreiro e as molas pedem quadros; sem nada se mexendo, o laço para até a próxima mensagem
  if (mexe || feed.itens.length) pedirQuadro();
}

function desenharTopico(agora) {
  const ui = S.ui, res = S.res.get(ui.topico);
  if (!res) return;
  const encerrada = !!res.encerrada;
  // comparação com o TSE: no modo "TSE" a lista, o cartão da apuração e o dos votos mostram o arquivo do TSE; em
  // "Ambos", a lista é a do Urna Velox com as colunas do TSE e a diferença ao lado
  const modo = S.comparar;
  const tm = modo !== 'uv' ? S.tse.get(ui.topico) : null;
  const d = tm?.dados || null;
  const tsePorN = new Map((d?.cand || []).map((k) => [k.n, k]));
  const usaTse = modo === 'tse' && !!d;
  const linhaDe = (k, pos, v, p) => {
    const l = ui.linhas.get(k.n);
    if (!l) return null;
    if (l.pos !== pos) { l.li.style.setProperty('--pos', pos); l.pos = pos; }
    l.pctH.definir(p);
    l.votH.definir(v);
    l.barra.style.transform = `scaleX(${Math.max(0, Math.min(1, (p ?? 0) / 100)).toFixed(4)})`;
    l.li.classList.toggle('nao-apto', p == null && k.dvt != null && k.dvt !== '' && !String(k.dvt).startsWith('Válido'));
    return l;
  };
  let diverge = [];
  if (usaTse) {
    for (const k of d.cand) linhaDe(k, k.pos - 1, k.v, k.pct);
  } else {
    res.cand.forEach((k, pos) => {
      const l = linhaDe(k, pos, k.v, k.pct);
      if (!l) return;
      if (modo === 'ambos') {
        const kt = tsePorN.get(k.n);
        const dv = desenharCelulas(l.tc, kt, k.v, pos) && (k.v > 0 || (kt?.v || 0) > 0);
        if (dv) diverge.push(k.n);
        if (dv !== l.diverge) { l.diverge = dv; l.li.classList.toggle('diverge', dv); }
      } else if (l.diverge) { l.diverge = false; l.li.classList.remove('diverge'); }
    });
  }
  if (modo === 'tse' && d) for (const l of ui.linhas.values()) if (l.diverge) { l.diverge = false; l.li.classList.remove('diverge'); }
  // pílulas e brilho (situação): do Urna Velox; no modo TSE, a do arquivo do TSE
  const sitTse = usaTse ? situacaoTse(d) : null;
  for (const k of (usaTse ? d.cand : res.cand)) {
    const l = ui.linhas.get(k.n);
    if (!l) continue;
    let pil = '';
    if (usaTse) { if (k.st) pil = k.st; else if (k.e && sitTse) pil = sitTse === 'eleito' ? 'eleito (TSE)' : '2º turno (TSE)'; }
    else if (encerrada && k.st) pil = k.st;
    else if (k.mat) pil = k.mat.estado === 'eleito' ? `eleito às ${hora(k.mat.em)}` : `2º turno às ${hora(k.mat.em)}`;
    if (l.pilula.textContent !== pil) { l.pilula.textContent = pil; l.pilula.classList.toggle('ligado', !!pil); }
    const glow = usaTse ? (!!k.e && !!sitTse) : (!!k.mat || k.st === 'Eleito');
    if (glow !== l.glow) {
      l.glow = glow;
      if (glow && !movimentoReduzido()) { l.li.classList.add('brilho'); l.li.append(el('span', 'halo')); }
      else { l.li.classList.remove('brilho'); l.li.querySelector('.halo')?.remove(); }
    }
  }
  // selo da circunscrição
  let selo = '';
  const nomeL = (k) => ui.linhas.get(k?.n)?.nome || '';
  if (usaTse) {
    const quem = d.cand.filter((k) => k.e).map(nomeL);
    if (sitTse === 'eleito') selo = `TSE: ${d.finalizada ? 'eleito' : 'matematicamente eleito'}${quem.length ? ': ' + quem.join(' e ') : ''}`;
    else if (sitTse === '2t') selo = `TSE: 2º turno${quem.length ? ': ' + quem.join(' × ') : ''}`;
  } else {
    const lider = res.cand[0];
    if (res.estado === 'matematicamente_eleito' || (res.estado === 'definido' && res.cand.some((k) => k.st === 'Eleito'))) {
      const el2 = res.cand.filter((k) => k.mat?.estado === 'eleito' || k.st === 'Eleito');
      const em = el2[0]?.mat?.em;
      selo = `${encerrada ? 'Eleito' : 'Matematicamente eleito'}: ${el2.map(nomeL).join(' e ') || nomeL(lider)}${em ? ' às ' + hora(em) : ''}`;
    } else if (res.estado === 'matematicamente_2t' || (res.estado === 'definido' && res.cand.some((k) => k.st === '2º turno'))) {
      const dois = res.cand.filter((k) => k.mat?.estado === '2t' || k.st === '2º turno');
      const em = dois.map((k) => k.mat?.em).filter(Boolean).sort()[0];
      selo = `${encerrada ? '2º turno' : 'Matematicamente 2º turno'}${dois.length ? ': ' + dois.map(nomeL).join(' × ') : ''}${em ? ' às ' + hora(em) : ''}`;
    } else if (res.estado === 'sem_eleito') selo = 'Sem eleito (art. 224)';
  }
  if (ui.selo.textContent !== selo) {
    ui.selo.textContent = selo;
    ui.selo.classList.toggle('ligado', !!selo);
    if (selo && !movimentoReduzido()) { ui.selo.classList.add('brilho'); if (!ui.selo.querySelector('.halo')) ui.selo.append(el('span', 'halo')); }
    else { ui.selo.classList.remove('brilho'); }
  }
  // apuração e votos: do Urna Velox, ou do arquivo do TSE no modo TSE
  const fonte = usaTse
    ? { secoes: { total: d.secoes.total, apuradas: d.secoes.totalizadas }, comparecimento: d.comparecimento, apurado: d.comparecimento + d.abstencao, vb: d.vb, vn: d.vn, vnt: d.vnt, ultimo_bu: d.geracao }
    : res;
  const rot = usaTse ? 'Horário do arquivo do TSE' : 'Horário do último BU';
  if (ui.rotBu && ui.rotBu.textContent !== rot) ui.rotBu.textContent = rot;
  const tot = fonte.secoes?.total || 0, ap = fonte.secoes?.apuradas || 0;
  ui.anel.definir(tot ? ap / tot : 0);
  ui.pctSec.definir(tot ? (100 * ap) / tot : 0);
  ui.secAp.definir(ap);
  ui.secTot.definir(tot);
  ui.ultBu.definir(fonte.ultimo_bu ? hora(fonte.ultimo_bu) : null);
  const c = fonte.comparecimento || 0, abst = Math.max(0, (fonte.apurado || 0) - c);
  const pc = (x, b) => (b ? (100 * x) / b : null);
  ui.comp.h.definir(c); ui.comp.p.definir(pc(c, fonte.apurado));
  ui.abst.h.definir(abst); ui.abst.p.definir(pc(abst, fonte.apurado));
  ui.brancos.h.definir(fonte.vb || 0); ui.brancos.p.definir(pc(fonte.vb || 0, c));
  ui.nulos.h.definir((fonte.vn || 0) + (fonte.vnt || 0)); ui.nulos.p.definir(pc((fonte.vn || 0) + (fonte.vnt || 0), c));
  // curva (do Urna Velox; no máximo 4 pontos por segundo)
  const totUv = res.secoes?.total || 0, apUv = res.secoes?.apuradas || 0;
  if (agora - ui.ultCurva > 250 || encerrada) {
    ui.ultCurva = agora;
    const a = res.cand[0], b = res.cand[1];
    ui.curva.linhas([a, b].filter(Boolean).map((k) => ({ n: k.n, cor: ui.linhas.get(k.n)?.cor })));
    ui.curva.ponto(segundos(res.ultimo_bu), totUv ? (100 * apUv) / totUv : 0, res.cand.map((k) => [k.n, k.pct]));
    const leg = `${a?.n}|${b?.n}`;
    if (ui.leg !== leg) {
      ui.leg = leg;
      const item = (cor, txt) => { const i = el('i'); i.style.setProperty('--c', cor); return el('span', null, [i, txt]); };
      ui.legenda.replaceChildren(item('#a996ff', 'seções (0–100%)'), ...[a, b].filter(Boolean).map((k) => item(ui.linhas.get(k.n)?.cor, ui.linhas.get(k.n)?.nome)));
    }
  }
  // ordem diferente entre as fontes por mais de dois intervalos do TSE (60 s de arquivos): um item no feed
  if (modo === 'ambos' && d) ordemDiferente(ui.topico, res, d, diverge);
  verificarConfirmacao(ui.topico);
  S.fontesSujo = true;
  S.faltamSujo = true;
}

/** Cabeçalho das fontes (comparação): % de seções de cada fonte, idade do arquivo do TSE e o estado da fonte. */
function desenharFontes() {
  S.fontesSujo = false;
  const ui = S.ui;
  if (!ui || ui.pendente) return;
  const res = S.res.get(ui.topico);
  const tm = S.comparar !== 'uv' ? S.tse.get(ui.topico) : null;
  const f = textoFontes(S.comparar, res, tm, agoraSite());
  if (ui.fa.textContent !== f.a) ui.fa.textContent = f.a;
  if (ui.fb.textContent !== f.b) ui.fb.textContent = f.b;
  const e = S.comparar === 'uv' ? '' : S.tseDesligada ? 'comparação desligada neste servidor' : textoEstado(tm);
  if (ui.fe.textContent !== e) { ui.fe.textContent = e; ui.fe.dataset.estado = tm?.estado || 'aguardando'; ui.fe.classList.toggle('ligado', !!e); }
}

/** Hora do site agora (s), interpolada desde a última mensagem `estado` (no replay, na velocidade dele). */
function agoraSite() {
  const r = S.relogio;
  if (!r || r.t == null) return null;
  return r.t + ((performance.now() - r.em) / 1000) * r.x;
}

/** Marco "TSE confirmou o matematicamente às hh:mm — Urna Velox às hh:mm, N min antes" (uma vez por disputa). */
function verificarConfirmacao(t) {
  if (S.confirmados.has(t)) return;
  const d = S.tse.get(t)?.dados;
  const sit = situacaoTse(d);
  const res = S.res.get(t);
  if (!sit || !res || res.circ !== res.abr) return;
  const uv = res.estado === 'matematicamente_eleito' ? 'eleito' : res.estado === 'matematicamente_2t' ? '2t' : null;
  const ems = res.cand.map((k) => k.mat?.em).filter(Boolean).sort();
  if (uv !== sit || !ems.length) return;
  const tUv = segundos(ems[0]), tTse = segundosBr(d.geracao);
  if (tUv == null || tTse == null) return;
  const it = itemConfirmou(S.N, res.circ, res.cargo, sit, ems[0], d.geracao, tUv, tTse);
  S.confirmados.set(t, { sit, uv: ems[0], tse: d.geracao, min: Math.round((tTse - tUv) / 60) });
  it.seq = S.seqMax || res.seq;
  porNoFeed(it);
}

function ordemDiferente(t, res, d, diverge) {
  const g = segundosBr(d.geracao);
  if (!diverge.length || g == null) { S.ordemDesde.delete(t); return; }
  let o = S.ordemDesde.get(t);
  if (!o) { o = { desde: g, geracao: d.geracao, avisado: false }; S.ordemDesde.set(t, o); }
  if (!o.avisado && g - o.desde >= 60) {
    o.avisado = true;
    const quem = diverge.slice(0, 3).map((n) => {
      const uv = res.cand.findIndex((k) => k.n === n) + 1, ts = d.cand.find((k) => k.n === n)?.pos;
      return `${S.ui?.linhas.get(n)?.nome || n} ${uv}º no Urna Velox, ${ts}º no TSE`;
    }).join('; ');
    const it = itemOrdemDiferente(S.N, res.circ, res.cargo, o.geracao, quem);
    it.seq = S.seqMax || res.seq;
    porNoFeed(it);
  }
}

/** Troca de modo pelo controle: guarda, ajusta a tela e as inscrições (a de `tse:` liga a busca no servidor). */
function mudarModo(m) {
  S.comparar = m;
  gravarModo(m);
  const ui = S.ui;
  if (ui && !ui.pendente) {
    ui.controle.definir(m);
    for (const x of [ui.corrida, ui.lista]) { x.classList.remove('m-uv', 'm-ambos', 'm-tse'); x.classList.add(`m-${m}`); }
    S.sujos.add(ui.topico);
  }
  atualizarTopicos();
  S.fontesSujo = true;
  pedirQuadro();
}

function desenharFaltam() {
  const ui = S.ui, res = S.res.get(ui.topico);
  if (!res) return;
  const f = S.faltam.get(`${res.circ}-c${cargo4(res.cargo)}`);
  const decidido = res.estado !== 'apurando' || f?.faltam === 0;
  if (decidido) {
    const t = res.estado === 'matematicamente_2t' ? 'Matematicamente 2º turno' : res.estado === 'sem_eleito' ? 'Sem eleito' : 'Matematicamente decidido';
    const txt = `${t}${f?.horario ? ' às ' + hora(f.horario) : ''}`;
    if (ui.camadaD.dataset.txt !== txt) {
      ui.camadaD.dataset.txt = txt;
      ui.camadaD.replaceChildren(el('div', null, txt), el('small', null, f ? `no BU nº ${mil(f.seq)} da noite` : ''));
    }
    ui.faltH.definir(0);
  } else {
    ui.faltH.definir(f && f.faltam != null ? f.faltam : null);
  }
  ui.camadaN.classList.toggle('oculta', decidido);
  ui.camadaD.classList.toggle('oculta', !decidido);
}

// ---------------------------------------------------------------- faixa e conexão
let faixa;
function montarFaixa() {
  const ritmoH = new Hodometro({ inteiros: 6 });
  const ritmo = el('span', 'chip', [ritmoH.el, el('span', null, ' BUs/min')]);
  ritmo.style.width = '150px';
  const pausa = el('span', 'chip alerta oculto', '');
  pausa.style.width = '260px';
  const alerta = el('span', 'chip alerta oculto', '');
  $('#faixa').replaceChildren(ritmo, pausa, alerta);
  faixa = { ritmoH, ritmo, pausa, alerta };
}

function desenharEstado() {
  S.estadoSujo = false;
  const e = S.estado;
  const c = $('#conexao');
  let est = S.conexao, txt = 'conectando…';
  if (S.conexao === 'vivo') { txt = 'ao vivo'; }
  if (S.conexao === 'caiu') {
    const s = Math.max(0, Math.round(((S.conexaoInfo?.em || Date.now()) - Date.now()) / 1000));
    txt = `conexão caiu · volta em ${s}s`;
    clearTimeout(S.tCaiu);
    S.tCaiu = setTimeout(() => { S.estadoSujo = true; pedirQuadro(); }, 1000);
  }
  if (S.conexao === 'vivo' && e?.pausa_429?.ativa) { est = 'pausa'; txt = 'TSE pediu pausa'; }
  if (S.conexao === 'vivo' && e?.encerrada) { est = 'encerrada'; txt = 'apuração encerrada'; }
  if (c.dataset.estado !== est) c.dataset.estado = est;
  const t = c.querySelector('.txt');
  if (t.textContent !== txt) t.textContent = txt;
  if (!e) return;
  const pz = e.pausa_429?.ativa ? `Pausa de rede (HTTP 429)${e.pausa_429.retomada_em ? ' até ' + hora(e.pausa_429.retomada_em) : ''}` : '';
  if (faixa.pausa.textContent !== pz) { faixa.pausa.textContent = pz; faixa.pausa.title = pz; faixa.pausa.classList.toggle('oculto', !pz); }
  const al = e.alertas || {};
  const avisos = [];
  if (al.secoes_esperadas_incoerentes) avisos.push('o % de seções de algumas UFs pode estar errado (seções esperadas incoerentes)');
  if (al.destinacao_presumida) avisos.push('destinação dos votos presumida "Válido"');
  if (al.reenvio_diferente || al.anulacao) avisos.push('houve BU reenviado ou seção anulada');
  const at = avisos.length ? `Atenção: ${avisos.join('; ')}` : '';
  if (faixa.alerta.textContent !== at) { faixa.alerta.textContent = at; faixa.alerta.title = at; faixa.alerta.classList.toggle('oculto', !at); }
}

// ---------------------------------------------------------------- curiosidades calculadas aqui (a cada 2 s)
function curiosidades() {
  // ritmo: BUs por minuto do tópico mais movimentado, janela de 60 s (relógio da página)
  const agora = performance.now();
  if (S.seqMax != null) S.ritmo.push([agora, S.seqMax]);
  while (S.ritmo.length > 2 && agora - S.ritmo[0][0] > 60000) S.ritmo.shift();
  if (S.ritmo.length >= 2) {
    const [t0, s0] = S.ritmo[0], [t1, s1] = S.ritmo[S.ritmo.length - 1];
    faixa.ritmoH.definir(t1 > t0 ? Math.round(((s1 - s0) * 60000) / Math.max(5000, t1 - t0)) : 0);
  } else faixa.ritmoH.definir(0);
  if (S.primeirosSujo) { S.primeirosSujo = false; for (const it of primeiros(S.N, S.eventos.values())) porNoFeed(it); }
  // disputa mais apertada e mais folgada (circunscrições com ≥ 1% das seções), comparecimento por UF
  let aperto = null, folga = null, maiorC = null, menorC = null;
  for (const [t, r] of S.res) {
    if (!r?.cand || r.cand.length < 2) continue;
    const ap = r.secoes?.apuradas || 0, tot = r.secoes?.total || 0;
    if (r.circ === r.abr && tot && ap * 100 >= tot && r.cand[0].pct != null && r.cand[1].pct != null && !r.encerrada) {
      const d = r.cand[0].pct - r.cand[1].pct;
      if (!aperto || d < aperto.d) aperto = { t, r, d };
      if (!folga || d > folga.d) folga = { t, r, d };
    }
    if (r.cargo === 1 && r.abr.length === 2 && r.abr !== 'zz' && r.apurado > 0 && ap * 20 >= tot) {
      const c = (100 * r.comparecimento) / r.apurado;
      if (!maiorC || c > maiorC.c) maiorC = { t, r, c };
      if (!menorC || c < menorC.c) menorC = { t, r, c };
    }
  }
  const disputa = (x) => S.N.disputa(x.r.circ, x.r.cargo);
  const lider = (x, i) => S.N.nomeCand(x.r.circ, x.r.cargo, x.r.cand[i].n);
  const linhasD = (x) => [['Disputa', disputa(x)], ['1º', `${lider(x, 0)} — ${pct(x.r.cand[0].pct)}%`], ['2º', `${lider(x, 1)} — ${pct(x.r.cand[1].pct)}%`],
    ['Diferença', `${pct(x.d)} ponto(s) percentual(is)`], ['Seções apuradas', `${pct((100 * x.r.secoes.apuradas) / x.r.secoes.total)}%`]];
  if (aperto) feed.por({ id: 'c:aperto', fixo: true, ic: '≈', texto: `Disputa mais apertada agora: ${disputa(aperto)}, diferença de ${pct(aperto.d)} p.p.`, detalhe: { titulo: 'Disputa mais apertada agora', linhas: linhasD(aperto) } });
  if (folga && folga.t !== aperto?.t) feed.por({ id: 'c:folga', fixo: true, ic: '↔', texto: `Mais folgada: ${disputa(folga)}, ${pct(folga.d)} p.p. entre 1º e 2º`, detalhe: { titulo: 'Disputa mais folgada agora', linhas: linhasD(folga) } });
  if (maiorC && menorC && maiorC.t !== menorC.t) {
    feed.por({ id: 'c:comp', fixo: true, ic: '◎', texto: `Comparecimento: maior em ${S.N.uf(maiorC.r.abr)} (${pct(maiorC.c, 1)}%), menor em ${S.N.uf(menorC.r.abr)} (${pct(menorC.c, 1)}%)`,
      detalhe: { titulo: 'Comparecimento por UF (seções apuradas até agora)', linhas: [['Maior', `${S.N.uf(maiorC.r.abr)} — ${pct(maiorC.c, 1)}%`], ['Menor', `${S.N.uf(menorC.r.abr)} — ${pct(menorC.c, 1)}%`], ['Base', 'Presidente, UFs com ao menos 5% das seções']] } });
  }
}

/** Itens fixos do feed com os eleitores que faltam (hodômetro): Presidente, as disputas mais perto de se decidir e a da tela. */
const itensFaltam = new Map();
function feedFaltam() {
  const tela = S.ui?.circ && S.tela ? `${S.ui.circ}-c${cargo4(S.tela.cargo)}` : null;
  const abertos = [...S.faltam.values()].filter((f) => f.faltam != null && f.faltam > 0);
  abertos.sort((a, b) => a.faltam - b.faltam);
  const quero = new Set(abertos.slice(0, 8).map((f) => f.topico));
  if (S.faltam.get('br-c0001')?.faltam > 0) quero.add('br-c0001');
  if (tela && S.faltam.get(tela)?.faltam > 0) quero.add(tela);
  for (const t of quero) {
    const f = S.faltam.get(t);
    let x = itensFaltam.get(t);
    const d = S.N.disputa(f.abrangencia, f.cargo);
    if (!x || !feed.porId.has(`f:${t}`)) {
      const res = S.res.get(t);
      const elei = S.N.cand.get(t) ? Math.max(res?.eleitorado || 0, f.restante || 0, f.faltam) : Math.max(f.restante || 0, f.faltam);
      const hod = new Hodometro({ inteiros: digitos(elei), regressivo: true });
      x = { hod };
      itensFaltam.set(t, x);
      // rótulo curto (cargo + sigla da UF): no celular a reticência comia o "faltam"; o nome inteiro fica no detalhe
      const ab = f.abrangencia;
      const curto = ab === 'br' ? S.N.cargo(f.cargo) : (ab.length === 2 && ab !== 'zz' ? `${S.N.cargo(f.cargo)} ${ab.toUpperCase()}` : d);
      feed.por({ id: `f:${t}`, fixo: true, ic: '↓', hod, texto: `${curto}: faltam`, t: 'eleitores para o matematicamente', detalhe: null });
    }
    x.hod.definir(f.faltam);
    const it = feed.porId.get(`f:${t}`);
    if (it) it.sair = false;
    if (it) it.detalhe = { titulo: `${d}: eleitores que faltam para o matematicamente`, linhas: [['Faltam apurar', mil(f.faltam)], ['Eleitores ainda não apurados', mil(f.restante)],
      ['Como se conta', 'quantos dos eleitores restantes ainda precisam ser apurados, votando no pior caso, até o resultado não poder mais mudar'], ['Atualizado no BU nº', mil(f.seq)]] };
  }
  for (const [t, x] of itensFaltam) {
    if (quero.has(t)) continue;
    // o número continua certo até o item sair (0 quando a disputa se decidiu)
    if (S.faltam.get(t)?.faltam != null) x.hod.definir(S.faltam.get(t).faltam);
    const it = feed.porId.get(`f:${t}`);
    // sai quando passar pela borda esquerda; se voltar a interessar antes disso, fica (o mesmo hodômetro)
    if (it) it.sair = true; else itensFaltam.delete(t);
  }
}

// ---------------------------------------------------------------- partida
async function iniciar() {
  const tema = (() => { try { return localStorage.getItem('uv-tema'); } catch { return null; } })();
  if (tema) document.documentElement.dataset.tema = tema;
  $('#tema').addEventListener('click', () => {
    const t = document.documentElement.dataset.tema === 'claro' ? 'escuro' : 'claro';
    document.documentElement.dataset.tema = t;
    try { localStorage.setItem('uv-tema', t); } catch { /* sem armazenamento */ }
  });
  folha = new Folha($('#folha'));
  feed = new Feed($('#feed'), (d) => folha.abrir(d));
  hist = new Historico($('#hist'), $('#feed-hist'));
  // a fonte embutida antes do primeiro desenho dos números (sem troca de fonte depois: nada de layout shift)
  const fonte = document.fonts.load('700 20px "Archivo UV"').catch(() => null);
  const [nomes, topicos, player] = await Promise.all([
    fetch('/api/v1/nomes').then((r) => (r.ok ? r.json() : null)).catch(() => null),
    fetch('/api/v1/topicos').then((r) => (r.ok ? r.json() : [])).catch(() => []),
    // barra do replay só no site simulado; montada (e o espaço dela reservado) antes do primeiro desenho
    iniciarPlayer(),
    fonte,
  ]);
  S.player = player;
  await document.fonts.ready;
  montarFaixa();
  S.N = new Nomes(nomes);
  S.topicos = new Set(topicos.map((t) => t.topico));
  S.con = new Conexao(receber, estadoConexao);
  window.addEventListener('hashchange', montarTela);
  // aba escondida: cancela a comparação com o TSE (e a refaz ao voltar)
  document.addEventListener('visibilitychange', () => { if (S.comparar !== 'uv') atualizarTopicos(); });
  // a idade do arquivo do TSE anda mesmo sem mensagens: um quadro por segundo enquanto a comparação está ligada
  setInterval(() => { if (S.comparar !== 'uv') pedirQuadro(); }, 1000);
  montarTela();
  S.con.abrir();
}

iniciar();
