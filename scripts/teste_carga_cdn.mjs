#!/usr/bin/env node
/**
 * teste_carga_cdn.mjs — Teste de Carga e Escala na CDN Cloudflare (dados.urnavelox.com.br)
 * 
 * Simula usuários simultâneos no modo HTTP do Urna Velox consumindo a CDN Cloudflare:
 * - Consultas de /api/v1/estado e /api/v1/resultado/{topico}
 * - Envio de ETag / If-None-Match
 * - Medição de latência (p50/p95/p99/máx), taxa de HIT vs MISS na Cloudflare, status HTTP e impacto na OCI.
 */

import { performance } from 'node:perf_hooks';

const TOPICOS = [
  'ac01007-c0011',
  'al27006-c0011',
  'al27014-c0011',
  'al27022-c0011',
  'al27030-c0011',
  'to92452-c0011',
  'to92479-c0011',
  'to92487-c0011'
];

function parseArgs() {
  const args = process.argv.slice(2);
  const cfg = {
    url: 'https://dados.urnavelox.com.br',
    origemUrl: 'https://urnavelox.com.br/api/v1/estado',
    degraus: [5000, 10000, 20000, 30000],
    duracaoS: 30,
    cicloUsuarioS: 60, // cada usuário real faz 1 requisição a cada 60s
    concorrenciaMax: 200, // sockets concorrentes locais
  };

  for (let i = 0; i < args.length; i++) {
    if (args[i] === '--url' && args[i + 1]) cfg.url = args[++i];
    else if (args[i] === '--duracao' && args[i + 1]) cfg.duracaoS = parseInt(args[++i], 10);
    else if (args[i] === '--degraus' && args[i + 1]) cfg.degraus = args[++i].split(',').map(Number);
    else if (args[i] === '--ciclo' && args[i + 1]) cfg.cicloUsuarioS = parseFloat(args[++i]);
    else if (args[i] === '--concorrencia' && args[i + 1]) cfg.concorrenciaMax = parseInt(args[++i], 10);
  }
  return cfg;
}

function percentil(arr, p) {
  if (!arr.length) return 0;
  const idx = Math.min(arr.length - 1, Math.floor(arr.length * (p / 100)));
  return arr[idx];
}

async function checarOrigem(url) {
  try {
    const t0 = performance.now();
    const res = await fetch(url, { headers: { 'User-Agent': 'uv-teste-carga-audit' }, signal: AbortSignal.timeout(4000) });
    const ms = performance.now() - t0;
    if (res.ok) {
      const data = await res.json();
      return { ok: true, ms: Math.round(ms), versao: data.versao, estado: data.estado, clientesWs: data.clientes };
    }
  } catch (err) {
    return { ok: false, erro: err.message };
  }
  return { ok: false, erro: 'status não ok' };
}

