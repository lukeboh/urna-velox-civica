// Teste do B-048 (docs/BUGS.md): a página da playlist (`deploy/oci/servidor/uv-playlist.html`, servida pelo
// uv-playlist.py) com a mesma identidade visual da página da apuração (`site/dist/`): mesmo tema padrão (escuro, sem
// seguir o do sistema, como o app) e o claro só pela escolha guardada em "uv-tema", as mesmas cores do tema, a mesma
// fonte, o mesmo fundo (aurora), a mesma marca "Urna Velox", os mesmos cartões de vidro e títulos na largura da Archivo
// do app. Com Playwright/Chromium e **nenhum servidor** (as duas páginas e a API falsa servidas por rotas).
//   cd site && node testes/playlist_identidade.mjs [--saida /tmp/uv-playlist-identidade]
import { chromium } from 'playwright';
import { mkdirSync, readFileSync, existsSync } from 'node:fs';
import { dirname, join, extname } from 'node:path';
import { fileURLToPath } from 'node:url';

const arg = (k, d) => { const i = process.argv.indexOf(`--${k}`); return i > 0 ? process.argv[i + 1] : d; };
const saida = arg('saida', '/tmp/uv-playlist-identidade');
mkdirSync(saida, { recursive: true });
const AQUI = dirname(fileURLToPath(import.meta.url));
const DIST = join(AQUI, '..', 'dist');
const PLAYLIST = readFileSync(join(AQUI, '..', '..', 'deploy', 'oci', 'servidor', 'uv-playlist.html'), 'utf8');
const INDEX = readFileSync(join(DIST, 'index.html'), 'utf8').replaceAll('{{versao}}', 'teste');
const TIPOS = { '.js': 'text/javascript', '.css': 'text/css', '.svg': 'image/svg+xml', '.woff2': 'font/woff2' };
const falhas = [];
const conferir = (ok, o) => { console.log(`${ok ? 'ok   ' : 'FALHA'} ${o}`); if (!ok) falhas.push(o); };

const PL = { agora: '2026-10-22T15:00:00-03:00', suspensao: null, proxima_suspensao: null,
  proxima_ao_vivo: { eleicao: '2026-10-25', nome: '2º turno das Eleições Gerais 2026', inicio: '2026-10-25T17:00:00-03:00', fim: '2026-10-26T08:00:00-03:00', ao_vivo: false },
  no_ar: { item: '452-cadencia', modo: 'cadencia', titulo: '1º turno de 2024', eleicao: 'Eleições Municipais 2024', data_eleicao: '2024-10-06', inicio: '2026-10-22T14:00:00-03:00', previsao_fim: '2026-10-22T18:00:00-03:00', encerrada: false },
  fila: [{ item: '3220-rapido', modo: 'rapido', titulo: '1º turno de 2026', eleicao: 'Eleições Gerais 2026', data_eleicao: '2026-10-04', inicio_previsto: '2026-10-22T18:00:00-03:00', fim_previsto: '2026-10-22T18:10:00-03:00', origem: 'votos', votos: 3 }],
  opcoes: [{ item: '3220-rapido', titulo: '1º turno de 2026', eleicao: 'Eleições Gerais 2026', data_eleicao: '2026-10-04', votos: 3, modo: 'rapido', modo_descricao: 'Mesma ordem, acelerado.', duracao_estimada_s: 600 }],
  meu_voto: null, total_votos: 3, tocados: [] };

const nav = await chromium.launch();
/** Abre uma das páginas; `tema` = null (nada guardado) | 'claro' | 'escuro'; `sistema` = esquema de cores do sistema. */
async function abrir(caminho, { largura, tema, sistema }) {
  const movel = largura < 640;
  const ctx = await nav.newContext({ viewport: { width: largura, height: 900 }, deviceScaleFactor: movel ? 2 : 1, isMobile: movel, colorScheme: sistema });
  await ctx.addInitScript((t) => { try { if (t) localStorage.setItem('uv-tema', t); localStorage.setItem('uv-aviso-dia', '2099-01-01'); } catch { /* sem armazenamento */ } }, tema);
  await ctx.route('http://uv.teste/**', (r) => {
    const p = new URL(r.request().url()).pathname;
    if (p === '/') return r.fulfill({ status: 200, contentType: 'text/html; charset=utf-8', body: INDEX });
    if (p === '/playlist') return r.fulfill({ status: 200, contentType: 'text/html; charset=utf-8', body: PLAYLIST });
    if (p.startsWith('/static/')) {
      const f = join(DIST, p.slice(8));
      if (existsSync(f)) return r.fulfill({ status: 200, contentType: TIPOS[extname(f)] || 'application/octet-stream', body: readFileSync(f) });
    }
    if (p === '/api/v1/playlist') return r.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(PL) });
    if (p === '/api/v1/nomes') return r.fulfill({ status: 200, contentType: 'application/json', body: 'null' });
    if (p === '/api/v1/topicos') return r.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
    return r.fulfill({ status: 404, contentType: 'application/json', body: '{}' });
  });
  await ctx.routeWebSocket('ws://uv.teste/ws', () => { /* sem apuração: só a moldura da página */ });
  const p = await ctx.newPage();
  await p.goto(`http://uv.teste${caminho}`);
  await p.waitForTimeout(1200);
  await p.evaluate(() => document.fonts.ready);
  return { ctx, p };
}

