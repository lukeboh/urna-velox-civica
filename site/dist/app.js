// Urna Velox — página de resultados (gerado por site/build.mjs a partir de site/src; não edite aqui)

// src/conexao.js
var MAX_POR_MSG = 90;
var Conexao = class {
  /** @param {(m: object) => void} aoReceber  @param {(estado: string, info?: object) => void} aoEstado */
  constructor(aoReceber, aoEstado) {
    this.aoReceber = aoReceber;
    this.aoEstado = aoEstado;
    this.topicos = /* @__PURE__ */ new Set();
    this.faltam = [];
    this.ws = null;
    this.espera = 1e3;
    this.timer = null;
    this.quedas = 0;
    this.fila = [];
  }
  url() {
    const p = location.protocol === "https:" ? "wss:" : "ws:";
    return `${p}//${location.host}/ws`;
  }
  abrir() {
    clearTimeout(this.timer);
    this.aoEstado("conectando");
    let ws;
    try {
      ws = new WebSocket(this.url());
    } catch {
      return this.#agendar();
    }
    this.ws = ws;
    ws.onopen = () => {
      this.espera = 1e3;
      this.aoEstado("vivo");
      this.#mandarInscricoes([...this.topicos]);
      this.#mandar({ op: "eventos", escopo: "uf" });
      this.#mandar({ op: "marcos", escopo: "uf", faltam: this.faltam });
    };
    ws.onmessage = (e) => {
      const d = e.data;
      if (d.startsWith('{"tipo":"atualizacao"') || d.startsWith('{"tipo":"instantaneo"')) {
        const r = /"topico":"([^"]+)","seq":(\d+)/.exec(d.slice(0, 160));
        if (r) return this.aoReceber({ tipo: "resultado", topico: r[1], seq: +r[2], txt: d });
      }
      let m;
      try {
        m = JSON.parse(e.data);
      } catch {
        return;
      }
      this.aoReceber(m);
    };
    ws.onclose = () => {
      if (this.ws !== ws) return;
      this.ws = null;
      this.quedas++;
      this.#agendar();
    };
    ws.onerror = () => {
    };
  }
  #agendar() {
    const t = Math.min(3e4, this.espera) * (0.75 + Math.random() * 0.5);
    this.espera = Math.min(3e4, this.espera * 2);
    this.aoEstado("caiu", { em: Date.now() + t });
    this.timer = setTimeout(() => this.abrir(), t);
  }
  #mandar(o) {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) this.ws.send(JSON.stringify(o));
  }
  #mandarInscricoes(lista) {
    for (let i = 0; i < lista.length; i += MAX_POR_MSG) this.#mandar({ op: "inscrever", topicos: lista.slice(i, i + MAX_POR_MSG) });
  }
  /** Troca a lista de tópicos com `faltam_para_mat` (o servidor manda o valor atual dos que entraram). */
  definirFaltam(lista) {
    if (lista.length === this.faltam.length && lista.every((t, i) => t === this.faltam[i])) return;
    this.faltam = lista;
    this.#mandar({ op: "marcos", escopo: "uf", faltam: lista });
  }
  /** Inscreve os tópicos que faltam e cancela os que saíram (a lista inteira desejada). */
  definirTopicos(lista) {
    const novo = new Set(lista);
    const entrar = [...novo].filter((t) => !this.topicos.has(t));
    const sair = [...this.topicos].filter((t) => !novo.has(t));
    this.topicos = novo;
    if (entrar.length) this.#mandarInscricoes(entrar);
    for (let i = 0; i < sair.length; i += MAX_POR_MSG) this.#mandar({ op: "cancelar", topicos: sair.slice(i, i + MAX_POR_MSG) });
  }
};

// src/util.js
var movimentoReduzido = () => window.matchMedia("(prefers-reduced-motion: reduce)").matches;
function mil(x) {
  if (x == null || !Number.isFinite(x)) return "—";
  return Math.round(x).toString().replace(/\B(?=(\d{3})+(?!\d))/g, ".");
}
function pct(x, casas = 2) {
  if (x == null || !Number.isFinite(x)) return "—";
  return x.toFixed(casas).replace(".", ",");
}
var NOME_CARGO = { 1: "Presidente", 3: "Governador", 5: "Senador", 11: "Prefeito" };
var NOME_CARGO_PLURAL = { 1: "presidente", 3: "governador", 5: "senador", 11: "prefeito" };
function abr(cod) {
  if (cod === "br") return { tipo: "br", uf: null, mun: null };
  if (cod.length === 2) return { tipo: "uf", uf: cod, mun: null };
  return { tipo: "mun", uf: cod.slice(0, 2), mun: cod };
}
function titulo(s) {
  if (!s) return "";
  const pequenas = /* @__PURE__ */ new Set(["de", "da", "do", "das", "dos", "e", "d'"]);
  return s.toLowerCase().split(/(\s+|-)/).map((p, i) => i > 0 && pequenas.has(p) ? p : p.charAt(0).toUpperCase() + p.slice(1)).join("");
}
function hora(t) {
  if (!t) return "";
  const m = /(\d{2}):(\d{2}):(\d{2})/.exec(t);
  return m ? `${m[1]}:${m[2]}:${m[3]}` : "";
}
function segundos(t) {
  if (!t) return null;
  const m = /(\d{4})-(\d{2})-(\d{2})[T ](\d{2}):(\d{2}):(\d{2})/.exec(t);
  if (!m) return null;
  return Date.UTC(+m[1], +m[2] - 1, +m[3], +m[4], +m[5], +m[6]) / 1e3;
}
function el(tag, cls, filhos) {
  const e = document.createElement(tag);
  if (cls) e.className = cls;
  if (filhos != null) for (const f2 of [].concat(filhos)) e.append(f2);
  return e;
}
function svg(tag, attrs = {}) {
  const e = document.createElementNS("http://www.w3.org/2000/svg", tag);
  for (const [k, v] of Object.entries(attrs)) e.setAttribute(k, v);
  return e;
}
function digitos(n) {
  return Math.max(1, Math.floor(Math.log10(Math.max(1, n))) + 1);
}

// src/hodometro.js
var COPIAS = 3;
var POS = COPIAS * 10;
function coluna() {
  const col = el("span", "col");
  for (let c = 0; c < COPIAS; c++) for (let d = 0; d <= 9; d++) col.append(el("span", null, String(d)));
  col.style.transform = posicao(10);
  return col;
}
var posicao = (p) => `translateY(${-p * 100 / POS}%)`;
function naTela(col) {
  const alt = col.offsetHeight / POS;
  if (!(alt > 0)) return null;
  const t = getComputedStyle(col).transform;
  if (!t || t === "none") return 0;
  return -new DOMMatrixReadOnly(t).m42 / alt;
}
var Hodometro = class {
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
    this.el = el("span", op.enxuto ? "hod enxuto" : "hod");
    this.dig = [];
    this.seps = [];
    this.valor = void 0;
    const vis = el("span", "vis");
    vis.setAttribute("aria-hidden", "true");
    this.vis = vis;
    this.desloc = 0;
    this.texto = el("span", "sr");
    if (op.molde) {
      for (const ch of op.molde) {
        if (ch === "#") this.#digito(vis);
        else vis.append(el("span", "sp", ch));
      }
    } else {
      const n = Math.max(1, op.inteiros || 1);
      for (let i = 0; i < n; i++) {
        if (i > 0 && (n - i) % 3 === 0) {
          const s = el("span", "sp", ".");
          this.seps.push({ span: s, apos: i - 1 });
          vis.append(s);
        }
        this.#digito(vis);
      }
      this.nInt = n;
      if (op.casas) {
        vis.append(el("span", "sp", ","));
        for (let i = 0; i < op.casas; i++) this.#digito(vis);
      }
    }
    if (op.sufixo) vis.append(el("span", "suf", op.sufixo));
    this.el.append(vis, this.texto);
    this.definir(null);
  }
  #digito(pai) {
    const wrap = el("span", "dg");
    const col = coluna();
    wrap.append(col);
    pai.append(wrap);
    this.dig.push({ wrap, col, d: 0, p: 10, vazio: null });
  }
  #por(i, ch) {
    const g = this.dig[i];
    const vazio = ch === " ";
    if (g.vazio !== vazio) {
      g.wrap.classList.toggle("vazio", vazio);
      g.vazio = vazio;
    }
    if (vazio) return;
    const d = ch.charCodeAt(0) - 48;
    if (d === g.d) return;
    const s = this.sentido;
    const passos = s > 0 ? (d - g.d + 10) % 10 : (g.d - d + 10) % 10;
    let alvo = g.p + s * passos;
    if (alvo < 0 || alvo > POS - 1) {
      const agora = naTela(g.col);
      const novo = s > 0 ? 20 + d : d;
      if (agora === null) {
        g.col.style.transition = "none";
        g.col.style.transform = posicao(novo);
        void g.col.offsetHeight;
        g.col.style.transition = "";
      } else {
        let falta = Math.max(0, s * (alvo - agora));
        if (falta > 20) falta -= 10 * Math.ceil((falta - 20) / 10);
        g.col.style.transition = "none";
        g.col.style.transform = posicao(novo - s * falta);
        void g.col.offsetHeight;
        g.col.style.transition = "";
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
      if (!x.classList.contains("vazio")) break;
      w += x.getBoundingClientRect().width;
    }
    if (!(this.vis.offsetWidth > 0)) return;
    const esc = this.vis.offsetWidth / (this.vis.getBoundingClientRect().width || this.vis.offsetWidth);
    w = Math.round(w * esc * 100) / 100;
    if (this.op.centro) w /= 2;
    if (w === this.desloc && this.medido) return;
    const primeira = !this.medido;
    this.medido = true;
    this.desloc = w;
    if (primeira) this.vis.style.transition = "none";
    this.vis.style.transform = w ? `translateX(${-w}px)` : "";
    if (primeira) {
      void this.vis.offsetWidth;
      this.vis.style.transition = "";
    }
  }
  /** Valor novo (número; texto no modo `molde`; `null` = desconhecido). */
  definir(v) {
    if (v === this.valor) return;
    const antes = this.valor;
    this.valor = v;
    const nulo = v == null || typeof v === "number" && !Number.isFinite(v);
    this.el.classList.toggle("nulo", nulo);
    if (this.op.molde) {
      const ds = nulo ? "" : String(v).replace(/\D/g, "");
      for (let i = 0; i < this.dig.length; i++) this.#por(i, ds[i] ?? "0");
      this.texto.textContent = nulo ? "—" : String(v);
    } else {
      const casas = this.op.casas || 0;
      const x = nulo ? 0 : Math.max(0, v);
      let [int, dec = ""] = x.toFixed(casas).split(".");
      if (int.length > this.nInt) int = "9".repeat(this.nInt);
      const cheio = int.padStart(this.nInt, " ");
      for (let i = 0; i < this.nInt; i++) this.#por(i, cheio[i]);
      for (let i = 0; i < casas; i++) this.#por(this.nInt + i, dec[i] ?? "0");
      for (const s of this.seps) s.span.classList.toggle("vazio", cheio[s.apos] === " ");
      if (this.op.enxuto) this.#deslizar();
      this.texto.textContent = nulo ? "—" : int.replace(/\B(?=(\d{3})+(?!\d))/g, ".") + (casas ? "," + dec : "") + (this.op.sufixo ? " " + this.op.sufixo : "");
    }
    if (antes !== void 0 && antes !== null && movimentoReduzido() && this.el.animate) {
      this.el.animate([{ opacity: 0.35 }, { opacity: 1 }], { duration: 220, easing: "ease-out" });
    }
  }
};

// src/graficos.js
var Mola = class {
  constructor(v = 0, tau = 380) {
    this.v = v;
    this.alvo = v;
    this.tau = tau;
  }
  definir(a) {
    this.alvo = a;
    if (movimentoReduzido()) this.v = a;
  }
  /** avança `dt` ms; devolve true se ainda está se movendo */
  passo(dt) {
    const d = this.alvo - this.v;
    if (Math.abs(d) < 1e-4 * Math.max(1, Math.abs(this.alvo))) {
      this.v = this.alvo;
      return false;
    }
    this.v += d * (1 - Math.exp(-dt / this.tau));
    return true;
  }
};
var Anel = class {
  constructor() {
    const r = 54, c = 2 * Math.PI * r;
    this.c = c;
    this.el = el("div", "anel");
    const s = svg("svg", { viewBox: "0 0 132 132", "aria-hidden": "true" });
    s.append(svg("circle", { class: "fundo-anel", cx: 66, cy: 66, r, fill: "none", "stroke-width": 11 }));
    this.arco = svg("circle", { class: "arco", cx: 66, cy: 66, r, fill: "none", "stroke-width": 11, "stroke-dasharray": c, "stroke-dashoffset": c });
    s.append(this.arco);
    this.centro = el("div", "centro");
    this.el.append(s, this.centro);
    this.mola = new Mola(0, 500);
  }
  definir(frac) {
    this.mola.definir(Math.max(0, Math.min(1, frac || 0)));
  }
  quadro(dt) {
    const mexe = this.mola.passo(dt);
    this.arco.setAttribute("stroke-dashoffset", String(this.c * (1 - this.mola.v)));
    return mexe;
  }
};
var Curva = class {
  constructor() {
    this.svg = svg("svg", { viewBox: "0 0 320 140", preserveAspectRatio: "none", role: "img", "aria-label": "Curva da apuração" });
    for (const y of [0, 35, 70, 105, 140]) this.svg.append(svg("line", { class: "eixo", x1: 0, x2: 320, y1: y, y2: y }));
    const defs = svg("defs");
    const g = svg("linearGradient", { id: "grad-area", x1: 0, y1: 0, x2: 0, y2: 1 });
    g.append(svg("stop", { offset: 0, "stop-color": "#8c6bff", "stop-opacity": 0.45 }), svg("stop", { offset: 1, "stop-color": "#8c6bff", "stop-opacity": 0 }));
    defs.append(g);
    this.svg.append(defs);
    this.area = svg("path", { class: "area", fill: "url(#grad-area)", d: "" });
    this.linhaSec = svg("path", { class: "linha", stroke: "#a996ff", d: "" });
    this.ls = [svg("path", { class: "linha", d: "" }), svg("path", { class: "linha", d: "" })];
    this.svg.append(this.area, this.linhaSec, this.ls[1], this.ls[0]);
    this.rot = [0, 1, 2].map((k) => el("span", `rot-y r${k}`, ""));
    this.rotT = [el("span", "rot-t ini", ""), el("span", "rot-t fim", "")];
    this.el = el("div", "graf", [this.svg, ...this.rot, ...this.rotT]);
    this.pontos = [];
    this.mostrar = [];
    this.t0 = new Mola(0, 600);
    this.t1 = new Mola(1, 600);
    this.lo = new Mola(0, 500);
    this.hi = new Mola(100, 500);
    this.sujo = true;
  }
  limpar() {
    this.pontos = [];
    this.sujo = true;
  }
  /** Pontos do servidor (substituem os que houver antes do último deles). */
  semear(pontos) {
    if (!pontos.length) return;
    const fim = pontos[pontos.length - 1].t;
    this.pontos = [...pontos, ...this.pontos.filter((p) => p.t > fim)];
    this.reposicionar(true);
  }
  /** Candidatos das linhas: [{ n, cor }] (os dois primeiros agora). */
  linhas(v) {
    const chave = v.map((x) => `${x.n}:${x.cor}`).join("|");
    if (chave === this.chave) return;
    this.chave = chave;
    this.mostrar = v;
    this.ls.forEach((l, i) => {
      l.style.stroke = v[i]?.cor || "transparent";
    });
    this.reposicionar(false);
  }
  /** t = segundos com fração (relógio do sistema, M-003), s = % seções, cand = [[n, pct], …] */
  ponto(t, s, cand) {
    if (t == null) return;
    const u = this.pontos[this.pontos.length - 1];
    if (u && t < u.t) return;
    const p = { t, s, c: new Map(cand.filter((x) => x[1] != null)) };
    if (u && t === u.t) this.pontos[this.pontos.length - 1] = p;
    else this.pontos.push(p);
    if (this.pontos.length > 1500) this.pontos = this.pontos.filter((_, i) => i % 2 === 0 || i === this.pontos.length - 1);
    this.reposicionar(this.pontos.length === 1);
  }
  reposicionar(ja) {
    if (!this.pontos.length) return;
    const p0 = this.pontos[0].t, p1 = this.pontos[this.pontos.length - 1].t;
    this.t0.definir(p0);
    this.t1.definir(Math.max(p1, p0 + 60));
    let lo = Infinity, hi = -Infinity;
    const ult = this.pontos[this.pontos.length - 1];
    const base2 = ult.s >= 1 ? this.pontos.filter((p) => p.s >= 1) : this.pontos;
    for (const p of base2) for (const m of this.mostrar) {
      const v = p.c.get(m.n);
      if (v != null) {
        lo = Math.min(lo, v);
        hi = Math.max(hi, v);
      }
    }
    if (lo === Infinity) {
      lo = 0;
      hi = 100;
    }
    const meio = (lo + hi) / 2, meia = Math.max(2, (hi - lo) / 2 * 1.1);
    lo = Math.max(0, meio - meia);
    hi = Math.min(100, meio + meia);
    this.lo.definir(lo);
    this.hi.definir(hi);
    if (ja) {
      this.t0.v = p0;
      this.t1.v = Math.max(p1, p0 + 60);
      this.lo.v = lo;
      this.hi.v = hi;
    }
    this.sujo = true;
  }
  quadro(dt) {
    const m = this.t0.passo(dt) | this.t1.passo(dt) | this.lo.passo(dt) | this.hi.passo(dt);
    if (!m && !this.sujo) return false;
    this.sujo = false;
    const t0 = this.t0.v, span = Math.max(1, this.t1.v - t0), lo = this.lo.v, alt = Math.max(0.5, this.hi.v - lo);
    const X = (t) => (t - t0) / span * 320;
    const Ys = (p) => 140 - Math.max(0, Math.min(100, p || 0)) / 100 * 140;
    const Yc = (v) => Math.max(-4, Math.min(144, 140 - (v - lo) / alt * 140));
    const ds = this.pontos.map((p, i) => `${i ? "L" : "M"}${X(p.t).toFixed(1)},${Ys(p.s).toFixed(1)}`).join("");
    this.linhaSec.setAttribute("d", ds);
    if (this.pontos.length) {
      const ult = this.pontos[this.pontos.length - 1];
      this.area.setAttribute("d", `${ds}L${X(ult.t).toFixed(1)},140L${X(this.pontos[0].t).toFixed(1)},140Z`);
    }
    this.ls.forEach((l, i) => {
      const n = this.mostrar[i]?.n;
      let d = "", novo = true;
      if (n != null) for (const p of this.pontos) {
        const v = p.c.get(n);
        if (v == null) {
          novo = true;
          continue;
        }
        d += `${novo ? "M" : "L"}${X(p.t).toFixed(1)},${Yc(v).toFixed(1)}`;
        novo = false;
      }
      l.setAttribute("d", d);
    });
    const f2 = (v) => `${v.toFixed(alt < 6 ? 1 : 0).replace(".", ",")}%`;
    [this.hi.v, lo + alt / 2, lo].forEach((v, k) => {
      const x = this.mostrar.length ? f2(v) : "";
      if (this.rot[k].textContent !== x) this.rot[k].textContent = x;
    });
    const hm3 = (t) => this.pontos.length ? new Date(Math.floor(t) * 1e3).toISOString().slice(11, span < 600 ? 19 : 16) : "";
    [hm3(t0), hm3(t0 + span)].forEach((x, k) => {
      if (this.rotT[k].textContent !== x) this.rotT[k].textContent = x;
    });
    return m;
  }
};

