// Hodômetro: cada dígito é uma coluna que rola por translateY até o valor novo (transição CSS: um valor novo no meio
// da animação só redireciona a coluna, não recomeça). A largura é reservada para o maior número possível (`inteiros`
// dígitos), com os zeros à esquerda invisíveis (opacity) e os separadores de milhar fixos no lugar: 999 → 1.000 não
// muda a largura de nada. Com prefers-reduced-motion, a troca é um fade curto.
//
// Como um hodômetro de verdade, **cada dígito gira sempre no mesmo sentido** (pedido do Luciano, 0.5.0): de 7 para 2
// ele passa por 8, 9, 0, 1 — nunca volta por 6, 5, 4, 3 —, mesmo quando o número diminui (um % que cai). Os contadores
// regressivos (`regressivo: true`, ex.: eleitores que faltam para o matematicamente) giram no sentido oposto. Para isso
// a coluna tem os dígitos 0–9 três vezes seguidas e a posição de cada dígito anda sem limite; quando sairia da coluna,
// ela é trazida de volta 10 ou 20 posições (o mesmo dígito na cópia vizinha: nada muda na tela).

import { el, movimentoReduzido } from './util.js';

const COPIAS = 3, POS = COPIAS * 10;

function coluna() {
  const col = el('span', 'col');
  for (let c = 0; c < COPIAS; c++) for (let d = 0; d <= 9; d++) col.append(el('span', null, String(d)));
  col.style.transform = posicao(10);
  return col;
}

const posicao = (p) => `translateY(${(-p * 100) / POS}%)`;

/** Posição em que a coluna está agora na tela (no meio da transição, inclusive), em dígitos; `null` se não medir. */
function naTela(col) {
  const alt = col.offsetHeight / POS;
  if (!(alt > 0)) return null;
  const t = getComputedStyle(col).transform;
  if (!t || t === 'none') return 0;
  return -new DOMMatrixReadOnly(t).m42 / alt;
}

export class Hodometro {
  /**
   * @param {{inteiros?: number, casas?: number, sufixo?: string, molde?: string, rotulo?: string, regressivo?: boolean}} op
   * `molde`: modo de texto fixo ("##:##:##"; `#` = dígito); senão número com `inteiros` dígitos reservados, `casas`
   * decimais (vírgula) e separador de milhar. `regressivo`: contador que desce (os dígitos giram no sentido oposto).
   * `enxuto`: número sozinho na linha (rótulo em cima): os dígitos à esquerda ainda vazios não aparecem como espaço, para
   * o número ficar alinhado ao rótulo. A largura reservada continua a mesma (nada ao lado se mexe); o número todo é que
   * desliza para a esquerda por `transform` (não conta no CLS) o tanto dos dígitos e separadores vazios. `centro: true`
   * quando o número está centralizado (o anel da apuração): desliza metade.
   */
  constructor(op = {}) {
    this.op = op;
    this.sentido = op.regressivo ? -1 : 1;
    this.el = el('span', op.enxuto ? 'hod enxuto' : 'hod');
    this.dig = [];   // { wrap, col, d }
    this.seps = [];  // { span, apos } — visível se o dígito `apos` (à esquerda) está visível
    this.valor = undefined;
    const vis = el('span', 'vis');
    vis.setAttribute('aria-hidden', 'true');
    this.vis = vis;
    this.desloc = 0;
    this.texto = el('span', 'sr');
    if (op.molde) {
      for (const ch of op.molde) {
        if (ch === '#') this.#digito(vis); else vis.append(el('span', 'sp', ch));
      }
    } else {
      const n = Math.max(1, op.inteiros || 1);
      for (let i = 0; i < n; i++) {
        if (i > 0 && (n - i) % 3 === 0) {
          const s = el('span', 'sp', '.');
          this.seps.push({ span: s, apos: i - 1 });
          vis.append(s);
        }
        this.#digito(vis);
      }
      this.nInt = n;
      if (op.casas) {
        vis.append(el('span', 'sp', ','));
        for (let i = 0; i < op.casas; i++) this.#digito(vis);
      }
    }
    if (op.sufixo) vis.append(el('span', 'suf', op.sufixo));
    this.el.append(vis, this.texto);
    this.definir(null);
  }

