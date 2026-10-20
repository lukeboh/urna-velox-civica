// Gráficos desenhados à mão em SVG, sem biblioteca: o anel de seções apuradas e a curva da apuração. Os valores são
// interpolados (mola amortecida) entre o anterior e o novo; um valor novo no meio do caminho só muda o alvo.

import { svg, el, movimentoReduzido } from './util.js';

/** Aproximação exponencial ao alvo (redirecionável): `tau` em ms. */
export class Mola {
  constructor(v = 0, tau = 380) { this.v = v; this.alvo = v; this.tau = tau; }
  definir(a) { this.alvo = a; if (movimentoReduzido()) this.v = a; }
  /** avança `dt` ms; devolve true se ainda está se movendo */
  passo(dt) {
    const d = this.alvo - this.v;
    if (Math.abs(d) < 1e-4 * Math.max(1, Math.abs(this.alvo))) { this.v = this.alvo; return false; }
    this.v += d * (1 - Math.exp(-dt / this.tau));
    return true;
  }
}

export class Anel {
  constructor() {
    const r = 54, c = 2 * Math.PI * r;
    this.c = c;
    this.el = el('div', 'anel');
    const s = svg('svg', { viewBox: '0 0 132 132', 'aria-hidden': 'true' });
    s.append(svg('circle', { class: 'fundo-anel', cx: 66, cy: 66, r, fill: 'none', 'stroke-width': 11 }));
    this.arco = svg('circle', { class: 'arco', cx: 66, cy: 66, r, fill: 'none', 'stroke-width': 11, 'stroke-dasharray': c, 'stroke-dashoffset': c });
    s.append(this.arco);
    this.centro = el('div', 'centro');
    this.el.append(s, this.centro);
    this.mola = new Mola(0, 500);
  }
  definir(frac) { this.mola.definir(Math.max(0, Math.min(1, frac || 0))); }
  quadro(dt) {
    const mexe = this.mola.passo(dt);
    this.arco.setAttribute('stroke-dashoffset', String(this.c * (1 - this.mola.v)));
    return mexe;
  }
}

/** Curva da apuração: % das seções apuradas (área, escala 0–100 %) e o % de cada candidato mostrado (linhas, escala
 * própria, ajustada ao intervalo dos valores: perto do fim da noite os percentuais mudam décimos, e na escala 0–100 as
 * linhas pareciam retas). Pontos do servidor (`GET /api/v1/curva/<topico>`, a noite inteira até agora) mais os que
 * chegam ao vivo; cada ponto guarda o % de todos os candidatos, então uma virada não troca as linhas de dono.
 * Domínios do tempo e da escala dos candidatos acompanham os dados com uma mola. */
