// Multipleito (etapa 1.1): todos os pleitos publicados no site de resultados do TSE, com a situação de cada um
// (`GET /api/v1/pleitos`, o pleitos.json do `uv-coletor pleitos`; docs/arquitetura/PROTOCOLO_WS.md §6). O motor
// totaliza um pleito por vez: aqui se vê o que está no ar, os agendados (data, cargos, onde), os em curso e os
// encerrados (os que têm o acervo pronto estão na playlist de replays). Sem a lista (serviço sem `--pleitos`), nada
// aparece. O painel é fixo e de altura fixa, como o histórico: abrir ou filtrar não desloca nada (CLS 0).

import { basesConsulta } from './serie.js';
import { el, mil } from './util.js';

const SITUACOES = [
  ['em_curso', 'Em curso', 'Do dia da eleição até as 8 h do dia seguinte: a apuração da noite.'],
  ['agendado', 'Agendados', 'Já publicados pelo TSE, com a data marcada.'],
  ['encerrado', 'Encerrados', 'Apuração terminada. Os que estão na playlist podem ser vistos de novo em replay, com os dados reais.'],
];

/** O pleitos.json (null sem ele). */
/** Os pleitos com a série de consulta (`/consulta/indice.json`: `{pleitos: [{pleito, …}]}`); sem ela, nenhum. */
export async function buscarConsultas() {
  for (const b of basesConsulta()) {
    try {
      const r = await fetch(`${b}/indice.json`, { credentials: 'omit' });
      if (!r.ok) continue;
      const j = await r.json();
      return new Set((j.pleitos || []).map((x) => String(x.pleito)));
    } catch { /* a próxima base */ }
  }
  return new Set();
}

export async function buscarPleitos() {
  try {
    const r = await fetch('/api/v1/pleitos');
    if (!r.ok) return null;
    const j = await r.json();
    return j && Array.isArray(j.pleitos) ? j : null;
  } catch { return null; }
}

const DIAS = ['domingo', 'segunda', 'terça', 'quarta', 'quinta', 'sexta', 'sábado'];

/** "aaaa-mm-dd" → dias inteiros de `hoje` (aaaa-mm-dd do visitante, no fuso de Brasília) até a data. */
export function diasAte(iso, hoje) {
  const a = Date.UTC(+iso.slice(0, 4), +iso.slice(5, 7) - 1, +iso.slice(8, 10));
  const b = Date.UTC(+hoje.slice(0, 4), +hoje.slice(5, 7) - 1, +hoje.slice(8, 10));
  return Math.round((a - b) / 86400e3);
}

/** "domingo, 06/12/2026" */
export function dataTxt(p) {
  const d = new Date(Date.UTC(+p.data_iso.slice(0, 4), +p.data_iso.slice(5, 7) - 1, +p.data_iso.slice(8, 10)));
  return `${DIAS[d.getUTCDay()]}, ${p.data}`;
}

/** Cargos do pleito, em ordem e sem repetir: "Presidente · Governador · Senador", "Prefeito"; as consultas populares
 * (o "cargo" delas no ele-c.json é o texto da pergunta) viram "3 consultas populares". */
export function cargosTxt(p) {
  const cargos = new Map();
  let consultas = 0;
  for (const e of p.eleicoes || []) {
    for (const c of e.cargos || []) {
      if (e.tipo === 'consulta') consultas++;
      else if (c.nome && !cargos.has(c.codigo)) cargos.set(c.codigo, c.nome);
    }
  }
  const v = [...cargos].sort((a, b) => a[0] - b[0]).map((x) => x[1]);
  if (consultas) v.push(consultas === 1 ? 'consulta popular' : `${consultas} consultas populares`);
  return v.join(' · ');
}

/** Onde: "Brasil", "Paranhos (MS)", "Neves Paulista (SP) e Eldorado (SP)", "Roraima", "5 UFs". */
export function ondeTxt(p) {
  if ((p.ufs || []).includes('br')) return p.complementares ? `Brasil, com ${p.complementares} ${p.complementares === 1 ? 'eleição suplementar ou consulta' : 'eleições suplementares e consultas'} no mesmo dia` : 'Brasil';
  const locais = (p.eleicoes || []).map((e) => e.local).filter(Boolean);
  if (locais.length) return locais.length > 3 ? `${locais.length} lugares` : juntar(locais);
  const ufs = (p.ufs || []).map((u) => u.toUpperCase());
  return ufs.length > 3 ? `${ufs.length} UFs` : juntar(ufs);
}

function juntar(v) { return v.length < 2 ? (v[0] || '') : `${v.slice(0, -1).join(', ')} e ${v[v.length - 1]}`; }

