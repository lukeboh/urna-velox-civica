// Benchmark de totalização (Plano C — Java). Mesmo corpus/layout gerado por `urnabench gen`.
// Uso: java -cp .:bcprov.jar Bench <dir> <geral|municipal> <variante> <threads> <bus>
//   variantes: manual-dense | bc-dense | manual-chm
// Saída: lang,variant,kind,threads,bus,seconds,bus_per_s,consolidate_ms,peak_rss_mb,corpus_mb,invalid
import java.nio.file.*;
import java.util.*;
import java.util.concurrent.*;
import java.util.concurrent.atomic.*;

public class Bench {
    static final int N_PARTIDOS = 31;
    static String kind; static int nc, slots; static int[] base, cnt; static byte[] muniUf;
    static byte[] blob; static int[] st, en;
    static AtomicIntegerArray tally; static AtomicLongArray dedup;
    static final AtomicLong invalid = new AtomicLong();

    static int cargoIndex(int c) { switch (c) { case 1: case 11: return 0; case 3: case 13: return 1; case 5: return 2; case 6: return 3; case 7: case 8: return 4; default: return 99; } }
    static int codeBase(int ci) { return kind.equals("geral") ? new int[]{10, 10, 100, 1000, 10000}[ci] : new int[]{10, 10000}[ci]; }
    static int vagas(int c) { return c == 5 ? 2 : 1; }

    interface Sink { void secao(int m); void cargo(int code, int comp); void voto(int tipo, int qtd, int partido, int codigo); void fimCargo(); }

    static final class Dense implements Sink {
        int muni, b, n, cb, comp, vag, soma; long inv;
        public void secao(int m) { muni = m; }
        public void cargo(int code, int c) { int ci = cargoIndex(code), k = muni * nc + ci; b = base[k]; n = cnt[k]; cb = codeBase(ci); comp = c; vag = vagas(code); soma = 0; tally.getAndAdd(b + n + N_PARTIDOS + 2, c); }
        public void voto(int tipo, int q, int p, int c) {
            int off = tipo == 1 ? c - cb : tipo == 4 ? n + (p - 10) : tipo == 2 ? n + N_PARTIDOS : n + N_PARTIDOS + 1;
            tally.getAndAdd(b + off, q); soma += q;
        }
        public void fimCargo() { if (soma != comp * vag) inv++; }
    }

    // Desenho "ingênuo": mapas concorrentes aninhados em 3 níveis, com boxing.
    static final class TallyM { final ConcurrentHashMap<Integer, AtomicLong> nominal = new ConcurrentHashMap<>(), party = new ConcurrentHashMap<>(); final AtomicLong blank = new AtomicLong(), nul = new AtomicLong(), comp = new AtomicLong(); }
    static final ConcurrentHashMap<Long, TallyM> MUNI = new ConcurrentHashMap<>(), UF = new ConcurrentHashMap<>(), BR = new ConcurrentHashMap<>();
    static final class Chm implements Sink {
        int muni, code, comp; final ArrayList<int[]> buf = new ArrayList<>();
        public void secao(int m) { muni = m; }
        public void cargo(int c, int cp) { code = c; comp = cp; buf.clear(); }
        public void voto(int t, int q, int p, int c) { buf.add(new int[]{t, q, p, c}); }
        void apply(TallyM t) { t.comp.addAndGet(comp); for (int[] v : buf) { switch (v[0]) {
            case 1: t.nominal.computeIfAbsent(v[3], k -> new AtomicLong()).addAndGet(v[1]); break;
            case 4: t.party.computeIfAbsent(v[2], k -> new AtomicLong()).addAndGet(v[1]); break;
            case 2: t.blank.addAndGet(v[1]); break; default: t.nul.addAndGet(v[1]); } } }
        public void fimCargo() {
            apply(MUNI.computeIfAbsent(((long) muni << 8) | code, k -> new TallyM()));
            apply(UF.computeIfAbsent(((long) muniUf[muni] << 8) | code, k -> new TallyM()));
            if (code == 1) apply(BR.computeIfAbsent((long) code, k -> new TallyM()));
        }
    }

