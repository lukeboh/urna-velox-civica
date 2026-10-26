// Mapa das seções: cada seção totalizada acende um ponto. O servidor numera as seções do catálogo e diz quais já
// acenderam (docs/PROTOCOLO_WS.md §4.8: GET /api/v1/mapa/secoes, GET /api/v1/mapa/estado e `op: mapa` no WebSocket);
// a geografia vem de /static/mapa.json (site/geo/gerar_mapa.py): contorno das UFs, centro dos municípios, locais de
// votação (se houver), malha dos continentes e cidades do exterior.
//
// Duas vistas: "Brasil" (a silhueta do país, escura, com as luzes por cima) e "Exterior" (globo: esfera translúcida que se gira,
// com o exterior; cada BU de fora viaja num arco até Brasília). Tudo em canvas: as luzes acesas ficam numa camada que só
// recebe os pontos novos; as explosões e o globo são redesenhados por quadro, só enquanto o cartão está na tela.
// Nada pula: o palco tem proporção fixa e as linhas de texto, altura reservada.

import { Hodometro } from './hodometro.js';
import { el, mil, movimentoReduzido } from './util.js';

const PREF = 'uv-mapa';
const ler = () => { try { return JSON.parse(localStorage.getItem(PREF)) || {}; } catch { return {}; } };
const gravar = (o) => { try { localStorage.setItem(PREF, JSON.stringify(o)); } catch { /* sem armazenamento */ } };
const rad = Math.PI / 180;
const vet = (lat, lon) => [Math.cos(lat * rad) * Math.cos(lon * rad), Math.cos(lat * rad) * Math.sin(lon * rad), Math.sin(lat * rad)];
const BSB = vet(-15.79, -47.88);
const GRAD = [[79, 195, 255], [140, 107, 255], [255, 95, 210]];
/** Cor da faixa de horário (1–240, 6 min cada): ciano às 16h, violeta às 18h30, rosa das 21h em diante. */
function corFaixa(f, a) {
  const min = (f - 1) * 6 - 960;
  const x = Math.max(0, Math.min(1, min / 300)) * 2, i = Math.min(1, Math.floor(x)), u = x - i;
  const c = GRAD[i].map((v, j) => Math.round(v + (GRAD[i + 1][j] - v) * u));
  return `rgba(${c[0]},${c[1]},${c[2]},${a})`;
}
/** Ruído determinístico de uma seção (a mesma posição em toda visita). */
function hash(a, b, c) {
  let h = Math.imul(a ^ 0x9e3779b9, 0x85ebca6b) ^ Math.imul(b + 0x632be5ab, 0xc2b2ae35) ^ Math.imul(c + 0x27d4eb2f, 0x165667b1);
  h ^= h >>> 15; h = Math.imul(h, 0x2c1b3c6d); h ^= h >>> 12; h = Math.imul(h, 0x297a2d39); h ^= h >>> 15;
  return (h >>> 0) / 4294967296;
}
const normNome = (s) => String(s || '').normalize('NFD').replace(/[̀-ͯ]/g, '').toUpperCase().replace(/[-']/g, ' ').replace(/\s+/g, ' ').trim();

export class CartaoMapa {
  /**
   * @param {{obter: (caminho: string, op?: object) => Promise<Response|null>, pedirMapa: (f: () => object) => void, nomes: () => any,
   *          versao: string, aoMarco?: (txt: string) => void}} op
   */
  constructor(op) {
    this.op = op;
    const p = ler();
    this.vista = p.vista === 'globo' ? 'globo' : 'brasil';
    this.cor = p.cor === 'hora' ? 'hora' : 'luz';
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
      const g = el('div', 'mp-modo');
      g.setAttribute('role', 'group');
      g.setAttribute('aria-label', rotulo);
      const bs = itens.map(([v, t]) => {
        const b = el('button', null, t);
        b.type = 'button';
        b.dataset.v = v;
        b.setAttribute('aria-pressed', String(v === atual));
        b.addEventListener('click', () => { bs.forEach((x) => x.setAttribute('aria-pressed', String(x === b))); aoMudar(v); });
        return b;
      });
      g.append(...bs);
      return g;
    };
    this.hodCaixa = el('span', 'mp-n', '—');
    this.deTxt = el('small', null, '');
    this.extTxt = el('div', 'mp-ext', '');
    const acesas = el('div', 'mp-acesas', [el('div', 'mp-rot', 'Seções acesas'), el('div', 'mp-val', [this.hodCaixa, this.deTxt]), this.extTxt]);
    const ctl = el('div', 'mp-ctl', [
      botoes('Vista do mapa', [['brasil', 'Brasil'], ['globo', 'Exterior']], this.vista, (v) => { this.vista = v; this.#salvar(); this.#aplicarVista(); }),
      botoes('Cor dos pontos', [['luz', 'Luz'], ['hora', 'Horário']], this.cor, (v) => { this.cor = v; this.#salvar(); this.legenda.classList.toggle('ligado', v === 'hora'); this.#redesenhar(); }),
    ]);
    this.cv = {};
    const palco = el('div', 'mp-palco');
    for (const n of ['base', 'acc', 'globo', 'brilho', 'fx']) { const c = el('canvas', `mp-${n}`); c.setAttribute('aria-hidden', 'true'); this.cv[n] = c; palco.append(c); }
    this.cv.fx.removeAttribute('aria-hidden');
    this.cv.fx.setAttribute('role', 'img');
    this.legenda = el('div', 'mp-legenda', [el('i'), el('span', null, [el('em', null, '16h'), el('em', null, '18h30'), el('em', null, '21h')])]);
    this.legenda.classList.toggle('ligado', this.cor === 'hora');
    this.chip = el('div', 'mp-marco', '');
    this.aviso = el('div', 'mp-aviso', 'Carregando o mapa…');
    palco.append(this.legenda, this.chip, el('div', 'mp-dica', 'arraste para girar · duplo clique volta ao Brasil'), this.aviso);
    this.palco = palco;
    this.ultimo = el('div', 'mp-ultimo', [el('span', 'mp-pt'), el('span', 'mp-ult-txt', 'Aguardando o primeiro BU…')]);
    this.ultTxt = this.ultimo.lastChild;
    this.ufs = el('div', 'mp-ufs', el('h3', null, 'Seções acesas por UF'));
    this.corpo = el('div', 'mp-corpo', [el('div', 'mp-topo', [acesas, ctl]), palco, this.ultimo, this.ufs]);
    this.el = el('section', 'vidro mapa', [el('h2', 'cartao-titulo', 'Mapa das seções'), this.corpo]);
    this.el.setAttribute('aria-label', 'Mapa das seções');
    this.#aplicarVista();
    this.#interacao();
    // só busca e desenha quando o cartão aparece (e para de desenhar quando sai)
    this.io = new IntersectionObserver((es) => {
      this.visivel = es.some((e) => e.isIntersecting) || this.el.classList.contains('max');
      if (this.visivel) { this.#carregar(); this.#pedirQuadro(); }
    }, { rootMargin: '200px' });
    this.io.observe(this.el);
    this.ro = new ResizeObserver(() => this.#redesenhar());
    this.ro.observe(palco);
  }

  #salvar() { gravar({ vista: this.vista, cor: this.cor }); }

  #aplicarVista() {
    this.palco.classList.toggle('globo', this.vista === 'globo');
    this.cv.fx.setAttribute('aria-label', this.vista === 'globo'
      ? 'Globo com um ponto de luz para cada seção totalizada, no Brasil e no exterior; arraste para girar'
      : 'Mapa do Brasil com um ponto de luz para cada seção totalizada');
    this.#atualizarBrilho();
    this.#pedirQuadro();
  }

  /** O cartão foi maximizado ou restaurado (cartoes.js): desenha mesmo fora da área visível e mostra as UFs. */
  maximizado(sim) {
    this.visivel = sim || this.visivel;
    if (sim) { this.#carregar(); this.#painelUfs(); }
    this.#pedirQuadro();
  }

  // ---------------------------------------------------------------- dados
  async #json(caminho, mesmaOrigem) {
    try {
      const r = mesmaOrigem ? await fetch(caminho) : await this.op.obter(caminho);
      return r && r.ok ? await r.json() : null;
    } catch { return null; }
  }

  async #carregar() {
    if (this.carregando) return;
    this.carregando = true;
    const [geo, lista, est] = await Promise.all([this.#json(`/static/mapa.json?v=${this.op.versao}`, true), this.#json('/api/v1/mapa/secoes'), this.#json('/api/v1/mapa/estado')]);
    if (!geo || !lista || !est) {
      this.aviso.textContent = 'O mapa das seções não está disponível neste servidor.';
      this.carregando = false;
      setTimeout(() => { if (!this.carregado) this.#carregar(); }, 60000);
      return;
    }
    this.geo = geo;
    this.#decodificar(lista);
    this.#posicionar();
    this.#fotografia(est);
    this.carregado = true;
    this.aviso.classList.add('oculto');
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
      this.munCod.push(cod); this.munUf.push(uf);
      const i0 = i;
      for (let z = 0; z < nz; z++) {
        const zona = m[p++], ns = m[p++];
        let s = 0;
        for (let k = 0; k < ns; k++) { s = k ? s + m[p++] : m[p++]; this.mu[i] = mi; this.zona[i] = zona; this.secao[i] = s; i++; }
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
    while (lo <= hi) { const md = (lo + hi) >> 1, v = ch(md); if (v === alvo) return md; if (v < alvo) lo = md + 1; else hi = md - 1; }
    return -1;
  }

  /** Posição de cada seção: local de votação (se houver), senão em volta do centro do município; exterior pelo nome. */
  #posicionar() {
    const g = this.geo, N = this.N;
    const centro = new Map();
    for (let i = 0; i < g.mun.length; i += 3) centro.set(g.mun[i], [g.mun[i + 1] / 10, g.mun[i + 2] / 10]);
    const locais = new Map(); // "mun/zona" → [[s0, x, y], …] em ordem de s0 (x = y = 0: sem coordenada)
    const L = g.locais || [];
    if (g.locais_v === 2) {
      // v2: mu, zona, m, m × (Δs0, x, y), posição em 1/locais_div do plano
      const dv = g.locais_div || 10;
      for (let i = 0; i < L.length;) {
        const m = L[i + 2], v = [];
        let s0 = 0;
        for (let j = 0; j < m; j++) { const b = i + 3 + j * 3; s0 += L[b]; v.push([s0, L[b + 1] / dv, L[b + 2] / dv]); }
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
    this.cid = new Int16Array(N).fill(-1);   // exterior: índice da cidade
    this.cidades = [];                       // [nome, lat, lon, vetor]
    const cidIdx = new Map();
    this.ext = new Uint8Array(N);
    this.nExt = 0;
    this.noLocal = 0;                        // seções postas no local de votação (as demais, em volta do município)
    for (let i = 0; i < N; i++) {
      const mi = this.mu[i], cod = this.munCod[mi], uf = this.munUf[mi];
      const a = hash(cod, this.zona[i], this.secao[i]), b = hash(this.secao[i], cod, this.zona[i] + 7);
      if (uf === 'zz') {
        this.ext[i] = 1; this.nExt++;
        const nome = nomes[`zz${String(cod).padStart(5, '0')}`] || '';
        let ci = cidIdx.get(cod);
        if (ci === undefined) {
          const ll = g.exterior[normNome(nome)];
          ci = ll ? this.cidades.length : -1;
          if (ll) this.cidades.push([nome, ll[0] / 100, ll[1] / 100, vet(ll[0] / 100, ll[1] / 100), 0]);
          else this.semPosicao.push(nome || `zz${cod}`);
          cidIdx.set(cod, ci);
        }
        this.cid[i] = ci;
        this.X[i] = -1e4; this.Y[i] = -1e4;
        continue;
      }
      // gaussiana pelo método de Box-Muller com os dois ruídos da seção
      const r = Math.sqrt(-2 * Math.log(1 - a * 0.999)), t = 2 * Math.PI * b, gx = r * Math.cos(t), gy = r * Math.sin(t);
      const lv = locais.get(`${cod}/${this.zona[i]}`);
      if (lv) {
        let lo = 0, hi = lv.length - 1, k = -1;
        while (lo <= hi) { const md = (lo + hi) >> 1; if (lv[md][0] <= this.secao[i]) { k = md; lo = md + 1; } else hi = md - 1; }
        if (k >= 0 && (lv[k][1] || lv[k][2])) { this.X[i] = lv[k][1] + gx * 0.12; this.Y[i] = lv[k][2] + gy * 0.12; this.noLocal++; continue; }
      }
      const c = centro.get(cod) || g.centros[uf] || [500, 433];
      const n = this.munN[mi];
      const sig = (hash(cod, this.secao[i], 3) < 0.18 ? 2.6 : 1) * (0.28 + Math.sqrt(n) * 0.032) + (hash(cod, this.secao[i], 3) < 0.18 ? 1.2 : 0);
      this.X[i] = c[0] + gx * sig; this.Y[i] = c[1] + gy * sig;
    }
    // globo: o Brasil agrupado numa grade do plano (6 unidades ≈ 0,27°)
    const P = g.plano, GS = 6;
    this.gc = Math.ceil(P.largura / GS); this.gr = Math.ceil(P.altura / GS) + 1;
    this.bin = new Uint32Array(N);
    this.binN = new Uint32Array(this.gc * this.gr);
    this.binAtivos = [];
    this.BV = new Float32Array(this.gc * this.gr * 3);
    for (let b = 0; b < this.gc * this.gr; b++) {
      const x = ((b % this.gc) + 0.5) * GS, y = (Math.floor(b / this.gc) + 0.5) * GS;
      this.BV.set(vet(P.lat1 - (y * P.span) / P.largura, P.lon0 + (x * P.span) / P.largura), b * 3);
    }
    for (let i = 0; i < N; i++) {
      if (this.ext[i]) continue;
      const c = Math.max(0, Math.min(this.gc - 1, Math.floor(this.X[i] / GS))), r = Math.max(0, Math.min(this.gr - 1, Math.floor(this.Y[i] / GS)));
      this.bin[i] = r * this.gc + c;
    }
    this.MV = new Float32Array((g.mundo.length / 2) * 3);
    for (let i = 0; i < g.mundo.length / 2; i++) this.MV.set(vet(g.mundo[i * 2 + 1] / 10, g.mundo[i * 2] / 10), i * 3);
    this.paths = g.ufs.map(([, d]) => new Path2D(d));
    this.brasil = new Path2D(g.ufs.map(([, d]) => d).join(''));
    this.acesasUf = new Map();
    this.totUf = new Map();
    for (let i = 0; i < N; i++) { const u = this.munUf[this.mu[i]]; this.totUf.set(u, (this.totUf.get(u) || 0) + 1); }
    this.totUfLista = new Map(this.totUf);
    this.nExtLista = this.nExt;
    if (this.semPosicao.length) console.info(`mapa: ${this.semPosicao.length} cidades do exterior sem posição (fora do globo):`, this.semPosicao.join(', '));
  }

  #acender(i, f) {
    this.hora[i] = f;
    const u = this.munUf[this.mu[i]];
    this.acesasUf.set(u, (this.acesasUf.get(u) || 0) + 1);
    if (this.ext[i]) { this.extAcesas++; if (this.cid[i] >= 0) this.cidades[this.cid[i]][4]++; return; }
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
    this.cidades.forEach((c) => { c[4] = 0; });
    this.extAcesas = 0; this.brAcesas = 0;
    const n = Math.min(bin.length, this.N);
    for (let i = 0; i < n; i++) { const f = bin.charCodeAt(i); if (f) this.#acender(i, f); }
    // B-064: seções que se revelaram agregadas na noite saem do total (o índice continua na lista do processo)
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
    const est = await this.#json('/api/v1/mapa/estado');
    if (!est) return;
    this.#fotografia(est);
    this.#redesenhar();
    this.#textos();
    this.op.pedirMapa(() => ({ desde: this.n, epoca: this.epoca }));
  }

  /** Modo HTTP (B-026, a cada consulta): relê a fotografia e acende o que é novo. */
  async consultar() {
    if (!this.carregado) return;
    const est = await this.#json('/api/v1/mapa/estado');
    if (!est) return;
    if (est.epoca !== this.epoca || (est.total ?? this.total) !== this.total) { this.#fotografia(est); this.#redesenhar(); this.#textos(); return; }
    if (est.n <= this.n) return;
    const bin = atob(est.h), novas = [];
    for (let i = 0; i < Math.min(bin.length, this.N); i++) { const f = bin.charCodeAt(i); if (f && !this.hora[i]) { this.#acender(i, f); novas.push(i); } }
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
    // B-064: o total mudou (agregada da noite): relê a fotografia, que diz quais seções saíram
    if (m.reiniciar || (m.epoca != null && m.epoca !== this.epoca) || m.de > this.n || (m.total ?? this.total) !== this.total) { this.#recarregar(); return; }
    if (m.n <= this.n) return;
    const s = m.s, ini = (this.n - m.de) * 2, novas = [];
    for (let k = ini; k < s.length; k += 2) {
      const i = s[k], f = s[k + 1];
      if (i >= this.N || this.hora[i]) continue;
      this.#acender(i, f);
      novas.push(i);
    }
    this.n = m.n;
    if (!novas.length) return;
    this.#pintar(novas);
    const agora = performance.now();
    // explosões: no máximo ~30 por mensagem (a 60× chegam milhares); todo BU do exterior ganha o seu arco
    const p = this.reduzido ? 0 : Math.min(1, 30 / novas.length);
    for (const i of novas) {
      if (this.ext[i]) { if (!this.reduzido && this.cid[i] >= 0) this.arcos.push({ c: this.cid[i], t0: agora + Math.random() * 300 }); continue; }
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
    if (!this.carregado || !['primeira_secao', 'bu_marco', 'ultima_secao'].includes(m.classe) || m.municipio == null) return;
    const i = this.indice(m.municipio, m.zona, m.secao);
    if (i < 0) return;
    const nomes = this.op.nomes();
    const lugar = nomes ? nomes.local(`${m.uf}${String(m.municipio).padStart(5, '0')}`) : '';
    const txt = m.classe === 'bu_marco' ? `BU nº ${mil(m.valor)} da noite: ${lugar}`
      : m.classe === 'ultima_secao' ? `Última seção: ${lugar}`
        : m.abrangencia === 'br' ? `1ª seção do Brasil: ${lugar}` : `1ª seção ${nomes ? nomes.em(m.abrangencia) : ''}: ${lugar}`;
    if (!this.reduzido && !this.ext[i]) this.fx.push({ i, t0: performance.now(), r: 3.2, grande: true });
    this.marcos.push({ i, txt });
    if (this.marcos.length > 3) this.marcos.splice(0, this.marcos.length - 3);
    this.#pedirQuadro();
  }

  // ---------------------------------------------------------------- desenho
  #dimensionar() {
    // tamanho de layout (sem a inclinação tátil nem o `scale` da animação de maximizar)
    const r = { width: this.palco.offsetWidth, height: this.palco.offsetHeight };
    if (!r.width || !r.height) return false;
    this.dpr = Math.min(2, window.devicePixelRatio || 1);
    const W = Math.round(r.width * this.dpr), H = Math.round(r.height * this.dpr);
    if (W === this.W && H === this.H && this.esc) return true;
    this.W = W; this.H = H;
    for (const n of ['base', 'acc', 'globo', 'fx']) { this.cv[n].width = this.W; this.cv[n].height = this.H; }
    this.cv.brilho.width = Math.ceil(this.W / 3); this.cv.brilho.height = Math.ceil(this.H / 3);
    const P = this.geo.plano;
    this.esc = Math.min(this.W / P.largura, this.H / P.altura) * 0.94;
    this.ox = (this.W - P.largura * this.esc) / 2; this.oy = (this.H - P.altura * this.esc) / 2;
    return true;
  }

  #redesenhar() {
    if (!this.carregado || !this.#dimensionar()) return;
    const c = this.cv.base.getContext('2d'), d = this.dpr;
    c.setTransform(1, 0, 0, 1, 0, 0); c.clearRect(0, 0, this.W, this.H);
    c.setTransform(this.esc, 0, 0, this.esc, this.ox, this.oy);
    c.lineJoin = 'round';
    // o Brasil inteiro escuro, com um halo da aurora na borda (o cartão continua com o vidro do portal)
    // um caminho só (as UFs juntas): sem emendas nem o halo de uma UF por cima da vizinha
    c.save(); c.shadowColor = 'rgba(120,100,255,0.45)'; c.shadowBlur = 28 * d; c.fillStyle = 'rgba(5,8,26,0.84)';
    c.fill(this.brasil);
    c.restore();
    c.strokeStyle = 'rgba(160,180,255,0.12)'; c.lineWidth = (0.7 / this.esc) * d;
    for (const p of this.paths) c.stroke(p);
    const a = this.cv.acc.getContext('2d');
    a.setTransform(1, 0, 0, 1, 0, 0); a.globalCompositeOperation = 'source-over'; a.clearRect(0, 0, this.W, this.H);
    const todas = [];
    for (let i = 0; i < this.N; i++) if (this.hora[i] && !this.ext[i]) todas.push(i);
    // em ordem de horário, para a cor da faixa mudar pouco (menos trocas de fillStyle)
    if (this.cor === 'hora') todas.sort((x, y) => this.hora[x] - this.hora[y]);
    this.#pintar(todas);
    this.#atualizarBrilho();
    this.#pedirQuadro();
  }

  /** Acende pontos na camada das luzes (luz aditiva: onde há muitas seções juntas, fica branco). */
  #pintar(lista) {
    if (!this.W || !lista.length) return;
    const a = this.cv.acc.getContext('2d');
    a.globalCompositeOperation = 'lighter';
    const e = this.esc / this.dpr;
    const f = Math.max(0.16, Math.min(1, Math.pow(e / 0.72, 1.4)));   // mapa pequeno: menos luz por ponto
    const s = Math.max(1, 1.15 * this.dpr * Math.sqrt(e / 0.7)), h = s / 2;
    let ult = -1;
    a.fillStyle = `rgba(150,205,255,${(0.42 * f).toFixed(3)})`;
    for (const i of lista) {
      if (this.ext[i]) continue;
      if (this.cor === 'hora' && this.hora[i] !== ult) { ult = this.hora[i]; a.fillStyle = corFaixa(ult, (0.55 * f).toFixed(3)); }
      a.fillRect(this.ox + this.X[i] * this.esc - h, this.oy + this.Y[i] * this.esc - h, s, s);
    }
  }

  #atualizarBrilho() {
    if (!this.W) return;
    const g = this.cv.brilho.getContext('2d');
    g.clearRect(0, 0, this.cv.brilho.width, this.cv.brilho.height);
    g.drawImage(this.vista === 'globo' ? this.cv.globo : this.cv.acc, 0, 0, this.cv.brilho.width, this.cv.brilho.height);
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
    const globo = this.vista === 'globo';
    if (globo) this.#desenharGlobo(agora, dt);
    if (this.brilhoSujo || globo) { if (agora - (this.tBrilho || 0) > (globo ? 60 : 180)) this.#atualizarBrilho(); }
    this.#desenharFx(agora);
    this.#mostrarMarco(agora);
    if (this.textosSujos && agora - (this.tTextos || 0) > 300) this.#textos();
    // continua enquanto há o que animar (explosões, arcos, globo girando) e o cartão está na tela
    const anima = globo || this.fx.length || this.arcos.length || this.brilhoSujo || this.textosSujos || this.marcos.length;
    if (anima && this.visivel) this.raf = requestAnimationFrame((t) => this.#quadro(t));
  }

  #textos() {
    this.textosSujos = false;
    this.tTextos = performance.now();
    this.hod?.definir(this.brAcesas + this.extAcesas);
    // B-064: o mesmo total de seções da página (sem as agregadas)
    this.deTxt.textContent = ` de ${mil(this.total ?? this.N)}`;
    // eleição sem exterior (municipal): a linha fica vazia, com a altura reservada
    if (this.nExt) this.extTxt.replaceChildren(el('span', 'mp-ptx'), `Exterior: ${mil(this.extAcesas)} de ${mil(this.nExt)} seções`);
    if (this.ult != null) {
      const i = this.ult, mi = this.mu[i], uf = this.munUf[mi], nomes = this.op.nomes();
      const lugar = nomes ? nomes.local(`${uf}${String(this.munCod[mi]).padStart(5, '0')}`) : `${uf}`;
      this.ultTxt.replaceChildren('Último BU: ', el('b', null, lugar), ` · zona ${this.zona[i]} · seção ${this.secao[i]}`);
    }
    if (this.el.classList.contains('max')) this.#painelUfs();
  }

  #painelUfs() {
    if (!this.carregado) return;
    if (!this.ufLinhas) {
      this.ufLinhas = [...this.totUf.keys()].sort().map((u) => {
        const t = el('span', 't', el('i')), v = el('span', 'v', '0%');
        const l = el('div', 'mp-uf', [el('b', null, u.toUpperCase()), t, v]);
        this.ufs.append(l);
        return { u, i: t.firstChild, v };
      });
    }
    for (const x of this.ufLinhas) {
      const f = (this.acesasUf.get(x.u) || 0) / (this.totUf.get(x.u) || 1);
      x.i.style.transform = `scaleX(${f})`;
      x.v.textContent = `${Math.floor(f * 100)}%`;
    }
  }

  #desenharFx(agora) {
    const c = this.cv.fx.getContext('2d');
    c.setTransform(1, 0, 0, 1, 0, 0); c.clearRect(0, 0, this.W, this.H);
    if (this.vista === 'globo') return;
    if (!this.fx.length) return;
    const d = this.dpr, fs = Math.min(1.3, Math.sqrt(this.esc / d / 0.72));
    c.globalCompositeOperation = 'lighter';
    let w = 0;
    for (const f of this.fx) {
      const vida = f.grande ? 2200 : 1100, e = (agora - f.t0) / vida;
      if (e >= 1) continue;
      this.fx[w++] = f;
      // B-061: o horário do quadro (requestAnimationFrame) pode ser anterior ao início da explosão: ela espera
      if (e < 0) continue;
      const x = this.ox + this.X[f.i] * this.esc, y = this.oy + this.Y[f.i] * this.esc;
      const o = 1 - Math.pow(1 - e, 3), a = Math.pow(1 - e, 2);
      const cor = this.cor === 'hora' ? corFaixa(this.hora[f.i], 1) : 'rgb(170,220,255)';
      c.globalAlpha = a * (f.grande ? 0.95 : 0.7);
      c.strokeStyle = cor; c.lineWidth = (f.grande ? 1.6 : 1) * d;
      c.beginPath(); c.arc(x, y, (2 + 13 * f.r * o) * d * fs, 0, 6.2832); c.stroke();
      if (f.grande) { c.globalAlpha = a * 0.5; c.beginPath(); c.arc(x, y, (2 + 13 * f.r * o) * d * fs * 0.55, 0, 6.2832); c.stroke(); }
      const raio = (f.grande ? 16 : 6) * d * fs;
      const g = c.createRadialGradient(x, y, 0, x, y, raio * (1 - e * 0.6));
      g.addColorStop(0, 'rgba(255,255,255,1)'); g.addColorStop(1, 'rgba(255,255,255,0)');
      c.globalAlpha = Math.pow(1 - e, 4); c.fillStyle = g;
      c.beginPath(); c.arc(x, y, raio, 0, 6.2832); c.fill();
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
    if (this.vista !== 'globo' && !this.ext[m.i]) {
      x = (this.ox + this.X[m.i] * this.esc) / this.dpr - w / 2;
      y = Math.max(12, (this.oy + this.Y[m.i] * this.esc) / this.dpr - 46);
    }
    this.chip.style.transform = `translate(${Math.max(8, Math.min(pw - w - 8, x))}px, ${y}px)`;
    this.chip.classList.add('ligado');
    clearTimeout(this.tChip);
    this.tChip = setTimeout(() => this.chip.classList.remove('ligado'), 2400);
    this.op.aoMarco?.(m.txt);
  }

  // ---------------------------------------------------------------- globo
  #desenharGlobo(agora, dt) {
    const G = this.G, c = this.cv.globo.getContext('2d'), W = this.W, H = this.H, d = this.dpr;
    if (!G.arr) {
      G.lam += (G.vl * dt) / 16; G.phi += (G.vp * dt) / 16; G.vl *= 0.94; G.vp *= 0.94;
      // parado: balança devagar em torno do Brasil (o exterior entra e sai de cena)
      if (agora - G.mexeu > 7000 && !this.reduzido) {
        const al = G.base + 38 * Math.sin(agora / 9000), ap = -14 + 6 * Math.sin(agora / 13000);
        G.lam += ((((al - G.lam + 540) % 360) - 180) * 0.012); G.phi += (ap - G.phi) * 0.012;
      }
    }
    G.phi = Math.max(-80, Math.min(80, G.phi));
    const l0 = G.lam * rad, p0 = G.phi * rad, cl = Math.cos(l0), sl = Math.sin(l0), cp = Math.cos(p0), sp = Math.sin(p0);
    const R = Math.min(W, H) * 0.41 * G.zoom, cx = W / 2, cy = H * 0.52;
    const P = [0, 0, 0];
    const proj = (x, y, z) => { const u = x * cl + y * sl, v = y * cl - x * sl; P[0] = cx + R * v; P[1] = cy - R * (cp * z - sp * u); P[2] = sp * z + cp * u; return P; };
    c.setTransform(1, 0, 0, 1, 0, 0); c.globalCompositeOperation = 'source-over'; c.globalAlpha = 1; c.clearRect(0, 0, W, H);
    const atm = c.createRadialGradient(cx, cy, R * 0.9, cx, cy, R * 1.22);
    atm.addColorStop(0, 'rgba(120,100,255,0.28)'); atm.addColorStop(0.35, 'rgba(79,195,255,0.10)'); atm.addColorStop(1, 'rgba(79,195,255,0)');
    c.fillStyle = atm; c.beginPath(); c.arc(cx, cy, R * 1.22, 0, 6.2832); c.fill();
    // esfera translúcida, mais escura no tema claro (as luzes precisam de noite)
    const claro = document.documentElement.dataset.tema === 'claro';
    const corpo = c.createRadialGradient(cx - R * 0.35, cy - R * 0.4, R * 0.1, cx, cy, R);
    corpo.addColorStop(0, claro ? 'rgba(30,36,100,0.62)' : 'rgba(60,70,150,0.16)');
    corpo.addColorStop(0.75, claro ? 'rgba(14,18,56,0.78)' : 'rgba(14,18,52,0.30)');
    corpo.addColorStop(1, claro ? 'rgba(10,14,44,0.88)' : 'rgba(10,14,44,0.55)');
    c.fillStyle = corpo; c.beginPath(); c.arc(cx, cy, R, 0, 6.2832); c.fill();
    c.strokeStyle = 'rgba(170,180,255,0.30)'; c.lineWidth = d; c.stroke();
    // continentes em malha de pontos (o verso aparece apagado, através da esfera)
    const sz = Math.max(1, 1.5 * d * Math.sqrt(G.zoom) * Math.min(1.2, R / (300 * d)));
    const MV = this.MV;
    c.fillStyle = 'rgb(170,182,255)';
    for (let i = 0; i < MV.length; i += 3) {
      proj(MV[i], MV[i + 1], MV[i + 2]);
      const z = P[2], t = z < 0 ? sz * 0.7 : sz;
      c.globalAlpha = z < 0 ? 0.05 + 0.07 * (1 + z) : 0.18 + 0.45 * z;
      c.fillRect(P[0] - t / 2, P[1] - t / 2, t, t);
    }
    // seções do Brasil acesas
    c.globalCompositeOperation = 'lighter';
    c.fillStyle = 'rgb(150,205,255)';
    const kb = Math.min(1.4, R / (320 * d)) * d, BV = this.BV;
    for (const b of this.binAtivos) {
      const n = this.binN[b];
      proj(BV[b * 3], BV[b * 3 + 1], BV[b * 3 + 2]);
      const z = P[2], t = Math.min(3.2 * d, (0.8 + 0.09 * Math.sqrt(n)) * kb);
      c.globalAlpha = Math.min(1, 0.3 + 0.06 * Math.sqrt(n)) * (z < 0 ? 0.06 : 0.35 + 0.65 * z);
      c.fillRect(P[0] - t / 2, P[1] - t / 2, t, t);
    }
    // cidades do exterior acesas (rosa da marca)
    for (const ci of this.cidades) {
      if (!ci[4]) continue;
      proj(ci[3][0], ci[3][1], ci[3][2]);
      if (P[2] < -0.05) continue;
      const r = (1.6 + 0.45 * Math.sqrt(ci[4])) * kb, x = P[0], y = P[1];
      const g = c.createRadialGradient(x, y, 0, x, y, r * 2.2);
      g.addColorStop(0, 'rgba(255,220,250,1)'); g.addColorStop(0.35, 'rgba(255,95,210,0.75)'); g.addColorStop(1, 'rgba(255,95,210,0)');
      c.globalAlpha = 0.5 + 0.5 * Math.max(0, P[2]); c.fillStyle = g;
      c.beginPath(); c.arc(x, y, r * 2.2, 0, 6.2832); c.fill();
    }
    // pulsos das seções novas do Brasil
    const pl = this.geo.plano;
    c.lineWidth = d; c.strokeStyle = 'rgb(170,220,255)';
    let w = 0;
    for (const f of this.fx) {
      const vida = f.grande ? 2200 : 1100, e = (agora - f.t0) / vida;
      if (e >= 1) continue;
      this.fx[w++] = f;
      if (e < 0) continue; // B-061
      const v = vet(pl.lat1 - (this.Y[f.i] * pl.span) / pl.largura, pl.lon0 + (this.X[f.i] * pl.span) / pl.largura);
      proj(v[0], v[1], v[2]);
      if (P[2] < 0) continue;
      c.globalAlpha = Math.pow(1 - e, 2) * (f.grande ? 0.9 : 0.6) * P[2];
      c.beginPath(); c.arc(P[0], P[1], (1.5 + 9 * f.r * (1 - Math.pow(1 - e, 3))) * kb, 0, 6.2832); c.stroke();
    }
    this.fx.length = w;
    // arcos: da cidade no exterior até Brasília, por cima da esfera
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
      pt(cauda); ax = P[0]; ay = P[1];
      for (let s = 1; s <= 36; s++) {
        const q = pt(cauda + ((cab - cauda) * s) / 36), f = s / 36;
        const vis = q[2] > 0 || Math.hypot(q[0] - cx, q[1] - cy) > R;
        c.globalAlpha = (vis ? 0.85 : 0.18) * f * (1 - Math.max(0, e - 0.8) / 0.2);
        c.strokeStyle = f > 0.7 ? 'rgb(150,220,255)' : 'rgb(255,110,215)';
        c.lineWidth = (0.8 + 1.2 * f) * d;
        c.beginPath(); c.moveTo(ax, ay); c.lineTo(q[0], q[1]); c.stroke();
        ax = q[0]; ay = q[1];
      }
      if (cab < 1) { c.globalAlpha = 1; c.fillStyle = 'rgb(255,240,255)'; c.beginPath(); c.arc(ax, ay, 2.2 * d, 0, 6.2832); c.fill(); } else {
        const ee = (e - 0.6) / 0.4;
        proj(BSB[0], BSB[1], BSB[2]);
        c.globalAlpha = (1 - ee) * 0.8; c.strokeStyle = 'rgb(255,150,225)'; c.lineWidth = 1.2 * d;
        c.beginPath(); c.arc(P[0], P[1], (3 + 14 * ee) * kb, 0, 6.2832); c.stroke();
      }
    }
    this.arcos.length = wa;
    c.globalAlpha = 1; c.globalCompositeOperation = 'source-over';
  }

  #interacao() {
    const fx = this.cv.fx, G = this.G;
    fx.addEventListener('pointerdown', (e) => {
      if (this.vista !== 'globo' || e.button) return;
      G.arr = { x: e.clientX, y: e.clientY, t: performance.now() }; G.vl = G.vp = 0; G.mexeu = performance.now();
      fx.setPointerCapture(e.pointerId); this.palco.classList.add('girando');
    });
    fx.addEventListener('pointermove', (e) => {
      if (!G.arr) return;
      const k = 0.32 / G.zoom, dx = e.clientX - G.arr.x, dy = e.clientY - G.arr.y, agora = performance.now(), dtt = Math.max(8, agora - G.arr.t);
      G.lam -= dx * k; G.phi += dy * k; G.vl = (-dx * k * 16) / dtt; G.vp = (dy * k * 16) / dtt;
      G.arr = { x: e.clientX, y: e.clientY, t: agora }; G.mexeu = agora;
      this.#pedirQuadro();
    });
    const soltar = () => { if (!G.arr) return; G.arr = null; this.palco.classList.remove('girando'); G.mexeu = performance.now(); G.base = G.lam; };
    fx.addEventListener('pointerup', soltar);
    fx.addEventListener('pointercancel', soltar);
    fx.addEventListener('wheel', (e) => {
      if (this.vista !== 'globo' || !(e.ctrlKey || this.el.classList.contains('max'))) return;
      e.preventDefault();
      G.zoom = Math.max(0.7, Math.min(3.2, G.zoom * Math.exp(-e.deltaY * 0.0015))); G.mexeu = performance.now();
      this.#pedirQuadro();
    }, { passive: false });
    fx.addEventListener('dblclick', (e) => { if (this.vista !== 'globo') return; e.stopPropagation(); G.base = -50; G.mexeu = -1e9; G.zoom = 1; this.#pedirQuadro(); });
  }

  /** Para os ensaios (Playwright). */
  resumo() {
    return { carregado: this.carregado, N: this.N, n: this.n, acesas: (this.brAcesas || 0) + (this.extAcesas || 0), exterior: this.extAcesas, nExt: this.nExt,
      semPosicao: this.semPosicao, noLocal: this.noLocal, vista: this.vista, cor: this.cor };
  }
}

