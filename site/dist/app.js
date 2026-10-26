// Urna Velox — página de resultados (gerado por site/build.mjs a partir de site/src; não edite aqui)

// src/conexao.js
var MAX_POR_MSG = 90;
var FALHAS_PARA_HTTP = 3;
var VIDA_MINIMA_MS = 3e4;
var CONSULTA_MS = 6e4;
var NOVO_WS_MS = 12e4;
var BASE_PAUSA_MS = 10 * 6e4;
var variar = (ms) => ms * (0.8 + Math.random() * 0.4);
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
    this.falhas = 0;
    this.http = false;
    this.tHttp = null;
    this.etag = /* @__PURE__ */ new Map();
    this.ultSeq = /* @__PURE__ */ new Map();
    this.baseForaAte = 0;
    this.mapa = null;
    this.aoConsultar = null;
    const o = globalThis.__uvConexao || {};
    this.consultaMs = o.consultaMs || CONSULTA_MS;
    this.novoWsMs = o.novoWsMs || NOVO_WS_MS;
    const meta = typeof document !== "undefined" ? document.querySelector('meta[name="uv-dados"]')?.content : "";
    this.base = String(o.base ?? meta ?? "").trim().replace(/\/+$/, "");
    if (this.base && typeof location !== "undefined" && this.base === location.origin) this.base = "";
    try {
      this.forcarHttp = new URLSearchParams(location.search).get("modo") === "http";
    } catch {
      this.forcarHttp = false;
    }
  }
  url() {
    const p = location.protocol === "https:" ? "wss:" : "ws:";
    return `${p}//${location.host}/ws`;
  }
  abrir() {
    clearTimeout(this.timer);
    if (!this.http) this.aoEstado("conectando");
    if (this.forcarHttp) return this.#falhou();
    let ws;
    try {
      ws = new WebSocket(this.url());
    } catch {
      return this.#falhou();
    }
    this.ws = ws;
    let aberta = 0;
    ws.onopen = () => {
      aberta = Date.now();
      this.espera = 1e3;
      if (this.http) this.#sairHttp();
      this.aoEstado("vivo");
      this.#mandarInscricoes([...this.topicos]);
      this.#mandar({ op: "eventos", escopo: "uf" });
      this.#mandar({ op: "marcos", escopo: "uf", faltam: this.faltam });
      if (this.mapa) this.#mandar({ op: "mapa", ...this.mapa() });
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
      if (aberta && Date.now() - aberta >= VIDA_MINIMA_MS) this.falhas = 0;
      this.#falhou();
    };
    ws.onerror = () => {
    };
  }
  /** Uma recusa ou queda do WebSocket: reconecta com espera crescente ou, depois de algumas seguidas, modo HTTP. */
  #falhou() {
    this.falhas++;
    if (this.http) return this.#agendarWs();
    if (this.falhas >= FALHAS_PARA_HTTP) return this.#entrarHttp();
    this.#agendar();
  }
  #agendar() {
    const t = Math.min(3e4, this.espera) * (0.75 + Math.random() * 0.5);
    this.espera = Math.min(3e4, this.espera * 2);
    this.aoEstado("caiu", { em: Date.now() + t });
    this.timer = setTimeout(() => this.abrir(), t);
  }
  // ---------------------------------------------------------------- modo HTTP (B-026)
  #entrarHttp() {
    this.http = true;
    this.aoEstado("http", { consultaMs: this.consultaMs });
    this.#consultar();
    this.#agendarWs();
  }
  #sairHttp() {
    this.http = false;
    clearTimeout(this.tHttp);
    this.tHttp = null;
  }
  #agendarWs() {
    clearTimeout(this.timer);
    this.timer = setTimeout(() => this.abrir(), variar(this.novoWsMs));
  }
  /** Tópicos consultáveis por HTTP: os da tela, sem os da comparação com o TSE (`tse:…`, só pelo WebSocket). */
  #topicosHttp(lista) {
    return lista.filter((t) => !t.includes(":"));
  }
  async #consultar() {
    clearTimeout(this.tHttp);
    if (!this.http) return;
    await Promise.all([this.#buscar(null), ...this.#topicosHttp([...this.topicos]).map((t) => this.#buscar(t))]);
    if (!this.http) return;
    try {
      this.aoConsultar?.();
    } catch {
    }
    clearTimeout(this.tHttp);
    this.tHttp = setTimeout(() => this.#consultar(), variar(this.consultaMs));
  }
  /** Um GET: estado (`topico` null) ou resultado do tópico. Base das consultas primeiro; se falhar, o próprio domínio. */
  async #buscar(topico) {
    const caminho = topico ? `/api/v1/resultado/${encodeURIComponent(topico)}` : "/api/v1/estado";
    let r = null;
    if (this.base && Date.now() >= this.baseForaAte) {
      try {
        r = await fetch(this.base + caminho, { cache: "no-cache", credentials: "omit" });
        if (r.status >= 500) {
          this.baseForaAte = Date.now() + BASE_PAUSA_MS;
          r = null;
        } else if (!r.ok) r = null;
      } catch {
        this.baseForaAte = Date.now() + BASE_PAUSA_MS;
        r = null;
      }
    }
    if (!r) {
      const et = topico ? this.etag.get(topico) : null;
      try {
        r = await fetch(caminho, { cache: "no-store", headers: et ? { "If-None-Match": et } : {} });
      } catch {
        return;
      }
      if (r.status === 304 || !r.ok) return;
      if (topico && r.headers.get("ETag")) this.etag.set(topico, r.headers.get("ETag"));
    }
    if (!this.http) return;
    let txt;
    try {
      txt = await r.text();
    } catch {
      return;
    }
    if (!topico) {
      let m;
      try {
        m = JSON.parse(txt);
      } catch {
        return;
      }
      if (m && typeof m === "object") this.aoReceber({ ...m, tipo: "estado" });
      return;
    }
    const s = /"seq":(\d+)/.exec(txt.slice(0, 400));
    if (!s || this.ultSeq.get(topico) === +s[1]) return;
    this.ultSeq.set(topico, +s[1]);
    this.aoReceber({ tipo: "resultado", topico, seq: +s[1], txt: `{"tipo":"instantaneo","topico":${JSON.stringify(topico)},"seq":${s[1]},"resultado":${txt}}` });
  }
  /** GET de leitura (mapa das seções): base das consultas primeiro (cache da borda); se falhar, o próprio domínio. */
  async obter(caminho) {
    if (this.base && Date.now() >= this.baseForaAte) {
      try {
        const r = await fetch(this.base + caminho, { credentials: "omit" });
        if (r.ok) return r;
        if (r.status >= 500) this.baseForaAte = Date.now() + BASE_PAUSA_MS;
      } catch {
        this.baseForaAte = Date.now() + BASE_PAUSA_MS;
      }
    }
    try {
      const r = await fetch(caminho);
      return r.ok ? r : null;
    } catch {
      return null;
    }
  }
  /** Liga o mapa das seções: `f()` dá `{desde, epoca}` (pedido agora e a cada reconexão). */
  pedirMapa(f2) {
    this.mapa = f2;
    this.#mandar({ op: "mapa", ...f2() });
  }
  // ---------------------------------------------------------------- inscrições
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
    if (this.http) for (const t of this.#topicosHttp(entrar)) this.#buscar(t);
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
  * `prefixo`: um sinal ("+", "−") antes do número, sempre colado no primeiro dígito à vista (B-031): o número fica no
  * lugar (alinhado à direita) e só o sinal desliza para a direita, por `transform`, o tanto dos dígitos vazios.
  * `arrastarEl` (propriedade, depois de montar): um elemento em volta do número (a linha "312.954 de 499.248", o
  * conteúdo de um chip) que desliza no lugar dele, para o que vem depois do número andar junto (B-041); com `centro`,
  * metade.
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
    if (op.prefixo) {
      this.pre = el("span", "pre sinal", "");
      vis.append(this.pre);
    }
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
  /** Enxuto: desliza o número (só `transform`) para os dígitos vazios à esquerda não aparecerem como espaço. Com
   *  `prefixo`, é o sinal que desliza (para a direita), até o primeiro dígito à vista. */
  #deslizar() {
    let w = 0;
    for (const x of this.vis.children) {
      if (x === this.pre) continue;
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
    const alvo = this.arrastarEl || (this.op.enxuto ? this.vis : this.pre);
    if (primeira) alvo.style.transition = "none";
    alvo.style.transform = w ? `translateX(${alvo === this.pre ? w : -w}px)` : "";
    if (primeira) {
      void alvo.offsetWidth;
      alvo.style.transition = "";
    }
  }
  /** B-041: o elemento que desliza no lugar do número; medido no quadro seguinte (já no documento), mesmo sem valor novo. */
  set arrastarEl(e) {
    this.arrastar = e;
    this.medido = false;
    requestAnimationFrame(() => this.#deslizar());
  }
  get arrastarEl() {
    return this.arrastar;
  }
  /** Sinal antes do número (só com `prefixo`). */
  prefixar(t) {
    if (this.pre && this.pre.textContent !== t) this.pre.textContent = t;
  }
  /** Valor novo (número; texto no modo `molde`; `null` = desconhecido). */
  definir(v) {
    if (v === this.valor) {
      if ((this.op.enxuto || this.op.prefixo || this.arrastarEl) && !this.medido) this.#deslizar();
      return;
    }
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
      if (this.op.enxuto || this.op.prefixo || this.arrastarEl) this.#deslizar();
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
   * Acrescenta (ou atualiza, pelo id) um item. `it` = { id, ic, texto, t, detalhe, fixo, hod, anunciar, cls }.
   * `fixo`: item de largura fixa cujo conteúdo muda (ex.: eleitores que faltam), atualizado no lugar.
   */
  por(it) {
    const ja = this.porId.get(it.id);
    if (ja) {
      if (it.texto != null && ja.txt.textContent !== it.texto) {
        ja.txt.textContent = it.texto;
        const w = ja.el.offsetWidth, d = w - ja.w;
        if (d) {
          for (const i of this.itens) if (i !== ja && i.x > ja.x) i.x += d;
          ja.w = w;
        }
      }
      if (it.t != null) ja.t.textContent = it.t;
      ja.detalhe = it.detalhe;
      return ja;
    }
    const b = el("button", "item" + (it.fixo ? " fixo" : "") + (it.cls ? ` ${it.cls}` : ""));
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
    this.acoes = el("div", "folha-acoes");
    this.acoes.hidden = true;
    raiz.append(this.acoes);
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
    this.acoes.replaceChildren(...(d.acoes || []).map(([rot, fn, marcado]) => {
      const b = el("button", marcado ? "ativo" : null, rot);
      b.type = "button";
      b.addEventListener("click", () => {
        fn();
        this.fechar();
      });
      return b;
    }));
    this.acoes.hidden = !(d.acoes && d.acoes.length);
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
var PREP_UF = {
  ac: "no",
  al: "em",
  am: "no",
  ap: "no",
  ba: "na",
  ce: "no",
  df: "no",
  es: "no",
  go: "em",
  ma: "no",
  mg: "em",
  ms: "em",
  mt: "em",
  pa: "no",
  pb: "na",
  pe: "em",
  pi: "no",
  pr: "no",
  rj: "no",
  rn: "no",
  rs: "no",
  ro: "em",
  rr: "em",
  sc: "em",
  se: "em",
  sp: "em",
  to: "no",
  zz: "no"
};
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
  /** "no Brasil", "na Bahia", "em São Paulo", "em Manaus (AM)" */
  em(cod) {
    const a = abr(cod);
    if (a.tipo === "br") return "no Brasil";
    if (a.tipo === "uf") return `${PREP_UF[a.uf] || "em"} ${this.uf(a.uf)}`;
    return `em ${this.local(cod)}`;
  }
  cargo(c) {
    return NOME_CARGO[c] || (this.cargos.get(c)?.consulta || c >= 25 ? "Consulta" : `Cargo ${c}`);
  }
  /** "Governador em São Paulo", "Governador na Bahia", "Presidente", "Prefeito de Manaus (AM)" */
  disputa(cod, c) {
    const a = abr(cod);
    const nome = this.cargo(c);
    if (a.tipo === "br") return nome;
    if (a.tipo === "uf") return `${nome} ${this.em(cod)}`;
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
      const onde = m.abrangencia === "br" ? "do Brasil" : N.em(m.abrangencia);
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
          texto: `Primeiro ${nome} eleito ${N.em(a.uf)}: ${quem}, ${N.local(e.abrangencia)}`,
          detalhe: { titulo: `Primeiro ${nome} matematicamente eleito ${N.em(a.uf)}`, linhas: [["Eleito", quem], ["Onde", N.local(e.abrangencia)], ["Horário (Urna Velox)", t]] }
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
  if (sim) linhas.push(["TSE no relógio do sistema", `${horaDe(cmp.tse)} — a noite do TSE ${anc.x !== 1 ? `a ${String(anc.x).replace(".", ",")}× ` : ""}a partir do início deste replay`]);
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
    this.el.setAttribute("aria-label", "Controle do replay");
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
    this.play.setAttribute("aria-label", pausado ? "Continuar o replay" : "Pausar o replay");
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
var MODOS = [["uv", "Urna Velox"], ["ambos", "Comparar"], ["tse", "TSE oficial"]];
var EXPLICA = {
  uv: "Boletins somados assim que o TSE os publica",
  ambos: "Lado a lado: veja quem está à frente",
  tse: "Só o arquivo oficial do TSE (≈ 1 min)"
};
var EXPLICA_SEM_TSE = { ambos: "Sem totais no TSE ainda; só o Urna Velox", tse: "Ainda sem totais no TSE: veja o Urna Velox" };
var DICA_TEXTO = "Toque em Comparar para conferir com o TSE";
var DICA_MS = 6e3;
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
  const difH = new Hodometro({ inteiros: dig, prefixo: true });
  const pos = el("span", "pos-tse", "");
  pos.title = "Posição no TSE";
  const tse = el("span", "tse", [el("span", "tse-l1", [pos, el("span", "tse-pct", pctH.el)]), el("span", "tse-qtd", votH.el)]);
  const dif = el("span", "dif", [el("span", "dif-rot", "UV − TSE"), el("span", "dif-val", difH.el)]);
  dif.title = "Diferença de votos: Urna Velox − TSE";
  return { tse, dif, pos, pctH, votH, difH, ultPos: null, ultSinal: null };
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
    c.difH.prefixar(s);
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
  const semTotais = !!d && st === 0;
  const arq = d?.geracao ? `arquivo do TSE das ${hora(d.geracao)}${sim ? ` (≙ ${horaDe(eq)} aqui)` : ""}${agora != null && eq != null ? ` · ${idade(agora - eq)}` : ""}` : "";
  const b = semTotais ? "O TSE ainda não começou a totalizar" : arq;
  const x = semTotais && EXPLICA_SEM_TSE[modo] || EXPLICA[modo];
  if (modo === "uv") return { a: uv, b: "", x };
  if (modo === "tse") return { a: tTse + (d ? " das seções" : ""), b, x };
  return { a: `${uv} · ${tTse}`, b, x };
}
var INFO_FONTE = { titulo: "De onde vêm os números", linhas: [
  ["Urna Velox", "Somamos cada boletim de urna (BU) assim que o TSE o publica. Por isso os números aparecem antes e andam a cada boletim."],
  ["TSE oficial", "O arquivo de resultados que o próprio TSE divulga, atualizado a cada cerca de 1 minuto. É o número oficial, mas chega um pouco depois."],
  ["Comparar", "Mostra as duas fontes lado a lado, com a diferença entre elas. Normalmente o Urna Velox está um pouco à frente do TSE."],
  ["Antes das 17h", "O TSE só começa a totalizar às 17h. Até lá, só o Urna Velox tem números: os dos eleitores do exterior."]
] };
var CHAVE_DICA = "uv-dica-fonte";
function dicaVista() {
  try {
    return localStorage.getItem(CHAVE_DICA) === "1";
  } catch {
    return true;
  }
}
function marcarDica() {
  try {
    localStorage.setItem(CHAVE_DICA, "1");
  } catch {
  }
}
function medirUso(evento) {
  try {
    if (navigator.sendBeacon) navigator.sendBeacon(`/api/v1/uso/${evento}`, "");
  } catch {
  }
}

// src/previsao.js
var comSegundos = false;
var hm2 = (t) => {
  const m = /(\d{2}):(\d{2}):(\d{2})/.exec(t || "");
  return m ? comSegundos ? `${m[1]}:${m[2]}:${m[3]}` : `${m[1]}:${m[2]}` : "—";
};
var faixaTxt = (f2) => f2 ? !f2.incerta ? `entre ${hm2(f2.p10)} e ${hm2(f2.p90)}` : f2.eta ? `por volta de ${hm2(f2.eta)}` : "em horário ainda incerto" : "";
var provavel = (f2) => !f2 ? null : !f2.incerta ? f2.p50 : f2.eta || null;
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
    this.hodCaixa = el("div", "pv-hod", [this.hod.el, el("small", null, "mais provável")]);
    this.el = el("section", "vidro previsao", [
      el("h2", "cartao-titulo", ["Previsão ", el("small", null, "(estimativa de horário)")]),
      el("div", "pv-topo", [el("div", "pv-txt", [this.rot, this.faixa, this.nota]), this.hodCaixa]),
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
    this.hodCaixa.classList.toggle("seg", comSegundos);
    const molde = comSegundos ? "##:##:##" : "##:##";
    if (this.hod.molde !== molde) {
      const novo = new Hodometro({ molde });
      this.hod.el.replaceWith(novo.el);
      this.hod = novo;
      this.hod.molde = molde;
    }
    this.chave = "";
  }
  /** p = última previsão do tópico (ou null); circ = código da circunscrição do tópico; nomeCirc = lugar com a preposição
   *  ("no Brasil", "na Bahia"; B-042). */
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
        if (m.incerta && m.eta) nota = `pelo ritmo atual · ${nota}`;
        hod = provavel(m) && hm2(provavel(m));
      } else if (!ehCirc) {
        rot = "O matematicamente";
        faixa2 = `se decide ${nomeCirc}`;
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
    this.hodCaixa.classList.toggle("vazio", !hod);
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

// src/mapa.js
var PREF = "uv-mapa";
var ler = () => {
  try {
    return JSON.parse(localStorage.getItem(PREF)) || {};
  } catch {
    return {};
  }
};
var gravar = (o) => {
  try {
    localStorage.setItem(PREF, JSON.stringify(o));
  } catch {
  }
};
var rad = Math.PI / 180;
var vet = (lat, lon) => [Math.cos(lat * rad) * Math.cos(lon * rad), Math.cos(lat * rad) * Math.sin(lon * rad), Math.sin(lat * rad)];
var BSB = vet(-15.79, -47.88);
var GRAD = [[79, 195, 255], [140, 107, 255], [255, 95, 210]];
function corFaixa(f2, a) {
  const min = (f2 - 1) * 6 - 960;
  const x = Math.max(0, Math.min(1, min / 300)) * 2, i = Math.min(1, Math.floor(x)), u = x - i;
  const c = GRAD[i].map((v, j) => Math.round(v + (GRAD[i + 1][j] - v) * u));
  return `rgba(${c[0]},${c[1]},${c[2]},${a})`;
}
function hash(a, b, c) {
  let h = Math.imul(a ^ 2654435769, 2246822507) ^ Math.imul(b + 1663821227, 3266489909) ^ Math.imul(c + 668265263, 374761393);
  h ^= h >>> 15;
  h = Math.imul(h, 739982445);
  h ^= h >>> 12;
  h = Math.imul(h, 695872825);
  h ^= h >>> 15;
  return (h >>> 0) / 4294967296;
}
var normNome = (s) => String(s || "").normalize("NFD").replace(/[̀-ͯ]/g, "").toUpperCase().replace(/[-']/g, " ").replace(/\s+/g, " ").trim();
var CartaoMapa = class {
  /**
   * @param {{obter: (caminho: string, op?: object) => Promise<Response|null>, pedirMapa: (f: () => object) => void, nomes: () => any,
   *          versao: string, aoMarco?: (txt: string) => void}} op
   */
  constructor(op) {
    this.op = op;
    const p = ler();
    this.vista = p.vista === "globo" ? "globo" : "brasil";
    this.cor = p.cor === "hora" ? "hora" : "luz";
    this.reduzido = movimentoReduzido();
    this.fx = [];
    this.arcos = [];
    this.marcos = [];
    this.marcoAte = 0;
    this.carregado = false;
    this.visivel = false;
    this.semPosicao = [];
    this.G = { lam: -50, phi: -14, zoom: 1, vl: 0, vp: 0, mexeu: -1e9, base: -50, arr: null };
    this.#montar();
  }
  // ---------------------------------------------------------------- DOM
  #montar() {
    const botoes = (rotulo, itens, atual, aoMudar) => {
      const g = el("div", "mp-modo");
      g.setAttribute("role", "group");
      g.setAttribute("aria-label", rotulo);
      const bs = itens.map(([v, t]) => {
        const b = el("button", null, t);
        b.type = "button";
        b.dataset.v = v;
        b.setAttribute("aria-pressed", String(v === atual));
        b.addEventListener("click", () => {
          bs.forEach((x) => x.setAttribute("aria-pressed", String(x === b)));
          aoMudar(v);
        });
        return b;
      });
      g.append(...bs);
      return g;
    };
    this.hodCaixa = el("span", "mp-n", "—");
    this.deTxt = el("small", null, "");
    this.extTxt = el("div", "mp-ext", "");
    const acesas = el("div", "mp-acesas", [el("div", "mp-rot", "Seções acesas"), el("div", "mp-val", [this.hodCaixa, this.deTxt]), this.extTxt]);
    const ctl = el("div", "mp-ctl", [
      botoes("Vista do mapa", [["brasil", "Brasil"], ["globo", "Exterior"]], this.vista, (v) => {
        this.vista = v;
        this.#salvar();
        this.#aplicarVista();
      }),
      botoes("Cor dos pontos", [["luz", "Luz"], ["hora", "Horário"]], this.cor, (v) => {
        this.cor = v;
        this.#salvar();
        this.legenda.classList.toggle("ligado", v === "hora");
        this.#redesenhar();
      })
    ]);
    this.cv = {};
    const palco = el("div", "mp-palco");
    for (const n of ["base", "acc", "globo", "brilho", "fx"]) {
      const c = el("canvas", `mp-${n}`);
      c.setAttribute("aria-hidden", "true");
      this.cv[n] = c;
      palco.append(c);
    }
    this.cv.fx.removeAttribute("aria-hidden");
    this.cv.fx.setAttribute("role", "img");
    this.legenda = el("div", "mp-legenda", [el("i"), el("span", null, [el("em", null, "16h"), el("em", null, "18h30"), el("em", null, "21h")])]);
    this.legenda.classList.toggle("ligado", this.cor === "hora");
    this.chip = el("div", "mp-marco", "");
    this.aviso = el("div", "mp-aviso", "Carregando o mapa…");
    palco.append(this.legenda, this.chip, el("div", "mp-dica", "arraste para girar · duplo clique volta ao Brasil"), this.aviso);
    this.palco = palco;
    this.ultimo = el("div", "mp-ultimo", [el("span", "mp-pt"), el("span", "mp-ult-txt", "Aguardando o primeiro BU…")]);
    this.ultTxt = this.ultimo.lastChild;
    this.ufs = el("div", "mp-ufs", el("h3", null, "Seções acesas por UF"));
    this.corpo = el("div", "mp-corpo", [el("div", "mp-topo", [acesas, ctl]), palco, this.ultimo, this.ufs]);
    this.el = el("section", "vidro mapa", [el("h2", "cartao-titulo", "Mapa das seções"), this.corpo]);
    this.el.setAttribute("aria-label", "Mapa das seções");
    this.#aplicarVista();
    this.#interacao();
    this.io = new IntersectionObserver((es) => {
      this.visivel = es.some((e) => e.isIntersecting) || this.el.classList.contains("max");
      if (this.visivel) {
        this.#carregar();
        this.#pedirQuadro();
      }
    }, { rootMargin: "200px" });
    this.io.observe(this.el);
    this.ro = new ResizeObserver(() => this.#redesenhar());
    this.ro.observe(palco);
  }
  #salvar() {
    gravar({ vista: this.vista, cor: this.cor });
  }
  #aplicarVista() {
    this.palco.classList.toggle("globo", this.vista === "globo");
    this.cv.fx.setAttribute("aria-label", this.vista === "globo" ? "Globo com um ponto de luz para cada seção totalizada, no Brasil e no exterior; arraste para girar" : "Mapa do Brasil com um ponto de luz para cada seção totalizada");
    this.#atualizarBrilho();
    this.#pedirQuadro();
  }
  /** O cartão foi maximizado ou restaurado (cartoes.js): desenha mesmo fora da área visível e mostra as UFs. */
  maximizado(sim) {
    this.visivel = sim || this.visivel;
    if (sim) {
      this.#carregar();
      this.#painelUfs();
    }
    this.#pedirQuadro();
  }
  // ---------------------------------------------------------------- dados
  async #json(caminho, mesmaOrigem) {
    try {
      const r = mesmaOrigem ? await fetch(caminho) : await this.op.obter(caminho);
      return r && r.ok ? await r.json() : null;
    } catch {
      return null;
    }
  }
  async #carregar() {
    if (this.carregando) return;
    this.carregando = true;
    const [geo, lista, est] = await Promise.all([this.#json(`/static/mapa.json?v=${this.op.versao}`, true), this.#json("/api/v1/mapa/secoes"), this.#json("/api/v1/mapa/estado")]);
    if (!geo || !lista || !est) {
      this.aviso.textContent = "O mapa das seções não está disponível neste servidor.";
      this.carregando = false;
      setTimeout(() => {
        if (!this.carregado) this.#carregar();
      }, 6e4);
      return;
    }
    this.geo = geo;
    this.#decodificar(lista);
    this.#posicionar();
    this.#fotografia(est);
    this.carregado = true;
    this.aviso.classList.add("oculto");
    const hod = new Hodometro({ inteiros: String(this.N).length, enxuto: true });
    this.hod = hod;
    this.hodCaixa.replaceChildren(hod.el);
    this.#redesenhar();
    this.#textos();
    this.op.pedirMapa(() => ({ desde: this.n, epoca: this.epoca }));
  }
  /** `GET /api/v1/mapa/secoes`: índice → município (posição na tabela), zona, seção; chave ordenada para buscas. */
  #decodificar(j) {
    const N = j.n, m = j.m;
    this.N = N;
    this.mu = new Uint16Array(N);
    this.zona = new Uint16Array(N);
    this.secao = new Uint16Array(N);
    this.munCod = [];
    this.munUf = [];
    this.munN = [];
    let i = 0, p = 0;
    while (p < m.length && i < N) {
      const cod = m[p++], uf = j.ufs[m[p++]], nz = m[p++], mi = this.munCod.length;
      this.munCod.push(cod);
      this.munUf.push(uf);
      const i0 = i;
      for (let z = 0; z < nz; z++) {
        const zona = m[p++], ns = m[p++];
        let s = 0;
        for (let k = 0; k < ns; k++) {
          s = k ? s + m[p++] : m[p++];
          this.mu[i] = mi;
          this.zona[i] = zona;
          this.secao[i] = s;
          i++;
        }
      }
      this.munN.push(i - i0);
    }
    this.hora = new Uint8Array(N);
  }
  /** Índice da seção (município TSE, zona, seção) por busca binária (a lista está na ordem da chave). */
  indice(cod, zona, secao) {
    const ch = (i) => this.munCod[this.mu[i]] * 4294967296 + this.zona[i] * 65536 + this.secao[i];
    const alvo = cod * 4294967296 + zona * 65536 + secao;
    let lo = 0, hi = this.N - 1;
    while (lo <= hi) {
      const md = lo + hi >> 1, v = ch(md);
      if (v === alvo) return md;
      if (v < alvo) lo = md + 1;
      else hi = md - 1;
    }
    return -1;
  }
  /** Posição de cada seção: local de votação (se houver), senão em volta do centro do município; exterior pelo nome. */
  #posicionar() {
    const g = this.geo, N = this.N;
    const centro = /* @__PURE__ */ new Map();
    for (let i = 0; i < g.mun.length; i += 3) centro.set(g.mun[i], [g.mun[i + 1] / 10, g.mun[i + 2] / 10]);
    const locais = /* @__PURE__ */ new Map();
    const L = g.locais || [];
    if (g.locais_v === 2) {
      const dv = g.locais_div || 10;
      for (let i = 0; i < L.length; ) {
        const m = L[i + 2], v = [];
        let s0 = 0;
        for (let j = 0; j < m; j++) {
          const b = i + 3 + j * 3;
          s0 += L[b];
          v.push([s0, L[b + 1] / dv, L[b + 2] / dv]);
        }
        locais.set(`${L[i]}/${L[i + 1]}`, v);
        i += 3 + m * 3;
      }
    } else {
      for (let i = 0; i < L.length; i += 6) {
        const k = `${L[i]}/${L[i + 1]}`;
        if (!locais.has(k)) locais.set(k, []);
        locais.get(k).push([L[i + 2], L[i + 4] / 10, L[i + 5] / 10]);
      }
    }
    const nomes = this.op.nomes()?.n?.municipios || {};
    this.X = new Float32Array(N);
    this.Y = new Float32Array(N);
    this.cid = new Int16Array(N).fill(-1);
    this.cidades = [];
    const cidIdx = /* @__PURE__ */ new Map();
    this.ext = new Uint8Array(N);
    this.nExt = 0;
    this.noLocal = 0;
    for (let i = 0; i < N; i++) {
      const mi = this.mu[i], cod = this.munCod[mi], uf = this.munUf[mi];
      const a = hash(cod, this.zona[i], this.secao[i]), b = hash(this.secao[i], cod, this.zona[i] + 7);
      if (uf === "zz") {
        this.ext[i] = 1;
        this.nExt++;
        const nome = nomes[`zz${String(cod).padStart(5, "0")}`] || "";
        let ci = cidIdx.get(cod);
        if (ci === void 0) {
          const ll = g.exterior[normNome(nome)];
          ci = ll ? this.cidades.length : -1;
          if (ll) this.cidades.push([nome, ll[0] / 100, ll[1] / 100, vet(ll[0] / 100, ll[1] / 100), 0]);
          else this.semPosicao.push(nome || `zz${cod}`);
          cidIdx.set(cod, ci);
        }
        this.cid[i] = ci;
        this.X[i] = -1e4;
        this.Y[i] = -1e4;
        continue;
      }
      const r = Math.sqrt(-2 * Math.log(1 - a * 0.999)), t = 2 * Math.PI * b, gx = r * Math.cos(t), gy = r * Math.sin(t);
      const lv = locais.get(`${cod}/${this.zona[i]}`);
      if (lv) {
        let lo = 0, hi = lv.length - 1, k = -1;
        while (lo <= hi) {
          const md = lo + hi >> 1;
          if (lv[md][0] <= this.secao[i]) {
            k = md;
            lo = md + 1;
          } else hi = md - 1;
        }
        if (k >= 0 && (lv[k][1] || lv[k][2])) {
          this.X[i] = lv[k][1] + gx * 0.12;
          this.Y[i] = lv[k][2] + gy * 0.12;
          this.noLocal++;
          continue;
        }
      }
      const c = centro.get(cod) || g.centros[uf] || [500, 433];
      const n = this.munN[mi];
      const sig = (hash(cod, this.secao[i], 3) < 0.18 ? 2.6 : 1) * (0.28 + Math.sqrt(n) * 0.032) + (hash(cod, this.secao[i], 3) < 0.18 ? 1.2 : 0);
      this.X[i] = c[0] + gx * sig;
      this.Y[i] = c[1] + gy * sig;
    }
    const P = g.plano, GS = 6;
    this.gc = Math.ceil(P.largura / GS);
    this.gr = Math.ceil(P.altura / GS) + 1;
    this.bin = new Uint32Array(N);
    this.binN = new Uint32Array(this.gc * this.gr);
    this.binAtivos = [];
    this.BV = new Float32Array(this.gc * this.gr * 3);
    for (let b = 0; b < this.gc * this.gr; b++) {
      const x = (b % this.gc + 0.5) * GS, y = (Math.floor(b / this.gc) + 0.5) * GS;
      this.BV.set(vet(P.lat1 - y * P.span / P.largura, P.lon0 + x * P.span / P.largura), b * 3);
    }
    for (let i = 0; i < N; i++) {
      if (this.ext[i]) continue;
      const c = Math.max(0, Math.min(this.gc - 1, Math.floor(this.X[i] / GS))), r = Math.max(0, Math.min(this.gr - 1, Math.floor(this.Y[i] / GS)));
      this.bin[i] = r * this.gc + c;
    }
    this.MV = new Float32Array(g.mundo.length / 2 * 3);
    for (let i = 0; i < g.mundo.length / 2; i++) this.MV.set(vet(g.mundo[i * 2 + 1] / 10, g.mundo[i * 2] / 10), i * 3);
    this.paths = g.ufs.map(([, d]) => new Path2D(d));
    this.brasil = new Path2D(g.ufs.map(([, d]) => d).join(""));
    this.acesasUf = /* @__PURE__ */ new Map();
    this.totUf = /* @__PURE__ */ new Map();
    for (let i = 0; i < N; i++) {
      const u = this.munUf[this.mu[i]];
      this.totUf.set(u, (this.totUf.get(u) || 0) + 1);
    }
    this.totUfLista = new Map(this.totUf);
    this.nExtLista = this.nExt;
    if (this.semPosicao.length) console.info(`mapa: ${this.semPosicao.length} cidades do exterior sem posição (fora do globo):`, this.semPosicao.join(", "));
  }
  #acender(i, f2) {
    this.hora[i] = f2;
    const u = this.munUf[this.mu[i]];
    this.acesasUf.set(u, (this.acesasUf.get(u) || 0) + 1);
    if (this.ext[i]) {
      this.extAcesas++;
      if (this.cid[i] >= 0) this.cidades[this.cid[i]][4]++;
      return;
    }
    this.brAcesas++;
    if (!this.binN[this.bin[i]]++) this.binAtivos.push(this.bin[i]);
  }
  /** `GET /api/v1/mapa/estado`: tudo o que já acendeu, de uma vez (sem explosões). */
  #fotografia(e) {
    const bin = atob(e.h);
    this.hora.fill(0);
    this.binN.fill(0);
    this.binAtivos.length = 0;
    this.acesasUf.clear();
    this.cidades.forEach((c) => {
      c[4] = 0;
    });
    this.extAcesas = 0;
    this.brAcesas = 0;
    const n = Math.min(bin.length, this.N);
    for (let i = 0; i < n; i++) {
      const f2 = bin.charCodeAt(i);
      if (f2) this.#acender(i, f2);
    }
    this.totUf = new Map(this.totUfLista);
    this.nExt = this.nExtLista;
    for (const i of e.ret || []) {
      if (i >= this.N || this.hora[i]) continue;
      const u = this.munUf[this.mu[i]];
      this.totUf.set(u, (this.totUf.get(u) || 1) - 1);
      if (this.ext[i]) this.nExt--;
    }
    this.total = e.total ?? this.N;
    this.n = e.n;
    this.epoca = e.epoca;
    this.ultimaFaixa = 0;
  }
  async #recarregar() {
    const est = await this.#json("/api/v1/mapa/estado");
    if (!est) return;
    this.#fotografia(est);
    this.#redesenhar();
    this.#textos();
    this.op.pedirMapa(() => ({ desde: this.n, epoca: this.epoca }));
  }
  /** Modo HTTP (B-026, a cada consulta): relê a fotografia e acende o que é novo. */
  async consultar() {
    if (!this.carregado) return;
    const est = await this.#json("/api/v1/mapa/estado");
    if (!est) return;
    if (est.epoca !== this.epoca || (est.total ?? this.total) !== this.total) {
      this.#fotografia(est);
      this.#redesenhar();
      this.#textos();
      return;
    }
    if (est.n <= this.n) return;
    const bin = atob(est.h), novas = [];
    for (let i = 0; i < Math.min(bin.length, this.N); i++) {
      const f2 = bin.charCodeAt(i);
      if (f2 && !this.hora[i]) {
        this.#acender(i, f2);
        novas.push(i);
      }
    }
    this.n = est.n;
    if (!novas.length) return;
    this.#pintar(novas);
    this.ult = novas[novas.length - 1];
    this.brilhoSujo = true;
    this.textosSujos = true;
    this.#pedirQuadro();
  }
  /** Mensagem `mapa` do WebSocket. */
  receber(m) {
    if (!this.carregado) return;
    if (m.reiniciar || m.epoca != null && m.epoca !== this.epoca || m.de > this.n || (m.total ?? this.total) !== this.total) {
      this.#recarregar();
      return;
    }
    if (m.n <= this.n) return;
    const s = m.s, ini = (this.n - m.de) * 2, novas = [];
    for (let k = ini; k < s.length; k += 2) {
      const i = s[k], f2 = s[k + 1];
      if (i >= this.N || this.hora[i]) continue;
      this.#acender(i, f2);
      novas.push(i);
    }
    this.n = m.n;
    if (!novas.length) return;
    this.#pintar(novas);
    const agora = performance.now();
    const p = this.reduzido ? 0 : Math.min(1, 30 / novas.length);
    for (const i of novas) {
      if (this.ext[i]) {
        if (!this.reduzido && this.cid[i] >= 0) this.arcos.push({ c: this.cid[i], t0: agora + Math.random() * 300 });
        continue;
      }
      if (Math.random() < p) this.fx.push({ i, t0: agora, r: 1 });
    }
    if (this.fx.length > 600) this.fx.splice(0, this.fx.length - 600);
    if (this.arcos.length > 80) this.arcos.splice(0, this.arcos.length - 80);
    this.ult = novas[novas.length - 1];
    this.brilhoSujo = true;
    this.textosSujos = true;
    this.#pedirQuadro();
  }
  /** Marco da noite (primeira seção do Brasil/de uma UF, BU nº 100 mil…): onda maior e selo no mapa. */
  marco(m) {
    if (!this.carregado || !["primeira_secao", "bu_marco", "ultima_secao"].includes(m.classe) || m.municipio == null) return;
    const i = this.indice(m.municipio, m.zona, m.secao);
    if (i < 0) return;
    const nomes = this.op.nomes();
    const lugar2 = nomes ? nomes.local(`${m.uf}${String(m.municipio).padStart(5, "0")}`) : "";
    const txt = m.classe === "bu_marco" ? `BU nº ${mil(m.valor)} da noite: ${lugar2}` : m.classe === "ultima_secao" ? `Última seção: ${lugar2}` : m.abrangencia === "br" ? `1ª seção do Brasil: ${lugar2}` : `1ª seção ${nomes ? nomes.em(m.abrangencia) : ""}: ${lugar2}`;
    if (!this.reduzido && !this.ext[i]) this.fx.push({ i, t0: performance.now(), r: 3.2, grande: true });
    this.marcos.push({ i, txt });
    if (this.marcos.length > 3) this.marcos.splice(0, this.marcos.length - 3);
    this.#pedirQuadro();
  }
  // ---------------------------------------------------------------- desenho
  #dimensionar() {
    const r = { width: this.palco.offsetWidth, height: this.palco.offsetHeight };
    if (!r.width || !r.height) return false;
    this.dpr = Math.min(2, window.devicePixelRatio || 1);
    const W = Math.round(r.width * this.dpr), H = Math.round(r.height * this.dpr);
    if (W === this.W && H === this.H && this.esc) return true;
    this.W = W;
    this.H = H;
    for (const n of ["base", "acc", "globo", "fx"]) {
      this.cv[n].width = this.W;
      this.cv[n].height = this.H;
    }
    this.cv.brilho.width = Math.ceil(this.W / 3);
    this.cv.brilho.height = Math.ceil(this.H / 3);
    const P = this.geo.plano;
    this.esc = Math.min(this.W / P.largura, this.H / P.altura) * 0.94;
    this.ox = (this.W - P.largura * this.esc) / 2;
    this.oy = (this.H - P.altura * this.esc) / 2;
    return true;
  }
  #redesenhar() {
    if (!this.carregado || !this.#dimensionar()) return;
    const c = this.cv.base.getContext("2d"), d = this.dpr;
    c.setTransform(1, 0, 0, 1, 0, 0);
    c.clearRect(0, 0, this.W, this.H);
    c.setTransform(this.esc, 0, 0, this.esc, this.ox, this.oy);
    c.lineJoin = "round";
    c.save();
    c.shadowColor = "rgba(120,100,255,0.45)";
    c.shadowBlur = 28 * d;
    c.fillStyle = "rgba(5,8,26,0.84)";
    c.fill(this.brasil);
    c.restore();
    c.strokeStyle = "rgba(160,180,255,0.12)";
    c.lineWidth = 0.7 / this.esc * d;
    for (const p of this.paths) c.stroke(p);
    const a = this.cv.acc.getContext("2d");
    a.setTransform(1, 0, 0, 1, 0, 0);
    a.globalCompositeOperation = "source-over";
    a.clearRect(0, 0, this.W, this.H);
    const todas = [];
    for (let i = 0; i < this.N; i++) if (this.hora[i] && !this.ext[i]) todas.push(i);
    if (this.cor === "hora") todas.sort((x, y) => this.hora[x] - this.hora[y]);
    this.#pintar(todas);
    this.#atualizarBrilho();
    this.#pedirQuadro();
  }
  /** Acende pontos na camada das luzes (luz aditiva: onde há muitas seções juntas, fica branco). */
  #pintar(lista) {
    if (!this.W || !lista.length) return;
    const a = this.cv.acc.getContext("2d");
    a.globalCompositeOperation = "lighter";
    const e = this.esc / this.dpr;
    const f2 = Math.max(0.16, Math.min(1, Math.pow(e / 0.72, 1.4)));
    const s = Math.max(1, 1.15 * this.dpr * Math.sqrt(e / 0.7)), h = s / 2;
    let ult = -1;
    a.fillStyle = `rgba(150,205,255,${(0.42 * f2).toFixed(3)})`;
    for (const i of lista) {
      if (this.ext[i]) continue;
      if (this.cor === "hora" && this.hora[i] !== ult) {
        ult = this.hora[i];
        a.fillStyle = corFaixa(ult, (0.55 * f2).toFixed(3));
      }
      a.fillRect(this.ox + this.X[i] * this.esc - h, this.oy + this.Y[i] * this.esc - h, s, s);
    }
  }
  #atualizarBrilho() {
    if (!this.W) return;
    const g = this.cv.brilho.getContext("2d");
    g.clearRect(0, 0, this.cv.brilho.width, this.cv.brilho.height);
    g.drawImage(this.vista === "globo" ? this.cv.globo : this.cv.acc, 0, 0, this.cv.brilho.width, this.cv.brilho.height);
    this.brilhoSujo = false;
    this.tBrilho = performance.now();
  }
  #pedirQuadro() {
    if (this.raf || !this.carregado || !this.visivel) return;
    this.raf = requestAnimationFrame((t) => this.#quadro(t));
  }
  #quadro(agora) {
    this.raf = 0;
    const dt = Math.min(100, agora - (this.tQuadro || agora));
    this.tQuadro = agora;
    const globo = this.vista === "globo";
    if (globo) this.#desenharGlobo(agora, dt);
    if (this.brilhoSujo || globo) {
      if (agora - (this.tBrilho || 0) > (globo ? 60 : 180)) this.#atualizarBrilho();
    }
    this.#desenharFx(agora);
    this.#mostrarMarco(agora);
    if (this.textosSujos && agora - (this.tTextos || 0) > 300) this.#textos();
    const anima = globo || this.fx.length || this.arcos.length || this.brilhoSujo || this.textosSujos || this.marcos.length;
    if (anima && this.visivel) this.raf = requestAnimationFrame((t) => this.#quadro(t));
  }
  #textos() {
    this.textosSujos = false;
    this.tTextos = performance.now();
    this.hod?.definir(this.brAcesas + this.extAcesas);
    this.deTxt.textContent = ` de ${mil(this.total ?? this.N)}`;
    if (this.nExt) this.extTxt.replaceChildren(el("span", "mp-ptx"), `Exterior: ${mil(this.extAcesas)} de ${mil(this.nExt)} seções`);
    if (this.ult != null) {
      const i = this.ult, mi = this.mu[i], uf = this.munUf[mi], nomes = this.op.nomes();
      const lugar2 = nomes ? nomes.local(`${uf}${String(this.munCod[mi]).padStart(5, "0")}`) : `${uf}`;
      this.ultTxt.replaceChildren("Último BU: ", el("b", null, lugar2), ` · zona ${this.zona[i]} · seção ${this.secao[i]}`);
    }
    if (this.el.classList.contains("max")) this.#painelUfs();
  }
  #painelUfs() {
    if (!this.carregado) return;
    if (!this.ufLinhas) {
      this.ufLinhas = [...this.totUf.keys()].sort().map((u) => {
        const t = el("span", "t", el("i")), v = el("span", "v", "0%");
        const l = el("div", "mp-uf", [el("b", null, u.toUpperCase()), t, v]);
        this.ufs.append(l);
        return { u, i: t.firstChild, v };
      });
    }
    for (const x of this.ufLinhas) {
      const f2 = (this.acesasUf.get(x.u) || 0) / (this.totUf.get(x.u) || 1);
      x.i.style.transform = `scaleX(${f2})`;
      x.v.textContent = `${Math.floor(f2 * 100)}%`;
    }
  }
  #desenharFx(agora) {
    const c = this.cv.fx.getContext("2d");
    c.setTransform(1, 0, 0, 1, 0, 0);
    c.clearRect(0, 0, this.W, this.H);
    if (this.vista === "globo") return;
    if (!this.fx.length) return;
    const d = this.dpr, fs = Math.min(1.3, Math.sqrt(this.esc / d / 0.72));
    c.globalCompositeOperation = "lighter";
    let w = 0;
    for (const f2 of this.fx) {
      const vida = f2.grande ? 2200 : 1100, e = (agora - f2.t0) / vida;
      if (e >= 1) continue;
      this.fx[w++] = f2;
      if (e < 0) continue;
      const x = this.ox + this.X[f2.i] * this.esc, y = this.oy + this.Y[f2.i] * this.esc;
      const o = 1 - Math.pow(1 - e, 3), a = Math.pow(1 - e, 2);
      const cor = this.cor === "hora" ? corFaixa(this.hora[f2.i], 1) : "rgb(170,220,255)";
      c.globalAlpha = a * (f2.grande ? 0.95 : 0.7);
      c.strokeStyle = cor;
      c.lineWidth = (f2.grande ? 1.6 : 1) * d;
      c.beginPath();
      c.arc(x, y, (2 + 13 * f2.r * o) * d * fs, 0, 6.2832);
      c.stroke();
      if (f2.grande) {
        c.globalAlpha = a * 0.5;
        c.beginPath();
        c.arc(x, y, (2 + 13 * f2.r * o) * d * fs * 0.55, 0, 6.2832);
        c.stroke();
      }
      const raio = (f2.grande ? 16 : 6) * d * fs;
      const g = c.createRadialGradient(x, y, 0, x, y, raio * (1 - e * 0.6));
      g.addColorStop(0, "rgba(255,255,255,1)");
      g.addColorStop(1, "rgba(255,255,255,0)");
      c.globalAlpha = Math.pow(1 - e, 4);
      c.fillStyle = g;
      c.beginPath();
      c.arc(x, y, raio, 0, 6.2832);
      c.fill();
    }
    this.fx.length = w;
    c.globalAlpha = 1;
  }
  #mostrarMarco(agora) {
    if (agora < this.marcoAte || !this.marcos.length) return;
    const m = this.marcos.shift();
    this.marcoAte = agora + 2800;
    this.chip.textContent = m.txt;
    const pw = this.palco.clientWidth, w = Math.min(this.chip.offsetWidth || 240, pw - 16);
    let x = pw / 2 - w / 2, y = 12;
    if (this.vista !== "globo" && !this.ext[m.i]) {
      x = (this.ox + this.X[m.i] * this.esc) / this.dpr - w / 2;
      y = Math.max(12, (this.oy + this.Y[m.i] * this.esc) / this.dpr - 46);
    }
    this.chip.style.transform = `translate(${Math.max(8, Math.min(pw - w - 8, x))}px, ${y}px)`;
    this.chip.classList.add("ligado");
    clearTimeout(this.tChip);
    this.tChip = setTimeout(() => this.chip.classList.remove("ligado"), 2400);
    this.op.aoMarco?.(m.txt);
  }
  // ---------------------------------------------------------------- globo
  #desenharGlobo(agora, dt) {
    const G = this.G, c = this.cv.globo.getContext("2d"), W = this.W, H = this.H, d = this.dpr;
    if (!G.arr) {
      G.lam += G.vl * dt / 16;
      G.phi += G.vp * dt / 16;
      G.vl *= 0.94;
      G.vp *= 0.94;
      if (agora - G.mexeu > 7e3 && !this.reduzido) {
        const al = G.base + 38 * Math.sin(agora / 9e3), ap = -14 + 6 * Math.sin(agora / 13e3);
        G.lam += ((al - G.lam + 540) % 360 - 180) * 0.012;
        G.phi += (ap - G.phi) * 0.012;
      }
    }
    G.phi = Math.max(-80, Math.min(80, G.phi));
    const l0 = G.lam * rad, p0 = G.phi * rad, cl = Math.cos(l0), sl = Math.sin(l0), cp = Math.cos(p0), sp = Math.sin(p0);
    const R = Math.min(W, H) * 0.41 * G.zoom, cx = W / 2, cy = H * 0.52;
    const P = [0, 0, 0];
    const proj = (x, y, z) => {
      const u = x * cl + y * sl, v = y * cl - x * sl;
      P[0] = cx + R * v;
      P[1] = cy - R * (cp * z - sp * u);
      P[2] = sp * z + cp * u;
      return P;
    };
    c.setTransform(1, 0, 0, 1, 0, 0);
    c.globalCompositeOperation = "source-over";
    c.globalAlpha = 1;
    c.clearRect(0, 0, W, H);
    const atm = c.createRadialGradient(cx, cy, R * 0.9, cx, cy, R * 1.22);
    atm.addColorStop(0, "rgba(120,100,255,0.28)");
    atm.addColorStop(0.35, "rgba(79,195,255,0.10)");
    atm.addColorStop(1, "rgba(79,195,255,0)");
    c.fillStyle = atm;
    c.beginPath();
    c.arc(cx, cy, R * 1.22, 0, 6.2832);
    c.fill();
    const claro = document.documentElement.dataset.tema === "claro";
    const corpo = c.createRadialGradient(cx - R * 0.35, cy - R * 0.4, R * 0.1, cx, cy, R);
    corpo.addColorStop(0, claro ? "rgba(30,36,100,0.62)" : "rgba(60,70,150,0.16)");
    corpo.addColorStop(0.75, claro ? "rgba(14,18,56,0.78)" : "rgba(14,18,52,0.30)");
    corpo.addColorStop(1, claro ? "rgba(10,14,44,0.88)" : "rgba(10,14,44,0.55)");
    c.fillStyle = corpo;
    c.beginPath();
    c.arc(cx, cy, R, 0, 6.2832);
    c.fill();
    c.strokeStyle = "rgba(170,180,255,0.30)";
    c.lineWidth = d;
    c.stroke();
    const sz = Math.max(1, 1.5 * d * Math.sqrt(G.zoom) * Math.min(1.2, R / (300 * d)));
    const MV = this.MV;
    c.fillStyle = "rgb(170,182,255)";
    for (let i = 0; i < MV.length; i += 3) {
      proj(MV[i], MV[i + 1], MV[i + 2]);
      const z = P[2], t = z < 0 ? sz * 0.7 : sz;
      c.globalAlpha = z < 0 ? 0.05 + 0.07 * (1 + z) : 0.18 + 0.45 * z;
      c.fillRect(P[0] - t / 2, P[1] - t / 2, t, t);
    }
    c.globalCompositeOperation = "lighter";
    c.fillStyle = "rgb(150,205,255)";
    const kb = Math.min(1.4, R / (320 * d)) * d, BV = this.BV;
    for (const b of this.binAtivos) {
      const n = this.binN[b];
      proj(BV[b * 3], BV[b * 3 + 1], BV[b * 3 + 2]);
      const z = P[2], t = Math.min(3.2 * d, (0.8 + 0.09 * Math.sqrt(n)) * kb);
      c.globalAlpha = Math.min(1, 0.3 + 0.06 * Math.sqrt(n)) * (z < 0 ? 0.06 : 0.35 + 0.65 * z);
      c.fillRect(P[0] - t / 2, P[1] - t / 2, t, t);
    }
    for (const ci of this.cidades) {
      if (!ci[4]) continue;
      proj(ci[3][0], ci[3][1], ci[3][2]);
      if (P[2] < -0.05) continue;
      const r = (1.6 + 0.45 * Math.sqrt(ci[4])) * kb, x = P[0], y = P[1];
      const g = c.createRadialGradient(x, y, 0, x, y, r * 2.2);
      g.addColorStop(0, "rgba(255,220,250,1)");
      g.addColorStop(0.35, "rgba(255,95,210,0.75)");
      g.addColorStop(1, "rgba(255,95,210,0)");
      c.globalAlpha = 0.5 + 0.5 * Math.max(0, P[2]);
      c.fillStyle = g;
      c.beginPath();
      c.arc(x, y, r * 2.2, 0, 6.2832);
      c.fill();
    }
    const pl = this.geo.plano;
    c.lineWidth = d;
    c.strokeStyle = "rgb(170,220,255)";
    let w = 0;
    for (const f2 of this.fx) {
      const vida = f2.grande ? 2200 : 1100, e = (agora - f2.t0) / vida;
      if (e >= 1) continue;
      this.fx[w++] = f2;
      if (e < 0) continue;
      const v = vet(pl.lat1 - this.Y[f2.i] * pl.span / pl.largura, pl.lon0 + this.X[f2.i] * pl.span / pl.largura);
      proj(v[0], v[1], v[2]);
      if (P[2] < 0) continue;
      c.globalAlpha = Math.pow(1 - e, 2) * (f2.grande ? 0.9 : 0.6) * P[2];
      c.beginPath();
      c.arc(P[0], P[1], (1.5 + 9 * f2.r * (1 - Math.pow(1 - e, 3))) * kb, 0, 6.2832);
      c.stroke();
    }
    this.fx.length = w;
    let wa = 0;
    for (const a of this.arcos) {
      const e = (agora - a.t0) / 2400;
      if (e >= 1) continue;
      this.arcos[wa++] = a;
      if (e < 0) continue;
      const v0 = this.cidades[a.c][3];
      const om = Math.acos(Math.max(-1, Math.min(1, v0[0] * BSB[0] + v0[1] * BSB[1] + v0[2] * BSB[2]))), so = Math.sin(om) || 1e-6;
      const alt = 0.08 + 0.22 * Math.min(1, om / 1.6), cab = Math.min(1, e / 0.6), cauda = Math.max(0, (e - 0.25) / 0.75);
      const pt = (u) => {
        const w0 = Math.sin((1 - u) * om) / so, w1 = Math.sin(u * om) / so, hh = 1 + alt * Math.sin(Math.PI * u);
        return proj((v0[0] * w0 + BSB[0] * w1) * hh, (v0[1] * w0 + BSB[1] * w1) * hh, (v0[2] * w0 + BSB[2] * w1) * hh);
      };
      let ax, ay;
      pt(cauda);
      ax = P[0];
      ay = P[1];
      for (let s = 1; s <= 36; s++) {
        const q = pt(cauda + (cab - cauda) * s / 36), f2 = s / 36;
        const vis = q[2] > 0 || Math.hypot(q[0] - cx, q[1] - cy) > R;
        c.globalAlpha = (vis ? 0.85 : 0.18) * f2 * (1 - Math.max(0, e - 0.8) / 0.2);
        c.strokeStyle = f2 > 0.7 ? "rgb(150,220,255)" : "rgb(255,110,215)";
        c.lineWidth = (0.8 + 1.2 * f2) * d;
        c.beginPath();
        c.moveTo(ax, ay);
        c.lineTo(q[0], q[1]);
        c.stroke();
        ax = q[0];
        ay = q[1];
      }
      if (cab < 1) {
        c.globalAlpha = 1;
        c.fillStyle = "rgb(255,240,255)";
        c.beginPath();
        c.arc(ax, ay, 2.2 * d, 0, 6.2832);
        c.fill();
      } else {
        const ee = (e - 0.6) / 0.4;
        proj(BSB[0], BSB[1], BSB[2]);
        c.globalAlpha = (1 - ee) * 0.8;
        c.strokeStyle = "rgb(255,150,225)";
        c.lineWidth = 1.2 * d;
        c.beginPath();
        c.arc(P[0], P[1], (3 + 14 * ee) * kb, 0, 6.2832);
        c.stroke();
      }
    }
    this.arcos.length = wa;
    c.globalAlpha = 1;
    c.globalCompositeOperation = "source-over";
  }
  #interacao() {
    const fx = this.cv.fx, G = this.G;
    fx.addEventListener("pointerdown", (e) => {
      if (this.vista !== "globo" || e.button) return;
      G.arr = { x: e.clientX, y: e.clientY, t: performance.now() };
      G.vl = G.vp = 0;
      G.mexeu = performance.now();
      fx.setPointerCapture(e.pointerId);
      this.palco.classList.add("girando");
    });
    fx.addEventListener("pointermove", (e) => {
      if (!G.arr) return;
      const k = 0.32 / G.zoom, dx = e.clientX - G.arr.x, dy = e.clientY - G.arr.y, agora = performance.now(), dtt = Math.max(8, agora - G.arr.t);
      G.lam -= dx * k;
      G.phi += dy * k;
      G.vl = -dx * k * 16 / dtt;
      G.vp = dy * k * 16 / dtt;
      G.arr = { x: e.clientX, y: e.clientY, t: agora };
      G.mexeu = agora;
      this.#pedirQuadro();
    });
    const soltar = () => {
      if (!G.arr) return;
      G.arr = null;
      this.palco.classList.remove("girando");
      G.mexeu = performance.now();
      G.base = G.lam;
    };
    fx.addEventListener("pointerup", soltar);
    fx.addEventListener("pointercancel", soltar);
    fx.addEventListener("wheel", (e) => {
      if (this.vista !== "globo" || !(e.ctrlKey || this.el.classList.contains("max"))) return;
      e.preventDefault();
      G.zoom = Math.max(0.7, Math.min(3.2, G.zoom * Math.exp(-e.deltaY * 15e-4)));
      G.mexeu = performance.now();
      this.#pedirQuadro();
    }, { passive: false });
    fx.addEventListener("dblclick", (e) => {
      if (this.vista !== "globo") return;
      e.stopPropagation();
      G.base = -50;
      G.mexeu = -1e9;
      G.zoom = 1;
      this.#pedirQuadro();
    });
  }
  /** Para os ensaios (Playwright). */
  resumo() {
    return {
      carregado: this.carregado,
      N: this.N,
      n: this.n,
      acesas: (this.brAcesas || 0) + (this.extAcesas || 0),
      exterior: this.extAcesas,
      nExt: this.nExt,
      semPosicao: this.semPosicao,
      noLocal: this.noLocal,
      vista: this.vista,
      cor: this.cor
    };
  }
};

