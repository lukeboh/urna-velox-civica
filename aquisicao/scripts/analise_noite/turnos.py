#!/usr/bin/env python3
"""Chegada dos BUs ao TSE (recebimento do aux.json) em duas noites: o pleito analisado × um anterior.

  python3 turnos.py --pleito <pasta AAAA-MM-DD-código> --anterior <pasta AAAA-MM-DD-código>

O pleito analisado é lido do secoes.log do primeiro nó; o anterior, da linha-do-tempo.csv da pasta dele. Grava
<pleito>/analise/chegada_turnos.csv (minuto desde 17h e % acumulado de cada noite) e imprime picos, marcos e pausas."""
import argparse, csv, datetime as dt, os
from collections import Counter
from comum import data_do_pleito, instancias

ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
ap.add_argument("--pleito", required=True)
ap.add_argument("--anterior", required=True)
a = ap.parse_args()


def h(s):
    return dt.datetime.strptime(s, "%d/%m/%Y %H:%M:%S")


cam = next(iter(instancias(a.pleito).values()))
vistos, t2 = set(), []
for r in csv.DictReader(open(cam)):
    k = (r["uf"], r["municipio"], r["zona"], r["secao"])
    if r["acao"] == "aplicado" and r["recebido"] and k not in vistos:
        vistos.add(k); t2.append(h(r["recebido"]))
t1 = [h(r["recebido_tse"]) for r in csv.DictReader(open(os.path.join(a.anterior, "linha-do-tempo.csv")))
      if r["recebido_tse"]]
d1, d2 = data_do_pleito(a.anterior), data_do_pleito(a.pleito)


def por_minuto(ts, dia):
    base = dt.datetime(dia.year, dia.month, dia.day, 17, 0)
    return Counter(int((t - base).total_seconds() // 60) for t in ts)


m1, m2, n1, n2 = por_minuto(t1, d1), por_minuto(t2, d2), len(t1), len(t2)
with open(os.path.join(a.pleito, "analise", "chegada_turnos.csv"), "w", newline="") as f:
    w = csv.writer(f)
    w.writerow(["min_desde_17h", "hora", "anterior_bu_min", "pleito_bu_min", "anterior_pct_acum", "pleito_pct_acum"])
    a1 = sum(v for k, v in m1.items() if k < 0); a2 = sum(v for k, v in m2.items() if k < 0)
    for k in range(600):
        a1 += m1[k]; a2 += m2[k]
        w.writerow([k, f"{17 + k // 60:02d}:{k % 60:02d}", m1[k], m2[k], round(100 * a1 / n1, 3), round(100 * a2 / n2, 3)])


def quando(ts, f):
    v = sorted(ts); return v[int(f * len(v)) - 1].strftime("%H:%M")


for nome, ts, m in ((f"anterior ({d1})", t1, m1), (f"pleito ({d2})", t2, m2)):
    n = len(ts)
    pico = max(((k, v) for k, v in m.items() if k >= 0), key=lambda x: x[1])
    p10 = max(sum(m[k + i] for i in range(10)) for k in range(600)) / 10
    p60 = max(sum(m[k + i] for i in range(60)) for k in range(600)) / 60
    print(f"{nome}: {n} BUs | 50% {quando(ts, .5)} 90% {quando(ts, .9)} 99% {quando(ts, .99)} | pico 1 min {pico[1]}"
          f" BUs/min às {17 + pico[0] // 60:02d}:{pico[0] % 60:02d}; 10 min {p10:.0f}/min; 60 min {p60:.0f}/min")
    print("   até 19h/20h/21h:", " / ".join(f"{100 * sum(v for kk, v in m.items() if kk < k) / n:.1f} %" for k in (120, 180, 240)))
    ps, ini = [], None
    for k in range(360):
        if m[k] == 0 and ini is None: ini = k
        if m[k] > 0 and ini is not None:
            if k - ini >= 3: ps.append(f"{17 + ini // 60:02d}:{ini % 60:02d}–{17 + (k - 1) // 60:02d}:{(k - 1) % 60:02d}")
            ini = None
    print("   pausas de 3 min ou mais:", ", ".join(ps) or "nenhuma")
