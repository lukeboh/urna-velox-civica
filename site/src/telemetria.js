// Janela de telemetria (aberta pelo chip de BUs/min): velocímetros do ritmo, do atraso de processamento medido pela
// sonda (sonda.js), do atraso BU recebido → publicado (do `estado`) e da idade da última confirmação. Só transform e
// opacity (a janela é fixa, por cima: nada da página se mexe); os ponteiros giram por transição CSS.

import { el, mil, svg } from './util.js';
import { textoMs } from './sonda.js';

const CX = 60, CY = 62, R = 48;
const ponto = (f) => [CX - R * Math.cos(Math.PI * f), CY - R * Math.sin(Math.PI * f)];
function arco(f0, f1) {
  const [x0, y0] = ponto(f0), [x1, y1] = ponto(f1);
  return `M${x0.toFixed(2)} ${y0.toFixed(2)} A${R} ${R} 0 0 1 ${x1.toFixed(2)} ${y1.toFixed(2)}`;
}

/** Velocímetro em SVG: meia-volta, faixas coloridas, marcas e ponteiro. */
export class Velocimetro {
  /**
   * @param {{titulo: string, escala?: 'lin'|'log', min?: number, max: number, faixas?: Array<[number, string]>,
   *   marcas?: number[], rotuloMarca?: (v: number) => string, formatar: (v: number|null) => string}} op
   * `faixas`: [até, cor] em ordem crescente (cores: 'ok' | 'alerta' | 'erro').
   */
  constructor(op) {
    this.op = { escala: 'lin', min: 0, faixas: [], marcas: [], rotuloMarca: String, ...op };
    this.ptr = svg('g', { class: 'ponteiro' });
    this.ptr.append(svg('line', { x1: CX, y1: CY, x2: CX, y2: CY - R + 6 }), svg('circle', { cx: CX, cy: CY, r: 4 }));
    this.zonas = svg('g', { class: 'zonas' });
    this.marcasG = svg('g', { class: 'marcas' });
    const s = svg('svg', { viewBox: '0 0 120 70', 'aria-hidden': 'true' });
    s.append(svg('path', { class: 'trilho', d: arco(0, 1) }), this.zonas, this.marcasG, this.ptr);
    this.valor = el('div', 'vel-valor', '—');
    this.el = el('figure', 'vel', [s, this.valor, el('figcaption', null, this.op.titulo)]);
    this.f = 0;
    this.desenharEscala();
    this.definir(null);
  }

  /** Valor → fração da meia-volta (0 a 1). */
  fracao(v) {
    const { escala, min, max } = this.op;
    if (v == null || !Number.isFinite(v)) return 0;
    let f;
    if (escala === 'log') { const a = Math.log10(Math.max(min, 1e-9)), b = Math.log10(max); f = (Math.log10(Math.max(v, min)) - a) / (b - a); }
    else f = (v - min) / (max - min);
    return Math.min(1, Math.max(0, f));
  }

  desenharEscala() {
    const { faixas, marcas, rotuloMarca } = this.op;
    this.zonas.replaceChildren();
    let ini = 0;
    for (const [ate, cor] of faixas) {
      const f = this.fracao(ate);
      if (f > ini) this.zonas.append(svg('path', { class: `zona ${cor}`, d: arco(ini, f) }));
      ini = Math.max(ini, f);
    }
    this.marcasG.replaceChildren();
    for (const m of marcas) {
      const f = this.fracao(m), a = Math.PI * f;
      const [x, y] = [CX - (R - 9) * Math.cos(a), CY - (R - 9) * Math.sin(a)];
      const t = svg('text', { x: x.toFixed(1), y: (y + 2).toFixed(1), 'text-anchor': f < 0.34 ? 'start' : f > 0.66 ? 'end' : 'middle' });
      t.textContent = rotuloMarca(m);
      this.marcasG.append(t);
    }
  }

  /** Novo máximo (escala linear que cresce com o valor): redesenha as faixas e as marcas. */
  definirMax(max, marcas) {
    if (max === this.op.max) return;
    this.op.max = max;
    if (marcas) this.op.marcas = marcas;
    this.desenharEscala();
  }

