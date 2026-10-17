import java.nio.file.*;
import java.util.*;
import java.util.concurrent.*;
import java.util.concurrent.atomic.*;

public class Mem {
    static final class Tally {
        final ConcurrentHashMap<Integer, AtomicLong> nominal = new ConcurrentHashMap<>();
        final ConcurrentHashMap<Integer, AtomicLong> party = new ConcurrentHashMap<>();
        final AtomicLong b = new AtomicLong(), n = new AtomicLong(), e = new AtomicLong(), v = new AtomicLong();
    }
    static long used() {
        for (int i = 0; i < 4; i++) { System.gc(); try { Thread.sleep(150); } catch (Exception x) {} }
        Runtime r = Runtime.getRuntime(); return r.totalMemory() - r.freeMemory();
    }
    static long rssKb() throws Exception {
        for (String l : Files.readAllLines(Paths.get("/proc/self/status"))) if (l.startsWith("VmRSS")) return Long.parseLong(l.replaceAll("\\D", ""));
        return -1;
    }
    public static void main(String[] a) throws Exception {
        String mode = a[0];
        List<int[]> wl = new ArrayList<>();
        for (String l : Files.readAllLines(Paths.get("wl.csv"))) { String[] p = l.split(","); wl.add(new int[]{Integer.parseInt(p[0]), Integer.parseInt(p[1]), Integer.parseInt(p[2]), Integer.parseInt(p[3])}); }
        long base = used(); long rss0 = rssKb();
        Random r = new Random(7);
        Object keep = null;
        if (mode.equals("naive")) {
            ConcurrentHashMap<Long, Tally> st = new ConcurrentHashMap<>();
            for (int[] w : wl) {
                Tally t = new Tally();
                while (t.nominal.size() < w[2]) t.nominal.put(10 + r.nextInt(99989), new AtomicLong(1));
                for (int j = 0; j < w[3]; j++) t.party.put(10 + j, new AtomicLong(1));
                st.put(((long) w[0] << 8) | w[1], t);
            }
            keep = st;
        } else if (mode.equals("adder")) {
            ConcurrentHashMap<Long, ConcurrentHashMap<Integer, LongAdder>> st = new ConcurrentHashMap<>();
            for (int[] w : wl) {
                ConcurrentHashMap<Integer, LongAdder> m = new ConcurrentHashMap<>();
                while (m.size() < w[2] + w[3]) { LongAdder la = new LongAdder(); la.increment(); m.put(10 + r.nextInt(99989), la); }
                st.put(((long) w[0] << 8) | w[1], m);
            }
            keep = st;
        } else {
            int n = 0; int[] off = new int[wl.size()];
            for (int i = 0; i < wl.size(); i++) { off[i] = n; n += wl.get(i)[2] + wl.get(i)[3]; }
            AtomicIntegerArray t = new AtomicIntegerArray(n);
            for (int i = 0; i < n; i++) t.set(i, 1);
            keep = new Object[]{off, t};
        }
        long u = used();
        System.out.printf("java %-6s heap vivo = %.1f MB | heap comprometido = %.1f MB | RSS antes=%.0f MB depois=%.0f MB%n",
            mode, (u - base) / 1048576.0, Runtime.getRuntime().totalMemory() / 1048576.0, rss0 / 1024.0, rssKb() / 1024.0);
        if (keep.hashCode() == 42) System.out.println();
    }
}
