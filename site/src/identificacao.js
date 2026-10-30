// M-025 (30/10/2026; proposta P2 escolhida pelo Luciano, com os ajustes dele): a barra da eleição que está na tela, logo
// abaixo do topo (que só tem o que é geral: a marca e o tema). Presa no alto ao rolar (`position: sticky`), de altura
// fixa (nada pula quando ela se preenche):
//  * 1ª linha: a eleição — "Eleições Gerais 2026", "Eleição suplementar · Roraima", "Eleições Municipais 2024" —, o turno e
//    a data do pleito, num botão que abre a escolha da eleição (pleitos.js); e o selo do modo (AO VIVO / REPLAY /
//    CONSULTA / EM BREVE, app.js);
//  * 2ª linha: o local (um seletor nativo por baixo do texto: Brasil, os estados e os municípios do estado da tela) e
//    as pílulas de cargo; as que não cabem vão para o botão "…" (também um seletor nativo), e a do cargo da tela fica
//    sempre à vista.
// Substitui o cartão da M-001 (que tinha o botão de recolher), a escolha da abrangência do topo e o chip do pleito da faixa.

const $ = (s, r = document) => r.querySelector(s);
const DIAS = ['domingo', 'segunda-feira', 'terça-feira', 'quarta-feira', 'quinta-feira', 'sexta-feira', 'sábado'];
const DIAS_C = ['dom.', 'seg.', 'ter.', 'qua.', 'qui.', 'sex.', 'sáb.'];

const trocar = (e, s) => { if (e && e.textContent !== s) e.textContent = s; };

export class Identificacao {
  /** `aoLocal(abr)`, `aoCargo(cargo)`: o visitante escolheu outro local / outro cargo (pelo "…") */
  constructor({ aoLocal, aoCargo } = {}) {
    this.raiz = $('#eleicao');
    this.btn = $('.el-pleito', this.raiz);
    this.el = { nome: $('.el-nome', this.raiz), turno: $('.el-turno', this.raiz), dia: $('.el-dia', this.raiz), diaC: $('.el-dia-c', this.raiz),
      dd: $('.el-dd', this.raiz), local: $('.el-local-txt', this.raiz) };
    this.localSel = $('.el-local select', this.raiz);
    this.localSel.addEventListener('change', () => { if (this.localSel.value) aoLocal?.(this.localSel.value); });
    this.cargos = $('#cargos', this.raiz);
    this.mais = $('.el-mais', this.raiz);
    this.maisSel = $('select', this.mais);
    this.maisSel.addEventListener('change', () => { const v = this.maisSel.value; this.maisSel.value = ''; if (v) aoCargo?.(+v); });
    this.info = { nome: '', dataIso: '', turno: null, local: '', cargo: '' };
    // presa no alto: a faixa por trás da barra (cobre o que rola por baixo) aparece só por opacidade
    const ver = () => {
      const presa = window.scrollY > 0 && this.raiz.getBoundingClientRect().top <= 0.5;
      if (this.raiz.classList.contains('presa') !== presa) this.raiz.classList.toggle('presa', presa);
    };
    window.addEventListener('scroll', ver, { passive: true });
    // os cargos que cabem dependem da largura
    let pend = 0;
    window.addEventListener('resize', () => { ver(); cancelAnimationFrame(pend); pend = requestAnimationFrame(() => this.ajustarCargos()); }, { passive: true });
    ver();
  }

  /** A eleição: { nome: "Eleições Gerais 2026", dataIso: "2026-10-04" (ou "") }. */
  eleicao({ nome, dataIso }) {
    if (nome) { this.info.nome = nome; trocar(this.el.nome, nome); }
    if (dataIso !== undefined) {
      this.info.dataIso = dataIso || '';
      const m = /^(\d{4})-(\d\d)-(\d\d)$/.exec(dataIso || '');
      const d = m ? new Date(Date.UTC(+m[1], +m[2] - 1, +m[3])).getUTCDay() : null;
      trocar(this.el.dia, m ? `${DIAS[d]}, ` : '');
      trocar(this.el.diaC, m ? `${DIAS_C[d]} ` : '');
      trocar(this.el.dd, m ? `${m[3]}/${m[2]}/${m[1]}` : '');
    }
    this.#rotulos();
  }

  /** O que está na tela: { turno, local, cargo }. */
  definir({ turno, local, cargo }) {
    Object.assign(this.info, { turno, local, cargo });
    trocar(this.el.turno, turno ? `${turno}º turno` : '');
    trocar(this.el.local, local || '');
    this.#rotulos();
    // o nome do local mudou a largura que sobra para os cargos (ainda antes do primeiro desenho)
    this.ajustarCargos();
    document.title = `${cargo ? `${cargo} · ${local} · ` : ''}${turno ? `${turno}º turno · ` : ''}Urna Velox`;
  }

  #rotulos() {
    const i = this.info;
    const data = i.dataIso ? `${this.el.dia.textContent}${this.el.dd.textContent}` : '';
    const t = [i.nome, i.turno ? `${i.turno}º turno` : '', data].filter(Boolean).join(' · ');
    const dica = `${t}. Trocar de eleição: em curso, agendadas e encerradas`;
    if (this.btn.title !== dica) { this.btn.title = dica; this.btn.setAttribute('aria-label', dica); }
    this.raiz.classList.toggle('sem-data', !i.dataIso);
  }

  /** O seletor do local: `grupos` = [[rótulo do grupo ou null, [[valor, texto], …]], …], `atual` = valor da tela. */
  locais(grupos, atual) {
    const s = this.localSel;
    s.replaceChildren();
    let n = 0;
    for (const [rot, itens] of grupos) {
      if (!itens.length) continue;
      const alvo = rot ? Object.assign(document.createElement('optgroup'), { label: rot }) : s;
      for (const [v, t] of itens) { alvo.append(new Option(t, v, false, v === atual)); n++; }
      if (rot) s.append(alvo);
    }
    s.disabled = n < 2;
    this.raiz.classList.toggle('local-unico', n < 2);
  }

  /** Depois de as pílulas entrarem em #cargos: as que não cabem vão para o "…" (a da tela fica sempre). */
  ajustarCargos() {
    const pils = [...this.cargos.children];
    for (const b of pils) b.hidden = false;
    this.mais.classList.add('oculto');
    const cabe = () => this.cargos.scrollWidth <= this.cargos.clientWidth + 1;
    if (!cabe()) {
      this.mais.classList.remove('oculto');
      for (let k = pils.length - 1; k >= 0 && !cabe(); k--) if (pils[k].getAttribute('aria-selected') !== 'true') pils[k].hidden = true;
    }
    const fora = pils.filter((b) => b.hidden);
    this.maisSel.replaceChildren(new Option('Outros cargos…', ''), ...fora.map((b) => new Option(b.textContent, b.dataset.cargo)));
    this.mais.classList.toggle('oculto', !fora.length);
    this.maisSel.value = '';
  }
}
