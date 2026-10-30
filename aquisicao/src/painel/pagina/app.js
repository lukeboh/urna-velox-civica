'use strict';
// Painel dos nós do Urna Velox (docs/operacao/PAINEL.md). Sem bibliotecas: o DOM é montado com textContent (nada vindo da rede vira HTML).

const API = '/painel/api';
const INTERVALO_MS = 5000;
const MAX_S = 7200;

const est = {
  eu: null,
  nos: new Map(),     // id -> {resumo, alcancavel, erro, url, serie: {t: [], ...}}
  ultimo: 0,          // maior t já recebido (ms)
  janela: 900,
  aba: 'visao',
  noConfig: null,
  noDetalhe: null,
  cfgAtual: null,
};

// ------------------------------------------------------------------ utilitários

function h(tag, atrs, ...filhos) {
  const e = document.createElement(tag);
  for (const [k, v] of Object.entries(atrs || {})) {
    if (v == null || v === false) continue;
    if (k === 'class') e.className = v;
    else if (k.startsWith('on')) e.addEventListener(k.slice(2), v);
    else if (k === 'texto') e.textContent = v;
    else if (k === 'style') e.style.cssText = v; // CSSOM: permitido pela CSP (o atributo style no HTML, não)
    else e.setAttribute(k, v === true ? '' : v);
  }
  for (const f of filhos.flat()) if (f != null && f !== false) e.append(f instanceof Node ? f : document.createTextNode(String(f)));
  return e;
}
const SVG = 'http://www.w3.org/2000/svg';
function s(tag, atrs) {
  const e = document.createElementNS(SVG, tag);
  for (const [k, v] of Object.entries(atrs || {})) {
    // cor por variável CSS: pelo estilo (atributos de apresentação não resolvem var())
    if ((k === 'stroke' || k === 'fill') && String(v).startsWith('var(')) e.style[k] = v; else e.setAttribute(k, v);
  }
  return e;
}

const nf = (d) => new Intl.NumberFormat('pt-BR', { maximumFractionDigits: d, minimumFractionDigits: 0 });
function num(v, d = 1) { return v == null || Number.isNaN(v) ? '—' : nf(d).format(v); }
function inteiro(v) { return v == null ? '—' : nf(0).format(Math.round(v)); }
function duracao(seg) {
  if (seg == null) return '—';
  seg = Math.round(seg);
  const d = Math.floor(seg / 86400), hh = Math.floor(seg % 86400 / 3600), m = Math.floor(seg % 3600 / 60);
  return d ? `${d}d ${hh}h` : hh ? `${hh}h ${m}min` : m ? `${m}min` : `${seg}s`;
}
function hora(ms) { return new Date(ms).toLocaleTimeString('pt-BR', { hour: '2-digit', minute: '2-digit', second: '2-digit' }); }

function aviso(msg) {
  const a = document.getElementById('aviso');
  a.textContent = msg; a.hidden = false;
  clearTimeout(aviso.t); aviso.t = setTimeout(() => { a.hidden = true; }, 4000);
}

async function api(caminho, opcoes = {}) {
  const r = await fetch(API + caminho, { credentials: 'same-origin', cache: 'no-store', ...opcoes,
    headers: { 'content-type': 'application/json', 'x-uv-csrf': est.eu ? est.eu.csrf : '', ...(opcoes.headers || {}) } });
  if (r.status === 401) { location.reload(); throw new Error('sessão encerrada'); }
  let corpo = null;
  try { corpo = await r.json(); } catch (_) { /* sem corpo */ }
  return { status: r.status, corpo };
}

function ids() { return [...est.nos.keys()].sort(); }
function corDe(id) { const i = ids().indexOf(id); return `var(--serie-${(i % 3) + 1})`; }

// ------------------------------------------------------------------ dados

async function atualizar() {
  try {
    const desde = est.ultimo ? est.ultimo : 0;
    const { status, corpo } = await api(`/visao?desde=${desde}`);
    if (status !== 200) throw new Error(corpo && corpo.mensagem || `HTTP ${status}`);
    for (const n of corpo.nos) {
      const ant = est.nos.get(n.id) || { serie: { t: [] } };
      const novo = { ...ant, alcancavel: n.alcancavel, erro: n.erro, url: n.url, resumo: n.resumo || ant.resumo, intervalo_s: n.intervalo_s };
      if (n.serie) novo.serie = juntar(ant.serie, n.serie);
      est.nos.set(n.id, novo);
      if (n.serie && n.serie.t.length) est.ultimo = Math.max(est.ultimo, n.serie.t[n.serie.t.length - 1]);
    }
    const sel = document.getElementById('atualizado');
    sel.textContent = `ao vivo · ${hora(corpo.gerado_em)}`;
    sel.classList.remove('caiu');
    if (est.aba === 'visao') desenharVisao();
    montarSeletores();
  } catch (e) {
    const sel = document.getElementById('atualizado');
    sel.textContent = `sem resposta (${e.message})`;
    sel.classList.add('caiu');
  }
}

/// Junta as colunas novas ao fim (os nós amostram cada um no seu relógio; `desde` é o maior t visto em qualquer nó,
/// então um nó um pouco atrasado pode mandar pontos repetidos: só entram os posteriores ao último dele).
function juntar(a, b) {
  const ult = a.t.length ? a.t[a.t.length - 1] : 0;
  const r = {};
  for (const k of Object.keys(b)) r[k] = (a[k] || []).slice();
  b.t.forEach((t, i) => { if (t > ult) for (const k of Object.keys(b)) r[k].push(b[k][i]); });
  const corte = Date.now() - MAX_S * 1000;
  let i0 = 0; while (i0 < r.t.length && r.t[i0] < corte) i0++;
  if (i0) for (const k of Object.keys(r)) r[k] = r[k].slice(i0);
  return r;
}

function ultimo(n, col) {
  const v = n.resumo && n.resumo.ultima && n.resumo.ultima.valores;
  return v ? v[col] : null;
}

// ------------------------------------------------------------------ visão geral

