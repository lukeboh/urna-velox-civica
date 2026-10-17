#!/usr/bin/env python3
"""Passo 5 do benchmark — lê os resultados, imprime as tabelas no terminal e gera
RELATORIO.md, summary.csv e os gráficos.

Uso:  python3 bench/scripts/relatorio.py [--resultados bench/results] [--img <pasta>]
      (por padrão os gráficos vão para <resultados>/graficos/; docs/img/ guarda a cópia
       de referência usada em docs/ESTIMATIVAS_MEMORIA_E_TEMPO.md)
Requer matplotlib. Cores: paleta categórica de referência, ordem fixa por linguagem
(Rust=azul, Java=laranja, Node=verde-água, Bun=amarelo) — a cor segue a linguagem em todos os gráficos.
"""
import argparse, csv, re, statistics, pathlib, json
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib.ticker import FuncFormatter

ROOT = pathlib.Path(__file__).resolve().parents[2]
ap = argparse.ArgumentParser()
ap.add_argument("--resultados", default=str(ROOT / "bench" / "results"))
ap.add_argument("--img", default=None)
args = ap.parse_args()
RES = pathlib.Path(args.resultados)
IMG = pathlib.Path(args.img) if args.img else RES / "graficos"
IMG.mkdir(parents=True, exist_ok=True)
MACHINE = (RES / "machine.txt").read_text(errors="ignore") if (RES / "machine.txt").exists() else ""
_m = re.search(r"Model name:\s*(.+)", MACHINE); CPU = _m.group(1).strip() if _m else "CPU não identificada"

SURFACE, INK, INK2, MUTED, GRID = "#fcfcfb", "#0b0b0b", "#52514e", "#8a8984", "#e6e5e1"
COR = {"rust": "#2a78d6", "java": "#eb6834", "node": "#1baf7a", "bun": "#eda100"}
NOME = {"rust": "Rust (Plano A)", "java": "Java (Plano C)", "node": "Node.js (Plano B)", "bun": "Bun (Plano B)"}
ELEICAO_BUS = 500_000

plt.rcParams.update({
    "figure.facecolor": SURFACE, "axes.facecolor": SURFACE, "savefig.facecolor": SURFACE,
    "axes.edgecolor": GRID, "axes.labelcolor": INK2, "xtick.color": INK2, "ytick.color": INK2,
    "text.color": INK, "font.size": 10, "axes.titlesize": 12, "axes.titleweight": "bold",
    "axes.spines.top": False, "axes.spines.right": False, "axes.grid": True, "grid.color": GRID,
    "grid.linewidth": 0.8, "axes.axisbelow": True, "legend.frameon": False,
})

def br(x, nd=1):
    return f"{x:,.{nd}f}".replace(",", "X").replace(".", ",").replace("X", ".")

# ------------------------------------------------------------------ dados de tempo
rows = list(csv.DictReader(open(RES / "time.csv")))
for r in rows:
    r["threads"] = int(r["threads"]); r["bus"] = int(r["bus"]); r["seconds"] = float(r["seconds"])
    r["peak_rss_mb"] = float(r["peak_rss_mb"]); r["corpus_mb"] = float(r["corpus_mb"])
    # tempo por BU e projeção para 500 mil BUs (variantes rodadas com menos BUs são extrapoladas linearmente)
    r["us_per_bu_wall"] = r["seconds"] / r["bus"] * 1e6
    r["t500k"] = r["seconds"] * ELEICAO_BUS / r["bus"]

def med(lang, variant, kind, t, key="t500k"):
    v = [r[key] for r in rows if r["lang"] == lang and r["variant"] == variant and r["kind"] == kind and r["threads"] == t]
    return statistics.median(v) if v else None

THREADS = sorted({r["threads"] for r in rows})
BUS_REF = max(r["bus"] for r in rows)
TMAX = max(THREADS)
VARIANTS = sorted({(r["lang"], r["variant"], r["kind"]) for r in rows})

