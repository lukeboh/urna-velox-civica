#!/usr/bin/env python3
"""Passo 8 do teste real — lê os resultados da aquisição, imprime as tabelas, grava
RELATORIO.md e os gráficos em <resultados>/graficos/.

Uso:  python3 aquisicao/scripts/relatorio.py [--resultados aquisicao/results] [--sintetico bench/results/time.csv]
Requer matplotlib. Cores: paleta categórica de referência do projeto (mesma de bench/), em ordem fixa;
toda série tem rótulo direto além da cor.
"""
import argparse, csv, datetime as dt, pathlib, re, statistics, collections

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib.ticker import FuncFormatter

AQ = pathlib.Path(__file__).resolve().parents[1]
ap = argparse.ArgumentParser()
ap.add_argument("--resultados", default=str(AQ / "results"))
ap.add_argument("--sintetico", default=str(AQ.parent / "bench" / "results" / "time.csv"))
ap.add_argument("--img", default=None)
a = ap.parse_args()
RES = pathlib.Path(a.resultados)
IMG = pathlib.Path(a.img) if a.img else RES / "graficos"
IMG.mkdir(parents=True, exist_ok=True)

SURFACE, INK, INK2, MUTED, GRID = "#fcfcfb", "#0b0b0b", "#52514e", "#8a8984", "#e6e5e1"
AZUL, LARANJA, VERDE, AMARELO = "#2a78d6", "#eb6834", "#1baf7a", "#eda100"
plt.rcParams.update({
    "figure.facecolor": SURFACE, "axes.facecolor": SURFACE, "savefig.facecolor": SURFACE,
    "axes.edgecolor": GRID, "axes.labelcolor": INK2, "xtick.color": INK2, "ytick.color": INK2,
    "text.color": INK, "font.size": 10, "axes.titlesize": 12, "axes.titleweight": "bold",
    "axes.spines.top": False, "axes.spines.right": False, "axes.grid": True, "grid.color": GRID,
    "grid.linewidth": 0.8, "axes.axisbelow": True, "legend.frameon": False,
})


def br(x, nd=1):
    return f"{x:,.{nd}f}".replace(",", "X").replace(".", ",").replace("X", ".")


def ler(nome):
    p = RES / nome
    return list(csv.DictReader(open(p))) if p.exists() else []


def num(r, k):
    try:
        return float(r[k])
    except (KeyError, ValueError, TypeError):
        return 0.0


MACHINE = (RES / "machine.txt").read_text(errors="ignore") if (RES / "machine.txt").exists() else ""
_m = re.search(r"Model name:\s*(.+)", MACHINE)
CPU = _m.group(1).strip() if _m else "CPU não identificada"
_s = re.search(r"total: (\d+) seções", (RES / "secoes.txt").read_text() if (RES / "secoes.txt").exists() else "")
SECOES_BR = int(_s.group(1)) if _s else 517_179

resumo = ler("coleta_resumo.csv")
for r in resumo:
    for k in ("secoes", "ok", "erros", "concorrencia", "conexoes"):
        r[k] = int(float(r[k] or 0))
    for k in ("segundos", "secoes_por_s", "req_por_s", "mb", "mb_por_s", "lat_aux_p50_ms", "lat_bu_p50_ms", "lat_p95_ms", "lat_p99_ms", "rps_limite"):
        r[k] = num(r, k)

md = ["# Aquisição de BUs reais — relatório da última execução", "",
      f"Máquina: {CPU}. Servidor: resultados.tse.jus.br (CDN Akamai). "
      f"Pleito 3220 (1º turno de 2026), {br(SECOES_BR, 0)} seções no Brasil.", ""]
figs = []