  definir(v, texto) {
    this.f = this.fracao(v);
    this.ptr.style.transform = `rotate(${(this.f * 180 - 90).toFixed(2)}deg)`;
    const txt = texto ?? this.op.formatar(v);
    if (this.valor.textContent !== txt) this.valor.textContent = txt;
    let cor = '';
    if (v != null && Number.isFinite(v)) for (const [ate, c] of this.op.faixas) { cor = c; if (v <= ate) break; }
    if (this.el.dataset.cor !== cor) this.el.dataset.cor = cor;
  }
}

/** Escala do ritmo: um máximo "redondo" acima do valor (a escala só cresce numa visita). */
export function maxRitmo(v, atual = 0) {
  const alvo = Math.max(1000, v * 1.25);
  const p = 10 ** Math.floor(Math.log10(alvo));
  const m = [1, 2, 5, 10].map((k) => k * p).find((x) => x >= alvo);
  return Math.max(atual, m);
}

const curto = (v) => (v >= 1e6 ? `${mil(v / 1e6)} mi` : v >= 1000 ? `${mil(v / 1000)} mil` : String(v));
const MS = [1, 10, 100, 1000, 10000];
const rotMs = (v) => (v >= 1000 ? `${v / 1000}s` : `${v}`);

export class PainelTelemetria {
  constructor() {
    this.ritmo = new Velocimetro({ titulo: 'Ritmo (BUs/min)', max: 1000, marcas: [0, 500, 1000], rotuloMarca: curto, formatar: (v) => (v == null ? '—' : mil(v)) });
    this.atraso = new Velocimetro({ titulo: 'Atraso de processamento', escala: 'log', min: 0.5, max: 10000, marcas: MS, rotuloMarca: rotMs,
      faixas: [[200, 'ok'], [2000, 'alerta'], [10000, 'erro']], formatar: textoMs });
    this.bu = new Velocimetro({ titulo: 'BU recebido → publicado', escala: 'log', min: 0.5, max: 10000, marcas: MS, rotuloMarca: rotMs,
      faixas: [[500, 'ok'], [3000, 'alerta'], [10000, 'erro']], formatar: textoMs });
    this.idade = new Velocimetro({ titulo: 'Última confirmação', max: 90, marcas: [0, 30, 60, 90], rotuloMarca: (v) => `${v}s`,
      faixas: [[60, 'ok'], [75, 'alerta'], [90, 'erro']], formatar: (v) => (v == null ? '—' : textoMs(v * 1000)) });
    this.estado = el('p', 'tele-estado', '');
    this.linhas = el('dl', 'tele-linhas');
    const fechar = el('button', 'icone-btn fechar', '✕');
    fechar.type = 'button';
    fechar.setAttribute('aria-label', 'Fechar');
    fechar.addEventListener('click', () => this.fechar());
    const titulo = el('h2', null, 'Telemetria da apuração');
    titulo.id = 'tele-titulo';
    const nota = el('p', 'tele-nota', 'A cada ~30 s o servidor põe uma mensagem de teste (sonda) na mesma fila dos boletins de urna. Ela passa pela '
      + 'totalização, pela publicação e pela transmissão sem mudar nenhum voto. Se ela chega, o caminho está andando; o atraso dela é o '
      + 'que um BU recebido agora leva para ser processado e publicado.');
    this.raiz = el('section', 'tele vidro', [fechar, titulo, this.estado, el('div', 'tele-vels', [this.ritmo.el, this.atraso.el, this.bu.el, this.idade.el]), this.linhas, nota]);
    this.raiz.id = 'telemetria';
    this.raiz.setAttribute('role', 'dialog');
    this.raiz.setAttribute('aria-modal', 'false');
    this.raiz.setAttribute('aria-labelledby', 'tele-titulo');
    this.raiz.setAttribute('aria-hidden', 'true');
    document.body.append(this.raiz);
    document.addEventListener('keydown', (e) => { if (e.key === 'Escape') this.fechar(); });
    document.addEventListener('pointerdown', (e) => {
      if (this.aberta() && !this.raiz.contains(e.target) && !e.target.closest?.('.chip.ritmo')) this.fechar();
    });
  }

