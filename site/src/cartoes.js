// Cartões que se maximizam e se reorganizam. Cada cartão do painel ganha, no canto, a alça (arrastar para outra
// posição ou para a outra coluna; no teclado, setas com a alça em foco) e o botão de maximizar (Esc ou o véu fecham).
// A ordem fica neste navegador (localStorage `uv-cartoes`). Regras da casa: tudo por `translate`/`scale` (a `transform`
// é da inclinação tátil) e nada pula para quem não está mexendo — a vaga do cartão segura o lugar dele.

import { el, movimentoReduzido } from './util.js';

const CHAVE = 'uv-cartoes';
const ler = () => { try { const v = JSON.parse(localStorage.getItem(CHAVE)); return Array.isArray(v) && v.length === 2 ? v : null; } catch { return null; } };
const gravar = (v) => { try { localStorage.setItem(CHAVE, JSON.stringify(v)); } catch { /* sem armazenamento */ } };
const SVG_ALCA = '<svg viewBox="0 0 16 16" aria-hidden="true"><circle cx="5.5" cy="3.5" r="1.4"/><circle cx="10.5" cy="3.5" r="1.4"/><circle cx="5.5" cy="8" r="1.4"/><circle cx="10.5" cy="8" r="1.4"/><circle cx="5.5" cy="12.5" r="1.4"/><circle cx="10.5" cy="12.5" r="1.4"/></svg>';
const SVG_MAX = '<svg class="ic-max" viewBox="0 0 16 16" aria-hidden="true"><path d="M9.5 2.5h4v4M6.5 13.5h-4v-4M13.5 2.5 9 7M2.5 13.5 7 9"/></svg><svg class="ic-min" viewBox="0 0 16 16" aria-hidden="true"><path d="M13 7H9V3M3 9h4v4M9 7l4.5-4.5M7 9l-4.5 4.5"/></svg>';

export class Cartoes {
  /** @param {{aoMaximizar?: (c: HTMLElement, sim: boolean) => void}} op */
  constructor(op = {}) {
    this.op = op;
    this.trilhos = [];
    this.padrao = null;
    this.max = null;
    this.arr = null;
    this.veu = el('div', 'cartao-veu');
    this.veu.addEventListener('click', () => this.restaurar());
    document.body.append(this.veu);
    addEventListener('keydown', (e) => { if (e.key === 'Escape' && this.max) this.restaurar(); });
  }

