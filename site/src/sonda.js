// Sondas na página (padrão Test Message; docs/arquitetura/SONDA.md, PROTOCOLO_WS §4.9): a cada ~30 s o servidor injeta uma mensagem
// de teste na mesma fila dos boletins de urna; ela passa pela totalização, pela publicação e pela difusão sem mudar
// nenhum voto e chega aqui (`op: sonda`). Se ela chega, o caminho está andando; o atraso dela (injeção → difusão, no
// relógio do servidor) é o que um BU recebido agora espera para ser processado e publicado.
//
// Só lógica, sem DOM (testada em Node por testes/telemetria.mjs). O tempo da página vem de fora (ms, performance.now).

const AMOSTRAS = 20;
const INTERVALO_PADRAO_MS = 30000;

const mediana = (v) => { if (!v.length) return null; const s = [...v].sort((a, b) => a - b); const m = s.length >> 1; return s.length % 2 ? s[m] : (s[m - 1] + s[m]) / 2; };

export class Telemetria {
  constructor() {
    this.suportada = false;   // o servidor mandou alguma sonda (servidor antigo: nunca; não acusa nada)
    this.ultima = null;       // última mensagem `sonda` deste servidor (nova ou de inscrição)
    this.emPagina = null;     // quando chegou a última sonda nova (ms da página)
    this.ref = null;          // início da contagem da idade: conexão ou a última sonda nova
    this.recebidas = 0;
    this.pulos = 0;
    this.injecoes = [];       // t_injecao (ms) das últimas sondas novas: estima o intervalo do servidor
    this.atrasos = [];        // ms injeção → difusão (relógio do servidor)
    this.envios = [];         // ms injeção → escrita no socket (relógio do servidor)
    this.remotas = new Map(); // federação: origem → sondas vistas (eco dos outros nós)
  }

  /** Conexão (re)aberta: a idade conta daqui até a primeira sonda nova. */
  conectou(agora) { this.ref = agora; }

  /** Uma mensagem `sonda`. Devolve true se era uma sonda nova deste servidor. */
  receber(m, agora) {
    if (!m || m.tipo !== 'sonda' || !Number.isFinite(m.id) || !Number.isFinite(m.t_injecao)) return false;
    this.suportada = true;
    if (m.local === false) { if (!m.instantaneo) this.remotas.set(m.origem, (this.remotas.get(m.origem) || 0) + 1); return false; }
    const u = this.ultima;
    const mesma = u && u.sessao === m.sessao;
    if (mesma && m.id <= u.id) return false;
    if (mesma && m.id > u.id + 1 && !m.instantaneo && this.recebidas > 0) this.pulos += m.id - u.id - 1;
    this.ultima = m;
    if (m.instantaneo) return false;
    this.recebidas++;
    this.emPagina = agora;
    this.ref = agora;
    const por = (v, x) => { if (Number.isFinite(x) && x >= 0) { v.push(x); if (v.length > AMOSTRAS) v.shift(); } };
    if (!mesma) this.injecoes = [];
    por(this.injecoes, m.t_injecao / 1000);
    if (Number.isFinite(m.t_difusao)) por(this.atrasos, (m.t_difusao - m.t_injecao) / 1000);
    if (Number.isFinite(m.t_envio)) por(this.envios, (m.t_envio - m.t_injecao) / 1000);
    return true;
  }

  /** Intervalo entre as sondas do servidor (ms): a mediana dos intervalos vistos, ou 30 s. */
  intervaloMs() {
    const v = this.injecoes, d = [];
    for (let i = 1; i < v.length; i++) d.push(v[i] - v[i - 1]);
    const m = mediana(d.filter((x) => x > 0));
    return m || INTERVALO_PADRAO_MS;
  }

  /** Sem sonda nova por mais que isto, o caminho é dado como parado. */
  limiteMs() { const i = this.intervaloMs(); return Math.max(2.5 * i, i + 10000); }

  /** Atraso de processamento da última sonda (ms) e a mediana das recentes. */
  atrasoMs() { return this.atrasos.length ? this.atrasos[this.atrasos.length - 1] : null; }
  atrasoMedianoMs() { return mediana(this.atrasos); }
  envioMs() { return this.envios.length ? this.envios[this.envios.length - 1] : null; }

  /** Segundos desde a última sonda nova (ou desde a conexão, se nenhuma veio). */
  idadeMs(agora) { return this.ref == null ? null : Math.max(0, agora - this.ref); }

  /**
   * 'desconhecida' (servidor sem sonda, ainda nada, ou fora do ar), 'estavel' (sonda nova dentro do limite) ou
   * 'atrasada' (o servidor tem sonda, a conexão está viva e nada novo chega há mais que o limite).
   */
  situacao(agora, conectado = true) {
    if (!conectado || !this.suportada || this.ref == null) return 'desconhecida';
    if (this.idadeMs(agora) > this.limiteMs()) return 'atrasada';
    return this.emPagina != null ? 'estavel' : 'desconhecida';
  }
}

/** Milissegundos para ler: "<1 ms", "37 ms", "1,2 s", "2 min". */
export function textoMs(ms) {
  if (ms == null || !Number.isFinite(ms)) return '—';
  if (ms < 1) return '<1 ms';
  if (ms < 1000) return `${Math.round(ms)} ms`;
  if (ms < 60000) return `${(ms / 1000).toFixed(ms < 10000 ? 1 : 0).replace('.', ',')} s`;
  return `${Math.round(ms / 60000)} min`;
}

/** Texto curto do chip para a situação (null = nada a alternar). */
export function textoChip(t, agora, conectado = true) {
  const s = t.situacao(agora, conectado);
  if (s === 'estavel') return { situacao: s, curto: '✓ ativo', longo: '✓ checkup ok · ativo' };
  if (s === 'atrasada') return { situacao: s, curto: `⚠ ${textoMs(t.idadeMs(agora))}`, longo: `⚠ sem confirmação há ${textoMs(t.idadeMs(agora))}` };
  return null;
}