// src/cartoes.js
var CHAVE2 = "uv-cartoes";
var ler2 = () => {
  try {
    const v = JSON.parse(localStorage.getItem(CHAVE2));
    return Array.isArray(v) && v.length === 2 ? v : null;
  } catch {
    return null;
  }
};
var gravar2 = (v) => {
  try {
    localStorage.setItem(CHAVE2, JSON.stringify(v));
  } catch {
  }
};
var SVG_ALCA = '<svg viewBox="0 0 16 16" aria-hidden="true"><circle cx="5.5" cy="3.5" r="1.4"/><circle cx="10.5" cy="3.5" r="1.4"/><circle cx="5.5" cy="8" r="1.4"/><circle cx="10.5" cy="8" r="1.4"/><circle cx="5.5" cy="12.5" r="1.4"/><circle cx="10.5" cy="12.5" r="1.4"/></svg>';
var SVG_MAX = '<svg class="ic-max" viewBox="0 0 16 16" aria-hidden="true"><path d="M9.5 2.5h4v4M6.5 13.5h-4v-4M13.5 2.5 9 7M2.5 13.5 7 9"/></svg><svg class="ic-min" viewBox="0 0 16 16" aria-hidden="true"><path d="M13 7H9V3M3 9h4v4M9 7l4.5-4.5M7 9l-4.5 4.5"/></svg>';
var Cartoes = class {
  /** @param {{aoMaximizar?: (c: HTMLElement, sim: boolean) => void}} op */
  constructor(op = {}) {
    this.op = op;
    this.trilhos = [];
    this.padrao = null;
    this.max = null;
    this.arr = null;
    this.veu = el("div", "cartao-veu");
    this.veu.addEventListener("click", () => this.restaurar());
    document.body.append(this.veu);
    addEventListener("keydown", (e) => {
      if (e.key === "Escape" && this.max) this.restaurar();
    });
  }
  /** Põe a alça e o botão de maximizar num cartão (uma vez por elemento). */
  preparar(c, id, titulo2) {
    c.dataset.cartao = id;
    c.classList.add("cartao-mov");
    if (c.querySelector(":scope > .cartao-ferr")) return c;
    const alca = el("button", "cf-alca");
    alca.type = "button";
    alca.innerHTML = SVG_ALCA;
    alca.setAttribute("aria-label", `Mover o cartão ${titulo2} (arraste, ou use as setas)`);
    alca.title = "Arrastar para reorganizar";
    const max = el("button", "cf-max");
    max.type = "button";
    max.innerHTML = SVG_MAX;
    max.setAttribute("aria-label", `Maximizar o cartão ${titulo2}`);
    max.title = "Maximizar";
    c.append(el("div", "cartao-ferr", [alca, max]));
    alca.addEventListener("pointerdown", (e) => {
      if (e.button || this.max) return;
      e.preventDefault();
      alca.setPointerCapture(e.pointerId);
      this.#iniciar(c, e);
    });
    alca.addEventListener("pointermove", (e) => this.#mover(e));
    alca.addEventListener("pointerup", () => this.#soltar());
    alca.addEventListener("pointercancel", () => this.#soltar());
    alca.addEventListener("keydown", (e) => {
      if (e.key.startsWith("Arrow")) {
        e.preventDefault();
        this.#teclado(c, e.key);
      }
    });
    max.addEventListener("click", () => this.max?.c === c ? this.restaurar() : this.maximizar(c));
    return c;
  }
  /** Os dois trilhos do painel (a coluna larga e a lateral), já com os cartões na ordem padrão: aplica a salva. */
  montar(trilhos) {
    this.trilhos = trilhos;
    const ordem = (t) => [...t.children].filter((x) => x.dataset.cartao).map((x) => x.dataset.cartao);
    this.padrao = trilhos.map(ordem);
    const salvo = ler2();
    if (salvo) this.#aplicar(salvo);
  }
  restaurarLayout() {
    this.restaurar(true);
    if (!this.padrao) return;
    this.#flip(() => this.#aplicar(this.padrao));
    gravar2(this.padrao);
  }
  #aplicar(o) {
    const porId = new Map(this.trilhos.flatMap((t) => [...t.children]).filter((x) => x.dataset.cartao).map((x) => [x.dataset.cartao, x]));
    const usados = /* @__PURE__ */ new Set();
    o.forEach((lista, i) => (lista || []).forEach((id) => {
      const c = porId.get(id);
      if (c && !usados.has(id)) {
        this.trilhos[i].append(c);
        usados.add(id);
      }
    }));
    this.padrao.forEach((lista, i) => lista.forEach((id) => {
      const c = porId.get(id);
      if (c && !usados.has(id)) this.trilhos[i].append(c);
    }));
  }
  #ordem() {
    return this.trilhos.map((t) => [...t.children].filter((x) => x.dataset.cartao).map((x) => x.dataset.cartao));
  }
  /** FLIP pela propriedade `translate`: os vizinhos deslizam até o lugar novo. */
  #flip(mudar) {
    const els = this.trilhos.flatMap((t) => [...t.children]).filter((x) => !x.classList.contains("arrastando") && !x.classList.contains("max"));
    const antes = new Map(els.map((x) => [x, x.getBoundingClientRect()]));
    mudar();
    if (movimentoReduzido()) return;
    for (const x of els) {
      const a = antes.get(x), d = x.getBoundingClientRect(), dx = a.left - d.left, dy = a.top - d.top;
      if (!dx && !dy) continue;
      x.style.transition = "none";
      x.style.translate = `${dx}px ${dy}px`;
      x.getBoundingClientRect();
      x.style.transition = "";
      x.style.translate = "0px 0px";
      setTimeout(() => {
        x.style.translate = "";
      }, 420);
    }
  }
  #iniciar(c, e) {
    const r = c.getBoundingClientRect();
    const vaga = el("div", "cartao-vaga");
    vaga.style.height = `${r.height}px`;
    c.before(vaga);
    Object.assign(c.style, { width: `${r.width}px`, left: `${r.left}px`, top: `${r.top}px` });
    c.style.setProperty("--rx", "0deg");
    c.style.setProperty("--ry", "0deg");
    c.classList.add("arrastando");
    document.body.classList.add("arrastando-cartao");
    this.arr = { c, vaga, x0: e.clientX, y0: e.clientY };
  }
  #mover(e) {
    const a = this.arr;
    if (!a) return;
    a.c.style.translate = `${e.clientX - a.x0}px ${e.clientY - a.y0}px`;
    const px = e.clientX, py = e.clientY;
    let alvo = this.trilhos.find((t) => {
      const b = t.getBoundingClientRect();
      return px >= b.left && px <= b.right && py >= b.top - 40 && py <= b.bottom + 40;
    });
    if (!alvo) {
      let dmin = Infinity;
      for (const t of this.trilhos) {
        const b = t.getBoundingClientRect(), dd2 = Math.hypot(Math.max(b.left - px, 0, px - b.right), Math.max(b.top - py, 0, py - b.bottom));
        if (dd2 < dmin) {
          dmin = dd2;
          alvo = t;
        }
      }
    }
    const irmaos = [...alvo.children].filter((x) => x.dataset.cartao && x !== a.c);
    const antesDe = irmaos.find((x) => {
      const b = x.getBoundingClientRect();
      return py < b.top + b.height / 2;
    }) || null;
    if (a.vaga.parentNode !== alvo || a.vaga.nextElementSibling !== antesDe) {
      this.#flip(() => {
        if (antesDe) alvo.insertBefore(a.vaga, antesDe);
        else alvo.append(a.vaga);
      });
    }
  }
  #soltar() {
    const a = this.arr;
    if (!a) return;
    this.arr = null;
    const d = a.vaga.getBoundingClientRect(), r = a.c.getBoundingClientRect();
    const [tx, ty] = (a.c.style.translate || "0px 0px").split(" ").map((v) => parseFloat(v) || 0);
    a.c.style.translate = `${tx + d.left - r.left}px ${ty + d.top - r.top}px`;
    const fim = () => {
      a.c.classList.remove("arrastando");
      document.body.classList.remove("arrastando-cartao");
      for (const p of ["width", "left", "top", "translate"]) a.c.style[p] = "";
      a.vaga.replaceWith(a.c);
      gravar2(this.#ordem());
    };
    if (movimentoReduzido()) fim();
    else setTimeout(fim, 260);
  }
  #teclado(c, tecla) {
    const t = c.parentNode, i = this.trilhos.indexOf(t);
    this.#flip(() => {
      if (tecla === "ArrowUp" && c.previousElementSibling) t.insertBefore(c, c.previousElementSibling);
      else if (tecla === "ArrowDown" && c.nextElementSibling) t.insertBefore(c.nextElementSibling, c);
      else if (tecla === "ArrowLeft" && i === 1) this.trilhos[0].append(c);
      else if (tecla === "ArrowRight" && i === 0) this.trilhos[1].prepend(c);
    });
    gravar2(this.#ordem());
    c.querySelector(".cf-alca")?.focus();
  }
  maximizar(c) {
    if (this.max) this.restaurar(true);
    const r0 = c.getBoundingClientRect();
    const vaga = el("div", "cartao-vaga-max");
    vaga.style.height = `${r0.height}px`;
    c.before(vaga);
    c.style.setProperty("--rx", "0deg");
    c.style.setProperty("--ry", "0deg");
    c.classList.add("max");
    this.veu.classList.add("ligado");
    document.body.classList.add("cartao-maximizado");
    this.max = { c, vaga };
    c.querySelector(".cf-max")?.setAttribute("aria-label", "Restaurar o cartão");
    this.op.aoMaximizar?.(c, true);
    if (movimentoReduzido()) return;
    const r1 = c.getBoundingClientRect();
    c.style.transformOrigin = "0 0";
    c.style.translate = `${r0.left - r1.left}px ${r0.top - r1.top}px`;
    c.style.scale = `${r0.width / r1.width} ${r0.height / r1.height}`;
    c.getBoundingClientRect();
    c.classList.add("anim-max");
    c.style.translate = "0px 0px";
    c.style.scale = "1";
    setTimeout(() => {
      c.classList.remove("anim-max");
      c.style.translate = "";
      c.style.scale = "";
      c.style.transformOrigin = "";
    }, 380);
  }
  /** Fecha o cartão maximizado (`ja`: sem animação, ao trocar de tela). */
  restaurar(ja) {
    const m = this.max;
    if (!m) return;
    this.max = null;
    this.veu.classList.remove("ligado");
    document.body.classList.remove("cartao-maximizado");
    m.c.querySelector(".cf-max")?.setAttribute("aria-label", "Maximizar o cartão");
    const fim = () => {
      m.c.classList.remove("max", "anim-max");
      for (const p of ["translate", "scale", "transformOrigin"]) m.c.style[p] = "";
      m.vaga.remove();
      this.op.aoMaximizar?.(m.c, false);
    };
    if (ja || movimentoReduzido() || !m.vaga.isConnected) return fim();
    const r1 = m.c.getBoundingClientRect(), r0 = m.vaga.getBoundingClientRect();
    m.c.style.transformOrigin = "0 0";
    m.c.classList.add("anim-max");
    m.c.style.translate = `${r0.left - r1.left}px ${r0.top - r1.top}px`;
    m.c.style.scale = `${r0.width / r1.width} ${r0.height / r1.height}`;
    setTimeout(fim, 320);
  }
};