  /** Põe a alça e o botão de maximizar num cartão (uma vez por elemento). */
  preparar(c, id, titulo) {
    c.dataset.cartao = id;
    c.classList.add('cartao-mov');
    if (c.querySelector(':scope > .cartao-ferr')) return c;
    const alca = el('button', 'cf-alca');
    alca.type = 'button';
    alca.innerHTML = SVG_ALCA;
    alca.setAttribute('aria-label', `Mover o cartão ${titulo} (arraste, ou use as setas)`);
    alca.title = 'Arrastar para reorganizar';
    const max = el('button', 'cf-max');
    max.type = 'button';
    max.innerHTML = SVG_MAX;
    max.setAttribute('aria-label', `Maximizar o cartão ${titulo}`);
    max.title = 'Maximizar';
    c.append(el('div', 'cartao-ferr', [alca, max]));
    alca.addEventListener('pointerdown', (e) => {
      if (e.button || this.max) return;
      e.preventDefault();
      alca.setPointerCapture(e.pointerId);
      this.#iniciar(c, e);
    });
    alca.addEventListener('pointermove', (e) => this.#mover(e));
    alca.addEventListener('pointerup', () => this.#soltar());
    alca.addEventListener('pointercancel', () => this.#soltar());
    alca.addEventListener('keydown', (e) => { if (e.key.startsWith('Arrow')) { e.preventDefault(); this.#teclado(c, e.key); } });
    max.addEventListener('click', () => (this.max?.c === c ? this.restaurar() : this.maximizar(c)));
    return c;
  }

  /** Os dois trilhos do painel (a coluna larga e a lateral), já com os cartões na ordem padrão: aplica a salva. */
  montar(trilhos) {
    this.trilhos = trilhos;
    const ordem = (t) => [...t.children].filter((x) => x.dataset.cartao).map((x) => x.dataset.cartao);
    this.padrao = trilhos.map(ordem);
    const salvo = ler();
    if (salvo) this.#aplicar(salvo);
  }

  restaurarLayout() {
    this.restaurar(true);
    if (!this.padrao) return;
    this.#flip(() => this.#aplicar(this.padrao));
    gravar(this.padrao);
  }

  #aplicar(o) {
    const porId = new Map(this.trilhos.flatMap((t) => [...t.children]).filter((x) => x.dataset.cartao).map((x) => [x.dataset.cartao, x]));
    const usados = new Set();
    o.forEach((lista, i) => (lista || []).forEach((id) => { const c = porId.get(id); if (c && !usados.has(id)) { this.trilhos[i].append(c); usados.add(id); } }));
    // cartão novo (que não estava na ordem salva) fica onde a ordem padrão o põe
    this.padrao.forEach((lista, i) => lista.forEach((id) => { const c = porId.get(id); if (c && !usados.has(id)) this.trilhos[i].append(c); }));
  }

  #ordem() { return this.trilhos.map((t) => [...t.children].filter((x) => x.dataset.cartao).map((x) => x.dataset.cartao)); }

  /** FLIP pela propriedade `translate`: os vizinhos deslizam até o lugar novo. */
  #flip(mudar) {
    const els = this.trilhos.flatMap((t) => [...t.children]).filter((x) => !x.classList.contains('arrastando') && !x.classList.contains('max'));
    const antes = new Map(els.map((x) => [x, x.getBoundingClientRect()]));
    mudar();
    if (movimentoReduzido()) return;
    for (const x of els) {
      const a = antes.get(x), d = x.getBoundingClientRect(), dx = a.left - d.left, dy = a.top - d.top;
      if (!dx && !dy) continue;
      x.style.transition = 'none';
      x.style.translate = `${dx}px ${dy}px`;
      x.getBoundingClientRect();
      x.style.transition = '';
      x.style.translate = '0px 0px';
      setTimeout(() => { x.style.translate = ''; }, 420);
    }
  }

  #iniciar(c, e) {
    const r = c.getBoundingClientRect();
    const vaga = el('div', 'cartao-vaga');
    vaga.style.height = `${r.height}px`;
    c.before(vaga);
    Object.assign(c.style, { width: `${r.width}px`, left: `${r.left}px`, top: `${r.top}px` });
    c.style.setProperty('--rx', '0deg'); c.style.setProperty('--ry', '0deg');
    c.classList.add('arrastando');
    document.body.classList.add('arrastando-cartao');
    this.arr = { c, vaga, x0: e.clientX, y0: e.clientY };
  }

  #mover(e) {
    const a = this.arr;
    if (!a) return;
    a.c.style.translate = `${e.clientX - a.x0}px ${e.clientY - a.y0}px`;
    const px = e.clientX, py = e.clientY;
    // trilho sob o ponteiro (empilhados no celular: o que contém o y); senão o mais perto
    let alvo = this.trilhos.find((t) => { const b = t.getBoundingClientRect(); return px >= b.left && px <= b.right && py >= b.top - 40 && py <= b.bottom + 40; });
    if (!alvo) {
      let dmin = Infinity;
      for (const t of this.trilhos) {
        const b = t.getBoundingClientRect(), dd = Math.hypot(Math.max(b.left - px, 0, px - b.right), Math.max(b.top - py, 0, py - b.bottom));
        if (dd < dmin) { dmin = dd; alvo = t; }
      }
    }
    const irmaos = [...alvo.children].filter((x) => x.dataset.cartao && x !== a.c);
    const antesDe = irmaos.find((x) => { const b = x.getBoundingClientRect(); return py < b.top + b.height / 2; }) || null;
    if (a.vaga.parentNode !== alvo || a.vaga.nextElementSibling !== antesDe) {
      this.#flip(() => { if (antesDe) alvo.insertBefore(a.vaga, antesDe); else alvo.append(a.vaga); });
    }
  }

  #soltar() {
    const a = this.arr;
    if (!a) return;
    this.arr = null;
    const d = a.vaga.getBoundingClientRect(), r = a.c.getBoundingClientRect();
    const [tx, ty] = (a.c.style.translate || '0px 0px').split(' ').map((v) => Number.parseFloat(v) || 0);
    a.c.style.translate = `${tx + d.left - r.left}px ${ty + d.top - r.top}px`;
    const fim = () => {
      a.c.classList.remove('arrastando');
      document.body.classList.remove('arrastando-cartao');
      for (const p of ['width', 'left', 'top', 'translate']) a.c.style[p] = '';
      a.vaga.replaceWith(a.c);
      gravar(this.#ordem());
    };
    if (movimentoReduzido()) fim(); else setTimeout(fim, 260);
  }

  #teclado(c, tecla) {
    const t = c.parentNode, i = this.trilhos.indexOf(t);
    this.#flip(() => {
      if (tecla === 'ArrowUp' && c.previousElementSibling) t.insertBefore(c, c.previousElementSibling);
      else if (tecla === 'ArrowDown' && c.nextElementSibling) t.insertBefore(c.nextElementSibling, c);
      else if (tecla === 'ArrowLeft' && i === 1) this.trilhos[0].append(c);
      else if (tecla === 'ArrowRight' && i === 0) this.trilhos[1].prepend(c);
    });
    gravar(this.#ordem());
    c.querySelector('.cf-alca')?.focus();
  }