# tabela-resumo persistida (usada no documento de estimativas)
summary = []
for (lang, var, kind) in VARIANTS:
    for t in THREADS:
        s = med(lang, var, kind, t)
        if s is None: continue
        rss = med(lang, var, kind, t, "peak_rss_mb"); corp = med(lang, var, kind, t, "corpus_mb")
        summary.append({"lang": lang, "variant": var, "kind": kind, "threads": t, "t500k_s": round(s, 4),
                        "bus_per_s": round(ELEICAO_BUS / s), "us_por_bu_1thread_equiv": round(s / ELEICAO_BUS * 1e6 * t, 2),
                        "peak_rss_mb": rss, "rss_sem_corpus_mb": round(rss - corp, 1)})
with open(RES / "summary.csv", "w", newline="") as f:
    w = csv.DictWriter(f, fieldnames=list(summary[0].keys())); w.writeheader(); w.writerows(summary)

# ------------------------------------------------------------------ Fig. 1 — memória do estado
def grab(path, pat):
    txt = open(path).read() if path.exists() else ""
    m = re.search(pat, txt); return float(m.group(1).replace(",", ".")) if m else None

mr, mj, mn = RES / "mem_rust.txt", RES / "mem_java.txt", RES / "mem_node.txt"
dense_full_mb = None
dense_full_slots = None
for fn in ("geracao.txt", "run.log"):
    for line in open(RES / fn, errors="ignore") if (RES / fn).exists() else []:
        m = re.search(r"\[gen\] geral:.*slots_densos=(\d+) \(([\d.]+) MB em u32\)", line)
        if m: dense_full_slots, dense_full_mb = int(m.group(1)), float(m.group(2))
_md = re.search(r"DashMap aninhado,\s*(\d+) shards/mapa \(padrão\)\s*:\s*([\d.]+)", open(mr).read() if mr.exists() else "")
SHARDS_PADRAO = int(_md.group(1)) if _md else None
mem = [
    ("rust", f"DashMap aninhado (docs originais), {SHARDS_PADRAO} shards (padrão nesta máquina)", float(_md.group(2)) if _md else None),
    ("rust", "DashMap aninhado, 128 shards (28–32 threads)", grab(mr, r"DashMap aninhado, 128 shards/mapa\s{3,}:\s*([\d.]+)") if SHARDS_PADRAO != 128 else None),
    ("rust", "DashMap aninhado, 64 shards (16 threads)", grab(mr, r" 64 shards/mapa\s*:\s*([\d.]+)")),
    ("java", "ConcurrentHashMap + AtomicLong (boxing)", grab(mj, r"java naive\s+heap vivo = ([\d.]+)")),
    ("bun", "Map aninhado (JSC)", None),
    ("node", "Map aninhado (V8)", grab(mn, r"node naive\s+heap vivo = ([\d.]+)")),
    ("rust", "Tabela hash plana (hashbrown)", grab(mr, r"hashbrown plano u64->u32\s*:\s*([\d.]+)")),
    ("rust", f"Vetor denso, layout completo ({br((dense_full_slots or 0) / 1e6, 2)} M contadores)", dense_full_mb),
    ("java", "Vetor denso só com pares observados (AtomicIntegerArray)", grab(mj, r"java dense\s+heap vivo = ([\d.]+)")),
    ("node", "Vetor denso só com pares observados (SharedArrayBuffer)", grab(mn, r"SAB fora do heap: ([\d.]+)")),
    ("rust", "Vetor denso só com pares observados (Vec<AtomicU32>)", grab(mr, r"Denso Vec<AtomicU32> \+ offsets\s*:\s*([\d.]+)")),
]
mem = [m for m in mem if m[2] is not None]
fig, ax = plt.subplots(figsize=(9.5, 5.2))
ys = range(len(mem))[::-1]
for y, (lang, lab, v) in zip(ys, mem):
    ax.barh(y, v, height=0.62, color=COR[lang], edgecolor=SURFACE, linewidth=2)
    ax.text(v + 12, y, f"{br(v)} MB", va="center", fontsize=9, color=INK)
