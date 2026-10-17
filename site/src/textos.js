// Textos do feed: cada marco do servidor, cada evento do "matematicamente" e cada curiosidade calculada aqui vira um
// item curto (com o detalhe que abre ao tocar). Só seção, zona e município: nada que exponha pessoa.

import { NOME_CARGO, NOME_CARGO_PLURAL, abr, hora, mil, pct, titulo } from './util.js';

export class Nomes {
  constructor(n) {
    this.n = n || { ufs: {}, municipios: {}, eleicoes: [] };
    this.cand = new Map();   // "circ-cNNNN" → Map(numero → candidato)
    this.cargos = new Map(); // cargo → { consulta, vagas, turno }
    for (const e of this.n.eleicoes || []) {
      for (const c of e.cargos || []) {
        this.cargos.set(c.cargo, { consulta: c.consulta, vagas: c.vagas, turno: e.turno, eleicao: e.codigo });
        for (const ci of c.circunscricoes || []) {
          const m = new Map();
          [...ci.candidatos].sort((a, b) => a.n - b.n).forEach((k, i) => m.set(k.n, { ...k, idx: i }));
          this.cand.set(`${ci.abr}-c${String(c.cargo).padStart(4, '0')}`, m);
        }
      }
    }
  }
  uf(u) { return u === 'zz' ? 'Exterior' : titulo(this.n.ufs?.[u] || u.toUpperCase()); }
  mun(cod) { return titulo(this.n.municipios?.[cod] || cod); }
  /** "Brasil", "São Paulo", "Manaus (AM)" */
  local(cod) {
    const a = abr(cod);
    if (a.tipo === 'br') return 'Brasil';
    if (a.tipo === 'uf') return this.uf(a.uf);
    return `${this.mun(cod)} (${a.uf.toUpperCase()})`;
  }
  cargo(c) { return NOME_CARGO[c] || (this.cargos.get(c)?.consulta || c >= 25 ? 'Consulta' : `Cargo ${c}`); }
  /** "Governador em São Paulo", "Presidente", "Prefeito de Manaus (AM)" */
  disputa(cod, c) {
    const a = abr(cod);
    const nome = this.cargo(c);
    if (a.tipo === 'br') return nome;
    if (a.tipo === 'uf') return `${nome} em ${this.uf(a.uf)}`;
    return `${nome} de ${this.local(cod)}`;
  }
  candidato(circ, c, n) {
    const k = this.cand.get(`${circ}-c${String(c).padStart(4, '0')}`)?.get(n);
    return k ? { nome: k.nome_urna || k.nome || String(n), partido: k.partido || '', vices: k.vices || [], idx: k.idx } : { nome: String(n), partido: '', vices: [], idx: n % 10 };
  }
  nomeCand(circ, c, n) { const k = this.candidato(circ, c, n); return k.partido ? `${k.nome} (${k.partido})` : k.nome; }
}

function lugar(N, m) {
  if (m.municipio == null) return '';
  const nome = m.municipio_nome ? titulo(m.municipio_nome) : N.mun(`${m.uf}${String(m.municipio).padStart(5, '0')}`);
  return `${nome} (${String(m.uf).toUpperCase()})`;
}

function linhasSecao(N, m) {
  return [['Seção', String(m.secao)], ['Zona', String(m.zona)], ['Município', lugar(N, m)], ['Horário do BU', hora(m.horario)], ['BUs aplicados (seq)', mil(m.seq)]];
}

/** Categoria de um item no histórico do feed: decisões (eleitos, 2º turno, seção decisiva, primeiros eleitos),
 * viradas de liderança, andamento (primeira/última seção, % de seções, BU nº N, apuração encerrada). */
export const CATEGORIAS = [['decisao', 'Decisões'], ['virada', 'Viradas'], ['andamento', 'Andamento']];