function situacao(n) {
  if (!n.alcancavel) return ['critico', 'painel fora do alcance'];
  const sv = n.resumo && n.resumo.servico;
  if (!sv || sv.versao == null) return ['critico', 'sem saude.json (serviço parado?)'];
  const idade = ultimo(n, 'idade_saude_s');
  if (idade != null && idade > 120) return ['critico', `saúde parada há ${duracao(idade)}`];
  if (idade != null && idade > 30) return ['atencao', `saúde atrasada ${duracao(idade)}`];
  if (sv.pausa_429_s > 0) return ['critico', `pausa por HTTP 429 até ${sv.retomada_em || '?'}`];
  // sonda (Test Message) que não volta: o processo está vivo, mas o caminho dos BUs parou
  const sd = n.resumo.sonda;
  if (sd && sd.atrasada) return ['critico', `caminho dos BUs parado: sonda pendente há ${duracao(sd.atraso_s)}`];
  const e = String(sv.estado || 'ativo');
  if (/parad|erro|falh|abort/i.test(e)) return ['atencao', e];
  return ['bom', e];
}

function tile(rotulo, valor, sub, nivel) {
  return h('div', { class: `tile ${nivel || ''}` }, h('div', { class: 'rotulo', title: rotulo }, rotulo), h('div', { class: 'valor' }, valor),
    sub ? h('div', { class: 'sub', title: sub }, sub) : null);
}

function cartaoNo(id, n) {
  const [nivel, texto] = situacao(n);
  const topo = h('div', { class: 'no-topo' },
    h('h2', {}, h('span', { class: 'marcador', style: `background:${corDe(id)}` }), id),
    h('span', { class: `situacao ${nivel}` }, texto));
  if (!n.alcancavel) {
    return h('article', { class: 'no' }, topo, h('p', { class: 'inalcancavel' }, n.erro || 'sem resposta'),
      h('p', { class: 'mudo' }, `Painel do nó em ${n.url || '?'} não respondeu. O serviço do nó pode estar no ar: o painel dele é que não respondeu.`));
  }
  const r = n.resumo || {}, sv = r.servico || {}, sis = r.sistema || {}, fed = r.federacao, sec = sv.secoes || {};
  const v = (c) => ultimo(n, c);
  const procs = sis.processos || {};
  const mem = sis.memoria_mb || {};
  const pct = sec.catalogo ? 100 * (sec.aplicadas || 0) / sec.catalogo : null;
  const info = h('div', { class: 'linha-info' },
    h('span', {}, 'versão ', h('b', {}, sv.versao || '—')),
    h('span', {}, 'pleito ', h('b', {}, sv.pleito ?? '—')),
    h('span', {}, 'servidor ', h('b', {}, sv.base || '—')),
    h('span', {}, 'no ar há ', h('b', {}, duracao(sv.ativo_s))),
    h('span', {}, 'relógio do site ', h('b', {}, sv.relogio_site || '—')),
    sv.simulacao ? h('span', {}, 'replay ', h('b', {}, sv.simulacao.rotulo || 'sim')) : null,
    h('span', {}, 'máquina ', h('b', {}, sis.hostname || '—')));
  const teto = v('teto_req_s');
  const reqNivel = teto && v('req_s') > teto * 1.02 ? 'critico' : '';
  const t429 = v('http429');
  const coleta = h('div', { class: 'tiles' },
    tile('BUs aplicados/s', num(v('bus_s'), 1), `${inteiro(sec.aplicadas)} seções`),
    tile('Seções apuradas', pct == null ? '—' : `${num(pct, 2)}%`, `de ${inteiro(sec.catalogo)}`),
    tile('BUs baixados/s', num(v('baixados_s'), 1), `fila ${inteiro(v('fila'))} · pend. ${inteiro(v('pendentes'))}`),
    tile('Requisições/s', num(v('req_s'), 1), teto >= 100000 ? 'sem limite (modo espelho)'
      : `teto ${num(teto, 0)}${String((sv.rede || {}).acesso || '').startsWith('interno') ? ' (interno)' : ''}`, reqNivel),
    tile('HTTP 429', inteiro(t429), sv.pausa_429_s > 0 ? `pausa ${duracao(sv.pausa_429_s)}` : 'sem pausa', t429 > 0 ? 'critico' : ''),
    tile('Último BU', (sv.ultimo_bu && sv.ultimo_bu.site || '—').split(' ').pop(), 'hora do site'));
  const memNivel = mem.total && mem.usada / mem.total > 0.9 ? 'critico' : mem.total && mem.usada / mem.total > 0.8 ? 'atencao' : '';
  const disco = v('disco_livre_gb');
  const recursos = h('div', { class: 'tiles' },
    tile('CPU do serviço', `${num(v('cpu_pct'), 0)}%`, `${sis.cpus || '?'} CPUs na máquina`),
    tile('Memória do serviço', `${num(v('rss_mb'), 0)} MB`, procs['uv-servico'] ? `pico ${inteiro(procs['uv-servico'].pico_rss_mb)} MB` : ''),
    tile('CPU da máquina', `${num(v('sis_cpu_pct'), 0)}%`, `carga ${num(v('carga1'), 2)}`),
    tile('Memória da máquina', mem.total ? `${num(mem.usada / 1024, 1)} GB` : '—', mem.total ? `de ${num(mem.total / 1024, 1)} GB` : '', memNivel),
    tile('Disco livre', disco == null ? '—' : `${num(disco, 1)} GB`, '/srv', disco != null && disco < 5 ? 'critico' : disco != null && disco < 15 ? 'atencao' : ''),
    tile('Clientes WebSocket', inteiro(v('ws_clientes')), `${num(v('ws_msgs_s'), 1)} msg/s · ${num(v('ws_kb_s'), 1)} KB/s`));
  const partes = [topo, info, h('div', { class: 'subtitulo' }, 'Coleta e totalização'), coleta, h('div', { class: 'subtitulo' }, 'Recursos'), recursos];
  if (fed) {
    const ativos = (fed.ativos || []).map((a) => a.no).join(', ') || '—';
    const fora = (fed.fora || []).map((f) => `${f.no} (${f.motivo})`).join(', ');
    partes.push(h('div', { class: 'subtitulo' }, 'Federação'), h('div', { class: 'tiles' },
      tile('Recebidos dos pares/s', num(v('fed_recebidos_s'), 1), `${inteiro(fed.recebidos)} no total`),
      tile('Publicados aos pares/s', num(v('fed_publicados_s'), 1), `${inteiro(fed.publicados)} no total`),
      tile('Lotes copiados/s', num(v('fed_lotes_s'), 2), `erros de busca ${inteiro(fed.sincronia && fed.sincronia.buscas_erro)}`),
      tile('Nós ativos', inteiro((fed.ativos || []).length), ativos),
      tile('Fora da partição', inteiro((fed.fora || []).length), fora || 'nenhum', (fed.fora || []).length ? 'atencao' : ''),
      tile('Cota deste nó', fed.rps_cota == null ? '—' : `${num(fed.rps_cota, 0)} req/s`, `federação ${num(fed.rps_federacao, 0)} req/s`,
        fed.participa ? '' : 'atencao')));
  }
  const sd = r.sonda, vf = r.sonda_verificador;
  if (sd || vf) {
    const alvos = vf && vf.alvos ? Object.entries(vf.alvos) : [];
    const ruins = alvos.filter(([, a]) => a.situacao !== 'ok');
    const vfVelho = vf && vf.idade_arquivo_s > 30;
    partes.push(h('div', { class: 'subtitulo' }, 'Sondas (caminho dos BUs)'), h('div', { class: 'tiles' },
      sd ? tile('Sonda injeção → gravação', sd.ultima ? `${num(sd.ultima.ms && sd.ultima.ms.gravacao, 1)} ms` : '—',
        `pendente mais antiga ${duracao(sd.atraso_s)} · ${inteiro(sd.concluidas)}/${inteiro(sd.injetadas)} voltaram`, sd.atrasada ? 'critico' : '') : null,
      sd ? tile('Episódios de atraso', inteiro(sd.episodios_atraso), `limite ${duracao(sd.limite_s)} · perdidas ${inteiro(sd.perdidas)}`, sd.episodios_atraso ? 'atencao' : '') : null,
      vf ? tile('Verificador externo', vfVelho ? 'parado' : vf.ok ? 'ok' : `${ruins.length} com problema`,
        vfVelho ? `relatório de ${duracao(vf.idade_arquivo_s)} atrás` : ruins.map(([u, a]) => `${u}: ${a.situacao}`).join(' · ') || `${alvos.length} alvo(s)`,
        vfVelho ? 'atencao' : vf.ok ? '' : 'critico') : null));
  }
  const al = r.alertas;
  if (Array.isArray(al) && al.length) {
    partes.push(h('div', { class: 'subtitulo' }, 'Alertas recentes'), h('ul', { class: 'alertas' },
      al.map((a) => h('li', {}, h('b', {}, a.tipo || a.codigo || 'alerta'), ' ', a.quando || a.t || '', ' — ', a.mensagem || a.texto || JSON.stringify(a)))));
  }
  return h('article', { class: 'no' }, partes);
}