ax.set_yticks(list(ys)); ax.set_yticklabels([m[1] for m in mem], fontsize=9)
ax.axvline(500, color=MUTED, ls="--", lw=1.2); ax.text(505, len(mem) - 0.6, "meta original: 500 MB (processo inteiro)", color=INK2, fontsize=8.5)
ax.set_xlabel("MB só para o estado de totalização, eleição geral")
ax.set_title("Memória do estado de totalização por desenho e linguagem")
ax.grid(axis="y", visible=False)
handles = [plt.Rectangle((0, 0), 1, 1, color=COR[k]) for k in ("rust", "java", "node")]
ax.legend(handles, [NOME[k] for k in ("rust", "java", "node")], loc="lower right")
ax.set_xlim(0, max(m[2] for m in mem) * 1.18)
fig.tight_layout(); fig.savefig(IMG / "memoria_estado.png", dpi=150); plt.close(fig)

# ------------------------------------------------------------------ Fig. 2 — tempo total vs threads (parser manual + vetor denso)
fig, ax = plt.subplots(figsize=(9, 5))
for lang in ("rust", "java", "node", "bun"):
    pts = [(t, med(lang, "manual-dense", "geral", t)) for t in THREADS]
    pts = [p for p in pts if p[1] is not None]
    if not pts: continue
    ax.plot([p[0] for p in pts], [p[1] for p in pts], marker="o", ms=6, lw=2, color=COR[lang], label=NOME[lang])
    t, v = pts[-1]; ax.annotate(f"{br(v, 2)} s", (t, v), xytext=(6, 0), textcoords="offset points", va="center", fontsize=9, color=INK)
    t, v = pts[0]; ax.annotate(f"{br(v, 2)} s", (t, v), xytext=(-8, 0), textcoords="offset points", ha="right", va="center", fontsize=9, color=INK)
ax.set_yscale("log"); ax.set_yticks([0.1, 0.2, 0.5, 1, 2, 5]); ax.yaxis.set_major_formatter(FuncFormatter(lambda y, _: br(y, 1 if y < 1 else 0) + " s"))
ax.yaxis.set_minor_formatter(FuncFormatter(lambda y, _: "")); ax.set_ylim(0.1, 6)
ax.set_xticks(THREADS); ax.set_xlim(-1.5, TMAX + 3); ax.set_xlabel(f"threads ({CPU})")
ax.set_ylabel("tempo para totalizar 500 mil BUs (escala log)")
ax.set_title("Eleição geral, BUs já em memória: parser BER manual + vetor denso")
ax.legend(loc="upper right")
fig.tight_layout(); fig.savefig(IMG / "tempo_vs_threads.png", dpi=150); plt.close(fig)

# ------------------------------------------------------------------ Fig. 3 — todas as variantes no máximo de threads
LABEL = {
    ("rust", "manual-dense"): "Rust · parser manual + denso", ("rust", "manual-dense-nosig"): "Rust · manual + denso, BU sem assinatura por votável",
    ("rust", "manual-dense-sha512"): "Rust · manual + denso + SHA-512 de cada BU", ("rust", "rasn-dense"): "Rust · rasn (biblioteca) + denso",
    ("rust", "manual-dashmap"): "Rust · DashMap aninhado (docs originais)", ("java", "manual-dense"): "Java · parser manual + denso",
    ("java", "bc-dense"): "Java · Bouncy Castle + denso", ("java", "manual-chm"): "Java · ConcurrentHashMap aninhado",
    ("node", "manual-dense"): "Node · parser manual + denso", ("node", "asn1js-dense"): "Node · asn1js + denso",
    ("bun", "manual-dense"): "Bun · parser manual + denso",
}
def best_t(lang, var, kind="geral"):
    ts = [t for t in THREADS if med(lang, var, kind, t) is not None]
    return max(ts) if ts else None
