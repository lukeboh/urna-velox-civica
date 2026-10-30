// Cliente do protocolo público (docs/arquitetura/PROTOCOLO_WS.md): só /ws e /api/v1/*. Reconexão com espera crescente (1, 2, 4…
// até 30 s, com variação aleatória, §5); a cada conexão reinscreve os tópicos, pede `eventos` e `marcos`.
//
// B-026 — modo HTTP: depois de FALHAS_PARA_HTTP falhas seguidas do WebSocket (recusa — 503 "limite de clientes"
// acima do `--max-clientes` — ou queda), a página passa a consultar por HTTP, a cada ~60 s (±20 %), só o resultado
// dos tópicos da tela (`GET /api/v1/resultado/<tópico>`, com If-None-Match) e o estado da apuração
// (`GET /api/v1/estado`); as respostas vão para o mesmo `aoReceber` do WebSocket. O WebSocket é tentado de novo a cada
// ~2 min; quando abre, a página volta ao tempo real. Eventos, marcos, "faltam", comparação com o TSE e previsão só vêm
// pelo WebSocket. Base das consultas: `<meta name="uv-dados" content="…">` no index.html (no passo 3 do plano de
// borda, o subdomínio `dados`, com cache na borda); vazia, ou fora do ar, o próprio domínio.

const MAX_POR_MSG = 90; // tópicos por mensagem (cada mensagem ≤ 4 KB)
const FALHAS_PARA_HTTP = 3; // falhas seguidas do WebSocket até o modo HTTP (~3 s: tentativas em 0, 1 e 2 s)
const VIDA_MINIMA_MS = 30000; // uma conexão que durou isso zera a contagem de falhas
const CONSULTA_MS = 60000; // modo HTTP: intervalo das consultas (±20 %)
const NOVO_WS_MS = 120000; // modo HTTP: intervalo das novas tentativas do WebSocket (±20 %)
const BASE_PAUSA_MS = 10 * 60000; // base das consultas fora do ar: próprio domínio por esse tempo

const variar = (ms) => ms * (0.8 + Math.random() * 0.4);

export class Conexao {
  /** @param {(m: object) => void} aoReceber  @param {(estado: string, info?: object) => void} aoEstado */
  constructor(aoReceber, aoEstado) {
    this.aoReceber = aoReceber;
    this.aoEstado = aoEstado;
    this.topicos = new Set();
    // tópicos de que a página quer o `faltam_para_mat` (B-014: não o das 5.574 circunscrições)
    this.faltam = [];
    this.ws = null;
    this.espera = 1000;
    this.timer = null;
    this.quedas = 0;
    this.fila = [];
    // modo HTTP (B-026)
    this.falhas = 0;
    this.http = false;
    this.tHttp = null;
    this.etag = new Map(); // tópico → ETag da última resposta (próprio domínio)
    this.ultSeq = new Map(); // tópico → seq do último resultado entregue por HTTP
    this.baseForaAte = 0;
    // mapa das seções (mapa.js): função que diz de onde o servidor deve continuar (`{desde, epoca}`); null = sem mapa
    this.mapa = null;
    this.aoConsultar = null;
    // ajustes (os testes encurtam os intervalos; a base vem do index.html)
    const o = globalThis.__uvConexao || {};
    this.consultaMs = o.consultaMs || CONSULTA_MS;
    this.novoWsMs = o.novoWsMs || NOVO_WS_MS;
    const meta = typeof document !== 'undefined' ? document.querySelector('meta[name="uv-dados"]')?.content : '';
    this.base = String(o.base ?? meta ?? '').trim().replace(/\/+$/, '');
    if (this.base && typeof location !== 'undefined' && this.base === location.origin) this.base = '';
    // `?modo=http` na URL: este navegador age como se o /ws fosse recusado (conferir o modo HTTP do celular, sem
    // mexer no servidor); as consultas vão ao servidor de verdade, nos intervalos de verdade
    try { this.forcarHttp = new URLSearchParams(location.search).get('modo') === 'http'; } catch { this.forcarHttp = false; }
  }

  url() {
    const p = location.protocol === 'https:' ? 'wss:' : 'ws:';
    return `${p}//${location.host}/ws`;
  }