// src/feed.js
var VEL = 55;
var GAP = 28;
var MAX = 70;
var Feed = class {
  constructor(raiz, abrirDetalhe) {
    this.raiz = raiz;
    this.trilho = raiz.querySelector("#trilho");
    this.vazio = raiz.querySelector("#feed-vazio");
    this.anuncio = raiz.querySelector("#anuncio");
    this.abrirDetalhe = abrirDetalhe;
    this.itens = [];
    this.porId = /* @__PURE__ */ new Map();
    this.offset = 0;
    this.pausas = /* @__PURE__ */ new Set();
    this.reduzido = movimentoReduzido();
    this.atual = 0;
    this.trocaEm = 0;
    this.filaAnuncio = [];
    this.anuncioEm = 0;
    const pausar = (k, on) => {
      if (on) this.pausas.add(k);
      else this.pausas.delete(k);
      raiz.dataset.pausado = this.pausas.size ? "1" : "0";
    };
    raiz.addEventListener("pointerenter", (e) => {
      if (e.pointerType === "mouse") pausar("mouse", true);
    });
    raiz.addEventListener("pointerleave", (e) => {
      if (e.pointerType === "mouse") pausar("mouse", false);
    });
    raiz.addEventListener("focusin", () => pausar("foco", true));
    raiz.addEventListener("focusout", () => pausar("foco", false));
    raiz.addEventListener("pointerdown", (e) => {
      if (e.pointerType === "mouse") return;
      pausar("toque", true);
      clearTimeout(this.tToque);
      this.tToque = setTimeout(() => pausar("toque", false), 6e3);
    });
    window.matchMedia("(prefers-reduced-motion: reduce)").addEventListener?.("change", () => {
      this.reduzido = movimentoReduzido();
    });
  }
  // o trilho começa depois do botão do histórico
  get largura() {
    return this.trilho.clientWidth;
  }
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
    const b = el("button", "item" + (it.fixo ? " fixo" : ""));
    b.type = "button";
    const ic = el("span", "ic", it.ic || "•");
    ic.setAttribute("aria-hidden", "true");
    const txt = el("span", "txt", it.texto || "");
    const t = el("span", "t", it.t || "");
    b.append(ic, txt);
    if (it.hod) b.append(it.hod.el);
    b.append(t);
    const item = { id: it.id, el: b, txt, t, w: 0, x: 0, fixo: !!it.fixo, detalhe: it.detalhe, sair: false };
    b.addEventListener("click", () => this.abrirDetalhe(item.detalhe));
    b.style.opacity = this.reduzido ? "0" : "1";
    this.trilho.append(b);
    item.w = b.offsetWidth;
    this.#posicionar(item);
    this.itens.push(item);
    this.porId.set(it.id, item);
    this.vazio.style.opacity = "0";
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
    let p = this.offset + W;
    for (const i of this.itens) if (i.x < this.offset + W && i.x + i.w > p - GAP) p = Math.max(p, i.x + i.w + GAP);
    if (!this.itens.length) p = this.offset + 16;
    for (const i of this.itens) if (i.x >= p - 1) i.x += novo.w + GAP;
    novo.x = p;
    novo.el.style.transform = `translateX(${(novo.x - this.offset).toFixed(1)}px)`;
  }
  quadro(dt, agora) {
    if (this.filaAnuncio.length && agora - this.anuncioEm > 4e3) {
      this.anuncio.textContent = this.filaAnuncio.shift();
      this.anuncioEm = agora;
      if (this.filaAnuncio.length > 3) this.filaAnuncio.splice(0, this.filaAnuncio.length - 3);
    }
    if (!this.itens.length) return;
    if (this.reduzido) return this.#quadroReduzido(agora);
    if (!this.pausas.size && !document.hidden) this.offset += VEL * dt / 1e3;
    let fim = -Infinity;
    for (const i of this.itens) fim = Math.max(fim, i.x + i.w);
    const W = this.largura;
    for (let k = 0; k < this.itens.length; k++) {
      const i = this.itens[k];
      if (i.x + i.w < this.offset - 4) {
        if (i.sair) {
          i.el.remove();
          this.porId.delete(i.id);
          this.itens.splice(k--, 1);
          continue;
        }
        i.x = Math.max(fim + GAP, this.offset + W);
        fim = i.x + i.w;
      }
      i.el.style.transform = `translateX(${(i.x - this.offset).toFixed(1)}px)`;
    }
  }
  #quadroReduzido(agora) {
    for (let k = 0; k < this.itens.length; k++) {
      const i = this.itens[k];
      if (i.sair && k !== this.atual) {
        i.el.remove();
        this.porId.delete(i.id);
        this.itens.splice(k, 1);
        if (k < this.atual) this.atual--;
        k--;
      }
    }
    if (!this.itens.length) return;
    if (this.atual >= this.itens.length) this.atual = 0;
    if (agora < this.trocaEm && this.itens[this.atual]?.el.style.opacity === "1") return;
    if (!this.pausas.size) this.atual = (this.atual + 1) % this.itens.length;
    this.trocaEm = agora + 6e3;
    this.itens.forEach((i, k) => {
      i.el.style.transform = "translateX(16px)";
      i.el.style.opacity = k === this.atual ? "1" : "0";
      i.el.tabIndex = k === this.atual ? 0 : -1;
    });
  }
};
var Folha = class {
  constructor(raiz) {
    this.raiz = raiz;
    this.titulo = raiz.querySelector("#folha-titulo");
    this.corpo = raiz.querySelector("#folha-corpo");
    raiz.querySelector("#folha-fechar").addEventListener("click", () => this.fechar());
    document.addEventListener("keydown", (e) => {
      if (e.key === "Escape") this.fechar();
    });
    document.addEventListener("pointerdown", (e) => {
      if (this.raiz.classList.contains("aberta") && !this.raiz.contains(e.target) && !e.target.closest?.(".feed")) this.fechar();
    });
  }
  abrir(d) {
    if (!d) return;
    this.titulo.textContent = d.titulo;
    this.corpo.replaceChildren(...d.linhas.flatMap(([a, b]) => [el("dt", null, a), el("dd", null, b)]));
    this.raiz.classList.add("aberta");
    this.raiz.setAttribute("aria-hidden", "false");
  }
  fechar() {
    this.raiz.classList.remove("aberta");
    this.raiz.setAttribute("aria-hidden", "true");
  }
};

// src/relogio.js
function segundosF(t) {
  const s = segundos(t);
  if (s == null) return null;
  const m = /\d{2}:\d{2}:\d{2}\.(\d{1,3})/.exec(t);
  return m ? s + Number(m[1].padEnd(3, "0")) / 1e3 : s;
}
function ancora(r) {
  const noite0 = segundos(r?.noite0), sis0 = segundosF(r?.sistema0);
  if (noite0 == null || sis0 == null) return null;
  return { noite0, sis0, x: r.acelerar > 0 ? r.acelerar : 1 };
}
function tseNoSistema(a, t) {
  if (t == null) return null;
  return a ? a.sis0 + (t - a.noite0) / a.x : t;
}
function quando(m, campo = "sistema") {
  return m?.[campo] || m?.horario || m?.ultimo_bu || null;
}
function horaDe(s) {
  if (s == null || !Number.isFinite(s)) return "";
  return new Date(Math.floor(s) * 1e3).toISOString().slice(11, 19);
}
function diaDe(s) {
  if (s == null || !Number.isFinite(s)) return "";
  const d = new Date(Math.floor(s) * 1e3).toISOString();
  return `${d.slice(8, 10)}/${d.slice(5, 7)}`;
}
function distancia(s) {
  if (s == null || !Number.isFinite(s)) return "";
  const a = Math.abs(s), lado = s > 0 ? "antes" : "depois";
  if (a < 1) return "no mesmo instante";
  if (a < 90) return `${Math.round(a)} s ${lado}`;
  const m = Math.round(a / 60);
  if (m < 60) return `${m} min ${lado}`;
  return `${Math.floor(m / 60)} h ${String(m % 60).padStart(2, "0")} min ${lado}`;
}
function comparar(a, tUv, tTse) {
  const tse = tseNoSistema(a, tTse);
  if (tUv == null || tse == null) return null;
  const s = tse - tUv;
  return { tse, s, noite: s * (a?.x || 1), texto: distancia(s) };
}