/** Item do feed para um marco do servidor (exceto `faltam_para_mat`, que é um item fixo). */
export function itemMarco(N, m) {
  const it = corpoMarco(N, m);
  if (it) Object.assign(it, { seq: m.seq, cat: m.classe === 'secao_decisiva' ? 'decisao' : m.classe === 'virada' ? 'virada' : 'andamento' });
  return it;
}
function corpoMarco(N, m) {
  const id = `m:${m.classe}:${m.abrangencia}:${m.cargo ?? ''}:${m.seq}:${m.secao ?? ''}:${m.valor ?? ''}`;
  const t = hora(m.horario);
  switch (m.classe) {
    case 'primeira_secao': {
      const onde = m.abrangencia === 'br' ? 'do Brasil' : `em ${N.local(m.abrangencia)}`;
      return { id, ic: '1', t, texto: `Primeira seção totalizada ${onde}: seção ${m.secao}, zona ${m.zona}, ${lugar(N, m)}`,
        anunciar: `Primeira seção totalizada ${onde}, às ${t}.`,
        detalhe: { titulo: `Primeira seção totalizada ${onde}`, linhas: linhasSecao(N, m) } };
    }
    case 'secao_decisiva': {
      const d = N.disputa(m.abrangencia, m.cargo);
      const o = m.estado === 'matematicamente_2t' ? 'o 2º turno' : 'o resultado';
      return { id, ic: '◆', t, destaque: true, texto: `A seção ${m.secao} (zona ${m.zona}, ${lugar(N, m)}) garantiu ${o} — ${d}`,
        anunciar: `${d}: seção ${m.secao} de ${lugar(N, m)} garantiu ${o}.`,
        detalhe: { titulo: `Seção que garantiu ${o} — ${d}`, linhas: [...linhasSecao(N, m), ['BUs no mesmo lote', String(m.lote_bus ?? 1)],
          ['Observação', (m.lote_bus ?? 1) > 1 ? 'o motor reavalia por lote: esta é a última seção do lote dentro da circunscrição' : 'lote com um BU só']] } };
    }
    case 'bu_marco':
      return { id, ic: '#', t, texto: `BU nº ${mil(m.valor)} da noite: seção ${m.secao}, zona ${m.zona}, ${lugar(N, m)}`,
        detalhe: { titulo: `BU nº ${mil(m.valor)} da noite`, linhas: linhasSecao(N, m) } };
    case 'ultima_secao':
      return { id, ic: '■', t, texto: `Última seção da noite: seção ${m.secao}, zona ${m.zona}, ${lugar(N, m)}`,
        detalhe: { titulo: 'Última seção aplicada', linhas: linhasSecao(N, m) } };
    case 'pct_secoes': {
      const onde = N.local(m.abrangencia);
      return { id, ic: '%', t, texto: `${onde}: ${m.valor}% das seções apuradas`,
        detalhe: { titulo: `${onde}: ${m.valor}% das seções apuradas`, linhas: [['Horário', t], ['Seções', m.secoes != null ? `${mil(m.secoes)} de ${mil(m.total)}` : '—'], ['BUs aplicados (seq)', mil(m.seq)]] } };
    }
    case 'virada': {
      const d = N.disputa(m.abrangencia, m.cargo);
      const novo = N.nomeCand(m.abrangencia, m.cargo, m.candidato), ant = N.nomeCand(m.abrangencia, m.cargo, m.anterior);
      return { id, ic: '⇅', t, texto: `Virada — ${d}: ${novo} passa ${ant} (${m.valor}ª da noite)`,
        detalhe: { titulo: `Virada de liderança — ${d}`, linhas: [['Novo líder', novo], ['Antes', ant], ['Viradas nesta disputa', String(m.valor)], ['Horário', t], ['BUs aplicados (seq)', mil(m.seq)]] } };
    }
    default:
      return null;
  }
}

/** Item do feed para um evento do "matematicamente" (só os de candidato e a apuração encerrada). */
export function itemEvento(N, e) {
  const t = hora(e.horario);
  const id = `e:${e.seq}:${e.abrangencia}:${e.cargo}:${e.candidato}:${e.estado}`;
  if (e.classe === 'apuracao') {
    if (e.estado !== 'apuracao_encerrada') return null;
    return { id, ic: '✓', t, seq: e.seq, cat: 'andamento', texto: 'Apuração encerrada: todas as seções totalizadas', anunciar: 'Apuração encerrada.',
      detalhe: { titulo: 'Apuração encerrada', linhas: [['Horário', t], ['BUs aplicados', mil(e.seq)]] } };
  }
  if (String(e.estado).startsWith('revogado:') || e.candidato == null) return null;
  const d = N.disputa(e.abrangencia, e.cargo);
  const quem = N.nomeCand(e.abrangencia, e.cargo, e.candidato);
  const o = e.estado === 'eleito' ? 'matematicamente eleito' : 'matematicamente no 2º turno';
  return { id, ic: '★', t, seq: e.seq, cat: 'decisao', destaque: true, texto: `${quem}: ${o} — ${d}`, anunciar: `${d}: ${quem} ${o}.`,
    detalhe: { titulo: `${quem} — ${o}`, linhas: [['Disputa', d], ['Horário do BU que decidiu', t], ['BUs aplicados (seq)', mil(e.seq)]] } };
}