const GRAFICOS = [
  { col: 'bus_s', titulo: 'BUs aplicados por segundo', unid: 'seções totalizadas/s' },
  { col: 'baixados_s', titulo: 'BUs baixados por segundo', unid: 'BUs/s vindos do servidor de resultados' },
  { col: 'req_s', titulo: 'Requisições por segundo', unid: 'req/s (tracejado: teto do nó)', ref: 'teto_req_s' },
  { col: 'fed_recebidos_s', titulo: 'BUs recebidos dos outros nós', unid: 'BUs/s pela federação' },
  { col: 'fed_publicados_s', titulo: 'BUs publicados aos outros nós', unid: 'BUs/s pela federação' },
  { col: 'rss_mb', titulo: 'Memória do uv-servico', unid: 'MB (RSS)' },
  { col: 'sis_cpu_pct', titulo: 'CPU da máquina', unid: '%', max: 100 },
  { col: 'ws_clientes', titulo: 'Clientes WebSocket', unid: 'conexões abertas' },
  { col: 'sonda_ms', titulo: 'Sonda: injeção → gravação', unid: 'ms pelo caminho dos BUs (motor e gravação)' },
];

function desenharVisao() {
  const nos = document.getElementById('nos');
  nos.replaceChildren(...ids().map((id) => cartaoNo(id, est.nos.get(id))));
  const leg = document.getElementById('legenda');
  leg.replaceChildren(...(ids().length > 1 ? ids().map((id) => h('span', {}, h('span', { class: 'marcador', style: `background:${corDe(id)}` }), id)) : []));
  const g = document.getElementById('graficos');
  if (!g.children.length) for (const d of GRAFICOS) g.append(h('div', { class: 'grafico', 'data-col': d.col }, h('h3', {}, d.titulo), h('div', { class: 'unid' }, d.unid), s('svg', {})));
  for (const el of g.children) grafico(el, GRAFICOS.find((d) => d.col === el.dataset.col));
}

