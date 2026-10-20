// M-001: identificação permanente da eleição, do turno e da abrangência que estão na tela (variação "C", escolhida pelo
// Luciano em 18/10/2026 entre três protótipos): cartão à moda do aplicativo da Justiça Eleitoral — "Eleições 2026 ·
// Gerais", o turno em destaque e, numa faixa colorida, o local e o cargo. O lugar do cartão é reservado no `index.html`
// (nada pula quando ele se preenche). Quando ele sai da tela ao rolar, aparece a barra `.ident-barra` — a mesma
// identificação numa linha, sempre `position: fixed` no alto; ela entra só por opacidade e deslocamento (transform),
// que não contam como deslocamento de layout (CLS). O botão recolhe o cartão e a barra até um botão pequeno, e a
// escolha fica lembrada no navegador.

const $ = (s) => document.querySelector(s);
const lembrar = (v) => { try { if (v == null) return localStorage.getItem('uv-ident'); localStorage.setItem('uv-ident', v); } catch { /* sem armazenamento */ } return null; };
// recolhido da última visita: já ao carregar o módulo, antes do primeiro desenho do painel
if (lembrar() === 'recolhida') document.documentElement.classList.add('ident-recolhida');

export class Identificacao {
  constructor() {
    this.lugar = $('#ident');
    const todos = (s) => [...this.lugar.querySelectorAll(s)];
    this.el = { eleicao: todos('.id-eleicao'), tipo: todos('.id-tipo'), local: todos('.id-local'), cargo: todos('.id-cargo'), seg: todos('.id-seg') };
    this.btns = todos('.id-recolher');
    for (const b of this.btns) b.addEventListener('click', () => this.recolher(!document.documentElement.classList.contains('ident-recolhida')));
    this.recolher(lembrar() === 'recolhida', false);
    // a barra aparece quando o cartão sai da tela (o lugar do cartão continua igual: nada pula)
    const ver = () => {
      const preso = this.lugar.getBoundingClientRect().bottom < 4;
      if (this.lugar.classList.contains('preso') !== preso) this.lugar.classList.toggle('preso', preso);
    };
    window.addEventListener('scroll', ver, { passive: true });
    window.addEventListener('resize', ver, { passive: true });
    ver();
  }

  recolher(sim, guardar = true) {
    document.documentElement.classList.toggle('ident-recolhida', sim);
    for (const b of this.btns) {
      b.textContent = sim ? '⌄' : '⌃';
      b.setAttribute('aria-expanded', String(!sim));
      b.setAttribute('aria-label', sim ? 'Mostrar a identificação da eleição' : 'Recolher a identificação da eleição');
    }
    if (guardar) lembrar(sim ? 'recolhida' : 'aberta');
  }

  /** { ano, geral, turno, cargo, local } */
  definir(x) {
    const t = (es, s) => { for (const e of es) if (e.textContent !== s) e.textContent = s; };
    t(this.el.eleicao, `Eleições ${x.ano || ''}`.trim());
    t(this.el.tipo, x.geral ? 'Gerais' : 'Municipais');
    t(this.el.cargo, x.cargo || '');
    t(this.el.local, x.local || '');
    for (const seg of this.el.seg) {
      [...seg.children].forEach((i, k) => i.classList.toggle('sel', k + 1 === x.turno));
      seg.setAttribute('aria-label', x.turno ? `${x.turno}º turno` : '');
    }
    document.title = `${x.cargo ? `${x.cargo} · ${x.local} · ` : ''}${x.turno ? `${x.turno}º turno · ` : ''}Urna Velox`;
  }
}
