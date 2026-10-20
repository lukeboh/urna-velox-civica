// Previsão (etapa 8, PROTOCOLO_WS §4.7): cartão "Previsão (estimativa)" com a faixa de horário do "matematicamente", o
// hodômetro do horário mais provável, a faixa P10–P90 ao longo da noite (a previsão convergindo) e o fim da apuração.
// Sempre "estimativa", nunca "resultado". Caixa de altura fixa; todo texto em linhas de altura reservada (CLS 0).

import { Hodometro } from './hodometro.js';
import { el, svg, segundos } from './util.js';

// M-003: os horários da previsão vêm no relógio do sistema (`relogio: "sistema"`). No simulado acelerado (a 500×, 1 s
// do sistema = 8 min da noite) os segundos contam: o cartão mostra hh:mm:ss (`definirAceleracao`).
let comSegundos = false;
/** "aaaa-mm-dd hh:mm:ss" → "hh:mm" (ou "hh:mm:ss" no simulado acelerado) */
const hm = (t) => { const m = /(\d{2}):(\d{2}):(\d{2})/.exec(t || ''); return m ? (comSegundos ? `${m[1]}:${m[2]}:${m[3]}` : `${m[1]}:${m[2]}`) : '—'; };
const faixaTxt = (f) => (f ? (f.incerta ? 'em horário ainda incerto' : `entre ${hm(f.p10)} e ${hm(f.p90)}`) : '');

/**
 * Junta uma mensagem `previsao` à anterior do tópico: a da inscrição traz o histórico inteiro; as outras acrescentam a
 * faixa do momento (no máximo um ponto por minuto da noite, como no servidor: `x` = aceleração do simulado).
 */
export function juntarPrevisao(ant, m, x = 1) {
  const hist = m.instantaneo ? (m.historico || []) : (ant?.historico || []).slice();
  if (!m.instantaneo && m.mat) {
    const p = [segundos(m.instante), segundos(m.mat.p10), segundos(m.mat.p50), segundos(m.mat.p90)];
    const u = hist[hist.length - 1];
    const passo = Math.max(1, 60 / (x || 1));
    if (u && Math.floor(u[0] / passo) === Math.floor(p[0] / passo)) hist[hist.length - 1] = p; else hist.push(p);
    if (hist.length > 1500) hist.splice(0, hist.length - 1500);
  }
  return { ...m, historico: hist };
}

export class CartaoPrevisao {
  constructor() {
    this.rot = el('div', 'pv-rot', 'Matematicamente');
    this.hod = new Hodometro({ molde: '##:##' });
    this.faixa = el('div', 'pv-faixa', '');
    this.nota = el('div', 'pv-nota', '');
    this.g = svg('svg', { viewBox: '0 0 300 80', preserveAspectRatio: 'none', role: 'img', 'aria-label': 'Faixa prevista ao longo da noite' });
    this.banda = svg('path', { class: 'pv-banda', d: '' });
    this.med = svg('path', { class: 'pv-med', d: '' });
    this.real = svg('line', { class: 'pv-real', x1: 0, x2: 300, y1: 0, y2: 0 });
    this.real.style.opacity = '0';
    this.g.append(this.banda, this.med, this.real);
    this.fim = el('div', 'pv-fim', '');
    this.el = el('section', 'vidro previsao', [
      el('h2', 'cartao-titulo', ['Previsão ', el('small', null, '(estimativa de horário)')]),
      el('div', 'pv-topo', [el('div', 'pv-txt', [this.rot, this.faixa, this.nota]), el('div', 'pv-hod', [this.hod.el, el('small', null, 'mais provável')])]),
      el('div', 'pv-graf', this.g),
      this.fim,
    ]);
    this.el.setAttribute('aria-live', 'off');
    this.chave = '';
    this.x = 1;
  }

  /** M-003: aceleração do simulado (âncora do relógio do sistema). Acima de 1×, horários com segundos. */
  definirAceleracao(x) {
    x = x > 0 ? x : 1;
    if (x === this.x) return;
    this.x = x;
    comSegundos = x > 1;
    const molde = comSegundos ? '##:##:##' : '##:##';
    if (this.hod.molde !== molde) {
      const novo = new Hodometro({ molde });
      this.hod.el.replaceWith(novo.el);
      this.hod = novo;
      this.hod.molde = molde;
    }
    this.chave = '';
  }

