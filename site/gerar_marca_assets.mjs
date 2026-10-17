import { chromium } from "playwright";
import fs from "node:fs";
import path from "node:path";

const dirMarca = path.resolve("..", "docs", "marca");
if (!fs.existsSync(dirMarca)) {
  fs.mkdirSync(dirMarca, { recursive: true });
}

// SVG do Símbolo Padrão em viewBox 128x128
function getSvgSimbolo(tamanho, options = {}) {
  const { maskable = false, raio = 32, fundo = "#0d1431" } = options;
  
  if (maskable) {
    // 512 maskable: fundo sangrando até a borda, símbolo dentro de 80% (zona de 409px)
    return `
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 128 128" width="${tamanho}" height="${tamanho}">
        <defs>
          <linearGradient id="g" x1="19" y1="24" x2="109" y2="104" gradientUnits="userSpaceOnUse">
            <stop offset="0%" stop-color="#4fc3ff"/>
            <stop offset="50%" stop-color="#8c6bff"/>
            <stop offset="100%" stop-color="#ff5fd2"/>
          </linearGradient>
        </defs>
        <rect width="128" height="128" fill="${fundo}"/>
        <g transform="translate(14, 14) scale(0.78125)">
          <path d="
            M 19 24
            L 51 24
            L 51 66
            C 51 78 59 86 69 86
            C 79 86 87 78 87 66
            L 87 44
            L 77 36
            L 109 36
            L 109 66
            C 109 92 91 104 69 104
            C 47 104 29 92 29 66
            L 29 32
            Z
          " fill="url(#g)"/>
        </g>
      </svg>
    `;
  }

  // Padrão com bordas arredondadas e filete de luz
  return `
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 128 128" width="${tamanho}" height="${tamanho}">
      <defs>
        <linearGradient id="g" x1="19" y1="24" x2="109" y2="104" gradientUnits="userSpaceOnUse">
          <stop offset="0%" stop-color="#4fc3ff"/>
          <stop offset="50%" stop-color="#8c6bff"/>
          <stop offset="100%" stop-color="#ff5fd2"/>
        </linearGradient>
      </defs>
      <rect x="4" y="4" width="120" height="120" rx="${raio}" fill="${fundo}"/>
      <rect x="4" y="4" width="120" height="120" rx="${raio}" fill="none" stroke="rgba(255,255,255,0.15)" stroke-width="1.5"/>
      <line x1="36" y1="4.5" x2="92" y2="4.5" stroke="rgba(255,255,255,0.32)" stroke-width="1" stroke-linecap="round"/>
      <path d="
        M 19 24
        L 51 24
        L 51 66
        C 51 78 59 86 69 86
        C 79 86 87 78 87 66
        L 87 44
        L 77 36
        L 109 36
        L 109 66
        C 109 92 91 104 69 104
        C 47 104 29 92 29 66
        L 29 32
        Z
      " fill="url(#g)"/>
    </svg>
  `;
}

async function gerarImagens() {
  console.log("Iniciando navegador Chromium via Playwright...");
  const chromePath = "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe";
  const browser = await chromium.launch({ executablePath: chromePath });
  const page = await browser.newPage();

  const tamanhos = [
    { nome: "icone_1024.png", px: 1024, raio: 32 },
    { nome: "icone_512.png", px: 512, raio: 32 },
    { nome: "icone_512_maskable.png", px: 512, maskable: true },
    { nome: "icone_192.png", px: 192, raio: 32 },
    { nome: "icone_180.png", px: 180, raio: 30 }, // Apple Touch Icon
    { nome: "icone_48.png", px: 48, raio: 12 },
    { nome: "icone_32.png", px: 32, raio: 8 },
    { nome: "icone_16.png", px: 16, raio: 4 },
  ];

  for (const item of tamanhos) {
    const destino = path.join(dirMarca, item.nome);
    if (fs.existsSync(destino)) {
      console.log(`Já gerado: ${destino}`);
      continue;
    }
    const svg = getSvgSimbolo(item.px, item);
    const html = `
      <!doctype html>
      <html>
      <head>
        <style>
          * { margin: 0; padding: 0; }
          body { background: transparent; overflow: hidden; }
          #icon { display: inline-block; width: ${item.px}px; height: ${item.px}px; }
        </style>
      </head>
      <body><div id="icon">${svg}</div></body>
      </html>
    `;
    await page.setViewportSize({ width: Math.max(item.px, 256), height: Math.max(item.px, 256) });
    await page.setContent(html);

    await page.locator("#icon").screenshot({ path: destino, omitBackground: true });
    console.log(`Gerado: ${destino} (${item.px}x${item.px})`);
  }

  await browser.close();
  console.log("Todos os PNGs gerados com sucesso!");
}

gerarImagens().catch(err => {
  console.error("Erro ao gerar imagens:", err);
  process.exit(1);
});
