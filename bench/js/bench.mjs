// Benchmark de totalização (Plano B — JavaScript, Node/Bun). Mesmo corpus/layout de `urnabench gen`.
// Uso: node bench.mjs <dir> <geral|municipal> <manual-dense|asn1js-dense> <threads> <bus>
// Saída: lang,variant,kind,threads,bus,seconds,bus_per_s,consolidate_ms,peak_rss_mb,corpus_mb,invalid
import fs from 'node:fs';
import path from 'node:path';
import { Worker, isMainThread, workerData, parentPort } from 'node:worker_threads';
import { fileURLToPath } from 'node:url';

const N_PARTIDOS = 31;
const cargoIndex = (c) => (c === 1 || c === 11) ? 0 : (c === 3 || c === 13) ? 1 : c === 5 ? 2 : c === 6 ? 3 : (c === 7 || c === 8) ? 4 : 99;
const vagas = (c) => c === 5 ? 2 : 1;

if (isMainThread) {
  const [dir, kind, variant, threadsS, busS] = process.argv.slice(2);
  const threads = Number(threadsS), bus = Number(busS);
  const L = fs.readFileSync(path.join(dir, `layout_${kind}.csv`), 'utf8').trim().split('\n');
  const h = L[0].split(','); const nm = +h[1], nc = +h[2], slots = +h[3];
  const lay = new Int32Array(new SharedArrayBuffer(nm * nc * 2 * 4)); const muniUf = new Uint8Array(nm);
  for (let i = 0; i < nm; i++) { const v = L[i + 1].split(',').map(Number); muniUf[i] = v[1]; for (let c = 0; c < nc; c++) { lay[(i * nc + c) * 2] = v[2 + 2 * c]; lay[(i * nc + c) * 2 + 1] = v[3 + 2 * c]; } }
  const file = fs.readFileSync(path.join(dir, `corpus_${kind}.bin`));
  const corpus = new Uint8Array(new SharedArrayBuffer(file.length)); corpus.set(file);
  const dv = new DataView(corpus.buffer); const n = dv.getUint32(0, true);
  const idx = new Int32Array(new SharedArrayBuffer(n * 8)); let p = 4;
  for (let i = 0; i < n; i++) { const l = dv.getUint32(p, true); p += 4; idx[2 * i] = p; idx[2 * i + 1] = p + l; p += l; }
  const mk = () => ({ tally: new Uint32Array(new SharedArrayBuffer(slots * 4)), dedup: new Int32Array(new SharedArrayBuffer((Math.ceil(bus / 32) + 1) * 4)), next: new Int32Array(new SharedArrayBuffer(8)), inv: new Int32Array(new SharedArrayBuffer(8)) });
  const self = fileURLToPath(import.meta.url);
  const runPool = (st, nbus, nthreads) => new Promise((res, rej) => {
    let done = 0;
    for (let t = 0; t < nthreads; t++) {
      const w = new Worker(self, { workerData: { kind, variant, nc, lay, corpus, idx, n, bus: nbus, ...st } });
      w.on('message', () => { if (++done === nthreads) res(); }); w.on('error', rej);
    }
  });
  // aquecimento (JIT + criação dos workers não entram na medição do segundo pool? -> entram: medimos o pool completo, inclusive spawn)
  await runPool(mk(), Math.min(4000, bus), Math.min(threads, 4));
  const st = mk();
  const t0 = performance.now();
  await runPool(st, bus, threads);
  const secs = (performance.now() - t0) / 1000;
  const t1 = performance.now();
  const uf = new Map();
  for (let m = 0; m < nm; m++) for (let c = 0; c < nc; c++) { const k = m * nc + c, b = lay[2 * k], sz = lay[2 * k + 1] + N_PARTIDOS + 3, key = muniUf[m] * nc + c;
    let a = uf.get(key); if (!a || a.length < sz) { const na = new Float64Array(sz); if (a) na.set(a); a = na; uf.set(key, a); } for (let j = 0; j < sz; j++) a[j] += st.tally[b + j]; }
  const cons = performance.now() - t1;
  if (process.env.CHECK) { let ck = 0n; for (let i = 0; i < slots; i++) ck += BigInt((i % 1009) + 1) * BigInt(st.tally[i]); console.error('checksum=' + ck); }
  const isBun = typeof Bun !== 'undefined';
  const peak = process.resourceUsage().maxRSS / 1024;
  console.log(`${isBun ? 'bun' : 'node'},${variant},${kind},${threads},${bus},${secs.toFixed(4)},${(bus / secs).toFixed(0)},${cons.toFixed(1)},${peak.toFixed(1)},${(file.length / 1048576).toFixed(1)},${st.inv[0]}`);
  process.exit(0);
} else {
  const { kind, variant, nc, lay, corpus: b, idx, n, bus, tally, dedup, next, inv } = workerData;
  const codeBase = kind === 'geral' ? [10, 10, 100, 1000, 10000] : [10, 10000];
  let asn1js = null; if (variant === 'asn1js-dense') asn1js = await import('asn1js');
  // agregador denso
  let muni = 0, base = 0, cnt = 0, cb = 0, comp = 0, vag = 1, soma = 0, invalid = 0;
  const secao = (m) => { muni = m; };
  const cargo = (code, c) => { const ci = cargoIndex(code), k = muni * nc + ci; base = lay[2 * k]; cnt = lay[2 * k + 1]; cb = codeBase[ci]; comp = c; vag = vagas(code); soma = 0; Atomics.add(tally, base + cnt + N_PARTIDOS + 2, c); };
  const voto = (tp, q, pa, co) => { const off = tp === 1 ? co - cb : tp === 4 ? cnt + (pa - 10) : tp === 2 ? cnt + N_PARTIDOS : cnt + N_PARTIDOS + 1; Atomics.add(tally, base + off, q); soma += q; };
  const fimCargo = () => { if (soma !== comp * vag) invalid++; };
  // TLV: devolve tag e grava início/fim do valor em vs/ve
  let vs = 0, ve = 0;
  const tlv = (p) => { const t = b[p], l0 = b[p + 1]; if (l0 < 0x80) { vs = p + 2; ve = vs + l0; } else { const k = l0 & 0x7f; let l = 0; for (let j = 0; j < k; j++) l = (l << 8) | b[p + 2 + j]; vs = p + 2 + k; ve = vs + l; } return t; };
  const uint = (s, e) => { let v = 0; for (let k = s; k < e; k++) v = (v * 256) + b[k]; return v; };
  function parseManual(s0) {
    tlv(s0); let p = vs, e = ve, cs = 0;
    while (p < e) { const t = tlv(p); if (t === 0x84) cs = vs; p = ve; }
    tlv(cs); p = vs; e = ve;
    while (p < e) { const t = tlv(p); const a0 = vs, z0 = ve;
      if (t === 0xA3) { let q = a0; while (q < z0) { const t2 = tlv(q); if (t2 === 0x80) secao(uint(vs, ve)); q = ve; } }
      else if (t === 0xA7) { let q = a0;
        while (q < z0) { tlv(q); const z1 = ve; let r = vs;
          while (r < z1) { const t3 = tlv(r); const z2 = ve;
            if (t3 === 0xA2) { let rv = vs;
              while (rv < z2) { tlv(rv); const z3 = ve; let x = vs, cp = 0;
                while (x < z3) { const t5 = tlv(x); const z4 = ve;
                  if (t5 === 0x81) cp = uint(vs, ve);
                  else if (t5 === 0xA2) { let tc = vs;
                    while (tc < z4) { tlv(tc); const z5 = ve; let y = vs;
                      while (y < z5) { const t7 = tlv(y); const z6 = ve;
                        if (t7 === 0x80) cargo(uint(vs, ve), cp);
                        else if (t7 === 0xA2) { let vv = vs;
                          while (vv < z6) { tlv(vv); const z7 = ve; let w = vs, tp = 0, qt = 0, pa = 0, co = 0;
                            while (w < z7) { const t9 = tlv(w); const z8 = ve;
                              if (t9 === 0x80) tp = uint(vs, ve); else if (t9 === 0x81) qt = uint(vs, ve);
                              else if (t9 === 0xA2) { let iv = vs; while (iv < z8) { const ta = tlv(iv); if (ta === 0x80) pa = uint(vs, ve); else if (ta === 0x81) co = uint(vs, ve); iv = ve; } }
                              w = z8; }
                            voto(tp, qt, pa, co); vv = z7; }
                          fimCargo(); }
                        y = z6; }
                      tc = z5; } }
                  x = z4; }
                rv = z3; } }
            r = z2; }
          q = z1; } }
      p = z0; }
  }
  // variante com biblioteca: asn1js monta a árvore de objetos
  const kids = (o) => o.valueBlock.value;
  const tagNo = (o) => o.idBlock.tagNumber;
  const pint = (o) => { const h = o.valueBlock.valueHexView; let v = 0; for (let k = 0; k < h.length; k++) v = v * 256 + h[k]; return v; };
  function parseAsn1js(s, e) {
    const env = asn1js.fromBER(b.slice(s, e)).result; let inner = null;
    for (const x of kids(env)) if (tagNo(x) === 4) inner = x.valueBlock.valueHexView;
    const bu = asn1js.fromBER(inner.slice()).result;
    for (const x of kids(bu)) {
      if (tagNo(x) === 3) { for (const y of kids(x)) if (tagNo(y) === 0) secao(pint(y)); }
      else if (tagNo(x) === 7) { for (const el of kids(x)) for (const f of kids(el)) { if (tagNo(f) !== 2) continue;
        for (const rv of kids(f)) { let cp = 0; for (const g of kids(rv)) {
          if (tagNo(g) === 1) cp = pint(g);
          else if (tagNo(g) === 2) for (const tc of kids(g)) for (const hh of kids(tc)) {
            if (tagNo(hh) === 0) cargo(pint(hh), cp);
            else if (tagNo(hh) === 2) { for (const vv of kids(hh)) { let tp = 0, qt = 0, pa = 0, co = 0;
              for (const k of kids(vv)) { const tn = tagNo(k); if (tn === 0) tp = pint(k); else if (tn === 1) qt = pint(k); else if (tn === 2) for (const id of kids(k)) { if (tagNo(id) === 0) pa = pint(id); else co = pint(id); } }
              voto(tp, qt, pa, co); } fimCargo(); } } } } } }
    }
  }
  const CH = 64;
  for (;;) { const s = Atomics.add(next, 0, CH); if (s >= bus) break;
    for (let i = s; i < Math.min(s + CH, bus); i++) {
      const bit = 1 << (i & 31); if (Atomics.or(dedup, i >> 5, bit) & bit) continue;
      const k = i % n;
      if (variant === 'manual-dense') parseManual(idx[2 * k]); else parseAsn1js(idx[2 * k], idx[2 * k + 1]);
    } }
  Atomics.add(inv, 0, invalid);
  parentPort.postMessage(1);
}