// src/textos.js
var Nomes = class {
  constructor(n) {
    this.n = n || { ufs: {}, municipios: {}, eleicoes: [] };
    this.cand = /* @__PURE__ */ new Map();
    this.cargos = /* @__PURE__ */ new Map();
    for (const e of this.n.eleicoes || []) {
      for (const c of e.cargos || []) {
        this.cargos.set(c.cargo, { consulta: c.consulta, vagas: c.vagas, turno: e.turno, eleicao: e.codigo });
        for (const ci of c.circunscricoes || []) {
          const m = /* @__PURE__ */ new Map();
          [...ci.candidatos].sort((a, b) => a.n - b.n).forEach((k, i) => m.set(k.n, { ...k, idx: i }));
          this.cand.set(`${ci.abr}-c${String(c.cargo).padStart(4, "0")}`, m);
        }
      }
    }
  }
  uf(u) {
    return u === "zz" ? "Exterior" : titulo(this.n.ufs?.[u] || u.toUpperCase());
  }
  mun(cod) {
    return titulo(this.n.municipios?.[cod] || cod);
  }
  /** "Brasil", "São Paulo", "Manaus (AM)" */
  local(cod) {
    const a = abr(cod);
    if (a.tipo === "br") return "Brasil";
    if (a.tipo === "uf") return this.uf(a.uf);
    return `${this.mun(cod)} (${a.uf.toUpperCase()})`;
  }
  cargo(c) {
    return NOME_CARGO[c] || (this.cargos.get(c)?.consulta || c >= 25 ? "Consulta" : `Cargo ${c}`);
  }
  /** "Governador em São Paulo", "Presidente", "Prefeito de Manaus (AM)" */
  disputa(cod, c) {
    const a = abr(cod);
    const nome = this.cargo(c);
    if (a.tipo === "br") return nome;
    if (a.tipo === "uf") return `${nome} em ${this.uf(a.uf)}`;
    return `${nome} de ${this.local(cod)}`;
  }
  candidato(circ, c, n) {
    const k = this.cand.get(`${circ}-c${String(c).padStart(4, "0")}`)?.get(n);
    return k ? { nome: k.nome_urna || k.nome || String(n), partido: k.partido || "", vices: k.vices || [], idx: k.idx } : { nome: String(n), partido: "", vices: [], idx: n % 10 };
  }
  nomeCand(circ, c, n) {
    const k = this.candidato(circ, c, n);
    return k.partido ? `${k.nome} (${k.partido})` : k.nome;
  }
};
function lugar(N, m) {
  if (m.municipio == null) return "";
  const nome = m.municipio_nome ? titulo(m.municipio_nome) : N.mun(`${m.uf}${String(m.municipio).padStart(5, "0")}`);
  return `${nome} (${String(m.uf).toUpperCase()})`;
}
function linhasSecao(N, m) {
  return [["Seção", String(m.secao)], ["Zona", String(m.zona)], ["Município", lugar(N, m)], ["Urna Velox (relógio do sistema)", hora(quando(m))], ["Horário do BU no TSE", hora(m.horario)], ["BUs aplicados (seq)", mil(m.seq)]];
}
var CATEGORIAS = [["decisao", "Decisões"], ["virada", "Viradas"], ["andamento", "Andamento"]];
function itemMarco(N, m) {
  const it = corpoMarco(N, m);
  if (it) Object.assign(it, { seq: m.seq, cat: m.classe === "secao_decisiva" ? "decisao" : m.classe === "virada" ? "virada" : "andamento" });
  return it;
}
function corpoMarco(N, m) {
  const id = `m:${m.classe}:${m.abrangencia}:${m.cargo ?? ""}:${m.seq}:${m.secao ?? ""}:${m.valor ?? ""}`;
  const t = hora(quando(m));
  switch (m.classe) {
    case "primeira_secao": {
      const onde = m.abrangencia === "br" ? "do Brasil" : `em ${N.local(m.abrangencia)}`;
      return {
        id,
        ic: "1",
        t,
        texto: `Primeira seção totalizada ${onde}: seção ${m.secao}, zona ${m.zona}, ${lugar(N, m)}`,
        anunciar: `Primeira seção totalizada ${onde}, às ${t}.`,
        detalhe: { titulo: `Primeira seção totalizada ${onde}`, linhas: linhasSecao(N, m) }
      };
    }
    case "secao_decisiva": {
      const d = N.disputa(m.abrangencia, m.cargo);
      const o = m.estado === "matematicamente_2t" ? "o 2º turno" : "o resultado";
      return {
        id,
        ic: "◆",
        t,
        destaque: true,
        texto: `A seção ${m.secao} (zona ${m.zona}, ${lugar(N, m)}) garantiu ${o} — ${d}`,
        anunciar: `${d}: seção ${m.secao} de ${lugar(N, m)} garantiu ${o}.`,
        detalhe: { titulo: `Seção que garantiu ${o} — ${d}`, linhas: [
          ...linhasSecao(N, m),
          ["BUs no mesmo lote", String(m.lote_bus ?? 1)],
          ["Observação", (m.lote_bus ?? 1) > 1 ? "o motor reavalia por lote: esta é a última seção do lote dentro da circunscrição" : "lote com um BU só"]
        ] }
      };
    }
    case "bu_marco":
      return {
        id,
        ic: "#",
        t,
        texto: `BU nº ${mil(m.valor)} da noite: seção ${m.secao}, zona ${m.zona}, ${lugar(N, m)}`,
        detalhe: { titulo: `BU nº ${mil(m.valor)} da noite`, linhas: linhasSecao(N, m) }
      };
    case "ultima_secao":
      return {
        id,
        ic: "■",
        t,
        texto: `Última seção da noite: seção ${m.secao}, zona ${m.zona}, ${lugar(N, m)}`,
        detalhe: { titulo: "Última seção aplicada", linhas: linhasSecao(N, m) }
      };
    case "pct_secoes": {
      const onde = N.local(m.abrangencia);
      return {
        id,
        ic: "%",
        t,
        texto: `${onde}: ${m.valor}% das seções apuradas`,
        detalhe: { titulo: `${onde}: ${m.valor}% das seções apuradas`, linhas: [["Horário (Urna Velox)", t], ["Seções", m.secoes != null ? `${mil(m.secoes)} de ${mil(m.total)}` : "—"], ["BUs aplicados (seq)", mil(m.seq)]] }
      };
    }
    case "virada": {
      const d = N.disputa(m.abrangencia, m.cargo);
      const novo = N.nomeCand(m.abrangencia, m.cargo, m.candidato), ant = N.nomeCand(m.abrangencia, m.cargo, m.anterior);
      return {
        id,
        ic: "⇅",
        t,
        texto: `Virada — ${d}: ${novo} passa ${ant} (${m.valor}ª da noite)`,
        detalhe: { titulo: `Virada de liderança — ${d}`, linhas: [["Novo líder", novo], ["Antes", ant], ["Viradas nesta disputa", String(m.valor)], ["Horário (Urna Velox)", t], ["BUs aplicados (seq)", mil(m.seq)]] }
      };
    }
    default:
      return null;
  }
}
function itemEvento(N, e) {
  const t = hora(quando(e));
  const id = `e:${e.seq}:${e.abrangencia}:${e.cargo}:${e.candidato}:${e.estado}`;
  if (e.classe === "apuracao") {
    if (e.estado !== "apuracao_encerrada") return null;
    return {
      id,
      ic: "✓",
      t,
      seq: e.seq,
      cat: "andamento",
      texto: "Apuração encerrada: todas as seções totalizadas",
      anunciar: "Apuração encerrada.",
      detalhe: { titulo: "Apuração encerrada", linhas: [["Horário (Urna Velox)", t], ["BUs aplicados", mil(e.seq)]] }
    };
  }
  if (String(e.estado).startsWith("revogado:") || e.candidato == null) return null;
  const d = N.disputa(e.abrangencia, e.cargo);
  const quem = N.nomeCand(e.abrangencia, e.cargo, e.candidato);
  const o = e.estado === "eleito" ? "matematicamente eleito" : "matematicamente no 2º turno";
  return {
    id,
    ic: "★",
    t,
    seq: e.seq,
    cat: "decisao",
    destaque: true,
    texto: `${quem}: ${o} — ${d}`,
    anunciar: `${d}: ${quem} ${o}.`,
    detalhe: { titulo: `${quem} — ${o}`, linhas: [["Disputa", d], ["Urna Velox (relógio do sistema)", t], ["Horário do BU que decidiu (TSE)", hora(e.horario)], ["BUs aplicados (seq)", mil(e.seq)]] }
  };
}
function primeiros(N, eventos) {
  const vistos = /* @__PURE__ */ new Map();
  const out = [];
  const ord = [...eventos].filter((e) => e.classe === "matematicamente" && e.estado === "eleito").sort((a, b) => a.seq - b.seq);
  for (const e of ord) {
    const a = abr(e.abrangencia);
    const nome = NOME_CARGO_PLURAL[e.cargo];
    if (!nome) continue;
    const quem = N.nomeCand(e.abrangencia, e.cargo, e.candidato);
    const t = hora(quando(e));
    const pais = `pais:${e.cargo}`;
    if (!vistos.has(pais) && e.cargo !== 1) {
      vistos.set(pais, 1);
      out.push({
        id: `p:${pais}`,
        ic: "①",
        t,
        seq: e.seq,
        cat: "decisao",
        destaque: true,
        texto: `Primeiro ${nome} matematicamente eleito do país: ${quem}, ${N.local(e.abrangencia)}`,
        detalhe: { titulo: `Primeiro ${nome} matematicamente eleito do país`, linhas: [["Eleito", quem], ["Onde", N.local(e.abrangencia)], ["Horário (Urna Velox)", t], ["BUs aplicados (seq)", mil(e.seq)]] }
      });
    }
    if (a.tipo === "mun") {
      const k = `uf:${e.cargo}:${a.uf}`;
      if (!vistos.has(k)) {
        vistos.set(k, 1);
        out.push({
          id: `p:${k}`,
          ic: "①",
          t,
          seq: e.seq,
          cat: "decisao",
          texto: `Primeiro ${nome} eleito em ${N.uf(a.uf)}: ${quem}, ${N.local(e.abrangencia)}`,
          detalhe: { titulo: `Primeiro ${nome} matematicamente eleito em ${N.uf(a.uf)}`, linhas: [["Eleito", quem], ["Onde", N.local(e.abrangencia)], ["Horário (Urna Velox)", t]] }
        });
      }
    }
  }
  return out;
}
function hhmm(t) {
  const m = /(\d{2}):(\d{2}):\d{2}/.exec(t || "");
  return m ? `${m[1]}:${m[2]}` : "";
}
function itemConfirmou(N, circ, cargo, sit, emUv, geracao, cmp, anc) {
  const d = N.disputa(circ, cargo);
  const ante = cmp?.texto || "";
  const sim = anc && (anc.x !== 1 || Math.abs(cmp.tse - (segundosHora(geracao) ?? cmp.tse)) > 3600);
  const o = sit === "2t" ? "o 2º turno" : "o matematicamente";
  const tseTxt = sim ? `${hhmm(geracao)} do TSE (≙ ${horaDe(cmp.tse).slice(0, 5)} aqui)` : hhmm(geracao);
  const linhas = [["Urna Velox (relógio do sistema)", hora(emUv)], ["TSE (arquivo gerado às, relógio do TSE)", hora(geracao)]];
  if (sim) linhas.push(["TSE no relógio do sistema", `${horaDe(cmp.tse)} — a noite do TSE ${anc.x !== 1 ? `a ${String(anc.x).replace(".", ",")}× ` : ""}a partir do início deste simulado`]);
  linhas.push(
    ["Antecedência do Urna Velox", `${ante}${sim && anc.x !== 1 ? ` (${distancia(cmp.noite)} no tempo da noite)` : ""}`],
    ["Como se compara", "o arquivo de resultado do TSE desta disputa (buscado a cada 30–60 s) passou a mostrar a mesma situação"]
  );
  return {
    id: `c:tse-conf:${circ}:${cargo}`,
    ic: "⇄",
    t: hora(emUv),
    cat: "decisao",
    destaque: true,
    texto: `TSE confirmou ${o} às ${tseTxt} — Urna Velox às ${hhmm(emUv)}, ${ante} · ${d}`,
    anunciar: `${d}: o TSE confirmou ${o} às ${tseTxt}; o Urna Velox às ${hhmm(emUv)}, ${ante}.`,
    detalhe: { titulo: `TSE confirmou ${o} — ${d}`, linhas }
  };
}
function segundosHora(s) {
  const m = /(\d{2})\/(\d{2})\/(\d{4}) (\d{2}):(\d{2}):(\d{2})/.exec(s || "");
  return m ? Date.UTC(+m[3], +m[2] - 1, +m[1], +m[4], +m[5], +m[6]) / 1e3 : null;
}
function itemOrdemDiferente(N, circ, cargo, geracao, quem) {
  const d = N.disputa(circ, cargo);
  return {
    id: `c:tse-ordem:${circ}:${cargo}:${geracao}`,
    ic: "≠",
    t: hora(geracao),
    cat: "andamento",
    texto: `Ordem diferente entre as fontes — ${d}: ${quem}`,
    detalhe: { titulo: `Ordem diferente entre Urna Velox e TSE — ${d}`, linhas: [
      ["Desde o arquivo do TSE das", hora(geracao)],
      ["Candidatos", quem],
      ["Por quê", "o Urna Velox costuma estar à frente (BUs aplicados antes de o TSE divulgar); com mais seções nas duas fontes, a ordem tende a coincidir"]
    ] }
  };
}

// src/historico.js
var MAX2 = 2e4;
var Historico = class {
  constructor(raiz, botao) {
    this.raiz = raiz;
    this.botao = botao;
    this.lista = raiz.querySelector("#hist-lista");
    this.tempo = raiz.querySelector("#hist-tempo");
    this.hora = raiz.querySelector("#hist-hora");
    this.ini = raiz.querySelector("#hist-ini");
    this.fim = raiz.querySelector("#hist-fim");
    this.vazio = raiz.querySelector("#hist-vazio");
    this.texto = raiz.querySelector("#hist-busca");
    this.itens = [];
    this.porId = /* @__PURE__ */ new Map();
    this.vis = [];
    this.cats = new Set(CATEGORIAS.map((c) => c[0]));
    this.busca = "";
    this.atual = -1;
    this.vivo = true;
    this.n = 0;
    this.chips = /* @__PURE__ */ new Map();
    const fil = raiz.querySelector("#hist-filtros");
    for (const [k, nome] of CATEGORIAS) {
      const b = el("button", "chip", [el("span", "chip-nome", nome), el("span", "chip-n", "0")]);
      b.type = "button";
      b.dataset.cat = k;
      b.setAttribute("aria-pressed", "true");
      b.addEventListener("click", () => {
        if (this.cats.has(k)) this.cats.delete(k);
        else this.cats.add(k);
        b.setAttribute("aria-pressed", String(this.cats.has(k)));
        this.#filtrar();
      });
      this.chips.set(k, b);
      fil.insertBefore(b, this.texto);
    }
    this.texto.addEventListener("input", () => {
      this.busca = this.texto.value.trim().toLowerCase();
      this.#filtrar();
    });
    this.tempo.addEventListener("input", () => {
      this.vivo = +this.tempo.value >= this.vis.length - 1;
      this.#ir(+this.tempo.value, true);
    });
    this.lista.addEventListener("scroll", () => {
      if (this.rolandoEu) {
        this.rolandoEu = false;
        return;
      }
      const meio = this.lista.scrollTop + this.lista.clientHeight / 2;
      let a = 0, b = this.vis.length - 1;
      while (a < b) {
        const m = a + b + 1 >> 1;
        if (this.vis[m].row.offsetTop <= meio) a = m;
        else b = m - 1;
      }
      this.vivo = this.lista.scrollTop + this.lista.clientHeight >= this.lista.scrollHeight - 4;
      this.#ir(this.vivo ? this.vis.length - 1 : a, false);
    }, { passive: true });
    botao.addEventListener("click", () => this.aberto ? this.fechar() : this.abrir());
    raiz.querySelector("#hist-fechar").addEventListener("click", () => this.fechar());
    document.addEventListener("keydown", (e) => {
      if (e.key === "Escape" && this.aberto) this.fechar();
    });
  }
  get aberto() {
    return this.raiz.classList.contains("aberta");
  }
  abrir() {
    this.raiz.classList.add("aberta");
    this.raiz.setAttribute("aria-hidden", "false");
    this.botao.setAttribute("aria-expanded", "true");
    this.vivo = true;
    this.#ir(this.vis.length - 1, true);
    this.tempo.focus({ preventScroll: true });
  }
  fechar() {
    if (!this.aberto) return;
    this.raiz.classList.remove("aberta");
    this.raiz.setAttribute("aria-hidden", "true");
    this.botao.setAttribute("aria-expanded", "false");
  }
  /** Guarda um item do feed (os de `cat` definida). Repetido pelo id: atualiza o texto. */
  por(it) {
    if (!it.cat) return;
    const ja = this.porId.get(it.id);
    if (ja) {
      if (it.texto != null && it.texto !== ja.texto) {
        ja.texto = it.texto;
        if (ja.row) ja.row.querySelector(".h-txt").textContent = it.texto;
      }
      ja.detalhe = it.detalhe;
      return;
    }
    const r = { id: it.id, cat: it.cat, seq: it.seq ?? Infinity, n: this.n++, ic: it.ic || "•", texto: it.texto || "", t: it.t || "", detalhe: it.detalhe, row: null };
    r.chave = `${r.texto} ${r.t}`.toLowerCase();
    let k = this.itens.length;
    while (k > 0 && this.itens[k - 1].seq > r.seq) k--;
    this.itens.splice(k, 0, r);
    this.porId.set(r.id, r);
    this.conta ||= /* @__PURE__ */ new Map();
    this.conta.set(r.cat, (this.conta.get(r.cat) || 0) + 1);
    if (this.itens.length > MAX2) {
      const v = this.itens.shift();
      this.porId.delete(v.id);
      v.row?.remove();
      this.contaSuja = true;
    }
    if (!this.aberto) {
      this.sujo = true;
      return;
    }
    this.#linha(r, this.itens[k + 1]?.row || null);
    if (!this.#passa(r)) return;
    let p = this.vis.length;
    while (p > 0 && this.vis[p - 1].seq > r.seq) p--;
    this.vis.splice(p, 0, r);
    if (this.vivo) this.#ir(this.vis.length - 1, true);
    else {
      if (p <= this.atual) this.atual++;
      this.#rotulos();
    }
  }
  #passa(r) {
    return this.cats.has(r.cat) && (!this.busca || r.chave.includes(this.busca));
  }
  #contar() {
    const c = /* @__PURE__ */ new Map();
    for (const r of this.itens) c.set(r.cat, (c.get(r.cat) || 0) + 1);
    this.conta = c;
    this.contaSuja = false;
  }
  #mostrarContas() {
    if (this.contaSuja) this.#contar();
    for (const [k, b] of this.chips) {
      const x = mil(this.conta?.get(k) || 0);
      const s = b.querySelector(".chip-n");
      if (s.textContent !== x) s.textContent = x;
    }
  }
  #linha(r, antes) {
    const cab = el("button", "h-cab", [el("span", "ic", r.ic), el("span", "h-txt", r.texto), el("span", "h-t", r.t)]);
    cab.type = "button";
    cab.setAttribute("aria-expanded", "false");
    const row = el("div", `h-item c-${r.cat}`, [cab]);
    row.setAttribute("role", "listitem");
    cab.addEventListener("click", () => {
      const aberto = cab.getAttribute("aria-expanded") === "true";
      row.querySelector(".h-det")?.remove();
      cab.setAttribute("aria-expanded", String(!aberto));
      if (!aberto && r.detalhe) row.append(el("dl", "h-det", r.detalhe.linhas.flatMap(([a, b]) => [el("dt", null, a), el("dd", null, b)])));
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
      for (const r of this.itens) {
        if (!r.row) this.#linha(r, null);
        frag.append(r.row);
      }
      this.lista.append(frag);
    }
    const antes = this.vis[this.atual];
    this.vis = [];
    for (const r of this.itens) {
      const p = this.#passa(r);
      r.row.hidden = !p;
      if (p) this.vis.push(r);
    }
    let i = antes ? this.vis.indexOf(antes) : -1;
    if (antes && i < 0) {
      i = this.vis.findIndex((r) => r.seq > antes.seq || r.seq === antes.seq && r.n > antes.n);
      if (i < 0) i = this.vis.length - 1;
    }
    this.#ir(this.vivo || i < 0 ? this.vis.length - 1 : i, true);
  }
  /** Vai ao item i de `vis`: marca, rola a lista até ele (se `rolar`) e atualiza o slider e os rótulos. */
  #ir(i, rolar) {
    if (this.sujo && this.aberto) return this.#filtrar();
    const ant = this.vis[this.atual];
    this.atual = Math.max(-1, Math.min(i, this.vis.length - 1));
    const r = this.vis[this.atual];
    if (ant !== r) {
      ant?.row?.classList.remove("atual");
      r?.row?.classList.add("atual");
    }
    if (rolar && r?.row && this.aberto) {
      const alvo = Math.max(0, r.row.offsetTop - (this.lista.clientHeight - r.row.offsetHeight) / 2);
      if (Math.abs(this.lista.scrollTop - alvo) > 1) {
        this.rolandoEu = true;
        this.lista.scrollTop = alvo;
      }
    }
    this.#rotulos();
  }
  #rotulos() {
    const n = this.vis.length, r = this.vis[this.atual];
    this.#mostrarContas();
    this.tempo.max = String(Math.max(0, n - 1));
    this.tempo.value = String(Math.max(0, this.atual));
    this.tempo.disabled = n < 2;
    this.tempo.style.setProperty("--p", `${n > 1 ? 100 * this.atual / (n - 1) : 100}%`);
    this.tempo.setAttribute("aria-valuetext", r ? `${r.t}: ${r.texto}` : "nenhum item");
    const h = r ? `${r.t || "—"} · ${mil(this.atual + 1)} de ${mil(n)}` : "nada ainda";
    if (this.hora.textContent !== h) this.hora.textContent = h;
    this.ini.textContent = this.vis[0]?.t || "";
    this.fim.textContent = n ? this.vivo ? "agora" : this.vis[n - 1].t : "";
    this.vazio.hidden = n > 0;
  }
};

