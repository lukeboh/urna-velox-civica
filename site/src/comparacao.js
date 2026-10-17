// Comparação Urna Velox × TSE (etapa 7, docs/HANDOFF_0.7.0.md; protocolo: docs/PROTOCOLO_WS.md §4.6).
//
// Controle de três posições "Urna Velox · Ambos · TSE" no topo da corrida. Com "Ambos" ou "TSE", a página se inscreve
// em `tse:<tópico da tela>` — é essa inscrição que faz o servidor buscar o arquivo de resultado do TSE (no máximo uma
// vez a cada 30–60 s, e só enquanto alguém olha: aba escondida cancela). A escolha fica no navegador (localStorage, em
// try/catch) e pode vir na URL (`?comparar=ambos`, `?comparar=tse`) para compartilhar.
//
// Mesmas regras da etapa 5: nada pula (as células do TSE existem desde a montagem, com largura reservada; trocar de
// modo é um clique, que não conta no CLS), números em hodômetro (o do TSE anda em degraus, um a cada arquivo novo).

import { Hodometro } from './hodometro.js';
import { el, hora, mil, pct } from './util.js';

export const MODOS = [['uv', 'Urna Velox'], ['ambos', 'Ambos'], ['tse', 'TSE']];
const VALIDO = new Set(MODOS.map((m) => m[0]));
const CHAVE = 'uv-comparar';

/** Modo inicial: a URL (?comparar=) vence o guardado; padrão "uv". */
export function modoInicial() {
  const q = new URLSearchParams(location.search).get('comparar');
  if (VALIDO.has(q)) return q;
  try { const v = localStorage.getItem(CHAVE); if (VALIDO.has(v)) return v; } catch { /* sem armazenamento */ }
  return 'uv';
}

/** Guarda a escolha e a põe na URL (sem recarregar; o hash da rota fica). */
export function gravarModo(m) {
  try { localStorage.setItem(CHAVE, m); } catch { /* sem armazenamento */ }
  try {
    const u = new URL(location.href);
    if (m === 'uv') u.searchParams.delete('comparar'); else u.searchParams.set('comparar', m);
    history.replaceState(history.state, '', u.pathname + u.search + u.hash);
  } catch { /* history indisponível */ }
}

/** Controle segmentado (radiogroup). `aoMudar(modo)` no clique. */
export class Controle {
  constructor(modo, aoMudar) {
    this.el = el('div', 'comparar');
    this.el.setAttribute('role', 'radiogroup');
    this.el.setAttribute('aria-label', 'Fonte do resultado');
    this.bot = new Map();
    for (const [m, rot] of MODOS) {
      const b = el('button', null, rot);
      b.type = 'button';
      b.setAttribute('role', 'radio');
      b.dataset.modo = m;
      b.addEventListener('click', () => { if (this.modo !== m) aoMudar(m); });
      this.bot.set(m, b);
      this.el.append(b);
    }
    this.definir(modo);
  }
  definir(m) {
    this.modo = m;
    for (const [k, b] of this.bot) b.setAttribute('aria-checked', String(k === m));
    this.el.dataset.modo = m;
  }
}

/** Células do TSE e da diferença numa linha de candidato (sempre montadas; o CSS mostra conforme o modo). */
export function celulasTse(dig) {
  const pctH = new Hodometro({ inteiros: 3, casas: 2, sufixo: '%' });
  const votH = new Hodometro({ inteiros: dig, sufixo: 'votos' });
  const difH = new Hodometro({ inteiros: dig });
  const pos = el('span', 'pos-tse', '');
  pos.title = 'Posição no TSE';
  const sinal = el('span', 'sinal', '');
  const tse = el('span', 'tse', [el('span', 'tse-l1', [pos, el('span', 'tse-pct', pctH.el)]), el('span', 'tse-qtd', votH.el)]);
  const dif = el('span', 'dif', [el('span', 'dif-rot', 'UV − TSE'), el('span', 'dif-val', [sinal, difH.el])]);
  dif.title = 'Diferença de votos: Urna Velox − TSE';
  return { tse, dif, pos, pctH, votH, difH, sinal, ultPos: null, ultSinal: null };
}

