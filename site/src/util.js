// Utilidades: números em pt-BR, nomes de cargos e abrangências, elementos.

export const movimentoReduzido = () => window.matchMedia('(prefers-reduced-motion: reduce)').matches;

export function mil(x) {
  if (x == null || !Number.isFinite(x)) return '—';
  return Math.round(x).toString().replace(/\B(?=(\d{3})+(?!\d))/g, '.');
}

export function pct(x, casas = 2) {
  if (x == null || !Number.isFinite(x)) return '—';
  return x.toFixed(casas).replace('.', ',');
}

export const NOME_CARGO = { 1: 'Presidente', 3: 'Governador', 5: 'Senador', 11: 'Prefeito' };
export const NOME_CARGO_PLURAL = { 1: 'presidente', 3: 'governador', 5: 'senador', 11: 'prefeito' };

export function nomeCargo(c, consulta) {
  if (NOME_CARGO[c]) return NOME_CARGO[c];
  return consulta || c >= 25 ? 'Consulta' : `Cargo ${c}`;
}

/** Código de abrangência → { tipo, uf, mun } (br | uf | mun). */
export function abr(cod) {
  if (cod === 'br') return { tipo: 'br', uf: null, mun: null };
  if (cod.length === 2) return { tipo: 'uf', uf: cod, mun: null };
  return { tipo: 'mun', uf: cod.slice(0, 2), mun: cod };
}

/** "SÃO PAULO" → "São Paulo" (conectivos em minúsculas). */
export function titulo(s) {
  if (!s) return '';
  const pequenas = new Set(['de', 'da', 'do', 'das', 'dos', 'e', "d'"]);
  return s.toLowerCase().split(/(\s+|-)/).map((p, i) => (i > 0 && pequenas.has(p)) ? p : p.charAt(0).toUpperCase() + p.slice(1)).join('');
}

/** Horário "aaaa-mm-dd hh:mm:ss" ou ISO → "hh:mm:ss". */
export function hora(t) {
  if (!t) return '';
  const m = /(\d{2}):(\d{2}):(\d{2})/.exec(t);
  return m ? `${m[1]}:${m[2]}:${m[3]}` : '';
}

/** Horário → segundos (para ordenar e para a curva); só hh:mm:ss e a data. */
export function segundos(t) {
  if (!t) return null;
  const m = /(\d{4})-(\d{2})-(\d{2})[T ](\d{2}):(\d{2}):(\d{2})/.exec(t);
  if (!m) return null;
  return Date.UTC(+m[1], +m[2] - 1, +m[3], +m[4], +m[5], +m[6]) / 1000;
}


export function el(tag, cls, filhos) {
  const e = document.createElement(tag);
  if (cls) e.className = cls;
  if (filhos != null) for (const f of [].concat(filhos)) e.append(f);
  return e;
}

export function svg(tag, attrs = {}) {
  const e = document.createElementNS('http://www.w3.org/2000/svg', tag);
  for (const [k, v] of Object.entries(attrs)) e.setAttribute(k, v);
  return e;
}

/** Quantos dígitos tem o inteiro (pelo menos 1). */
export function digitos(n) { return Math.max(1, Math.floor(Math.log10(Math.max(1, n))) + 1); }
