#!/usr/bin/env python3
"""Senadores eleitos a partir dos totais do Urna Velox, com a situação oficial ao lado (tabela Markdown).

    python3 senador_eleitos.py --totais ~/uvreal/resultado/totais_uf.csv --oficial ~/uvreal/oficial
"""
import argparse, collections, csv, json, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from comparar_oficial import UFS, candidatos, detalhes, situacao, n  # noqa: E402


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--totais", required=True)
    ap.add_argument("--oficial", required=True)
    a = ap.parse_args()
    votos = collections.defaultdict(dict)
    for r in csv.DictReader(open(a.totais)):
        if r["cargo"] == "5" and r["tipo"] == "nominal":
            votos[r["uf"]][int(r["codigo"])] = int(r["votos"])
    print("| UF | nº | candidato | votos (Urna Velox) | % dos válidos | situação oficial |")
    print("|---|---:|---|---:|---:|---|")
    total = iguais = 0
    for uf in UFS:
        o = json.load(open(os.path.join(a.oficial, f"{uf}-c0005-e006259-u.json"), encoding="utf-8"))
        carg = o["carg"][0]
        of_c, info = candidatos(carg), detalhes(carg)
        v = {c: votos[uf].get(c, 0) for c in of_c}
        st = situacao(5, max(1, n(carg.get("nv"))), v, info)
        validos = sum(v[c] for c in st)
        for c in sorted((c for c in st if st[c] == "Eleito"), key=lambda c: -v[c]):
            total += 1
            iguais += info[c]["st"] == "Eleito"
            vs = f"{v[c]:,}".replace(",", ".")
            pct = f"{100 * v[c] / validos:.2f}".replace(".", ",")
            print(f"| {uf.upper()} | {c} | {of_c[c][0]} | {vs} | {pct} % | {info[c]['st']} |")
    print(f"\n{iguais} de {total} eleitos calculados pelo Urna Velox coincidem com a situação oficial.")


if __name__ == "__main__":
    main()