  /** p = última previsão do tópico (ou null); circ = código da circunscrição do tópico; nomeCirc = nome para o texto. */
  definir(p, ehCirc, nomeCirc) {
    const chave = p ? `${p.instante}|${p.situacao}|${ehCirc}` : `nada|${ehCirc}`;
    if (chave === this.chave) return;
    this.chave = chave;
    let rot = 'Matematicamente', faixa = '', nota = '', hod = null, fim = '';
    if (!p) {
      rot = ehCirc ? 'Matematicamente' : 'Fim da apuração';
      faixa = 'aguardando base';
      nota = 'a previsão começa com 5% dos eleitores apurados e 10 min de apuração';
    } else {
      const conf = p.confirmacao;
      if (p.situacao === 'definida' || p.definida) {
        rot = 'Matematicamente: saiu';
        faixa = `às ${hm(p.definida || conf?.saiu)}`;
        if (conf) nota = conf.dentro ? `dentro da faixa prevista (${hm(conf.faixa.p10)}–${hm(conf.faixa.p90)})${conf.desde ? ` desde ${hm(conf.desde)}` : ''}`
          : `fora da faixa prevista (${hm(conf.faixa.p10)}–${hm(conf.faixa.p90)})`;
        else nota = 'saiu antes de haver base para a previsão';
        hod = hm(p.definida || conf?.saiu);
      } else if (p.mat) {
        const m = p.mat;
        // B-013: o que se prevê é quando o "matematicamente" sai (a frase continua na faixa: "sai entre 21:10 e 21:40")
        rot = m.tipo === 'matematicamente_2t' ? 'Matematicamente 2º turno sai' : 'Matematicamente eleito sai';
        faixa = faixaTxt(m);
        const a = Math.round(100 * (m.apurado_p50 || 0));
        nota = m.prob_antes_fim < 0.2 ? 'deve ir até o fim da apuração' : `chance de sair antes do fim: ${Math.round(100 * m.prob_antes_fim)}% · com ~${a}% apurado`;
        hod = m.incerta ? null : hm(m.p50);
      } else if (!ehCirc) {
        rot = 'O matematicamente';
        faixa = `se decide em ${nomeCirc}`;
        nota = 'abra a circunscrição para ver a previsão';
      }
      // chegada dos BUs parada no país: a previsão é a última normal, adiada pelo tempo parado
      if (p.parada && p.situacao === 'suspensa') {
        if (p.mat) faixa = `${faixaTxt(p.mat)} (adiada)`;
        nota = `chegada de BUs parada desde ${hm(p.parada.desde)} — previsão adiada`;
      }
      const f99 = p.chegou?.fim99 ? `99% às ${hm(p.chegou.fim99)}` : p.fim99 ? `99% ${faixaTxt(p.fim99)}` : '';
      const f100 = p.chegou?.fim100 ? `100% às ${hm(p.chegou.fim100)}` : p.fim100 ? `100% ${faixaTxt(p.fim100)}` : '';
      fim = [f99, f100].filter(Boolean).join(' · ');
      // B-013: a segunda previsão do cartão é a do fim da apuração (das seções), separada do "matematicamente"
      if (fim) fim = `Fim da apuração: ${fim}`;
    }
    if (this.rot.textContent !== rot) this.rot.textContent = rot;
    if (this.faixa.textContent !== faixa) this.faixa.textContent = faixa;
    if (this.nota.textContent !== nota) { this.nota.textContent = nota; this.nota.title = nota; }
    this.nota.classList.toggle('alerta', !!(p?.parada && p.situacao === 'suspensa'));
    if (this.fim.textContent !== fim) { this.fim.textContent = fim; this.fim.title = fim; }
    this.hod.definir(hod);
    this.desenhar(p);
  }

  /** A faixa P10–P90 (área) e o P50 (linha) de cada previsão da noite; a linha horizontal é o horário real. */
  desenhar(p) {
    const h = (p?.historico || []).filter((x) => x.every((v) => v != null));
    const real = p?.definida ? segundos(p.definida) : null;
    if (h.length < 2) { this.banda.setAttribute('d', ''); this.med.setAttribute('d', ''); this.real.style.opacity = '0'; return; }
    // M-003: no relógio do sistema; as folgas mínimas (1 e 5 min da noite) encolhem com a aceleração do simulado
    const k = 1 / (this.x || 1);
    const x0 = h[0][0], x1 = Math.max(h[h.length - 1][0], x0 + 60 * k);
    let lo = Math.min(...h.map((x) => x[1])), hi = Math.max(...h.map((x) => x[3]));
    if (real != null) { lo = Math.min(lo, real); hi = Math.max(hi, real); }
    const pad = Math.max(300 * k, (hi - lo) * 0.08);
    lo -= pad; hi += pad;
    const X = (t) => ((t - x0) / (x1 - x0)) * 300, Y = (t) => 80 - ((t - lo) / (hi - lo)) * 80;
    const cima = h.map((x) => `${X(x[0]).toFixed(1)},${Y(x[3]).toFixed(1)}`);
    const baixo = h.map((x) => `${X(x[0]).toFixed(1)},${Y(x[1]).toFixed(1)}`).reverse();
    this.banda.setAttribute('d', `M${cima.join('L')}L${baixo.join('L')}Z`);
    this.med.setAttribute('d', `M${h.map((x) => `${X(x[0]).toFixed(1)},${Y(x[2]).toFixed(1)}`).join('L')}`);
    // horário real (depois que saiu): linha tracejada; antes, escondida
    if (real != null) { const yr = Y(real).toFixed(1); this.real.setAttribute('y1', yr); this.real.setAttribute('y2', yr); }
    this.real.style.opacity = real != null ? '1' : '0';
  }
}
