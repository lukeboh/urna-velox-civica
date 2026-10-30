#!/usr/bin/env python3
"""Gráficos da análise de uma noite (PNG): chegada ao TSE nas duas noites, seções aplicadas e fila.

  python3 graficos.py --pleito <pasta> --saida docs/img [--pausas B=19:47-19:57,A=19:53-20:03,A=21:00-21:10]
                      [--marco 'TSE: matematicamente eleito ~21:09=21:09'] [--marco 'Urna Velox 21:23=21:23']

Lê <pleito>/analise/chegada_turnos.csv (turnos.py) e minuto_a_minuto.csv (curvas.py). Precisa do matplotlib.
Os títulos dos gráficos são os da noite de 25/10/2026; para outra noite, troque-os aqui."""
import argparse, csv, datetime as dt, os
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
import matplotlib.dates as mdates

ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
ap.add_argument("--pleito", required=True)
ap.add_argument("--saida", required=True)
ap.add_argument("--pausas", default="B=19:47-19:57,A=19:53-20:03,A=21:00-21:10")
ap.add_argument("--marco", action="append", default=[])
a = ap.parse_args()
S = os.path.join(a.pleito, "analise")
data = os.path.basename(os.path.normpath(a.pleito))[:10]
pref = os.path.join(a.saida, f"analise_{data}_")
CINZA, AZUL, LARANJA = "#4a4a48", "#2a78d6", "#eb6834"
TXT, TXT2, GRADE = "#1f1f1e", "#5f5e5a", "#e4e2dc"
plt.rcParams.update({"font.size": 10.5, "axes.edgecolor": GRADE, "axes.labelcolor": TXT2, "xtick.color": TXT2,
                     "ytick.color": TXT2, "axes.spines.top": False, "axes.spines.right": False})
pausas = [(p.split("=")[0], *p.split("=")[1].split("-")) for p in a.pausas.split(",") if p]
d = lambda h: dt.datetime.strptime(f"{data} {h}", "%Y-%m-%d %H:%M")

# 1) chegada ao TSE: noite anterior × esta
L = list(csv.DictReader(open(os.path.join(S, "chegada_turnos.csv"))))
x = [int(r["min_desde_17h"]) / 60 + 17 for r in L]
fig, ax = plt.subplots(figsize=(9, 4.4), dpi=150)
ax.plot(x, [float(r["anterior_pct_acum"]) for r in L], color=AZUL, lw=2, label="1º turno (04/10)")
ax.plot(x, [float(r["pleito_pct_acum"]) for r in L], color=LARANJA, lw=2, label="2º turno (25/10)")
ax.axvspan(17 + 9 / 60, 19 + 8 / 60, color=LARANJA, alpha=0.08, lw=0)
ax.text(17.2, 80, "2º turno: votação prorrogada até 19h;\nnenhum BU recebido de 17:09 a 19:08", color=TXT, ha="left", fontsize=10)
ax.text(18.95, 62, "1º turno", color=AZUL, fontsize=10, ha="right"); ax.text(20.05, 25, "2º turno", color=LARANJA, fontsize=10)
ax.set_xlim(17, 22.5); ax.set_ylim(0, 101)
ax.set_xticks([17, 18, 19, 20, 21, 22]); ax.set_xticklabels(["17h", "18h", "19h", "20h", "21h", "22h"])
ax.set_ylabel("BUs recebidos pelo TSE (% acumulado)"); ax.grid(axis="y", color=GRADE, lw=0.8)
ax.set_title("No 2º turno os BUs chegaram 2 h mais tarde e quase todos em 2 h", loc="left",
             color=TXT, fontsize=12, fontweight="bold")
ax.legend(frameon=False, loc="lower right")
fig.tight_layout(); fig.savefig(pref + "chegada_turnos.png"); plt.close(fig)

# 2) seções aplicadas: recebimento × espelho × produção (19h–21h45)
M = [r for r in csv.DictReader(open(os.path.join(S, "minuto_a_minuto.csv")))
     if r["minuto"].startswith(data) and "19:00" <= r["minuto"][11:] <= "21:45"]
