// Urna Velox — página de resultados (etapa 5). Acompanha a apuração só pelo protocolo público (docs/arquitetura/PROTOCOLO_WS.md):
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
import { Controle, celulasTse, desenharCelulas, DICA_MS, DICA_TEXTO, dicaVista, gravarModo, INFO_FONTE, marcarDica, medirUso, modoInicial, MODOS, segundosBr, situacaoTse, textoEstado, textoFontes } from './comparacao.js';
import { CartaoPrevisao, juntarPrevisao } from './previsao.js';
import { ancora, comparar, diaDe, horaDe, quando, segundosF, tseNoSistema } from './relogio.js';
import { Identificacao } from './identificacao.js';
import { Telemetria, textoChip } from './sonda.js';
import { PainelTelemetria } from './telemetria.js';
import { CartaoMapa } from './mapa.js';
import { Cartoes } from './cartoes.js';
import { buscarPlaylist, contagemPartes, diaLocal, modoDe, proximaDe, quandoTxt } from './proxima.js';
import { buscarConsultas, buscarPleitos, PainelPleitos, resumoTxt } from './pleitos.js';
import { comLinks } from './normas.js';
import { ConexaoSerie, PlayerLocal, carregarSerie, rotaConsulta } from './serie.js';
import { abr, digitos, el, hora, mil, pct, segundos, movimentoReduzido, noConsole, porCodigo } from './util.js';

const $ = (s) => document.querySelector(s);
const cargo4 = (c) => String(c).padStart(4, '0');
const TEXTO_HTTP = 'Muitos acessos agora: em vez de receber a apuração em tempo real, esta página a consulta a cada minuto '
  + '(o letreiro de eventos, a comparação com o TSE e a previsão ficam parados). Volta ao tempo real sozinha assim que houver vaga.';

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
  ritmo: [],                // [instante ms, seq] do br/maior tópico (BUs por minuto), no relógio da página (servidor antigo)
  ritmoSrv: [],             // B-053: [instante ms, seq] do `estado`, no relógio do serviço
  ritmoSessao: undefined,   // `sistema0` da sessão das amostras (replay que recomeça: seq volta do zero)
  // comparação com o TSE (etapa 7, comparacao.js)
  comparar: modoInicial(),  // 'uv' | 'ambos' | 'tse'
  tse: new Map(),           // tópico → última mensagem `tse` (PROTOCOLO_WS §4.6)
  relogio: null,            // { t: segundos do site, em: performance.now(), x: velocidade } da última mensagem `estado`
  // M-003 (relogio.js): relógio do sistema do servidor { t, em } e a âncora com a noite do TSE { noite0, sis0, x }
  relSis: null,
  anc: null,
  matSis: new Map(),        // tópico da circunscrição → horário do sistema do evento do "matematicamente" (sem candidato)
  confirmados: new Map(),   // tópico → marco "TSE confirmou" (para os ensaios)
  ordemDesde: new Map(),    // tópico → geração (s) do 1º arquivo do TSE com a ordem diferente
  // previsão (etapa 8, previsao.js): tópico → última mensagem `previsao` (com o histórico das faixas)
  prev: new Map(),
  prevConfirmadas: new Map(), // tópico → confirmação (marco "previsão confirmada", para os ensaios)
  // sondas (sonda.js): o caminho dos BUs está andando? e com que atraso
  tele: new Telemetria(),
  ritmoAtual: null,         // BUs/min mostrado no chip
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
      // M-003: quando o Urna Velox viu o "matematicamente" da disputa, no relógio do sistema (para a comparação com o TSE)
      if (m.classe === 'matematicamente' && m.sistema && m.topico && !String(m.estado).startsWith('revogado') && !S.matSis.has(m.topico)) S.matSis.set(m.topico, m.sistema);
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
      S.mapa?.marco(m);
      const it = itemMarco(S.N, m);
      if (it) porNoFeed(it);
      if (m.classe === 'pct_secoes' && m.valor === 100 && m.abrangencia.length === 2 && !S.primeiraUf100) {
        S.primeiraUf100 = m;
        porNoFeed({ id: 'c:uf100', ic: '★', t: hora(quando(m)), seq: m.seq, cat: 'andamento', texto: `Primeira UF com 100% das seções apuradas: ${S.N.uf(m.abrangencia)}`,
          detalhe: { titulo: 'Primeira UF 100% apurada', linhas: [['UF', S.N.uf(m.abrangencia)], ['Horário (Urna Velox)', hora(quando(m))], ['Horário do BU no TSE', hora(m.horario)], ['BUs aplicados (seq)', mil(m.seq)]] } });
      }
      break;
    }
    case 'mapa':
      S.mapa?.receber(m);
      break;
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
    case 'previsao': {
      S.prev.set(m.topico, juntarPrevisao(S.prev.get(m.topico), m, S.anc?.x || 1));
      S.prevSujo = true;
      // marco "previsão confirmada" (uma vez por disputa): o "matematicamente" saiu e havia previsão antes
      const c = m.confirmacao;
      if (c && m.circunscricao && !S.prevConfirmadas.has(m.topico)) {
        S.prevConfirmadas.set(m.topico, c);
        const d = S.N.disputa(m.abrangencia, m.cargo);
        // no simulado acelerado, os segundos do relógio do sistema contam (1 s = 8 min da noite a 500×)
        const hm = (t) => hora(t).slice(0, (S.anc?.x || 1) > 1 ? 8 : 5);
        const txt = c.dentro ? `Previsão confirmada — ${d}: saiu às ${hm(c.saiu)}, dentro da faixa prevista${c.desde ? ` desde ${hm(c.desde)}` : ''}`
          : `Previsão fora da faixa — ${d}: saiu às ${hm(c.saiu)}, previsto entre ${hm(c.faixa.p10)} e ${hm(c.faixa.p90)}`;
        porNoFeed({ id: `p:prev:${m.topico}`, ic: '◷', t: hora(c.saiu), seq: S.seqMax, cat: 'andamento', texto: txt,
          detalhe: { titulo: `Previsão do matematicamente — ${d}`, linhas: [['Saiu às', hora(c.saiu)], ['Última faixa prevista', `${hora(c.faixa.p10)} a ${hora(c.faixa.p90)}`],
            ['Dentro da faixa', c.dentro ? 'sim' : 'não'], ['Faixas certas desde', c.desde ? hora(c.desde) : '—'],
            ['O que é', 'estimativa (P10–P90) a partir da velocidade de chegada dos BUs e da tendência dos votos; nunca um resultado']] } });
      }
      pedirQuadro();
      break;
    }
    case 'estado':
      // B-088: a playlist trocou o pleito no ar (o serviço recomeçou com outro): nomes, tópicos e mapa eram do anterior
      // (municípios pelo código, candidatos pelo número) — a página recomeça com os do pleito novo
      if (!S.consulta && m.pleito != null) {
        if (S.pleitoVivo != null && String(m.pleito) !== String(S.pleitoVivo)) { location.reload(); return; }
        S.pleitoVivo = m.pleito;
      }
      S.estado = m;
      desenharSeletor();
      verificarVersao(m.versao);
      // ritmo da apuração também pelo estado (≤ 10 s): a página não recebe mais os tópicos de todo o país (B-014)
      if (m.ultimo_bu?.seq != null) S.seqMax = Math.max(S.seqMax || 0, m.ultimo_bu.seq);
      if (m.ultimo_bu?.seq != null && m.relogio_sistema?.agora) amostrarRitmo(m.ultimo_bu.seq, segundosF(m.relogio_sistema.agora), m.relogio_sistema.sistema0);
      S.relogio = { t: segundosBr(m.relogio_site), em: performance.now(), x: m.replay ? (m.replay.pausado ? 0 : (m.replay.acelerar || 1)) : 1 };
      // M-003: relógio do sistema do servidor (anda a 1×) e a âncora com a noite do TSE
      if (m.relogio_sistema) {
        S.relSis = { t: segundosF(m.relogio_sistema.agora), em: performance.now() };
        const a = ancora(m.relogio_sistema);
        if (a && (!S.anc || a.sis0 !== S.anc.sis0 || a.x !== S.anc.x)) { S.anc = a; S.ui?.prev?.definirAceleracao(a.x); S.prevSujo = true; S.fontesSujo = true; }
      }
      S.estadoSujo = true;
      // barra do replay (uv-servico --controle-replay): sessão nova depois de voltar no tempo → recarrega
      S.player?.estadoWs(m.replay);
      pedirQuadro();
      break;
    case 'sonda':
      if (S.tele.receber(m, performance.now())) desenharChipSonda();
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
  if (e === 'vivo' && S.conexao !== 'vivo') S.tele.conectou(performance.now());
  S.conexao = e;
  S.conexaoInfo = info;
  S.estadoSujo = true;
  pedirQuadro();
}