/// Gráfico de linhas: um traço por nó, eixo y único a partir de zero, cruz com dica no passar do dedo/mouse.
function grafico(el, def) {
  const svg = el.querySelector('svg');
  const W = Math.max(260, el.clientWidth - 24), H = 170, m = { e: 40, d: 8, c: 8, b: 20 };
  svg.setAttribute('viewBox', `0 0 ${W} ${H}`);
  const fim = Date.now(), ini = fim - est.janela * 1000;
  const series = ids().map((id) => {
    const sr = est.nos.get(id).serie || { t: [] };
    const pts = [];
    (sr.t || []).forEach((t, i) => { const y = sr[def.col] && sr[def.col][i]; if (t >= ini) pts.push([t, y]); });
    return { id, cor: corDe(id), pts };
  });
  let ref = null;
  if (def.ref) for (const id of ids()) { const r = ultimo(est.nos.get(id), def.ref); if (r != null) ref = Math.max(ref || 0, r); }
  if (ref != null && ref >= 100000) ref = null; // modo espelho sem limite: sem linha de teto
  const vals = series.flatMap((x) => x.pts.map((p) => p[1]).filter((y) => y != null));
  let ymax = Math.max(def.max || 0, ref || 0, ...vals, 1);
  if (!def.max) ymax = passoBonito(ymax * 1.1);
  const X = (t) => m.e + (t - ini) / (fim - ini) * (W - m.e - m.d);
  const Y = (y) => H - m.b - (y / ymax) * (H - m.b - m.c);
  const filhos = [];
  for (let i = 0; i <= 3; i++) {
    const y = ymax * i / 3;
    filhos.push(s('line', { class: 'grade', x1: m.e, x2: W - m.d, y1: Y(y), y2: Y(y) }));
    const t = s('text', { class: 'eixo-txt', x: m.e - 6, y: Y(y) + 3.5, 'text-anchor': 'end' }); t.textContent = curto(y); filhos.push(t);
  }
  const passos = est.janela <= 900 ? 5 : est.janela <= 3600 ? 15 : 30;
  const p0 = Math.ceil(ini / (passos * 60000)) * passos * 60000;
  for (let t = p0; t <= fim; t += passos * 60000) {
    const x = X(t);
    const tx = s('text', { class: 'eixo-txt', x, y: H - 5, 'text-anchor': 'middle' });
    tx.textContent = new Date(t).toLocaleTimeString('pt-BR', { hour: '2-digit', minute: '2-digit' }); filhos.push(tx);
  }
  if (ref != null) {
    filhos.push(s('line', { class: 'ref', x1: m.e, x2: W - m.d, y1: Y(ref), y2: Y(ref) }));
    const tx = s('text', { class: 'ref-txt', x: W - m.d - 2, y: Y(ref) - 4, 'text-anchor': 'end' }); tx.textContent = `teto ${curto(ref)}`; filhos.push(tx);
  }
  let algum = false;
  for (const sr of series) {
    let d = '', caneta = false, tAnt = 0;
    const lacuna = 3 * 1000 * ((est.nos.get(sr.id).intervalo_s) || 5);
    for (const [t, y] of sr.pts) {
      if (y == null || (caneta && t - tAnt > lacuna)) { caneta = false; if (y == null) continue; }
      d += `${caneta ? 'L' : 'M'}${X(t).toFixed(1)},${Y(y).toFixed(1)}`; caneta = true; tAnt = t; algum = true;
    }
    if (d) filhos.push(s('path', { class: 'linha', d, stroke: sr.cor }));
  }
  if (!algum) { const tx = s('text', { class: 'vazio', x: W / 2, y: H / 2, 'text-anchor': 'middle' }); tx.textContent = 'sem dados nesta janela'; filhos.push(tx); }
  const cruz = s('line', { class: 'cruz', y1: m.c, y2: H - m.b, visibility: 'hidden' });
  const pontos = series.map((sr) => s('circle', { class: 'ponto', r: 4.5, fill: sr.cor, visibility: 'hidden' }));
  const alvo = s('rect', { x: m.e, y: 0, width: W - m.e - m.d, height: H, fill: 'transparent' });
  filhos.push(cruz, ...pontos, alvo);
  svg.replaceChildren(...filhos);
  const dica = document.getElementById('dica');
  const mover = (ev) => {
    const box = svg.getBoundingClientRect();
    const x = (ev.clientX - box.left) * W / box.width;
    const t = ini + (x - m.e) / (W - m.e - m.d) * (fim - ini);
    const linhas = [];
    let tRef = null;
    series.forEach((sr, i) => {
      let melhor = null;
      for (const p of sr.pts) if (p[1] != null && (!melhor || Math.abs(p[0] - t) < Math.abs(melhor[0] - t))) melhor = p;
      const perto = melhor && Math.abs(melhor[0] - t) < 30000 + est.janela * 15;
      pontos[i].setAttribute('visibility', perto ? 'visible' : 'hidden');
      if (perto) { pontos[i].setAttribute('cx', X(melhor[0])); pontos[i].setAttribute('cy', Y(melhor[1])); tRef = tRef ?? melhor[0]; linhas.push([sr, melhor[1]]); }
    });
    if (tRef == null) { esconder(); return; }
    cruz.setAttribute('x1', X(tRef)); cruz.setAttribute('x2', X(tRef)); cruz.setAttribute('visibility', 'visible');
    dica.replaceChildren(h('div', { class: 'h' }, hora(tRef)), ...linhas.map(([sr, y]) =>
      h('div', {}, h('span', { class: 'marcador', style: `background:${sr.cor}` }), `${sr.id}: `, h('b', {}, num(y, 1)))));
    dica.hidden = false;
    const dx = ev.clientX + 14 + dica.offsetWidth > innerWidth ? -dica.offsetWidth - 14 : 14;
    dica.style.left = `${ev.clientX + dx}px`; dica.style.top = `${ev.clientY + 12}px`;
  };
  const esconder = () => { cruz.setAttribute('visibility', 'hidden'); pontos.forEach((p) => p.setAttribute('visibility', 'hidden')); dica.hidden = true; };
  alvo.addEventListener('pointermove', mover);
  alvo.addEventListener('pointerdown', mover);
  alvo.addEventListener('pointerleave', esconder);
}

function passoBonito(v) {
  const p = 10 ** Math.floor(Math.log10(v));
  for (const f of [1, 1.5, 2, 2.5, 3, 4, 5, 6, 8, 10]) if (f * p >= v) return f * p;
  return 10 * p;
}
function curto(v) { return v >= 1e6 ? `${num(v / 1e6, 1)} mi` : v >= 1e4 ? `${num(v / 1e3, 0)} mil` : num(v, v < 10 ? 1 : 0); }

// ------------------------------------------------------------------ seletores de nó

function montarSeletores() {
  for (const [sel, chave, carregar] of [['sel-config', 'noConfig', carregarConfig], ['sel-detalhe', 'noDetalhe', carregarDetalhe]]) {
    const el = document.getElementById(sel);
    const lista = ids();
    if (!est[chave] || !lista.includes(est[chave])) est[chave] = est.eu && lista.includes(est.eu.no) ? est.eu.no : lista[0];
    const atuais = [...el.children].map((b) => b.dataset.no).join();
    if (atuais !== lista.join()) {
      el.replaceChildren(...lista.map((id) => h('button', { 'data-no': id, onclick: () => { est[chave] = id; montarSeletores(); carregar(); } },
        h('span', { class: 'marcador', style: `background:${corDe(id)}` }), id)));
    }
    for (const b of el.children) b.setAttribute('aria-pressed', String(b.dataset.no === est[chave]));
  }
}

// ------------------------------------------------------------------ configuração