    // ---- parser BER manual sobre byte[] (sem alocação por campo)
    static int tStart, tEnd;
    static int tlv(byte[] b, int p, int[] o) { // o[0]=início do valor, o[1]=fim; retorna tag
        int tag = b[p] & 0xff, l0 = b[p + 1] & 0xff;
        if (l0 < 0x80) { o[0] = p + 2; o[1] = p + 2 + l0; } else { int n = l0 & 0x7f, l = 0; for (int k = 0; k < n; k++) l = (l << 8) | (b[p + 2 + k] & 0xff); o[0] = p + 2 + n; o[1] = o[0] + l; }
        return tag;
    }
    static int uint(byte[] b, int s, int e) { int v = 0; for (int k = s; k < e; k++) v = (v << 8) | (b[k] & 0xff); return v; }

    static void parseManual(byte[] b, int s0, int e0, Sink sk, int[][] o) {
        int[] a = o[0]; tlv(b, s0, a); int p = a[0], e = a[1], cs = 0, ce = 0;
        while (p < e) { int t = tlv(b, p, a); if (t == 0x84) { cs = a[0]; ce = a[1]; } p = a[1]; }
        tlv(b, cs, a); p = a[0]; e = a[1];
        while (p < e) {
            int t = tlv(b, p, a); int vs = a[0], ve = a[1];
            if (t == 0xA3) { int q = vs, mu = 0; int[] c = o[1]; while (q < ve) { int t2 = tlv(b, q, c); if (t2 == 0x80) mu = uint(b, c[0], c[1]); q = c[1]; } sk.secao(mu); }
            else if (t == 0xA7) {
                int q = vs; int[] c1 = o[1], c2 = o[2], c3 = o[3], c4 = o[4], c5 = o[5], c6 = o[6], c7 = o[7], c8 = o[8], c9 = o[9];
                while (q < ve) { tlv(b, q, c1); int r = c1[0], z = c1[1];
                    while (r < z) { int t3 = tlv(b, r, c2);
                        if (t3 == 0xA2) { int rv = c2[0];
                            while (rv < c2[1]) { tlv(b, rv, c3); int x = c3[0], comp = 0;
                                while (x < c3[1]) { int t5 = tlv(b, x, c4);
                                    if (t5 == 0x81) comp = uint(b, c4[0], c4[1]);
                                    else if (t5 == 0xA2) { int tc = c4[0];
                                        while (tc < c4[1]) { tlv(b, tc, c5); int y = c5[0];
                                            while (y < c5[1]) { int t7 = tlv(b, y, c6);
                                                if (t7 == 0x80) sk.cargo(uint(b, c6[0], c6[1]), comp);
                                                else if (t7 == 0xA2) { int vv = c6[0];
                                                    while (vv < c6[1]) { tlv(b, vv, c7); int w = c7[0], tp = 0, qt = 0, pa = 0, co = 0;
                                                        while (w < c7[1]) { int t9 = tlv(b, w, c8);
                                                            if (t9 == 0x80) tp = uint(b, c8[0], c8[1]); else if (t9 == 0x81) qt = uint(b, c8[0], c8[1]);
                                                            else if (t9 == 0xA2) { int iv = c8[0]; while (iv < c8[1]) { int ta = tlv(b, iv, c9); if (ta == 0x80) pa = uint(b, c9[0], c9[1]); else if (ta == 0x81) co = uint(b, c9[0], c9[1]); iv = c9[1]; } }
                                                            w = c8[1]; }
                                                        sk.voto(tp, qt, pa, co); vv = c7[1]; }
                                                    sk.fimCargo(); }
                                                y = c6[1]; }
                                            tc = c5[1]; } }
                                    x = c4[1]; }
                                rv = c3[1]; } }
                        r = c2[1]; }
                    q = z; }
            }
            p = ve;
        }
    }

