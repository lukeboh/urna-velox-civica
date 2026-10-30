// Validação da 1.1 contra o site no ar (HANDOFF_1.1.0 §4; relatório em docs/relatorios/VALIDACAO_1.1_IMPLANTACAO_*.md):
// os testes que dependiam de "estar na tela na hora certa" passam a usar a consulta (slider) dos encerrados.
//   cd site && node testes/validacao_1_1.mjs        (UV_URL = outro endereço; precisa do Firefox do Playwright)
//   T7 — suplementar municipal (3226, Oiapoque/AP) consultável: abre no resultado final do município;
//   T8 — B-070 na tela (3237, Governador de RR): ninguém "Eleito"; selo "Sem eleito (art. 224)";
//   T9 — o "art. 224" do selo é link para a norma no TSE (fragmento de texto, aba nova), no Chromium e no Firefox;
//   T1 — o seletor de pleito (1º chip da faixa) mostra o pleito e abre a lista, a 1366 e 360 px.
import { chromium, firefox } from 'playwright';

const URL0 = process.env.UV_URL || 'https://urnavelox.com.br';
const falhas = [];
const conferir = (ok, o) => { console.log(`${ok ? 'ok   ' : 'FALHA'} ${o}`); if (!ok) falhas.push(o); };
const esperar = (pg, f, arg, ms = 30000) => pg.waitForFunction(f, arg, { timeout: ms }).then(() => true).catch(() => false);

for (const [nome, motor] of [['chromium', chromium], ['firefox', firefox]]) {
  const nav = await motor.launch();
  const ctx = await nav.newContext({ viewport: { width: 1366, height: 900 }, timezoneId: 'America/Sao_Paulo', locale: 'pt-BR' });
  const pg = await ctx.newPage();
  pg.on('pageerror', (e) => conferir(false, `${nome}: erro na página: ${e.message}`));
  // T8 e T9: 3237 no fim da noite
  await pg.goto(`${URL0}/#/p/3237`);
  // no fim da noite: a publicação final (estado sem_eleito)
  const abriu = await esperar(pg, () => window.__uv?.consulta && window.__uv.res?.get('rr-c0003')?.estado === 'sem_eleito');
  conferir(abriu, `${nome} T8: consulta do 3237 abre`);
  if (abriu) {
    // a situação de cada candidato (o campo do motor, como no -u.json: "st"/"situacao") e o texto da tela
    const sit = await pg.evaluate(() => [...new Set((JSON.stringify(window.__uv.res.get('rr-c0003')).match(/"(st|situacao|sit)":"[^"]*"/g) || []))]);
    const txt = await pg.evaluate(() => document.querySelector('main')?.innerText || document.body.innerText);
    conferir(sit.length > 0 && !sit.some((s) => /:"eleito"$/i.test(s)), `${nome} T8: ninguém "eleito" no tópico (${sit.join(' ')})`);
    const el = txt.match(/[^\n]*\bEleit[oa]\b[^\n]*/g) || [];
    conferir(el.length === 0, `${nome} T8: nenhum "Eleito" na tela${el.length ? ` (${el.slice(0, 3).join(' | ')})` : ''}`);
    const viu = await pg.waitForSelector('.selo.ligado a.norma', { timeout: 15000 }).then(() => true).catch(() => false);
    conferir(viu, `${nome} T8: selo com o link do artigo`);
    if (viu) {
      const a = await pg.evaluate(() => { const x = document.querySelector('.selo a.norma'); const s = x.closest('.selo');
        return { txt: s.textContent, link: x.textContent, href: x.href, target: x.target, rel: x.rel }; });
      conferir(a.txt === 'Sem eleito (art. 224)', `${nome} T8: selo "${a.txt}"`);
      conferir(/^https:\/\/www\.tse\.jus\.br\//.test(a.href) && a.href.includes('#:~:text='), `${nome} T9: href na norma do TSE com fragmento (${a.href.slice(0, 90)}…)`);
      conferir(a.target === '_blank' && /noopener/.test(a.rel), `${nome} T9: aba nova (target ${a.target}, rel "${a.rel}")`);
      const [pop] = await Promise.all([ctx.waitForEvent('page', { timeout: 8000 }).catch(() => null), pg.click('.selo a.norma').catch(() => null)]);
      conferir(!!pop, `${nome} T9: o clique abre outra aba`);
      if (pop) {
        await pop.waitForLoadState('domcontentloaded', { timeout: 20000 }).catch(() => {});
        const u = pop.url();
        conferir(/tse\.jus\.br/.test(u), `${nome} T9: a aba nova é a norma no TSE (${u.slice(0, 80)})`);
        const tem = await pop.evaluate(() => /Art\.\s*224/.test(document.body?.innerText || '')).catch(() => false);
        conferir(tem, `${nome} T9: a norma aberta tem o art. 224`);
        await pop.close().catch(() => {});
      }
      conferir(pg.url().startsWith(URL0), `${nome} T9: a apuração continua na aba dela`);
    }
  }
  if (nome === 'chromium') {
    // T7: suplementar municipal
    await pg.goto('about:blank');
    await pg.goto(`${URL0}/#/p/3226`);
    const ok7 = await esperar(pg, () => window.__uv?.consulta && [...window.__uv.res].some(([, r]) => r?.secoes?.apuradas > 0));
    const tela = await pg.evaluate(() => window.__uv?.tela?.topico);
    const ap = await pg.evaluate(() => { const r = window.__uv?.res?.get(window.__uv.tela?.topico); return r ? `${r.secoes?.apuradas}/${r.secoes?.total}` : '—'; });
    conferir(ok7 && /^ap/.test(tela || ''), `${nome} T7: 3226 (Oiapoque) consultável, tela ${tela}, seções ${ap}`);
    // T1: seletor de pleito
    for (const w of [1366, 360]) {
      await pg.setViewportSize({ width: w, height: w < 640 ? 760 : 900 });
      await pg.goto('about:blank');
      await pg.goto(`${URL0}/`);
      // M-025: o seletor de pleito é a eleição da barra (#eleicao), com o nome e a data do pleito
      const sel = await pg.waitForSelector('#eleicao .el-pleito', { timeout: 20000 }).then(() => true).catch(() => false);
      conferir(sel, `${nome} T1 ${w}px: seletor de pleito na barra da eleição`);
      if (sel) {
        const id = await esperar(pg, () => /\d\d\/\d\d\/20\d\d/.test(document.querySelector('#eleicao .el-data')?.textContent || '') && /Elei/.test(document.querySelector('#eleicao .el-nome')?.textContent || ''));
        conferir(id, `${nome} T1 ${w}px: a barra diz a eleição e a data (${await pg.evaluate(() => document.querySelector('#eleicao .el-pleito')?.title)})`);
        await pg.click('#eleicao .el-pleito');
        conferir(await pg.waitForSelector('#pleitos [data-pleito]', { timeout: 10000 }).then(() => true).catch(() => false), `${nome} T1 ${w}px: abre a lista dos pleitos`);
        const larg = await pg.evaluate(() => document.querySelector('#faixa').scrollWidth <= window.innerWidth + 1);
        conferir(larg, `${nome} T1 ${w}px: a faixa cabe na tela`);
      }
    }
  }
  await ctx.close();
  await nav.close();
}
console.log(falhas.length ? `\n${falhas.length} falha(s)` : '\ntodos os cenários passaram');
process.exit(falhas.length ? 1 : 0);