async function carregarConfig() {
  const alvo = document.getElementById('config');
  const id = est.noConfig;
  if (!id) return;
  alvo.replaceChildren(h('p', { class: 'mudo' }, `carregando a configuração de ${id}…`));
  const [{ status, corpo }, rt] = await Promise.all([api(`/no/${encodeURIComponent(id)}/config`), api(`/no/${encodeURIComponent(id)}/rotas`)]);
  if (id !== est.noConfig) return;
  if (status !== 200) { alvo.replaceChildren(h('p', { class: 'inalcancavel' }, `não foi possível ler: ${corpo && corpo.mensagem || status}`)); return; }
  est.cfgAtual = corpo;
  const tela = montarConfig(id, corpo);
  if (rt.status === 200) tela.firstChild.prepend(montarEnderecos(id, rt.corpo));
  const aviso = avisoCiclo(corpo);
  alvo.replaceChildren(...(aviso ? [aviso, tela] : [tela]));
}

/// Durante o ciclo de simulados (uv-ciclo-no) o nó roda um uv-servico próprio, com a configuração do ciclo; o no.env
/// abaixo é o do urna-velox.service, que fica parado: as mudanças valem quando o nó voltar ao serviço normal (B-021).
function avisoCiclo(c) {
  const estado = (u) => ((c.servicos || []).find((x) => x.unidade === u) || {}).estado;
  if (estado('uv-ciclo-no.service') !== 'active' || estado('urna-velox.service') === 'active') return null;
  return h('p', { class: 'aviso-ciclo' }, h('b', {}, 'Este nó está no ciclo de replays'), ' (uv-ciclo-no). O que a visão geral mostra ',
    '(pleito, servidor, versão) é do replay em andamento. Esta configuração é a do urna-velox.service, parado durante o ciclo: ',
    'o que for gravado aqui vale quando o nó voltar ao serviço normal.');
}

function montarConfig(id, c) {
  const res = h('div', { class: 'resultado' });
  const conf = h('div', { class: 'confirmar', hidden: true });
  const inputs = new Map();
  const campos = h('div', { class: 'campos' });
  for (const f of c.no_env.campos) {
    const atual = f.valor ?? '';
    let inp;
    if (f.tipo && f.tipo.opcoes) {
      inp = h('select', { id: `c-${f.chave}`, disabled: !f.editavel }, f.tipo.opcoes.map((o) => h('option', { value: o, selected: o === atual }, o)));
      if (!f.tipo.opcoes.includes(atual) && atual) inp.prepend(h('option', { value: atual, selected: true }, atual));
    } else {
      inp = h('input', { id: `c-${f.chave}`, value: atual, disabled: !f.editavel, inputmode: f.tipo === 'numero' ? 'decimal' : null,
        placeholder: f.opcional ? '(desligado)' : '', spellcheck: 'false', autocomplete: 'off' });
    }
    inp.dataset.original = atual;
    inp.addEventListener('input', () => { inp.classList.toggle('mudou', inp.value !== inp.dataset.original); atualizarBotao(); });
    if (f.editavel) inputs.set(f.chave, inp);
    campos.append(h('label', { for: `c-${f.chave}` }, f.rotulo, f.sensivel ? h('span', { class: 'sensivel' }, 'sensível') : null, h('small', {}, f.chave)), inp);
    if (f.ajuda) campos.append(h('div', { class: 'ajuda' }, f.ajuda));
  }
  const reiniciar = h('input', { type: 'checkbox' });
  const aplicar = h('button', { class: 'botao primario', disabled: true }, 'Aplicar mudanças');
  function mudancas() { const o = {}; for (const [k, i] of inputs) if (i.value !== i.dataset.original) o[k] = i.value.trim(); return o; }
  function atualizarBotao() { aplicar.disabled = !Object.keys(mudancas()).length && !reiniciar.checked; }
  reiniciar.addEventListener('change', atualizarBotao);
  aplicar.addEventListener('click', () => enviar(`/no/${encodeURIComponent(id)}/config`, { alteracoes: mudancas(), reiniciar: reiniciar.checked }, res, conf, aplicar));

  const outros = Object.entries(c.no_env.outros || {});
  const caixaEnv = h('section', { class: 'caixa' },
    h('h3', {}, 'Configuração do nó ', h('code', {}, c.arquivo)),
    c.modo_local ? h('p', { class: 'mudo' }, 'Modo local: as mudanças são gravadas direto no arquivo e nenhum serviço é reiniciado.') : null,
    c.existe ? null : h('p', { class: 'inalcancavel' }, 'O arquivo não existe neste nó.'),
    campos,
    outros.length ? h('details', {}, h('summary', { class: 'mudo' }, `outras ${outros.length} chaves (só leitura)`),
      h('table', { class: 'tabela' }, outros.map(([k, v]) => h('tr', {}, h('th', {}, k), h('td', {}, h('code', {}, v)))))) : null,
    h('div', { class: 'acoes' }, aplicar, h('label', {}, reiniciar, 'reiniciar o urna-velox depois de gravar')),
    conf, res);

  const resS = h('div', { class: 'resultado' });
  const confS = h('div', { class: 'confirmar', hidden: true });
  const botaoServ = (acao, rotulo, cls) => h('button', { class: `botao ${cls || ''}`, onclick: (ev) =>
    enviar(`/no/${encodeURIComponent(id)}/servico`, { acao }, resS, confS, ev.currentTarget) }, rotulo);
  const caixaServ = h('section', { class: 'caixa' }, h('h3', {}, 'Serviços'),
    h('table', { class: 'tabela' }, h('tr', {}, h('th', {}, 'unidade'), h('th', {}, 'estado')),
      (c.servicos || []).map((u) => h('tr', {}, h('td', {}, h('code', {}, u.unidade)), h('td', {}, h('span', { class: `estado-unidade ${u.estado}` }, u.estado))))),
    h('p', {}, 'Atualizador automático: ', h('b', {}, c.controle_manual ? 'travado (controle manual)' : 'livre'),
      h('span', { class: 'mudo' }, ' — travado, o uv-atualizador não mexe neste nó.')),
    h('div', { class: 'acoes' }, botaoServ('iniciar', 'Iniciar'), botaoServ('reiniciar', 'Reiniciar', 'perigo'), botaoServ('parar', 'Parar', 'perigo'),
      c.controle_manual ? botaoServ('destravar', 'Destravar o atualizador') : botaoServ('travar', 'Travar o atualizador')),
    confS, resS);

  const fed = c.federacao;
  const caixaFed = h('section', { class: 'caixa' }, h('h3', {}, 'Federação ', h('span', { class: 'mudo' }, '(federacao.json, só leitura)')),
    fed ? [h('table', { class: 'tabela' }, h('tr', {}, ['nó', 'órgão', 'IP de saída', 'teto', 'coleta'].map((t) => h('th', {}, t))),
      (fed.nos || []).map((n) => h('tr', {}, h('td', {}, n.id), h('td', {}, n.orgao), h('td', {}, h('code', {}, n.ip_saida)), h('td', {}, `${n.teto_rps ?? 100} req/s`),
        h('td', {}, n.coleta === false ? 'só recebe' : 'sim')))),
    h('p', { class: 'mudo' }, `teto da coleta ${fed.teto_coleta_rps ?? 95} req/s · batimento ${fed.batimento_s ?? 10} s · morte ${fed.morte_s ?? 30} s · pausa 429 ${fed.pausa_429_s ?? 600} s`)]
      : h('p', { class: 'mudo' }, 'sem federacao.json neste nó'));

  const caixaAud = h('section', { class: 'caixa' }, h('h3', {}, 'Auditoria deste nó'),
    (c.auditoria || []).length ? h('table', { class: 'tabela' }, (c.auditoria || []).map((a) => h('tr', {},
      h('td', {}, a.quando || ''), h('td', {}, a.operador || ''), h('td', {}, h('b', {}, a.acao || '')),
      h('td', {}, h('code', {}, JSON.stringify(a.detalhe || {})))))) : h('p', { class: 'mudo' }, 'nada registrado ainda'));

  return h('div', { class: 'painel-cfg' }, h('div', {}, caixaEnv, caixaAud), h('div', {}, caixaServ, caixaFed));
}