// src/cores.js
var PARTIDOS = {
  PL: "#015AAA",
  PT: "#E20E28",
  UNIAO: "#2FBEF2",
  PP: "#203F71",
  PSD: "#FFA500",
  REPUBLICANOS: "#0070C5",
  MDB: "#30914D",
  PSB: "#FFCC00",
  PODE: "#2DA933",
  PSDB: "#0080FF",
  PDT: "#C21E56",
  PSOL: "#E2202A",
  PCDOB: "#820000",
  AVANTE: "#088F8F",
  NOVO: "#F3701B",
  SOLIDARIEDADE: "#FF9C2B",
  PRD: "#008000",
  PV: "#006600",
  CIDADANIA: "#EC008C",
  REDE: "#379E8D",
  MISSAO: "#FDBF35",
  PRTB: "#2CB53F",
  AGIR: "#025B97",
  MOBILIZA: "#EC2300",
  DC: "#015F9D",
  PCO: "#BB0000",
  PCB: "#FF0000",
  PSTU: "#C4122D",
  UP: "#000000",
  DEMOCRATA: "#003366",
  PMB: "#333C61"
};
var FEDERACOES = [
  { nome: "Brasil da Esperança", cor: "#E20E28", partidos: ["PT", "PCDOB", "PV"] },
  { nome: "PSDB Cidadania", cor: "#0080FF", partidos: ["PSDB", "CIDADANIA"] },
  { nome: "PSOL REDE", cor: "#E2202A", partidos: ["PSOL", "REDE"] },
  { nome: "Renovação Solidária", cor: "#FF9C2B", partidos: ["SOLIDARIEDADE", "PRD"] },
  { nome: "União Progressista", cor: "#2FBEF2", partidos: ["UNIAO", "PP"] }
];
var RESERVA = [255, 0, 80, 165, 295, 45, 215, 105, 135, 330, 60, 230, 30, 120];
function normalizarSigla(s) {
  return String(s || "").normalize("NFD").replace(/[̀-ͯ]/g, "").replace(/[\s*]/g, "").toUpperCase();
}
function oklch(hex) {
  const n = parseInt(hex.slice(1), 16);
  const lin = (v) => {
    v /= 255;
    return v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
  };
  const r = lin(n >> 16 & 255), g = lin(n >> 8 & 255), b = lin(n & 255);
  const l = Math.cbrt(0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b);
  const m = Math.cbrt(0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b);
  const s = Math.cbrt(0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b);
  const L = 0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s;
  const A = 1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s;
  const B = 0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s;
  const C = Math.hypot(A, B);
  return [L, C, (Math.atan2(B, A) * 180 / Math.PI + 360) % 360];
}
function base(sigla, i) {
  const sg = normalizarSigla(sigla);
  const hex = PARTIDOS[sg] ?? FEDERACOES.find((f2) => f2.partidos.includes(sg))?.cor;
  if (!hex) return { h: RESERVA[i % RESERVA.length], cromatica: true, chave: `?${i}` };
  const [, C, h] = oklch(hex);
  return C < 0.04 ? { h: 265, cromatica: false, chave: sg } : { h, cromatica: true, chave: sg };
}
var DL = [0, 0.07, -0.07, 0.14, -0.14];
var DH = [0, 12, -12, 24, -24];
var OPCOES = DL.flatMap((dl) => DH.map((dh) => [dl, dh])).sort((a, b) => Math.abs(a[0]) / 0.07 + Math.abs(a[1]) / 12 - (Math.abs(b[0]) / 0.07 + Math.abs(b[1]) / 12));
var ALVO = 0.14;
var f = (x, n = 3) => +x.toFixed(n);
var lab = ([L, C, h]) => [L, C * Math.cos(h * Math.PI / 180), C * Math.sin(h * Math.PI / 180)];
var de = (a, b) => Math.hypot(a[0] - b[0], a[1] - b[1], a[2] - b[2]);
function tons(b, [dL, dh]) {
  const h = (b.h + dh + 360) % 360;
  const escuro = b.cromatica ? [Math.min(0.88, Math.max(0.62, 0.76 + dL)), 0.14, h] : [Math.min(0.9, Math.max(0.62, 0.8 + dL)), 0.015, h];
  const claro = b.cromatica ? [Math.min(0.72, Math.max(0.42, 0.6 - dL)), 0.15, h] : [Math.min(0.62, Math.max(0.3, 0.45 - dL)), 0.02, h];
  return { escuro, claro };
}
var css = ([L, C, h]) => `oklch(${f(L)} ${f(C)} ${f(h, 1)})`;
function coresDaDisputa(cands) {
  const bases = cands.map((k, i) => base(k.partido, i));
  const tabela = bases.map((b) => OPCOES.map((o) => {
    const t = tons(b, o);
    return { t, e: lab(t.escuro), c: lab(t.claro) };
  }));
  const esc = cands.map(() => 0);
  const dist = (i, oi, j) => {
    const a = tabela[i][oi], b = tabela[j][esc[j]];
    return Math.min(de(a.e, b.e), de(a.c, b.c));
  };
  const menor = (i, oi) => {
    let m = Infinity;
    for (let j = 0; j < cands.length; j++) if (j !== i) m = Math.min(m, dist(i, oi, j));
    return m;
  };
  for (let rodada = 0; rodada < 8; rodada++) {
    let mudou = false;
    for (let i = 0; i < cands.length; i++) {
      let melhor = esc[i], vm = Math.min(ALVO, menor(i, esc[i]));
      for (let oi = 0; oi < OPCOES.length; oi++) {
        const v = Math.min(ALVO, menor(i, oi));
        if (v > vm + 1e-4 || Math.abs(v - vm) <= 1e-4 && oi < melhor) {
          melhor = oi;
          vm = v;
        }
      }
      if (melhor !== esc[i]) {
        esc[i] = melhor;
        mudou = true;
      }
    }
    if (!mudou) break;
  }
  const saida = /* @__PURE__ */ new Map();
  cands.forEach((k, i) => {
    const t = tabela[i][esc[i]].t;
    saida.set(k.n, `light-dark(${css(t.claro)}, ${css(t.escuro)})`);
  });
  return saida;
}

// src/replay.js
var VELOCIDADES = [1, 2, 5, 10, 20, 30, 60, 120, 180, 240, 300];
var hms = (t) => new Date(t * 1e3).toISOString().slice(11, 19);
var hm = (t) => new Date(t * 1e3).toISOString().slice(11, 16);
var vezes = (x) => `${String(x).replace(".", ",")}×`;
async function postar(caminho, corpo) {
  const r = await fetch(caminho, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(corpo) });
  const j = await r.json().catch(() => ({}));
  if (!r.ok) throw new Error(j.erro || `HTTP ${r.status}`);
  return j;
}
var Player = class {
  /** @param {object} info primeira resposta de GET /api/v1/replay */
  constructor(info) {
    this.info = info;
    this.sessao = info.sessao;
    this.medidoEm = performance.now();
    this.arrastando = false;
    this.ultima = info.acelerar > 0 ? info.acelerar : 10;
    this.voltando = null;
    this.play = el("button", "icone-btn pl-play", "❚❚");
    this.play.type = "button";
    this.hora = el("output", "pl-hora", "—");
    this.tempo = el("input", "pl-tempo");
    Object.assign(this.tempo, { type: "range", step: "60", min: String(info.inicio), max: String(info.fim) });
    this.tempo.setAttribute("aria-label", "Momento da noite");
    this.dica = el("span", "pl-dica", "");
    this.marcas = el("div", "pl-marcas");
    this.alvoMarca = el("i", "pl-alvo");
    const linha = el("div", "pl-linha", [this.marcas, this.alvoMarca, this.tempo, this.dica]);
    this.ini = el("span", "pl-ini", hm(info.inicio));
    this.fim = el("span", "pl-fim", hm(info.fim));
    this.vel = el("select", "pl-vel");
    this.vel.setAttribute("aria-label", "Velocidade");
    for (const v of VELOCIDADES) {
      const o = el("option", null, vezes(v));
      o.value = String(v);
      this.vel.append(o);
    }
    this.estado = el("span", "pl-estado", "");
    this.estado.setAttribute("aria-live", "polite");
    this.el = el("section", "player vidro", [this.play, this.hora, this.ini, linha, this.fim, this.vel, this.estado]);
    this.el.setAttribute("aria-label", "Controle da simulação");
    this.veu = el("div", "pl-veu", [el("div", "pl-veu-txt", "")]);
    this.veu.hidden = true;
    const span = info.fim - info.inicio;
    for (let h = Math.ceil(info.inicio / 3600) * 3600; h < info.fim; h += 3600) {
      const m = el("span", null, hm(h).slice(0, 2));
      m.style.left = `${(h - info.inicio) / span * 100}%`;
      this.marcas.append(m);
    }
    this.play.addEventListener("click", () => this.velocidade(this.info.acelerar === 0 ? this.ultima : 0));
    this.vel.addEventListener("change", () => this.velocidade(+this.vel.value));
    this.tempo.addEventListener("input", () => {
      this.arrastando = true;
      this.dica.textContent = `ir para ${hm(+this.tempo.value)}`;
      this.dica.style.left = `${this.pos(+this.tempo.value) * 100}%`;
      this.el.classList.add("com-dica");
    });
    this.tempo.addEventListener("change", () => this.ir(+this.tempo.value));
    document.body.append(this.veu, this.el);
    this.aplicar(info);
    setInterval(() => this.consultar(), 1e3);
    setInterval(() => this.desenhar(), 250);
  }
  pos(t) {
    return Math.min(1, Math.max(0, (t - this.info.inicio) / Math.max(1, this.info.fim - this.info.inicio)));
  }
  /** Relógio da simulação agora (interpolado entre as consultas pela aceleração real). */
  agora() {
    const i = this.info;
    if (i.t == null) return i.inicio;
    const dt = (performance.now() - this.medidoEm) / 1e3;
    return Math.min(i.busca ? i.busca.alvo : i.fim + 3600, i.t + dt * (i.real || 0));
  }
  aplicar(info) {
    this.info = info;
    this.medidoEm = performance.now();
    if (info.sessao && this.sessao && info.sessao !== this.sessao) {
      this.recarregar();
      return;
    }
    if (info.acelerar > 0) this.ultima = info.acelerar;
    const pausado = info.acelerar === 0;
    this.play.textContent = pausado ? "▶" : "❚❚";
    this.play.setAttribute("aria-label", pausado ? "Continuar a simulação" : "Pausar a simulação");
    this.el.classList.toggle("pausado", pausado && !info.busca);
    if (document.activeElement !== this.vel) this.vel.value = String(VELOCIDADES.includes(info.acelerar) ? info.acelerar : this.ultima);
    this.el.classList.toggle("buscando", !!info.busca);
    this.alvoMarca.style.left = info.busca ? `${this.pos(info.busca.alvo) * 100}%` : "";
    this.desenhar();
  }
  desenhar() {
    const t = this.agora();
    const txt = hms(t);
    if (this.hora.textContent !== txt) this.hora.textContent = txt;
    if (!this.arrastando) {
      this.tempo.value = String(t);
      this.tempo.style.setProperty("--p", `${this.pos(t) * 100}%`);
    }
    const b = this.info.busca;
    let e = "";
    if (this.voltando) e = `indo para ${hm(this.voltando)}: retotalizando…`;
    else if (b) e = `avançando até ${hm(b.alvo)} · ${Math.round(100 * Math.min(1, Math.max(0, (t - b.de) / Math.max(1, b.alvo - b.de))))}%`;
    else if (this.info.acelerar === 0) e = "pausado";
    if (this.estado.textContent !== e) this.estado.textContent = e;
  }
  async consultar() {
    try {
      const r = await fetch("/api/v1/replay", { cache: "no-store" });
      if (r.ok) this.aplicar(await r.json());
    } catch {
    }
  }
  /** Bloco `replay` da mensagem `estado` (chega pelo WebSocket): só a sessão interessa aqui. */
  estadoWs(rp) {
    if (rp?.sessao && rp.sessao !== this.sessao) this.recarregar();
  }
  recarregar() {
    if (this.recarregando) return;
    this.recarregando = true;
    location.reload();
  }
  async velocidade(x) {
    try {
      this.aplicar(await postar("/api/v1/replay/velocidade", { acelerar: x }));
    } catch (e) {
      this.estado.textContent = `não mudou: ${e.message}`;
    }
  }
  async ir(t) {
    this.arrastando = false;
    this.el.classList.remove("com-dica");
    try {
      const j = await postar("/api/v1/replay/posicao", { t });
      if (j.reinicio) {
        this.voltando = t;
        this.veu.firstChild.textContent = `Indo para ${hm(t)}: retotalizando os BUs já baixados até lá…`;
        this.veu.hidden = false;
        this.desenhar();
      } else this.aplicar(j);
    } catch (e) {
      this.estado.textContent = `não foi: ${e.message}`;
    }
  }
};
async function iniciarPlayer() {
  for (let i = 0; i < 10; i++) {
    try {
      const r = await fetch("/api/v1/replay", { cache: "no-store" });
      if (!r.ok) return null;
      const info = await r.json();
      if (info.inicio != null && info.fim != null) {
        document.documentElement.classList.add("com-replay");
        return new Player(info);
      }
    } catch {
    }
    await new Promise((ok) => setTimeout(ok, 300));
  }
  return null;
}