/** "faltam 3 dias", "amanhã", "hoje" (agendado); "" nos outros. */
export function faltaTxt(p, hoje) {
  if (p.situacao !== 'agendado') return '';
  const n = diasAte(p.data_iso, hoje);
  return n <= 0 ? 'hoje' : n === 1 ? 'amanhã' : `faltam ${mil(n)} dias`;
}

/** Contagem para o botão: "2 agendados · 1 em curso · 43 encerrados". */
export function resumoTxt(lista) {
  const c = lista?.contagem || {};
  const v = [];
  if (c.em_curso) v.push(`${c.em_curso} em curso`);
  if (c.agendado) v.push(`${c.agendado} ${c.agendado === 1 ? 'agendado' : 'agendados'}`);
  if (c.encerrado) v.push(`${c.encerrado} ${c.encerrado === 1 ? 'encerrado' : 'encerrados'}`);
  return v.join(' · ');
}

const hojeBr = () => new Intl.DateTimeFormat('en-CA', { timeZone: 'America/Sao_Paulo' }).format(new Date());

/** O painel dos pleitos (criado na primeira abertura; o `noAr` diz o código do pleito que o serviço totaliza). */
export class PainelPleitos {
  /** `consultas()`: os códigos dos pleitos com a série de consulta (`/consulta/indice.json`; 1.1, serie.js) */
  /** `naTela()`: o pleito na tela (o da consulta ou o que o serviço totaliza); `emConsulta()`: a página está numa consulta */
  constructor({ noAr, comPlaylist, consultas = () => new Set(), naTela = noAr, emConsulta = () => false }) {
    this.noAr = noAr;
    this.naTela = naTela;
    this.emConsulta = emConsulta;
    this.comPlaylist = comPlaylist;
    this.consultas = consultas;
    this.lista = null;
    this.busca = '';
    this.raiz = null;
  }

  definir(lista) {
    this.lista = lista;
    if (this.aberto) this.#desenhar();
  }

  get aberto() { return !!this.raiz?.classList.contains('aberta'); }