/** Primeiros eleitos por cargo (país e cada UF), dos eventos em ordem de `seq`. */
export function primeiros(N, eventos) {
  const vistos = new Map();
  const out = [];
  const ord = [...eventos].filter((e) => e.classe === 'matematicamente' && e.estado === 'eleito').sort((a, b) => a.seq - b.seq);
  for (const e of ord) {
    const a = abr(e.abrangencia);
    const nome = NOME_CARGO_PLURAL[e.cargo];
    if (!nome) continue;
    const quem = N.nomeCand(e.abrangencia, e.cargo, e.candidato);
    const t = hora(e.horario);
    const pais = `pais:${e.cargo}`;
    if (!vistos.has(pais) && e.cargo !== 1) {
      vistos.set(pais, 1);
      out.push({ id: `p:${pais}`, ic: '①', t, seq: e.seq, cat: 'decisao', destaque: true, texto: `Primeiro ${nome} matematicamente eleito do país: ${quem}, ${N.local(e.abrangencia)}`,
        detalhe: { titulo: `Primeiro ${nome} matematicamente eleito do país`, linhas: [['Eleito', quem], ['Onde', N.local(e.abrangencia)], ['Horário', t], ['BUs aplicados (seq)', mil(e.seq)]] } });
    }
    // por UF só faz sentido onde a UF tem várias circunscrições do cargo (Prefeito)
    if (a.tipo === 'mun') {
      const k = `uf:${e.cargo}:${a.uf}`;
      if (!vistos.has(k)) {
        vistos.set(k, 1);
        out.push({ id: `p:${k}`, ic: '①', t, seq: e.seq, cat: 'decisao', texto: `Primeiro ${nome} eleito em ${N.uf(a.uf)}: ${quem}, ${N.local(e.abrangencia)}`,
          detalhe: { titulo: `Primeiro ${nome} matematicamente eleito em ${N.uf(a.uf)}`, linhas: [['Eleito', quem], ['Onde', N.local(e.abrangencia)], ['Horário', t]] } });
      }
    }
  }
  return out;
}

/** "hh:mm" de "dd/mm/aaaa hh:mm:ss", ISO ou "aaaa-mm-dd hh:mm:ss". */
function hhmm(t) { const m = /(\d{2}):(\d{2}):\d{2}/.exec(t || ''); return m ? `${m[1]}:${m[2]}` : ''; }

/** Comparação com o TSE (etapa 7): o arquivo do TSE mostrou a mesma situação ("matematicamente") que o Urna Velox.
 * `tUv`/`tTse` em segundos (mesma escala); `emUv` = horário do evento no Urna Velox, `geracao` = do arquivo do TSE. */
export function itemConfirmou(N, circ, cargo, sit, emUv, geracao, tUv, tTse) {
  const d = N.disputa(circ, cargo);
  const min = Math.round((tTse - tUv) / 60);
  const quando = min > 0 ? `${min} min antes` : min < 0 ? `${-min} min depois` : 'no mesmo minuto';
  const o = sit === '2t' ? 'o 2º turno' : 'o matematicamente';
  return { id: `c:tse-conf:${circ}:${cargo}`, ic: '⇄', t: hora(geracao), cat: 'decisao', destaque: true,
    texto: `TSE confirmou ${o} às ${hhmm(geracao)} — Urna Velox às ${hhmm(emUv)}, ${quando} · ${d}`,
    anunciar: `${d}: o TSE confirmou ${o} às ${hhmm(geracao)}; o Urna Velox às ${hhmm(emUv)}, ${quando}.`,
    detalhe: { titulo: `TSE confirmou ${o} — ${d}`, linhas: [['Urna Velox', hora(emUv)], ['TSE (arquivo gerado às)', hora(geracao)],
      ['Antecedência do Urna Velox', quando], ['Como se compara', 'o arquivo de resultado do TSE desta disputa (buscado a cada 30–60 s) passou a mostrar a mesma situação']] } };
}

/** Comparação com o TSE: a ordem dos candidatos difere entre as fontes há mais de dois arquivos do TSE. */
export function itemOrdemDiferente(N, circ, cargo, geracao, quem) {
  const d = N.disputa(circ, cargo);
  return { id: `c:tse-ordem:${circ}:${cargo}:${geracao}`, ic: '≠', t: hora(geracao), cat: 'andamento',
    texto: `Ordem diferente entre as fontes — ${d}: ${quem}`,
    detalhe: { titulo: `Ordem diferente entre Urna Velox e TSE — ${d}`, linhas: [['Desde o arquivo do TSE das', hora(geracao)], ['Candidatos', quem],
      ['Por quê', 'o Urna Velox costuma estar à frente (BUs aplicados antes de o TSE divulgar); com mais seções nas duas fontes, a ordem tende a coincidir']] } };
}

export { pct, mil };