// ---------------------------------------------------------------- feed
let feed, folha, hist;
// B-081: o letreiro recebe os itens em lote (no fim da tarefa que os entregou): com muitos de uma vez (a consulta
// acelerada de um geral, uma reconexão), só os últimos FEED_LOTE entram — cada item novo mede a própria largura, e
// milhares de medidas seguidas travavam a página; o histórico recebe todos na hora. No máximo um lote a cada
// FEED_INTERVALO ms (o letreiro anda a poucos itens por segundo: mais que isso não se vê).
const FEED_LOTE = 8;
const FEED_INTERVALO = 250;
let filaFeed = null;
let feedEm = -Infinity;
function porNoFeed(it) {
  hist.por(it);
  if (!filaFeed) { filaFeed = new Map(); setTimeout(descarregarFeed, Math.max(0, FEED_INTERVALO - (performance.now() - feedEm))); }
  filaFeed.delete(it.id);
  filaFeed.set(it.id, it);
}
function descarregarFeed() {
  feedEm = performance.now();
  const l = [...filaFeed.values()];
  filaFeed = null;
  // os que já estão no letreiro (atualizações) e os fixos entram sempre; dos novos, os últimos FEED_LOTE
  const corte = l.length - FEED_LOTE;
  l.forEach((it, i) => {
    if (i < corte && !it.fixo && !feed.porId.has(it.id)) return;
    const item = feed.por(it);
    if (it.destaque) destacar(item.el);
  });
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
  // consulta de um pleito (multipleito, 1.1): #/p/<pleito>/<abrangência>/c<cargo>
  const m = /^#\/(?:p\/\d+\/)?([a-z]{2}(?:\d{5})?)\/c(\d{4})$/.exec(location.hash);
  // o endereço só escolhe: o tópico usado (e pedido ao servidor) é sempre um dos que o servidor publicou, nunca o texto
  // do endereço (SonarCloud jssecurity:S8476)
  const pedido = m ? `${m[1]}-c${cargo4(+m[2])}` : null;
  let t = pedido && [...S.topicos].find((x) => x === pedido);
  if (!t) {
    // padrão: Presidente no Brasil; senão o primeiro tópico do cargo de menor código
    const pref = ['br-c0001', ...[...S.topicos].sort(porCodigo)];
    t = pref.find((x) => S.topicos.has(x));
    if (!t) return null;
  }
  const p = /^(.+)-c(\d{4})$/.exec(t);
  return { abr: p[1], cargo: +p[2], topico: t };
}

function cargosDe(a) {
  return [...S.topicos].filter((t) => t.startsWith(`${a}-c`)).map((t) => +t.slice(-4)).sort((x, y) => x - y);
}

/** M-025: o seletor do local na barra da eleição — Brasil, os estados e os municípios do estado da tela. */
function montarRota(t) {
  const a = abr(t.abr);
  const raizes = [...S.topicos].map((x) => x.split('-')[0]);
  const ufs = [...new Set(raizes.filter((x) => x.length === 2 && x !== 'br'))];
  const munis = a.uf ? [...new Set(raizes.filter((x) => x.length === 7 && x.startsWith(a.uf)))] : [];
  // UFs: as que têm tópico próprio e as que só têm municípios (eleições municipais)
  const ufsTodas = [...new Set([...ufs, ...raizes.filter((x) => x.length === 7).map((x) => x.slice(0, 2))])]
    .map((u) => [u, S.N.uf(u)]).sort((x, y) => x[1].localeCompare(y[1], 'pt-BR'));
  const grupos = [];
  if (raizes.includes('br')) grupos.push([null, [['br', 'Brasil']]]);
  grupos.push([ufsTodas.length > 1 || grupos.length ? 'Estados' : null, ufsTodas]);
  if (munis.length) grupos.push([`Municípios · ${a.uf.toUpperCase()}`, munis.map((m) => [m, S.N.local(m)]).sort((x, y) => x[1].localeCompare(y[1], 'pt-BR'))]);
  S.ident.locais(grupos, a.tipo === 'mun' ? t.abr : a.tipo === 'uf' ? a.uf : 'br');
  // já com a tela (sem esperar o primeiro resultado): o local muda a largura que sobra para os cargos, e nada pode andar
  // depois do primeiro desenho
  S.ident.definir({ turno: S.N.cargos.get(t.cargo)?.turno, cargo: S.N.cargo(t.cargo), local: t.abr === 'br' ? 'Brasil' : S.N.local(t.abr) });
  S.ident.eleicao(eleicaoAtual(null));
}

function irPara(a, c) {
  const cs = cargosDe(a);
  let cargo = c ?? (S.tela && cs.includes(S.tela.cargo) ? S.tela.cargo : cs[0]);
  if (cargo == null) {
    // UF sem tópico próprio (só municípios): vai para o primeiro município
    const m = [...S.topicos].map((x) => x.split('-')[0]).filter((x) => x.startsWith(a) && x.length === 7).sort(porCodigo)[0];
    if (!m) return;
    a = m; cargo = cargosDe(m)[0];
  }
  location.hash = `${S.consulta ? `#/p/${S.consulta.pleito}` : '#'}/${a}/c${cargo4(cargo)}`;
}

function montarCargos(t) {
  const nav = $('#cargos');
  nav.replaceChildren(...cargosDe(t.abr).map((c) => {
    const b = el('button', null, S.N.cargo(c));
    b.type = 'button';
    b.dataset.cargo = String(c);
    b.setAttribute('role', 'tab');
    b.setAttribute('aria-selected', String(c === t.cargo));
    b.addEventListener('click', () => irPara(t.abr, c));
    return b;
  }));
  // M-025: os que não cabem na barra vão para o "…" (antes do primeiro desenho: nada pula)
  S.ident.ajustarCargos();
}

function cardTatil(c) {
  // inclina de leve sob o ponteiro (só transform) e acende o reflexo; no toque, a mesma resposta enquanto pressiona
  if (movimentoReduzido()) return c;
  const mover = (e) => {
    // maximizado, sendo arrastado ou girando o globo do mapa: sem inclinação
    if (c.classList.contains('max') || c.classList.contains('arrastando') || e.target?.closest?.('.mp-palco.globo')) return;
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
  // a tela vai ser montada de novo: o cartão maximizado volta ao lugar antes
  S.cartoes?.restaurar(true);
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
  const altura = Number.parseFloat(getComputedStyle(document.documentElement).getPropertyValue('--linha-cand')) || 76;
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
    linhas.set(k.n, { li, pctH, votH, barra, pilula, pilTxt: '', tc, pos: -1, glow: false, diverge: false, nome: info.nome, cor });
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
  // M-015: rótulo "De onde vêm os números" + ⓘ em cima, controle, e embaixo uma frase de uma linha que muda com o modo
  // (mais o % das seções e o arquivo do TSE); tudo de altura fixa: nada pula (docs/BUGS.md, M-015)
  const controle = new Controle(S.comparar, (m) => mudarModo(m, true));
  const fx = el('span', 'f-x', ''), fa = el('span', 'f-a', ''), fb = el('span', 'f-b', ''), fe = el('span', 'f-est', '');
  const info = el('button', 'f-info', 'i');
  info.type = 'button';
  info.title = 'Como funciona a escolha da fonte dos números';
  info.setAttribute('aria-label', 'Como funciona a escolha da fonte dos números');
  info.addEventListener('click', () => { medirUso('fonte-info'); fimDaDica(); abrirInfoFonte(); });
  const cab = el('div', 'f-cab', [el('span', 'f-rot', 'De onde vêm os números'), info]);
  const fontes = el('div', 'fontes', [cab, controle.el, el('div', 'f-txt', [fx, fa, fb]), fe]);
  const colunas = el('div', 'colunas', [
    el('span', 'c-uv', [el('span', 'uv-longo', 'Urna Velox'), el('span', 'uv-curto', 'UV')]),
    el('span', 'c-tse', 'TSE'),
    el('span', 'c-dif', 'Diferença'),
  ]);
  colunas.setAttribute('aria-hidden', 'true');
  const corrida = cardTatil(el('section', 'vidro corrida', [el('header', 'corrida-cab', [el('div', null, [h1, sub]), selo]), fontes, colunas, lista]));
  corrida.classList.add(`m-${S.comparar}`);
  iniciarDica();
  if (S.dicaAtiva) controle.el.classList.add('dica');

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
      el('div', null, [el('div', 'rot', 'Seções apuradas'), secAp.arrastarEl = el('div', 'val', [secAp.el, el('span', 'de', ' de '), secTot.el])]),
      // M-003: o horário do Urna Velox (relógio do sistema) em destaque; o do TSE embaixo, com rótulo próprio
      el('div', null, [el('div', 'rot', 'Totalizado pelo Urna Velox às'), el('div', 'val', ultBu.el), el('div', 'hora-tse', '')]),
    ]),
    el('div', 'faltam', [camadaN, camadaD]),
  ]));

  // curva
  const curva = new Curva();
  // a noite até agora (o servidor guarda um ponto por minuto de cada tópico): a curva não começa na hora em que a página abriu
  // na consulta de um pleito, a curva sai da série (até o instante do slider)
  (S.con?.curva ? S.con.pronto(t.topico).then(() => S.con.curva(t.topico)) : fetch(`/api/v1/curva/${encodeURIComponent(t.topico)}`).then((r) => (r.ok ? r.json() : null))).then((j) => {
    if (!j || S.ui?.curva !== curva) return;
    curva.semear(j.pontos.map(([tt, ap, tot, c]) => ({ t: tt, s: tot ? (100 * ap) / tot : 0, c: new Map(c) })), S.consulta ? null : j.relogio === 'sistema' ? 'sistema' : 'noite');
    pedirQuadro();
  }).catch(() => {});
  const legenda = el('div', 'legenda');
  const cardCurva = cardTatil(el('section', 'vidro curva', [el('h2', 'cartao-titulo', 'Curva da apuração'), curva.el, legenda]));

  // votos
  const comp = stat('Comparecimento', dig, true), abst = stat('Abstenção', dig, true), brancos = stat('Brancos', dig, true), nulos = stat('Nulos', dig, true);
  const cardVotos = cardTatil(el('section', 'vidro votos', [el('h2', 'cartao-titulo', 'Votos'), comp.caixa, abst.caixa, brancos.caixa, nulos.caixa]));

  // previsão (etapa 8): estimativa do horário do matematicamente e do fim da apuração
  const prev = new CartaoPrevisao();
  const cardPrev = cardTatil(prev.el);

  // dois trilhos: a coluna larga (corrida e o mapa das seções) e a lateral; todo cartão se maximiza e muda de lugar
  // (cartoes.js); o mapa é o mesmo em todas as telas (o canvas e as luzes acesas não se perdem ao trocar de cargo)
  const K = S.cartoes;
  K.preparar(corrida, 'corrida', 'Corrida');
  K.preparar(apuracao, 'apuracao', 'Apuração');
  K.preparar(cardPrev, 'previsao', 'Previsão');
  K.preparar(cardCurva, 'curva', 'Curva da apuração');
  K.preparar(cardVotos, 'votos', 'Votos');
  const principal = el('div', 'principal coluna-cartoes', [corrida]);
  if (S.mapa) {
    if (!S.mapa.tatil) { cardTatil(S.mapa.el); S.mapa.tatil = true; }
    principal.append(K.preparar(S.mapa.el, 'mapa', 'Mapa das seções'));
  }
  const lado = el('aside', 'lado coluna-cartoes', [apuracao, cardPrev, cardCurva, cardVotos]);
  painel.replaceChildren(principal, lado);
  K.montar([principal, lado]);
  S.ui = { topico: t.topico, circ, linhas, selo, anel, pctSec, secAp, secTot, ultBu, faltH, camadaN, camadaD, curva, legenda, comp, abst, brancos, nulos, altura, ultCurva: 0,
    corrida, lista, controle, fx, fa, fb, fe, rotBu: apuracao.querySelector('.numeros > div:nth-child(2) .rot'),
    horaTse: apuracao.querySelector('.numeros .hora-tse'), prev, cargo: res.cargo };
  if (S.anc) prev.definirAceleracao(S.anc.x);
  S.sujos.add(t.topico);
  S.faltamSujo = true;
  S.prevSujo = true;
  // a circunscrição da tela só se conhece com o primeiro resultado: agora entra nas inscrições
  atualizarTopicos();
  // M-025: turno, local e cargo na barra da eleição; o nome e a data da eleição
  S.ident.definir({ turno: res.turno, cargo: S.N.cargo(res.cargo), local: t.abr === 'br' ? 'Brasil' : S.N.local(t.abr) });
  S.ident.eleicao(eleicaoAtual(res));
}

