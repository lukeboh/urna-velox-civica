// Páginas de texto do site (/termos e /privacidade), geradas no build a partir dos arquivos do repositório — a fonte
// única: `docs/LICENCIAMENTO.md`, `LICENSE`, `LICENCA.pt-BR.md` e `docs/PRIVACIDADE.md`. Assim o visitante lê as regras
// no próprio site, sem depender do GitHub, e o texto do site nunca se afasta do repositório.
// Conversor de Markdown mínimo, só para o que esses arquivos usam: títulos, parágrafos, listas, citações, tabelas,
// **negrito**, *itálico*, `código` e links. Endereços externos viram texto (a página não leva a lugar nenhum fora do
// site); links entre os documentos viram links internos (MAPA).
import { readFileSync } from 'node:fs';
import path from 'node:path';

// destino no site de cada arquivo citado por link relativo (sem destino: fica só o texto)
const MAPA = {
  'LICENSE': '/termos#licenca-en', '../LICENSE': '/termos#licenca-en',
  'LICENCA.pt-BR.md': '/termos#licenca-pt', '../LICENCA.pt-BR.md': '/termos#licenca-pt',
  'LICENCIAMENTO.md': '/termos', 'docs/LICENCIAMENTO.md': '/termos',
  'PRIVACIDADE.md': '/privacidade', 'docs/PRIVACIDADE.md': '/privacidade',
};

const esc = (s) => s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
// âncora no estilo do GitHub (os arquivos se citam por elas), com o prefixo da seção da página
export const ancora = (t) => t.toLowerCase().replace(/[`*_]/g, '').replace(/[^\p{L}\p{N}\s-]/gu, '').trim().replace(/\s/g, '-');

function linha(s, pre) {
  const cod = [];
  s = s.replace(/`([^`]+)`/g, (_, c) => { cod.push(c); return `\u0000${cod.length - 1}\u0000`; });
  s = esc(s);
  s = s.replace(/&lt;(https?:\/\/[^\s&]+)&gt;/g, '$1');
  s = s.replace(/\[([^\]]+)\]\(([^)\s]+)\)/g, (_, txt, url) => {
    url = url.replace(/&amp;/g, '&');
    if (url.startsWith('#')) return `<a href="#${pre}-${url.slice(1)}">${txt}</a>`;
    if (/^https?:/.test(url)) return url.startsWith('https://urnavelox.com.br/') ? `<a href="${url.slice(24)}">${txt}</a>` : txt;
    const [arq, frag] = url.split('#');
    const dest = MAPA[arq];
    return dest ? `<a href="${dest}${frag && !dest.includes('#') ? `#${dest === '/termos' ? 'termos' : 'privacidade'}-${frag}` : ''}">${txt}</a>` : txt;
  });
  s = s.replace(/\*\*(.+?)\*\*/g, '<b>$1</b>').replace(/(^|[^\w*])\*([^*\s][^*]*)\*/g, '$1<i>$2</i>');
  return s.replace(/\u0000(\d+)\u0000/g, (_, i) => `<code>${esc(cod[+i])}</code>`);
}

