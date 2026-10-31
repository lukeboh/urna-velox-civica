// Consulta de um pleito encerrado (multipleito, etapa 1.1; docs/propostas/PROPOSTA_SERIE_CONSULTA.md): a página abre
// em `#/p/<pleito>/<abrangência>/c<cargo>` e, em vez do WebSocket, lê a série de consulta do pleito — arquivos
// estáticos em `/consulta/<pleito>/serie/` (servidos pela borda, sem chegar aos nós). A `ConexaoSerie` tem a mesma
// interface da `Conexao` (conexao.js) e entrega as mesmas mensagens do protocolo (resultado, tse, evento, marco, mapa,
// estado), filtradas pelo instante do slider: todos os componentes da página funcionam como na noite. O slider é a
// barra do replay (replay.js) com o relógio local: cada visitante tem a sua linha do tempo.
import { noConsole, semBarraFinal } from './util.js';

import { Player } from './replay.js';

const pad4 = (c) => String(c).padStart(4, '0');
const iso = (t) => new Date(t * 1000).toISOString();
/** "dd/mm/aaaa hh:mm:ss" (hora de Brasília: t já está na escala do horário civil, como no servidor). */
export const textoBr = (t) => { const s = iso(t); return `${s.slice(8, 10)}/${s.slice(5, 7)}/${s.slice(0, 4)} ${s.slice(11, 19)}`; };
const textoIso = (t) => `${iso(t).slice(0, 10)} ${iso(t).slice(11, 19)}`;

/** `#/p/<pleito>[/<abr>/c<cargo>]` → { pleito, abr, cargo }; outra rota → null. */
export function rotaConsulta(hash) {
  const m = /^#\/p\/(\d+)(?:\/([a-z]{2}(?:\d{5})?)\/c(\d{4}))?$/.exec(hash || '');
  return m ? { pleito: +m[1], abr: m[2] || null, cargo: m[3] ? +m[3] : null } : null;
}

/** Último índice de `v` ([[t, …], …], em ordem; t null = antes do primeiro BU) com t ≤ `t`; -1 = nenhum. */
export function ate(v, t) {
  let lo = 0, hi = v.length - 1, r = -1;
  while (lo <= hi) {
    const m = (lo + hi) >> 1;
    if ((v[m][0] ?? -Infinity) <= t) { r = m; lo = m + 1; } else hi = m - 1;
  }
  return r;
}

/** `{colunas, linhas}` (marcos.json, eventos.json) → objetos com `t`, em ordem de t (sem hora = no começo). */
function linhas(j) {
  if (!j?.colunas) return [];
  const c = j.colunas;
  return j.linhas.map((l) => Object.fromEntries(c.map((k, i) => [k, l[i]]))).sort((a, b) => (a.t ?? -Infinity) - (b.t ?? -Infinity));
}

const num = (x) => (x === '' || x == null ? null : Number.isNaN(+x) ? x : +x);

/** Bases da consulta, na ordem: a do `<meta name="uv-dados">` (a borda, com cache: a consulta não chega aos nós) e o
 * próprio domínio (se a borda falhar, como no modo HTTP da conexao.js). */
export function basesConsulta() {
  const m = semBarraFinal((typeof document !== 'undefined' ? document.querySelector('meta[name="uv-dados"]')?.content || '' : '').trim());
  const o = typeof location !== 'undefined' ? location.origin : '';
  return m && m !== o ? [`${m}/consulta`, '/consulta'] : ['/consulta'];
}

