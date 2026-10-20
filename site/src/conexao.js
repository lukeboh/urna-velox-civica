// Cliente do protocolo público (docs/PROTOCOLO_WS.md): só /ws e /api/v1/*. Reconexão com espera crescente (1, 2, 4…
// até 30 s, com variação aleatória, §5); a cada conexão reinscreve os tópicos, pede `eventos` e `marcos`.

const MAX_POR_MSG = 90; // tópicos por mensagem (cada mensagem ≤ 4 KB)

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
  }

  url() {
    const p = location.protocol === 'https:' ? 'wss:' : 'ws:';
    return `${p}//${location.host}/ws`;
  }

  abrir() {
    clearTimeout(this.timer);
    this.aoEstado('conectando');
    let ws;
    try { ws = new WebSocket(this.url()); } catch { return this.#agendar(); }
    this.ws = ws;
    ws.onopen = () => {
      this.espera = 1000;
      this.aoEstado('vivo');
      this.#mandarInscricoes([...this.topicos]);
      // B-014: eventos e marcos do Brasil e das UFs (e os dos tópicos inscritos), não os das 5.574 circunscrições municipais
      this.#mandar({ op: 'eventos', escopo: 'uf' });
      this.#mandar({ op: 'marcos', escopo: 'uf', faltam: this.faltam });
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
      this.#agendar();
    };
    ws.onerror = () => { /* o onclose vem em seguida */ };
  }

  #agendar() {
    const t = Math.min(30000, this.espera) * (0.75 + Math.random() * 0.5);
    this.espera = Math.min(30000, this.espera * 2);
    this.aoEstado('caiu', { em: Date.now() + t });
    this.timer = setTimeout(() => this.abrir(), t);
  }

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
  }
}