    // ---- variante com biblioteca: Bouncy Castle monta a árvore de objetos ASN.1
    static int bint(org.bouncycastle.asn1.ASN1TaggedObject t) { return org.bouncycastle.asn1.ASN1Integer.getInstance(t, false).intValueExact(); }
    static org.bouncycastle.asn1.ASN1Sequence bseq(org.bouncycastle.asn1.ASN1TaggedObject t) { return org.bouncycastle.asn1.ASN1Sequence.getInstance(t, false); }
    static void parseBC(byte[] b, int s, int e, Sink sk) throws Exception {
        org.bouncycastle.asn1.ASN1Sequence env = org.bouncycastle.asn1.ASN1Sequence.getInstance(org.bouncycastle.asn1.ASN1Primitive.fromByteArray(Arrays.copyOfRange(b, s, e)));
        byte[] inner = null;
        for (org.bouncycastle.asn1.ASN1Encodable x : env) { org.bouncycastle.asn1.ASN1TaggedObject t = (org.bouncycastle.asn1.ASN1TaggedObject) x; if (t.getTagNo() == 4) inner = org.bouncycastle.asn1.ASN1OctetString.getInstance(t, false).getOctets(); }
        org.bouncycastle.asn1.ASN1Sequence bu = org.bouncycastle.asn1.ASN1Sequence.getInstance(org.bouncycastle.asn1.ASN1Primitive.fromByteArray(inner));
        for (org.bouncycastle.asn1.ASN1Encodable x : bu) { org.bouncycastle.asn1.ASN1TaggedObject t = (org.bouncycastle.asn1.ASN1TaggedObject) x;
            if (t.getTagNo() == 3) { for (org.bouncycastle.asn1.ASN1Encodable y : bseq(t)) { org.bouncycastle.asn1.ASN1TaggedObject u = (org.bouncycastle.asn1.ASN1TaggedObject) y; if (u.getTagNo() == 0) sk.secao(bint(u)); } }
            else if (t.getTagNo() == 7) { for (org.bouncycastle.asn1.ASN1Encodable el : bseq(t)) { for (org.bouncycastle.asn1.ASN1Encodable f : (org.bouncycastle.asn1.ASN1Sequence) el) { org.bouncycastle.asn1.ASN1TaggedObject tf = (org.bouncycastle.asn1.ASN1TaggedObject) f; if (tf.getTagNo() != 2) continue;
                for (org.bouncycastle.asn1.ASN1Encodable rv : bseq(tf)) { int comp = 0;
                    for (org.bouncycastle.asn1.ASN1Encodable g : (org.bouncycastle.asn1.ASN1Sequence) rv) { org.bouncycastle.asn1.ASN1TaggedObject tg = (org.bouncycastle.asn1.ASN1TaggedObject) g;
                        if (tg.getTagNo() == 1) comp = bint(tg);
                        else if (tg.getTagNo() == 2) { for (org.bouncycastle.asn1.ASN1Encodable tc : bseq(tg)) { for (org.bouncycastle.asn1.ASN1Encodable h : (org.bouncycastle.asn1.ASN1Sequence) tc) { org.bouncycastle.asn1.ASN1TaggedObject th = (org.bouncycastle.asn1.ASN1TaggedObject) h;
                            if (th.getTagNo() == 0) sk.cargo(bint(th), comp);
                            else if (th.getTagNo() == 2) { for (org.bouncycastle.asn1.ASN1Encodable vv : bseq(th)) { int tp = 0, qt = 0, pa = 0, co = 0;
                                for (org.bouncycastle.asn1.ASN1Encodable k : (org.bouncycastle.asn1.ASN1Sequence) vv) { org.bouncycastle.asn1.ASN1TaggedObject tk = (org.bouncycastle.asn1.ASN1TaggedObject) k;
                                    switch (tk.getTagNo()) { case 0: tp = bint(tk); break; case 1: qt = bint(tk); break;
                                        case 2: for (org.bouncycastle.asn1.ASN1Encodable id : bseq(tk)) { org.bouncycastle.asn1.ASN1TaggedObject ti = (org.bouncycastle.asn1.ASN1TaggedObject) id; if (ti.getTagNo() == 0) pa = bint(ti); else co = bint(ti); } break;
                                        default: org.bouncycastle.asn1.ASN1OctetString.getInstance(tk, false); } }
                                sk.voto(tp, qt, pa, co); }
                                sk.fimCargo(); } } } } } } } } }
        }
    }

    static double peakRssMb() throws Exception { for (String l : Files.readAllLines(Paths.get("/proc/self/status"))) if (l.startsWith("VmHWM")) return Long.parseLong(l.replaceAll("\\D", "")) / 1024.0; return -1; }

