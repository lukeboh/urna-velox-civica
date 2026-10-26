#!/usr/bin/env python3
"""Curvas de uma noite de apuração: recebimento no TSE, disponibilização no cs.json e aplicação em cada instância.

  python3 curvas.py --pleito <pasta AAAA-MM-DD-código>

Grava em <pasta>/analise: minuto_a_minuto.csv (contagem e acumulado por minuto, 16h–01h), percentis.json (atraso por
janela de recebimento, em segundos) e marcos_pct.json (hora de 50/90/95/99 % das seções)."""
import argparse, csv, datetime as dt, json, os, statistics as st
from collections import Counter, defaultdict
from comum import FUSO, data_do_pleito, instancias, ler_secoes, minuto, pasta_analise

ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
ap.add_argument("--pleito", required=True)
a = ap.parse_args()
dia = data_do_pleito(a.pleito)
rec, disp, apl = ler_secoes(instancias(a.pleito))
SAI = pasta_analise(a.pleito)

ini = dt.datetime(dia.year, dia.month, dia.day, 16, 0, tzinfo=FUSO).timestamp()
fim = ini + 9 * 3600
cont = defaultdict(Counter)
for t in rec.values(): cont[minuto(t)]["recebido"] += 1
for t in disp.values(): cont[minuto(t)]["disponivel"] += 1
for n, v in apl.items():
    for t in v.values(): cont[minuto(t)]["aplicado_" + n] += 1
cols = ["recebido", "disponivel"] + ["aplicado_" + n for n in apl]
bases = {"recebido": rec, "disponivel": disp, **{"aplicado_" + n: apl[n] for n in apl}}
acum = {c: sum(1 for t in bases[c].values() if t < ini) for c in cols}
with open(os.path.join(SAI, "minuto_a_minuto.csv"), "w", newline="") as f:
    w = csv.writer(f)
    w.writerow(["minuto"] + cols + [c + "_acum" for c in cols])
    t = ini
    while t < fim:
        m = minuto(t)
        linha = [cont[m][c] for c in cols]
        for c, v in zip(cols, linha): acum[c] += v
        w.writerow([m] + linha + [acum[c] for c in cols])
        t += 60


def pct(v, p):
    v = sorted(v)
    return v[min(len(v) - 1, int(p / 100 * len(v)))] if v else None


def resumo(v):
    return {"n": len(v), "p50": pct(v, 50), "p90": pct(v, 90), "p99": pct(v, 99), "max": max(v) if v else None,
            "media": round(st.mean(v), 1) if v else None}


def hm(t):
    h = dt.datetime.fromtimestamp(t, FUSO)
    return h.strftime("%H:%M") if h.date() == dia else "24:00"


janelas = [("17:00", "18:00"), ("18:00", "18:30"), ("18:30", "19:00"), ("19:00", "19:30"), ("19:30", "20:00"),
           ("20:00", "20:30"), ("20:30", "21:00"), ("21:00", "21:30"), ("21:30", "22:00"), ("22:00", "24:00"),
           ("17:00", "24:00")]
out = {"secoes": {c: len(bases[c]) for c in cols}, "disp_menos_rec_s": {},
       "apl_menos_disp_s": {n: {} for n in apl}, "apl_menos_rec_s": {n: {} for n in apl}}
for i, j in janelas:
    ch = f"{i}-{j}"
    ks = [k for k, t in rec.items() if i <= hm(t) < j]
    out["disp_menos_rec_s"][ch] = resumo([disp[k] - rec[k] for k in ks if k in disp])
    for n in apl:
        out["apl_menos_disp_s"][n][ch] = resumo([apl[n][k] - disp[k] for k in ks if k in disp and k in apl[n]])
        out["apl_menos_rec_s"][n][ch] = resumo([apl[n][k] - rec[k] for k in ks if k in apl[n]])
json.dump(out, open(os.path.join(SAI, "percentis.json"), "w"), indent=1, ensure_ascii=False)

tot = len(rec)
marc = {}
for c in cols:
    v = sorted(bases[c].values())
    marc[c] = {f"{int(f * 100)}%": (minuto(v[int(f * tot) - 1]) if len(v) >= int(f * tot) else None)
               for f in (0.5, 0.9, 0.95, 0.99)}
json.dump(marc, open(os.path.join(SAI, "marcos_pct.json"), "w"), indent=1)

print("seções:", json.dumps(out["secoes"]))
print("marcos:", json.dumps(marc))
for n in apl:
    r = out["apl_menos_rec_s"][n]["17:00-24:00"]
    print(f"{n}: aplicado − recebido p50 {r['p50']:.0f} s, p90 {r['p90']:.0f} s, p99 {r['p99']:.0f} s (n={r['n']})")
for i, j in janelas[:-1]:
    ch = f"{i}-{j}"
    print(ch, "n", out["disp_menos_rec_s"][ch]["n"], "|",
          " | ".join(f"{n} p50/p90 {out['apl_menos_rec_s'][n][ch]['p50']}/{out['apl_menos_rec_s'][n][ch]['p90']}"
                     for n in apl))