/** Carrega o que a página precisa antes do primeiro desenho; null se o pleito não tem série. */
export async function carregarSerie(pleito, bases = basesConsulta()) {
  let raiz = null, indice = null;
  const ler = (rz, n) => fetch(`${rz}/${n}.json`, { credentials: 'omit' }).then((r) => (r.ok ? r.json() : null)).catch(() => null);
  for (const b of bases) {
    indice = await ler(`${b}/${pleito}/serie`, 'indice');
    if (indice) { raiz = `${b}/${pleito}/serie`; break; }
  }
  if (!indice) return null;
  const j = (n) => ler(raiz, n);
  const [nomes, topicos, marcos, eventos, mapa] = await Promise.all([j('nomes'), j('topicos'), j('marcos'), j('eventos'), j('mapa')]);
  return { raiz, pleito, indice, nomes, topicos: topicos || [], marcos: linhas(marcos), eventos: linhas(eventos), mapa };
}

export class ConexaoSerie {
  /** @param {(m: object) => void} aoReceber  @param {(e: string, info?: object) => void} aoEstado */
  constructor(aoReceber, aoEstado, serie, rotulo) {
    this.aoReceber = aoReceber;
    this.aoEstado = aoEstado;
    this.s = serie;
    this.ind = serie.indice;
    this.rotulo = rotulo;
    this.topicos = new Set();
    this.series = new Map(); // tópico (ou tse:<tópico>) → { v, i, arquivo, fonte } | { carregando }
    this.t = -Infinity;
    this.epoca = 1;
    this.iMa = 0;
    this.iEv = 0;
    this.nMapa = 0;
    this.seq = 0;
    this.mapaF = null;
    this.aoReiniciar = null;
    this.aoConsultar = null;
    this.aoIndice = null;
    this.#prepararMapa();
    // pleito em curso (série refeita durante a noite): o índice é relido; quando a noite avança, o slider se estende
    if (this.ind.em_curso) setInterval(() => this.#atualizarIndice(), globalThis.__uvSerie?.atualizarMs || 60000);
  }

  async #atualizarIndice() {
    const ler = (n) => fetch(`${this.s.raiz}/${n}.json`, { credentials: 'omit', cache: 'no-cache' }).then((r) => (r.ok ? r.json() : null)).catch(() => null);
    const j = await ler('indice');
    if (!j || !(j.fim > this.ind.fim)) return;
    const [ma, ev, mp] = await Promise.all([ler('marcos'), ler('eventos'), ler('mapa')]);
    this.ind = j;
    this.s.indice = j;
    // o que já foi entregue até o instante atual não se repete: as listas novas continuam de onde estavam
    const ate_t = (l) => l.filter((x) => (x.t ?? -Infinity) <= this.t).length;
    if (ma) { this.s.marcos = linhas(ma); this.iMa = ate_t(this.s.marcos); }
    if (ev) { this.s.eventos = linhas(ev); this.iEv = ate_t(this.s.eventos); }
    if (mp) {
      this.s.mapa = mp;
      this.#prepararMapa();
      this.nMapa = 0;
      this.#mapaAte(this.t, false);
      this.epoca++;
      if (this.mapaF) this.aoReceber({ tipo: 'mapa', reiniciar: true, epoca: this.epoca });
    }
    // as versões novas dos tópicos da tela
    for (const t of this.topicos) { this.series.delete(t); this.#carregar(t).catch(noConsole('série')); }
    this.aoIndice?.(j);
  }

  // ---------------------------------------------------------------- mapa das seções (protocolo §4.8, sem servidor)
  #prepararMapa() {
    const m = this.s.mapa;
    this.ordem = [];
    if (!m?.m) return;
    const b = Uint8Array.from(atob(m.m), (c) => c.charCodeAt(0));
    for (let i = 0; i < m.total; i++) {
      const min = b[2 * i] | (b[2 * i + 1] << 8);
      if (!min) continue;
      const hora = m.inicio + (min - 1) * 60;
      this.ordem.push([hora, i, Math.min(240, Math.floor((((hora % 86400) + 86400) % 86400) / 360) + 1)]);
    }
    this.ordem.sort((a, b2) => a[0] - b2[0] || a[1] - b2[1]);
  }