t = [dt.datetime.strptime(r["minuto"], "%Y-%m-%d %H:%M") for r in M]
T = int(list(csv.DictReader(open(os.path.join(S, "minuto_a_minuto.csv"))))[-1]["recebido_acum"])
prod = "aplicado_A_acum"
fig, ax = plt.subplots(figsize=(9, 4.8), dpi=150)
ax.plot(t, [100 * int(r["recebido_acum"]) / T for r in M], color=CINZA, lw=2, label="recebido pelo TSE (aux.json)")
if "aplicado_L_acum" in M[0]:
    ax.plot(t, [100 * int(r["aplicado_L_acum"]) / T for r in M], color=AZUL, lw=2, label="Urna Velox, validação no espelho (sem teto)")
ax.plot(t, [100 * int(r[prod]) / T for r in M], color=LARANJA, lw=2, label="Urna Velox em produção (nós A e B, borda)")
for _, i, f in pausas:
    ax.axvspan(d(i), d(f), color=LARANJA, alpha=0.10, lw=0)
ax.text(d("19:55"), 40, "pausas\nde 10 min\n(HTTP 429)", ha="center", color=TXT2, fontsize=9)
ax.text(d("21:02"), 50, "pausa de\n10 min\n(HTTP 429)", ha="center", color=TXT2, fontsize=9)
for k, m in enumerate(a.marco or ["TSE: matematicamente\neleito ~21:09=21:09", "Urna Velox\n21:23=21:23"]):
    rot, h = m.rsplit("=", 1); rot = rot.replace("\\n", "\n")
    cor = CINZA if k == 0 else LARANJA
    ax.axvline(d(h), color=cor, lw=1, ls=(0, (3, 3)))
    ax.text(d(h) + dt.timedelta(minutes=-1 if k == 0 else 1), 8, rot, ha="right" if k == 0 else "left",
            color=TXT2 if k == 0 else LARANJA, fontsize=9)
ax.set_ylim(0, 101); ax.set_ylabel(f"seções com BU (% de {T:,})".replace(",", "."))
ax.xaxis.set_major_formatter(mdates.DateFormatter("%H:%M")); ax.grid(axis="y", color=GRADE, lw=0.8)
ax.set_title("Em produção o Urna Velox chegou a ficar ~12 min (63 mil BUs) atrás do recebimento", loc="left",
             color=TXT, fontsize=12, fontweight="bold")
ax.legend(frameon=False, loc="upper left", fontsize=9.5)
fig.tight_layout(); fig.savefig(pref + "curvas.png"); plt.close(fig)

# 3) fila
fig, ax = plt.subplots(figsize=(9, 3.6), dpi=150)
ax.plot(t, [(int(r["recebido_acum"]) - int(r[prod])) / 1000 for r in M], color=LARANJA, lw=2, label="produção (borda, 2 nós)")
if "aplicado_L_acum" in M[0]:
    ax.plot(t, [(int(r["recebido_acum"]) - int(r["aplicado_L_acum"])) / 1000 for r in M], color=AZUL, lw=2, label="validação no espelho")
for _, i, f in pausas:
    ax.axvspan(d(i), d(f), color=LARANJA, alpha=0.10, lw=0)
ax.set_ylabel("BUs na fila (mil)"); ax.set_ylim(bottom=0)
ax.xaxis.set_major_formatter(mdates.DateFormatter("%H:%M")); ax.grid(axis="y", color=GRADE, lw=0.8)
ax.set_title("A fila cresce nas pausas por HTTP 429 e só zera quando a chegada acaba (21:13)", loc="left",
             color=TXT, fontsize=12, fontweight="bold")
ax.legend(frameon=False, loc="upper left")
fig.tight_layout(); fig.savefig(pref + "fila.png"); plt.close(fig)
print("gravados:", pref + "{chegada_turnos,curvas,fila}.png")
