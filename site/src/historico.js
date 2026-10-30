// Histórico do feed: tudo o que passou pelo letreiro (marcos, eventos, primeiros eleitos), em ordem de BU aplicado,
// num painel que sobe do rodapé. Filtro por categoria e por texto, e uma linha do tempo própria (slider) para voltar
// ao que já passou sem mexer na apuração. O painel é fixo e de altura fixa: abrir, filtrar ou receber itens não
// desloca nada da página (CLS 0). Os itens fixos (eleitores que faltam, disputas, comparecimento) mudam no lugar e
// não entram aqui.

import { el, mil } from './util.js';
import { CATEGORIAS } from './textos.js';

const MAX = 20000;

export class Historico {
  constructor(raiz, botao) {
    this.raiz = raiz;
    this.botao = botao;
    this.lista = raiz.querySelector('#hist-lista');
    this.tempo = raiz.querySelector('#hist-tempo');
    this.hora = raiz.querySelector('#hist-hora');
    this.ini = raiz.querySelector('#hist-ini');
    this.fim = raiz.querySelector('#hist-fim');
    this.vazio = raiz.querySelector('#hist-vazio');
    this.texto = raiz.querySelector('#hist-busca');
    this.itens = [];          // ordenados por (seq, ordem de chegada)
    this.porId = new Map();
    this.vis = [];            // os que passam no filtro, na ordem
    this.cats = new Set(CATEGORIAS.map((c) => c[0]));
    this.busca = '';
    this.atual = -1;          // índice em vis
    this.vivo = true;         // acompanha o mais novo (slider no fim)
    this.n = 0;
    this.chips = new Map();
    const fil = raiz.querySelector('#hist-filtros');
    for (const [k, nome] of CATEGORIAS) {
      const b = el('button', 'chip', [el('span', 'chip-nome', nome), el('span', 'chip-n', '0')]);
      b.type = 'button';
      b.dataset.cat = k;
      b.setAttribute('aria-pressed', 'true');
      b.addEventListener('click', () => {
        if (this.cats.has(k)) this.cats.delete(k); else this.cats.add(k);
        b.setAttribute('aria-pressed', String(this.cats.has(k)));
        this.#filtrar();
      });
      this.chips.set(k, b);
      fil.insertBefore(b, this.texto);
    }
    this.texto.addEventListener('input', () => { this.busca = this.texto.value.trim().toLowerCase(); this.#filtrar(); });
    this.tempo.addEventListener('input', () => { this.vivo = +this.tempo.value >= this.vis.length - 1; this.#ir(+this.tempo.value, true); });
    this.lista.addEventListener('scroll', () => {
      if (this.rolandoEu) { this.rolandoEu = false; return; }
      // rolagem à mão: o slider acompanha o item do meio da lista
      const meio = this.lista.scrollTop + this.lista.clientHeight / 2;
      let a = 0, b = this.vis.length - 1;
      while (a < b) { const m = (a + b + 1) >> 1; if (this.vis[m].row.offsetTop <= meio) a = m; else b = m - 1; }
      this.vivo = this.lista.scrollTop + this.lista.clientHeight >= this.lista.scrollHeight - 4;
      this.#ir(this.vivo ? this.vis.length - 1 : a, false);
    }, { passive: true });
    botao.addEventListener('click', () => (this.aberto ? this.fechar() : this.abrir()));
    raiz.querySelector('#hist-fechar').addEventListener('click', () => this.fechar());
    document.addEventListener('keydown', (e) => { if (e.key === 'Escape' && this.aberto) this.fechar(); });
  }

  get aberto() { return this.raiz.classList.contains('aberta'); }

  abrir() {
    this.raiz.classList.add('aberta');
    this.raiz.setAttribute('aria-hidden', 'false');
    this.botao.setAttribute('aria-expanded', 'true');
    this.vivo = true;
    this.#ir(this.vis.length - 1, true);
    this.tempo.focus({ preventScroll: true });
  }

  fechar() {
    if (!this.aberto) return;
    this.raiz.classList.remove('aberta');
    this.raiz.setAttribute('aria-hidden', 'true');
    this.botao.setAttribute('aria-expanded', 'false');
  }

  /** Guarda um item do feed (os de `cat` definida). Repetido pelo id: atualiza o texto. */
  /** Tira todos os itens (consulta de um pleito: o slider voltou no tempo). */
  limpar() {
    for (const x of this.itens) x.row?.remove();
    this.itens = [];
    this.porId.clear();
    this.vis = [];
    this.atual = -1;
    this.conta = new Map();
    this.contaSuja = true;
    this.sujo = true;
  }

  por(it) {
    if (!it.cat) return;
    const ja = this.porId.get(it.id);
    if (ja) {
      if (it.texto != null && it.texto !== ja.texto) { ja.texto = it.texto; if (ja.row) ja.row.querySelector('.h-txt').textContent = it.texto; }
      ja.detalhe = it.detalhe;
      return;
    }
    const r = { id: it.id, cat: it.cat, seq: it.seq ?? Infinity, n: this.n++, ic: it.ic || '•', texto: it.texto || '', t: it.t || '', detalhe: it.detalhe, row: null };
    r.chave = `${r.texto} ${r.t}`.toLowerCase();
    // posição: por seq e, no empate, por ordem de chegada (quase sempre é o fim)
    let k = this.itens.length;
    while (k > 0 && this.itens[k - 1].seq > r.seq) k--;
    this.itens.splice(k, 0, r);
    this.porId.set(r.id, r);
    this.conta ||= new Map();
    this.conta.set(r.cat, (this.conta.get(r.cat) || 0) + 1);
    if (this.itens.length > MAX) { const v = this.itens.shift(); this.porId.delete(v.id); v.row?.remove(); this.contaSuja = true; }
    if (!this.aberto) { this.sujo = true; return; }
    this.#linha(r, this.itens[k + 1]?.row || null);
    if (!this.#passa(r)) return;
    let p = this.vis.length;
    while (p > 0 && (this.vis[p - 1].seq > r.seq)) p--;
    this.vis.splice(p, 0, r);
    if (this.vivo) this.#ir(this.vis.length - 1, true);
    else { if (p <= this.atual) this.atual++; this.#rotulos(); }
  }

  #passa(r) { return this.cats.has(r.cat) && (!this.busca || r.chave.includes(this.busca)); }

  #contar() {
    const c = new Map();
    for (const r of this.itens) c.set(r.cat, (c.get(r.cat) || 0) + 1);
    this.conta = c;
    this.contaSuja = false;
  }
  #mostrarContas() {
    if (this.contaSuja) this.#contar();
    for (const [k, b] of this.chips) { const x = mil(this.conta?.get(k) || 0); const s = b.querySelector('.chip-n'); if (s.textContent !== x) s.textContent = x; }
  }

  #linha(r, antes) {
    const cab = el('button', 'h-cab', [el('span', 'ic', r.ic), el('span', 'h-txt', r.texto), el('span', 'h-t', r.t)]);
    cab.type = 'button';
    cab.setAttribute('aria-expanded', 'false');
    const row = el('div', `h-item c-${r.cat}`, [cab]);
    row.setAttribute('role', 'listitem');
    cab.addEventListener('click', () => {
      const aberto = cab.getAttribute('aria-expanded') === 'true';
      row.querySelector('.h-det')?.remove();
      cab.setAttribute('aria-expanded', String(!aberto));
      if (!aberto && r.detalhe) row.append(el('dl', 'h-det', r.detalhe.linhas.flatMap(([a, b]) => [el('dt', null, a), el('dd', null, b)])));
      this.vivo = false;
      this.#ir(this.vis.indexOf(r), false);
    });
    r.row = row;
    row.hidden = !this.#passa(r);
    this.lista.insertBefore(row, antes);
  }

  /** Monta as linhas que faltam (o painel só cria o DOM quando aberto) e refaz o filtro. */
  #filtrar() {
    if (this.sujo) {
      this.sujo = false;
      const frag = document.createDocumentFragment();
      this.lista.replaceChildren();
      for (const r of this.itens) { if (!r.row) this.#linha(r, null); frag.append(r.row); }
      this.lista.append(frag);
    }
    const antes = this.vis[this.atual];
    this.vis = [];
    for (const r of this.itens) { const p = this.#passa(r); r.row.hidden = !p; if (p) this.vis.push(r); }
    // fica no mesmo item; se ele saiu do filtro, no primeiro que vem depois dele
    let i = antes ? this.vis.indexOf(antes) : -1;
    if (antes && i < 0) { i = this.vis.findIndex((r) => r.seq > antes.seq || (r.seq === antes.seq && r.n > antes.n)); if (i < 0) i = this.vis.length - 1; }
    this.#ir(this.vivo || i < 0 ? this.vis.length - 1 : i, true);
  }

  /** Vai ao item i de `vis`: marca, rola a lista até ele (se `rolar`) e atualiza o slider e os rótulos. */
  #ir(i, rolar) {
    if (this.sujo && this.aberto) return this.#filtrar();
    const ant = this.vis[this.atual];
    this.atual = Math.max(-1, Math.min(i, this.vis.length - 1));
    const r = this.vis[this.atual];
    if (ant !== r) { ant?.row?.classList.remove('atual'); r?.row?.classList.add('atual'); }
    if (rolar && r?.row && this.aberto) {
      const alvo = Math.max(0, r.row.offsetTop - (this.lista.clientHeight - r.row.offsetHeight) / 2);
      if (Math.abs(this.lista.scrollTop - alvo) > 1) { this.rolandoEu = true; this.lista.scrollTop = alvo; }
    }
    this.#rotulos();
  }

  #rotulos() {
    const n = this.vis.length, r = this.vis[this.atual];
    this.#mostrarContas();
    this.tempo.max = String(Math.max(0, n - 1));
    this.tempo.value = String(Math.max(0, this.atual));
    this.tempo.disabled = n < 2;
    this.tempo.style.setProperty('--p', `${n > 1 ? (100 * this.atual) / (n - 1) : 100}%`);
    this.tempo.setAttribute('aria-valuetext', r ? `${r.t}: ${r.texto}` : 'nenhum item');
    const h = r ? `${r.t || '—'} · ${mil(this.atual + 1)} de ${mil(n)}` : 'nada ainda';
    if (this.hora.textContent !== h) this.hora.textContent = h;
    this.ini.textContent = this.vis[0]?.t || '';
    this.fim.textContent = n ? (this.vivo ? 'agora' : this.vis[n - 1].t) : '';
    this.vazio.hidden = n > 0;
  }
}