  abrir() {
    clearTimeout(this.timer);
    // no modo HTTP a nova tentativa é silenciosa: a tela continua dizendo "a cada minuto"
    if (!this.http) this.aoEstado('conectando');
    // ?modo=http: a "recusa" sem abrir conexão (a espera crescente e a entrada no modo HTTP são as de sempre)
    if (this.forcarHttp) return this.#falhou();
    let ws;
    try { ws = new WebSocket(this.url()); } catch { return this.#falhou(); }
    this.ws = ws;
    let aberta = 0;
    ws.onopen = () => {
      aberta = Date.now();
      this.espera = 1000;
      if (this.http) this.#sairHttp();
      this.aoEstado('vivo');
      this.#mandarInscricoes([...this.topicos]);
      // B-014: eventos e marcos do Brasil e das UFs (e os dos tópicos inscritos), não os das 5.574 circunscrições municipais
      this.#mandar({ op: 'eventos', escopo: 'uf' });
      this.#mandar({ op: 'marcos', escopo: 'uf', faltam: this.faltam });
      // sondas (PROTOCOLO_WS §4.9): uma mensagem pequena a cada ~30 s, para o chip e a telemetria mostrarem que o
      // caminho dos BUs está andando (servidor antigo responde `op_desconhecida`, ignorado)
      this.#mandar({ op: 'sonda' });
      if (this.mapa) this.#mandar({ op: 'mapa', ...this.mapa() });
    };
    ws.onmessage = (e) => {
      const d = e.data;
      // resultado (a maioria das mensagens; a 60× são milhares por segundo): só o envelope é lido aqui; o JSON do
      // resultado é interpretado quando alguém precisa dele (o tópico da tela, uma vez por quadro)
      if (d.startsWith('{"tipo":"atualizacao"') || d.startsWith('{"tipo":"instantaneo"')) {
        const r = /"topico":"([^"]+)","seq":(\d+)/.exec(d.slice(0, 160));
        if (r) return this.aoReceber({ tipo: 'resultado', topico: r[1], seq: +r[2], txt: d });
      }
      let m;
      try { m = JSON.parse(e.data); } catch { return; }
      this.aoReceber(m);
    };
    ws.onclose = () => {
      if (this.ws !== ws) return;
      this.ws = null;
      this.quedas++;
      if (aberta && Date.now() - aberta >= VIDA_MINIMA_MS) this.falhas = 0;
      this.#falhou();
    };
    ws.onerror = () => { /* o onclose vem em seguida */ };
  }

  /** Uma recusa ou queda do WebSocket: reconecta com espera crescente ou, depois de algumas seguidas, modo HTTP. */
  #falhou() {
    this.falhas++;
    if (this.http) return this.#agendarWs();
    if (this.falhas >= FALHAS_PARA_HTTP) return this.#entrarHttp();
    this.#agendar();
  }

  #agendar() {
    const t = Math.min(30000, this.espera) * (0.75 + Math.random() * 0.5);
    this.espera = Math.min(30000, this.espera * 2);
    this.aoEstado('caiu', { em: Date.now() + t });
    this.timer = setTimeout(() => this.abrir(), t);
  }

  // ---------------------------------------------------------------- modo HTTP (B-026)
  #entrarHttp() {
    this.http = true;
    this.aoEstado('http', { consultaMs: this.consultaMs });
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
  #topicosHttp(lista) { return lista.filter((t) => !t.includes(':')); }

  async #consultar() {
    clearTimeout(this.tHttp);
    if (!this.http) return;
    await Promise.all([this.#buscar(null), ...this.#topicosHttp([...this.topicos]).map((t) => this.#buscar(t))]);
    if (!this.http) return;
    // o mapa das seções relê a fotografia (a borda guarda por 5 s)
    try { this.aoConsultar?.(); } catch { /* o mapa cuida de si */ }
    clearTimeout(this.tHttp);
    this.tHttp = setTimeout(() => this.#consultar(), variar(this.consultaMs));
  }

  /** Um GET: estado (`topico` null) ou resultado do tópico. Base das consultas primeiro; se falhar, o próprio domínio. */
  async #buscar(topico) {
    const caminho = topico ? `/api/v1/resultado/${encodeURIComponent(topico)}` : '/api/v1/estado';
    let r = null;
    if (this.base && Date.now() >= this.baseForaAte) {
      // outro domínio: sem cabeçalho próprio (If-None-Match pediria um preflight CORS, que a borda não atende); o
      // navegador revalida sozinho com o ETag guardado (`no-cache`) e devolve o corpo do cache num 304
      try {
        r = await fetch(this.base + caminho, { cache: 'no-cache', credentials: 'omit' });
        if (r.status >= 500) { this.baseForaAte = Date.now() + BASE_PAUSA_MS; r = null; } else if (!r.ok) r = null;
      } catch { this.baseForaAte = Date.now() + BASE_PAUSA_MS; r = null; }
    }
    if (!r) {
      const et = topico ? this.etag.get(topico) : null;
      try { r = await fetch(caminho, { cache: 'no-store', headers: et ? { 'If-None-Match': et } : {} }); } catch { return; }
      if (r.status === 304 || !r.ok) return;
      if (topico && r.headers.get('ETag')) this.etag.set(topico, r.headers.get('ETag'));
    }
    if (!this.http) return;
    let txt;
    try { txt = await r.text(); } catch { return; }
    if (!topico) {
      let m;
      try { m = JSON.parse(txt); } catch { return; }
      if (m && typeof m === 'object') this.aoReceber({ ...m, tipo: 'estado' });
      return;
    }
    // o corpo é o arquivo publicado (o `resultado` do WebSocket): o seq está no começo dele
    const s = /"seq":(\d+)/.exec(txt.slice(0, 400));
    if (!s || this.ultSeq.get(topico) === +s[1]) return;
    this.ultSeq.set(topico, +s[1]);
    this.aoReceber({ tipo: 'resultado', topico, seq: +s[1], txt: `{"tipo":"instantaneo","topico":${JSON.stringify(topico)},"seq":${s[1]},"resultado":${txt}}` });
  }

  /** GET de leitura (mapa das seções): base das consultas primeiro (cache da borda); se falhar, o próprio domínio. */
  async obter(caminho) {
    if (this.base && Date.now() >= this.baseForaAte) {
      try {
        const r = await fetch(this.base + caminho, { credentials: 'omit' });
        if (r.ok) return r;
        if (r.status >= 500) this.baseForaAte = Date.now() + BASE_PAUSA_MS;
      } catch { this.baseForaAte = Date.now() + BASE_PAUSA_MS; }
    }
    try { const r = await fetch(caminho); return r.ok ? r : null; } catch { return null; }
  }

  /** Liga o mapa das seções: `f()` dá `{desde, epoca}` (pedido agora e a cada reconexão). */
  pedirMapa(f) {
    this.mapa = f;
    this.#mandar({ op: 'mapa', ...f() });
  }

  // ---------------------------------------------------------------- inscrições
  #mandar(o) {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) this.ws.send(JSON.stringify(o));
  }

  #mandarInscricoes(lista) {
    for (let i = 0; i < lista.length; i += MAX_POR_MSG) this.#mandar({ op: 'inscrever', topicos: lista.slice(i, i + MAX_POR_MSG) });
  }

  /** Troca a lista de tópicos com `faltam_para_mat` (o servidor manda o valor atual dos que entraram). */
  definirFaltam(lista) {
    if (lista.length === this.faltam.length && lista.every((t, i) => t === this.faltam[i])) return;
    this.faltam = lista;
    this.#mandar({ op: 'marcos', escopo: 'uf', faltam: lista });
  }

  /** Inscreve os tópicos que faltam e cancela os que saíram (a lista inteira desejada). */
  definirTopicos(lista) {
    const novo = new Set(lista);
    const entrar = [...novo].filter((t) => !this.topicos.has(t));
    const sair = [...this.topicos].filter((t) => !novo.has(t));
    this.topicos = novo;
    if (entrar.length) this.#mandarInscricoes(entrar);
    for (let i = 0; i < sair.length; i += MAX_POR_MSG) this.#mandar({ op: 'cancelar', topicos: sair.slice(i, i + MAX_POR_MSG) });
    // modo HTTP: o tópico que entrou na tela é consultado na hora (os outros seguem no intervalo)
    if (this.http) for (const t of this.#topicosHttp(entrar)) this.#buscar(t);
  }
}
