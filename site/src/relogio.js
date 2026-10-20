// Relógio do sistema (M-003, docs/PROTOCOLO_WS.md §4.3 `relogio_sistema`): os horários do Urna Velox são os da
// máquina que totalizou (a data e a hora reais, também no simulado), separados dos horários do TSE (recebimento dos
// BUs, geração dos arquivos de divulgação). Para medir a distância entre as duas apurações, a âncora do servidor diz
// que o instante `noite0` do TSE corresponde a `sistema0` do sistema, com a aceleração `acelerar` do simulado (1 na
// noite real): um instante `t` do TSE equivale a `sistema0 + (t − noite0) / acelerar` no relógio do sistema.
// Todos os instantes em segundos na escala de `segundos()` (horário civil de Brasília lido como UTC).

import { segundos } from './util.js';

/** "2026-10-18T20:41:03.512-03:00" (ou sem fração) → segundos com fração; null se não for horário. */
export function segundosF(t) {
  const s = segundos(t);
  if (s == null) return null;
  const m = /\d{2}:\d{2}:\d{2}\.(\d{1,3})/.exec(t);
  return m ? s + Number(m[1].padEnd(3, '0')) / 1000 : s;
}

/** Bloco `relogio_sistema` do `estado` → { noite0, sis0, x } (s) ou null (antes do primeiro horário do site). */
export function ancora(r) {
  const noite0 = segundos(r?.noite0), sis0 = segundosF(r?.sistema0);
  if (noite0 == null || sis0 == null) return null;
  return { noite0, sis0, x: r.acelerar > 0 ? r.acelerar : 1 };
}

/** Instante do TSE (s) → o equivalente no relógio do sistema (s). Sem âncora, o próprio instante. */
export function tseNoSistema(a, t) {
  if (t == null) return null;
  return a ? a.sis0 + (t - a.noite0) / a.x : t;
}

/** Horário do sistema da mensagem (`totalizado_em`, `sistema`), ou o do TSE quando o servidor é anterior à M-003. */
export function quando(m, campo = 'sistema') { return m?.[campo] || m?.horario || m?.ultimo_bu || null; }

/** "hh:mm:ss" de segundos (escala de `segundos`). */
export function horaDe(s) {
  if (s == null || !Number.isFinite(s)) return '';
  return new Date(Math.floor(s) * 1000).toISOString().slice(11, 19);
}

/** "dd/mm" de segundos (escala de `segundos`). */
export function diaDe(s) {
  if (s == null || !Number.isFinite(s)) return '';
  const d = new Date(Math.floor(s) * 1000).toISOString();
  return `${d.slice(8, 10)}/${d.slice(5, 7)}`;
}

/** Distância em segundos (do sistema) em texto: "no mesmo instante", "12 s antes", "3 min depois", "1 h 05 min antes". */
export function distancia(s) {
  if (s == null || !Number.isFinite(s)) return '';
  const a = Math.abs(s), lado = s > 0 ? 'antes' : 'depois';
  if (a < 1) return 'no mesmo instante';
  if (a < 90) return `${Math.round(a)} s ${lado}`;
  const m = Math.round(a / 60);
  if (m < 60) return `${m} min ${lado}`;
  return `${Math.floor(m / 60)} h ${String(m % 60).padStart(2, '0')} min ${lado}`;
}

/**
 * Comparação de dois instantes: o do Urna Velox (relógio do sistema, s) e o do TSE (s, no relógio do TSE). Devolve
 * { tse: equivalente do TSE no relógio do sistema, s: antecedência do Urna Velox em segundos do sistema (positivo =
 * Urna Velox antes), noite: a mesma antecedência em segundos da noite (× aceleração), texto }.
 */
export function comparar(a, tUv, tTse) {
  const tse = tseNoSistema(a, tTse);
  if (tUv == null || tse == null) return null;
  const s = tse - tUv;
  return { tse, s, noite: s * (a?.x || 1), texto: distancia(s) };
}
