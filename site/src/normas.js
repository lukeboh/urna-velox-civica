// Normas citadas na página: todo artigo de lei que a página mostra para explicar uma regra é um link para a norma no
// site do TSE, numa aba nova, com âncora no artigo exato (regra do Luciano, 29/10/2026; CLAUDE.md, "Convenções").
//
// A âncora é um fragmento de texto (`#:~:text=`, Text Fragments: Chrome/Edge 89+, Safari 16.1+, Firefox 131+), com o
// começo literal do artigo: o navegador rola até ele e o destaca, sem depender de um `id` na página do TSE (que não
// publica âncoras estáveis por artigo). Navegador sem suporte abre a norma no topo. Artigo novo: uma entrada em
// ARTIGOS, com o início do caput copiado da norma (único na página) — o teste `normas_links.mjs` confere a tabela.

import { el } from './util.js';

export const NORMAS = {
  CE: {
    nome: 'Código Eleitoral (Lei nº 4.737, de 15 de julho de 1965)',
    url: 'https://www.tse.jus.br/legislacao/codigo-eleitoral/codigo-eleitoral-1/codigo-eleitoral-lei-nb0-4.737-de-15-de-julho-de-1965',
  },
};

/** Artigos citados: chave = como aparece na página ("art. 224"); `inicio` = começo literal do caput na norma. */
export const ARTIGOS = {
  'art. 224': {
    norma: 'CE', artigo: '224', inicio: 'Art. 224. Se a nulidade atingir a mais de metade',
    resumo: 'nova eleição quando a nulidade atinge mais da metade dos votos (caput) ou quando o eleito em pleito majoritário tem o registro indeferido ou é cassado (§ 3º)',
  },
};

/** Fragmento de texto: espaço e reservados codificados; o hífen (delimitador da sintaxe) também. */
const fragmento = (s) => encodeURIComponent(s).replace(/-/g, '%2D');

/** URL da norma com a âncora no artigo. */
export function urlArtigo(chave) {
  const a = ARTIGOS[chave];
  if (!a) return null;
  return `${NORMAS[a.norma].url}#:~:text=${fragmento(a.inicio)}`;
}

/** O link de um artigo (aba nova, sem enviar a página de origem). */
export function linkArtigo(chave, texto = chave) {
  const a = ARTIGOS[chave];
  const url = urlArtigo(chave);
  if (!url) return texto;
  const l = el('a', 'norma', texto);
  l.href = url;
  l.target = '_blank';
  l.rel = 'noopener noreferrer';
  l.title = `${NORMAS[a.norma].nome}, art. ${a.artigo}: ${a.resumo}. Abre o texto no site do TSE, numa aba nova.`;
  return l;
}

const RE = new RegExp(`(${Object.keys(ARTIGOS).map((k) => k.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')).join('|')})`, 'g');

/** Texto → nós, com cada artigo conhecido trocado pelo link dele. */
export function comLinks(texto) {
  if (!texto) return [];
  return texto.split(RE).filter((p) => p !== '').map((p) => (ARTIGOS[p] ? linkArtigo(p) : p));
}