// src/comparacao.js
var MODOS = [["uv", "Urna Velox"], ["ambos", "Ambos"], ["tse", "TSE"]];
var VALIDO = new Set(MODOS.map((m) => m[0]));
var CHAVE = "uv-comparar";
function modoInicial() {
  const q = new URLSearchParams(location.search).get("comparar");
  if (VALIDO.has(q)) return q;
  try {
    const v = localStorage.getItem(CHAVE);
    if (VALIDO.has(v)) return v;
  } catch {
  }
  return "uv";
}
function gravarModo(m) {
  try {
    localStorage.setItem(CHAVE, m);
  } catch {
  }
  try {
    const u = new URL(location.href);
    if (m === "uv") u.searchParams.delete("comparar");
    else u.searchParams.set("comparar", m);
    history.replaceState(history.state, "", u.pathname + u.search + u.hash);
  } catch {
  }
}
var Controle = class {
  constructor(modo, aoMudar) {
    this.el = el("div", "comparar");
    this.el.setAttribute("role", "radiogroup");
    this.el.setAttribute("aria-label", "Fonte do resultado");
    this.bot = /* @__PURE__ */ new Map();
    for (const [m, rot] of MODOS) {
      const b = el("button", null, rot);
      b.type = "button";
      b.setAttribute("role", "radio");
      b.dataset.modo = m;
      b.addEventListener("click", () => {
        if (this.modo !== m) aoMudar(m);
      });
      this.bot.set(m, b);
      this.el.append(b);
    }
    this.definir(modo);
  }
  definir(m) {
    this.modo = m;
    for (const [k, b] of this.bot) b.setAttribute("aria-checked", String(k === m));
    this.el.dataset.modo = m;
  }
};
function celulasTse(dig) {
  const pctH = new Hodometro({ inteiros: 3, casas: 2, sufixo: "%" });
  const votH = new Hodometro({ inteiros: dig, sufixo: "votos" });
  const difH = new Hodometro({ inteiros: dig });
  const pos = el("span", "pos-tse", "");
  pos.title = "Posição no TSE";
  const sinal = el("span", "sinal", "");
  const tse = el("span", "tse", [el("span", "tse-l1", [pos, el("span", "tse-pct", pctH.el)]), el("span", "tse-qtd", votH.el)]);
  const dif = el("span", "dif", [el("span", "dif-rot", "UV − TSE"), el("span", "dif-val", [sinal, difH.el])]);
  dif.title = "Diferença de votos: Urna Velox − TSE";
  return { tse, dif, pos, pctH, votH, difH, sinal, ultPos: null, ultSinal: null };
}
function desenharCelulas(c, k, uv, posUv) {
  c.pctH.definir(k ? k.pct : null);
  c.votH.definir(k ? k.v : null);
  const p = k ? `${k.pos}º` : "";
  if (c.ultPos !== p) {
    c.pos.textContent = p;
    c.ultPos = p;
  }
  const d = k && uv != null ? uv - k.v : null;
  c.difH.definir(d == null ? null : Math.abs(d));
  const s = d == null ? "" : d > 0 ? "+" : d < 0 ? "−" : "";
  if (c.ultSinal !== s) {
    c.sinal.textContent = s;
    c.ultSinal = s;
  }
  return k != null && posUv != null && k.pos !== posUv + 1;
}
function segundosBr(s) {
  const m = /(\d{2})\/(\d{2})\/(\d{4}) (\d{2}):(\d{2}):(\d{2})/.exec(s || "");
  return m ? Date.UTC(+m[3], +m[2] - 1, +m[1], +m[4], +m[5], +m[6]) / 1e3 : null;
}
function situacaoTse(d) {
  if (!d) return null;
  if (d.md === "e") return "eleito";
  if (d.md === "s") return "2t";
  if (d.finalizada) {
    if (d.cand?.some((k) => k.st === "2º turno")) return "2t";
    if (d.cand?.some((k) => k.st === "Eleito" || String(k.st).startsWith("Eleito"))) return "eleito";
  }
  return null;
}
function textoEstado(m) {
  if (!m) return "buscando no TSE…";
  switch (m.estado) {
    case "ok":
      return "";
    case "aguardando":
      return "buscando no TSE…";
    case "desatualizado":
      return "TSE desatualizado";
    case "pausado":
      return "TSE em pausa (HTTP 429)";
    case "indisponivel":
      return m.motivo === "limite_topicos" ? "comparação indisponível no momento" : m.motivo === "404" ? "TSE ainda sem arquivo" : "TSE indisponível";
    default:
      return "";
  }
}
function idade(s) {
  if (s == null || !Number.isFinite(s)) return "";
  s = Math.max(0, Math.round(s));
  if (s < 90) return `há ${s} s`;
  const m = Math.round(s / 60);
  if (m < 60) return `há ${m} min`;
  return `há ${Math.floor(m / 60)} h ${String(m % 60).padStart(2, "0")} min`;
}
function textoFontes(modo, res, tse, agora, anc) {
  const tot = res?.secoes?.total || 0, ap = res?.secoes?.apuradas || 0;
  const uv = `Urna Velox ${pct(tot ? 100 * ap / tot : 0)}% das seções`;
  const d = tse?.dados;
  const ts = d?.secoes?.total || 0, st = d?.secoes?.totalizadas || 0;
  const tTse = d ? `TSE ${pct(ts ? 100 * st / ts : 0)}%` : "TSE —";
  const g = segundosBr(d?.geracao);
  const eq = tseNoSistema(anc, g);
  const sim = anc && g != null && (anc.x !== 1 || Math.abs(eq - g) > 3600);
  const arq = d?.geracao ? `arquivo do TSE das ${hora(d.geracao)}${sim ? ` (≙ ${horaDe(eq)} aqui)` : ""}${agora != null && eq != null ? ` · ${idade(agora - eq)}` : ""}` : "";
  if (modo === "uv") return { a: uv, b: "" };
  if (modo === "tse") return { a: tTse + (d ? " das seções" : ""), b: arq };
  return { a: `${uv} · ${tTse}`, b: arq };
}

// src/previsao.js
var comSegundos = false;
var hm2 = (t) => {
  const m = /(\d{2}):(\d{2}):(\d{2})/.exec(t || "");
  return m ? comSegundos ? `${m[1]}:${m[2]}:${m[3]}` : `${m[1]}:${m[2]}` : "—";
};
var faixaTxt = (f2) => f2 ? f2.incerta ? "em horário ainda incerto" : `entre ${hm2(f2.p10)} e ${hm2(f2.p90)}` : "";
function juntarPrevisao(ant, m, x = 1) {
  const hist2 = m.instantaneo ? m.historico || [] : (ant?.historico || []).slice();
  if (!m.instantaneo && m.mat) {
    const p = [segundos(m.instante), segundos(m.mat.p10), segundos(m.mat.p50), segundos(m.mat.p90)];
    const u = hist2[hist2.length - 1];
    const passo = Math.max(1, 60 / (x || 1));
    if (u && Math.floor(u[0] / passo) === Math.floor(p[0] / passo)) hist2[hist2.length - 1] = p;
    else hist2.push(p);
    if (hist2.length > 1500) hist2.splice(0, hist2.length - 1500);
  }
  return { ...m, historico: hist2 };
}
var CartaoPrevisao = class {
  constructor() {
    this.rot = el("div", "pv-rot", "Matematicamente");
    this.hod = new Hodometro({ molde: "##:##" });
    this.faixa = el("div", "pv-faixa", "");
    this.nota = el("div", "pv-nota", "");
    this.g = svg("svg", { viewBox: "0 0 300 80", preserveAspectRatio: "none", role: "img", "aria-label": "Faixa prevista ao longo da noite" });
    this.banda = svg("path", { class: "pv-banda", d: "" });
    this.med = svg("path", { class: "pv-med", d: "" });
    this.real = svg("line", { class: "pv-real", x1: 0, x2: 300, y1: 0, y2: 0 });
    this.real.style.opacity = "0";
    this.g.append(this.banda, this.med, this.real);
    this.fim = el("div", "pv-fim", "");
    this.el = el("section", "vidro previsao", [
      el("h2", "cartao-titulo", ["Previsão ", el("small", null, "(estimativa de horário)")]),
      el("div", "pv-topo", [el("div", "pv-txt", [this.rot, this.faixa, this.nota]), el("div", "pv-hod", [this.hod.el, el("small", null, "mais provável")])]),
      el("div", "pv-graf", this.g),
      this.fim
    ]);
    this.el.setAttribute("aria-live", "off");
    this.chave = "";
    this.x = 1;
  }
  /** M-003: aceleração do simulado (âncora do relógio do sistema). Acima de 1×, horários com segundos. */
  definirAceleracao(x) {
    x = x > 0 ? x : 1;
    if (x === this.x) return;
    this.x = x;
    comSegundos = x > 1;
    const molde = comSegundos ? "##:##:##" : "##:##";
    if (this.hod.molde !== molde) {
      const novo = new Hodometro({ molde });
      this.hod.el.replaceWith(novo.el);
      this.hod = novo;
      this.hod.molde = molde;
    }
    this.chave = "";
  }
  /** p = última previsão do tópico (ou null); circ = código da circunscrição do tópico; nomeCirc = nome para o texto. */
  definir(p, ehCirc, nomeCirc) {
    const chave = p ? `${p.instante}|${p.situacao}|${ehCirc}` : `nada|${ehCirc}`;
    if (chave === this.chave) return;
    this.chave = chave;
    let rot = "Matematicamente", faixa2 = "", nota = "", hod = null, fim = "";
    if (!p) {
      rot = ehCirc ? "Matematicamente" : "Fim da apuração";
      faixa2 = "aguardando base";
      nota = "a previsão começa com 5% dos eleitores apurados e 10 min de apuração";
    } else {
      const conf = p.confirmacao;
      if (p.situacao === "definida" || p.definida) {
        rot = "Matematicamente: saiu";
        faixa2 = `às ${hm2(p.definida || conf?.saiu)}`;
        if (conf) nota = conf.dentro ? `dentro da faixa prevista (${hm2(conf.faixa.p10)}–${hm2(conf.faixa.p90)})${conf.desde ? ` desde ${hm2(conf.desde)}` : ""}` : `fora da faixa prevista (${hm2(conf.faixa.p10)}–${hm2(conf.faixa.p90)})`;
        else nota = "saiu antes de haver base para a previsão";
        hod = hm2(p.definida || conf?.saiu);
      } else if (p.mat) {
        const m = p.mat;
        rot = m.tipo === "matematicamente_2t" ? "Matematicamente 2º turno sai" : "Matematicamente eleito sai";
        faixa2 = faixaTxt(m);
        const a = Math.round(100 * (m.apurado_p50 || 0));
        nota = m.prob_antes_fim < 0.2 ? "deve ir até o fim da apuração" : `chance de sair antes do fim: ${Math.round(100 * m.prob_antes_fim)}% · com ~${a}% apurado`;
        hod = m.incerta ? null : hm2(m.p50);
      } else if (!ehCirc) {
        rot = "O matematicamente";
        faixa2 = `se decide em ${nomeCirc}`;
        nota = "abra a circunscrição para ver a previsão";
      }
      if (p.parada && p.situacao === "suspensa") {
        if (p.mat) faixa2 = `${faixaTxt(p.mat)} (adiada)`;
        nota = `chegada de BUs parada desde ${hm2(p.parada.desde)} — previsão adiada`;
      }
      const f99 = p.chegou?.fim99 ? `99% às ${hm2(p.chegou.fim99)}` : p.fim99 ? `99% ${faixaTxt(p.fim99)}` : "";
      const f100 = p.chegou?.fim100 ? `100% às ${hm2(p.chegou.fim100)}` : p.fim100 ? `100% ${faixaTxt(p.fim100)}` : "";
      fim = [f99, f100].filter(Boolean).join(" · ");
      if (fim) fim = `Fim da apuração: ${fim}`;
    }
    if (this.rot.textContent !== rot) this.rot.textContent = rot;
    if (this.faixa.textContent !== faixa2) this.faixa.textContent = faixa2;
    if (this.nota.textContent !== nota) {
      this.nota.textContent = nota;
      this.nota.title = nota;
    }
    this.nota.classList.toggle("alerta", !!(p?.parada && p.situacao === "suspensa"));
    if (this.fim.textContent !== fim) {
      this.fim.textContent = fim;
      this.fim.title = fim;
    }
    this.hod.definir(hod);
    this.desenhar(p);
  }
  /** A faixa P10–P90 (área) e o P50 (linha) de cada previsão da noite; a linha horizontal é o horário real. */
  desenhar(p) {
    const h = (p?.historico || []).filter((x) => x.every((v) => v != null));
    const real = p?.definida ? segundos(p.definida) : null;
    if (h.length < 2) {
      this.banda.setAttribute("d", "");
      this.med.setAttribute("d", "");
      this.real.style.opacity = "0";
      return;
    }
    const k = 1 / (this.x || 1);
    const x0 = h[0][0], x1 = Math.max(h[h.length - 1][0], x0 + 60 * k);
    let lo = Math.min(...h.map((x) => x[1])), hi = Math.max(...h.map((x) => x[3]));
    if (real != null) {
      lo = Math.min(lo, real);
      hi = Math.max(hi, real);
    }
    const pad = Math.max(300 * k, (hi - lo) * 0.08);
    lo -= pad;
    hi += pad;
    const X = (t) => (t - x0) / (x1 - x0) * 300, Y = (t) => 80 - (t - lo) / (hi - lo) * 80;
    const cima = h.map((x) => `${X(x[0]).toFixed(1)},${Y(x[3]).toFixed(1)}`);
    const baixo = h.map((x) => `${X(x[0]).toFixed(1)},${Y(x[1]).toFixed(1)}`).reverse();
    this.banda.setAttribute("d", `M${cima.join("L")}L${baixo.join("L")}Z`);
    this.med.setAttribute("d", `M${h.map((x) => `${X(x[0]).toFixed(1)},${Y(x[2]).toFixed(1)}`).join("L")}`);
    if (real != null) {
      const yr = Y(real).toFixed(1);
      this.real.setAttribute("y1", yr);
      this.real.setAttribute("y2", yr);
    }
    this.real.style.opacity = real != null ? "1" : "0";
  }
};

// src/identificacao.js
var $ = (s) => document.querySelector(s);
var lembrar = (v) => {
  try {
    if (v == null) return localStorage.getItem("uv-ident");
    localStorage.setItem("uv-ident", v);
  } catch {
  }
  return null;
};
if (lembrar() === "recolhida") document.documentElement.classList.add("ident-recolhida");
var Identificacao = class {
  constructor() {
    this.lugar = $("#ident");
    const todos = (s) => [...this.lugar.querySelectorAll(s)];
    this.el = { eleicao: todos(".id-eleicao"), tipo: todos(".id-tipo"), local: todos(".id-local"), cargo: todos(".id-cargo"), seg: todos(".id-seg") };
    this.btns = todos(".id-recolher");
    for (const b of this.btns) b.addEventListener("click", () => this.recolher(!document.documentElement.classList.contains("ident-recolhida")));
    this.recolher(lembrar() === "recolhida", false);
    const ver = () => {
      const preso = this.lugar.getBoundingClientRect().bottom < 4;
      if (this.lugar.classList.contains("preso") !== preso) this.lugar.classList.toggle("preso", preso);
    };
    window.addEventListener("scroll", ver, { passive: true });
    window.addEventListener("resize", ver, { passive: true });
    ver();
  }
  recolher(sim, guardar = true) {
    document.documentElement.classList.toggle("ident-recolhida", sim);
    for (const b of this.btns) {
      b.textContent = sim ? "⌄" : "⌃";
      b.setAttribute("aria-expanded", String(!sim));
      b.setAttribute("aria-label", sim ? "Mostrar a identificação da eleição" : "Recolher a identificação da eleição");
    }
    if (guardar) lembrar(sim ? "recolhida" : "aberta");
  }
  /** { ano, geral, turno, cargo, local } */
  definir(x) {
    const t = (es, s) => {
      for (const e of es) if (e.textContent !== s) e.textContent = s;
    };
    t(this.el.eleicao, `Eleições ${x.ano || ""}`.trim());
    t(this.el.tipo, x.geral ? "Gerais" : "Municipais");
    t(this.el.cargo, x.cargo || "");
    t(this.el.local, x.local || "");
    for (const seg of this.el.seg) {
      [...seg.children].forEach((i, k) => i.classList.toggle("sel", k + 1 === x.turno));
      seg.setAttribute("aria-label", x.turno ? `${x.turno}º turno` : "");
    }
    document.title = `${x.cargo ? `${x.cargo} · ${x.local} · ` : ""}${x.turno ? `${x.turno}º turno · ` : ""}Urna Velox`;
  }
};