  #montar() {
    const fechar = el('button', 'icone-btn fechar', '✕');
    fechar.type = 'button';
    fechar.setAttribute('aria-label', 'Fechar a lista de pleitos');
    fechar.addEventListener('click', () => this.fechar());
    this.busca = '';
    const busca = el('input', 'hist-busca');
    busca.type = 'search';
    busca.placeholder = 'Filtrar: cidade, UF, ano, cargo…';
    busca.setAttribute('aria-label', 'Filtrar os pleitos');
    busca.autocomplete = 'off';
    busca.addEventListener('input', () => { this.busca = busca.value.trim().toLowerCase(); this.#desenhar(); });
    this.sub = el('p', 'pl-sub', '');
    this.corpo = el('div', 'pl-lista');
    this.corpo.setAttribute('role', 'list');
    this.raiz = el('section', 'pleitos vidro', [
      el('header', 'hist-cab', [el('h2', null, 'Escolha a eleição'), fechar]),
      // M-024: o que se faz aqui, em uma linha
      el('div', null, [el('p', 'pl-intro', 'Toque numa eleição para ver o resultado e reviver a noite da apuração.'), this.sub]),
      el('div', 'hist-filtros', busca), this.corpo,
    ]);
    this.raiz.id = 'pleitos';
    this.raiz.setAttribute('role', 'dialog');
    this.raiz.setAttribute('aria-modal', 'false');
    this.raiz.setAttribute('aria-label', 'Escolha a eleição (pleitos do TSE)');
    this.raiz.setAttribute('aria-hidden', 'true');
    document.body.append(this.raiz);
    document.addEventListener('keydown', (e) => { if (e.key === 'Escape' && this.aberto) this.fechar(); });
  }

  abrir() {
    if (!this.lista) return;
    if (!this.raiz) this.#montar();
    this.#desenhar();
    this.raiz.classList.add('aberta');
    this.raiz.setAttribute('aria-hidden', 'false');
  }

  fechar() {
    if (!this.aberto) return;
    this.raiz.classList.remove('aberta');
    this.raiz.setAttribute('aria-hidden', 'true');
  }

  #desenhar() {
    const l = this.lista;
    const noAr = this.noAr();
    const hoje = hojeBr();
    this.sub.textContent = `Pleitos do TSE: ${resumoTxt(l)}${l.ele_c?.dg ? ` · lista atualizada pelo TSE em ${l.ele_c.dg}` : ''}`;
    const filtro = (p) => !this.busca || [p.titulo, p.nome, p.data, String(p.codigo), cargosTxt(p), ondeTxt(p), ...(p.ufs || [])]
      .join(' ').toLowerCase().includes(this.busca);
    const blocos = [];
    for (const [sit, nome, dica] of SITUACOES) {
      let itens = l.pleitos.filter((p) => p.situacao === sit && filtro(p));
      // agendados: o mais próximo primeiro; os outros, o mais recente primeiro (a ordem do pleitos.json)
      if (sit === 'agendado') itens = [...itens].sort((a, b) => a.data_iso.localeCompare(b.data_iso));
      if (!itens.length) continue;
      const cab = el('h3', 'pl-grupo', [nome, el('small', null, ` ${mil(itens.length)}`)]);
      cab.title = dica;
      blocos.push(cab, ...itens.map((p) => this.#item(p, noAr, hoje, String(this.naTela() ?? ''), this.emConsulta())));
    }
    if (!blocos.length) blocos.push(el('p', 'hist-vazio', 'Nenhum pleito passa no filtro.'));
    this.corpo.replaceChildren(...blocos);
  }

  #item(p, noAr, hoje, naTela, emConsulta) {
    const cod = String(p.codigo);
    const vivo = cod === String(noAr ?? ''), tela = cod === naTela;
    const marcas = [];
    if (vivo) marcas.push(el('span', 'pl-chip no-ar', 'no ar'));
    if (tela) marcas.push(el('span', 'pl-chip tela', 'na tela'));
    if (p.situacao === 'em_curso') marcas.push(el('span', 'pl-chip vivo', 'em curso'));
    const falta = faltaTxt(p, hoje);
    if (falta) marcas.push(el('span', 'pl-chip', falta));
    const linhas = [
      el('div', 'pl-l1', [el('b', 'pl-nome', p.titulo || p.nome), ...marcas]),
      el('div', 'pl-l2', [dataTxt(p), ' · ', ondeTxt(p)]),
    ];
    const cargos = cargosTxt(p);
    if (cargos) linhas.push(el('div', 'pl-l3', cargos));
    // M-024: as ações em botões, a principal em destaque; o item inteiro leva à principal
    const acao = (cls, txt, href, dica) => { const a = el('a', `pl-acao ${cls}`, txt); a.href = href; if (dica) a.title = dica; return a; };
    const acoes = [];
    let principal = null;
    const comSerie = this.consultas().has(cod);
    // o que está no ar, visto de dentro de uma consulta: volta à página em tempo real
    if (vivo && emConsulta) {
      principal = acao(`primaria${p.situacao === 'em_curso' ? ' vivo' : ''}`, p.situacao === 'em_curso' ? '● Ver ao vivo' : '▶ Ver o replay no ar', '/#/',
        'A apuração que está no ar agora');
      acoes.push(principal);
    }
    if (comSerie && (p.situacao === 'encerrado' || p.situacao === 'em_curso')) {
      // a noite inteira (ou, em curso, até o último minuto consolidado) na linha do tempo de quem vê
      const em = p.situacao === 'em_curso';
      const c = acao(`${principal ? '' : 'primaria '}pl-consulta`, em ? '⧗ Linha do tempo até agora' : 'Ver esta eleição ›', `/#/p/${cod}`,
        em ? 'A noite em curso até o último minuto, com a linha do tempo' : 'O resultado e a noite da apuração inteira, minuto a minuto, com a linha do tempo');
      acoes.push(c);
      principal ||= c;
    }
    if (p.situacao === 'encerrado' && p.replay && this.comPlaylist()) acoes.push(acao('', '▶ Pedir na playlist', '/playlist', 'Votar para esta eleição passar no replay do site, para todos'));
    if (acoes.length) linhas.push(el('div', 'pl-acoes', acoes));
    if (p.situacao === 'encerrado' && !p.replay && !comSerie) linhas.push(el('div', 'pl-l3 pl-fraco', 'replay em preparo (os boletins de urna ainda estão sendo reunidos)'));
    else if (p.situacao === 'encerrado' && !p.replay) linhas.push(el('div', 'pl-l3 pl-fraco', 'replay na playlist em preparo'));
    if (p.situacao === 'agendado') linhas.push(el('div', 'pl-l3 pl-fraco', 'No dia da eleição, a apuração aparece aqui ao vivo.'));
    const it = el('div', `pl-item s-${p.situacao}${principal ? ' clicavel' : ''}${tela ? ' na-tela' : ''}`, linhas);
    it.setAttribute('role', 'listitem');
    it.dataset.pleito = cod;
    if (principal) {
      it.addEventListener('click', (e) => { if (e.target.closest('a, button')) return; principal.click(); });
      for (const a of acoes) a.addEventListener('click', () => this.fechar());
    }
    return it;
  }
}