/** Markdown → HTML; `pre` prefixa as âncoras (`#x` vira `#<pre>-x`). */
export function markdown(texto, pre) {
  const ls = texto.replace(/\r/g, '').split('\n');
  const out = [];
  let i = 0;
  const bloco = (ini) => { const b = []; while (i < ls.length && ls[i].trim() && ini(ls[i])) b.push(ls[i++]); return b; };
  while (i < ls.length) {
    const l = ls[i];
    if (!l.trim()) { i++; continue; }
    let m;
    if ((m = /^(#{1,6})\s+(.*)$/.exec(l))) {
      const n = Math.min(m[1].length + 1, 6);
      out.push(`<h${n} id="${pre}-${ancora(m[2])}">${linha(m[2], pre)}</h${n}>`); i++;
    } else if (/^\s*[-*]\s+/.test(l)) {
      const itens = [];
      while (i < ls.length && ls[i].trim() && (/^\s*[-*]\s+/.test(ls[i]) || /^\s{2,}\S/.test(ls[i]))) {
        if (/^\s*[-*]\s+/.test(ls[i])) itens.push(ls[i].replace(/^\s*[-*]\s+/, '')); else itens[itens.length - 1] += ` ${ls[i].trim()}`;
        i++;
      }
      out.push(`<ul>${itens.map((t) => `<li>${linha(t, pre)}</li>`).join('')}</ul>`);
    } else if (/^>/.test(l)) {
      const b = bloco((x) => /^>/.test(x)).map((x) => x.replace(/^>\s?/, ''));
      out.push(`<blockquote>${markdown(b.join('\n'), pre)}</blockquote>`);
    } else if (/^\|/.test(l)) {
      const b = bloco((x) => /^\|/.test(x)).filter((x) => !/^\|[\s|:-]+\|$/.test(x));
      const cel = (x) => x.replace(/^\||\|$/g, '').split('|').map((c) => linha(c.trim(), pre));
      const [cab, ...corpo] = b;
      out.push(`<div class="tabela" role="region" tabindex="0"><table><thead><tr>${cel(cab).map((c) => `<th>${c}</th>`).join('')}</tr></thead><tbody>${
        corpo.map((r) => `<tr>${cel(r).map((c) => `<td>${c}</td>`).join('')}</tr>`).join('')}</tbody></table></div>`);
    } else {
      const b = bloco((x) => !/^(#{1,6}\s|>|\||\s*[-*]\s)/.test(x));
      out.push(`<p>${linha(b.map((x) => x.trim()).join(' '), pre)}</p>`);
    }
  }
  return out.join('\n');
}

const ler = (raiz, f) => readFileSync(path.join(raiz, f), 'utf8');
// sem o primeiro título (# …): a página põe o seu
const corpo = (t) => t.replace(/^\s*#\s[^\n]*\n/, '');

/** As duas páginas: { 'termos.html': …, 'privacidade.html': … } a partir do molde `site/src/documento.html`. */
export function paginas(raizRepo, molde) {
  const montar = (o) => molde.replace('{{titulo}}', esc(o.titulo)).replace('{{h1}}', esc(o.h1)).replace('{{descricao}}', esc(o.descricao))
    .replace('{{caminho}}', o.caminho).replace('{{conteudo}}', () => o.conteudo);
  const lic = ler(raizRepo, 'LICENSE');
  const licPt = ler(raizRepo, 'LICENCA.pt-BR.md');
  // as linhas "Required Notice:" ficam como estão no arquivo (a licença pede que acompanhem o software)
  const avisos = (t) => t.split('\n').filter((l) => l.startsWith('Required Notice:')).map((l) => `<p class="aviso-req">${esc(l)}</p>`).join('\n');
  const semAvisos = (t) => t.split('\n').filter((l) => !l.startsWith('Required Notice:')).join('\n');
  const termos = [
    '<section id="termos">', markdown(corpo(ler(raizRepo, 'docs/LICENCIAMENTO.md')), 'termos'), '</section>',
    '<section id="licenca-pt"><h2>Licença: tradução para o português do Brasil</h2>', avisos(licPt),
    markdown(semAvisos(licPt).replace(/^\s*#\s[^\n]*\n/m, ''), 'licenca-pt'), '</section>',
    '<section id="licenca-en" lang="en"><h2>License: official text (PolyForm Noncommercial 1.0.0)</h2>', avisos(lic),
    markdown(semAvisos(lic).replace(/^\s*#\s[^\n]*\n/m, ''), 'licenca-en'), '</section>',
  ].join('\n');
  return {
    'termos.html': montar({
      titulo: 'Termos de uso e licença · Urna Velox', h1: 'Termos de uso e licença', caminho: 'termos',
      descricao: 'Termos de uso e licença do Urna Velox: uso livre sem fins comerciais (PolyForm Noncommercial 1.0.0); uso comercial exige licença do autor.',
      conteudo: termos,
    }),
    'privacidade.html': montar({
      titulo: 'Privacidade e uso de dados · Urna Velox', h1: 'Privacidade, acesso e retenção de dados', caminho: 'privacidade',
      descricao: 'Privacidade, acesso e retenção de dados no Urna Velox: coletamos o mínimo, não vendemos nem compartilhamos informações pessoais dos visitantes.',
      conteudo: `<section id="privacidade">${markdown(corpo(ler(raizRepo, 'docs/PRIVACIDADE.md')), 'privacidade')}</section>`,
    }),
  };
}