# ------------------------------------------------------------------ 1. varredura de concorrência
conc = sorted([r for r in resumo if r["rotulo"] == "conc"], key=lambda r: r["concorrencia"])
h1 = sorted([r for r in resumo if r["rotulo"] == "http1"], key=lambda r: r["concorrencia"])
cx = sorted([r for r in resumo if r["rotulo"] == "conexoes"], key=lambda r: r["conexoes"])
if conc:
    md += ["## 1. Vazão da coleta pela API (aux.json + bu.dat)", "",
           "Cada linha é uma amostra diferente de seções de SP, MG, BA e RJ (sem repetir objetos já em cache no CDN). "
           "Uma seção custa 2 requisições.", "",
           "| rótulo | concorrência | conexões | protocolo | seções | seções/s | req/s | MB/s | lat. p50 aux | lat. p50 BU | p95 | p99 | 404 / erros | 429 |",
           "|---|---:|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|"]
    for r in conc + h1 + cx:
        md.append(f"| {r['rotulo']} | {r['concorrencia']} | {r['conexoes']} | {r['http']} | {r['secoes']} | {br(r['secoes_por_s'])} | {br(r['req_por_s'])} | "
                  f"{br(r['mb_por_s'], 2)} | {br(r['lat_aux_p50_ms'], 0)} ms | {br(r['lat_bu_p50_ms'], 0)} ms | {br(r['lat_p95_ms'], 0)} ms | {br(r['lat_p99_ms'], 0)} ms | {r['erros']} | {r['http429']} |")
    md.append("")
    # lei de Little: seções/s = concorrência ÷ tempo por seção (medido nas concorrências baixas, longe do teto)
    t_sec = statistics.median([r["concorrencia"] / r["secoes_por_s"] for r in conc if r["concorrencia"] <= 16 and r["secoes_por_s"] > 0])
    teto = min(r["rps_limite"] for r in conc) / 2 if conc else 50

    fig, ax = plt.subplots(figsize=(8.2, 4.6))
    xs = [r["concorrencia"] for r in conc]
    xx = [min(xs), max(xs) * 1.05]
    ax.plot(xx, [x / t_sec for x in xx], color=MUTED, lw=1.2, ls=(0, (4, 3)))
    ax.text(xs[1] * 1.15, xs[1] * 1.15 / t_sec * 0.62, f"lei de Little: concorrência ÷ {br(t_sec, 2)} s por seção", color=INK2, fontsize=8.5)
    ax.axhline(teto, color=MUTED, lw=1.0)
    ax.text(xs[0], teto * 1.06, f"teto do coletor: {br(teto * 2, 0)} req/s = {br(teto, 0)} seções/s", color=INK2, fontsize=8.5)
    ax.plot(xs, [r["secoes_por_s"] for r in conc], color=AZUL, lw=2, marker="o", ms=7, mec=SURFACE, mew=2)
    ax.annotate(f"HTTP/2: {br(conc[-1]['secoes_por_s'], 0)}/s", (xs[-1], conc[-1]["secoes_por_s"]), textcoords="offset points", xytext=(8, -4), ha="left", color=INK, fontsize=9)
    ax.set_xlim(min(xs) / 1.3, max(xs) * 2.4)
    if h1:
        ax.plot([r["concorrencia"] for r in h1], [r["secoes_por_s"] for r in h1], color=LARANJA, lw=0, marker="s", ms=8, mec=SURFACE, mew=2)
        ax.annotate("HTTP/1.1", (h1[-1]["concorrencia"], h1[-1]["secoes_por_s"]), textcoords="offset points", xytext=(-10, 6), ha="right", color=INK, fontsize=9)
    ax.set_xscale("log", base=2); ax.set_yscale("log")
    ax.xaxis.set_major_formatter(FuncFormatter(lambda v, _: f"{int(v)}"))
    ax.yaxis.set_major_formatter(FuncFormatter(lambda v, _: br(v, 0)))
    ax.yaxis.set_minor_formatter(FuncFormatter(lambda v, _: ""))
    ax.set_xlabel("requisições em andamento (concorrência)"); ax.set_ylabel("seções (BUs) por segundo")
    ax.set_title("Vazão da coleta: linear na concorrência até o teto de 100 req/s", loc="left")
    fig.tight_layout(); fig.savefig(IMG / "vazao_concorrencia.png", dpi=150); plt.close(fig)
    figs.append(("vazao_concorrencia.png", "Vazão × concorrência"))

    fig, ax = plt.subplots(figsize=(8.2, 4.2))
    ax.plot(xs, [r["lat_aux_p50_ms"] for r in conc], color=AZUL, lw=2, marker="o", ms=7, mec=SURFACE, mew=2)
    ax.plot(xs, [r["lat_p95_ms"] for r in conc], color=LARANJA, lw=2, marker="o", ms=7, mec=SURFACE, mew=2)
    ax.annotate("mediana (aux.json)", (xs[-1], conc[-1]["lat_aux_p50_ms"]), textcoords="offset points", xytext=(6, -4), fontsize=9)
    ax.annotate("percentil 95", (xs[-1], conc[-1]["lat_p95_ms"]), textcoords="offset points", xytext=(6, -4), fontsize=9)
    ax.set_xscale("log", base=2); ax.xaxis.set_major_formatter(FuncFormatter(lambda v, _: f"{int(v)}"))
    ax.set_ylim(0, max(r["lat_p95_ms"] for r in conc) * 1.25)
    ax.set_xlabel("requisições em andamento (concorrência)"); ax.set_ylabel("latência por requisição (ms)")
    ax.set_title("Latência por requisição × concorrência", loc="left")
    ax.set_xlim(min(xs) / 1.3, max(xs) * 2.2)
    fig.tight_layout(); fig.savefig(IMG / "latencia_concorrencia.png", dpi=150); plt.close(fig)
    figs.append(("latencia_concorrencia.png", "Latência × concorrência"))