/** Desenha as células do TSE de um candidato (`k` = o candidato no resumo do TSE; `uv` = votos no Urna Velox). */
export function desenharCelulas(c, k, uv, posUv) {
  c.pctH.definir(k ? k.pct : null);
  c.votH.definir(k ? k.v : null);
  const p = k ? `${k.pos}º` : '';
  if (c.ultPos !== p) { c.pos.textContent = p; c.ultPos = p; }
  const d = k && uv != null ? uv - k.v : null;
  c.difH.definir(d == null ? null : Math.abs(d));
  const s = d == null ? '' : d > 0 ? '+' : d < 0 ? '−' : '';
  if (c.ultSinal !== s) { c.sinal.textContent = s; c.ultSinal = s; }
  return k != null && posUv != null && k.pos !== posUv + 1;
}

/** "dd/mm/aaaa hh:mm:ss" → segundos (mesma escala de util.segundos: o horário de Brasília lido como UTC). */
export function segundosBr(s) {
  const m = /(\d{2})\/(\d{2})\/(\d{4}) (\d{2}):(\d{2}):(\d{2})/.exec(s || '');
  return m ? Date.UTC(+m[3], +m[2] - 1, +m[1], +m[4], +m[5], +m[6]) / 1000 : null;
}

/** Situação que o arquivo do TSE mostra: 'eleito', '2t' ou null (md enquanto não há totalização final; depois, st/e). */
export function situacaoTse(d) {
  if (!d) return null;
  if (d.md === 'e') return 'eleito';
  if (d.md === 's') return '2t';
  if (d.finalizada) {
    if (d.cand?.some((k) => k.st === '2º turno')) return '2t';
    if (d.cand?.some((k) => k.st === 'Eleito' || String(k.st).startsWith('Eleito'))) return 'eleito';
  }
  return null;
}

/** Texto curto do estado da fonte TSE (vazio quando ok). */
export function textoEstado(m) {
  if (!m) return 'buscando no TSE…';
  switch (m.estado) {
    case 'ok': return '';
    case 'aguardando': return 'buscando no TSE…';
    case 'desatualizado': return 'TSE desatualizado';
    case 'pausado': return 'TSE em pausa (HTTP 429)';
    case 'indisponivel': return m.motivo === 'limite_topicos' ? 'comparação indisponível no momento' : m.motivo === '404' ? 'TSE ainda sem arquivo' : 'TSE indisponível';
    default: return '';
  }
}

/** "há 40 s", "há 3 min", "há 1 h 05 min". */
export function idade(s) {
  if (s == null || !Number.isFinite(s)) return '';
  s = Math.max(0, Math.round(s));
  if (s < 90) return `há ${s} s`;
  const m = Math.round(s / 60);
  if (m < 60) return `há ${m} min`;
  return `há ${Math.floor(m / 60)} h ${String(m % 60).padStart(2, '0')} min`;
}

/** Cabeçalho das fontes: % de seções de cada uma e a idade do arquivo do TSE. */
export function textoFontes(modo, res, tse, agoraSite) {
  const tot = res?.secoes?.total || 0, ap = res?.secoes?.apuradas || 0;
  const uv = `Urna Velox ${pct(tot ? (100 * ap) / tot : 0)}% das seções`;
  const d = tse?.dados;
  const ts = d?.secoes?.total || 0, st = d?.secoes?.totalizadas || 0;
  const tTse = d ? `TSE ${pct(ts ? (100 * st) / ts : 0)}%` : 'TSE —';
  const g = segundosBr(d?.geracao);
  const arq = d?.geracao ? `arquivo das ${hora(d.geracao)}${agoraSite != null && g != null ? ` (${idade(agoraSite - g)})` : ''}` : '';
  if (modo === 'uv') return { a: uv, b: '' };
  if (modo === 'tse') return { a: tTse + (d ? ' das seções' : ''), b: arq };
  return { a: `${uv} · ${tTse}`, b: arq };
}

export { mil };