async function executarDegrau(cfg, clientesVirtuais) {
  // Para N clientes com ciclo de C segundos, a taxa teórica de requisições por segundo é N / C
  const rpsAlvo = Math.max(1, Math.round(clientesVirtuais / cfg.cicloUsuarioS));
  console.log(`\n================================================================================`);
  console.log(`▶ DEGRAU: ${clientesVirtuais.toLocaleString('pt-BR')} CLIENTES VIRTUAIS HTTP`);
  console.log(`  Taxa Alvo: ~${rpsAlvo.toLocaleString('pt-BR')} req/s | Duração: ${cfg.duracaoS}s | Alvo: ${cfg.url}`);
  console.log(`================================================================================`);

  const latencias = [];
  const statusCodes = {};
  const cacheStatus = {};
  let totalReqs = 0;
  let erros = 0;

  const fimEm = Date.now() + cfg.duracaoS * 1000;
  const intervaloMs = 1000 / rpsAlvo;
  
  let ativas = 0;
  let proximoDisparo = performance.now();

  const etags = new Map();

  async function dispararUma() {
    totalReqs++;
    ativas++;
    const t0 = performance.now();

    // Alterna entre /api/v1/estado e /api/v1/resultado/{topico}
    const ehEstado = Math.random() < 0.25;
    const topico = TOPICOS[Math.floor(Math.random() * TOPICOS.length)];
    const rota = ehEstado ? '/api/v1/estado' : `/api/v1/resultado/${topico}`;
    const endpoint = `${cfg.url}${rota}`;

    const headers = {
      'User-Agent': 'Mozilla/5.0 (UrnaVelox Carga HTTP/2.0)',
      'Accept': 'application/json',
    };
    const etagSalvo = etags.get(rota);
    if (etagSalvo) headers['If-None-Match'] = etagSalvo;

    try {
      const res = await fetch(endpoint, {
        headers,
        signal: AbortSignal.timeout(5000),
      });

      const lat = performance.now() - t0;
      latencias.push(lat);

      statusCodes[res.status] = (statusCodes[res.status] || 0) + 1;
      const cfStatus = res.headers.get('cf-cache-status') || 'SEM_CABECALHO';
      cacheStatus[cfStatus] = (cacheStatus[cfStatus] || 0) + 1;

      const novoEtag = res.headers.get('etag');
      if (novoEtag) etags.set(rota, novoEtag);

      // Consome o corpo para liberar o socket HTTP
      await res.arrayBuffer();
    } catch (err) {
      erros++;
      statusCodes['ERRO_' + (err.name || 'FALHA')] = (statusCodes['ERRO_' + (err.name || 'FALHA')] || 0) + 1;
    } finally {
      ativas--;
    }
  }

  const logInterval = setInterval(() => {
    const restante = Math.max(0, Math.round((fimEm - Date.now()) / 1000));
    console.log(`  [Progresso] Restam ${restante}s | Reqs: ${totalReqs} (ativas: ${ativas}) | Erros: ${erros}`);
  }, 5000);

  while (Date.now() < fimEm) {
    if (ativas < cfg.concorrenciaMax) {
      dispararUma();
    }
    proximoDisparo += intervaloMs;
    const espera = proximoDisparo - performance.now();
    if (espera > 1) {
      await new Promise((r) => setTimeout(r, Math.min(espera, 50)));
    } else {
      // Pequeno yield para o event loop
      await new Promise((r) => setImmediate(r));
    }
  }

  // Aguarda conexões pendentes terminarem (até 3s)
  const limiteEspera = Date.now() + 3000;
  while (ativas > 0 && Date.now() < limiteEspera) {
    await new Promise((r) => setTimeout(r, 100));
  }
  clearInterval(logInterval);

  latencias.sort((a, b) => a - b);
  const p50 = percentil(latencias, 50).toFixed(2);
  const p95 = percentil(latencias, 95).toFixed(2);
  const p99 = percentil(latencias, 99).toFixed(2);
  const max = (latencias[latencias.length - 1] || 0).toFixed(2);
  const rpsReal = (totalReqs / cfg.duracaoS).toFixed(1);

  const totalSucessos = (statusCodes[200] || 0) + (statusCodes[304] || 0);
  const taxaSucesso = totalReqs > 0 ? ((totalSucessos / totalReqs) * 100).toFixed(2) : '0';

  const hits = cacheStatus['HIT'] || 0;
  const reval = cacheStatus['REVALIDATED'] || 0;
  const taxaHit = totalReqs > 0 ? (((hits + reval) / totalReqs) * 100).toFixed(1) : '0';

  console.log(`\n--- RESULTADO DEGRAU ${clientesVirtuais.toLocaleString('pt-BR')} CLIENTES ---`);
  console.log(`  Vazão Real:          ${rpsReal} req/s (Total: ${totalReqs} requisições)`);
  console.log(`  Taxa de Sucesso:     ${taxaSucesso}% (${totalSucessos} sucessos / ${erros} falhas)`);
  console.log(`  Taxa de Cache CDN:   ${taxaHit}% HIT/REVALIDATED (${hits} HIT, ${reval} REVALIDATED, ${cacheStatus['MISS'] || 0} MISS)`);
  console.log(`  Latência Borda (ms): p50: ${p50} ms | p95: ${p95} ms | p99: ${p99} ms | Máx: ${max} ms`);
  console.log(`  Status HTTP:         ${JSON.stringify(statusCodes)}`);
  console.log(`  Cache Cloudflare:    ${JSON.stringify(cacheStatus)}`);

  return {
    clientes: clientesVirtuais,
    rpsAlvo,
    rpsReal: parseFloat(rpsReal),
    totalReqs,
    taxaSucesso: parseFloat(taxaSucesso),
    taxaHit: parseFloat(taxaHit),
    p50: parseFloat(p50),
    p95: parseFloat(p95),
    p99: parseFloat(p99),
    max: parseFloat(max),
    erros,
    statusCodes,
    cacheStatus
  };
}

async function main() {
  const cfg = parseArgs();
  console.log(`================================================================================`);
  console.log(`INICIANDO TESTE DE CARGA CDN CLOUDFLARE — URNA VELOX`);
  console.log(`Alvo:      ${cfg.url}`);
  console.log(`Degraus:   ${cfg.degraus.map(n => n.toLocaleString('pt-BR')).join(' → ')} clientes virtuais`);
  console.log(`Duração:   ${cfg.duracaoS} segundos por degrau`);
  console.log(`Ciclo HTTP:${cfg.cicloUsuarioS}s por cliente`);
  console.log(`================================================================================`);

  console.log(`\n[1/3] Verificando saúde da origem OCI antes do teste...`);
  const saudeAntes = await checarOrigem(cfg.origemUrl);
  console.log(`  Origem: ${JSON.stringify(saudeAntes)}`);

  const resultados = [];
  for (const degrau of cfg.degraus) {
    const res = await executarDegrau(cfg, degrau);
    resultados.push(res);
    // Intervalo de respiro entre degraus
    await new Promise((r) => setTimeout(r, 2000));
  }

  console.log(`\n[3/3] Verificando saúde da origem OCI após a carga...`);
  const saudeDepois = await checarOrigem(cfg.origemUrl);
  console.log(`  Origem: ${JSON.stringify(saudeDepois)}`);

  console.log(`\n================================================================================`);
  console.log(`RESUMO FINAL DA BATERIA`);
  console.log(`================================================================================`);
  console.table(resultados.map(r => ({
    'Clientes': r.clientes.toLocaleString('pt-BR'),
    'Req/s Real': r.rpsReal,
    'Sucesso %': `${r.taxaSucesso}%`,
    'Cache HIT %': `${r.taxaHit}%`,
    'p50 (ms)': `${r.p50} ms`,
    'p99 (ms)': `${r.p99} ms`,
    'Máx (ms)': `${r.max} ms`,
    'Erros': r.erros
  })));
}

main().catch(err => {
  console.error('Erro fatal no teste:', err);
  process.exit(1);
});
