#!/usr/bin/env node
/**
 * teste_combinado.mjs — Teste Combinado de Estresse:
 * 1. Saturação de WebSocket na OCI (16.000 conexões para testar o novo limite de 15.000 no cluster)
 * 2. Carga concorrente na CDN Cloudflare (dados.urnavelox.com.br a 500 req/s / 30.000 clientes)
 */

import { spawn } from 'node:child_process';
import { performance } from 'node:perf_hooks';

const TOPICOS = [
  'ac01007-c0011',
  'al27006-c0011',
  'al27014-c0011',
  'al27022-c0011',
  'to92452-c0011',
  'to92479-c0011'
];

async function checarSaude(url) {
  try {
    const res = await fetch(url, { signal: AbortSignal.timeout(3000) });
    if (res.ok) {
      const d = await res.json();
      return { ok: true, versao: d.versao, estado: d.estado, clientes: d.clientes };
    }
  } catch (e) {
    return { ok: false, erro: e.message };
  }
  return { ok: false };
}

async function rodarCdnHttp(duracaoS = 35, rps = 500) {
  console.log(`\n[CDN] Disparando tráfego HTTP contra a Cloudflare: ~${rps} req/s por ${duracaoS}s...`);
  const latencias = [];
  const statusCodes = {};
  const cacheStatus = {};
  let totalReqs = 0;
  let erros = 0;
  const fimEm = Date.now() + duracaoS * 1000;
  const intervaloMs = 1000 / rps;
  let ativas = 0;
  let proximoDisparo = performance.now();
  const etags = new Map();

  async function dispararUma() {
    totalReqs++;
    ativas++;
    const t0 = performance.now();
    const rota = Math.random() < 0.25 ? '/api/v1/estado' : `/api/v1/resultado/${TOPICOS[Math.floor(Math.random() * TOPICOS.length)]}`;
    const endpoint = `https://dados.urnavelox.com.br${rota}`;
    const headers = { 'User-Agent': 'UrnaVelox Teste Combinado' };
    const etagSalvo = etags.get(rota);
    if (etagSalvo) headers['If-None-Match'] = etagSalvo;

    try {
      const res = await fetch(endpoint, { headers, signal: AbortSignal.timeout(5000) });
      const lat = performance.now() - t0;
      latencias.push(lat);
      statusCodes[res.status] = (statusCodes[res.status] || 0) + 1;
      const cfStatus = res.headers.get('cf-cache-status') || 'SEM_CABECALHO';
      cacheStatus[cfStatus] = (cacheStatus[cfStatus] || 0) + 1;
      const novoEtag = res.headers.get('etag');
      if (novoEtag) etags.set(rota, novoEtag);
      await res.arrayBuffer();
    } catch (err) {
      erros++;
      statusCodes['ERRO'] = (statusCodes['ERRO'] || 0) + 1;
    } finally {
      ativas--;
    }
  }

  while (Date.now() < fimEm) {
    if (ativas < 250) dispararUma();
    proximoDisparo += intervaloMs;
    const espera = proximoDisparo - performance.now();
    if (espera > 1) await new Promise(r => setTimeout(r, Math.min(espera, 40)));
    else await new Promise(r => setImmediate(r));
  }

  while (ativas > 0) await new Promise(r => setTimeout(r, 100));

  latencias.sort((a, b) => a - b);
  const p50 = latencias[Math.floor(latencias.length * 0.5)] || 0;
  const p99 = latencias[Math.floor(latencias.length * 0.99)] || 0;
  const hits = (cacheStatus['HIT'] || 0) + (cacheStatus['REVALIDATED'] || 0);

  return {
    totalReqs,
    vazao: (totalReqs / duracaoS).toFixed(1),
    taxaHit: totalReqs ? ((hits / totalReqs) * 100).toFixed(1) : 0,
    p50: p50.toFixed(2),
    p99: p99.toFixed(2),
    statusCodes,
    cacheStatus
  };
}

async function main() {
  console.log('================================================================================');
  console.log('TESTE COMBINADO: WEBSOCKET SATURADO NA OCI + CDN CLOUDFLARE SIMULTÂNEA');
  console.log('Novo limite por nó: 7.500 conexões (Total Cluster OCI: 15.000 conexões)');
  console.log('Alvo WebSocket:     wss://urnavelox.com.br/ws (16.000 clientes)');
  console.log('Alvo CDN HTTP:      https://dados.urnavelox.com.br (~500 req/s / 30.000 virtuais)');
  console.log('================================================================================');

  const saudeIni = await checarSaude('https://urnavelox.com.br/api/v1/estado');
  console.log('\n[Origem OCI antes]:', JSON.stringify(saudeIni));

  console.log('\n[1/3] Iniciando rampa WebSocket com uv-carga (16.000 clientes)...');
  const cargaProc = spawn('./target/release/uv-carga', [
    '--endereco', 'https://urnavelox.com.br',
    '--clientes', '16000',
    '--rampa', '500',
    '--duracao', '45',
    '--perfil', 'misto',
    '--relatorio', '/tmp/uv-carga-relatorio.json'
  ], {
    cwd: '/mnt/d/git/lukeboh/urna-velox',
    stdio: 'inherit'
  });

  console.log('Aguardando 30s para o WebSocket subir as 16.000 conexões e saturar o cluster...');
  await new Promise(r => setTimeout(r, 30000));

  const saudePico = await checarSaude('https://urnavelox.com.br/api/v1/estado');
  console.log('[Origem OCI em carga WS]:', JSON.stringify(saudePico));

  console.log('\n[2/3] Disparando teste de carga HTTP concorrente na Cloudflare sob saturação da OCI...');
  const resCdn = await rodarCdnHttp(25, 600); // 600 req/s por 25s

  console.log('\n[3/3] Aguardando conclusão do uv-carga...');
  await new Promise(r => cargaProc.on('close', r));

  console.log('\n================================================================================');
  console.log('RESULTADO DA CARGA CDN CLOUDFLARE (DURANTE A SATURAÇÃO NA ORIGEM)');
  console.log('================================================================================');
  console.log(`  Requisições HTTP:  ${resCdn.totalReqs} (${resCdn.vazao} req/s)`);
  console.log(`  Taxa Cache HIT:    ${resCdn.taxaHit}%`);
  console.log(`  Latência Borda:    p50: ${resCdn.p50} ms | p99: ${resCdn.p99} ms`);
  console.log(`  Status HTTP:       ${JSON.stringify(resCdn.statusCodes)}`);
  console.log(`  Cache Cloudflare:  ${JSON.stringify(resCdn.cacheStatus)}`);

  console.log('\n================================================================================');
  console.log('RESULTADO DO WEBSOCKET (UV-CARGA NA OCI COM LIMITE 7.500/NÓ)');
  console.log('================================================================================');
  try {
    const fs = await import('node:fs/promises');
    const relatorio = await fs.readFile('/tmp/uv-carga-relatorio.json', 'utf8');
    console.log(relatorio);
  } catch (e) {
    console.log('Não foi possível ler /tmp/uv-carga-relatorio.json:', e.message);
  }

  const saudeFim = await checarSaude('https://urnavelox.com.br/api/v1/estado');
  console.log('\n[Origem OCI após teste combinado]:', JSON.stringify(saudeFim));
}

main().catch(console.error);