// src/app.js
var $2 = (s) => document.querySelector(s);
var cargo4 = (c) => String(c).padStart(4, "0");
var Resultados = class {
  constructor() {
    this.m = /* @__PURE__ */ new Map();
  }
  por(t, seq, txt) {
    this.m.set(t, { seq, txt, obj: null });
  }
  seq(t) {
    return this.m.get(t)?.seq ?? -1;
  }
  has(t) {
    return this.m.has(t);
  }
  get(t) {
    const x = this.m.get(t);
    if (!x) return void 0;
    if (!x.obj) {
      try {
        x.obj = JSON.parse(x.txt).resultado;
      } catch {
        x.obj = null;
      }
      x.txt = null;
    }
    return x.obj;
  }
  *[Symbol.iterator]() {
    for (const t of this.m.keys()) yield [t, this.get(t)];
  }
};
var S = {
  N: new Nomes(null),
  topicos: /* @__PURE__ */ new Set(),
  // tópicos existentes (GET /api/v1/topicos)
  res: new Resultados(),
  // tópico → último resultado
  sujos: /* @__PURE__ */ new Set(),
  eventos: /* @__PURE__ */ new Map(),
  // chave de conteúdo → evento (dedup na reconexão, §5)
  marcos: /* @__PURE__ */ new Map(),
  faltam: /* @__PURE__ */ new Map(),
  // tópico da circunscrição → último faltam_para_mat
  estado: null,
  conexao: "conectando",
  tela: null,
  // { abr, cargo, topico }
  ui: null,
  // elementos da tela atual
  cls: 0,
  msgs: 0,
  ritmo: [],
  // [instante ms, seq] do br/maior tópico (BUs por minuto)
  // comparação com o TSE (etapa 7, comparacao.js)
  comparar: modoInicial(),
  // 'uv' | 'ambos' | 'tse'
  tse: /* @__PURE__ */ new Map(),
  // tópico → última mensagem `tse` (PROTOCOLO_WS §4.6)
  relogio: null,
  // { t: segundos do site, em: performance.now(), x: velocidade } da última mensagem `estado`
  // M-003 (relogio.js): relógio do sistema do servidor { t, em } e a âncora com a noite do TSE { noite0, sis0, x }
  relSis: null,
  anc: null,
  matSis: /* @__PURE__ */ new Map(),
  // tópico da circunscrição → horário do sistema do evento do "matematicamente" (sem candidato)
  confirmados: /* @__PURE__ */ new Map(),
  // tópico → marco "TSE confirmou" (para os ensaios)
  ordemDesde: /* @__PURE__ */ new Map(),
  // tópico → geração (s) do 1º arquivo do TSE com a ordem diferente
  // previsão (etapa 8, previsao.js): tópico → última mensagem `previsao` (com o histórico das faixas)
  prev: /* @__PURE__ */ new Map(),
  prevConfirmadas: /* @__PURE__ */ new Map()
  // tópico → confirmação (marco "previsão confirmada", para os ensaios)
};
window.__uv = S;
try {
  new PerformanceObserver((l) => {
    for (const e of l.getEntries()) {
      if (e.hadRecentInput) continue;
      S.cls += e.value;
      if ((S.clsFontes ||= []).length < 20) S.clsFontes.push({ v: e.value, t: Math.round(e.startTime), de: (e.sources || []).map((s) => (s.node?.className || s.node?.nodeName || "?") + " " + JSON.stringify(s.previousRect) + "→" + JSON.stringify(s.currentRect)) });
    }
  }).observe({ type: "layout-shift", buffered: true });
} catch {
}
function receber(m) {
  S.msgs++;
  switch (m.tipo) {
    case "resultado": {
      if (S.res.seq(m.topico) > m.seq) return;
      S.res.por(m.topico, m.seq, m.txt);
      S.sujos.add(m.topico);
      S.seqMax = Math.max(S.seqMax || 0, m.seq);
      pedirQuadro();
      break;
    }
    case "evento": {
      const k = `${m.seq}|${m.abrangencia}|${m.cargo}|${m.candidato}|${m.estado}`;
      if (S.eventos.has(k)) return;
      S.eventos.set(k, m);
      if (m.classe === "matematicamente" && m.sistema && m.topico && !String(m.estado).startsWith("revogado") && !S.matSis.has(m.topico)) S.matSis.set(m.topico, m.sistema);
      const it = itemEvento(S.N, m);
      if (it) porNoFeed(it);
      S.primeirosSujo = true;
      if (m.classe === "apuracao") S.estadoSujo = true;
      pedirQuadro();
      break;
    }
    case "marco": {
      if (m.classe === "faltam_para_mat") {
        const ant = S.faltam.get(m.topico);
        if (ant && ant.seq > m.seq) return;
        S.faltam.set(m.topico, m);
        S.faltamSujo = true;
        pedirQuadro();
        return;
      }
      const k = `${m.classe}|${m.abrangencia}|${m.cargo}|${m.seq}|${m.secao}|${m.valor}`;
      if (S.marcos.has(k)) return;
      S.marcos.set(k, m);
      const it = itemMarco(S.N, m);
      if (it) porNoFeed(it);
      if (m.classe === "pct_secoes" && m.valor === 100 && m.abrangencia.length === 2 && !S.primeiraUf100) {
        S.primeiraUf100 = m;
        porNoFeed({
          id: "c:uf100",
          ic: "★",
          t: hora(quando(m)),
          seq: m.seq,
          cat: "andamento",
          texto: `Primeira UF com 100% das seções apuradas: ${S.N.uf(m.abrangencia)}`,
          detalhe: { titulo: "Primeira UF 100% apurada", linhas: [["UF", S.N.uf(m.abrangencia)], ["Horário (Urna Velox)", hora(quando(m))], ["Horário do BU no TSE", hora(m.horario)], ["BUs aplicados (seq)", mil(m.seq)]] }
        });
      }
      break;
    }
    case "tse": {
      const t = String(m.topico || "").slice(4);
      const ant = S.tse.get(t);
      if (ant && !m.instantaneo && ant.ver >= m.ver) return;
      S.tse.set(t, m);
      S.tseSujo = true;
      if (S.tela?.topico === t) S.sujos.add(t);
      verificarConfirmacao(t);
      pedirQuadro();
      break;
    }
    case "previsao": {
      S.prev.set(m.topico, juntarPrevisao(S.prev.get(m.topico), m, S.anc?.x || 1));
      S.prevSujo = true;
      const c = m.confirmacao;
      if (c && m.circunscricao && !S.prevConfirmadas.has(m.topico)) {
        S.prevConfirmadas.set(m.topico, c);
        const d = S.N.disputa(m.abrangencia, m.cargo);
        const hm3 = (t) => hora(t).slice(0, (S.anc?.x || 1) > 1 ? 8 : 5);
        const txt = c.dentro ? `Previsão confirmada — ${d}: saiu às ${hm3(c.saiu)}, dentro da faixa prevista${c.desde ? ` desde ${hm3(c.desde)}` : ""}` : `Previsão fora da faixa — ${d}: saiu às ${hm3(c.saiu)}, previsto entre ${hm3(c.faixa.p10)} e ${hm3(c.faixa.p90)}`;
        porNoFeed({
          id: `p:prev:${m.topico}`,
          ic: "◷",
          t: hora(c.saiu),
          seq: S.seqMax,
          cat: "andamento",
          texto: txt,
          detalhe: { titulo: `Previsão do matematicamente — ${d}`, linhas: [
            ["Saiu às", hora(c.saiu)],
            ["Última faixa prevista", `${hora(c.faixa.p10)} a ${hora(c.faixa.p90)}`],
            ["Dentro da faixa", c.dentro ? "sim" : "não"],
            ["Faixas certas desde", c.desde ? hora(c.desde) : "—"],
            ["O que é", "estimativa (P10–P90) a partir da velocidade de chegada dos BUs e da tendência dos votos; nunca um resultado"]
          ] }
        });
      }
      pedirQuadro();
      break;
    }
    case "estado":
      S.estado = m;
      if (m.ultimo_bu?.seq != null) S.seqMax = Math.max(S.seqMax || 0, m.ultimo_bu.seq);
      S.relogio = { t: segundosBr(m.relogio_site), em: performance.now(), x: m.replay ? m.replay.pausado ? 0 : m.replay.acelerar || 1 : 1 };
      if (m.relogio_sistema) {
        S.relSis = { t: segundosF(m.relogio_sistema.agora), em: performance.now() };
        const a = ancora(m.relogio_sistema);
        if (a && (!S.anc || a.sis0 !== S.anc.sis0 || a.x !== S.anc.x)) {
          S.anc = a;
          S.ui?.prev?.definirAceleracao(a.x);
          S.prevSujo = true;
          S.fontesSujo = true;
        }
      }
      S.estadoSujo = true;
      S.player?.estadoWs(m.replay);
      pedirQuadro();
      break;
    case "erro":
      if (m.motivo === "comparacao_desligada") {
        S.tseDesligada = true;
        S.fontesSujo = true;
        pedirQuadro();
      }
      break;
    default:
      break;
  }
}
function estadoConexao(e, info) {
  S.conexao = e;
  S.conexaoInfo = info;
  S.estadoSujo = true;
  pedirQuadro();
}
var feed;
var folha;
var hist;
function porNoFeed(it) {
  hist.por(it);
  const item = feed.por(it);
  if (it.destaque) destacar(item.el);
}
var destaques = [];
function destacar(e) {
  if (movimentoReduzido()) return;
  e.classList.add("brilho");
  const h = el("span", "halo");
  e.append(h);
  destaques.push([e, h]);
  while (destaques.length > 2) {
    const [a, b] = destaques.shift();
    a.classList.remove("brilho");
    b.remove();
  }
  setTimeout(() => {
    const i = destaques.findIndex((x) => x[0] === e);
    if (i >= 0) {
      destaques.splice(i, 1);
      e.classList.remove("brilho");
      h.remove();
    }
  }, 3e4);
}
function lerRota() {
  const m = /^#\/([a-z]{2}(?:\d{5})?)\/c(\d{4})$/.exec(location.hash);
  let a = m?.[1], c = m ? +m[2] : null;
  if (!a || !S.topicos.has(`${a}-c${cargo4(c)}`)) {
    const pref = ["br-c0001", ...[...S.topicos].sort()];
    const t = pref.find((x) => S.topicos.has(x));
    if (!t) return null;
    const p = /^(.+)-c(\d{4})$/.exec(t);
    a = p[1];
    c = +p[2];
  }
  return { abr: a, cargo: c, topico: `${a}-c${cargo4(c)}` };
}
function cargosDe(a) {
  return [...S.topicos].filter((t) => t.startsWith(`${a}-c`)).map((t) => +t.slice(-4)).sort((x, y) => x - y);
}
function montarRota(t) {
  const nav = $2("#rota");
  const a = abr(t.abr);
  const ufs = [...new Set([...S.topicos].map((x) => x.split("-")[0]).filter((x) => x.length === 2 && x !== "br"))].sort();
  const munis = a.uf ? [...new Set([...S.topicos].map((x) => x.split("-")[0]).filter((x) => x.length === 7 && x.startsWith(a.uf)))] : [];
  const br = el("button", "passo", "Brasil");
  br.type = "button";
  br.addEventListener("click", () => irPara("br"));
  const filhos = [];
  if (S.topicos.has("br-c0001") || [...S.topicos].some((x) => x.startsWith("br-"))) filhos.push(br);
  const ufsTodas = [.../* @__PURE__ */ new Set([...ufs, ...[...S.topicos].map((x) => x.split("-")[0]).filter((x) => x.length === 7).map((x) => x.slice(0, 2))])].sort();
  if (ufsTodas.length) {
    const s = el("select", "passo");
    s.setAttribute("aria-label", "Estado");
    s.append(new Option("Estado…", ""));
    for (const u of ufsTodas) s.append(new Option(S.N.uf(u), u, false, u === a.uf));
    s.addEventListener("change", () => s.value && irPara(s.value));
    filhos.push(s);
  }
  if (munis.length) {
    const s = el("select", "passo");
    s.setAttribute("aria-label", "Município");
    s.append(new Option("Município…", ""));
    munis.map((m) => [m, S.N.mun(m)]).sort((x, y) => x[1].localeCompare(y[1], "pt-BR")).forEach(([m, n]) => s.append(new Option(n, m, false, m === t.abr)));
    s.addEventListener("change", () => s.value && irPara(s.value));
    filhos.push(s);
  }
  nav.replaceChildren(...filhos);
}
function irPara(a, c) {
  const cs = cargosDe(a);
  let cargo = c ?? (S.tela && cs.includes(S.tela.cargo) ? S.tela.cargo : cs[0]);
  if (cargo == null) {
    const m = [...S.topicos].map((x) => x.split("-")[0]).filter((x) => x.startsWith(a) && x.length === 7).sort()[0];
    if (!m) return;
    a = m;
    cargo = cargosDe(m)[0];
  }
  location.hash = `#/${a}/c${cargo4(cargo)}`;
}
function montarCargos(t) {
  const nav = $2("#cargos");
  nav.replaceChildren(...cargosDe(t.abr).map((c) => {
    const b = el("button", null, S.N.cargo(c));
    b.type = "button";
    b.setAttribute("role", "tab");
    b.setAttribute("aria-selected", String(c === t.cargo));
    b.addEventListener("click", () => irPara(t.abr, c));
    return b;
  }));
}
function cardTatil(c) {
  if (movimentoReduzido()) return c;
  const mover = (e) => {
    const r = c.getBoundingClientRect();
    const x = (e.clientX - r.left) / r.width, y = (e.clientY - r.top) / r.height;
    c.style.setProperty("--ry", `${((x - 0.5) * 3).toFixed(2)}deg`);
    c.style.setProperty("--rx", `${((0.5 - y) * 3).toFixed(2)}deg`);
    c.style.setProperty("--mx", `${(x * 100).toFixed(1)}%`);
    c.style.setProperty("--my", `${(y * 100).toFixed(1)}%`);
    c.style.setProperty("--luz", "1");
  };
  const soltar = () => {
    c.style.setProperty("--rx", "0deg");
    c.style.setProperty("--ry", "0deg");
    c.style.setProperty("--luz", "0");
  };
  c.addEventListener("pointermove", (e) => {
    if (e.pointerType === "mouse" || e.buttons) mover(e);
  });
  c.addEventListener("pointerdown", mover);
  c.addEventListener("pointerleave", soltar);
  c.addEventListener("pointerup", (e) => {
    if (e.pointerType !== "mouse") soltar();
  });
  c.addEventListener("pointercancel", soltar);
  return c;
}
function stat(rot, dig, comPct) {
  const h = new Hodometro({ inteiros: dig, enxuto: true });
  const p = comPct ? new Hodometro({ inteiros: 3, casas: 2, sufixo: "%", enxuto: true }) : null;
  const caixa = el("div", null, [el("div", "rot", rot), el("div", "val", h.el)]);
  if (p) caixa.append(el("div", "p", p.el));
  return { caixa, h, p };
}
function montarPainel(t) {
  const res = S.res.get(t.topico);
  const painel = $2("#painel");
  if (!res) {
    painel.replaceChildren(el("div", "carregando", "Esperando o primeiro resultado deste cargo…"));
    S.ui = { topico: t.topico, pendente: true };
    return;
  }
  const circ = res.circ;
  const eleitorado = Math.max(res.eleitorado || 0, res.apurado || 0, 1);
  const dig = digitos(eleitorado);
  const digSec = digitos(Math.max(res.secoes?.total || 0, 1));
  const ordem = [...res.cand].sort((a, b) => a.n - b.n);
  const cores = coresDaDisputa(ordem.map((k) => ({ n: k.n, partido: S.N.candidato(circ, res.cargo, k.n).partido })));
  const linhas = /* @__PURE__ */ new Map();
  const lista = el("ol", "lista");
  lista.setAttribute("aria-label", "Candidatos em ordem de votos");
  const altura = parseFloat(getComputedStyle(document.documentElement).getPropertyValue("--linha-cand")) || 76;
  lista.style.height = `${Math.max(1, ordem.length) * altura}px`;
  for (const k of ordem) {
    const info = S.N.candidato(circ, res.cargo, k.n);
    const cor = cores.get(k.n);
    const li = el("li", "cand");
    li.style.setProperty("--c", cor);
    const pctH = new Hodometro({ inteiros: 3, casas: 2, sufixo: "%" });
    const votH = new Hodometro({ inteiros: dig, sufixo: "votos" });
    const barra = el("i");
    const pilula = el("span", "pilula", "");
    const vice = info.vices.find((v) => v.tp === "v") || info.vices[0];
    const det = [info.partido, vice ? `${vice.tp === "v" ? "vice" : "suplente"} ${vice.nome_urna}` : ""].filter(Boolean).join(" · ");
    const nome = el("div", "nome", [el("b", null, info.nome), pilula]);
    const tc = celulasTse(dig);
    li.append(
      el("span", "cor"),
      el("span", "num", String(k.n)),
      el("div", "quem", [nome, el("div", "detalhe", det || " ")]),
      el("span", "pct", pctH.el),
      el("span", "qtd", votH.el),
      tc.tse,
      tc.dif,
      el("span", "barra", barra)
    );
    li.dataset.n = k.n;
    lista.append(li);
    linhas.set(k.n, { li, pctH, votH, barra, pilula, tc, pos: -1, glow: false, diverge: false, nome: info.nome, cor });
  }
  lista.classList.add(`m-${S.comparar}`);
  const consulta = S.N.cargos.get(res.cargo)?.consulta;
  const h1 = el("h1", null, consulta ? `Consulta — ${S.N.local(t.abr)}` : t.abr === "br" ? `${S.N.cargo(res.cargo)} — Brasil` : `${S.N.cargo(res.cargo)} — ${S.N.local(t.abr)}`);
  const turno = res.turno ? `${res.turno}º turno` : "";
  const sub = el("p", "sub", [turno, circ !== t.abr ? `resultado na abrangência; quem decide é ${S.N.local(circ)}` : `eleitorado de ${mil(res.eleitorado)}`].filter(Boolean).join(" · "));
  const selo = el("div", "selo", "");
  selo.setAttribute("aria-live", "polite");
  const controle = new Controle(S.comparar, mudarModo);
  const fa = el("span", "f-a", ""), fb = el("span", "f-b", ""), fe = el("span", "f-est", "");
  const fontes = el("div", "fontes", [controle.el, el("div", "f-txt", [fa, fb]), fe]);
  const colunas = el("div", "colunas", [
    el("span", "c-uv", [el("span", "uv-longo", "Urna Velox"), el("span", "uv-curto", "UV")]),
    el("span", "c-tse", "TSE"),
    el("span", "c-dif", "Diferença")
  ]);
  colunas.setAttribute("aria-hidden", "true");
  const corrida = cardTatil(el("section", "vidro corrida", [el("header", "corrida-cab", [el("div", null, [h1, sub]), selo]), fontes, colunas, lista]));
  corrida.classList.add(`m-${S.comparar}`);
  const anel = new Anel();
  const pctSec = new Hodometro({ inteiros: 3, casas: 2, sufixo: "%", enxuto: true, centro: true });
  anel.centro.append(el("div", null, [pctSec.el, el("small", null, "das seções")]));
  const secAp = new Hodometro({ inteiros: digSec }), secTot = new Hodometro({ inteiros: digSec });
  const ultBu = new Hodometro({ molde: "##:##:##" });
  const faltH = new Hodometro({ inteiros: Math.max(dig, 9), regressivo: true, enxuto: true });
  const camadaN = el("div", "camada", [el("div", "rot", "Eleitores que faltam apurar para o matematicamente"), el("div", "val", faltH.el)]);
  const camadaD = el("div", "camada oculta decidido", "");
  const apuracao = cardTatil(el("section", "vidro apuracao", [
    el("h2", "cartao-titulo", "Apuração"),
    anel.el,
    el("div", "numeros", [
      el("div", null, [el("div", "rot", "Seções apuradas"), el("div", "val", [secAp.el, el("span", "de", " de "), secTot.el])]),
      // M-003: o horário do Urna Velox (relógio do sistema) em destaque; o do TSE embaixo, com rótulo próprio
      el("div", null, [el("div", "rot", "Totalizado pelo Urna Velox às"), el("div", "val", ultBu.el), el("div", "hora-tse", "")])
    ]),
    el("div", "faltam", [camadaN, camadaD])
  ]));
  const curva = new Curva();
  fetch(`/api/v1/curva/${t.topico}`).then((r) => r.ok ? r.json() : null).then((j) => {
    if (!j || S.ui?.curva !== curva) return;
    curva.semear(j.pontos.map(([tt, ap, tot, c]) => ({ t: tt, s: tot ? 100 * ap / tot : 0, c: new Map(c) })));
    pedirQuadro();
  }).catch(() => {
  });
  const legenda = el("div", "legenda");
  const cardCurva = cardTatil(el("section", "vidro curva", [el("h2", "cartao-titulo", "Curva da apuração"), curva.el, legenda]));
  const comp = stat("Comparecimento", dig, true), abst = stat("Abstenção", dig, true), brancos = stat("Brancos", dig, true), nulos = stat("Nulos", dig, true);
  const cardVotos = cardTatil(el("section", "vidro votos", [el("h2", "cartao-titulo", "Votos"), comp.caixa, abst.caixa, brancos.caixa, nulos.caixa]));
  const prev = new CartaoPrevisao();
  const cardPrev = cardTatil(prev.el);
  painel.replaceChildren(corrida, el("aside", "lado", [apuracao, cardPrev, cardCurva, cardVotos]));
  S.ui = {
    topico: t.topico,
    circ,
    linhas,
    selo,
    anel,
    pctSec,
    secAp,
    secTot,
    ultBu,
    faltH,
    camadaN,
    camadaD,
    curva,
    legenda,
    comp,
    abst,
    brancos,
    nulos,
    altura,
    ultCurva: 0,
    corrida,
    lista,
    controle,
    fa,
    fb,
    fe,
    rotBu: apuracao.querySelector(".numeros > div:nth-child(2) .rot"),
    horaTse: apuracao.querySelector(".numeros .hora-tse"),
    prev,
    cargo: res.cargo
  };
  if (S.anc) prev.definirAceleracao(S.anc.x);
  S.sujos.add(t.topico);
  S.faltamSujo = true;
  S.prevSujo = true;
  atualizarTopicos();
  if (S.ident) {
    const ano = [res.ultimo_bu, S.estado?.relogio_site].map((x) => /(20\d\d)/.exec(x || "")?.[1]).find(Boolean);
    S.ident.definir({
      ano,
      geral: [...S.topicos].some((x) => x.startsWith("br-")),
      turno: res.turno,
      cargo: S.N.cargo(res.cargo),
      local: t.abr === "br" ? "Brasil" : S.N.local(t.abr)
    });
  }
}
function montarTela() {
  const t = lerRota();
  if (!t) return;
  S.tela = t;
  montarRota(t);
  montarCargos(t);
  montarPainel(t);
  atualizarTopicos();
  pedirQuadro();
}
function atualizarTopicos() {
  const lista = [];
  if (S.tela) {
    lista.push(S.tela.topico);
    const circ = S.res.get(S.tela.topico)?.circ;
    const tc = circ ? `${circ}-c${cargo4(S.tela.cargo)}` : null;
    if (tc && tc !== S.tela.topico && S.topicos.has(tc)) lista.push(tc);
    const fal = [tc || S.tela.topico];
    if (S.topicos.has("br-c0001") && !fal.includes("br-c0001")) fal.push("br-c0001");
    S.con.definirFaltam(fal);
  }
  if (S.tela && S.comparar !== "uv" && document.visibilityState !== "hidden") lista.push(`tse:${S.tela.topico}`);
  S.con.definirTopicos(lista);
}
var pedido = false;
var ultimoQuadro = 0;
function pedirQuadro() {
  if (!pedido) {
    pedido = true;
    requestAnimationFrame(quadro);
  }
}
function quadro(agora) {
  pedido = false;
  const dt = Math.min(100, ultimoQuadro ? agora - ultimoQuadro : 16);
  ultimoQuadro = agora;
  let mexe = false;
  const ui = S.ui;
  if (ui?.pendente && S.res.has(ui.topico)) montarPainel(S.tela);
  if (S.ui && !S.ui.pendente) {
    if (S.sujos.has(S.ui.topico)) desenharTopico(agora);
    if (S.faltamSujo) desenharFaltam();
    if (S.prevSujo) {
      S.prevSujo = false;
      desenharPrevisao();
    }
    mexe = S.ui.anel.quadro(dt) | S.ui.curva.quadro(dt);
  }
  S.sujos.clear();
  if (S.ui && !S.ui.pendente && (S.fontesSujo || S.tseSujo || S.comparar !== "uv" && agora - (S.fontesEm || 0) > 1e3)) {
    desenharFontes();
    S.fontesEm = agora;
  }
  S.tseSujo = false;
  if (S.estadoSujo) desenharEstado();
  if (S.faltamSujo) {
    feedFaltam();
    S.faltamSujo = false;
  }
  if (agora - (S.curiosEm || 0) > 2e3) {
    curiosidades();
    S.curiosEm = agora;
  }
  feed.quadro(dt, agora);
  if (mexe || feed.itens.length) pedirQuadro();
}
function desenharTopico(agora) {
  const ui = S.ui, res = S.res.get(ui.topico);
  if (!res) return;
  const encerrada = !!res.encerrada;
  const modo = S.comparar;
  const tm = modo !== "uv" ? S.tse.get(ui.topico) : null;
  const d = tm?.dados || null;
  const tsePorN = new Map((d?.cand || []).map((k) => [k.n, k]));
  const usaTse = modo === "tse" && !!d;
  const linhaDe = (k, pos, v, p) => {
    const l = ui.linhas.get(k.n);
    if (!l) return null;
    if (l.pos !== pos) {
      l.li.style.setProperty("--pos", pos);
      l.pos = pos;
    }
    l.pctH.definir(p);
    l.votH.definir(v);
    l.barra.style.transform = `scaleX(${Math.max(0, Math.min(1, (p ?? 0) / 100)).toFixed(4)})`;
    l.li.classList.toggle("nao-apto", p == null && k.dvt != null && k.dvt !== "" && !String(k.dvt).startsWith("Válido"));
    return l;
  };
  let diverge = [];
  if (usaTse) {
    for (const k of d.cand) linhaDe(k, k.pos - 1, k.v, k.pct);
  } else {
    res.cand.forEach((k, pos) => {
      const l = linhaDe(k, pos, k.v, k.pct);
      if (!l) return;
      if (modo === "ambos") {
        const kt = tsePorN.get(k.n);
        const dv = desenharCelulas(l.tc, kt, k.v, pos) && (k.v > 0 || (kt?.v || 0) > 0);
        if (dv) diverge.push(k.n);
        if (dv !== l.diverge) {
          l.diverge = dv;
          l.li.classList.toggle("diverge", dv);
        }
      } else if (l.diverge) {
        l.diverge = false;
        l.li.classList.remove("diverge");
      }
    });
  }
  if (modo === "tse" && d) {
    for (const l of ui.linhas.values()) if (l.diverge) {
      l.diverge = false;
      l.li.classList.remove("diverge");
    }
  }
  const sitTse = usaTse ? situacaoTse(d) : null;
  for (const k of usaTse ? d.cand : res.cand) {
    const l = ui.linhas.get(k.n);
    if (!l) continue;
    let pil = "";
    if (usaTse) {
      if (k.st) pil = k.st;
      else if (k.e && sitTse) pil = sitTse === "eleito" ? "eleito (TSE)" : "2º turno (TSE)";
    } else if (encerrada && k.st) pil = k.st;
    else if (k.mat) pil = k.mat.estado === "eleito" ? `eleito às ${horaMat(res, k.mat.em)}` : `2º turno às ${horaMat(res, k.mat.em)}`;
    if (l.pilula.textContent !== pil) {
      l.pilula.textContent = pil;
      l.pilula.classList.toggle("ligado", !!pil);
    }
    const glow = usaTse ? !!k.e && !!sitTse : !!k.mat || k.st === "Eleito";
    if (glow !== l.glow) {
      l.glow = glow;
      if (glow && !movimentoReduzido()) {
        l.li.classList.add("brilho");
        l.li.append(el("span", "halo"));
      } else {
        l.li.classList.remove("brilho");
        l.li.querySelector(".halo")?.remove();
      }
    }
  }
  let selo = "";
  const nomeL = (k) => ui.linhas.get(k?.n)?.nome || "";
  if (usaTse) {
    const quem = d.cand.filter((k) => k.e).map(nomeL);
    if (sitTse === "eleito") selo = `TSE: ${d.finalizada ? "eleito" : "matematicamente eleito"}${quem.length ? ": " + quem.join(" e ") : ""}`;
    else if (sitTse === "2t") selo = `TSE: 2º turno${quem.length ? ": " + quem.join(" × ") : ""}`;
  } else {
    const lider = res.cand[0];
    if (res.estado === "matematicamente_eleito" || res.estado === "definido" && res.cand.some((k) => k.st === "Eleito")) {
      const el2 = res.cand.filter((k) => k.mat?.estado === "eleito" || k.st === "Eleito");
      const em = el2[0]?.mat?.em;
      selo = `${encerrada ? "Eleito" : "Matematicamente eleito"}: ${el2.map(nomeL).join(" e ") || nomeL(lider)}${em ? " às " + horaMat(res, em) : ""}`;
    } else if (res.estado === "matematicamente_2t" || res.estado === "definido" && res.cand.some((k) => k.st === "2º turno")) {
      const dois = res.cand.filter((k) => k.mat?.estado === "2t" || k.st === "2º turno");
      const em = dois.map((k) => k.mat?.em).filter(Boolean).sort()[0];
      selo = `${encerrada ? "2º turno" : "Matematicamente 2º turno"}${dois.length ? ": " + dois.map(nomeL).join(" × ") : ""}${em ? " às " + horaMat(res, em) : ""}`;
    } else if (res.estado === "sem_eleito") selo = "Sem eleito (art. 224)";
  }
  if (ui.selo.textContent !== selo) {
    ui.selo.textContent = selo;
    ui.selo.classList.toggle("ligado", !!selo);
    if (selo && !movimentoReduzido()) {
      ui.selo.classList.add("brilho");
      if (!ui.selo.querySelector(".halo")) ui.selo.append(el("span", "halo"));
    } else {
      ui.selo.classList.remove("brilho");
    }
  }
  const fonte = usaTse ? { secoes: { total: d.secoes.total, apuradas: d.secoes.totalizadas }, comparecimento: d.comparecimento, apurado: d.comparecimento + d.abstencao, vb: d.vb, vn: d.vn, vnt: d.vnt, ultimo_bu: d.geracao } : res;
  const rot = usaTse ? "Arquivo do TSE gerado às" : "Totalizado pelo Urna Velox às";
  if (ui.rotBu && ui.rotBu.textContent !== rot) ui.rotBu.textContent = rot;
  const tot = fonte.secoes?.total || 0, ap = fonte.secoes?.apuradas || 0;
  ui.anel.definir(tot ? ap / tot : 0);
  ui.pctSec.definir(tot ? 100 * ap / tot : 0);
  ui.secAp.definir(ap);
  ui.secTot.definir(tot);
  const grande = usaTse ? fonte.ultimo_bu : res.totalizado_em || res.ultimo_bu;
  ui.ultBu.definir(grande ? hora(grande) : null);
  if (ui.horaTse) {
    const tb = segundos(res.ultimo_bu);
    let txt = "";
    if (usaTse) txt = res.totalizado_em ? `Urna Velox: ${hora(res.totalizado_em)}` : "";
    else if (tb != null && res.totalizado_em) {
      const ts = segundosF(res.totalizado_em);
      txt = `último BU no TSE: ${hora(res.ultimo_bu)}${Math.abs(ts - tb) > 43200 ? ` de ${diaDe(tb)}` : ""}`;
    }
    if (ui.horaTse.textContent !== txt) ui.horaTse.textContent = txt;
  }
  const c = fonte.comparecimento || 0, abst = Math.max(0, (fonte.apurado || 0) - c);
  const pc = (x, b) => b ? 100 * x / b : null;
  ui.comp.h.definir(c);
  ui.comp.p.definir(pc(c, fonte.apurado));
  ui.abst.h.definir(abst);
  ui.abst.p.definir(pc(abst, fonte.apurado));
  ui.brancos.h.definir(fonte.vb || 0);
  ui.brancos.p.definir(pc(fonte.vb || 0, c));
  ui.nulos.h.definir((fonte.vn || 0) + (fonte.vnt || 0));
  ui.nulos.p.definir(pc((fonte.vn || 0) + (fonte.vnt || 0), c));
  const totUv = res.secoes?.total || 0, apUv = res.secoes?.apuradas || 0;
  if (agora - ui.ultCurva > 250 || encerrada) {
    ui.ultCurva = agora;
    const a = res.cand[0], b = res.cand[1];
    ui.curva.linhas([a, b].filter(Boolean).map((k) => ({ n: k.n, cor: ui.linhas.get(k.n)?.cor })));
    ui.curva.ponto(res.totalizado_em ? segundosF(res.totalizado_em) : segundos(res.ultimo_bu), totUv ? 100 * apUv / totUv : 0, res.cand.map((k) => [k.n, k.pct]));
    const leg = `${a?.n}|${b?.n}`;
    if (ui.leg !== leg) {
      ui.leg = leg;
      const item = (cor, txt) => {
        const i = el("i");
        i.style.setProperty("--c", cor);
        return el("span", null, [i, txt]);
      };
      ui.legenda.replaceChildren(item("#a996ff", "seções (0–100%)"), ...[a, b].filter(Boolean).map((k) => item(ui.linhas.get(k.n)?.cor, ui.linhas.get(k.n)?.nome)));
    }
  }
  if (modo === "ambos" && d) ordemDiferente(ui.topico, res, d, diverge);
  verificarConfirmacao(ui.topico);
  S.fontesSujo = true;
  S.faltamSujo = true;
}
function desenharFontes() {
  S.fontesSujo = false;
  const ui = S.ui;
  if (!ui || ui.pendente) return;
  const res = S.res.get(ui.topico);
  const tm = S.comparar !== "uv" ? S.tse.get(ui.topico) : null;
  const f2 = textoFontes(S.comparar, res, tm, agoraSistema(), S.anc);
  if (ui.fa.textContent !== f2.a) ui.fa.textContent = f2.a;
  if (ui.fb.textContent !== f2.b) ui.fb.textContent = f2.b;
  const e = S.comparar === "uv" ? "" : S.tseDesligada ? "comparação desligada neste servidor" : textoEstado(tm);
  if (ui.fe.textContent !== e) {
    ui.fe.textContent = e;
    ui.fe.dataset.estado = tm?.estado || "aguardando";
    ui.fe.classList.toggle("ligado", !!e);
  }
}
function horaMat(res, em) {
  const s = S.matSis.get(`${res.circ}-c${cargo4(res.cargo)}`);
  if (s) return hora(s);
  return S.anc ? horaDe(tseNoSistema(S.anc, segundos(em))) : hora(em);
}
function agoraSistema() {
  const r = S.relSis;
  if (!r || r.t == null) return null;
  return r.t + (performance.now() - r.em) / 1e3;
}
function verificarConfirmacao(t) {
  if (S.confirmados.has(t)) return;
  const d = S.tse.get(t)?.dados;
  const sit = situacaoTse(d);
  const res = S.res.get(t);
  if (!sit || !res || res.circ !== res.abr) return;
  const uv = res.estado === "matematicamente_eleito" ? "eleito" : res.estado === "matematicamente_2t" ? "2t" : null;
  const ems = res.cand.map((k) => k.mat?.em).filter(Boolean).sort();
  if (uv !== sit || !ems.length) return;
  const emSis = S.matSis.get(t) || null;
  const tUv = emSis ? segundosF(emSis) : tseNoSistema(S.anc, segundos(ems[0])), tTse = segundosBr(d.geracao);
  if (tUv == null || tTse == null) return;
  const cmp = comparar(S.anc, tUv, tTse);
  const emUv = emSis || `${horaDe(tUv)}`;
  const it = itemConfirmou(S.N, res.circ, res.cargo, sit, emUv, d.geracao, cmp, S.anc);
  S.confirmados.set(t, { sit, uv: emUv, tse: d.geracao, tse_sistema: horaDe(cmp.tse), s: Math.round(cmp.s), min: Math.round(cmp.s / 60) });
  it.seq = S.seqMax || res.seq;
  porNoFeed(it);
}
function ordemDiferente(t, res, d, diverge) {
  const g = segundosBr(d.geracao);
  if (!diverge.length || g == null) {
    S.ordemDesde.delete(t);
    return;
  }
  let o = S.ordemDesde.get(t);
  if (!o) {
    o = { desde: g, geracao: d.geracao, avisado: false };
    S.ordemDesde.set(t, o);
  }
  if (!o.avisado && g - o.desde >= 60) {
    o.avisado = true;
    const quem = diverge.slice(0, 3).map((n) => {
      const uv = res.cand.findIndex((k) => k.n === n) + 1, ts = d.cand.find((k) => k.n === n)?.pos;
      return `${S.ui?.linhas.get(n)?.nome || n} ${uv}º no Urna Velox, ${ts}º no TSE`;
    }).join("; ");
    const it = itemOrdemDiferente(S.N, res.circ, res.cargo, o.geracao, quem);
    it.seq = S.seqMax || res.seq;
    porNoFeed(it);
  }
}
function mudarModo(m) {
  S.comparar = m;
  gravarModo(m);
  const ui = S.ui;
  if (ui && !ui.pendente) {
    ui.controle.definir(m);
    for (const x of [ui.corrida, ui.lista]) {
      x.classList.remove("m-uv", "m-ambos", "m-tse");
      x.classList.add(`m-${m}`);
    }
    S.sujos.add(ui.topico);
  }
  atualizarTopicos();
  S.fontesSujo = true;
  pedirQuadro();
}
function desenharPrevisao() {
  const ui = S.ui;
  if (!ui || ui.pendente) return;
  const pTela = S.prev.get(ui.topico) || null;
  const tc = `${ui.circ}-c${cargo4(ui.cargo)}`;
  const pCirc = tc === ui.topico ? pTela : S.prev.get(tc) || null;
  if (!pCirc) {
    ui.prev.definir(pTela, tc === ui.topico, S.N.local(ui.circ));
    return;
  }
  const p = pTela === pCirc ? pTela : { ...pCirc, fim99: pTela?.fim99 ?? null, fim100: pTela?.fim100 ?? null, chegou: pTela?.chegou ?? {}, instante: `${pCirc.instante}|${pTela?.instante || ""}` };
  ui.prev.definir(p, true, S.N.local(ui.circ));
}
function desenharFaltam() {
  const ui = S.ui, res = S.res.get(ui.topico);
  if (!res) return;
  const f2 = S.faltam.get(`${res.circ}-c${cargo4(res.cargo)}`);
  const decidido = res.estado !== "apurando" || f2?.faltam === 0;
  if (decidido) {
    const t = res.estado === "matematicamente_2t" ? "Matematicamente 2º turno" : res.estado === "sem_eleito" ? "Sem eleito" : "Matematicamente decidido";
    const hs = S.matSis.get(`${res.circ}-c${cargo4(res.cargo)}`) || quando(f2);
    const txt = `${t}${hs ? " às " + hora(hs) : ""}`;
    if (ui.camadaD.dataset.txt !== txt) {
      ui.camadaD.dataset.txt = txt;
      ui.camadaD.replaceChildren(el("div", null, txt), el("small", null, f2 ? `no BU nº ${mil(f2.seq)} da noite` : ""));
    }
    ui.faltH.definir(0);
  } else {
    ui.faltH.definir(f2 && f2.faltam != null ? f2.faltam : null);
  }
  ui.camadaN.classList.toggle("oculta", decidido);
  ui.camadaD.classList.toggle("oculta", !decidido);
}
var faixa;
function montarFaixa() {
  const ritmoH = new Hodometro({ inteiros: 6 });
  const ritmo = el("span", "chip ritmo", [ritmoH.el, el("span", null, " BUs/min")]);
  ritmo.style.width = "150px";
  const pausa = el("span", "chip alerta oculto", "");
  pausa.style.width = "260px";
  const alerta = el("span", "chip alerta oculto", "");
  const sim = el("button", "chip sim oculto", "");
  sim.type = "button";
  sim.addEventListener("click", () => explicarSimulacao());
  const playlist = el("a", "chip playlist oculto", ["▶ Playlist", el("span", "longo", " de simulados · escolha o próximo")]);
  playlist.href = "/playlist";
  $2("#faixa").replaceChildren(sim, playlist, ritmo, pausa, alerta);
  faixa = { ritmoH, ritmo, pausa, alerta, sim, playlist };
}
function explicarSimulacao() {
  const rot = S.estado?.simulacao?.rotulo;
  if (!rot) return;
  const link = el("a", null, "Playlist de simulados: agenda e votação do próximo");
  link.href = "/playlist";
  folha.abrir({ titulo: "Dados reais: isto é uma retotalização", linhas: [
    ["Os dados", "Os boletins de urna reais, publicados pelo TSE. Nenhum voto é inventado."],
    ["O que se repete", "A totalização: o Urna Velox começa do zero e soma tudo de novo, na ordem em que os boletins chegaram à Justiça Eleitoral."],
    ["Para quê", "Ver a totalização e a auditoria acontecendo: no fim, o resultado é conferido com o oficial do TSE."],
    ["Agora", rot],
    ["A seguir", link]
  ] });
}
function talvezExplicarSimulacao() {
  if (S.simExplicada || !S.estado?.simulacao?.rotulo) return;
  S.simExplicada = true;
  const hoje = (/* @__PURE__ */ new Date()).toISOString().slice(0, 10);
  let visto = null;
  try {
    visto = localStorage.getItem("uv-sim-explicada");
  } catch {
  }
  if (visto === hoje) return;
  try {
    localStorage.setItem("uv-sim-explicada", hoje);
  } catch {
  }
  explicarSimulacao();
}
function desenharEstado() {
  S.estadoSujo = false;
  const e = S.estado;
  const c = $2("#conexao");
  let est = S.conexao, txt = "conectando…";
  if (S.conexao === "vivo") {
    txt = "ao vivo";
  }
  if (S.conexao === "caiu") {
    const s = Math.max(0, Math.round(((S.conexaoInfo?.em || Date.now()) - Date.now()) / 1e3));
    txt = `conexão caiu · volta em ${s}s`;
    clearTimeout(S.tCaiu);
    S.tCaiu = setTimeout(() => {
      S.estadoSujo = true;
      pedirQuadro();
    }, 1e3);
  }
  if (S.conexao === "vivo" && e?.pausa_429?.ativa) {
    est = "pausa";
    txt = "TSE pediu pausa";
  }
  if (S.conexao === "vivo" && e?.encerrada) {
    est = "encerrada";
    txt = "apuração encerrada";
  }
  if (c.dataset.estado !== est) c.dataset.estado = est;
  const t = c.querySelector(".txt");
  if (t.textContent !== txt) t.textContent = txt;
  const rot = e?.simulacao?.rotulo || "";
  if (c.classList.contains("sim") !== !!rot) c.classList.toggle("sim", !!rot);
  let comp = c.querySelector(".comp");
  if (rot && !comp) {
    comp = el("span", "comp", "");
    c.appendChild(comp);
  }
  if (comp) {
    const ct = rot ? "simulação" : "";
    if (comp.textContent !== ct) comp.textContent = ct;
  }
  const dica = rot ? `Simulação com dados reais: os boletins de urna são os publicados pelo TSE, totalizados de novo, do zero, na ordem em que chegaram à Justiça Eleitoral (${rot}); no fim, o resultado é conferido com o oficial. O relógio mostra o horário da noite da eleição.` : "";
  if (c.title !== dica) c.title = dica;
  const st = rot ? `Dados reais do TSE · retotalização · ${rot}` : "";
  if (faixa.sim.textContent !== st) {
    faixa.sim.replaceChildren(...rot ? [el("b", null, ["Dados reais", el("span", "tse", " do TSE")]), el("span", "resto", ` · retotalização · ${rot}`)] : []);
    faixa.sim.title = dica;
    faixa.sim.classList.toggle("oculto", !st);
    faixa.sim.setAttribute("aria-label", rot ? `Por que é simulação e os dados são reais: ${rot}` : "");
    faixa.playlist.classList.toggle("oculto", !st);
  }
  if (rot) talvezExplicarSimulacao();
  if (!e) return;
  const pz = e.pausa_429?.ativa ? `Pausa de rede (HTTP 429)${e.pausa_429.retomada_em ? " até " + hora(e.pausa_429.retomada_em) : ""}` : "";
  if (faixa.pausa.textContent !== pz) {
    faixa.pausa.textContent = pz;
    faixa.pausa.title = pz;
    faixa.pausa.classList.toggle("oculto", !pz);
  }
  const al = e.alertas || {};
  const avisos = [];
  if (al.secoes_esperadas_incoerentes) avisos.push("o % de seções de algumas UFs pode estar errado (seções esperadas incoerentes)");
  if (al.destinacao_presumida) avisos.push('destinação dos votos presumida "Válido"');
  if (al.reenvio_diferente || al.anulacao) avisos.push("houve BU reenviado ou seção anulada");
  const at = avisos.length ? `Atenção: ${avisos.join("; ")}` : "";
  if (faixa.alerta.textContent !== at) {
    faixa.alerta.textContent = at;
    faixa.alerta.title = at;
    faixa.alerta.classList.toggle("oculto", !at);
  }
}
function curiosidades() {
  const agora = performance.now();
  if (S.seqMax != null) S.ritmo.push([agora, S.seqMax]);
  while (S.ritmo.length > 2 && agora - S.ritmo[0][0] > 6e4) S.ritmo.shift();
  if (S.ritmo.length >= 2) {
    const [t0, s0] = S.ritmo[0], [t1, s1] = S.ritmo[S.ritmo.length - 1];
    faixa.ritmoH.definir(t1 > t0 ? Math.round((s1 - s0) * 6e4 / Math.max(5e3, t1 - t0)) : 0);
  } else faixa.ritmoH.definir(0);
  if (S.primeirosSujo) {
    S.primeirosSujo = false;
    for (const it of primeiros(S.N, S.eventos.values())) porNoFeed(it);
  }
}
var itensFaltam = /* @__PURE__ */ new Map();
function feedFaltam() {
  const tela = S.ui?.circ && S.tela ? `${S.ui.circ}-c${cargo4(S.tela.cargo)}` : null;
  const abertos = [...S.faltam.values()].filter((f2) => f2.faltam != null && f2.faltam > 0);
  abertos.sort((a, b) => a.faltam - b.faltam);
  const quero = new Set(abertos.slice(0, 8).map((f2) => f2.topico));
  if (S.faltam.get("br-c0001")?.faltam > 0) quero.add("br-c0001");
  if (tela && S.faltam.get(tela)?.faltam > 0) quero.add(tela);
  for (const t of quero) {
    const f2 = S.faltam.get(t);
    let x = itensFaltam.get(t);
    const d = S.N.disputa(f2.abrangencia, f2.cargo);
    if (!x || !feed.porId.has(`f:${t}`)) {
      const res = S.res.get(t);
      const elei = S.N.cand.get(t) ? Math.max(res?.eleitorado || 0, f2.restante || 0, f2.faltam) : Math.max(f2.restante || 0, f2.faltam);
      const hod = new Hodometro({ inteiros: digitos(elei), regressivo: true });
      x = { hod };
      itensFaltam.set(t, x);
      const ab = f2.abrangencia;
      const curto = ab === "br" ? S.N.cargo(f2.cargo) : ab.length === 2 && ab !== "zz" ? `${S.N.cargo(f2.cargo)} ${ab.toUpperCase()}` : d;
      feed.por({ id: `f:${t}`, fixo: true, ic: "↓", hod, texto: `${curto}: faltam`, t: "eleitores para o matematicamente", detalhe: null });
    }
    x.hod.definir(f2.faltam);
    const it = feed.porId.get(`f:${t}`);
    if (it) it.sair = false;
    if (it) it.detalhe = { titulo: `${d}: eleitores que faltam para o matematicamente`, linhas: [
      ["Faltam apurar", mil(f2.faltam)],
      ["Eleitores ainda não apurados", mil(f2.restante)],
      ["Como se conta", "quantos dos eleitores restantes ainda precisam ser apurados, votando no pior caso, até o resultado não poder mais mudar"],
      ["Atualizado no BU nº", mil(f2.seq)]
    ] };
  }
  for (const [t, x] of itensFaltam) {
    if (quero.has(t)) continue;
    if (S.faltam.get(t)?.faltam != null) x.hod.definir(S.faltam.get(t).faltam);
    const it = feed.porId.get(`f:${t}`);
    if (it) it.sair = true;
    else itensFaltam.delete(t);
  }
}
async function iniciar() {
  const tema = (() => {
    try {
      return localStorage.getItem("uv-tema");
    } catch {
      return null;
    }
  })();
  if (tema) document.documentElement.dataset.tema = tema;
  $2("#tema").addEventListener("click", () => {
    const t = document.documentElement.dataset.tema === "claro" ? "escuro" : "claro";
    document.documentElement.dataset.tema = t;
    try {
      localStorage.setItem("uv-tema", t);
    } catch {
    }
  });
  folha = new Folha($2("#folha"));
  feed = new Feed($2("#feed"), (d) => folha.abrir(d));
  hist = new Historico($2("#hist"), $2("#feed-hist"));
  const fonte = document.fonts.load('700 20px "Archivo UV"').catch(() => null);
  const [nomes, topicos, player] = await Promise.all([
    fetch("/api/v1/nomes").then((r) => r.ok ? r.json() : null).catch(() => null),
    fetch("/api/v1/topicos").then((r) => r.ok ? r.json() : []).catch(() => []),
    // barra do replay só no site simulado; montada (e o espaço dela reservado) antes do primeiro desenho
    iniciarPlayer(),
    fonte
  ]);
  S.player = player;
  await document.fonts.ready;
  montarFaixa();
  S.ident = new Identificacao();
  S.N = new Nomes(nomes);
  S.topicos = new Set(topicos.map((t) => t.topico));
  S.con = new Conexao(receber, estadoConexao);
  window.addEventListener("hashchange", montarTela);
  document.addEventListener("visibilitychange", () => {
    if (S.comparar !== "uv") atualizarTopicos();
  });
  setInterval(() => {
    if (S.comparar !== "uv") pedirQuadro();
  }, 1e3);
  montarTela();
  S.con.abrir();
}
iniciar();