// src/proxima.js
var FUSO = "America/Sao_Paulo";
var HORA_PADRAO = "17:00:00";
var NOITE_MS = 15 * 36e5;
function modoDe(e) {
  if (!e) return "";
  if (e.simulacao?.rotulo) return "replay";
  if ((e.secoes?.aplicadas || 0) > 0 || e.encerrada) return "ao_vivo";
  return "espera";
}
async function buscarPlaylist() {
  try {
    const r = await fetch("/api/v1/playlist", { cache: "no-store" });
    if (!r.ok) return null;
    const j = await r.json();
    if (!j || typeof j !== "object") return null;
    const ag = Date.parse(j.agora);
    j.desvio = Number.isFinite(ag) ? ag - Date.now() : 0;
    return j;
  } catch {
    return null;
  }
}
function proximaDe(pl) {
  if (!pl) return null;
  const p = pl.proxima_ao_vivo;
  if (p?.inicio) {
    const ini = Date.parse(p.inicio), fim = Date.parse(p.fim);
    if (Number.isFinite(ini)) return { nome: p.nome || "eleição oficial", inicio: ini, fim: Number.isFinite(fim) ? fim : ini + NOITE_MS };
  }
  const s = pl.suspensao || pl.proxima_suspensao;
  if (s?.eleicao) {
    const ini = Date.parse(`${s.eleicao}T${HORA_PADRAO}-03:00`);
    if (Number.isFinite(ini)) return { nome: s.nome || "eleição oficial", inicio: ini, fim: ini + NOITE_MS };
  }
  return null;
}
var partes = (ms, opc) => Object.fromEntries(new Intl.DateTimeFormat("pt-BR", { timeZone: FUSO, ...opc }).formatToParts(new Date(ms)).map((p) => [p.type, p.value]));
var dataBr = (ms) => {
  const p = partes(ms, { year: "numeric", month: "2-digit", day: "2-digit" });
  return `${p.year}-${p.month}-${p.day}`;
};
function quandoTxt(inicio, agora) {
  const p = partes(inicio, { weekday: "long", day: "2-digit", month: "2-digit", hour: "2-digit", minute: "2-digit", hourCycle: "h23" });
  const hora2 = `${+p.hour}h${p.minute !== "00" ? p.minute : ""}`;
  const d = dataBr(inicio);
  const dia = d === dataBr(agora) ? "hoje" : d === dataBr(agora + 864e5) ? "amanhã" : p.weekday;
  const curto = `${p.day}/${p.month}, ${hora2}`;
  return { dia, curto, longo: `${dia} (${p.day}/${p.month}), ${hora2}`, ddmm: `${p.day}/${p.month}`, hora: hora2 };
}
var dd = (n) => String(n).padStart(2, "0");
function contagemPartes(ms) {
  const s = Math.max(0, Math.floor(ms / 1e3));
  const d = Math.floor(s / 86400), h = Math.floor(s % 86400 / 3600), m = Math.floor(s % 3600 / 60), ss = s % 60;
  if (d > 0) return [`${d}d`, `${dd(h)}h`, `${dd(m)}min`];
  if (h > 0) return [`${h}h`, `${dd(m)}min`, `${dd(ss)}s`];
  return [`${m}min`, `${dd(ss)}s`];
}
function diaLocal(ms = Date.now()) {
  const d = new Date(ms);
  return `${d.getFullYear()}-${dd(d.getMonth() + 1)}-${dd(d.getDate())}`;
}

