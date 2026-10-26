// Painel dos nós (aquisicao/src/painel/pagina) no celular: nenhuma aba pode alargar a página além da tela (B-020).
// Serve a página com uma API falsa (dois nós, valores longos como os de produção: auditoria, EXTRA, URLs de BU,
// federação) e confere, a 360, 390 e 430 px e no desktop, em cada aba e nos temas escuro e claro, que nenhum
// elemento passa da borda direita (fora de caixas com rolagem própria). Também confere o aviso de que o nó está no
// ciclo de simulados (a configuração mostrada é a do urna-velox.service, parado durante o ciclo).
//   node testes/painel_celular.mjs [--fotos /pasta]
import { chromium } from 'playwright';
import { createServer } from 'node:http';
import { readFileSync, mkdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const arg = (k) => { const i = process.argv.indexOf(`--${k}`); return i > 0 ? process.argv[i + 1] : null; };
const fotos = arg('fotos');
if (fotos) mkdirSync(fotos, { recursive: true });
const PAG = fileURLToPath(new URL('../../aquisicao/src/painel/pagina/', import.meta.url));
const FONTE = fileURLToPath(new URL('../dist/archivo.woff2', import.meta.url));
const tipos = { '.html': 'text/html; charset=utf-8', '.js': 'text/javascript', '.css': 'text/css', '.svg': 'image/svg+xml', '.woff2': 'font/woff2' };

const agora = Date.now();
const serie = () => {
  const t = [], cols = { bus_s: [], baixados_s: [], req_s: [], teto_req_s: [], rss_mb: [], sis_cpu_pct: [], ws_clientes: [] };
  for (let i = 60; i >= 0; i--) {
    t.push(agora - i * 5000);
    cols.bus_s.push(40 + (i % 7)); cols.baixados_s.push(38); cols.req_s.push(90); cols.teto_req_s.push(95);
    cols.rss_mb.push(490); cols.sis_cpu_pct.push(35); cols.ws_clientes.push(12);
  }
  return { t, ...cols };
};
const resumo = (id) => ({
  servico: { versao: '0.8.31', pleito: 452, base: 'http://127.0.0.1:18080', ativo_s: 13440, relogio_site: '06/10/2024 20:43:56',
    simulacao: { rotulo: '1º turno de 2024 (eleição de 06/10/2024) · em tempo real, no ritmo em que os BUs chegaram ao TSE' },
    secoes: { aplicadas: 448123, catalogo: 472683 }, ultimo_bu: { site: '06/10/2024 20:43:51' }, rede: { acesso: 'borda' }, estado: 'acompanhando' },
  sistema: { hostname: `uv-${id}`, cpus: 4, memoria_mb: { usada: 6100, total: 23900 }, processos: { 'uv-servico': { pico_rss_mb: 812 } } },
  federacao: { ativos: [{ no: 'no-a' }, { no: 'no-b' }], fora: [], recebidos: 210334, publicados: 223001, rps_cota: 47, rps_federacao: 95,
    participa: true, sincronia: { buscas_erro: 0 } },
  ultima: { valores: { bus_s: 41, baixados_s: 38, req_s: 90, teto_req_s: 95, http429: 0, fila: 3, pendentes: 120, cpu_pct: 22, rss_mb: 491,
    sis_cpu_pct: 35, carga1: 1.2, disco_livre_gb: 61.4, ws_clientes: 12, ws_msgs_s: 30, ws_kb_s: 41, idade_saude_s: 2 } },
  alertas: [{ tipo: 'atraso', quando: '2026-10-19T15:40:02-03:00', mensagem: 'lote 0000000000000000000000000000000000000000000000004f2a1c3e9b7d copiado com atraso de 12 s' }],
});
const NOS = ['no-a', 'no-b'];
const config = (id) => ({
  arquivo: '/etc/urna-velox/no.env', existe: true, modo_local: false, controle_manual: false,
  no_env: {
    campos: [
      { chave: 'PLEITO', rotulo: 'Pleito', valor: '3220', editavel: true, sensivel: true, ajuda: 'código do pleito (3221 = 2º turno de 2026)' },
      { chave: 'BASE', rotulo: 'Servidor de resultados', valor: 'https://resultados.tse.jus.br', editavel: true, sensivel: true,
        ajuda: 'de onde o nó coleta; o TSE pede a confirmação "CONFIRMO TSE"' },
      { chave: 'AMBIENTE', rotulo: 'Ambiente', valor: 'oficial', editavel: true, sensivel: true, tipo: { opcoes: ['oficial', 'simulado'] } },
      { chave: 'ACESSO', rotulo: 'Acesso ao servidor', valor: 'borda', editavel: true, sensivel: true, tipo: { opcoes: ['borda', 'interno'] },
        ajuda: 'borda = borda pública do TSE (limite de 100 req/s, sempre); interno = servidor espelho dos arquivos, fora da borda pública (teto abaixo); vazio = borda' },
      { chave: 'TETO_RPS_INTERNO', rotulo: 'Teto no modo espelho', valor: '', editavel: true, opcional: true, tipo: 'numero' },
      { chave: 'NO_ID', rotulo: 'Identidade do nó', valor: id, editavel: false },
      { chave: 'EXTRA', rotulo: 'Opções extras', valor: "'--aceitar-incompleto --federacao /etc/urna-velox/federacao.json --armazem /srv/urna-velox/armazem'", editavel: false },
    ],
    outros: { UFS: 'ac,al,am,ap,ba,ce,df,es,go,ma,mg,ms,mt,pa,pb,pe,pi,pr,rj,rn,ro,rr,rs,sc,se,sp,to,zz',
      SAIDA: '/srv/urna-velox/saida/oficial-ele2026-3220-sem-espacos-e-bem-comprido-para-nao-quebrar' },
  },
  servicos: ['urna-velox.service', 'uv-ciclo-no.service', 'uv-playlist.service', 'uv-painel.service', 'caddy.service',
    'urna-velox-atualizador.timer', 'chrony.service'].map((u) => ({ unidade: u, estado: u === 'urna-velox.service' ? 'inactive' : 'active' })),
  federacao: { nos: [{ id: 'no-a', orgao: 'orgao-a-com-nome-comprido', ip_saida: '203.0.113.10', teto_rps: 100 },
    { id: 'no-b', orgao: 'orgao-b-com-nome-comprido', ip_saida: '203.0.113.11', teto_rps: 100 }], teto_coleta_rps: 95 },
  auditoria: [{ quando: '2026-10-19T15:47:01-03:00', operador: 'operador@exemplo.org', acao: 'config',
    detalhe: { mudancas: { RPS: '95', UFS: 'ac,al,am,ap,ba,ce,df,es,go,ma,mg,ms,mt,pa,pb,pe,pi,pr,rj,rn,ro,rr,rs,sc,se,sp,to' }, reiniciar: false } }],
});
const MODELOS = { ele_c: '{base}/{amb}/comum/config/ele-c.json', cs: '{base}/{amb}/{ciclo}/arquivo-urna/{pleito}/config/{uf}/{uf}-p{pleito06}-cs.json',
  aux: '{base}/{amb}/{ciclo}/arquivo-urna/{pleito}/dados/{uf}/{mun05}/{zona04}/{secao04}/p{pleito06}-{uf}-m{mun05}-z{zona04}-s{secao04}-aux.json',
  arquivo: '{base_arquivos}/{amb}/{ciclo}/arquivo-urna/{pleito}/dados/{uf}/{mun05}/{zona04}/{secao04}/{hash}/{nome}',
  mun_cm: '{base}/{amb}/{ciclo}/{pleito}/config/mun-e{eleicao06}-cm.json', u_json: '{base}/{amb}/{ciclo}/{pleito}/dados/{uf}/{uf}-c{cargo04}-e{eleicao06}-u.json' };
const B = 'https://resultados.tse.jus.br/oficial/ele2026/arquivo-urna/3220';
const rotas = () => ({ base: 'https://resultados.tse.jus.br', ambiente: 'oficial', ciclo: 'ele2026', pleito: 3220, padrao: true, rotas: null,
  modelos_padrao: MODELOS, rotas_servico: null,
  exemplos: { ele_c: 'https://resultados.tse.jus.br/oficial/comum/config/ele-c.json', cs: `${B}/config/sp/sp-p003220-cs.json`,
    aux: `${B}/dados/sp/71072/0001/0001/p003220-sp-m71072-z0001-s0001-aux.json`,
    arquivo: `${B}/dados/sp/71072/0001/0001/395a6b2f4f4d2b6e6d4b4d5a7a4c6c6f5a3d3d/o00406-7107200010001.bu` } });

const api = (u) => {
  const p = u.pathname;
  if (p === '/painel/api/eu') return { email: 'operador@exemplo.org', csrf: 'x', no: 'no-a', painel: '0.8.35' };
  if (p === '/painel/api/visao') return { gerado_em: Date.now(), local: 'no-a', nos: NOS.map((id) => ({ id, url: `http://${id}:8095`, alcancavel: true, resumo: resumo(id), serie: serie(), intervalo_s: 5 })) };
  let m = p.match(/^\/painel\/api\/no\/([^/]+)\/(config|rotas|saude)$/);
  if (m) return m[2] === 'config' ? config(m[1]) : m[2] === 'rotas' ? rotas() : resumo(m[1]);
  return null;
};
const srv = createServer((q, r) => {
  const u = new URL(q.url, 'http://x');
  const j = api(u);
  if (j) { r.writeHead(200, { 'content-type': 'application/json' }); r.end(JSON.stringify(j)); return; }
  const f = u.pathname === '/painel/' ? 'index.html' : u.pathname.replace(/^\/painel\//, '');
  let corpo;
  try { corpo = f === 'archivo.woff2' ? readFileSync(FONTE) : readFileSync(path.join(PAG, f)); } catch { r.writeHead(404); r.end(); return; }
  r.writeHead(200, { 'content-type': tipos[path.extname(f)] || 'text/plain' }); r.end(corpo);
}).listen(0, '127.0.0.1');
await new Promise((ok) => srv.once('listening', ok));
const url = `http://127.0.0.1:${srv.address().port}/painel/`;

const nav = await chromium.launch();
let falhas = 0;
const conferir = (ok, msg) => { console.log(`${ok ? 'ok ' : 'FALHOU'} ${msg}`); if (!ok) falhas++; };

/// Elementos que passam da borda direita da tela (ignorando o que está dentro de caixas com rolagem própria).
const vazamentos = () => {
  const W = document.documentElement.clientWidth;
  const rola = (e) => { for (let p = e.parentElement; p && p !== document.body; p = p.parentElement) {
    const o = getComputedStyle(p).overflowX; if (o === 'auto' || o === 'scroll' || o === 'hidden') return true; } return false; };
  const fora = [];
  for (const e of document.querySelectorAll('body *')) {
    if (e.closest('[hidden]') || e.closest('.aurora')) continue;
    const r = e.getBoundingClientRect();
    if (!r.width || !r.height) continue;
    if (r.right > W + 1 && !rola(e)) fora.push(`${e.tagName.toLowerCase()}${e.className && typeof e.className === 'string' ? '.' + e.className.split(' ')[0] : ''} (${Math.round(r.right)} > ${W})`);
  }
  return { largura: document.documentElement.scrollWidth, W, fora: fora.slice(0, 6), n: fora.length };
};

for (const [nome, vp] of [['360', { width: 360, height: 780 }], ['390', { width: 390, height: 844 }], ['430', { width: 430, height: 932 }], ['desktop', { width: 1366, height: 900 }]]) {
  for (const tema of ['escuro', 'claro']) {
    const ctx = await nav.newContext({ viewport: vp, ...(nome === 'desktop' ? {} : { isMobile: true, hasTouch: true, deviceScaleFactor: 2 }) });
    const pg = await ctx.newPage();
    if (tema === 'claro') await pg.addInitScript(() => { try { localStorage.setItem('uv-tema', 'claro'); } catch { /* */ } });
    await pg.goto(url);
    await pg.waitForSelector('#nos article');
    for (const aba of ['visao', 'config', 'detalhe']) {
      await pg.click(`[data-aba="${aba}"]`);
      if (aba === 'config') {
        await pg.waitForSelector('.painel-cfg');
        await pg.waitForSelector('.enderecos');
        for (const d of await pg.$$('#config details')) await d.evaluate((e) => { e.open = true; });
      }
      if (aba === 'detalhe') await pg.waitForFunction(() => document.getElementById('detalhe').textContent.length > 50);
      await pg.waitForTimeout(150);
      const v = await pg.evaluate(vazamentos);
      conferir(v.n === 0, `${nome} px · ${tema} · ${aba}: nada passa da tela (${v.W} px)${v.n ? ` — ${v.n} elementos, ex.: ${v.fora.join(', ')}` : ''}`);
      if (fotos) await pg.screenshot({ path: `${fotos}/${nome}-${tema}-${aba}.png`, fullPage: true });
    }
    if (nome === '390' && tema === 'escuro') {
      await pg.click('[data-aba="config"]');
      await pg.waitForSelector('#config .painel-cfg');
      const pequenos = await pg.$$eval('#config input:not([type=checkbox]), #config select', (l) => l.filter((e) => parseFloat(getComputedStyle(e).fontSize) < 16).length);
      conferir(pequenos === 0, `390 px · config: campos com letra de 16 px ou mais (o Safari do iPhone não aproxima ao tocar) — ${pequenos} abaixo`);
      const txt = await pg.textContent('#config');
      conferir(/ciclo de replays/i.test(txt), '390 px · config: avisa que o nó está no ciclo de replays (uv-ciclo-no ativo, urna-velox parado)');
    }
    await ctx.close();
  }
}
await nav.close();
srv.close();
console.log(falhas ? `painel_celular: ${falhas} falha(s)` : 'painel_celular: tudo ok');
process.exit(falhas ? 1 : 0);
