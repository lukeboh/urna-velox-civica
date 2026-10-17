// Feed de marcos e curiosidades: um letreiro horizontal fixo no rodapé, que passa por transform (translateX), pausa
// ao passar o mouse, ao focar ou ao tocar, e abre o detalhe ao clicar. Cada item tem posição própria (absoluta); um
// marco novo entra logo depois da borda direita visível e só os itens que estão fora da tela abrem espaço — nada que
// está à vista se mexe fora do ritmo. Com prefers-reduced-motion, mostra um item por vez com fade.

import { el, movimentoReduzido } from './util.js';

const VEL = 55;      // px/s
const GAP = 28;
const MAX = 70;      // itens no letreiro (os mais antigos saem quando passam pela borda esquerda)

export class Feed {
  constructor(raiz, abrirDetalhe) {
    this.raiz = raiz;
    this.trilho = raiz.querySelector('#trilho');
    this.vazio = raiz.querySelector('#feed-vazio');
    this.anuncio = raiz.querySelector('#anuncio');
    this.abrirDetalhe = abrirDetalhe;
    this.itens = [];           // { id, el, w, x, fixo, sair }
    this.porId = new Map();
    this.offset = 0;
    this.pausas = new Set();
    this.reduzido = movimentoReduzido();
    this.atual = 0;
    this.trocaEm = 0;
    this.filaAnuncio = [];
    this.anuncioEm = 0;
    const pausar = (k, on) => { if (on) this.pausas.add(k); else this.pausas.delete(k); raiz.dataset.pausado = this.pausas.size ? '1' : '0'; };
    raiz.addEventListener('pointerenter', (e) => { if (e.pointerType === 'mouse') pausar('mouse', true); });
    raiz.addEventListener('pointerleave', (e) => { if (e.pointerType === 'mouse') pausar('mouse', false); });
    raiz.addEventListener('focusin', () => pausar('foco', true));
    raiz.addEventListener('focusout', () => pausar('foco', false));
    raiz.addEventListener('pointerdown', (e) => {
      if (e.pointerType === 'mouse') return;
      pausar('toque', true);
      clearTimeout(this.tToque);
      this.tToque = setTimeout(() => pausar('toque', false), 6000);
    });
    window.matchMedia('(prefers-reduced-motion: reduce)').addEventListener?.('change', () => { this.reduzido = movimentoReduzido(); });
  }

  // o trilho começa depois do botão do histórico
  get largura() { return this.trilho.clientWidth; }

  /**
   * Acrescenta (ou atualiza, pelo id) um item. `it` = { id, ic, texto, t, detalhe, fixo, hod, anunciar }.
   * `fixo`: item de largura fixa cujo conteúdo muda (ex.: eleitores que faltam), atualizado no lugar.
   */
  por(it) {
    const ja = this.porId.get(it.id);
    if (ja) {
      if (it.texto != null && ja.txt.textContent !== it.texto) ja.txt.textContent = it.texto;
      if (it.t != null) ja.t.textContent = it.t;
      ja.detalhe = it.detalhe;
      return ja;
    }
    const b = el('button', 'item' + (it.fixo ? ' fixo' : ''));
    b.type = 'button';
    const ic = el('span', 'ic', it.ic || '•');
    ic.setAttribute('aria-hidden', 'true');
    const txt = el('span', 'txt', it.texto || '');
    const t = el('span', 't', it.t || '');
    b.append(ic, txt);
    if (it.hod) b.append(it.hod.el);
    b.append(t);
    const item = { id: it.id, el: b, txt, t, w: 0, x: 0, fixo: !!it.fixo, detalhe: it.detalhe, sair: false };
    b.addEventListener('click', () => this.abrirDetalhe(item.detalhe));
    b.style.opacity = this.reduzido ? '0' : '1';
    this.trilho.append(b);
    item.w = b.offsetWidth;
    this.#posicionar(item);
    this.itens.push(item);
    this.porId.set(it.id, item);
    this.vazio.style.opacity = '0';
    if (it.anunciar) this.filaAnuncio.push(it.anunciar);
    this.#limitar();
    return item;
  }