  #fotografia() {
    const m = this.s.mapa;
    const h = new Uint8Array(m.total);
    for (let k = 0; k < this.nMapa; k++) h[this.ordem[k][1]] = this.ordem[k][2];
    let bin = '';
    for (let i = 0; i < h.length; i += 0x8000) bin += String.fromCharCode.apply(null, h.subarray(i, i + 0x8000));
    return { v: 1, epoca: this.epoca, total: m.total, n: this.nMapa, fora: 0, ret: [], h: btoa(bin) };
  }

  #mapaAte(t, mandar) {
    const de = this.nMapa;
    while (this.nMapa < this.ordem.length && this.ordem[this.nMapa][0] <= t) this.nMapa++;
    if (!mandar || !this.mapaF || this.nMapa === de) return;
    const s = [];
    for (let k = de; k < this.nMapa; k++) s.push(this.ordem[k][1], this.ordem[k][2]);
    this.aoReceber({ tipo: 'mapa', epoca: this.epoca, de, n: this.nMapa, total: this.s.mapa.total, s });
  }

  // ---------------------------------------------------------------- interface da Conexao
  abrir() { this.aoEstado('vivo'); this.#estado(true); }
  definirFaltam(lista) { this.faltam = lista; }
  pedirMapa(f) {
    this.mapaF = f;
    const { desde, epoca } = f();
    if (epoca !== this.epoca) { this.aoReceber({ tipo: 'mapa', reiniciar: true, epoca: this.epoca }); return; }
    if (desde < this.nMapa) {
      const s = [];
      for (let k = desde; k < this.nMapa; k++) s.push(this.ordem[k][1], this.ordem[k][2]);
      this.aoReceber({ tipo: 'mapa', epoca: this.epoca, de: desde, n: this.nMapa, total: this.s.mapa.total, s });
    }
  }

  async obter(caminho) {
    if (caminho.startsWith('/api/v1/mapa/secoes')) return fetch(`${this.s.raiz}/mapa-secoes.json`, { credentials: 'omit' }).catch(() => null);
    if (caminho.startsWith('/api/v1/mapa/estado')) return this.s.mapa ? new Response(JSON.stringify(this.#fotografia())) : null;
    return fetch(caminho).catch(() => null);
  }

  definirTopicos(lista) {
    this.topicos = new Set(lista);
    for (const t of lista) {
      const s = this.series.get(t);
      if (!s) this.#carregar(t).catch(noConsole('série'));
      else if (s.v) this.#emitir(t, true);
    }
  }

  async #carregar(t) {
    this.series.set(t, { carregando: true });
    const tse = t.startsWith('tse:');
    const nome = tse ? `tse/${t.slice(4)}` : `t/${t}`;
    const j = await fetch(`${this.s.raiz}/${nome}.json`, { credentials: 'omit' }).then((r) => (r.ok ? r.json() : null)).catch(() => null);
    this.series.set(t, j ? { v: j.versoes, i: -1, arquivo: j.arquivo, fonte: j.fonte } : { v: [], i: -1, falta: true });
    if (tse && !j) {
      this.aoReceber({ tipo: 'tse', instantaneo: true, topico: t, ver: 0, estado: 'indisponivel', motivo: 'sem_serie', arquivo: null, buscado: null, novo_em: null, dados: null });
      return;
    }
    if (this.topicos.has(t)) this.#emitir(t, true);
  }

  /** Manda a versão em vigor no instante atual (se mudou, ou sempre com `forcar`). */
  #emitir(t, forcar) {
    const s = this.series.get(t);
    if (!s?.v) return;
    // no fim da noite, a última versão do TSE (que pode ter saído depois do último BU do Urna Velox)
    const i = ate(s.v, t.startsWith('tse:') && this.t >= this.ind.fim ? Infinity : this.t);
    if (i < 0 || (i === s.i && !forcar)) return;
    s.i = i;
    const [tv, d] = s.v[i];
    if (t.startsWith('tse:')) {
      this.aoReceber({ tipo: 'tse', instantaneo: true, topico: t, ver: this.epoca * 1e6 + i + 1, estado: 'ok', motivo: null, arquivo: s.arquivo,
        buscado: textoBr(Math.max(this.t, tv ?? this.t)), novo_em: tv != null ? textoBr(tv) : null, dados: d, reconstrucao: !!d?.reconstrucao, fonte: s.fonte });
      return;
    }
    const seq = d.seq ?? 0;
    // "totalizado pelo Urna Velox às": na série é a hora em que o replay que a gerou rodou; na consulta vale a da noite
    if (d.totalizado_em && d.ultimo_bu) d.totalizado_em = d.ultimo_bu;
    this.seq = Math.max(this.seq, seq);
    this.aoReceber({ tipo: 'resultado', topico: t, seq, txt: `{"tipo":"instantaneo","topico":${JSON.stringify(t)},"seq":${seq},"resultado":${JSON.stringify(d)}}` });
  }

  #estado(forcar) {
    const agora = performance.now();
    if (!forcar && agora - (this.ultEstado || 0) < 500) return;
    this.ultEstado = agora;
    const t = Number.isFinite(this.t) ? this.t : this.ind.inicio;
    const txt = textoBr(t);
    this.aoReceber({ tipo: 'estado', relogio_site: txt, ultimo_bu: { seq: this.seq, site: txt }, secoes: { aplicadas: this.seq },
      encerrada: t >= this.ind.fim, simulacao: { rotulo: this.rotulo }, consulta: { pleito: this.s.pleito, t, inicio: this.ind.inicio, fim: this.ind.fim } });
  }

  #marco(r) {
    const cargo = num(r.cargo);
    const m = { tipo: 'marco', classe: r.classe, seq: num(r.seq) ?? 0, horario: r.horario, abrangencia: r.abrangencia, cargo,
      topico: cargo != null && r.abrangencia ? `${r.abrangencia}-c${pad4(cargo)}` : null, uf: r.uf || null, municipio: num(r.municipio),
      municipio_nome: r.municipio ? this.s.nomes?.municipios?.[r.municipio] ?? this.s.nomes?.municipios?.[`${r.uf}${String(r.municipio).padStart(5, '0')}`] ?? null : null,
      zona: num(r.zona), secao: num(r.secao), estado: r.estado || null, valor: num(r.valor), lote_bus: num(r.lote_bus) };
    if (r.classe === 'faltam_para_mat') m.faltam = num(r.valor);
    if (r.classe === 'virada' && /^\d+:\d+$/.test(r.estado || '')) { const [a, b] = r.estado.split(':'); m.candidato = +a; m.anterior = +b; }
    return m;
  }

  #evento(r) {
    const cargo = num(r.cargo);
    return { tipo: 'evento', classe: 'matematicamente', seq: num(r.seq) ?? 0, horario: r.horario, sistema: null, abrangencia: r.abrangencia, cargo,
      topico: cargo != null ? `${r.abrangencia}-c${pad4(cargo)}` : null, candidato: num(r.candidato), estado: r.estado, motivo: '' };
  }

  /** O slider andou para a frente até `t` (para trás: [`reposicionar`]). */
  avancar(t) {
    if (t < this.t) { this.reposicionar(t); return; }
    if (t === this.t) { this.#estado(false); return; }
    this.t = t;
    for (const x of this.topicos) this.#emitir(x, false);
    for (; this.iEv < this.s.eventos.length && (this.s.eventos[this.iEv].t ?? -Infinity) <= t; this.iEv++) this.aoReceber(this.#evento(this.s.eventos[this.iEv]));
    for (; this.iMa < this.s.marcos.length && (this.s.marcos[this.iMa].t ?? -Infinity) <= t; this.iMa++) this.aoReceber(this.#marco(this.s.marcos[this.iMa]));
    this.#mapaAte(t, true);
    this.#estado(false);
  }

  /** Ir a um instante qualquer: a página recomeça (aoReiniciar) e recebe tudo de novo até lá. */
  reposicionar(t) {
    this.epoca++;
    this.iMa = 0; this.iEv = 0; this.nMapa = 0; this.seq = 0;
    for (const s of this.series.values()) if (s.v) s.i = -1;
    this.t = t;
    this.#mapaAte(t, false);
    this.aoReiniciar?.();
    this.t = -Infinity;
    this.avancar(t);
    this.#estado(true);
    if (this.mapaF) this.aoReceber({ tipo: 'mapa', reiniciar: true, epoca: this.epoca });
  }

  /** Curva da apuração do tópico até o instante atual (o corpo de GET /api/v1/curva, no relógio da noite). */
  curva(t) {
    const s = this.series.get(t);
    const v = s?.v || [];
    const n = ate(v, this.t);
    const pontos = [];
    for (let i = 0; i <= n; i++) {
      const [tv, d] = v[i];
      if (tv == null || !d?.secoes) continue;
      pontos.push([tv, d.secoes.apuradas, d.secoes.total, (d.cand || []).filter((c) => c.pct != null).map((c) => [c.n, c.pct])]);
    }
    return { topico: t, passo_s: 60, relogio: 'site', pontos };
  }

  /** Garante a série do tópico carregada (a curva é pedida logo depois da inscrição). */
  async pronto(t) {
    for (let k = 0; k < 50 && !this.series.get(t)?.v; k++) await new Promise((ok) => setTimeout(ok, 40));
  }
}

/** A barra do replay com o relógio local: o visitante comanda a própria linha do tempo. Abre parada no fim da noite. */
export class PlayerLocal extends Player {
  constructor(indice, aoIr) {
    super({ inicio: indice.inicio, fim: indice.fim, acelerar: 0, t: indice.fim, real: 0, sessao: null });
    this.aoIr = aoIr;
    this.ultima = 60;
    this.el.setAttribute('aria-label', 'Linha do tempo da noite');
    this.vel.value = '60';
    // pleito em curso: o botão que volta ao tempo real (a página ao vivo)
    this.aoVivo = null;
    if (indice.em_curso) {
      const b = document.createElement('button');
      b.type = 'button';
      b.className = 'pl-aovivo';
      b.textContent = '● ao vivo';
      b.title = 'Voltar à apuração em tempo real';
      b.addEventListener('click', () => this.aoVivo?.());
      this.play.after(b);
      this.botaoAoVivo = b;
    }
  }

  /** A noite em curso avançou: o fim do slider anda junto (e o tempo, se estava no fim). */
  estender(fim) {
    const noFim = this.agora() >= this.info.fim;
    this.info.fim = fim;
    this.tempo.max = String(fim);
    this.fim.textContent = new Date(fim * 1000).toISOString().slice(11, 16);
    this.desenharMarcas();
    if (noFim && this.info.acelerar === 0) { this.aplicar({ ...this.info, t: fim }); this.aoIr(fim); }
  }

  async consultar() { /* sem servidor */ }
  estadoWs() { /* sem sessão */ }
  agora() { return Math.min(this.info.fim, super.agora()); }

  async velocidade(x) {
    let t = this.agora();
    // tocar no fim recomeça do começo da noite
    if (x > 0 && t >= this.info.fim) { t = this.info.inicio; this.aoIr(t); }
    this.aplicar({ ...this.info, t, acelerar: x, real: x });
  }

  async ir(t) {
    this.arrastando = false;
    this.el.classList.remove('com-dica');
    this.aplicar({ ...this.info, t, real: this.info.acelerar });
    this.aoIr(t);
  }

  desenhar() {
    super.desenhar();
    // no fim, pausa sozinho
    if (this.info.acelerar > 0 && this.agora() >= this.info.fim) this.aplicar({ ...this.info, t: this.info.fim, acelerar: 0, real: 0 });
  }
}