const TOKENS = ['--fundo', '--fundo-2', '--aurora-1', '--aurora-2', '--aurora-3', '--vidro', '--vidro-forte', '--borda', '--texto', '--texto-2',
  '--texto-3', '--ok', '--alerta', '--sombra', '--raio-g', '--raio-m', '--raio-p'];
function medir(tokens) {
  const cs = (e, ps) => { if (!e) return null; const c = getComputedStyle(e); return Object.fromEntries(ps.map((k) => [k, c.getPropertyValue(k)])); };
  const raiz = getComputedStyle(document.documentElement);
  const tit = document.querySelector('.heroi h1, .corrida h1, h1');
  return {
    tokens: Object.fromEntries(tokens.map((t) => [t, raiz.getPropertyValue(t).trim()])),
    fundo: getComputedStyle(document.body).backgroundColor,
    corpo: cs(document.body, ['font-family', 'color', 'line-height']),
    aurora: cs(document.querySelector('.aurora'), ['background-image', 'position', 'z-index']),
    marca: cs(document.querySelector('.marca'), ['font-family', 'font-stretch', 'font-weight', 'font-size', 'letter-spacing', 'color']),
    velox: cs(document.querySelector('.marca span'), ['font-weight', 'color', 'font-stretch', 'background-image']),
    vidro: cs(document.querySelector('.vidro'), ['border-top-left-radius', 'border-top-color', 'background-color', 'box-shadow']),
    titulo: cs(tit, ['font-stretch', 'font-weight']),
    tema: document.documentElement.dataset.tema || '',
    botaoTema: !!document.querySelector('#tema'),
    largura: [document.documentElement.scrollWidth, document.documentElement.clientWidth],
  };
}

for (const largura of [1366, 390, 360]) for (const [tema, sistema] of [[null, 'light'], [null, 'dark'], ['claro', 'dark'], ['escuro', 'light']]) {
  const tag = `${largura} px · tema guardado ${tema || '(nenhum)'} · sistema ${sistema}`;
  const app = await abrir('/', { largura, tema, sistema });
  const a = await app.p.evaluate(medir, TOKENS);
  await app.ctx.close();
  const pl = await abrir('/playlist', { largura, tema, sistema });
  const b = await pl.p.evaluate(medir, TOKENS);
  if (largura === 390 || !tema) await pl.p.screenshot({ path: `${saida}/playlist-${largura}-${tema || 'nenhum'}-${sistema}.png`, fullPage: true });
  await pl.ctx.close();
  conferir(a.fundo === b.fundo, `${tag}: fundo da página igual ao do app (app ${a.fundo}, playlist ${b.fundo})`);
  const dif = TOKENS.filter((t) => a.tokens[t] !== b.tokens[t]);
  conferir(!dif.length, `${tag}: cores e raios do tema iguais aos do app${dif.length ? ` (diferem: ${dif.map((t) => `${t} app "${a.tokens[t]}" × playlist "${b.tokens[t]}"`).join('; ')})` : ''}`);
  conferir(/^"?Archivo UV"?/.test(b.corpo['font-family']) && a.corpo.color === b.corpo.color, `${tag}: fonte e cor do texto iguais (${b.corpo['font-family']}, ${b.corpo.color} × ${a.corpo.color})`);
  conferir(a.aurora['background-image'] === b.aurora['background-image'], `${tag}: o mesmo fundo (aurora) do app`);
  for (const k of ['font-stretch', 'font-weight', 'font-size', 'letter-spacing', 'color']) conferir(a.marca[k] === b.marca[k], `${tag}: marca "Urna Velox" — ${k} igual (app ${a.marca[k]}, playlist ${b.marca[k]})`);
  for (const k of ['font-weight', 'color', 'background-image']) conferir(a.velox[k] === b.velox[k], `${tag}: "Velox" da marca — ${k} igual (app ${a.velox[k]}, playlist ${b.velox[k]})`);
  // cor e borda: as do primeiro vidro do app (o letreiro, sem apuração na tela); o canto: o dos cartões do app (--raio-g;
  // o letreiro tem canto próprio)
  for (const k of ['border-top-color', 'background-color']) conferir(a.vidro[k] === b.vidro[k], `${tag}: cartão de vidro — ${k} igual (app ${a.vidro[k]}, playlist ${b.vidro[k]})`);
  conferir(b.vidro['border-top-left-radius'] === a.tokens['--raio-g'], `${tag}: cartão de vidro — canto dos cartões do app (${a.tokens['--raio-g']}; playlist ${b.vidro['border-top-left-radius']})`);
  conferir(parseFloat(b.titulo['font-stretch']) >= 110, `${tag}: título na largura da marca do app (font-stretch ${b.titulo['font-stretch']}; app ${a.titulo?.['font-stretch'] ?? '—'})`);
  conferir(b.botaoTema, `${tag}: botão de tema ◐, como no app`);
  conferir(b.largura[0] <= b.largura[1], `${tag}: a playlist não alarga a página (${b.largura[0]} > ${b.largura[1]} px)`);
}

await nav.close();
console.log(falhas.length ? `playlist_identidade: ${falhas.length} falha(s)` : 'playlist_identidade: tudo certo');
process.exit(falhas.length ? 1 : 0);