items = []
for (lang, var), lab in LABEL.items():
    t = best_t(lang, var)
    if t is None: continue
    v = med(lang, var, "geral", t); extrap = any(r["bus"] < BUS_REF for r in rows if r["lang"] == lang and r["variant"] == var and r["threads"] == t)
    items.append((lang, lab + (f" ({t} thr.)" if t != TMAX else "") + (" *" if extrap else ""), v))
items.sort(key=lambda x: x[2])
fig, ax = plt.subplots(figsize=(9.5, 5.6))
ys = range(len(items))[::-1]
for y, (lang, lab, v) in zip(ys, items):
    ax.barh(y, v, height=0.62, color=COR[lang], edgecolor=SURFACE, linewidth=2)
    ax.text(v * 1.08, y, f"{br(v, 2)} s", va="center", fontsize=9)
ax.set_xscale("log"); ax.xaxis.set_major_formatter(FuncFormatter(lambda x, _: br(x, 2 if x < 1 else 0) + " s"))
ax.set_yticks(list(ys)); ax.set_yticklabels([i[1] for i in items], fontsize=9)
ax.set_xlabel(f"tempo para 500 mil BUs, {TMAX} threads (escala log) · * extrapolado de amostra menor")
ax.set_title("Quanto custa cada escolha de parser e de estado")
ax.grid(axis="y", visible=False); ax.set_xlim(min(i[2] for i in items) * 0.6, max(i[2] for i in items) * 3)
handles = [plt.Rectangle((0, 0), 1, 1, color=COR[k]) for k in ("rust", "java", "node", "bun") if any(i[0] == k for i in items)]
ax.legend(handles, [NOME[k] for k in ("rust", "java", "node", "bun") if any(i[0] == k for i in items)], loc="upper right")
fig.tight_layout(); fig.savefig(IMG / "variantes_max_threads.png", dpi=150); plt.close(fig)

# ------------------------------------------------------------------ Fig. 4 — aceleração (speedup)
fig, ax = plt.subplots(figsize=(9, 5))
ax.plot(THREADS, THREADS, color=MUTED, ls="--", lw=1.2, label="ideal (linear)")
for lang, var, lab in (("rust", "manual-dense", "Rust · denso"), ("java", "manual-dense", "Java · denso"), ("node", "manual-dense", "Node · denso"), ("rust", "manual-dashmap", "Rust · DashMap aninhado")):
    b = med(lang, var, "geral", 1)
    if b is None: continue
    pts = [(t, b / med(lang, var, "geral", t)) for t in THREADS if med(lang, var, "geral", t)]
    ax.plot([p[0] for p in pts], [p[1] for p in pts], marker="o", ms=6, lw=2, color=COR[lang], ls=":" if "dashmap" in var else "-", label=lab)
    ax.annotate(f"{br(pts[-1][1])}×", pts[-1], xytext=(6, 0), textcoords="offset points", va="center", fontsize=9)
ax.set_xticks(THREADS); ax.set_xlabel("threads"); ax.set_ylabel("aceleração em relação a 1 thread")
ax.set_title("Escalabilidade: aceleração em relação a 1 thread")
ax.legend(loc="upper left")
fig.tight_layout(); fig.savefig(IMG / "aceleracao.png", dpi=150); plt.close(fig)

# ------------------------------------------------------------------ Fig. 5 — memória de pico do processo (sem o corpus)
pk = []
for (lang, var), lab in LABEL.items():
    t = best_t(lang, var)
    if t is None: continue
    rr = [r for r in rows if r["lang"] == lang and r["variant"] == var and r["kind"] == "geral" and r["threads"] == t]
    # o benchmark JS mantém duas cópias do corpus (Buffer lido do disco + SharedArrayBuffer): desconta as duas
    ncopias = 2 if lang in ("node", "bun") else 1
    pk.append((lang, lab, statistics.median([r["peak_rss_mb"] - ncopias * r["corpus_mb"] for r in rr])))
