#!/usr/bin/env python3
"""Horários de chegada para o replay a partir dos cs.json (sem baixar aux.json).

Quando os BUs vêm do zip de dados abertos, não há o horário de recebimento no TSE (`dr`/`hr` do aux.json).
O cs.json traz, por seção, a data/hora de **disponibilização** no site de resultados (`da`/`ha`): é o
instante em que o BU ficou público, um pouco depois do recebimento. Este script grava esses horários no
formato das métricas da coleta (coluna `recebido_tse`), para `uv-coletor replay --chegadas`.

    python3 chegadas_cs.py ~/uvreal/2024-1t/dados/config > ~/uvreal/2024-1t/chegadas_cs.csv
"""
import glob, json, os, sys

print("uf,municipio,zona,secao,recebido_tse")
for p in sorted(glob.glob(os.path.join(sys.argv[1], "*-cs.json"))):
    uf = os.path.basename(p)[:2]
    for a in json.load(open(p, encoding="utf-8"))["abr"]:
        for m in a.get("mu", []):
            for z in m.get("zon", []):
                for s in z.get("sec", []):
                    if s.get("da") and s.get("ha"):
                        print(f"{uf},{int(m['cd'])},{int(z['cd'])},{int(s['ns'])},{s['da']} {s['ha']}")
