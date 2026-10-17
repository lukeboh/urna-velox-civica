// Sonda de depuração: abre a página, espera e imprime o que o JS pedir (argumento: expressão). Fica fora do build.
import { chromium } from 'playwright';
const url = process.argv[2] || 'http://127.0.0.1:18080/';
const expr = process.argv[3] || 'document.title';
const foto = process.argv[4];
const b = await chromium.launch();
const p = await (await b.newContext({ viewport: { width: +(process.env.LARG || 1440), height: +(process.env.ALT || 900) } })).newPage();
p.on('console', (m) => console.log('console:', m.type(), m.text()));
p.on('pageerror', (e) => console.log('erro:', String(e)));
await p.goto(url);
await p.waitForTimeout(+(process.env.ESPERA || 4000));
console.log(JSON.stringify(await p.evaluate(expr), null, 1));
if (foto) await p.screenshot({ path: foto });
await b.close();
