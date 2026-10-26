// Barra de controle do replay, como a de um vídeo (só com o serviço em --controle-replay contra o site simulado; ver
// docs/PROTOCOLO_WS.md §11): tocar/pausar, linha do tempo da noite e velocidade. Ir a um instante já visto (para trás
// ou para a frente) = o serviço retotaliza do disco até lá em segundos; além do que já foi visto, avança baixando, mais
// rápido. Vale para todas as abas: cada uma acompanha GET /api/v1/replay e se recarrega quando a sessão do serviço muda.
//
// A barra fica presa acima do letreiro e a página reserva o espaço dela (classe `com-replay` posta antes do primeiro
// desenho): nada fica escondido embaixo e nada pula quando ela aparece.
import { el } from './util.js';

export const VELOCIDADES = [1, 2, 5, 10, 20, 30, 60, 120, 180, 240, 300];
// t: segundos na hora de Brasília (escala de Date.UTC sobre o horário civil de Brasília, como o servidor)
const hms = (t) => new Date(t * 1000).toISOString().slice(11, 19);
const hm = (t) => new Date(t * 1000).toISOString().slice(11, 16);
const vezes = (x) => `${String(x).replace('.', ',')}×`;

async function postar(caminho, corpo) {
  const r = await fetch(caminho, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(corpo) });
  const j = await r.json().catch(() => ({}));
  if (!r.ok) throw new Error(j.erro || `HTTP ${r.status}`);
  return j;
}

export class Player {
  /** @param {object} info primeira resposta de GET /api/v1/replay */
  constructor(info) {
    this.info = info;
    this.sessao = info.sessao;
    this.medidoEm = performance.now();
    this.arrastando = false;
    this.ultima = info.acelerar > 0 ? info.acelerar : 10;
    this.voltando = null;

    this.play = el('button', 'icone-btn pl-play', '❚❚');
    this.play.type = 'button';
    this.hora = el('output', 'pl-hora', '—');
    this.tempo = el('input', 'pl-tempo');
    Object.assign(this.tempo, { type: 'range', step: '60', min: String(info.inicio), max: String(info.fim) });
    this.tempo.setAttribute('aria-label', 'Momento da noite');
    this.dica = el('span', 'pl-dica', '');
    this.marcas = el('div', 'pl-marcas');
    this.alvoMarca = el('i', 'pl-alvo');
    const linha = el('div', 'pl-linha', [this.marcas, this.alvoMarca, this.tempo, this.dica]);
    this.ini = el('span', 'pl-ini', hm(info.inicio));
    this.fim = el('span', 'pl-fim', hm(info.fim));
    this.vel = el('select', 'pl-vel');
    this.vel.setAttribute('aria-label', 'Velocidade');
    for (const v of VELOCIDADES) { const o = el('option', null, vezes(v)); o.value = String(v); this.vel.append(o); }
    this.estado = el('span', 'pl-estado', '');
    this.estado.setAttribute('aria-live', 'polite');
    this.el = el('section', 'player vidro', [this.play, this.hora, this.ini, linha, this.fim, this.vel, this.estado]);
    this.el.setAttribute('aria-label', 'Controle do replay');
    this.veu = el('div', 'pl-veu', [el('div', 'pl-veu-txt', '')]);
    this.veu.hidden = true;
    // marcas de hora na linha do tempo
    const span = info.fim - info.inicio;
    for (let h = Math.ceil(info.inicio / 3600) * 3600; h < info.fim; h += 3600) {
      const m = el('span', null, hm(h).slice(0, 2));
      m.style.left = `${((h - info.inicio) / span) * 100}%`;
      this.marcas.append(m);
    }

    this.play.addEventListener('click', () => this.velocidade(this.info.acelerar === 0 ? this.ultima : 0));
    this.vel.addEventListener('change', () => this.velocidade(+this.vel.value));
    this.tempo.addEventListener('input', () => {
      this.arrastando = true;
      this.dica.textContent = `ir para ${hm(+this.tempo.value)}`;
      this.dica.style.left = `${this.pos(+this.tempo.value) * 100}%`;
      this.el.classList.add('com-dica');
    });
    this.tempo.addEventListener('change', () => this.ir(+this.tempo.value));
    document.body.append(this.veu, this.el);
    this.aplicar(info);
    setInterval(() => this.consultar(), 1000);
    setInterval(() => this.desenhar(), 250);
  }