/** M-025: o nome e a data da eleição na tela — pela lista dos pleitos (o título sem o turno, que vai ao lado: "Eleições
 *  Gerais 2026", "Eleição suplementar · Roraima") e, sem ela, pelo tipo (B-087), pelo ano e pela data da noite. */

/** Tira o " · 2º turno" do fim do título do pleito, sem regex com retrocesso (SonarCloud S8786). */
function semTurno(s) {
  const i = s.lastIndexOf('·');
  if (i < 0 || !/^\dº turno$/.test(s.slice(i + 1).trimStart())) return s;
  return s.slice(0, i).trimEnd();
}

function eleicaoAtual(res = S.tela && S.res.get(S.tela.topico)) {
  const cod = String(S.consulta?.pleito ?? S.estado?.pleito ?? '');
  const p = cod && S.pleitos?.pleitos?.find((x) => String(x.codigo) === cod);
  if (p?.titulo) return { nome: semTurno(p.titulo), dataIso: p.data_iso || '' };
  const ano = [res?.ultimo_bu, S.serieInicio, S.estado?.relogio_site].map((x) => /(20\d\d)/.exec(x || '')?.[1]).find(Boolean) || '';
  const tipo = tipoEleicao(res?.cargo);
  const nome = tipo === 'Suplementar' ? `Eleição suplementar ${ano}` : `Eleições ${tipo} ${ano}`;
  return { nome: nome.trim(), dataIso: dataDaNoite(res) };
}

/** A data da eleição sem a lista dos pleitos: o começo da série (consulta) ou o dia do último BU (ao vivo/replay; antes
 *  das 8 h pode ser a madrugada seguinte ou o exterior, e então fica sem data). */
function dataDaNoite(res) {
  const s = /^(\d\d)\/(\d\d)\/(\d{4})/.exec(S.serieInicio || '');
  if (s) return `${s[3]}-${s[2]}-${s[1]}`;
  const u = /^(\d{4}-\d\d-\d\d)T(\d\d)/.exec(res?.ultimo_bu || '');
  return u && +u[2] >= 8 ? u[1] : '';
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
  // B-014: resultado inteiro só da tela e da circunscrição dela (Presidente numa UF: o do Brasil, para o
  // "matematicamente" e a previsão); antes eram até 190 tópicos, e o celular travava no modo rápido. O letreiro vem
  // dos eventos e marcos do servidor; o `faltam_para_mat`, só o da tela e o de Presidente.
  const lista = [];
  if (S.tela) {
    lista.push(S.tela.topico);
    const circ = S.res.get(S.tela.topico)?.circ;
    const tc = circ ? `${circ}-c${cargo4(S.tela.cargo)}` : null;
    if (tc && tc !== S.tela.topico && S.topicos.has(tc)) lista.push(tc);
    const fal = [tc || S.tela.topico];
    if (S.topicos.has('br-c0001') && !fal.includes('br-c0001')) fal.push('br-c0001');
    S.con.definirFaltam(fal);
  }
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
    if (S.prevSujo) { S.prevSujo = false; desenharPrevisao(); }
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
    else if (k.mat) pil = k.mat.estado === 'eleito' ? `eleito às ${horaMat(res, k.mat.em)}` : `2º turno às ${horaMat(res, k.mat.em)}`;
    if (l.pilTxt !== pil) {
      // B-043: o horário fica num span à parte, que some quando a linha é estreita (o nome continua à vista)
      l.pilTxt = pil;
      const i = pil.indexOf(' às ');
      l.pilula.replaceChildren(...(i < 0 ? [pil] : [pil.slice(0, i), el('span', 'pl-hora', pil.slice(i))]));
      l.pilula.title = pil;
      l.pilula.classList.toggle('ligado', !!pil);
    }
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
      selo = `${encerrada ? 'Eleito' : 'Matematicamente eleito'}: ${el2.map(nomeL).join(' e ') || nomeL(lider)}${em ? ' às ' + horaMat(res, em) : ''}`;
    } else if (res.estado === 'matematicamente_2t' || (res.estado === 'definido' && res.cand.some((k) => k.st === '2º turno'))) {
      const dois = res.cand.filter((k) => k.mat?.estado === '2t' || k.st === '2º turno');
      const em = dois.map((k) => k.mat?.em).filter(Boolean).sort()[0];
      selo = `${encerrada ? '2º turno' : 'Matematicamente 2º turno'}${dois.length ? ': ' + dois.map(nomeL).join(' × ') : ''}${em ? ' às ' + horaMat(res, em) : ''}`;
    } else if (res.estado === 'sem_eleito') selo = 'Sem eleito (art. 224)';
  }
  if (ui.selo.dataset.texto !== selo) {
    // artigos citados (ex.: "Sem eleito (art. 224)") viram link para a norma no site do TSE (normas.js); um só
    // filho para o grid do selo centralizar o texto inteiro
    ui.selo.dataset.texto = selo;
    ui.selo.replaceChildren(...(selo ? [el('span', 'selo-txt', comLinks(selo))] : []));
    ui.selo.classList.toggle('ligado', !!selo);
    if (selo && !movimentoReduzido()) { ui.selo.classList.add('brilho'); if (!ui.selo.querySelector('.halo')) ui.selo.append(el('span', 'halo')); }
    else { ui.selo.classList.remove('brilho'); }
  }
  // apuração e votos: do Urna Velox, ou do arquivo do TSE no modo TSE
  const fonte = usaTse
    ? { secoes: { total: d.secoes.total, apuradas: d.secoes.totalizadas }, comparecimento: d.comparecimento, apurado: d.comparecimento + d.abstencao, vb: d.vb, vn: d.vn, vnt: d.vnt, ultimo_bu: d.geracao }
    : res;
  // M-003: no modo Urna Velox, o horário grande é o do relógio do sistema (`totalizado_em`); o do TSE (horário do
  // último BU no TSE) fica embaixo, com rótulo próprio — no simulado com a data da eleição original
  const rot = usaTse ? 'Arquivo do TSE gerado às' : 'Totalizado pelo Urna Velox às';
  if (ui.rotBu && ui.rotBu.textContent !== rot) ui.rotBu.textContent = rot;
  const tot = fonte.secoes?.total || 0, ap = fonte.secoes?.apuradas || 0;
  ui.anel.definir(tot ? ap / tot : 0);
  ui.pctSec.definir(tot ? (100 * ap) / tot : 0);
  ui.secAp.definir(ap);
  ui.secTot.definir(tot);
  const grande = usaTse ? fonte.ultimo_bu : (res.totalizado_em || res.ultimo_bu);
  ui.ultBu.definir(grande ? hora(grande) : null);
  if (ui.horaTse) {
    const tb = segundos(res.ultimo_bu);
    let txt = '';
    if (usaTse) txt = res.totalizado_em ? `Urna Velox: ${hora(res.totalizado_em)}` : '';
    else if (tb != null && res.totalizado_em) {
      const ts = segundosF(res.totalizado_em);
      txt = `último BU no TSE: ${hora(res.ultimo_bu)}${Math.abs(ts - tb) > 43200 ? ` de ${diaDe(tb)}` : ''}`;
    }
    if (ui.horaTse.textContent !== txt) ui.horaTse.textContent = txt;
  }
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
    // M-003: o eixo é o relógio do sistema (o mesmo do GET /api/v1/curva desde a M-003)
    // B-082: o relógio do ponto vai junto (uma publicação sem `totalizado_em` no meio das outras não mistura os eixos)
    ui.curva.ponto(res.totalizado_em ? segundosF(res.totalizado_em) : segundos(res.ultimo_bu), totUv ? (100 * apUv) / totUv : 0, res.cand.map((k) => [k.n, k.pct]),
      S.consulta ? null : res.totalizado_em ? 'sistema' : 'noite');
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
  const f = textoFontes(S.comparar, res, tm, agoraSistema(), S.anc);
  const x = S.dicaAtiva && S.comparar === 'uv' ? DICA_TEXTO : f.x;
  if (ui.fx.textContent !== x) ui.fx.textContent = x;
  if (ui.fa.textContent !== f.a) ui.fa.textContent = f.a;
  if (ui.fb.textContent !== f.b) ui.fb.textContent = f.b;
  const e = S.comparar === 'uv' ? '' : S.tseDesligada ? 'comparação desligada neste servidor' : textoEstado(tm);
  if (ui.fe.textContent !== e) { ui.fe.textContent = e; ui.fe.dataset.estado = tm?.estado || 'aguardando'; ui.fe.classList.toggle('ligado', !!e); }
}