  maximizar(c) {
    if (this.max) this.restaurar(true);
    const r0 = c.getBoundingClientRect();
    const vaga = el('div', 'cartao-vaga-max');
    vaga.style.height = `${r0.height}px`;
    c.before(vaga);
    c.style.setProperty('--rx', '0deg'); c.style.setProperty('--ry', '0deg');
    c.classList.add('max');
    this.veu.classList.add('ligado');
    document.body.classList.add('cartao-maximizado');
    this.max = { c, vaga };
    c.querySelector('.cf-max')?.setAttribute('aria-label', 'Restaurar o cartão');
    this.op.aoMaximizar?.(c, true);
    if (movimentoReduzido()) return;
    const r1 = c.getBoundingClientRect();
    c.style.transformOrigin = '0 0';
    c.style.translate = `${r0.left - r1.left}px ${r0.top - r1.top}px`;
    c.style.scale = `${r0.width / r1.width} ${r0.height / r1.height}`;
    c.getBoundingClientRect();
    c.classList.add('anim-max');
    c.style.translate = '0px 0px';
    c.style.scale = '1';
    setTimeout(() => { c.classList.remove('anim-max'); c.style.translate = ''; c.style.scale = ''; c.style.transformOrigin = ''; }, 380);
  }

  /** Fecha o cartão maximizado (`ja`: sem animação, ao trocar de tela). */
  restaurar(ja) {
    const m = this.max;
    if (!m) return;
    this.max = null;
    this.veu.classList.remove('ligado');
    document.body.classList.remove('cartao-maximizado');
    m.c.querySelector('.cf-max')?.setAttribute('aria-label', 'Maximizar o cartão');
    const fim = () => {
      m.c.classList.remove('max', 'anim-max');
      for (const p of ['translate', 'scale', 'transformOrigin']) m.c.style[p] = '';
      m.vaga.remove();
      this.op.aoMaximizar?.(m.c, false);
    };
    if (ja || movimentoReduzido() || !m.vaga.isConnected) return fim();
    const r1 = m.c.getBoundingClientRect(), r0 = m.vaga.getBoundingClientRect();
    m.c.style.transformOrigin = '0 0';
    m.c.classList.add('anim-max');
    m.c.style.translate = `${r0.left - r1.left}px ${r0.top - r1.top}px`;
    m.c.style.scale = `${r0.width / r1.width} ${r0.height / r1.height}`;
    // termina bem antes de 500 ms do clique/tecla: a volta do cartão ao fluxo conta como resposta ao usuário (CLS)
    setTimeout(fim, 320);
  }
}
