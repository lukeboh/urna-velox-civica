// Selo REPLAY, aviso do dia e Playlist de Replays (ciclo diário), com Playwright/Chromium:
//   node testes/playlist_pagina.mjs --url http://127.0.0.1:18088/ --saida /tmp/uv-playlist-fotos
// Espera um uv-servico com --simulacao (um replay) atrás do Caddy do ciclo (rota /playlist para o uv-playlist).
// Confere: o selo do topo diz REPLAY, o aviso abre sozinho na primeira visita do dia (e não na segunda), o selo abre o
// aviso e o link dele leva à playlist, a página mostra no ar / a seguir / opções, e um voto é registrado e pode ser
// trocado. Fotos em desktop e celular. (Os mesmos casos sem servidor nenhum: testes/modo_pagina.mjs.)
import { chromium } from 'playwright';
import { mkdirSync } from 'node:fs';

const arg = (k, d) => { const i = process.argv.indexOf(`--${k}`); return i > 0 ? process.argv[i + 1] : d; };
const url = arg('url', 'http://127.0.0.1:18088/');
const saida = arg('saida', '/tmp/uv-playlist-fotos');
mkdirSync(saida, { recursive: true });
const falhas = [];
const conferir = (ok, o) => { console.log(`${ok ? 'ok   ' : 'FALHA'} ${o}`); if (!ok) falhas.push(o); };

const nav = await chromium.launch();
for (const largura of [1366, 390, 360]) {
  const movel = largura < 640;
  const ctx = await nav.newContext(movel
    ? { viewport: { width: largura, height: 780 }, deviceScaleFactor: 2, isMobile: true, hasTouch: true, colorScheme: 'dark' }
    : { viewport: { width: largura, height: 900 }, colorScheme: 'light' });
  const p = await ctx.newPage();
  const tag = movel ? `celular-${largura}` : 'desktop';
  await p.goto(url);
  // M-004: o selo do topo diz REPLAY e é o acesso à playlist (antes: chip "Dados reais" + botão "▶ Playlist" na faixa)
  await p.waitForSelector('#conexao[data-modo="replay"]', { timeout: 20000 });
  const chip = await p.textContent('#conexao');
  conferir(/REPLAY/.test(chip), `${tag}: selo "${chip.slice(0, 60)}"`);
  await p.waitForSelector('#folha.aberta', { timeout: 5000 }).catch(() => null);
  await p.waitForTimeout(700);   // fim da transição da folha
  const caixa = await p.locator('#folha').boundingBox();
  conferir(caixa && caixa.y >= 0, `${tag}: o aviso cabe na tela (topo em ${Math.round(caixa?.y)} px)`);
  conferir(await p.isVisible('#folha.aberta'), `${tag}: aviso aberto na primeira visita`);
  conferir(/REPLAY/.test(await p.textContent('#folha-titulo')), `${tag}: título do aviso`);
  await p.screenshot({ path: `${saida}/apuracao-explicacao-${tag}.png` });
  await p.click('#folha-fechar');
  // B-018: selo, contagem da próxima eleição ao vivo e ritmo (BUs/min) inteiros, sem corte nem sumiço
  const larg = p.viewportSize().width;
  for (const [sel, nome] of [['#conexao', 'selo REPLAY'], ['.faixa .chip.promo', 'próxima eleição ao vivo'], ['.faixa .chip.ritmo', 'BUs/min']]) {
    const c = await p.locator(sel).boundingBox();
    const cortado = await p.locator(sel).evaluate((e) => e.scrollWidth > e.clientWidth + 1);
    conferir(!!c && c.width > 0 && c.x >= 16 && c.x + c.width <= larg - 16 && !cortado,   // com a margem lateral
      `${tag}: ${nome} inteiro à vista (${c ? `${Math.round(c.x)}–${Math.round(c.x + c.width)} de ${larg} px` : 'oculto'}${cortado ? ', texto cortado' : ''})`);
  }
  await p.screenshot({ path: `${saida}/apuracao-faixa-${tag}.png` });
  await p.reload();
  await p.waitForSelector('#conexao[data-modo="replay"]');
  await p.waitForTimeout(1500);
  conferir(!(await p.isVisible('#folha.aberta')), `${tag}: na segunda visita do dia o aviso não abre sozinho`);
  await p.click('#conexao');
  conferir(await p.isVisible('#folha.aberta'), `${tag}: o selo abre o aviso`);
  await p.click('#folha a[href="/playlist"]');
  await p.waitForSelector('#opcoes .op');
  conferir(new URL(p.url()).pathname === '/playlist', `${tag}: link leva a /playlist`);
  conferir((await p.locator('#opcoes .op').count()) === 6, `${tag}: seis opções votáveis`);
  conferir((await p.locator('#fila li').count()) >= 3, `${tag}: agenda com horários`);
  await p.screenshot({ path: `${saida}/playlist-${tag}.png`, fullPage: !movel });
  if (movel) { await p.locator('#opcoes').scrollIntoViewIfNeeded(); await p.screenshot({ path: `${saida}/playlist-opcoes-${tag}.png` }); }
  if (!movel) {
    const b = p.locator('#opcoes .op').filter({ hasText: '2º turno · 2024' }).filter({ hasText: 'acelerado' }).locator('button');
    await b.click();
    await p.waitForSelector('#opcoes .op.meu');
    conferir((await p.locator('#opcoes .op.meu').textContent()).includes('2º turno · 2024'), 'voto registrado no 2º turno de 2024, acelerado');
    const noAr = p.locator('#opcoes .op').filter({ hasText: '2º turno · 2024' }).filter({ hasText: 'Tempo real' }).locator('button');
    conferir(await noAr.isDisabled(), 'o replay no ar não pode ser votado');
    await p.screenshot({ path: `${saida}/playlist-votado-${tag}.png`, fullPage: true });
    await p.locator('#opcoes .op.meu button').click();
    await p.waitForFunction(() => !document.querySelector('#opcoes .op.meu'));
    conferir(true, 'voto retirado');
  }
  await ctx.close();
}
await nav.close();
console.log(falhas.length ? `${falhas.length} falha(s)` : 'tudo certo');
process.exit(falhas.length ? 1 : 0);