# ------------------------------------------------------------------ 2. modos de aquisição da UF de referência
modos = {r["rotulo"]: r for r in resumo if r["rotulo"] in ("uf-completa", "zip-faixas", "zip-completo", "fluxo", "acompanhar")}
if "uf-completa" in modos:
    uf = modos["uf-completa"]["ufs"].upper()
    md += [f"## 2. Caminhos de aquisição — {uf} completo", "",
           "| caminho | BUs | tempo | BUs/s | requisições/s | MB transferidos | observação |", "|---|---:|---:|---:|---:|---:|---|"]
    obs = {"uf-completa": "API de resultados (aux.json + bu.dat), gravando o pacote .uvpk",
           "zip-faixas": "zip de dados abertos: diretório central + 1 faixa HTTP Range por BU",
           "zip-completo": "zip de dados abertos inteiro + extração local",
           "fluxo": "API com totalização simultânea (objetos já em cache no CDN)",
           "acompanhar": "3 rodadas de cs.json com If-None-Match depois da coleta"}
    for k in ("uf-completa", "zip-faixas", "zip-completo", "fluxo", "acompanhar"):
        if k in modos:
            r = modos[k]
            md.append(f"| {k} | {r['ok']} | {br(r['segundos'], 1)} s | {br(r['secoes_por_s'], 0)} | {br(r['req_por_s'], 0)} | {br(r['mb'], 1)} | {obs[k]} |")
    if modos.get("fluxo", {}).get("lag_totalizacao_ms"):
        md.append(f"\nNa coleta em fluxo, a totalização terminou **{modos['fluxo']['lag_totalizacao_ms']} ms** depois do último BU recebido.")
    md.append("")
    ks = [k for k in ("uf-completa", "zip-faixas", "zip-completo") if k in modos]
    nomes = {"uf-completa": "API (aux + BU)", "zip-faixas": "zip, só os BUs (Range)", "zip-completo": "zip inteiro"}
    fig, axs = plt.subplots(1, 2, figsize=(9.6, 3.2))
    for ax, campo, rot, fmt in ((axs[0], "segundos", "tempo para ter todos os BUs (s)", 1), (axs[1], "mb", "dados transferidos (MB)", 0)):
        vals = [modos[k][campo] for k in ks]
        ax.barh(range(len(ks)), vals, color=[AZUL, VERDE, LARANJA][:len(ks)], height=0.55, edgecolor=SURFACE, linewidth=2)
        for i, v in enumerate(vals):
            ax.text(v, i, f"  {br(v, fmt)}", va="center", fontsize=9, color=INK)
        ax.set_yticks(range(len(ks)), [nomes[k] for k in ks] if ax is axs[0] else [""] * len(ks))
        ax.invert_yaxis(); ax.set_xlabel(rot); ax.grid(axis="y", visible=False)
        ax.set_xlim(0, max(vals) * 1.3)
    fig.suptitle(f"{uf}: três caminhos para obter os mesmos BUs", x=0.01, ha="left", fontweight="bold")
    fig.tight_layout(); fig.savefig(IMG / "caminhos_aquisicao.png", dpi=150); plt.close(fig)
    figs.append(("caminhos_aquisicao.png", "Caminhos de aquisição"))