// src/app.js
var $2 = (s) => document.querySelector(s);
var cargo4 = (c) => String(c).padStart(4, "0");
var TEXTO_HTTP = "Muitos acessos agora: em vez de receber a apuração em tempo real, esta página a consulta a cada minuto (o letreiro de eventos, a comparação com o TSE e a previsão ficam parados). Volta ao tempo real sozinha assim que houver vaga.";
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
  // [instante ms, seq] do br/maior tópico (BUs por minuto), no relógio da página (servidor antigo)
  ritmoSrv: [],
  // B-053: [instante ms, seq] do `estado`, no relógio do serviço
  ritmoSessao: void 0,
  // `sistema0` da sessão das amostras (replay que recomeça: seq volta do zero)
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
      S.mapa?.marco(m);
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
    case "mapa":
      S.mapa?.receber(m);
      break;
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
      verificarVersao(m.versao);
      if (m.ultimo_bu?.seq != null) S.seqMax = Math.max(S.seqMax || 0, m.ultimo_bu.seq);
      if (m.ultimo_bu?.seq != null && m.relogio_sistema?.agora) amostrarRitmo(m.ultimo_bu.seq, segundosF(m.relogio_sistema.agora), m.relogio_sistema.sistema0);
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
    if (c.classList.contains("max") || c.classList.contains("arrastando") || e.target?.closest?.(".mp-palco.globo")) return;
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
  S.cartoes?.restaurar(true);
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
    const info2 = S.N.candidato(circ, res.cargo, k.n);
    const cor = cores.get(k.n);
    const li = el("li", "cand");
    li.style.setProperty("--c", cor);
    const pctH = new Hodometro({ inteiros: 3, casas: 2, sufixo: "%" });
    const votH = new Hodometro({ inteiros: dig, sufixo: "votos" });
    const barra = el("i");
    const pilula = el("span", "pilula", "");
    const vice = info2.vices.find((v) => v.tp === "v") || info2.vices[0];
    const det = [info2.partido, vice ? `${vice.tp === "v" ? "vice" : "suplente"} ${vice.nome_urna}` : ""].filter(Boolean).join(" · ");
    const nome = el("div", "nome", [el("b", null, info2.nome), pilula]);
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
    linhas.set(k.n, { li, pctH, votH, barra, pilula, pilTxt: "", tc, pos: -1, glow: false, diverge: false, nome: info2.nome, cor });
  }
  lista.classList.add(`m-${S.comparar}`);
  const consulta = S.N.cargos.get(res.cargo)?.consulta;
  const h1 = el("h1", null, consulta ? `Consulta — ${S.N.local(t.abr)}` : t.abr === "br" ? `${S.N.cargo(res.cargo)} — Brasil` : `${S.N.cargo(res.cargo)} — ${S.N.local(t.abr)}`);
  const turno = res.turno ? `${res.turno}º turno` : "";
  const sub = el("p", "sub", [turno, circ !== t.abr ? `resultado na abrangência; quem decide é ${S.N.local(circ)}` : `eleitorado de ${mil(res.eleitorado)}`].filter(Boolean).join(" · "));
  const selo = el("div", "selo", "");
  selo.setAttribute("aria-live", "polite");
  const controle = new Controle(S.comparar, (m) => mudarModo(m, true));
  const fx = el("span", "f-x", ""), fa = el("span", "f-a", ""), fb = el("span", "f-b", ""), fe = el("span", "f-est", "");
  const info = el("button", "f-info", "i");
  info.type = "button";
  info.title = "Como funciona a escolha da fonte dos números";
  info.setAttribute("aria-label", "Como funciona a escolha da fonte dos números");
  info.addEventListener("click", () => {
    medirUso("fonte-info");
    fimDaDica();
    abrirInfoFonte();
  });
  const cab = el("div", "f-cab", [el("span", "f-rot", "De onde vêm os números"), info]);
  const fontes = el("div", "fontes", [cab, controle.el, el("div", "f-txt", [fx, fa, fb]), fe]);
  const colunas = el("div", "colunas", [
    el("span", "c-uv", [el("span", "uv-longo", "Urna Velox"), el("span", "uv-curto", "UV")]),
    el("span", "c-tse", "TSE"),
    el("span", "c-dif", "Diferença")
  ]);
  colunas.setAttribute("aria-hidden", "true");
  const corrida = cardTatil(el("section", "vidro corrida", [el("header", "corrida-cab", [el("div", null, [h1, sub]), selo]), fontes, colunas, lista]));
  corrida.classList.add(`m-${S.comparar}`);
  iniciarDica();
  if (S.dicaAtiva) controle.el.classList.add("dica");
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
      el("div", null, [el("div", "rot", "Seções apuradas"), secAp.arrastarEl = el("div", "val", [secAp.el, el("span", "de", " de "), secTot.el])]),
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
  const K = S.cartoes;
  K.preparar(corrida, "corrida", "Corrida");
  K.preparar(apuracao, "apuracao", "Apuração");
  K.preparar(cardPrev, "previsao", "Previsão");
  K.preparar(cardCurva, "curva", "Curva da apuração");
  K.preparar(cardVotos, "votos", "Votos");
  const principal = el("div", "principal coluna-cartoes", [corrida]);
  if (S.mapa) {
    if (!S.mapa.tatil) {
      cardTatil(S.mapa.el);
      S.mapa.tatil = true;
    }
    principal.append(K.preparar(S.mapa.el, "mapa", "Mapa das seções"));
  }
  const lado = el("aside", "lado coluna-cartoes", [apuracao, cardPrev, cardCurva, cardVotos]);
  painel.replaceChildren(principal, lado);
  K.montar([principal, lado]);
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
    fx,
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
    if (l.pilTxt !== pil) {
      l.pilTxt = pil;
      const i = pil.indexOf(" às ");
      l.pilula.replaceChildren(...i < 0 ? [pil] : [pil.slice(0, i), el("span", "pl-hora", pil.slice(i))]);
      l.pilula.title = pil;
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
  const x = S.dicaAtiva && S.comparar === "uv" ? DICA_TEXTO : f2.x;
  if (ui.fx.textContent !== x) ui.fx.textContent = x;
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
var versaoCarregada = (() => {
  try {
    const s = document.querySelector('script[src*="app.js"]');
    const v = s ? new URL(s.src, location.href).searchParams.get("v") : null;
    return v || document.querySelector(".marca-versao")?.textContent.replace(/^v/, "") || null;
  } catch {
    return null;
  }
})();
var numerosVersao = (v) => {
  const m = /^(\d+(?:\.\d+)*)/.exec(String(v || ""));
  return m ? m[1].split(".").map(Number) : null;
};
function versaoMaior(a, b) {
  const x = numerosVersao(a), y = numerosVersao(b);
  if (!x || !y) return false;
  for (let i = 0; i < Math.max(x.length, y.length); i++) {
    const d = (x[i] || 0) - (y[i] || 0);
    if (d) return d > 0;
  }
  return false;
}
function verificarVersao(servidor) {
  if (!versaoMaior(servidor, versaoCarregada) || S.versaoDispensada === servidor) return;
  let n = S.novaVersao;
  if (!n) {
    const at = el("button", "atualizar", "Atualizar");
    at.type = "button";
    at.addEventListener("click", () => {
      medirUso("versao-atualizar");
      location.reload();
    });
    const fe = el("button", "fechar", "✕");
    fe.type = "button";
    fe.setAttribute("aria-label", "Dispensar o aviso de versão nova");
    fe.addEventListener("click", () => {
      medirUso("versao-dispensar");
      S.versaoDispensada = S.novaVersaoV;
      n.hidden = true;
    });
    n = el("div", "nova-versao", [el("span", "nv-txt", "Nova versão disponível"), at, fe]);
    n.setAttribute("role", "status");
    n.hidden = true;
    document.body.append(n);
    S.novaVersao = n;
  }
  if (n.hidden) medirUso("versao-aviso");
  S.novaVersaoV = servidor;
  n.hidden = false;
}
function iniciarDica() {
  if (S.dicaIniciada) return;
  S.dicaIniciada = true;
  if (dicaVista()) return;
  marcarDica();
  if (S.comparar !== "uv") return;
  S.dicaAtiva = true;
  medirUso("fonte-dica");
  S.dicaTimer = setTimeout(fimDaDica, DICA_MS);
}
function fimDaDica() {
  if (!S.dicaAtiva) return;
  S.dicaAtiva = false;
  clearTimeout(S.dicaTimer);
  S.ui?.controle?.el.classList.remove("dica");
  S.fontesSujo = true;
  pedirQuadro();
}
function abrirInfoFonte() {
  folha.abrir({ ...INFO_FONTE, acoes: MODOS.map(([m, rot]) => [rot, () => mudarModo(m, true), S.comparar === m]) });
}
function mudarModo(m, medir = false) {
  if (medir) {
    medirUso(`fonte-${m}`);
    fimDaDica();
  }
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
    ui.prev.definir(pTela, tc === ui.topico, S.N.em(ui.circ));
    return;
  }
  const p = pTela === pCirc ? pTela : { ...pCirc, fim99: pTela?.fim99 ?? null, fim100: pTela?.fim100 ?? null, chegou: pTela?.chegou ?? {}, instante: `${pCirc.instante}|${pTela?.instante || ""}` };
  ui.prev.definir(p, true, S.N.em(ui.circ));
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
  const ritmoH = new Hodometro({ inteiros: 6, centro: true });
  const ritmo = el("span", "chip ritmo", ritmoH.arrastarEl = el("span", "ritmo-in", [ritmoH.el, el("span", null, " BUs/min")]));
  ritmo.style.width = "150px";
  const pausa = el("span", "chip alerta oculto", "");
  pausa.style.width = "260px";
  const alerta = el("span", "chip alerta oculto", "");
  const pMarca = el("span", "vivo", [el("i", "pt"), el("span", "longo", "Próxima eleição"), el("span", null, "AO VIVO")]);
  const pNome = el("span", "nome", "");
  const pQuando = el("span", "quando", "");
  const pCont = el("span", "cont", "");
  const pExt = el("span", "ext", "");
  const pA = el("span", "promo-a", [pNome, pQuando, pCont, pExt]);
  const pB = el("span", "promo-b");
  pB.setAttribute("aria-hidden", "true");
  const pTrilho = el("span", "promo-trilho", [pA, pB]);
  const pJanela = el("span", "promo-janela", pTrilho);
  const cedo = el("button", "chip cedo oculto", [el("span", "ic", "i"), el("span", null, "Oficial às 17h"), el("span", "longo", " · por enquanto, só BUs do exterior")]);
  cedo.type = "button";
  cedo.title = "Por que o Urna Velox já está totalizando antes das 17h";
  cedo.addEventListener("click", () => folha.abrir(CEDO));
  const promo = el("button", "chip promo oculto", [pMarca, pJanela]);
  promo.type = "button";
  promo.addEventListener("click", () => abrirAviso());
  const http = el("span", "chip http oculto", "atualizando a cada minuto (muitos acessos)");
  http.title = TEXTO_HTTP;
  const paradoTxt = el("span", null, "");
  const parado = el("span", "chip parado oculto", [el("span", "ic", "!"), paradoTxt, el("span", "longo", " · retomamos automaticamente quando voltarem")]);
  parado.setAttribute("role", "status");
  $2("#faixa").replaceChildren(parado, promo, http, ritmo, cedo, pausa, alerta);
  faixa = { ritmoH, ritmo, pausa, alerta, promo, pNome, pQuando, pCont, pExt, pA, pB, pTrilho, pJanela, http, cedo, parado, paradoTxt };
}
var agoraPl = () => Date.now() + (S.pl?.desvio || 0);
function promoAtual() {
  if (!S.estado) return null;
  const p = proximaDe(S.pl);
  if (!p) return null;
  const agora = agoraPl();
  if (agora >= p.fim) return null;
  const aoVivo = agora >= p.inicio;
  if (modoDe(S.estado) === "ao_vivo") return null;
  return { ...p, agora, aoVivo };
}
function textoExterior(p) {
  if (!/Elei[çc][õo]es Gerais/i.test(p.nome) || p.aoVivo) return "";
  return p.agora < p.inicio - 16 * 36e5 ? "BUs do exterior a partir da 01h" : "BUs do exterior já podem chegar";
}
var menosMovimento = window.matchMedia?.("(prefers-reduced-motion: reduce)");
function rolarPromo(f2) {
  const larg = f2.pA.scrollWidth, janela = f2.pJanela.clientWidth;
  const rola = larg > janela + 1 && !menosMovimento?.matches;
  if (f2.promo.classList.contains("rolando") !== rola) f2.promo.classList.toggle("rolando", rola);
  if (!rola) {
    if (f2.pB.childNodes.length) f2.pB.replaceChildren();
    return;
  }
  if (f2.pB.innerHTML !== f2.pA.innerHTML) f2.pB.innerHTML = f2.pA.innerHTML;
  const passo = larg + 40;
  if (Math.abs((f2.passo || 0) - passo) > 4) {
    f2.passo = passo;
    f2.pTrilho.style.setProperty("--passo", `${passo}px`);
    f2.pTrilho.style.setProperty("--dur", `${(passo / 40).toFixed(1)}s`);
  }
}
function textoPromo(p) {
  const q = quandoTxt(p.inicio, p.agora);
  const cont = p.aoVivo ? null : contagemPartes(p.inicio - p.agora);
  return { q, cont, linha: `${p.nome}: ${q.longo}`, falta: cont ? `faltam ${cont.join(" ")}` : "agora" };
}
function agoraSite() {
  const r = S.relogio;
  return r && Number.isFinite(r.t) ? r.t + (performance.now() - r.em) / 1e3 * (r.x || 0) : null;
}
function inicioOficial() {
  for (const m of S.marcos.values()) {
    if (m.classe !== "primeira_secao" || m.abrangencia !== "br") continue;
    const t = segundos(m.horario);
    if (t != null) return Math.floor(t / 86400) * 86400 + 17 * 3600;
  }
  const p = proximaDe(S.pl);
  return p ? p.inicio / 1e3 - 3 * 3600 : null;
}
function antesDas17() {
  const m = modoDe(S.estado);
  if (m !== "ao_vivo" && m !== "replay" || S.estado?.encerrada) return false;
  const t0 = inicioOficial(), agora = agoraSite();
  return t0 != null && agora != null && agora < t0 && agora >= t0 - 17 * 3600;
}
var CEDO = { titulo: "Totalização antes das 17h", linhas: [
  ["Oficial", "No Brasil, a totalização dos votos começa oficialmente às 17h (horário de Brasília), quando termina a votação em todo o país."],
  ["Exterior", "Os eleitores no exterior votam no horário local. Onde a votação termina antes, como na Nova Zelândia, na Austrália, na Ásia e na Europa, o TSE publica os boletins de urna (BUs) a partir de 1h da madrugada de domingo."],
  ["Urna Velox", "O Urna Velox totaliza cada boletim de urna assim que o TSE o publica. Por isso começa a totalizar antes das 17h: até lá, os números são só dos eleitores do exterior."]
] };
function desenharCedo() {
  const cedo = antesDas17(), f2 = faixa;
  if (f2.cedo.classList.contains("oculto") === cedo) f2.cedo.classList.toggle("oculto", !cedo);
  const it = feed.porId.get("cedo");
  if (!cedo) {
    if (it) it.sair = true;
    return;
  }
  feed.por({ id: "cedo", fixo: true, cls: "cedo", ic: "i", texto: "Totalização oficial às 17h: por enquanto, só os boletins de urna do exterior", t: "", detalhe: CEDO });
  const x = feed.porId.get("cedo");
  if (x) x.sair = false;
}
var PARADO_S = 600;
function ultimoBuSite() {
  const a = segundosBr(S.estado?.ultimo_bu?.site), b = segundos(S.res.get(S.tela?.topico)?.ultimo_bu);
  return a == null ? b : b == null ? a : Math.max(a, b);
}
function tseParado() {
  if (modoDe(S.estado) !== "ao_vivo" || S.estado?.encerrada) return null;
  const agora = agoraSite(), t0 = inicioOficial(), tb = ultimoBuSite();
  if (agora == null || t0 == null || tb == null || agora < t0) return null;
  return agora - Math.max(tb, t0) >= PARADO_S ? tb : null;
}
function desenharParado() {
  const tb = tseParado(), f2 = faixa;
  if (f2.parado.classList.contains("oculto") === (tb != null)) f2.parado.classList.toggle("oculto", tb == null);
  if (tb == null) return;
  const d = new Date(tb * 1e3), hh = String(d.getUTCHours()).padStart(2, "0"), mm = String(d.getUTCMinutes()).padStart(2, "0");
  const txt = `TSE sem novos BUs desde ${hh}h${mm}`;
  if (f2.paradoTxt.textContent !== txt) f2.paradoTxt.textContent = txt;
}
function desenharPromo() {
  desenharParado();
  desenharCedo();
  const p = promoAtual();
  const f2 = faixa;
  if (f2.promo.classList.contains("oculto") === !!p) f2.promo.classList.toggle("oculto", !p);
  const it = feed.porId.get("promo:ao-vivo");
  if (!p) {
    if (it) it.sair = true;
    return;
  }
  const t = textoPromo(p);
  const nome = p.nome.replace(/^(\dº turno) das /, "$1 · ");
  if (f2.pNome.textContent !== nome) f2.pNome.textContent = nome;
  const quando2 = p.aoVivo ? "agora" : t.q.longo;
  if (f2.pQuando.dataset.v !== quando2) {
    f2.pQuando.replaceChildren(...p.aoVivo ? ["agora"] : [el("span", "dia", t.q.dia), `(${t.q.ddmm}), ${t.q.hora}`]);
    f2.pQuando.dataset.v = quando2;
  }
  const contTxt = t.cont ? t.cont.join(" ") : "";
  if (f2.pCont.dataset.v !== contTxt) {
    f2.pCont.replaceChildren(...t.cont ? [el("span", "longo", "faltam"), el("b", null, `${t.cont[0]} ${t.cont[1]}`), el("span", "seg", t.cont[2] || "")] : []);
    f2.pCont.dataset.v = contTxt;
  }
  const ext = textoExterior(p);
  const extTxt = ext ? `· ${ext}` : "";
  if (f2.pExt.textContent !== extTxt) f2.pExt.textContent = extTxt;
  rolarPromo(f2);
  const titulo2 = p.aoVivo ? `AO VIVO agora: ${p.nome}` : `Próxima eleição AO VIVO: ${t.linha} · ${t.falta}${ext ? ` · ${ext}` : ""}`;
  if (f2.promo.title !== titulo2) {
    f2.promo.title = titulo2;
    f2.promo.setAttribute("aria-label", `${titulo2}. Toque para saber mais.`);
  }
  feed.por({
    id: "promo:ao-vivo",
    fixo: true,
    cls: "promo",
    ic: "●",
    texto: p.aoVivo ? `AO VIVO agora: ${p.nome}` : `Próxima eleição AO VIVO: ${t.linha}${ext ? ` · ${ext}` : ""}`,
    t: p.aoVivo ? "" : t.falta,
    detalhe: AVISO
  });
  const x = feed.porId.get("promo:ao-vivo");
  if (x) x.sair = false;
}
function detalheAviso() {
  const e = S.estado;
  const modo = modoDe(e);
  const linhas = [];
  let titulo2 = "Urna Velox";
  if (modo === "replay") {
    titulo2 = "REPLAY: dados reais, totalizados de novo";
    linhas.push(
      ["Replay", "A apuração de uma eleição que já aconteceu, passando de novo — como a reprise de um jogo."],
      ["Dados", "Os boletins de urna reais, publicados pelo TSE: nenhum voto é inventado. A totalização recomeça do zero, na ordem em que os boletins chegaram à Justiça Eleitoral, e no fim é conferida com o resultado oficial."],
      ["No ar", e.simulacao.rotulo]
    );
  } else if (modo === "espera") {
    titulo2 = "A apuração ainda não começou";
    linhas.push(["No ar", S.pl?.no_ar?.modo === "espera" && S.pl.no_ar.modo_descricao ? S.pl.no_ar.modo_descricao : "Os candidatos já carregados, à espera dos primeiros boletins de urna."]);
  } else if (modo === "ao_vivo") {
    titulo2 = e?.encerrada ? "Apuração encerrada" : "AO VIVO: totalização em tempo real";
    linhas.push(["No ar", "A totalização da eleição com os boletins de urna publicados pelo TSE, à medida que chegam."]);
  }
  const p = promoAtual();
  if (p) {
    const t = textoPromo(p);
    if (modo === "espera" || modo === "") titulo2 = p.aoVivo ? `AO VIVO agora: ${p.nome}` : "Próxima eleição AO VIVO";
    linhas.unshift([
      el("span", "aviso-selo", "AO VIVO"),
      el("strong", "aviso-vivo", p.aoVivo ? [`Agora: ${p.nome}`] : [p.nome, el("br"), `${t.q.dia} (${t.q.ddmm}), a partir das ${t.q.hora} · ${t.falta}`])
    ]);
    if (textoExterior(p)) {
      linhas.splice(1, 0, ["Exterior", `A partir da 01h do dia da eleição (${t.q.ddmm}), o TSE já pode publicar os boletins de urna (BUs) do exterior, e o Urna Velox totaliza cada um assim que sai. A totalização oficial no Brasil começa às 17h.`]);
    }
  }
  if (S.pl && !$2("#conexao")?.hidden) {
    const link = el("a", null, "▶ Playlist: a agenda dos replays e a votação do próximo");
    link.href = "/playlist";
    linhas.push(["Playlist", link]);
  }
  return { titulo: titulo2, linhas };
}
var AVISO = { get titulo() {
  return detalheAviso().titulo;
}, get linhas() {
  return detalheAviso().linhas;
} };
function abrirAviso() {
  const d = detalheAviso();
  if (d.linhas.length) folha.abrir(d);
}
function talvezAvisar() {
  if (S.avisado || !S.estado || !S.plPronta) return;
  const modo = modoDe(S.estado);
  if (modo === "ao_vivo") {
    S.avisado = true;
    return;
  }
  if (modo !== "replay" && !promoAtual()) return;
  S.avisado = true;
  const hoje = diaLocal();
  let visto = null;
  try {
    visto = localStorage.getItem("uv-aviso-dia");
  } catch {
  }
  if (visto === hoje) return;
  try {
    localStorage.setItem("uv-aviso-dia", hoje);
  } catch {
  }
  abrirAviso();
}
async function acompanharPlaylist() {
  const prazo = new Promise((ok) => setTimeout(ok, 3e3));
  const primeira = buscarPlaylist();
  const pl = await Promise.race([primeira, prazo.then(() => void 0)]);
  if (pl !== void 0) S.pl = pl;
  S.plPronta = true;
  S.estadoSujo = true;
  pedirQuadro();
  if (pl === void 0) {
    S.pl = await primeira;
    S.estadoSujo = true;
    pedirQuadro();
  }
  const proxima = () => setTimeout(async () => {
    const n = await buscarPlaylist();
    if (n) S.pl = n;
    S.estadoSujo = true;
    pedirQuadro();
    proxima();
  }, S.pl ? 15 * 6e4 + Math.random() * 6e4 : 25e3 + Math.random() * 1e4);
  proxima();
}
function desenharEstado() {
  S.estadoSujo = false;
  const e = S.estado;
  const c = $2("#conexao");
  const modo = S.conexao === "vivo" || e ? modoDe(e) : "";
  const http = S.conexao === "http";
  const ligada = S.conexao === "vivo" || http;
  const nomeModo = { replay: "REPLAY", ao_vivo: "AO VIVO", espera: "EM BREVE" }[modo] || "";
  let est = S.conexao, txt = nomeModo || "conectando…";
  if (S.conexao === "conectando" && !e) txt = "conectando…";
  if (S.conexao === "caiu") {
    const s = Math.max(0, Math.round(((S.conexaoInfo?.em || Date.now()) - Date.now()) / 1e3));
    txt = nomeModo ? `${nomeModo} · reconectando ${s}s` : `conexão caiu · volta em ${s}s`;
    clearTimeout(S.tCaiu);
    S.tCaiu = setTimeout(() => {
      S.estadoSujo = true;
      pedirQuadro();
    }, 1e3);
  }
  if (http && !e) txt = "a cada minuto";
  if (ligada && e?.pausa_429?.ativa) {
    est = "pausa";
    txt = `${nomeModo ? `${nomeModo} · ` : ""}TSE pediu pausa`;
  }
  if (ligada && e?.encerrada) {
    est = "encerrada";
    txt = modo === "replay" ? "REPLAY · fim" : "apuração encerrada";
  }
  if (faixa?.http && faixa.http.classList.contains("oculto") === http) {
    faixa.http.classList.toggle("oculto", !http);
    faixa.ritmo.classList.toggle("com-http", http);
  }
  if (c.dataset.estado !== est) c.dataset.estado = est;
  if (c.dataset.modo !== modo) c.dataset.modo = modo;
  const t = c.querySelector(".txt");
  if (t.textContent !== txt) t.textContent = txt;
  const pl = c.querySelector(".pl");
  const comPl = !!S.pl && modo !== "ao_vivo";
  if (pl.classList.contains("oculto") === comPl) pl.classList.toggle("oculto", !comPl);
  const rot = e?.simulacao?.rotulo || "";
  const dica = modo === "replay" ? `REPLAY com dados reais: os boletins de urna são os publicados pelo TSE, totalizados de novo, do zero, na ordem em que chegaram à Justiça Eleitoral (${rot}); no fim, o resultado é conferido com o oficial. O relógio mostra o horário da noite da eleição.` : modo === "espera" ? "A apuração ainda não começou." : modo === "ao_vivo" ? "Totalização ao vivo." : "";
  const dicaHttp = http ? ` ${TEXTO_HTTP}` : "";
  if (c.title !== dica + dicaHttp) {
    c.title = dica + dicaHttp;
    c.setAttribute("aria-label", `${txt}${dica + dicaHttp ? `. ${dica}${dicaHttp}` : ""} Toque para saber mais${comPl ? " e ver a playlist" : ""}.`);
  }
  talvezAvisar();
  desenharPromo();
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
  if (al.reenvio_diferente || al.anulacao) avisos.push("houve BU reenviado ou seção anulada");
  const at = avisos.length ? `Atenção: ${avisos.join("; ")}` : "";
  if (faixa.alerta.textContent !== at) {
    faixa.alerta.textContent = at;
    faixa.alerta.title = at;
    faixa.alerta.classList.toggle("oculto", !at);
  }
}
function amostrarRitmo(seq, t, sessao) {
  if (t == null) return;
  if (sessao !== S.ritmoSessao) {
    S.ritmoSessao = sessao;
    S.ritmoSrv = [];
  }
  const r = S.ritmoSrv, u = r[r.length - 1];
  if (u && (t * 1e3 <= u[0] || seq < u[1])) return;
  r.push([t * 1e3, seq]);
  while (r.length > 2 && t * 1e3 - r[0][0] > 6e4) r.shift();
}
function curiosidades() {
  const agora = performance.now();
  if (S.seqMax != null) S.ritmo.push([agora, S.seqMax]);
  while (S.ritmo.length > 2 && agora - S.ritmo[0][0] > 6e4) S.ritmo.shift();
  const r = S.ritmoSessao !== void 0 ? S.ritmoSrv : S.ritmo;
  if (r.length >= 2) {
    const [t0, s0] = r[0], [t1, s1] = r[r.length - 1];
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
  $2("#conexao").addEventListener("click", () => abrirAviso());
  acompanharPlaylist();
  setInterval(() => desenharPromo(), 1e3);
  S.ident = new Identificacao();
  S.N = new Nomes(nomes);
  S.topicos = new Set(topicos.map((t) => t.topico));
  S.con = new Conexao(receber, estadoConexao);
  const versao = new URL(import.meta.url).searchParams.get("v") || "";
  S.mapa = new CartaoMapa({ obter: (c) => S.con.obter(c), pedirMapa: (f2) => S.con.pedirMapa(f2), nomes: () => S.N, versao });
  S.con.aoConsultar = () => S.mapa?.consultar();
  S.cartoes = new Cartoes({ aoMaximizar: (c, sim) => {
    if (c === S.mapa?.el) S.mapa.maximizado(sim);
  } });
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