  #digito(pai) {
    const wrap = el('span', 'dg');
    const col = coluna();
    wrap.append(col);
    pai.append(wrap);
    // p = posição pedida (em dígitos, 0 a POS-1); a coluna nasce mostrando o 0 da cópia do meio
    this.dig.push({ wrap, col, d: 0, p: 10, vazio: null });
  }

  #por(i, ch) {
    const g = this.dig[i];
    const vazio = ch === ' ';
    if (g.vazio !== vazio) { g.wrap.classList.toggle('vazio', vazio); g.vazio = vazio; }
    if (vazio) return;
    const d = ch.charCodeAt(0) - 48;
    if (d === g.d) return;
    const s = this.sentido;
    // passos até d girando só no sentido do hodômetro (1 a 9)
    const passos = s > 0 ? (d - g.d + 10) % 10 : (g.d - d + 10) % 10;
    let alvo = g.p + s * passos;
    if (alvo < 0 || alvo > POS - 1) {
      // sairia da coluna: traz a coluna (onde ela está agora, no meio da transição) e o alvo para dentro, pelo mesmo
      // dígito de outra cópia — sem transição, invisível. Se a coluna estiver muito atrás (muitas versões seguidas),
      // descarta voltas inteiras do caminho que falta: o sentido continua o mesmo.
      const agora = naTela(g.col);
      const novo = s > 0 ? 20 + d : d;
      if (agora === null) {
        g.col.style.transition = 'none';
        g.col.style.transform = posicao(novo);
        void g.col.offsetHeight;
        g.col.style.transition = '';
      } else {
        let falta = Math.max(0, s * (alvo - agora));
        if (falta > 20) falta -= 10 * Math.ceil((falta - 20) / 10);
        g.col.style.transition = 'none';
        g.col.style.transform = posicao(novo - s * falta);
        void g.col.offsetHeight;
        g.col.style.transition = '';
        g.col.style.transform = posicao(novo);
      }
      alvo = novo;
    } else {
      g.col.style.transform = posicao(alvo);
    }
    g.p = alvo;
    g.d = d;
  }

  /** Enxuto: desliza o número (só `transform`) para os dígitos vazios à esquerda não aparecerem como espaço. */
  #deslizar() {
    let w = 0;
    for (const x of this.vis.children) {
      if (!x.classList.contains('vazio')) break;
      w += x.getBoundingClientRect().width; // fracionário (offsetWidth arredonda: erro somado de px)
    }
    if (!(this.vis.offsetWidth > 0)) return; // fora da tela ainda (sem layout): mede na próxima vez
    // sem a escala de algum ancestral (cartão entrando com scale): a medida volta para px de layout
    const esc = this.vis.offsetWidth / (this.vis.getBoundingClientRect().width || this.vis.offsetWidth);
    w = Math.round(w * esc * 100) / 100;
    if (this.op.centro) w /= 2;
    if (w === this.desloc && this.medido) return;
    // primeira medida com layout: vai direto para o lugar, sem deslizar
    const primeira = !this.medido;
    this.medido = true;
    this.desloc = w;
    if (primeira) this.vis.style.transition = 'none';
    this.vis.style.transform = w ? `translateX(${-w}px)` : '';
    if (primeira) { void this.vis.offsetWidth; this.vis.style.transition = ''; }
  }

  /** Valor novo (número; texto no modo `molde`; `null` = desconhecido). */
  definir(v) {
    if (v === this.valor) return;
    const antes = this.valor;
    this.valor = v;
    const nulo = v == null || (typeof v === 'number' && !Number.isFinite(v));
    this.el.classList.toggle('nulo', nulo);
    if (this.op.molde) {
      const ds = nulo ? '' : String(v).replace(/\D/g, '');
      for (let i = 0; i < this.dig.length; i++) this.#por(i, ds[i] ?? '0');
      this.texto.textContent = nulo ? '—' : String(v);
    } else {
      const casas = this.op.casas || 0;
      const x = nulo ? 0 : Math.max(0, v);
      let [int, dec = ''] = x.toFixed(casas).split('.');
      if (int.length > this.nInt) int = '9'.repeat(this.nInt); // não acontece: a largura é a do eleitorado
      const cheio = int.padStart(this.nInt, ' ');
      for (let i = 0; i < this.nInt; i++) this.#por(i, cheio[i]);
      for (let i = 0; i < casas; i++) this.#por(this.nInt + i, dec[i] ?? '0');
      for (const s of this.seps) s.span.classList.toggle('vazio', cheio[s.apos] === ' ');
      if (this.op.enxuto) this.#deslizar();
      this.texto.textContent = nulo ? '—' : (int.replace(/\B(?=(\d{3})+(?!\d))/g, '.') + (casas ? ',' + dec : '') + (this.op.sufixo ? ' ' + this.op.sufixo : ''));
    }
    if (antes !== undefined && antes !== null && movimentoReduzido() && this.el.animate) {
      this.el.animate([{ opacity: 0.35 }, { opacity: 1 }], { duration: 220, easing: 'ease-out' });
    }
  }
}