# ------------------------------------------------------------------ 3. curva de chegada dos BUs ao TSE (noite da eleição)
def curva(caminho):
    t = []
    if not caminho.exists():
        return t
    for r in csv.DictReader(open(caminho)):
        s = r.get("recebido_tse", "").strip()
        if s and not r.get("erro"):
            try:
                t.append(dt.datetime.strptime(s, "%d/%m/%Y %H:%M:%S"))
            except ValueError:
                pass
    return sorted(t)

curvas = []
for p in sorted(RES.glob("metricas_*.csv")):
    c = curva(p)
    if c:
        curvas.append((p.stem.split("_", 1)[1].upper(), c))
ROT_SUST = "nacional" if (RES / "chegada_nacional_por_minuto.csv").exists() else "multi"
nac = RES / f"chegada_{ROT_SUST}_por_minuto.csv"
sust = [r for r in resumo if r["rotulo"] == ROT_SUST]
NOME_SUST = "Brasil" if ROT_SUST == "nacional" else (sust[-1]["ufs"].upper().replace("+", ", ") if sust else "UFs")
cap = sust[-1]["secoes_por_s"] if sust else max([r["secoes_por_s"] for r in resumo if r["rotulo"] == "conc"] or [0])
if nac.exists():
    rows = list(csv.DictReader(open(nac)))
    tn = [dt.datetime.strptime(r["minuto"], "%Y-%m-%d %H:%M") for r in rows]
    qn = [int(r["bus"]) for r in rows]
    tot = sum(qn); acc = 0; cum = []
    for q in qn:
        acc += q; cum.append(100 * acc / tot)
    pico = max(qn); tpico = tn[qn.index(pico)]
    md += [f"## 3. Chegada dos BUs ao TSE na noite da eleição ({NOME_SUST})", "",
           f"Do campo de recebimento (`dr`/`hr`) de cada aux.json: {br(tot, 0)} BUs. Pico: **{br(pico, 0)} BUs/min** "
           f"({br(pico / 60, 1)}/s) às {tpico:%H:%M}. Vazão sustentada do coletor (teto de 100 req/s): **{br(cap, 0)} seções/s = {br(cap * 60, 0)}/min**, "
           f"ou {br(cap * 60 / pico, 1)} × o pico destas UFs.", ""]
    for marco in (50, 90, 99):
        i = next(i for i, v in enumerate(cum) if v >= marco)
        md.append(f"* {marco} % dos BUs recebidos até {tn[i]:%H:%M}")
    md.append("")
    # estimativa grosseira para o Brasil: amostra (UFs coletadas) escalada pelo nº de seções listadas
    sec_uf = {}
    for linha in ((RES / "secoes.txt").read_text().splitlines() if (RES / "secoes.txt").exists() else []):
        p_ = linha.split()
        if len(p_) >= 3 and len(p_[0]) == 2 and p_[2].isdigit():
            sec_uf[p_[0]] = int(p_[2])
    amostra = [u.lower() for u in (sust[-1]["ufs"].split("+") if sust else [])]
    ex_ = collections.Counter({t: q for t, q in zip(tn, qn)})
    dfm = RES / "metricas_df.csv"
    if dfm.exists() and "df" not in amostra:
        amostra.append("df")
        for t in curva(dfm):
            ex_[t.replace(second=0)] += 1
    fator = SECOES_BR / max(1, sum(sec_uf.get(u, 0) for u in amostra))
    tb = sorted(ex_); acc = 0; cb = []
    for t in tb:
        acc += ex_[t] * fator; cb.append(acc)
    tot_br = cb[-1]
    t99 = tb[next(i for i, v in enumerate(cb) if v >= 0.99 * tot_br)]
    t_ini = tb[0]
    # coletor começando com o primeiro BU: a cada minuto coleta até cap×60 seções, sem passar do que já chegou
    xs_c, ys_c = [t_ini], [0.0]
    passo = dt.timedelta(minutes=1)
    t = t_ini; feito = 0.0; j = 0
    while feito < tot_br - 1 and t < t_ini + dt.timedelta(hours=8):
        t += passo
        while j < len(tb) and tb[j] <= t:
            j += 1
        chegou = cb[j - 1] if j else 0
        feito = min(chegou, feito + cap * 60)
        xs_c.append(t); ys_c.append(feito)
    # atraso de cada BU = quando o coletor chega ao mesmo acumulado − quando o BU chegou ao TSE
    def quando(xs_, ys_, y):
        for x_, v_ in zip(xs_, ys_):
            if v_ >= y:
                return x_
        return xs_[-1]
    niveis = [tot_br * f / 100 for f in range(1, 100)]
    atrasos = [(quando(xs_c, ys_c, y) - quando(tb, cb, y)).total_seconds() / 60 for y in niveis]
    atraso_max = max(atrasos)
    folga = [i for i in range(1, len(xs_c)) if ys_c[i] - ys_c[i - 1] < cap * 60 * 0.99 and ys_c[i] > tot_br * 0.5]
    t_alc = xs_c[folga[0]] if folga else xs_c[-1]
    md += [f"Estimativa grosseira para o Brasil (amostra {', '.join(u.upper() for u in amostra)} escalada × {br(fator, 1)} pelo nº de seções): "
           f"99 % dos BUs chegariam até {t99:%H:%M}. Um coletor a 100 req/s ({br(cap, 0)} seções/s), começando no primeiro BU, fica atrás da chegada "
           f"no pico, com **atraso máximo de ~{br(atraso_max, 0)} min**, e alcança a chegada às **{t_alc:%H:%M}**; depois disso, cada BU é coletado quase assim que é publicado.", ""]
    fig, ax = plt.subplots(figsize=(8.6, 4.2))
    ax.plot(tb, cb, color=AZUL, lw=2)
    ax.plot(xs_c, ys_c, color=LARANJA, lw=2)
    k = len(tb) // 3
    ax.annotate("BUs recebidos pelo TSE (estimativa Brasil)", (tb[k], cb[k]), textcoords="offset points", xytext=(8, 4), fontsize=9)
    m_ = len(xs_c) // 2
    ax.annotate(f"coletados por 1 coletor a 100 req/s (alcança às {t_alc:%H:%M}; atraso máx. ~{br(atraso_max, 0)} min)", (xs_c[m_ // 2], ys_c[m_ // 2]), textcoords="offset points", xytext=(10, -14), fontsize=9)
    ax.set_xlim(tb[0], max(t99, t_alc) + dt.timedelta(minutes=40))
    ax.yaxis.set_major_formatter(FuncFormatter(lambda v, _: br(v / 1000, 0) + " mil"))
    ax.xaxis.set_major_formatter(matplotlib.dates.DateFormatter("%H:%M"))
    ax.set_ylabel("BUs acumulados")
    ax.set_title("Noite da eleição: chegada dos BUs × coleta a 100 req/s (horário de Brasília)", loc="left")
    fig.tight_layout(); fig.savefig(IMG / "ritmo_chegada.png", dpi=150); plt.close(fig)
    figs.append(("ritmo_chegada.png", "Chegada dos BUs × coleta a 100 req/s"))
    curvas.insert(0, (NOME_SUST, None))
if curvas:
    fig, ax = plt.subplots(figsize=(8.6, 4.2))
    cores = [AZUL, VERDE, LARANJA, AMARELO]
    for i, (nome, c) in enumerate(curvas[:4]):
        if c is None:
            xs, ys = tn, cum
        else:
            xs = c; ys = [100 * (j + 1) / len(c) for j in range(len(c))]
        ax.plot(xs, ys, color=cores[i], lw=2)
        k = next(j for j, v in enumerate(ys) if v >= 50)
        ax.annotate(nome, (xs[k], ys[k]), textcoords="offset points", xytext=(6, -10), fontsize=9)
    ax.set_ylim(0, 101); ax.set_ylabel("% dos BUs já recebidos pelo TSE")
    ax.xaxis.set_major_formatter(matplotlib.dates.DateFormatter("%H:%M"))
    lim = curvas[-1][1] if curvas[-1][1] else tn
    ax.set_title("Curva de chegada dos BUs (horário de Brasília)", loc="left")
    fig.tight_layout(); fig.savefig(IMG / "curva_chegada.png", dpi=150); plt.close(fig)
    figs.append(("curva_chegada.png", "Curva de chegada"))

# ------------------------------------------------------------------ 4. coleta nacional
nrow = sust
ps = RES / f"coleta_{ROT_SUST}_por_segundo.csv"
if nrow:
    r = nrow[-1]
    md += [f"## 4. Coleta sustentada — {NOME_SUST}", "",
           f"{br(r['ok'], 0)} BUs em **{br(r['segundos'] / 60, 1)} min** ({br(r['secoes_por_s'], 0)} seções/s, {br(r['req_por_s'], 0)} req/s, "
           f"{br(r['mb'] / 1000, 2)} GB, {br(r['mb_por_s'], 1)} MB/s); concorrência {r['concorrencia']}, {r['conexoes']} conexões {r['http']}; "
           f"novas tentativas {r['tentativas_extras']}, HTTP 429: {r['http429']}, seções sem BU/erros: {r['erros']}. "
           f"Projeção para o Brasil ({br(SECOES_BR, 0)} seções) neste ritmo: **{br(SECOES_BR / r['secoes_por_s'] / 3600, 1)} h**.", ""]
if ps.exists():
    rows = list(csv.DictReader(open(ps)))
    s = [int(x["segundo"]) for x in rows]; q = [int(x["secoes"]) for x in rows]
    acc = 0; cum = []
    for v in q:
        acc += v; cum.append(acc)
    fig, ax = plt.subplots(figsize=(8.6, 3.8))
    ax.plot([x / 60 for x in s], cum, color=AZUL, lw=2)
    ax.annotate(f"{br(cum[-1], 0)} BUs em {br(s[-1] / 60, 1)} min", (s[-1] / 60, cum[-1]), textcoords="offset points", xytext=(-6, -16), ha="right", fontsize=9)
    ax.yaxis.set_major_formatter(FuncFormatter(lambda v, _: br(v / 1000, 0) + " mil"))
    ax.set_xlabel("minutos desde o início"); ax.set_ylabel("BUs coletados")
    ax.set_title(f"Coleta sustentada ({NOME_SUST}): BUs acumulados", loc="left")
    fig.tight_layout(); fig.savefig(IMG / "coleta_sustentada.png", dpi=150); plt.close(fig)
    figs.append(("coleta_sustentada.png", "Coleta sustentada"))
    tx = RES / f"taxa_{ROT_SUST}.csv"
    if tx.exists():
        tr = list(csv.DictReader(open(tx)))
        tt = [float(x["t_s"]) for x in tr] + [s[-1]]; rr = [float(x["rps"]) for x in tr]; rr.append(rr[-1])
        fig, ax = plt.subplots(figsize=(8.6, 3.8))
        ini = RES / f"inicios_{ROT_SUST}.csv"
        if ini.exists():
            ir = list(csv.DictReader(open(ini)))
            si = [int(x["segundo"]) for x in ir]; ni = [int(x["requisicoes"]) for x in ir]
            ax.bar(si, ni, width=1.0, color=AZUL, linewidth=0)
            md.append(f"Requisições iniciadas por segundo na coleta sustentada: máximo **{max(ni)}**, média {br(statistics.mean(ni), 1)} (limite 100).\n")
            ax.text(si[len(si) // 2], 50, f"requisições iniciadas em cada segundo (máx. {max(ni)})", ha="center", fontsize=9.5, color=SURFACE, fontweight="bold")
        ax.step(tt, rr, where="post", color=LARANJA, lw=2)
        ax.axhline(100, color=INK2, lw=1, ls=(0, (4, 3)))
        ax.text(s[-1], 102, "limite: 100 req/s", ha="right", fontsize=8.5, color=INK2)
        ax.annotate("teto do controle adaptativo", (tt[min(20, len(tt) - 1)], rr[min(20, len(rr) - 1)]), textcoords="offset points", xytext=(10, -4), fontsize=9)
        ax.set_ylim(0, 112); ax.set_xlabel("segundos desde o início"); ax.set_ylabel("requisições por segundo"); ax.grid(axis="x", visible=False)
        ax.set_title("Controle adaptativo (AIMD): partida em 40 req/s, subida até o limite de 100", loc="left")
        fig.tight_layout(); fig.savefig(IMG / "taxa_adaptativa.png", dpi=150); plt.close(fig)
        figs.append(("taxa_adaptativa.png", "Controle adaptativo de taxa"))

# ------------------------------------------------------------------ 5. totalização dos BUs reais × sintéticos
tot = ler("totalizacao.csv")
for r in tot:
    r["threads"] = int(r["threads"]); r["bus"] = int(r["bus"]); r["seconds"] = float(r["seconds"])
if tot:
    md += ["## 5. Totalização dos BUs reais (BUs já em memória)", "",
           "| conjunto | threads | BUs | mediana (ms) | BUs/s | µs/BU·thread | consolidação UF (ms) | cargos inválidos | pico RSS (MB) | RSS anônimo (MB) |",
           "|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|"]
    grupos = collections.defaultdict(list)
    for r in tot:
        grupos[(r["rotulo"], r["threads"])].append(r)
    serie = collections.defaultdict(list)
    for (rot, t), v in sorted(grupos.items()):
        sec = statistics.median(x["seconds"] for x in v); n = v[0]["bus"]
        serie[rot].append((t, n / sec))
        md.append(f"| {rot} | {t} | {br(n, 0)} | {br(sec * 1000, 1)} | {br(n / sec, 0)} | {br(sec / n * 1e6 * t, 2)} | {br(statistics.median(float(x['consolidate_ms']) for x in v), 1)} | "
                  f"{v[0]['invalid']} | {br(max(float(x['peak_rss_mb']) for x in v), 0)} | {br(max(float(x.get('anon_rss_mb') or 0) for x in v), 0)} |")
    md.append("")
    sint = []
    p = pathlib.Path(a.sintetico)
    if p.exists():
        g = collections.defaultdict(list)
        for r in csv.DictReader(open(p)):
            if r["lang"] == "rust" and r["variant"] == "manual-dense" and r["kind"] == "geral":
                g[int(r["threads"])].append(int(r["bus"]) / float(r["seconds"]))
        sint = sorted((t, statistics.median(v)) for t, v in g.items())
    fig, ax = plt.subplots(figsize=(8.2, 4.4))
    cores = {"todas": AZUL}
    allx = []
    for i, (rot, pts) in enumerate(sorted(serie.items(), key=lambda kv: kv[0] != "todas")):
        cor = AZUL if rot == "todas" else VERDE
        xs, ys = zip(*pts); allx += xs
        ax.plot(xs, ys, color=cor, lw=2, marker="o", ms=7, mec=SURFACE, mew=2)
        n_rot = grupos[(rot, xs[0])][0]["bus"]
        nome = f"BUs reais — todos os coletados ({br(n_rot / 1000, 1)} mil)" if rot == "todas" else f"BUs reais — {rot.upper()} ({br(n_rot / 1000, 1)} mil)"
        ax.annotate(nome, (xs[-1], ys[-1]), textcoords="offset points", xytext=(6, -4), fontsize=9)
    if sint:
        xs, ys = zip(*[x for x in sint if x[0] <= max(allx)])
        ax.plot(xs, ys, color=LARANJA, lw=2, marker="s", ms=7, mec=SURFACE, mew=2)
        ax.annotate("BUs sintéticos (bench/)", (xs[-1], ys[-1]), textcoords="offset points", xytext=(6, -4), fontsize=9)
    ax.set_xscale("log", base=2)
    ax.xaxis.set_major_formatter(FuncFormatter(lambda v, _: f"{int(v)}"))
    ax.yaxis.set_major_formatter(FuncFormatter(lambda v, _: br(v / 1e6, 1) + " mi"))
    ax.set_ylim(0, None); ax.set_xlim(0.8, max(allx) * 3)
    ax.set_xlabel("threads"); ax.set_ylabel("BUs totalizados por segundo")
    ax.set_title("Totalização: BUs reais × BUs sintéticos do benchmark", loc="left")
    fig.tight_layout(); fig.savefig(IMG / "totalizacao_real_sintetico.png", dpi=150); plt.close(fig)
    figs.append(("totalizacao_real_sintetico.png", "Totalização real × sintética"))

    # ponta a ponta: coleta (medida ou projetada) + totalização (medida)
    best = {rot: max(v for _, v in pts) for rot, pts in serie.items()}
    taxa_tot = best.get("todas") or max(best.values())
    taxa_col = nrow[-1]["secoes_por_s"] if nrow else cap
    t_col = SECOES_BR / taxa_col if taxa_col else 0
    t_tot = SECOES_BR / taxa_tot
    md += ["## 6. Ponta a ponta (eleição inteira)", "",
           f"* coleta de {br(SECOES_BR, 0)} seções por um IP, a ≤ 100 req/s: **{br(t_col / 3600, 1)} h** ({'medido' if ROT_SUST == 'nacional' else 'projetado da coleta sustentada'}, {br(taxa_col, 0)} seções/s)",
           f"* totalização dos mesmos BUs: **{br(t_tot, 2)} s** (projetado da melhor taxa medida com BUs reais, {br(taxa_tot, 0)} BUs/s; o benchmark sintético mediu 0,17–0,2 s para 500 mil BUs)",
           f"* a coleta é **{br(t_col / t_tot, 0)} ×** mais demorada que a totalização: o gargalo é a rede, e a totalização em fluxo termina "
           "praticamente junto com o último BU.", ""]
    fig, ax = plt.subplots(figsize=(8.6, 2.6))
    ax.barh([0, 1], [t_col, t_tot], color=[AZUL, VERDE], height=0.55, edgecolor=SURFACE, linewidth=2)
    ax.text(t_col, 0, f"  {br(t_col / 3600, 1)} h", va="center", fontsize=9)
    ax.text(t_tot, 1, f"  {br(t_tot, 2)} s", va="center", fontsize=9)
    ax.set_yticks([0, 1], ["coleta (rede)", "totalização (CPU)"]); ax.invert_yaxis(); ax.grid(axis="y", visible=False)
    ax.set_xlabel("horas para a eleição inteira"); ax.set_xlim(0, t_col * 1.2)
    ax.set_xticks([h * 1800 for h in range(int(t_col * 1.2 / 1800) + 1)])
    ax.xaxis.set_major_formatter(FuncFormatter(lambda v, _: br(v / 3600, 1)))
    ax.set_title(f"Eleição inteira ({br(SECOES_BR, 0)} seções): a rede domina o tempo", loc="left")
    fig.tight_layout(); fig.savefig(IMG / "ponta_a_ponta.png", dpi=150); plt.close(fig)
    figs.append(("ponta_a_ponta.png", "Ponta a ponta"))

conf = RES / "conferencia.txt"
if conf.exists():
    md += ["## 7. Conferência com decodificador independente", "", "```", conf.read_text().strip(), "```", ""]
for nome in ("catalogo.txt", "totalizacao_todas.txt"):
    p = RES / nome
    if p.exists():
        md += [f"### {nome}", "", "```", p.read_text().strip()[-3000:], "```", ""]
md += ["## Gráficos", ""] + [f"![{t}](graficos/{f})" for f, t in figs] + [""]
(RES / "RELATORIO.md").write_text("\n".join(md))
print("\n".join(l for l in md if not l.startswith("![")))
print(f"\nRelatório: {RES / 'RELATORIO.md'}  ({len(figs)} gráficos em {IMG})")