    public static void main(String[] a) throws Exception {
        String dir = a[0]; kind = a[1]; String variant = a[2]; int threads = Integer.parseInt(a[3]); int bus = Integer.parseInt(a[4]);
        List<String> L = Files.readAllLines(Paths.get(dir, "layout_" + kind + ".csv"));
        String[] h = L.get(0).split(","); int nm = Integer.parseInt(h[1]); nc = Integer.parseInt(h[2]); slots = Integer.parseInt(h[3]);
        base = new int[nm * nc]; cnt = new int[nm * nc]; muniUf = new byte[nm];
        for (int i = 0; i < nm; i++) { String[] v = L.get(i + 1).split(","); muniUf[i] = (byte) Integer.parseInt(v[1]); for (int c = 0; c < nc; c++) { base[i * nc + c] = Integer.parseInt(v[2 + 2 * c]); cnt[i * nc + c] = Integer.parseInt(v[3 + 2 * c]); } }
        blob = Files.readAllBytes(Paths.get(dir, "corpus_" + kind + ".bin"));
        java.nio.ByteBuffer bb = java.nio.ByteBuffer.wrap(blob).order(java.nio.ByteOrder.LITTLE_ENDIAN);
        int n = bb.getInt(0), p = 4; st = new int[n]; en = new int[n];
        for (int i = 0; i < n; i++) { int l = bb.getInt(p); p += 4; st[i] = p; en[i] = p + l; p += l; }
        tally = new AtomicIntegerArray(slots); dedup = new AtomicLongArray(bus / 64 + 1);
        // aquecimento do JIT (não contabilizado): 1 passada curta numa cópia descartável do estado
        AtomicIntegerArray realTally = tally; tally = new AtomicIntegerArray(slots);
        warm(variant, Math.min(4000, bus), threads);
        tally = realTally; MUNI.clear(); UF.clear(); BR.clear(); dedup = new AtomicLongArray(bus / 64 + 1); invalid.set(0);
        final AtomicInteger next = new AtomicInteger(); final int CH = 64;
        long t0 = System.nanoTime();
        Thread[] ts = new Thread[threads];
        for (int t = 0; t < threads; t++) { ts[t] = new Thread(() -> work(variant, bus, next, CH)); ts[t].start(); }
        for (Thread t : ts) t.join();
        double secs = (System.nanoTime() - t0) / 1e9;
        long t1 = System.nanoTime();
        if (!variant.equals("manual-chm")) { // consolidação UF/Brasil
            long[][] uf = new long[27 * nc][];
            for (int m = 0; m < nm; m++) for (int c = 0; c < nc; c++) { int k = m * nc + c, b = base[k], sz = cnt[k] + N_PARTIDOS + 3; int u = muniUf[m] * nc + c; if (uf[u] == null || uf[u].length < sz) uf[u] = uf[u] == null ? new long[sz] : Arrays.copyOf(uf[u], sz); for (int j = 0; j < sz; j++) uf[u][j] += tally.get(b + j); }
        }
        double cons = (System.nanoTime() - t1) / 1e6;
        if (System.getenv("CHECK") != null) { long ck = 0; for (int i = 0; i < slots; i++) ck += (long) (i % 1009 + 1) * tally.get(i); System.err.println("checksum=" + ck); }
        System.out.printf(Locale.ROOT, "java,%s,%s,%d,%d,%.4f,%.0f,%.1f,%.1f,%.1f,%d%n", variant, kind, threads, bus, secs, bus / secs, cons, peakRssMb(), blob.length / 1048576.0, invalid.get());
    }
    static void warm(String variant, int bus, int threads) throws Exception {
        final AtomicInteger next = new AtomicInteger(); Thread[] ts = new Thread[Math.max(1, Math.min(threads, 4))];
        for (int t = 0; t < ts.length; t++) { ts[t] = new Thread(() -> work(variant, bus, next, 64)); ts[t].start(); } for (Thread t : ts) t.join();
    }
    static void work(String variant, int bus, AtomicInteger next, int CH) {
        Dense d = new Dense(); Chm c = new Chm(); int[][] o = new int[10][2];
        try {
            while (true) { int s = next.getAndAdd(CH); if (s >= bus) break;
                for (int i = s; i < Math.min(s + CH, bus); i++) {
                    long bit = 1L << (i & 63); if ((dedup.getAndAccumulate(i >> 6, bit, (x, y) -> x | y) & bit) != 0) continue;
                    int k = i % st.length;
                    switch (variant) {
                        case "manual-dense": parseManual(blob, st[k], en[k], d, o); break;
                        case "bc-dense": parseBC(blob, st[k], en[k], d); break;
                        case "manual-chm": parseManual(blob, st[k], en[k], c, o); break;
                        default: throw new RuntimeException("variante");
                    }
                } }
        } catch (Exception ex) { throw new RuntimeException(ex); }
        invalid.addAndGet(d.inv);
    }
}
