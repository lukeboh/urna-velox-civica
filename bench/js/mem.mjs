import fs from 'node:fs';
import { Worker } from 'node:worker_threads';
const isBun = typeof Bun !== 'undefined';
const gc = () => { for (let i = 0; i < 4; i++) { if (isBun) Bun.gc(true); else globalThis.gc(); } };
const heap = () => { gc(); return isBun ? (require('bun:jsc').heapStats().heapSize) : process.memoryUsage().heapUsed; };
const rss = () => process.memoryUsage().rss;
const MB = (b) => (b / 1048576).toFixed(1);
const mode = process.argv[2];
const wl = fs.readFileSync('wl.csv', 'utf8').trim().split('\n').map(l => l.split(',').map(Number));
let seed = 7; const rnd = () => (seed = (seed * 1103515245 + 12345) & 0x7fffffff) / 0x7fffffff;
const h0 = heap(), r0 = rss();
let keep;
if (mode === 'naive') {
  const st = new Map();
  for (const [m, c, k, p] of wl) {
    const t = { nominal: new Map(), party: new Map(), b: 0, n: 0, e: 0, v: 0 };
    while (t.nominal.size < k) t.nominal.set(10 + Math.floor(rnd() * 99989), 1);
    for (let j = 0; j < p; j++) t.party.set(10 + j, 1);
    st.set(m * 256 + c, t);
  }
  keep = st;
} else if (mode === 'dense') {
  let n = 0; const off = new Uint32Array(wl.length);
  wl.forEach((w, i) => { off[i] = n; n += w[2] + w[3]; });
  const t = new Uint32Array(new SharedArrayBuffer(n * 4)); t.fill(1);
  keep = [off, t];
} else if (mode === 'workers') {
  const N = Number(process.argv[3] || 8); const ws = [];
  for (let i = 0; i < N; i++) ws.push(new Worker('setInterval(()=>{},1000)', { eval: true }));
  await new Promise(r => setTimeout(r, 1500));
  console.log(`${isBun ? 'bun ' : 'node'} ${N} workers ociosos: RSS ${MB(r0)} -> ${MB(rss())} MB (+${MB((rss() - r0) / N)} MB/worker)`);
  process.exit(0);
}
const h1 = heap();
console.log(`${isBun ? 'bun ' : 'node'} ${mode.padEnd(6)} heap vivo = ${MB(h1 - h0)} MB | RSS antes=${MB(r0)} depois=${MB(rss())} MB` + (mode === 'dense' ? ` (SAB fora do heap: ${MB(keep[1].byteLength)} MB)` : ''));