/// POST com a confirmação por frase: um 409 traz a frase pedida; o operador digita e manda de novo.
async function enviar(caminho, corpo, res, conf, botao, confirmacao) {
  res.textContent = ''; res.className = 'resultado';
  botao.disabled = true;
  try {
    const { status, corpo: r } = await api(caminho, { method: 'POST', body: JSON.stringify(confirmacao ? { ...corpo, confirmacao } : corpo) });
    if (status === 409 && r && r.confirmacao) {
      const inp = h('input', { placeholder: r.confirmacao, autocomplete: 'off', spellcheck: 'false' });
      const ok = h('button', { class: 'botao perigo' }, 'Confirmar');
      const cancelar = h('button', { class: 'botao discreto' }, 'Cancelar');
      ok.addEventListener('click', () => { conf.hidden = true; enviar(caminho, corpo, res, conf, botao, inp.value.trim()); });
      cancelar.addEventListener('click', () => { conf.hidden = true; botao.disabled = false; });
      conf.replaceChildren(h('p', {}, r.mensagem, '. Digite ', h('b', {}, r.confirmacao), ' para seguir.'),
        h('div', { class: 'acoes' }, inp, ok, cancelar));
      conf.hidden = false; inp.focus();
      return;
    }
    if (status === 200) {
      res.className = 'resultado ok';
      res.textContent = [r.mudou && r.mudou.length ? `gravado: ${r.mudou.join(', ')}` : (r.mensagem || 'feito'),
        r.reiniciado ? 'serviço reiniciado' : '', r.mensagem && r.mudou && r.mudou.length ? r.mensagem : '', r.saida || ''].filter(Boolean).join('\n');
      aviso('feito');
      setTimeout(carregarConfig, 1200);
    } else {
      res.className = 'resultado erro';
      res.textContent = (r && (r.mensagem || r.erro)) || `HTTP ${status}`;
    }
  } catch (e) {
    res.className = 'resultado erro'; res.textContent = e.message;
  } finally {
    if (conf.hidden) botao.disabled = false;
  }
}

// ------------------------------------------------------------------ detalhe

async function carregarDetalhe() {
  const pre = document.getElementById('detalhe');
  const id = est.noDetalhe;
  if (!id) return;
  pre.textContent = 'carregando…';
  const { status, corpo } = await api(`/no/${encodeURIComponent(id)}/saude`);
  if (id !== est.noDetalhe) return;
  pre.textContent = status === 200 ? (corpo == null ? 'sem saude.json neste nó' : JSON.stringify(corpo, null, 2)) : `erro: ${corpo && corpo.mensagem || status}`;
}

// ------------------------------------------------------------------ início

function trocarAba(aba) {
  est.aba = aba;
  for (const b of document.querySelectorAll('[data-aba]')) b.setAttribute('aria-selected', String(b.dataset.aba === aba));
  for (const s2 of document.querySelectorAll('.aba')) s2.hidden = s2.id !== `aba-${aba}`;
  if (aba === 'visao') desenharVisao();
  if (aba === 'config') carregarConfig();
  if (aba === 'detalhe') carregarDetalhe();
}

async function iniciar() {
  const { status, corpo } = await api('/eu');
  if (status !== 200) { location.reload(); return; }
  est.eu = corpo;
  document.getElementById('email').textContent = corpo.email;
  for (const b of document.querySelectorAll('[data-aba]')) b.addEventListener('click', () => trocarAba(b.dataset.aba));
  for (const b of document.querySelectorAll('[data-janela]')) b.addEventListener('click', () => {
    est.janela = Number(b.dataset.janela);
    for (const o of document.querySelectorAll('[data-janela]')) o.setAttribute('aria-pressed', String(o === b));
    desenharVisao();
  });
  document.getElementById('sair').addEventListener('click', async () => {
    await fetch('/painel/sair', { method: 'POST', credentials: 'same-origin', redirect: 'manual' }).catch(() => {});
    location.href = '/painel/?erro=saiu';
  });
  addEventListener('uv-tema', () => { if (est.aba === 'visao') desenharVisao(); });
  let rz; addEventListener('resize', () => { clearTimeout(rz); rz = setTimeout(() => { if (est.aba === 'visao') desenharVisao(); }, 150); });
  await atualizar();
  setInterval(atualizar, INTERVALO_MS);
  setInterval(() => { if (est.aba === 'detalhe') carregarDetalhe(); }, 15000);
}

