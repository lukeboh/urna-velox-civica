#!/usr/bin/env python3
"""Resume os erros de uma coleta (CSV de métricas do uv-coletor): por tipo e por UF.

    python3 erros_coleta.py ~/uvreal/metricas_nacional.csv [--ufs mg,zz]
"""
import collections, csv, sys

arq = sys.argv[1]
ufs = set(sys.argv[3].split(",")) if len(sys.argv) > 3 and sys.argv[2] == "--ufs" else set()
tipos, por_uf, amostra = collections.Counter(), collections.Counter(), collections.defaultdict(list)
for r in csv.DictReader(open(arq, encoding="utf-8")):
    e = r["erro"]
    if not e:
        continue
    t = e.split(":")[0] + ":" + e.split(":")[1].strip()[:20] if ":" in e else e[:40]
    tipos[t] += 1
    por_uf[(r["uf"], t)] += 1
    if (not ufs or r["uf"] in ufs) and len(amostra[(r["uf"], t)]) < 5:
        amostra[(r["uf"], t)].append(f'{r["municipio"]}/{r["zona"]}/{r["secao"]} sit={r["situacao"]} rec={r["recebido_tse"]} :: {e}')
print("erros por tipo:")
for t, n in tipos.most_common():
    print(f"  {n:>7}  {t}")
print("erros que não são 'aux HTTP 404', por UF:")
for (uf, t), n in sorted(por_uf.items()):
    if "404" not in t or (ufs and uf in ufs):
        print(f"  {uf} {n:>6}  {t}")
for k, v in amostra.items():
    if ufs and k[0] in ufs:
        print(k, *v, sep="\n   ")
