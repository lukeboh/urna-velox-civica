// M-004: a próxima eleição ao vivo (contagem regressiva sempre à vista, também durante um replay) e o modo do selo.
// A data vem da playlist (`GET /api/v1/playlist`, uv-playlist do ciclo diário, campo `proxima_ao_vivo`; nas versões
// sem esse campo, da suspensão da playlist em volta da eleição oficial + 17h). Sem playlist (desenvolvimento, nó sem
// o ciclo), nada aparece. Horários sempre no fuso de Brasília; o "dia" do aviso é o do visitante.

const FUSO = 'America/Sao_Paulo';
const HORA_PADRAO = '17:00:00';
const NOITE_MS = 15 * 3600e3; // das 17h às 8h do dia seguinte (o mesmo do uv-playlist)

/** O modo do serviço: 'replay' (ciclo diário, rótulo), 'ao_vivo' (apuração andando ou encerrada), 'espera' (nenhum BU
 * ainda: a vitrine do 3221, ou a noite da eleição antes das 17h) ou '' (sem estado ainda). */
export function modoDe(e) {
  if (!e) return '';
  if (e.simulacao?.rotulo) return 'replay';
  if ((e.secoes?.aplicadas || 0) > 0 || e.encerrada) return 'ao_vivo';
  return 'espera';
}

/** A playlist (null sem ela). O relógio do servidor corrige o do visitante: `desvio` = servidor − visitante (ms). */
export async function buscarPlaylist() {
  try {
    const r = await fetch('/api/v1/playlist', { cache: 'no-store' });
    if (!r.ok) return null;
    const j = await r.json();
    if (!j || typeof j !== 'object') return null;
    const ag = Date.parse(j.agora);
    j.desvio = Number.isFinite(ag) ? ag - Date.now() : 0;
    return j;
  } catch { return null; }
}

/** { nome, inicio, fim } (ms) da próxima eleição ao vivo, ou null. */
export function proximaDe(pl) {
  if (!pl) return null;
  const p = pl.proxima_ao_vivo;
  if (p?.inicio) {
    const ini = Date.parse(p.inicio), fim = Date.parse(p.fim);
    if (Number.isFinite(ini)) return { nome: p.nome || 'eleição oficial', inicio: ini, fim: Number.isFinite(fim) ? fim : ini + NOITE_MS };
  }
  const s = pl.suspensao || pl.proxima_suspensao;
  if (s?.eleicao) {
    const ini = Date.parse(`${s.eleicao}T${HORA_PADRAO}-03:00`);
    if (Number.isFinite(ini)) return { nome: s.nome || 'eleição oficial', inicio: ini, fim: ini + NOITE_MS };
  }
  return null;
}

const partes = (ms, opc) => Object.fromEntries(new Intl.DateTimeFormat('pt-BR', { timeZone: FUSO, ...opc }).formatToParts(new Date(ms)).map((p) => [p.type, p.value]));
const dataBr = (ms) => { const p = partes(ms, { year: 'numeric', month: '2-digit', day: '2-digit' }); return `${p.year}-${p.month}-${p.day}`; };

/** "domingo (25/10), 17h" — ou "hoje (25/10), 17h" / "amanhã (25/10), 17h"; `curto` = "(25/10), 17h". */
export function quandoTxt(inicio, agora) {
  const p = partes(inicio, { weekday: 'long', day: '2-digit', month: '2-digit', hour: '2-digit', minute: '2-digit', hourCycle: 'h23' });
  const hora = `${+p.hour}h${p.minute !== '00' ? p.minute : ''}`;
  const d = dataBr(inicio);
  const dia = d === dataBr(agora) ? 'hoje' : d === dataBr(agora + 86400e3) ? 'amanhã' : p.weekday;
  const curto = `${p.day}/${p.month}, ${hora}`;
  return { dia, curto, longo: `${dia} (${p.day}/${p.month}), ${hora}`, ddmm: `${p.day}/${p.month}`, hora };
}

const dd = (n) => String(n).padStart(2, '0');
/** Contagem em três partes, da maior para a menor: ["4d", "02h", "13min"], ["2h", "13min", "05s"], ["13min", "05s"]. */
export function contagemPartes(ms) {
  const s = Math.max(0, Math.floor(ms / 1000));
  const d = Math.floor(s / 86400), h = Math.floor((s % 86400) / 3600), m = Math.floor((s % 3600) / 60), ss = s % 60;
  if (d > 0) return [`${d}d`, `${dd(h)}h`, `${dd(m)}min`];
  if (h > 0) return [`${h}h`, `${dd(m)}min`, `${dd(ss)}s`];
  return [`${m}min`, `${dd(ss)}s`];
}

/** O dia do visitante (aaaa-mm-dd no fuso do navegador) — B-023: antes era o dia em UTC (21h em Brasília = outro dia). */
export function diaLocal(ms = Date.now()) {
  const d = new Date(ms);
  return `${d.getFullYear()}-${dd(d.getMonth() + 1)}-${dd(d.getDate())}`;
}