/** M-003: "hh:mm:ss" do "matematicamente" de `res` no relógio do sistema — o do evento da disputa (`sistema`); sem
 * ele, o horário do BU (`mat.em`, relógio do TSE) levado ao relógio do sistema pela âncora. */
function horaMat(res, em) {
  const s = S.matSis.get(`${res.circ}-c${cargo4(res.cargo)}`);
  if (s) return hora(s);
  return S.anc ? horaDe(tseNoSistema(S.anc, segundos(em))) : hora(em);
}

/** Relógio do sistema do servidor agora (s, escala de `segundos`), andando a 1× desde a última mensagem `estado`. */
function agoraSistema() {
  const r = S.relSis;
  if (!r || r.t == null) return null;
  return r.t + (performance.now() - r.em) / 1000;
}

/** Marco "TSE confirmou o matematicamente às hh:mm — Urna Velox às hh:mm, N min antes" (uma vez por disputa).
 * M-003: o Urna Velox no relógio do sistema (evento `sistema`); o TSE levado ao mesmo relógio pela âncora. */
function verificarConfirmacao(t) {
  if (S.confirmados.has(t)) return;
  const d = S.tse.get(t)?.dados;
  const sit = situacaoTse(d);
  const res = S.res.get(t);
  if (!sit || !res || res.circ !== res.abr) return;
  const uv = res.estado === 'matematicamente_eleito' ? 'eleito' : res.estado === 'matematicamente_2t' ? '2t' : null;
  const ems = res.cand.map((k) => k.mat?.em).filter(Boolean).sort();
  if (uv !== sit || !ems.length) return;
  // horário do sistema do evento; sem ele (servidor antigo), o do BU levado ao relógio do sistema
  const emSis = S.matSis.get(t) || null;
  const tUv = emSis ? segundosF(emSis) : tseNoSistema(S.anc, segundos(ems[0])), tTse = segundosBr(d.geracao);
  if (tUv == null || tTse == null) return;
  const cmp = comparar(S.anc, tUv, tTse);
  const emUv = emSis || `${horaDe(tUv)}`;
  const it = itemConfirmou(S.N, res.circ, res.cargo, sit, emUv, d.geracao, cmp, S.anc);
  S.confirmados.set(t, { sit, uv: emUv, tse: d.geracao, tse_sistema: horaDe(cmp.tse), s: Math.round(cmp.s), min: Math.round(cmp.s / 60) });
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

// ---------------------------------------------------------------- aviso de versão nova (M-016)
// A página não recarrega sozinha quando o servidor troca de versão: quem deixou a aba aberta segue com o JavaScript antigo.
// A mensagem `estado` traz a versão do servidor; se for MAIOR que a da página, aparece um aviso discreto com "Atualizar".
// Só maior (nunca diferente): numa troca de nós um nó já está na nova e o outro ainda na antiga, e a página pode cair no
// antigo — isso não é "versão nova". O aviso é fixo na tela (não está no fluxo: nada se move) e não recarrega sozinho,
// para não interromper a leitura.
const versaoCarregada = (() => {
  try {
    const s = document.querySelector('script[src*="app.js"]');
    const v = s ? new URL(s.src, location.href).searchParams.get('v') : null;
    return v || document.querySelector('.marca-versao')?.textContent.replace(/^v/, '') || null;
  } catch { return null; }
})();
const numerosVersao = (v) => { const m = /^(\d+(?:\.\d+)*)/.exec(String(v || '')); return m ? m[1].split('.').map(Number) : null; };
/** `a` é uma versão maior que `b` ("0.9.26" > "0.9.25")? Versão ilegível: nunca. */
function versaoMaior(a, b) {
  const x = numerosVersao(a), y = numerosVersao(b);
  if (!x || !y) return false;
  for (let i = 0; i < Math.max(x.length, y.length); i++) { const d = (x[i] || 0) - (y[i] || 0); if (d) return d > 0; }
  return false;
}
function verificarVersao(servidor) {
  if (!versaoMaior(servidor, versaoCarregada) || S.versaoDispensada === servidor) return;
  let n = S.novaVersao;
  if (!n) {
    const at = el('button', 'atualizar', 'Atualizar');
    at.type = 'button';
    at.addEventListener('click', () => { medirUso('versao-atualizar'); location.reload(); });
    const fe = el('button', 'fechar', '✕');
    fe.type = 'button';
    fe.setAttribute('aria-label', 'Dispensar o aviso de versão nova');
    fe.addEventListener('click', () => { medirUso('versao-dispensar'); S.versaoDispensada = S.novaVersaoV; n.hidden = true; });
    n = el('div', 'nova-versao', [el('span', 'nv-txt', 'Nova versão disponível'), at, fe]);
    n.setAttribute('role', 'status');
    n.hidden = true;
    document.body.append(n);
    S.novaVersao = n;
  }
  if (n.hidden) medirUso('versao-aviso');
  S.novaVersaoV = servidor;
  n.hidden = false;
}

/** Dica da primeira visita (M-015): destaque de sombra no controle e a frase "Toque em Comparar…" por 6 s, uma vez por
 * navegador (localStorage). Termina ao tocar no controle ou no ⓘ. Só no modo padrão (quem já escolheu não precisa). */
function iniciarDica() {
  if (S.dicaIniciada) return;
  S.dicaIniciada = true;
  if (dicaVista()) return;
  marcarDica();
  if (S.comparar !== 'uv') return;
  S.dicaAtiva = true;
  medirUso('fonte-dica');
  S.dicaTimer = setTimeout(fimDaDica, DICA_MS);
}
function fimDaDica() {
  if (!S.dicaAtiva) return;
  S.dicaAtiva = false;
  clearTimeout(S.dicaTimer);
  S.ui?.controle?.el.classList.remove('dica');
  S.fontesSujo = true;
  pedirQuadro();
}

/** Folha "De onde vêm os números" (botão ⓘ): a explicação e um botão por fonte (escolhe e fecha). */
function abrirInfoFonte() {
  folha.abrir({ ...INFO_FONTE, acoes: MODOS.map(([m, rot]) => [rot, () => mudarModo(m, true), S.comparar === m]) });
}

/** Troca de modo pelo controle: guarda, ajusta a tela e as inscrições (a de `tse:` liga a busca no servidor).
 * `medir`: veio de um toque do usuário (conta no log de uso, M-015). */
function mudarModo(m, medir = false) {
  if (medir) { medirUso(`fonte-${m}`); fimDaDica(); }
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

/** Cartão da previsão: o "matematicamente" é o da circunscrição (Presidente numa UF: o do Brasil); o fim, o da tela. */
function desenharPrevisao() {
  const ui = S.ui;
  if (!ui || ui.pendente) return;
  const pTela = S.prev.get(ui.topico) || null;
  const tc = `${ui.circ}-c${cargo4(ui.cargo)}`;
  const pCirc = tc === ui.topico ? pTela : S.prev.get(tc) || null;
  if (!pCirc) { ui.prev.definir(pTela, tc === ui.topico, S.N.em(ui.circ)); return; }
  const p = pTela === pCirc ? pTela : { ...pCirc, fim99: pTela?.fim99 ?? null, fim100: pTela?.fim100 ?? null, chegou: pTela?.chegou ?? {}, instante: `${pCirc.instante}|${pTela?.instante || ''}` };
  ui.prev.definir(p, true, S.N.em(ui.circ));
}

function desenharFaltam() {
  const ui = S.ui, res = S.res.get(ui.topico);
  if (!res) return;
  const f = S.faltam.get(`${res.circ}-c${cargo4(res.cargo)}`);
  const decidido = res.estado !== 'apurando' || f?.faltam === 0;
  if (decidido) {
    const t = res.estado === 'matematicamente_2t' ? 'Matematicamente 2º turno' : res.estado === 'sem_eleito' ? 'Sem eleito' : 'Matematicamente decidido';
    // M-003: o horário do sistema em que saiu (o do evento da disputa; depois, o do marco)
    const hs = S.matSis.get(`${res.circ}-c${cargo4(res.cargo)}`) || quando(f);
    const txt = `${t}${hs ? ' às ' + hora(hs) : ''}`;
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
  // B-041: o conteúdo do chip desliza junto com o número (metade dos dígitos vazios): fica centrado
  const ritmoH = new Hodometro({ inteiros: 6, centro: true });
  // o chip alterna, de leve, o BUs/min e a confirmação da sonda (camada por cima, mesma caixa: nada se mexe); tocar
  // nele abre a telemetria
  const sondaL = el('span', 'longo', ''), sondaC = el('span', 'curto', '');
  const ritmo = el('button', 'chip ritmo', [ritmoH.arrastarEl = el('span', 'ritmo-in', [ritmoH.el, el('span', null, ' BUs/min')]),
    el('span', 'ritmo-sonda', [sondaL, sondaC])]);
  if (S.consulta) ritmo.classList.add('oculto');   // B-084: desde o começo (sem mexer na faixa depois)
  ritmo.type = 'button';
  ritmo.setAttribute('aria-haspopup', 'dialog');
  ritmo.setAttribute('aria-controls', 'telemetria');
  ritmo.title = 'Telemetria da apuração: ritmo e atraso de processamento';
  ritmo.style.width = '150px';
  ritmo.addEventListener('click', () => { S.painelTele.alternar(ritmo); atualizarTelemetria(); });
  const pausa = el('span', 'chip alerta oculto', '');
  pausa.style.width = '260px';
  const alerta = el('span', 'chip alerta oculto', '');
  // M-004: a próxima eleição ao vivo, com contagem regressiva — a "propaganda" da noite oficial, sempre à vista (também
  // durante um replay); some só quando essa noite está no ar. É um botão: abre o aviso (com a playlist)
  const pMarca = el('span', 'vivo', [el('i', 'pt'), el('span', 'longo', 'Próxima eleição'), el('span', null, 'AO VIVO')]);
  const pNome = el('span', 'nome', '');
  const pQuando = el('span', 'quando', '');
  const pCont = el('span', 'cont', '');
  // 0.9.19 (Luciano, 24/10): o aviso de que os BUs do exterior chegam a partir da 01h, e o texto do chip rodando como um
  // letreiro quando não cabe (o selo AO VIVO fica parado; o resto anda dentro da janela, com uma cópia emendada atrás)
  const pExt = el('span', 'ext', '');
  const pA = el('span', 'promo-a', [pNome, pQuando, pCont, pExt]);
  const pB = el('span', 'promo-b');
  pB.setAttribute('aria-hidden', 'true');
  const pTrilho = el('span', 'promo-trilho', [pA, pB]);
  const pJanela = el('span', 'promo-janela', pTrilho);
  // aviso da totalização antes das 17h (BUs do exterior): botão que abre a explicação
  const cedo = el('button', 'chip cedo oculto', [el('span', 'ic', 'i'), el('span', null, 'Oficial às 17h'), el('span', 'longo', '\u00a0· por enquanto, só BUs do exterior')]);
  cedo.type = 'button';
  cedo.title = 'Por que o Urna Velox já está totalizando antes das 17h';
  cedo.addEventListener('click', () => folha.abrir(CEDO));
  const promo = el('button', 'chip promo oculto', [pMarca, pJanela]);
  promo.type = 'button';
  promo.addEventListener('click', () => abrirAviso());
  // B-026: WebSocket recusado (muitos acessos) → a página consulta a cada minuto; aviso discreto no lugar do BUs/min
  const http = el('span', 'chip http oculto', 'atualizando a cada minuto (muitos acessos)');
  http.title = TEXTO_HTTP;
  // M-019: o TSE não publica BU novo há PARADO_S (depois das 17h); some sozinho no primeiro BU novo
  const paradoTxt = el('span', null, '');
  const parado = el('span', 'chip parado oculto', [el('span', 'ic', '!'), paradoTxt, el('span', 'longo', ' · retomamos automaticamente quando voltarem')]);
  parado.setAttribute('role', 'status');
  // M-025: o seletor de pleito saiu da faixa (a eleição da barra é o seletor); a faixa fica só com o que muda na noite.
  // B-077: o ritmo à frente de tudo o que aparece e some (a promo, o aviso de TSE parado, o das 17h): nada do que está
  // à vista anda (CLS 0). No celular o aviso de TSE parado toma o lugar do ritmo (que está em 0)
  $('#faixa').replaceChildren(ritmo, http, parado, promo, cedo, pausa, alerta);
  faixa = { ritmoH, ritmo, sondaL, sondaC, pausa, alerta, promo, pNome, pQuando, pCont, pExt, pA, pB, pTrilho, pJanela, http, cedo, parado, paradoTxt };
}

// ---------------------------------------------------------------- sonda no chip e telemetria
const CICLO_MS = 9000, SONDA_MS = 3500; // a cada 9 s, 3,5 s com a confirmação da sonda no lugar do BUs/min

/** Texto e camada do chip: a confirmação da sonda aparece por SONDA_MS a cada CICLO_MS (sempre, com o caminho parado). */
function desenharChipSonda() {
  if (!faixa) return;
  const agora = performance.now();
  const t = textoChip(S.tele, agora, S.conexao === 'vivo');
  const c = faixa.ritmo;
  const sit = t?.situacao || '';
  if (c.dataset.sonda !== sit) c.dataset.sonda = sit;
  if (t) {
    if (faixa.sondaL.textContent !== t.longo) faixa.sondaL.textContent = t.longo;
    if (faixa.sondaC.textContent !== t.curto) faixa.sondaC.textContent = t.curto;
  }
  // M-023: se o sistema está operando e processando BUs (ritmo > 0), a própria operação já indica
  // que está tudo ok; o checkup no chip só aparece quando ocioso/esperando BUs (ou em alerta de atraso).
  const ocioso = !S.ritmoAtual;
  const mostrar = !!t && (sit === 'atrasada' || (ocioso && (Date.now() % CICLO_MS) < SONDA_MS));
  if (c.classList.contains('mostra-sonda') !== mostrar) c.classList.toggle('mostra-sonda', mostrar);
  const rot = `${S.ritmoAtual == null ? '' : `${mil(S.ritmoAtual)} BUs por minuto. `}${t ? `${t.longo.replace('✓', 'Confirmado:').replace('⚠', 'Atenção:')}. ` : ''}Toque para ver a telemetria.`;
  if (c.getAttribute('aria-label') !== rot) c.setAttribute('aria-label', rot);
}

function atualizarTelemetria() {
  const p = S.painelTele;
  if (!p?.aberta()) return;
  const t = S.tele, agora = performance.now(), lb = S.estado?.latencia_bu_ms;
  p.atualizar({ situacao: t.situacao(agora, S.conexao === 'vivo'), conectado: S.conexao === 'vivo', suportada: t.suportada, ritmo: S.ritmoAtual,
    atrasoMs: t.atrasoMs(), atrasoMedMs: t.atrasoMedianoMs(), envioMs: t.envioMs(), idadeMs: t.idadeMs(agora), limiteMs: t.limiteMs(),
    intervaloMs: t.intervaloMs(), recebidas: t.recebidas, pulos: t.pulos, ultima: t.ultima, seq: S.seqMax ?? null, remotas: t.remotas,
    buMs: lb?.n ? lb.p50 : null, buP99: lb?.n ? lb.p99 : null });
}

// ---------------------------------------------------------------- M-004: modo, próxima eleição ao vivo e aviso do dia
/** Agora no relógio do servidor da playlist (o do visitante pode estar errado). */
const agoraPl = () => Date.now() + (S.pl?.desvio || 0);

/** A promoção da próxima eleição ao vivo, se há uma e ela não é a que está no ar: { nome, inicio, fim, agora, aoVivo }. */
function promoAtual() {
  // B-062: antes do primeiro estado a página não sabe se a noite já está ao vivo; anunciar a próxima eleição aí punha o
  // item no letreiro, e um item à vista só sai quando passa da borda esquerda (ficava anunciando na madrugada)
  if (!S.estado) return null;
  const p = proximaDe(S.pl);
  if (!p) return null;
  const agora = agoraPl();
  if (agora >= p.fim) return null;
  const aoVivo = agora >= p.inicio;
  // a noite oficial está na tela: não há o que anunciar — também antes das 17h, quando os BUs do exterior já chegam
  // (B-050: o selo dizia AO VIVO e a faixa, "Próxima eleição AO VIVO · faltam 15h")
  if (modoDe(S.estado) === 'ao_vivo') return null;
  return { ...p, agora, aoVivo };
}

/** Eleições gerais têm voto no exterior: os BUs de lá podem sair a partir da 01h (Brasília) do dia da eleição (no 1º
 * turno de 2026, o primeiro saiu às 01:49). Antes da 01h: "a partir da 01h"; depois, até o primeiro BU (quando a noite
 * passa a "ao vivo" e o chip some): "já podem chegar". '' para as outras eleições. */
function textoExterior(p) {
  if (!/Elei[çc][õo]es Gerais/i.test(p.nome) || p.aoVivo) return '';
  return p.agora < p.inicio - 16 * 3600e3 ? 'BUs do exterior a partir da 01h' : 'BUs do exterior já podem chegar';
}

/** Letreiro do chip: anda só se o texto não cabe na janela (e o sistema não pede menos movimento). Mede uma vez por
 * segundo, junto com a contagem; a animação é só transform (nada na página se mexe). */
const menosMovimento = window.matchMedia?.('(prefers-reduced-motion: reduce)');
function rolarPromo(f) {
  const larg = f.pA.scrollWidth, janela = f.pJanela.clientWidth;
  const rola = larg > janela + 1 && !menosMovimento?.matches;
  if (f.promo.classList.contains('rolando') !== rola) f.promo.classList.toggle('rolando', rola);
  if (!rola) { if (f.pB.childNodes.length) f.pB.replaceChildren(); return; }
  if (f.pB.innerHTML !== f.pA.innerHTML) f.pB.innerHTML = f.pA.innerHTML;
  const passo = larg + 40;   // 40 px: o espaço entre o fim do texto e a cópia que vem atrás (margem do .promo-b)
  if (Math.abs((f.passo || 0) - passo) > 4) {   // (algarismos de largura fixa: a contagem não mexe no passo)
    f.passo = passo;
    f.pTrilho.style.setProperty('--passo', `${passo}px`);
    f.pTrilho.style.setProperty('--dur', `${(passo / 40).toFixed(1)}s`);   // ~40 px por segundo
  }
}

function textoPromo(p) {
  const q = quandoTxt(p.inicio, p.agora);
  const cont = p.aoVivo ? null : contagemPartes(p.inicio - p.agora);
  return { q, cont, linha: `${p.nome}: ${q.longo}`, falta: cont ? `faltam ${cont.join(' ')}` : 'agora' };
}

// ---------------------------------------------------------------- totalização antes das 17h (pedido do Luciano, 23/10)
// No Brasil a totalização começa oficialmente às 17h (Brasília), quando termina a votação no país. Os boletins de urna
// do exterior saem antes: no 1º turno de 2026 o TSE publicou o primeiro às 01:49 (Wellington) e 781 das 1.310 seções do
// exterior antes das 17h. O Urna Velox totaliza cada BU assim que ele é publicado; até as 17h, a página explica.

/** Agora no relógio do serviço (segundos civis de Brasília; no replay, a hora da noite reproduzida), ou null. */
function agoraSite() {
  const r = S.relogio;
  return r && Number.isFinite(r.t) ? r.t + ((performance.now() - r.em) / 1000) * (r.x || 0) : null;
}

/** 17h do dia da eleição (mesma escala): do primeiro BU da noite (marco do servidor) ou, sem ele, da playlist. */
function inicioOficial() {
  for (const m of S.marcos.values()) {
    if (m.classe !== 'primeira_secao' || m.abrangencia !== 'br') continue;
    const t = segundos(m.horario);
    if (t != null) return Math.floor(t / 86400) * 86400 + 17 * 3600;
  }
  const p = proximaDe(S.pl);
  return p ? p.inicio / 1000 - 3 * 3600 : null;
}

/** A totalização está no ar e ainda não são 17h do dia da eleição. */
function antesDas17() {
  const m = modoDe(S.estado);
  if ((m !== 'ao_vivo' && m !== 'replay') || S.estado?.encerrada) return false;
  const t0 = inicioOficial(), agora = agoraSite();
  return t0 != null && agora != null && agora < t0 && agora >= t0 - 17 * 3600;
}

const CEDO = { titulo: 'Totalização antes das 17h', linhas: [
  ['Oficial', 'No Brasil, a totalização dos votos começa oficialmente às 17h (horário de Brasília), quando termina a votação em todo o país.'],
  ['Exterior', 'Os eleitores no exterior votam no horário local. Onde a votação termina antes, como na Nova Zelândia, na Austrália, na Ásia e na Europa, o TSE publica os boletins de urna (BUs) a partir de 1h da madrugada de domingo.'],
  ['Urna Velox', 'O Urna Velox totaliza cada boletim de urna assim que o TSE o publica. Por isso começa a totalizar antes das 17h: até lá, os números são só dos eleitores do exterior.'],
] };

function desenharCedo() {
  const cedo = antesDas17(), f = faixa;
  if (f.cedo.classList.contains('oculto') === cedo) f.cedo.classList.toggle('oculto', !cedo);
  const it = feed.porId.get('cedo');
  if (!cedo) { if (it) it.sair = true; return; }
  feed.por({ id: 'cedo', fixo: true, cls: 'cedo', ic: 'i', texto: 'Totalização oficial às 17h: por enquanto, só os boletins de urna do exterior', t: '', detalhe: CEDO });
  const x = feed.porId.get('cedo');
  if (x) x.sair = false;
}

// ---------------------------------------------------------------- M-019: TSE sem novos BUs (Luciano, 25/10, 17h42)
/** Sem BU novo no TSE há este tempo (s, relógio do site), depois das 17h: aviso no topo. */
const PARADO_S = 600;

/** Horário (segundos civis) do último BU no TSE que a página conhece: do `estado` (≤ 10 s) ou do resultado na tela. */
function ultimoBuSite() {
  const a = segundosBr(S.estado?.ultimo_bu?.site), b = segundos(S.res.get(S.tela?.topico)?.ultimo_bu);
  return a == null ? b : b == null ? a : Math.max(a, b);
}

/** O horário do último BU se a noite ao vivo está parada (TSE sem BU novo há PARADO_S depois das 17h); senão null.
 * Reavaliado a cada segundo (desenharPromo): o primeiro BU novo apaga o aviso. */
function tseParado() {
  if (modoDe(S.estado) !== 'ao_vivo' || S.estado?.encerrada) return null;
  const agora = agoraSite(), t0 = inicioOficial(), tb = ultimoBuSite();
  if (agora == null || t0 == null || tb == null || agora < t0) return null;
  return agora - Math.max(tb, t0) >= PARADO_S ? tb : null;
}

function desenharParado() {
  const tb = tseParado(), f = faixa;
  if (f.parado.classList.contains('oculto') === (tb != null)) f.parado.classList.toggle('oculto', tb == null);
  if (tb == null) return;
  const d = new Date(tb * 1000), hh = String(d.getUTCHours()).padStart(2, '0'), mm = String(d.getUTCMinutes()).padStart(2, '0');
  const agora = agoraSite();
  const deltaS = agora != null && agora > tb ? agora - tb : 0;
  const min = Math.floor(deltaS / 60);
  // M-023: tempo decorrido do silêncio do TSE além da hora de corte ("desde 17h11")
  const tempoTxt = min >= 60
    ? `${Math.floor(min / 60)}h${min % 60 ? `${min % 60}m` : ''}`
    : `${min} min`;
  const txtCurto = `TSE sem novos BUs há ${tempoTxt}`;
  const txtLongo = `TSE sem novos BUs há ${tempoTxt} (desde ${hh}h${mm})`;
  const txt = window.innerWidth <= 640 ? txtCurto : txtLongo;
  if (f.paradoTxt.textContent !== txt) f.paradoTxt.textContent = txt;
}

/** Faixa e letreiro, a cada segundo: só o texto muda (o chip só muda de largura quando a contagem troca de unidade). */
function desenharPromo() {
  desenharParado();
  desenharCedo();
  const p = promoAtual();
  const f = faixa;
  if (f.promo.classList.contains('oculto') === !!p) f.promo.classList.toggle('oculto', !p);
  const it = feed.porId.get('promo:ao-vivo');
  if (!p) { if (it) it.sair = true; return; }
  const t = textoPromo(p);
  // "2º turno das Eleições Gerais 2026" → "2º turno · Eleições Gerais 2026"
  const nome = p.nome.replace(/^(\dº turno) das /, '$1 · ');
  if (f.pNome.textContent !== nome) f.pNome.textContent = nome;
  const quando = p.aoVivo ? 'agora' : t.q.longo;
  if (f.pQuando.dataset.v !== quando) {
    f.pQuando.replaceChildren(...(p.aoVivo ? ['agora'] : [el('span', 'dia', t.q.dia), `(${t.q.ddmm}), ${t.q.hora}`]));
    f.pQuando.dataset.v = quando;
  }
  const contTxt = t.cont ? t.cont.join(' ') : '';
  if (f.pCont.dataset.v !== contTxt) {
    f.pCont.replaceChildren(...(t.cont ? [el('span', 'longo', 'faltam'), el('b', null, `${t.cont[0]} ${t.cont[1]}`), el('span', 'seg', t.cont[2] || '')] : []));
    f.pCont.dataset.v = contTxt;
  }
  const ext = textoExterior(p);
  const extTxt = ext ? `· ${ext}` : '';
  if (f.pExt.textContent !== extTxt) f.pExt.textContent = extTxt;
  rolarPromo(f);
  const titulo = p.aoVivo ? `AO VIVO agora: ${p.nome}` : `Próxima eleição AO VIVO: ${t.linha} · ${t.falta}${ext ? ` · ${ext}` : ''}`;
  if (f.promo.title !== titulo) { f.promo.title = titulo; f.promo.setAttribute('aria-label', `${titulo}. Toque para saber mais.`); }
  // letreiro: um item fixo (nunca sai pelo limite de itens), destacado
  feed.por({ id: 'promo:ao-vivo', fixo: true, cls: 'promo', ic: '●',
    texto: p.aoVivo ? `AO VIVO agora: ${p.nome}` : `Próxima eleição AO VIVO: ${t.linha}${ext ? ` · ${ext}` : ''}`, t: p.aoVivo ? '' : t.falta, detalhe: AVISO });
  const x = feed.porId.get('promo:ao-vivo');
  if (x) x.sair = false;
}

/** Folha do aviso: a próxima eleição ao vivo, o que está no ar (o replay explicado) e a playlist. */
function detalheAviso() {
  const e = S.estado;
  const modo = modoDe(e);
  const linhas = [];
  let titulo = 'Urna Velox';
  if (S.consulta) {
    titulo = 'CONSULTA: a noite de uma eleição encerrada';
    const vivo = el('a', null, '● Voltar ao que está no ar');
    vivo.href = '/';
    linhas.push(['Consulta', 'A noite inteira desta eleição, guardada minuto a minuto: a linha do tempo embaixo leva a qualquer instante, toca e acelera. Cada visitante tem a sua.'],
      ['Dados', 'Os boletins de urna reais, publicados pelo TSE, totalizados pelo Urna Velox; no fim, idênticos ao resultado oficial. O lado do TSE são os arquivos oficiais da noite ou, quando eles não foram guardados, a reconstrução pela hora em que o TSE recebeu cada boletim (marcada assim).'],
      ['Ao vivo', vivo]);
  } else if (modo === 'replay') {
    titulo = 'REPLAY: dados reais, totalizados de novo';
    linhas.push(['Replay', 'A apuração de uma eleição que já aconteceu, passando de novo — como a reprise de um jogo.'],
      ['Dados', 'Os boletins de urna reais, publicados pelo TSE: nenhum voto é inventado. A totalização recomeça do zero, na ordem em que os boletins chegaram à Justiça Eleitoral, e no fim é conferida com o resultado oficial.'],
      ['No ar', e.simulacao.rotulo]);
  } else if (modo === 'espera') {
    titulo = 'A apuração ainda não começou';
    linhas.push(['No ar', S.pl?.no_ar?.modo === 'espera' && S.pl.no_ar.modo_descricao ? S.pl.no_ar.modo_descricao : 'Os candidatos já carregados, à espera dos primeiros boletins de urna.']);
  } else if (modo === 'ao_vivo') {
    titulo = e?.encerrada ? 'Apuração encerrada' : 'AO VIVO: totalização em tempo real';
    linhas.push(['No ar', 'A totalização da eleição com os boletins de urna publicados pelo TSE, à medida que chegam.']);
  }
  const p = promoAtual();
  if (p) {
    const t = textoPromo(p);
    if (modo === 'espera' || modo === '') titulo = p.aoVivo ? `AO VIVO agora: ${p.nome}` : 'Próxima eleição AO VIVO';
    // em destaque, no alto: "AO VIVO · 2º turno das Eleições Gerais 2026 / domingo (25/10), a partir das 17h · faltam …"
    linhas.unshift([el('span', 'aviso-selo', 'AO VIVO'),
      el('strong', 'aviso-vivo', p.aoVivo ? [`Agora: ${p.nome}`] : [p.nome, el('br'), `${t.q.dia} (${t.q.ddmm}), a partir das ${t.q.hora} · ${t.falta}`])]);
    // 0.9.19: os BUs do exterior chegam antes (a partir da 01h do dia da eleição)
    if (textoExterior(p)) {
      linhas.splice(1, 0, ['Exterior', `A partir da 01h do dia da eleição (${t.q.ddmm}), o TSE já pode publicar os boletins de urna (BUs) do exterior, e o Urna Velox totaliza cada um assim que sai. A totalização oficial no Brasil começa às 17h.`]);
    }
  }
  // M-012: com o selo escondido (atributo hidden no #conexao, até o fim da totalização do 2º turno), o aviso também não
  // leva à playlist; tirar o hidden traz os dois de volta
  if (S.pl && !$('#conexao')?.hidden) {
    const link = el('a', null, '▶ Playlist: a agenda dos replays e a votação do próximo');
    link.href = '/playlist';
    linhas.push(['Playlist', link]);
  }
  // multipleito (1.1): todos os pleitos do site de resultados do TSE, com a situação de cada um
  if (S.pleitos) {
    const b = el('button', 'pl-abrir', [el('span', null, 'Ver todos os pleitos'), el('small', null, resumoTxt(S.pleitos))]);
    b.type = 'button';
    b.addEventListener('click', () => { folha.fechar(); S.painelPleitos.abrir(); });
    linhas.push(['Pleitos', b]);
  }
  return { titulo, linhas };
}

/** O detalhe do item do letreiro: montado na hora do toque (não a cada segundo). */
const AVISO = { get titulo() { return detalheAviso().titulo; }, get linhas() { return detalheAviso().linhas; } };

function abrirAviso() {
  const d = detalheAviso();
  if (d.linhas.length) folha.abrir(d);
}

/** Na primeira entrada do dia (o dia do visitante, B-023) o aviso abre sozinho: num replay ou com a próxima eleição ao
 * vivo anunciada; nunca com a noite oficial no ar. Espera a playlist (no máximo 3 s) para já vir completo. */
function talvezAvisar() {
  if (S.avisado || !S.estado || !S.plPronta) return;
  const modo = modoDe(S.estado);
  if (modo === 'ao_vivo') { S.avisado = true; return; }
  if (modo !== 'replay' && !promoAtual()) return;
  S.avisado = true;
  const hoje = diaLocal();
  let visto = null;
  try { visto = localStorage.getItem('uv-aviso-dia'); } catch { /* sem armazenamento */ }
  if (visto === hoje) return;
  try { localStorage.setItem('uv-aviso-dia', hoje); } catch { /* sem armazenamento */ }
  abrirAviso();
}

/** O seletor de pleito (faixa): o nome do pleito em exibição (consulta ou no ar); sem a lista, "Pleitos". */
/** B-087: "Gerais", "Municipais" ou "Suplementar" — pela lista dos pleitos (as eleições do pleito na tela) e, sem ela,
 *  pelos cargos (federais e estaduais = Gerais). Antes, sem tópico do Brasil era sempre "Municipais" (o suplementar de
 *  Governador de RR aparecia como municipal). */
function tipoEleicao(cargo) {
  const cod = String(S.consulta?.pleito ?? S.estado?.pleito ?? '');
  const p = cod && S.pleitos?.pleitos?.find((x) => String(x.codigo) === cod);
  const tipos = (p?.eleicoes || []).map((e) => e.tipo);
  if (tipos.length && tipos.every((t) => t === 'suplementar')) return 'Suplementar';
  if (p?.titulo) { if (/Gerais/.test(p.titulo)) return 'Gerais'; if (/Municipais/.test(p.titulo)) return 'Municipais'; }
  const c = cargo ?? S.tela?.cargo;
  return [...S.topicos].some((x) => x.startsWith('br-')) || [1, 3, 5, 6, 7, 8].includes(c) ? 'Gerais' : 'Municipais';
}

/** M-025: a lista dos pleitos chegou (ou mudou): o nome e a data da eleição na barra. */
function desenharSeletor() {
  if (S.tela && S.ident) S.ident.eleicao(eleicaoAtual());
}

/** Pleitos do TSE (multipleito, 1.1): na partida e a cada ~10 min (o uv-pleitos.sh relê o ele-c.json a cada 15 min). */
async function acompanharPleitos() {
  S.consultas = new Set();
  // M-024: "no ar" é o que o serviço totaliza (na consulta, o da playlist); "na tela", o que a página mostra
  S.painelPleitos = new PainelPleitos({ noAr: () => S.pl?.no_ar?.pleito ?? (S.consulta ? null : S.estado?.pleito), comPlaylist: () => !!S.pl, consultas: () => S.consultas,
    naTela: () => S.consulta?.pleito ?? S.estado?.pleito, emConsulta: () => !!S.consulta });
  // M-024/M-025: a eleição da barra abre a lista
  $('#eleicao .el-pleito').addEventListener('click', () => S.painelPleitos.abrir());
  buscarConsultas().then((c) => { S.consultas = c; if (S.pleitos) S.painelPleitos.definir(S.pleitos); }).catch(noConsole('consultas'));
  const buscar = async () => {
    const l = await buscarPleitos();
    if (l) { S.pleitos = l; S.painelPleitos.definir(l); desenharSeletor(); document.documentElement.classList.add('com-pleitos'); }
    setTimeout(buscar, l ? 10 * 60e3 + Math.random() * 60e3 : 60e3 + Math.random() * 30e3);
  };
  await buscar();
}

/** Playlist: na partida e a cada ~15 min (a agenda muda pouco; o uv-playlist é um só, no nó A). */
async function acompanharPlaylist() {
  const prazo = new Promise((ok) => setTimeout(ok, 3000));
  const primeira = buscarPlaylist();
  const pl = await Promise.race([primeira, prazo.then(() => undefined)]);
  if (pl !== undefined) S.pl = pl;
  S.plPronta = true;
  S.estadoSujo = true;
  pedirQuadro();
  if (pl === undefined) { S.pl = await primeira; S.estadoSujo = true; pedirQuadro(); }
  // B-024: sem a playlist (um nó fora do ar, uma falha de rede), tenta de novo em ~30 s em vez de esperar 15 min — a
  // contagem para a próxima eleição ao vivo tem de estar sempre à vista
  const proxima = () => setTimeout(async () => {
    const n = await buscarPlaylist();
    if (n) S.pl = n;
    S.estadoSujo = true; pedirQuadro(); proxima();
  }, S.pl ? 15 * 60e3 + Math.random() * 60e3 : 25e3 + Math.random() * 10e3);
  proxima();
}

function desenharEstado() {
  S.estadoSujo = false;
  const e = S.estado;
  const c = $('#conexao');
  // M-004: o selo diz o modo (REPLAY em âmbar, AO VIVO, aguardando a apuração) e, no ponto e no complemento, a conexão
  const modo = S.conexao === 'vivo' || e ? modoDe(e) : '';
  // B-026: WebSocket recusado (muitos acessos) → consultas por HTTP a cada minuto; o selo mantém o modo
  const http = S.conexao === 'http';
  const ligada = S.conexao === 'vivo' || http;
  // (espera: "EM BREVE" cabe inteiro no selo do celular, 136 px; "AGUARDANDO" era cortado)
  // consulta de um pleito encerrado (1.1, serie.js): a linha do tempo do visitante, não o que está no ar
  const nomeModo = S.consulta ? 'CONSULTA' : { replay: 'REPLAY', ao_vivo: 'AO VIVO', espera: 'EM BREVE' }[modo] || '';
  let est = S.conexao, txt = nomeModo || 'conectando…';
  if (S.conexao === 'conectando' && !e) txt = 'conectando…';
  if (S.conexao === 'caiu') {
    const s = Math.max(0, Math.round(((S.conexaoInfo?.em || Date.now()) - Date.now()) / 1000));
    txt = nomeModo ? `${nomeModo} · reconectando ${s}s` : `conexão caiu · volta em ${s}s`;
    clearTimeout(S.tCaiu);
    S.tCaiu = setTimeout(() => { S.estadoSujo = true; pedirQuadro(); }, 1000);
  }
  if (http && !e) txt = 'a cada minuto';
  if (ligada && e?.pausa_429?.ativa) { est = 'pausa'; txt = `${nomeModo ? `${nomeModo} · ` : ''}TSE pediu pausa`; }
  if (ligada && e?.encerrada) { est = 'encerrada'; txt = S.consulta ? 'CONSULTA' :   /* B-084: o fim já está no slider; "CONSULTA · fim" não cabia no celular */ modo === 'replay' ? 'REPLAY · fim' : 'apuração encerrada'; }
  // B-026: o aviso "a cada minuto" toma o lugar do BUs/min (que, com uma consulta por minuto, não diria nada)
  // B-084: o ritmo (BUs/min do serviço) não diz nada na consulta, que anda no relógio de quem vê
  if (faixa?.ritmo && faixa.ritmo.classList.contains('oculto') !== !!S.consulta) faixa.ritmo.classList.toggle('oculto', !!S.consulta);
  if (faixa?.http && faixa.http.classList.contains('oculto') === http) { faixa.http.classList.toggle('oculto', !http); faixa.ritmo.classList.toggle('com-http', http); }
  if (c.dataset.estado !== est) c.dataset.estado = est;
  if (c.dataset.modo !== modo) c.dataset.modo = modo;
  const t = c.querySelector('.txt');
  if (t.textContent !== txt) t.textContent = txt;
  // o selo também é o acesso à playlist (o aviso que ele abre tem o link)
  const pl = c.querySelector('.pl');
  // B-084: na consulta o selo não leva à playlist (a consulta é de um pleito só): "CONSULTA" cabe inteiro no celular
  const comPl = !!S.pl && modo !== 'ao_vivo' && !S.consulta;
  if (pl.classList.contains('oculto') === comPl) pl.classList.toggle('oculto', !comPl);
  const rot = e?.simulacao?.rotulo || '';
  const dica = S.consulta
    ? 'CONSULTA de uma eleição encerrada: a noite inteira guardada minuto a minuto, com os boletins de urna reais publicados pelo TSE. Use a linha do tempo para ir a qualquer instante, tocar e acelerar; o lado do TSE mostra os arquivos oficiais da noite, ou a reconstrução pela hora de recebimento quando eles não foram guardados.'
    : modo === 'replay'
    ? `REPLAY com dados reais: os boletins de urna são os publicados pelo TSE, totalizados de novo, do zero, na ordem em que chegaram à Justiça Eleitoral (${rot}); no fim, o resultado é conferido com o oficial. O relógio mostra o horário da noite da eleição.`
    : modo === 'espera' ? 'A apuração ainda não começou.' : modo === 'ao_vivo' ? 'Totalização ao vivo.' : '';
  const dicaHttp = http ? ` ${TEXTO_HTTP}` : '';
  if (c.title !== dica + dicaHttp) { c.title = dica + dicaHttp; c.setAttribute('aria-label', `${txt}${dica + dicaHttp ? `. ${dica}${dicaHttp}` : ''} Toque para saber mais${comPl ? ' e ver a playlist' : ''}.`); }
  talvezAvisar();
  desenharPromo();
  if (!e) return;
  // M-025: o pleito do serviço pode chegar depois da lista dos pleitos (o nome e a data da eleição dependem dos dois)
  if (S.pleitoNaBarra !== e.pleito) { S.pleitoNaBarra = e.pleito; desenharSeletor(); }
  const pz = e.pausa_429?.ativa ? `Pausa de rede (HTTP 429)${e.pausa_429.retomada_em ? ' até ' + hora(e.pausa_429.retomada_em) : ''}` : '';
  if (faixa.pausa.textContent !== pz) { faixa.pausa.textContent = pz; faixa.pausa.title = pz; faixa.pausa.classList.toggle('oculto', !pz); }
  const al = e.alertas || {};
  const avisos = [];
  if (al.secoes_esperadas_incoerentes) avisos.push('o % de seções de algumas UFs pode estar errado (seções esperadas incoerentes)');
  // M-010 (24/10/2026, Luciano): o aviso da destinação presumida sai da página até haver uma forma de apresentá-lo
  // que não deixe dúvidas; o alerta continua no estado (/api/v1/estado), nos logs e no 30_verificar.sh
  // if (al.destinacao_presumida) avisos.push('destinação dos votos presumida "Válido"');
  if (al.reenvio_diferente || al.anulacao) avisos.push('houve BU reenviado ou seção anulada');
  const at = avisos.length ? `Atenção: ${avisos.join('; ')}` : '';
  if (faixa.alerta.textContent !== at) { faixa.alerta.textContent = at; faixa.alerta.title = at; faixa.alerta.classList.toggle('oculto', !at); }
}

// ---------------------------------------------------------------- ritmo da apuração (BUs por minuto)
/**
 * B-053: uma amostra do ritmo pelo `estado` (BUs aplicados e o relógio do serviço na mesma mensagem). No relógio da
 * página, uma pausa sem mensagens (celular em segundo plano, WebSocket reconectando) e o salto do seq na volta davam
 * centenas de milhares de BUs/min: os BUs de minutos do serviço divididos pelos poucos segundos da página.
 */
function amostrarRitmo(seq, t, sessao) {
  if (t == null) return;
  if (sessao !== S.ritmoSessao) { S.ritmoSessao = sessao; S.ritmoSrv = []; }
  const r = S.ritmoSrv, u = r[r.length - 1];
  // outro nó um pouco atrás (modo HTTP, reconexão) ou mensagem fora de ordem: a série só anda para a frente
  if (u && (t * 1000 <= u[0] || seq < u[1])) return;
  r.push([t * 1000, seq]);
  while (r.length > 2 && t * 1000 - r[0][0] > 60000) r.shift();
}

// ---------------------------------------------------------------- curiosidades calculadas aqui (a cada 2 s)
function curiosidades() {
  // ritmo: BUs por minuto, janela de 60 s; no relógio do serviço (B-053) ou, servidor antigo, no da página
  const agora = performance.now();
  if (S.seqMax != null) S.ritmo.push([agora, S.seqMax]);
  while (S.ritmo.length > 2 && agora - S.ritmo[0][0] > 60000) S.ritmo.shift();
  const r = S.ritmoSessao !== undefined ? S.ritmoSrv : S.ritmo;
  if (r.length >= 2) {
    const [t0, s0] = r[0], [t1, s1] = r[r.length - 1];
    S.ritmoAtual = t1 > t0 ? Math.round(((s1 - s0) * 60000) / Math.max(5000, t1 - t0)) : 0;
  } else S.ritmoAtual = 0;
  faixa.ritmoH.definir(S.ritmoAtual);
  if (S.primeirosSujo) { S.primeirosSujo = false; for (const it of primeiros(S.N, S.eventos.values())) porNoFeed(it); }
  // B-014: "disputa mais apertada / mais folgada" e "comparecimento por UF" saíram do letreiro: precisavam do resultado
  // de todos os tópicos do país, e a página agora só recebe o da tela (podem voltar como marcos calculados no servidor)
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

/** Consulta de um pleito: o slider voltou no tempo — a página esquece o que viu e recebe tudo de novo até o instante. */
function reiniciarConsulta() {
  S.res = new Resultados();
  for (const k of ['eventos', 'marcos', 'faltam', 'tse', 'prev', 'matSis', 'confirmados', 'ordemDesde', 'prevConfirmadas']) S[k]?.clear?.();
  S.seqMax = 0;
  S.ritmo = [];
  S.ritmoSrv = [];
  S.primeiraUf100 = null;
  feed?.limpar();
  hist?.limpar();
  montarTela();
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
  folha.aoFechar = () => feed.retomar();   // B-089
  hist = new Historico($('#hist'), $('#feed-hist'));
  // a fonte embutida antes do primeiro desenho dos números (sem troca de fonte depois: nada de layout shift)
  const fonte = document.fonts.load('700 20px "Archivo UV"').catch(() => null);
  // consulta de um pleito encerrado (multipleito, 1.1; serie.js): a série estática no lugar do WebSocket, com o slider
  const rc = rotaConsulta(location.hash);
  const serie = rc ? await carregarSerie(rc.pleito) : null;
  if (rc && !serie) history.replaceState(null, '', location.pathname + location.search);
  if (serie) document.documentElement.classList.add('com-replay');
  const [nomes, topicos, player] = await Promise.all([
    serie ? serie.nomes : fetch('/api/v1/nomes').then((r) => (r.ok ? r.json() : null)).catch(() => null),
    serie ? serie.topicos : fetch('/api/v1/topicos').then((r) => (r.ok ? r.json() : [])).catch(() => []),
    // barra do replay só no site simulado (na consulta, o slider local); montada (e o espaço dela reservado) antes do primeiro desenho
    serie ? new PlayerLocal(serie.indice, (t) => S.con?.reposicionar(t)) : iniciarPlayer(),
    fonte,
  ]);
  S.player = player;
  // a faixa já nasce sabendo que é consulta (B-084: sem o chip do ritmo)
  if (serie) { S.consulta = { pleito: serie.pleito }; S.serieInicio = serie.indice?.inicio_texto || ''; }
  // M-025: a barra da eleição (o local e os cargos levam a outra tela)
  S.ident = new Identificacao({ aoLocal: (a) => irPara(a), aoCargo: (c) => S.tela && irPara(S.tela.abr, c) });
  await document.fonts.ready;
  montarFaixa();
  S.painelTele = new PainelTelemetria();
  // chip da sonda (alternância) e telemetria aberta: uma vez por segundo
  setInterval(() => { desenharChipSonda(); atualizarTelemetria(); }, 1000);
  // M-004: o selo abre o aviso (com a playlist); a contagem anda a cada segundo
  $('#conexao').addEventListener('click', () => abrirAviso());
  acompanharPlaylist().catch(noConsole('playlist'));
  acompanharPleitos().catch(noConsole('pleitos'));
  setInterval(() => desenharPromo(), 1000);
  S.N = new Nomes(nomes);
  S.topicos = new Set(topicos.map((t) => t.topico));
  if (serie) {
    S.avisado = true;
    S.con = new ConexaoSerie(receber, estadoConexao, serie, `Consulta · pleito ${serie.pleito}`);
    S.con.aoReiniciar = reiniciarConsulta;
    // pleito em curso: o slider se estende com a noite; "ao vivo" volta ao tempo real (a página ao vivo)
    S.con.aoIndice = (ind) => S.player?.estender(ind.fim);
    S.player.aoVivo = () => { location.hash = `#/${S.tela?.abr || 'br'}/c${cargo4(S.tela?.cargo ?? 1)}`; };
    setInterval(() => S.con.avancar(S.player.agora()), 250);
  } else S.con = new Conexao(receber, estadoConexao);
  // mapa das seções (mapa.js) e cartões que se maximizam e se reorganizam (cartoes.js)
  const versao = new URL(import.meta.url).searchParams.get('v') || '';
  S.mapa = new CartaoMapa({ obter: (c) => S.con.obter(c), pedirMapa: (f) => S.con.pedirMapa(f), nomes: () => S.N, versao });
  S.con.aoConsultar = () => S.mapa?.consultar();
  S.cartoes = new Cartoes({ aoMaximizar: (c, sim) => { if (c === S.mapa?.el) S.mapa.maximizado(sim); } });
  window.addEventListener('hashchange', () => {
    // entrar na consulta de um pleito, sair dela ou trocar de pleito: a página recomeça
    const p = rotaConsulta(location.hash)?.pleito ?? null;
    if (p !== (S.consulta?.pleito ?? null)) { location.reload(); return; }
    montarTela();
  });
  // aba escondida: cancela a comparação com o TSE (e a refaz ao voltar)
  document.addEventListener('visibilitychange', () => { if (S.comparar !== 'uv') atualizarTopicos(); });
  // a idade do arquivo do TSE anda mesmo sem mensagens: um quadro por segundo enquanto a comparação está ligada
  setInterval(() => { if (S.comparar !== 'uv') pedirQuadro(); }, 1000);
  montarTela();
  S.con.abrir();
}

iniciar().catch(noConsole('início'));