  aberta() { return this.raiz.classList.contains('aberta'); }
  /** Abre logo abaixo de `ancora` (o chip), sem passar do letreiro do rodapé. */
  abrir(ancora) {
    const r = ancora?.getBoundingClientRect();
    if (r) {
      const topo = Math.round(r.bottom + 8);
      const feed = parseFloat(getComputedStyle(document.documentElement).getPropertyValue('--feed-h')) || 64;
      this.raiz.style.top = `${topo}px`;
      this.raiz.style.maxHeight = `${Math.max(200, innerHeight - topo - feed - 16)}px`;
    }
    this.raiz.classList.add('aberta');
    this.raiz.setAttribute('aria-hidden', 'false');
  }
  fechar() { this.raiz.classList.remove('aberta'); this.raiz.setAttribute('aria-hidden', 'true'); }
  alternar(ancora) { if (this.aberta()) this.fechar(); else this.abrir(ancora); }

  /**
   * @param {{situacao: string, conectado: boolean, ritmo: number|null, atrasoMs: number|null, buMs: number|null,
   *   buP99: number|null, idadeMs: number|null, limiteMs: number, intervaloMs: number, envioMs: number|null,
   *   recebidas: number, pulos: number, ultima: object|null, seq: number|null, remotas: Map}} d
   */
  atualizar(d) {
    const mx = maxRitmo(d.ritmo || 0, this.ritmo.op.max);
    this.ritmo.definirMax(mx, [0, mx / 2, mx]);
    this.ritmo.definir(d.ritmo);
    // caminho parado: o atraso de um BU que chegasse agora é de pelo menos o tempo sem confirmação
    if (d.situacao === 'atrasada') this.atraso.definir(d.idadeMs, `> ${textoMs(d.idadeMs)}`);
    else this.atraso.definir(d.situacao === 'desconhecida' ? null : d.atrasoMs);
    this.bu.definir(d.buMs);
    const lim = Math.max(30, Math.round(d.limiteMs / 1000));
    const mxIdade = Math.round(lim * 1.5);
    if (this.idade.op.max !== mxIdade) {
      this.idade.op.faixas = [[lim * 0.8, 'ok'], [lim, 'alerta'], [mxIdade, 'erro']];
      this.idade.definirMax(mxIdade, [0, Math.round(mxIdade / 3), Math.round((2 * mxIdade) / 3), mxIdade]);
    }
    this.idade.definir(d.idadeMs == null || d.situacao === 'desconhecida' && !d.recebidas ? null : d.idadeMs / 1000);
    const txt = !d.conectado ? 'Sem conexão em tempo real: a sonda volta quando a conexão voltar.'
      : d.situacao === 'estavel' ? `✓ Sistema estável — sonda nº ${mil(d.ultima?.id)} confirmada há ${textoMs(d.idadeMs)}`
        : d.situacao === 'atrasada' ? `⚠ Nenhuma sonda nova há ${textoMs(d.idadeMs)}: o processamento pode estar atrasado ou parado`
          : d.suportada ? 'Aguardando a primeira sonda…' : 'Este servidor não envia sondas.';
    if (this.estado.textContent !== txt) this.estado.textContent = txt;
    this.raiz.dataset.situacao = d.conectado ? d.situacao : 'desconhecida';
    const linhas = [
      ['Atraso mediano (últimas sondas)', textoMs(d.atrasoMedMs)],
      ['Sonda até esta conexão', textoMs(d.envioMs)],
      ['BU recebido → publicado (p99)', textoMs(d.buP99)],
      ['Intervalo das sondas', textoMs(d.intervaloMs)],
      ['Sondas confirmadas nesta visita', `${mil(d.recebidas)}${d.pulos ? ` (${mil(d.pulos)} não chegaram)` : ''}`],
      ['BUs aplicados', d.seq == null ? '—' : mil(d.seq)],
    ];
    if (d.remotas?.size) linhas.push(['Outros nós da federação', [...d.remotas.keys()].join(', ')]);
    const chave = JSON.stringify(linhas);
    if (this.linhas.dataset.k !== chave) {
      this.linhas.dataset.k = chave;
      this.linhas.replaceChildren(...linhas.flatMap(([a, b]) => [el('dt', null, a), el('dd', null, b)]));
    }
  }
}
