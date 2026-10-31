// Cores dos candidatos a partir da cor do partido.
//
// Fonte das cores: Wikipédia em inglês, "Module:Political party/<letra>" (as cores usadas nos mapas e infoboxes
// eleitorais), consultada em 14/10/2026. A cor do partido dá só o MATIZ; luminosidade e croma são normalizados em
// OKLCH para cada tema (mesmo "peso" visual de todos os candidatos e contraste com o vidro), e candidatos da mesma
// disputa com matizes parecidos (vários vermelhos, vários azuis) recebem tons diferentes da própria cor — mais claro,
// mais escuro — em vez de cores trocadas. Partido sem cor conhecida: a da federação; sem federação: paleta neutra.
// O resultado é um `light-dark(claro, escuro)`, resolvido pelo `color-scheme` do tema, então a troca de tema não
// precisa recalcular nada.

/** sigla normalizada (maiúsculas, sem acento, sem espaço e sem "*") → cor de referência do partido */
export const PARTIDOS = {
  PL: '#015AAA', PT: '#E20E28', UNIAO: '#2FBEF2', PP: '#203F71', PSD: '#FFA500', REPUBLICANOS: '#0070C5',
  MDB: '#30914D', PSB: '#FFCC00', PODE: '#2DA933', PSDB: '#0080FF', PDT: '#C21E56', PSOL: '#E2202A',
  PCDOB: '#820000', AVANTE: '#088F8F', NOVO: '#F3701B', SOLIDARIEDADE: '#FF9C2B', PRD: '#008000', PV: '#006600',
  CIDADANIA: '#EC008C', REDE: '#379E8D', MISSAO: '#FDBF35', PRTB: '#2CB53F', AGIR: '#025B97', MOBILIZA: '#EC2300',
  DC: '#015F9D', PCO: '#BB0000', PCB: '#FF0000', PSTU: '#C4122D', UP: '#000000', DEMOCRATA: '#003366', PMB: '#333C61',
};

/** federações em vigor (TSE): partido → cor da federação (a do partido que a encabeça), usada só na falta da própria */
const FEDERACOES = [
  { nome: 'Brasil da Esperança', cor: '#E20E28', partidos: ['PT', 'PCDOB', 'PV'] },
  { nome: 'PSDB Cidadania', cor: '#0080FF', partidos: ['PSDB', 'CIDADANIA'] },
  { nome: 'PSOL REDE', cor: '#E2202A', partidos: ['PSOL', 'REDE'] },
  { nome: 'Renovação Solidária', cor: '#FF9C2B', partidos: ['SOLIDARIEDADE', 'PRD'] },
  { nome: 'União Progressista', cor: '#2FBEF2', partidos: ['UNIAO', 'PP'] },
];

// matizes de reserva (os da paleta anterior) para partido desconhecido
const RESERVA = [255, 0, 80, 165, 295, 45, 215, 105, 135, 330, 60, 230, 30, 120];

export function normalizarSigla(s) {
  return String(s || '').normalize('NFD').replace(/[̀-ͯ]/g, '').replace(/[\s*]/g, '').toUpperCase();
}

/** #rrggbb → [L, C, h] em OKLCH */
export function oklch(hex) {
  const n = Number.parseInt(hex.slice(1), 16);
  const lin = (v) => { v /= 255; return v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4; };
  const r = lin((n >> 16) & 255), g = lin((n >> 8) & 255), b = lin(n & 255);
  const l = Math.cbrt(0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b);
  const m = Math.cbrt(0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b);
  const s = Math.cbrt(0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b);
  const L = 0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s;
  const A = 1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s;
  const B = 0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s;
  const C = Math.hypot(A, B);
  return [L, C, ((Math.atan2(B, A) * 180) / Math.PI + 360) % 360];
}

function base(sigla, i) {
  const sg = normalizarSigla(sigla);
  const hex = PARTIDOS[sg] ?? FEDERACOES.find((f) => f.partidos.includes(sg))?.cor;
  if (!hex) return { h: RESERVA[i % RESERVA.length], cromatica: true, chave: `?${i}` };
  const [, C, h] = oklch(hex);
  // preto/cinza (UP): sem matiz — vira grafite levemente azulado, da família do fundo
  return C < 0.04 ? { h: 265, cromatica: false, chave: sg } : { h, cromatica: true, chave: sg };
}

