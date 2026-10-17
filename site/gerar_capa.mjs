// Gera a capa do repositório (docs/img/capa.png, 1280×640: topo do README e prévia social do GitHub) a partir de
// docs/marca/capa/capa.html. A captura do celular (docs/marca/capa/celular_capa.png) é a docs/img/marca/ref_celular_escuro.png
// com o nome do candidato da legenda desfocado (identidade neutra, GUIA_MARCA §6).
//   cd site && node gerar_capa.mjs
import { chromium } from "playwright";
import path from "node:path";
import { pathToFileURL } from "node:url";

const raiz = path.resolve("..");
const navegador = await chromium.launch();
const pagina = await navegador.newPage({ viewport: { width: 1280, height: 640 } });
await pagina.goto(pathToFileURL(path.join(raiz, "docs", "marca", "capa", "capa.html")).href);
await pagina.evaluate(() => document.fonts.ready);
await pagina.screenshot({ path: path.join(raiz, "docs", "img", "capa.png") });
await navegador.close();
console.log("docs/img/capa.png");