pk.sort(key=lambda x: x[2])
fig, ax = plt.subplots(figsize=(9.5, 5.6))
ys = range(len(pk))[::-1]
for y, (lang, lab, v) in zip(ys, pk):
    ax.barh(y, max(v, 1), height=0.62, color=COR[lang], edgecolor=SURFACE, linewidth=2)
    ax.text(max(v, 1) + 15, y, f"{br(v, 0)} MB", va="center", fontsize=9)
ax.axvline(500, color=MUTED, ls="--", lw=1.2); ax.text(530, len(pk) - 0.6, "meta original: 500 MB", color=INK2, fontsize=8.5)
ax.axvline(150, color=MUTED, ls=":", lw=1.2); ax.text(170, -0.75, "nova meta: 150 MB", color=INK2, fontsize=8.5)
ax.set_yticks(list(ys)); ax.set_yticklabels([p[1] for p in pk], fontsize=9)
ax.set_xlabel(f"pico de memória residente do processo menos o corpus de entrada, {TMAX} threads (MB)")
ax.set_title("Memória de pico do processo durante a totalização")
ax.grid(axis="y", visible=False); ax.set_xlim(0, max(p[2] for p in pk) * 1.2)
fig.tight_layout(); fig.savefig(IMG / "memoria_pico_processo.png", dpi=150); plt.close(fig)

# ------------------------------------------------------------------ relatório (terminal + RELATORIO.md)
def fmt_s(v): return "—" if v is None else br(v, 3 if v < 10 else 1)
def tabela(cab, linhas):
    out = ["| " + " | ".join(cab) + " |", "|" + "---|" * len(cab)]
    out += ["| " + " | ".join(str(c) for c in l) + " |" for l in linhas]
    return "\n".join(out)
def pico(lang, var, kind, t):
    rr = [r for r in rows if r["lang"] == lang and r["variant"] == var and r["kind"] == kind and r["threads"] == t]
    if not rr: return None
    n = 2 if lang in ("node", "bun") else 1
    return statistics.median([r["peak_rss_mb"] - n * r["corpus_mb"] for r in rr])
LANGS = [l for l in ("rust", "java", "node", "bun") if any(r["lang"] == l and r["variant"] == "manual-dense" for r in rows)]
t_mid = min(THREADS, key=lambda t: abs(t - 8))
md = []
_dt = re.search(r"^# (\S+)", MACHINE, re.M)
md.append("# Relatório do benchmark — " + (_dt.group(1) if _dt else ""))
md.append("")
md.append("Gerado por `bench/scripts/relatorio.py`. Significado dos termos: [docs/GLOSSARIO.md](../../docs/GLOSSARIO.md). "
          "Análise completa da execução de referência: [docs/ESTIMATIVAS_MEMORIA_E_TEMPO.md](../../docs/ESTIMATIVAS_MEMORIA_E_TEMPO.md).")
md.append("")
BUS_NOTA = "" if BUS_REF == ELEICAO_BUS else (f"> **Atenção:** esta execução totalizou {BUS_REF:,} BUs por medição (modo rápido ou `UV_BUS`); ".replace(",", ".") +
            "os tempos foram escalados linearmente para 500 mil BUs. Com poucos BUs, o custo fixo (criar threads/workers, aquecer o JIT) pesa mais, "
            "principalmente no Node.js com muitas threads.\n")