export class Curva {
  constructor() {
    this.svg = svg('svg', { viewBox: '0 0 320 140', preserveAspectRatio: 'none', role: 'img', 'aria-label': 'Curva da apuração' });
    for (const y of [0, 35, 70, 105, 140]) this.svg.append(svg('line', { class: 'eixo', x1: 0, x2: 320, y1: y, y2: y }));
    const defs = svg('defs');
    const g = svg('linearGradient', { id: 'grad-area', x1: 0, y1: 0, x2: 0, y2: 1 });
    g.append(svg('stop', { offset: 0, 'stop-color': '#8c6bff', 'stop-opacity': 0.45 }), svg('stop', { offset: 1, 'stop-color': '#8c6bff', 'stop-opacity': 0 }));
    defs.append(g);
    this.svg.append(defs);
    this.area = svg('path', { class: 'area', fill: 'url(#grad-area)', d: '' });
    this.linhaSec = svg('path', { class: 'linha', stroke: '#a996ff', d: '' });
    this.ls = [svg('path', { class: 'linha', d: '' }), svg('path', { class: 'linha', d: '' })];
    this.svg.append(this.area, this.linhaSec, this.ls[1], this.ls[0]);
    // rótulos em HTML (o SVG estica): escala dos candidatos à direita, começo e fim do tempo embaixo
    this.rot = [0, 1, 2].map((k) => el('span', `rot-y r${k}`, ''));
    this.rotT = [el('span', 'rot-t ini', ''), el('span', 'rot-t fim', '')];
    this.el = el('div', 'graf', [this.svg, ...this.rot, ...this.rotT]);
    this.pontos = []; // { t, s, c: Map(n → pct) }
    this.mostrar = []; // [{ n, cor }]
    this.t0 = new Mola(0, 600);
    this.t1 = new Mola(1, 600);
    this.lo = new Mola(0, 500);
    this.hi = new Mola(100, 500);
    this.sujo = true;
  }
  limpar() { this.pontos = []; this.sujo = true; }
  /** Pontos do servidor (substituem os que houver antes do último deles). */
  semear(pontos) {
    if (!pontos.length) return;
    const fim = pontos[pontos.length - 1].t;
    this.pontos = [...pontos, ...this.pontos.filter((p) => p.t > fim)];
    this.reposicionar(true);
  }
  /** Candidatos das linhas: [{ n, cor }] (os dois primeiros agora). */
  linhas(v) {
    const chave = v.map((x) => `${x.n}:${x.cor}`).join('|');
    if (chave === this.chave) return;
    this.chave = chave;
    this.mostrar = v;
    // style (não o atributo): a cor é um light-dark(), resolvido pelo tema
    this.ls.forEach((l, i) => { l.style.stroke = v[i]?.cor || 'transparent'; });
    this.reposicionar(false);
  }
  /** t = segundos com fração (relógio do sistema, M-003), s = % seções, cand = [[n, pct], …] */
  ponto(t, s, cand) {
    if (t == null) return;
    const u = this.pontos[this.pontos.length - 1];
    if (u && t < u.t) return;
    const p = { t, s, c: new Map(cand.filter((x) => x[1] != null)) };
    if (u && t === u.t) this.pontos[this.pontos.length - 1] = p; else this.pontos.push(p);
    if (this.pontos.length > 1500) this.pontos = this.pontos.filter((_, i) => i % 2 === 0 || i === this.pontos.length - 1);
    this.reposicionar(this.pontos.length === 1);
  }
  reposicionar(ja) {
    if (!this.pontos.length) return;
    const p0 = this.pontos[0].t, p1 = this.pontos[this.pontos.length - 1].t;
    this.t0.definir(p0);
    this.t1.definir(Math.max(p1, p0 + 60));
    let lo = Infinity, hi = -Infinity;
    // os primeiros minutos (menos de 1 % das seções) oscilam muito: ficam fora da escala (a linha sai pela borda)
    const ult = this.pontos[this.pontos.length - 1];
    const base = ult.s >= 1 ? this.pontos.filter((p) => p.s >= 1) : this.pontos;
    for (const p of base) for (const m of this.mostrar) { const v = p.c.get(m.n); if (v != null) { lo = Math.min(lo, v); hi = Math.max(hi, v); } }
    if (lo === Infinity) { lo = 0; hi = 100; }
    // pelo menos 4 p.p. de altura, 10 % de folga, sem passar de 0–100
    const meio = (lo + hi) / 2, meia = Math.max(2, ((hi - lo) / 2) * 1.1);
    lo = Math.max(0, meio - meia); hi = Math.min(100, meio + meia);
    this.lo.definir(lo); this.hi.definir(hi);
    if (ja) { this.t0.v = p0; this.t1.v = Math.max(p1, p0 + 60); this.lo.v = lo; this.hi.v = hi; }
    this.sujo = true;
  }
  quadro(dt) {
    const m = this.t0.passo(dt) | this.t1.passo(dt) | this.lo.passo(dt) | this.hi.passo(dt);
    if (!m && !this.sujo) return false;
    this.sujo = false;
    const t0 = this.t0.v, span = Math.max(1, this.t1.v - t0), lo = this.lo.v, alt = Math.max(0.5, this.hi.v - lo);
    const X = (t) => ((t - t0) / span) * 320;
    const Ys = (p) => 140 - (Math.max(0, Math.min(100, p || 0)) / 100) * 140;
    const Yc = (v) => Math.max(-4, Math.min(144, 140 - ((v - lo) / alt) * 140));
    const ds = this.pontos.map((p, i) => `${i ? 'L' : 'M'}${X(p.t).toFixed(1)},${Ys(p.s).toFixed(1)}`).join('');
    this.linhaSec.setAttribute('d', ds);
    if (this.pontos.length) {
      const ult = this.pontos[this.pontos.length - 1];
      this.area.setAttribute('d', `${ds}L${X(ult.t).toFixed(1)},140L${X(this.pontos[0].t).toFixed(1)},140Z`);
    }
    this.ls.forEach((l, i) => {
      const n = this.mostrar[i]?.n;
      let d = '', novo = true;
      if (n != null) for (const p of this.pontos) {
        const v = p.c.get(n);
        if (v == null) { novo = true; continue; }
        d += `${novo ? 'M' : 'L'}${X(p.t).toFixed(1)},${Yc(v).toFixed(1)}`;
        novo = false;
      }
      l.setAttribute('d', d);
    });
    const f = (v) => `${v.toFixed(alt < 6 ? 1 : 0).replace('.', ',')}%`;
    [this.hi.v, lo + alt / 2, lo].forEach((v, k) => { const x = this.mostrar.length ? f(v) : ''; if (this.rot[k].textContent !== x) this.rot[k].textContent = x; });
    // M-003: eixo no relógio do sistema; no simulado acelerado a noite cabe em poucos minutos: com os segundos
    const hm = (t) => (this.pontos.length ? new Date(Math.floor(t) * 1000).toISOString().slice(11, span < 600 ? 19 : 16) : '');
    [hm(t0), hm(t0 + span)].forEach((x, k) => { if (this.rotT[k].textContent !== x) this.rotT[k].textContent = x; });
    return m;
  }
}
