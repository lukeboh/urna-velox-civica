#!/usr/bin/env python3
"""Fila (BUs recebidos no TSE e ainda não aplicados) e picos, a partir de <pleito>/analise/minuto_a_minuto.csv.

  python3 fila.py --pleito <pasta AAAA-MM-DD-código>   (rode antes o curvas.py)"""
import argparse, csv, os

ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
ap.add_argument("--pleito", required=True)
a = ap.parse_args()
L = list(csv.DictReader(open(os.path.join(a.pleito, "analise", "minuto_a_minuto.csv"))))
inst = [c[len("aplicado_"):] for c in L[0] if c.startswith("aplicado_") and not c.endswith("_acum")]
tot = int(L[-1]["recebido_acum"])

pico = max(L, key=lambda r: int(r["recebido"]))
print(f"pico de chegada: {pico['recebido']} BUs/min às {pico['minuto'][11:]}")
for n in inst:
    fila = max((int(r["recebido_acum"]) - int(r[f"aplicado_{n}_acum"]), r["minuto"][11:]) for r in L)
    vaz = max(L, key=lambda r: int(r[f"aplicado_{n}"]))
    print(f"{n}: fila máxima {fila[0]} BUs às {fila[1]}; vazão máxima {vaz[f'aplicado_{n}']} BUs/min às {vaz['minuto'][11:]}")
with open(os.path.join(a.pleito, "analise", "fila.csv"), "w", newline="") as f:
    w = csv.writer(f)
    w.writerow(["minuto", "pct_recebido"] + [f"fila_{n}" for n in inst] + [f"pct_{n}" for n in inst])
    for r in L:
        w.writerow([r["minuto"], round(100 * int(r["recebido_acum"]) / tot, 2)]
                   + [int(r["recebido_acum"]) - int(r[f"aplicado_{n}_acum"]) for n in inst]
                   + [round(100 * int(r[f"aplicado_{n}_acum"]) / tot, 2) for n in inst])
print("gravado", os.path.join(a.pleito, "analise", "fila.csv"))
