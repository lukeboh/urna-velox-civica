// depuração do B-085: posição do número de seções acesas do mapa na consulta (node testes/sonda_mapa_num.mjs [pleito])
import * as pw from 'playwright';
const URL0 = process.env.UV_URL || 'https://urnavelox.com.br';
// UV_MOTOR=webkit: o motor do Safari
const nav = await pw[process.env.UV_MOTOR || 'chromium'].launch();
const ctx = await nav.newContext({ viewport: { width: 375, height: 812 }, deviceScaleFactor: 3, isMobile: process.env.UV_MOTOR !== 'firefox', hasTouch: true });
const pg = await ctx.newPage();
await pg.goto(`${URL0}/#/p/${process.argv[2] || '452'}`);
await pg.waitForFunction(() => window.__uv?.mapa?.carregado, null, { timeout: 30000 });
const medir = (r) => pg.evaluate((r) => {
  const n = document.querySelector('.mp-n'), c = n.closest('.vidro, section, .cartao') || n.parentElement;
  const v = n.querySelector('.vis'); const vis = [...v.children].filter((x) => !x.classList.contains('vazio')); const a = { left: Math.min(...vis.map((x) => x.getBoundingClientRect().left)), right: Math.max(...vis.map((x) => x.getBoundingClientRect().right)) }, b = c.getBoundingClientRect();
  return `${r}: número x=${a.left.toFixed(0)}..${a.right.toFixed(0)} cartão x=${b.left.toFixed(0)}..${b.right.toFixed(0)} vis.transform=${v?.style.transform} texto=${n.textContent.replace(/\s/g, '').slice(-12)}`;
}, r);
await pg.waitForTimeout(1500);
console.log(await medir('fim'));
await pg.evaluate(() => { const pl = window.__uv.player; pl.ir(pl.info.inicio); });
await pg.waitForTimeout(1500);
console.log(await medir('início'));
await pg.evaluate(() => window.__uv.player.velocidade(300));
await pg.waitForTimeout(6000);
await pg.evaluate(() => window.__uv.player.velocidade(0));
console.log(await medir('tocou 6 s'));
await pg.evaluate(() => { const pl = window.__uv.player; pl.ir(pl.info.fim); });
await pg.waitForTimeout(2000);
console.log(await medir('fim de novo'));
await pg.evaluate(() => { const pl = window.__uv.player; pl.ir(Math.round((pl.info.inicio + pl.info.fim) / 2)); });
await pg.waitForTimeout(2000);
console.log(await medir('meio'));
for (const rot of ['Exterior', 'Horário', 'Brasil', 'Luz']) {
  await pg.locator('.mapa button', { hasText: rot }).first().click().catch((e) => console.log('sem botão', rot));
  await pg.waitForTimeout(1200);
  console.log(await medir(`vista ${rot}`));
}
await pg.evaluate(() => { const pl = window.__uv.player; pl.ir(pl.info.inicio + 3 * 3600); pl.velocidade(180); });
for (let k = 0; k < 6; k++) { await pg.waitForTimeout(1500); console.log(await medir(`180× +${(k + 1) * 1.5}s`)); }
await pg.evaluate(() => window.__uv.player.velocidade(0));
await pg.waitForTimeout(1500);
console.log(await medir('pausado'));
await nav.close();