  /** Mais de MAX itens: tira os mais antigos. Fora da tela à direita, sai já e os seguintes (também fora da tela)
   * fecham o espaço; à vista, só quando passar pela borda esquerda. */
  #limitar() {
    const W = this.largura;
    let vivos = this.itens.filter((x) => !x.sair).length;
    for (const v of this.itens) {
      if (vivos <= MAX) break;
      if (v.fixo || v.sair) continue;
      vivos--;
      if (v.x > this.offset + W) {
        for (const i of this.itens) if (i.x > v.x) i.x -= v.w + GAP;
        v.el.remove();
        this.porId.delete(v.id);
        v.removido = true;
      } else v.sair = true;
    }
    this.itens = this.itens.filter((x) => !x.removido);
  }

  #posicionar(novo) {
    const W = this.largura;
    // ponto de entrada: logo depois do que cruza a borda direita (ou da própria borda)
    let p = this.offset + W;
    for (const i of this.itens) if (i.x < this.offset + W && i.x + i.w > p - GAP) p = Math.max(p, i.x + i.w + GAP);
    if (!this.itens.length) p = this.offset + 16;
    // abre espaço: só os itens que estão inteiramente fora da tela, à direita
    for (const i of this.itens) if (i.x >= p - 1) i.x += novo.w + GAP;
    novo.x = p;
    novo.el.style.transform = `translateX(${(novo.x - this.offset).toFixed(1)}px)`;
  }

  quadro(dt, agora) {
    if (this.filaAnuncio.length && agora - this.anuncioEm > 4000) {
      this.anuncio.textContent = this.filaAnuncio.shift();
      this.anuncioEm = agora;
      if (this.filaAnuncio.length > 3) this.filaAnuncio.splice(0, this.filaAnuncio.length - 3);
    }
    if (!this.itens.length) return;
    if (this.reduzido) return this.#quadroReduzido(agora);
    if (!this.pausas.size && !document.hidden) this.offset += (VEL * dt) / 1000;
    let fim = -Infinity;
    for (const i of this.itens) fim = Math.max(fim, i.x + i.w);
    const W = this.largura;
    for (let k = 0; k < this.itens.length; k++) {
      const i = this.itens[k];
      if (i.x + i.w < this.offset - 4) {
        if (i.sair) { i.el.remove(); this.porId.delete(i.id); this.itens.splice(k--, 1); continue; }
        // recomeça depois do último (o letreiro dá a volta)
        i.x = Math.max(fim + GAP, this.offset + W);
        fim = i.x + i.w;
      }
      i.el.style.transform = `translateX(${(i.x - this.offset).toFixed(1)}px)`;
    }
  }

  #quadroReduzido(agora) {
    // sem rolagem não há borda esquerda: os itens que saem vão embora já (menos o que está à vista)
    for (let k = 0; k < this.itens.length; k++) {
      const i = this.itens[k];
      if (i.sair && k !== this.atual) { i.el.remove(); this.porId.delete(i.id); this.itens.splice(k, 1); if (k < this.atual) this.atual--; k--; }
    }
    if (!this.itens.length) return;
    if (this.atual >= this.itens.length) this.atual = 0;
    if (agora < this.trocaEm && this.itens[this.atual]?.el.style.opacity === '1') return;
    if (!this.pausas.size) this.atual = (this.atual + 1) % this.itens.length;
    this.trocaEm = agora + 6000;
    this.itens.forEach((i, k) => {
      i.el.style.transform = 'translateX(16px)';
      i.el.style.opacity = k === this.atual ? '1' : '0';
      i.el.tabIndex = k === this.atual ? 0 : -1;
    });
  }
}

/** Folha de detalhe de um marco. */
export class Folha {
  constructor(raiz) {
    this.raiz = raiz;
    this.titulo = raiz.querySelector('#folha-titulo');
    this.corpo = raiz.querySelector('#folha-corpo');
    raiz.querySelector('#folha-fechar').addEventListener('click', () => this.fechar());
    document.addEventListener('keydown', (e) => { if (e.key === 'Escape') this.fechar(); });
    document.addEventListener('pointerdown', (e) => {
      if (this.raiz.classList.contains('aberta') && !this.raiz.contains(e.target) && !e.target.closest?.('.feed')) this.fechar();
    });
  }
  abrir(d) {
    if (!d) return;
    this.titulo.textContent = d.titulo;
    this.corpo.replaceChildren(...d.linhas.flatMap(([a, b]) => [el('dt', null, a), el('dd', null, b)]));
    this.raiz.classList.add('aberta');
    this.raiz.setAttribute('aria-hidden', 'false');
  }
  fechar() {
    this.raiz.classList.remove('aberta');
    this.raiz.setAttribute('aria-hidden', 'true');
  }
}