document.addEventListener('DOMContentLoaded', iniciar);

// ------------------------------------------------------------------ endereços do site de resultados

const MODELOS = [
  ['ele_c', 'Lista de pleitos (ele-c.json)'], ['cs', 'Seções da UF (cs.json)'], ['aux', 'Arquivos da seção (aux.json)'],
  ['arquivo', 'Arquivo de urna (BU)'], ['mun_cm', 'Municípios (cm.json)'], ['u_json', 'Resultado oficial (u.json)'],
];

function igualJSON(a, b) { return JSON.stringify(a) === JSON.stringify(b); }

function montarEnderecos(id, e) {
  const caixa = h('section', { class: 'caixa enderecos' });
  const modelos = e.rotas || e.modelos_padrao;
  const pendente = e.rotas_servico && !igualJSON(e.rotas_servico, e.rotas);
  const ex = e.exemplos || {};
  // append do DOM escreveria null como texto: só os itens que existem
  caixa.append(...[
    h('h3', {}, 'Endereços do site de resultados'),
    h('p', {}, 'Este nó coleta de ', h('b', {}, e.base), ` · ambiente ${e.ambiente} · ciclo ${e.ciclo} · pleito ${e.pleito}`, h('br'),
      'Caminhos: ', h('b', {}, e.padrao ? 'padrão do TSE' : 'personalizados'), ' · BUs de ',
      h('b', {}, (e.rotas && e.rotas.base_arquivos) || 'a mesma base')),
    e.erro ? h('p', { class: 'inalcancavel' }, `rotas_tse.json inválido (o nó usa o padrão): ${e.erro}`) : null,
    pendente ? h('p', { class: 'aviso-linha' }, 'Os caminhos gravados ainda não valem: o serviço está rodando com outros. Reinicie o urna-velox.') : null,
    h('details', {}, h('summary', { class: 'mudo' }, 'endereços que o nó monta hoje (exemplo: uma seção de SP)'),
      h('table', { class: 'tabela' }, [['lista de pleitos', ex.ele_c], ['seções da UF', ex.cs], ['arquivos da seção', ex.aux], ['BU', ex.arquivo]]
        .map(([k, v]) => h('tr', {}, h('th', {}, k), h('td', {}, h('code', { class: 'url' }, v || '—'))))))].filter(Boolean));

  const url = h('input', { value: e.base, spellcheck: 'false', autocomplete: 'off', inputmode: 'url', placeholder: 'https://resultados.tse.jus.br' });
  const pleito = h('input', { value: e.pleito || '', inputmode: 'numeric', spellcheck: 'false' });
  const amb = h('select', {}, ['oficial', 'simulado'].map((o) => h('option', { value: o, selected: o === e.ambiente }, o)));
  const outra = h('input', { value: (e.rotas && e.rotas.base_arquivos) || '', spellcheck: 'false', autocomplete: 'off', inputmode: 'url',
    placeholder: '(vazio = a mesma base)' });
  const campos = new Map();
  const grade = h('div', { class: 'campos' });
  for (const [k, rot] of MODELOS) {
    const i = h('input', { id: `m-${id}-${k}`, value: modelos[k] || '', spellcheck: 'false', autocomplete: 'off' });
    campos.set(k, i);
    grade.append(h('label', { for: `m-${id}-${k}` }, rot, h('small', {}, k)), i);
  }
  const restaurar = h('button', { class: 'botao discreto', onclick: () => { for (const [k, i] of campos) i.value = e.modelos_padrao[k]; } }, 'Restaurar o padrão do TSE');
  const avancado = h('details', {}, h('summary', {}, 'Modelos de caminho (avançado)'),
    h('p', { class: 'mudo' }, 'Variáveis: {base}, {base_arquivos}, {amb}, {ciclo}, {pleito}, {pleito06}, {uf}, {mun05}, {zona04}, {secao04}, {hash}, {nome}, ',
      '{eleicao}, {eleicao06}, {abr}, {cargo04}, {subpasta}. Todo modelo começa por {base}/ (o do BU pode começar por {base_arquivos}/).'),
    grade, h('div', { class: 'acoes' }, restaurar));

  const doisNos = h('input', { type: 'checkbox', checked: ids().length > 1 });
  const reiniciar = h('input', { type: 'checkbox' });
  const descobrir = h('button', { class: 'botao' }, 'Descobrir e testar');
  const usar = h('button', { class: 'botao primario' }, 'Usar estes endereços');
  const passos = h('div', { class: 'passos' });
  const conf = h('div', { class: 'confirmar', hidden: true });
  const res = h('div', { class: 'resultado' });
  let proposta = null;

  const rotasDoForm = () => {
    const r = {};
    for (const [k, i] of campos) r[k] = i.value.trim();
    const b = outra.value.trim().replace(/\/+$/, '');
    if (b) r.base_arquivos = b;
    return r;
  };
  const ehPadrao = (r) => !r.base_arquivos && MODELOS.every(([k]) => r[k] === e.modelos_padrao[k]);
  for (const el of [url, pleito, amb, outra, ...campos.values()]) el.addEventListener('input', () => { proposta = null; });

  descobrir.addEventListener('click', async () => {
    descobrir.disabled = true; usar.disabled = true; res.textContent = '';
    passos.replaceChildren(h('p', { class: 'mudo' }, `procurando a partir de ${id} (poucas requisições, espaçadas)…`));
    try {
      const { status, corpo: r } = await api(`/no/${encodeURIComponent(id)}/descobrir`, { method: 'POST',
        body: JSON.stringify({ url: url.value.trim(), pleito: pleito.value.trim(), ambiente: amb.value, base_arquivos: outra.value.trim(), rotas: rotasDoForm() }) });
      if (status !== 200) { passos.replaceChildren(h('p', { class: 'inalcancavel' }, (r && r.mensagem) || `HTTP ${status}`)); return; }
      passos.replaceChildren(mostrarDescoberta(r));
      proposta = r.ok ? r.proposta : null;
    } catch (err) { passos.replaceChildren(h('p', { class: 'inalcancavel' }, err.message)); }
    finally { descobrir.disabled = false; usar.disabled = false; }
  });

  usar.addEventListener('click', () => aplicarEnderecos(id, () => {
    const r = rotasDoForm();
    const p = proposta || { BASE: url.value.trim().replace(/\/+$/, ''), AMBIENTE: amb.value, PLEITO: pleito.value.trim() };
    return { base: p.BASE, AMBIENTE: p.AMBIENTE, CICLO: p.CICLO, PLEITO: p.PLEITO, rotas: ehPadrao(r) ? null : r, reiniciar: reiniciar.checked };
  }, doisNos.checked, usar, conf, res, () => !proposta && !confirm('Você não rodou "Descobrir e testar" com estes endereços. Gravar assim mesmo? O campo da URL vira a BASE do nó.')));

  caixa.append(
    h('div', { class: 'campos' },
      h('label', {}, 'URL de resultados do TSE', h('small', {}, 'a base ou o endereço da página do site')), url,
      h('label', {}, 'Pleito', h('small', {}, 'para testar os BUs agora, use um já apurado')), pleito,
      h('label', {}, 'Ambiente'), amb,
      h('label', {}, 'Buscar os BUs de outro endereço', h('small', {}, 'opcional: espelho ou CDN')), outra),
    avancado,
    h('div', { class: 'acoes' }, descobrir, usar,
      h('label', {}, doisNos, 'nos dois nós'), h('label', {}, reiniciar, 'reiniciar o urna-velox depois')),
    passos, conf, res);
  return caixa;
}