  pos(t) { return Math.min(1, Math.max(0, (t - this.info.inicio) / Math.max(1, this.info.fim - this.info.inicio))); }

  /** Relógio da simulação agora (interpolado entre as consultas pela aceleração real). */
  agora() {
    const i = this.info;
    if (i.t == null) return i.inicio;
    const dt = (performance.now() - this.medidoEm) / 1000;
    return Math.min(i.busca ? i.busca.alvo : i.fim + 3600, i.t + dt * (i.real || 0));
  }

  aplicar(info) {
    this.info = info;
    this.medidoEm = performance.now();
    if (info.sessao && this.sessao && info.sessao !== this.sessao) { this.recarregar(); return; }
    if (info.acelerar > 0) this.ultima = info.acelerar;
    const pausado = info.acelerar === 0;
    this.play.textContent = pausado ? '▶' : '❚❚';
    this.play.setAttribute('aria-label', pausado ? 'Continuar o replay' : 'Pausar o replay');
    this.el.classList.toggle('pausado', pausado && !info.busca);
    if (document.activeElement !== this.vel) this.vel.value = String(VELOCIDADES.includes(info.acelerar) ? info.acelerar : this.ultima);
    this.el.classList.toggle('buscando', !!info.busca);
    this.alvoMarca.style.left = info.busca ? `${this.pos(info.busca.alvo) * 100}%` : '';
    this.desenhar();
  }

  desenhar() {
    const t = this.agora();
    const txt = hms(t);
    if (this.hora.textContent !== txt) this.hora.textContent = txt;
    if (!this.arrastando) {
      this.tempo.value = String(t);
      this.tempo.style.setProperty('--p', `${this.pos(t) * 100}%`);
    }
    const b = this.info.busca;
    let e = '';
    if (this.voltando) e = `indo para ${hm(this.voltando)}: retotalizando…`;
    else if (b) e = `avançando até ${hm(b.alvo)} · ${Math.round(100 * Math.min(1, Math.max(0, (t - b.de) / Math.max(1, b.alvo - b.de))))}%`;
    else if (this.info.acelerar === 0) e = 'pausado';
    if (this.estado.textContent !== e) this.estado.textContent = e;
  }

  async consultar() {
    try {
      const r = await fetch('/api/v1/replay', { cache: 'no-store' });
      if (r.ok) this.aplicar(await r.json());
    } catch { /* serviço reiniciando: tenta de novo no próximo segundo */ }
  }

  /** Bloco `replay` da mensagem `estado` (chega pelo WebSocket): só a sessão interessa aqui. */
  estadoWs(rp) { if (rp?.sessao && rp.sessao !== this.sessao) this.recarregar(); }

  recarregar() {
    if (this.recarregando) return;
    this.recarregando = true;
    location.reload();
  }

  async velocidade(x) {
    try { this.aplicar(await postar('/api/v1/replay/velocidade', { acelerar: x })); } catch (e) { this.estado.textContent = `não mudou: ${e.message}`; }
  }

  async ir(t) {
    this.arrastando = false;
    this.el.classList.remove('com-dica');
    try {
      const j = await postar('/api/v1/replay/posicao', { t });
      if (j.reinicio) {
        // o serviço recomeça e retotaliza do disco até o instante: véu até a sessão nova responder, e então recarrega
        this.voltando = t;
        this.veu.firstChild.textContent = `Indo para ${hm(t)}: retotalizando os BUs já baixados até lá…`;
        this.veu.hidden = false;
        this.desenhar();
      } else this.aplicar(j);
    } catch (e) {
      this.estado.textContent = `não foi: ${e.message}`;
    }
  }
}

/** Liga a barra se o serviço tiver o controle do replay (antes do primeiro desenho da página). */
export async function iniciarPlayer() {
  // logo depois de o serviço subir, o relógio do simulador ainda pode não ter sido lido: algumas tentativas
  for (let i = 0; i < 10; i++) {
    try {
      const r = await fetch('/api/v1/replay', { cache: 'no-store' });
      if (!r.ok) return null; // sem --controle-replay: 404
      const info = await r.json();
      if (info.inicio != null && info.fim != null) {
        document.documentElement.classList.add('com-replay');
        return new Player(info);
      }
    } catch { /* tenta de novo */ }
    await new Promise((ok) => setTimeout(ok, 300));
  }
  return null;
}