// tons alternativos da própria cor: [ΔL, Δmatiz°]. O giro de matiz é limitado (o verde continua verde, o vermelho
// continua vermelho); a luminosidade é o que mais distingue. B-012: a escolha deixou de ser "o primeiro tom a 0,075 dos
// já escolhidos" (dois verdes de SP, MDB e PRTB, ficavam a 0,05) e passou a maximizar a menor distância da disputa.
const DL = [0, 0.07, -0.07, 0.14, -0.14];
const DH = [0, 12, -12, 24, -24];
// em ordem de preferência: quanto menos mexe na cor do partido, melhor
const OPCOES = DL.flatMap((dl) => DH.map((dh) => [dl, dh])).sort((a, b) => (Math.abs(a[0]) / 0.07 + Math.abs(a[1]) / 12) - (Math.abs(b[0]) / 0.07 + Math.abs(b[1]) / 12));
const ALVO = 0.14; // distância em OKLab que já basta (nos dois temas); acima dela, fica o tom mais perto da cor do partido
const f = (x, n = 3) => +x.toFixed(n);
const lab = ([L, C, h]) => [L, C * Math.cos((h * Math.PI) / 180), C * Math.sin((h * Math.PI) / 180)];
const de = (a, b) => Math.hypot(a[0] - b[0], a[1] - b[1], a[2] - b[2]);

function tons(b, [dL, dh]) {
  const h = (b.h + dh + 360) % 360;
  // escuro: claro o bastante para o vidro escuro; claro: escuro o bastante para o vidro claro (texto não usa estas cores)
  const escuro = b.cromatica ? [Math.min(0.88, Math.max(0.62, 0.76 + dL)), 0.14, h] : [Math.min(0.9, Math.max(0.62, 0.8 + dL)), 0.015, h];
  const claro = b.cromatica ? [Math.min(0.72, Math.max(0.42, 0.6 - dL)), 0.15, h] : [Math.min(0.62, Math.max(0.3, 0.45 - dL)), 0.02, h];
  return { escuro, claro };
}
const css = ([L, C, h]) => `oklch(${f(L)} ${f(C)} ${f(h, 1)})`;

/**
 * Cores de uma disputa. `cands` = [{ n, partido }] na ordem do número (estável a noite toda: a cor de um candidato não
 * muda quando ele troca de posição; depende só de quem está na disputa). Cada um parte da própria cor e, em rodadas,
 * troca para o tom que mais o afasta do candidato mais próximo (nos dois temas) até todos ficarem a `ALVO` dos outros ou
 * nada mais melhorar; com folga, fica o tom mais perto da cor do partido. Devolve Map n → `light-dark(<claro>, <escuro>)`.
 */
export function coresDaDisputa(cands) {
  const bases = cands.map((k, i) => base(k.partido, i));
  const tabela = bases.map((b) => OPCOES.map((o) => { const t = tons(b, o); return { t, e: lab(t.escuro), c: lab(t.claro) }; }));
  const esc = cands.map(() => 0);
  const dist = (i, oi, j) => { const a = tabela[i][oi], b = tabela[j][esc[j]]; return Math.min(de(a.e, b.e), de(a.c, b.c)); };
  const menor = (i, oi) => { let m = Infinity; for (let j = 0; j < cands.length; j++) if (j !== i) m = Math.min(m, dist(i, oi, j)); return m; };
  for (let rodada = 0; rodada < 8; rodada++) {
    let mudou = false;
    for (let i = 0; i < cands.length; i++) {
      let melhor = esc[i], vm = Math.min(ALVO, menor(i, esc[i]));
      for (let oi = 0; oi < OPCOES.length; oi++) {
        const v = Math.min(ALVO, menor(i, oi));
        // melhora a distância (com margem, para não oscilar) ou empata com um tom mais perto da cor do partido
        if (v > vm + 1e-4 || (Math.abs(v - vm) <= 1e-4 && oi < melhor)) { melhor = oi; vm = v; }
      }
      if (melhor !== esc[i]) { esc[i] = melhor; mudou = true; }
    }
    if (!mudou) break;
  }
  const saida = new Map();
  cands.forEach((k, i) => { const t = tabela[i][esc[i]].t; saida.set(k.n, `light-dark(${css(t.claro)}, ${css(t.escuro)})`); });
  return saida;
}
