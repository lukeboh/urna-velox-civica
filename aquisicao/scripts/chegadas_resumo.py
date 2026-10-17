#!/usr/bin/env python3
"""Resumo dos horários de recebimento no TSE (dr/hr do aux.json) gravados nas métricas da coleta:
BUs por hora e os mais tardios.   python3 chegadas_resumo.py ~/uvreal/metricas_*.csv"""
import collections, csv, sys
from datetime import datetime

por_hora, todos = collections.Counter(), []
for p in sys.argv[1:]:
    if "inicios" in p or "taxa" in p:
        continue
    for r in csv.DictReader(open(p, encoding="utf-8")):
        t = r.get("recebido_tse", "")
        if not t:
            continue
        d = datetime.strptime(t, "%d/%m/%Y %H:%M:%S")
        por_hora[d.strftime("%Y-%m-%d %H")] += 1
        todos.append((d, r["uf"], r["municipio"], r["zona"], r["secao"], r.get("situacao", "")))
todos.sort()
print("BUs por hora de recebimento:")
for h in sorted(por_hora):
    print(f"  {h}h  {por_hora[h]:>7}")
print("mais tardios:")
for x in todos[-8:]:
    print("  ", x[0], *x[1:])
corte = datetime(2026, 10, 5, 2, 59)
print("depois de 05/10 02:59:", sum(1 for x in todos if x[0] > corte))