function mostrarDescoberta(r) {
  const lista = h('ol', { class: 'lista-passos' }, (r.passos || []).map((p) => h('li', { class: p.ok ? 'ok' : 'falhou' },
    h('span', { class: 'icone', 'aria-label': p.ok ? 'ok' : 'falhou' }, p.ok ? '✓' : '✗'), h('b', {}, p.passo), ' — ', p.detalhe,
    p.url ? h('div', {}, h('code', { class: 'url' }, p.url)) : null)));
  const fim = r.ok
    ? (r.bu_ok === true ? 'Tudo no lugar: listas de seções e BUs encontrados e baixados.'
      : r.bu_ok === false ? 'Achei o servidor e as seções, mas o BU não conferiu. Veja os passos.'
      : 'Achei o servidor e as seções; ainda não há BU publicado para testar o caminho dos BUs.')
    : (r.erro || 'Não deu certo. Veja os passos.');
  const pro = r.proposta ? h('p', {}, 'Proposta: base ', h('b', {}, r.proposta.BASE), ` · ambiente ${r.proposta.AMBIENTE || '—'} · ciclo ${r.proposta.CICLO || '—'} · pleito ${r.proposta.PLEITO || '—'}`) : null;
  const pls = (r.pleitos || []).length && !r.ok ? h('details', { open: true }, h('summary', {}, 'pleitos deste servidor'),
    h('ul', {}, r.pleitos.map((x) => h('li', {}, `${x.cd} · ${x.ciclo} · ${x.data} — ${(x.eleicoes || []).join(', ')}`)))) : null;
  return h('div', {}, h('p', { class: r.ok && r.bu_ok !== false ? 'resultado ok' : 'resultado erro' }, fim, ` (${r.requisicoes ?? 0} requisições)`), pro, lista, pls);
}

/// Grava os endereços num nó ou nos dois; a frase de confirmação é pedida uma vez e vale para todos.
async function aplicarEnderecos(id, corpoFn, doisNos, botao, conf, res, cancelar) {
  if (cancelar()) return;
  const alvos = doisNos ? ids() : [id];
  const corpo = corpoFn();
  res.textContent = ''; res.className = 'resultado'; botao.disabled = true;
  const um = async (no, confirmacao) => api(`/no/${encodeURIComponent(no)}/rotas`, { method: 'POST', body: JSON.stringify({ ...corpo, confirmacao }) });
  try {
    let frase = '';
    const prim = await um(alvos[0], '');
    if (prim.status === 409 && prim.corpo && prim.corpo.confirmacao) {
      frase = await pedirFrase(conf, prim.corpo);
      if (frase == null) return;
    } else if (prim.status !== 200) {
      res.className = 'resultado erro'; res.textContent = `${alvos[0]}: ${(prim.corpo && prim.corpo.mensagem) || prim.status}`; return;
    }
    const linhas = [];
    for (const no of alvos) {
      if (no === alvos[0] && !frase) { linhas.push(`${no}: ${resumo(prim.corpo)}`); continue; }
      const r = await um(no, frase);
      linhas.push(`${no}: ${r.status === 200 ? resumo(r.corpo) : 'ERRO ' + ((r.corpo && r.corpo.mensagem) || r.status)}`);
    }
    res.className = linhas.some((l) => l.includes('ERRO')) ? 'resultado erro' : 'resultado ok';
    res.textContent = linhas.join('\n');
    setTimeout(carregarConfig, 1500);
  } catch (err) { res.className = 'resultado erro'; res.textContent = err.message; }
  finally { botao.disabled = false; }
}

function resumo(r) {
  if (!r) return 'feito';
  const p = [];
  if (r.rotas_mudaram) p.push('caminhos gravados');
  if (r.mudou && r.mudou.length) p.push(`no.env: ${r.mudou.join(', ')}`);
  if (r.reiniciado) p.push('serviço reiniciado');
  if (!p.length) p.push(r.mensagem || 'nada mudou');
  return p.join(' · ');
}

function pedirFrase(conf, r) {
  return new Promise((ok) => {
    const inp = h('input', { placeholder: r.confirmacao, autocomplete: 'off', spellcheck: 'false' });
    const sim = h('button', { class: 'botao perigo' }, 'Confirmar');
    const nao = h('button', { class: 'botao discreto' }, 'Cancelar');
    sim.addEventListener('click', () => { conf.hidden = true; ok(inp.value.trim()); });
    nao.addEventListener('click', () => { conf.hidden = true; ok(null); });
    conf.replaceChildren(h('p', {}, r.mensagem, '. Digite ', h('b', {}, r.confirmacao), ' para seguir.'), h('div', { class: 'acoes' }, inp, sim, nao));
    conf.hidden = false; inp.focus();
  });
}