if BUS_NOTA: md.append(BUS_NOTA)
md.append("## Máquina e parâmetros\n\n```\n" + MACHINE.strip() + "\n```")
md.append("\n## 1. Resumo — eleição geral, 500 mil BUs, parser manual + vetor denso\n")
md.append(tabela(["", *[NOME[l] for l in LANGS]], [
    ["1 thread", *[fmt_s(med(l, "manual-dense", "geral", 1)) + " s" for l in LANGS]],
    [f"{t_mid} threads", *[fmt_s(med(l, "manual-dense", "geral", t_mid)) + " s" for l in LANGS]],
    [f"{TMAX} threads", *[fmt_s(med(l, "manual-dense", "geral", TMAX)) + " s" for l in LANGS]],
    [f"Eleição municipal, {TMAX} threads", *[fmt_s(med(l, "manual-dense", "municipal", TMAX)) + " s" for l in LANGS]],
    ["Custo por BU numa thread", *[(br(med(l, "manual-dense", "geral", 1) / ELEICAO_BUS * 1e6, 1) + " µs") if med(l, "manual-dense", "geral", 1) else "—" for l in LANGS]],
    [f"Pico de memória sem o corpus ({TMAX} thr.)", *[(br(pico(l, "manual-dense", "geral", TMAX), 0) + " MB") if pico(l, "manual-dense", "geral", TMAX) is not None else "—" for l in LANGS]],
]))
md.append("\n## 2. Tempo (s) para 500 mil BUs por nº de threads — eleição geral\n")
md.append(tabela(["Variante", *[str(t) for t in THREADS]], [[NOME[l], *[fmt_s(med(l, "manual-dense", "geral", t)) for t in THREADS]] for l in LANGS]))
md.append("\n## 3. Todas as variantes\n")
lin = []
for (lang, var), lab in LABEL.items():
    t1, tm = med(lang, var, "geral", 1), med(lang, var, "geral", TMAX)
    if t1 is None and tm is None: continue
    ex = any(r["bus"] < BUS_REF for r in rows if r["lang"] == lang and r["variant"] == var)
    lin.append([lab + (" ¹" if ex else ""), fmt_s(t1), fmt_s(tm), br(t1 / ELEICAO_BUS * 1e6, 1) if t1 else "—",
                br(pico(lang, var, "geral", TMAX), 0) if pico(lang, var, "geral", TMAX) is not None else "—"])
lin.sort(key=lambda x: float(x[2].replace(".", "").replace(",", ".")) if x[2] != "—" else 1e9)
md.append(tabela(["Variante", "1 thread (s)", f"{TMAX} threads (s)", "µs por BU (1 thr.)", f"Pico − corpus, {TMAX} thr. (MB)"], lin))
md.append(f"\n¹ medida com menos BUs que as demais ({BUS_REF:,} nesta execução) e extrapolada linearmente para 500 mil.".replace(",", "."))
md.append("\n## 4. Memória só do estado de totalização (eleição geral)\n")
md.append(tabela(["Desenho", "Linguagem", "MB"], [[lab, NOME[lang], br(v)] for lang, lab, v in mem]))
inval = sum(int(r["invalid"]) for r in rows)
md.append(f"\n**BUs inválidos em todas as medições:** {inval} " + ("(correto)" if inval == 0 else "(**ATENÇÃO: deveria ser 0**)"))
md.append("\n## 5. Gráficos\n")
for f, t in (("tempo_vs_threads.png", "Tempo por nº de threads"), ("aceleracao.png", "Aceleração"), ("variantes_max_threads.png", "Todas as variantes"),
             ("memoria_estado.png", "Memória do estado"), ("memoria_pico_processo.png", "Memória de pico do processo")):
    md.append(f"### {t}\n\n![{t}]({pathlib.Path(IMG).resolve().relative_to(RES.resolve()) if str(IMG.resolve()).startswith(str(RES.resolve())) else IMG}/{f})\n")
texto = "\n".join(md) + "\n"
(RES / "RELATORIO.md").write_text(texto)
corte = texto.split("## 5. Gráficos")[0]
print(re.sub(r"^## ", "\n", corte, flags=re.M))
print(f"Gráficos em: {IMG}\nRelatório:   {RES / 'RELATORIO.md'}")
