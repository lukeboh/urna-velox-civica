#!/usr/bin/env python3
"""Compara a totalização do uv-coletor (a partir dos BUs) com os arquivos oficiais de divulgação do TSE.

Cargos: Presidente (eleição 6257, cargo 1) por UF, exterior (zz) e Brasil; Governador (6259, cargo 3) e
Senador (6259, cargo 5) por UF.
Para cada abrangência confere: votos de cada candidato (vap), brancos (vb), nulos (vn), nulos técnicos (vnt),
comparecimento (e.c), o número de BUs contra as seções totalizadas (s.st) e a **situação de cada candidato**
(eleito / 2º turno / não eleito) calculada a partir dos totais do Urna Velox contra o campo `st` oficial.

    python3 comparar_oficial.py --totais totais.csv --oficial ~/uvreal/oficial --dados ~/uvreal/dados --saida comparacao.md
"""
import argparse, collections, csv, glob, json, os, struct, sys
from datetime import date

UFS = "ac al am ap ba ce df es go ma mg ms mt pa pb pe pi pr rj rn ro rr rs sc se sp to".split()
CARGOS = {1: ("Presidente", "6257"), 3: ("Governador", "6259"), 5: ("Senador", "6259")}


def n(x):
    return int(x) if x not in (None, "") else 0


def detalhes(carg):
    """{número: {"dt": data de nascimento, "st": situação oficial, "dvt": destinação}} do arquivo oficial."""
    out = {}
    def visitar(o):
        if isinstance(o, dict):
            for c in o.get("cand", []):
                out[int(c["n"])] = {"dt": c.get("dt", ""), "st": c.get("st", ""), "dvt": c.get("dvt", "")}
            for v in o.values():
                visitar(v)
        elif isinstance(o, list):
            for v in o:
                visitar(v)
    visitar(carg)
    return out


def nascimento(s):
    try:
        d, m, a = (int(x) for x in s.split("/"))
        return date(a, m, d)
    except Exception:
        return date.max


def situacao(cargo, vagas, votos, info):
    """Situação de cada candidato apto a partir dos votos totalizados pelo Urna Velox.

    Presidente e Governador: maioria absoluta dos votos válidos elege no 1º turno; senão, os dois mais votados
    vão ao 2º turno. Senador: os `vagas` mais votados são eleitos (maioria simples). Empate: o mais idoso.
    Só concorrem candidatos com destinação "Válido" (os demais têm votos anulados ou sub judice)."""
    aptos = [c for c in votos if info.get(c, {}).get("dvt", "").startswith("Válido")]
    ordem = sorted(aptos, key=lambda c: (-votos[c], nascimento(info[c]["dt"])))
    validos = sum(votos[c] for c in aptos)
    st = {c: "Não eleito" for c in aptos}
    if cargo == 5:
        for c in ordem[:vagas]:
            st[c] = "Eleito"
    elif ordem:
        if 2 * votos[ordem[0]] > validos:
            st[ordem[0]] = "Eleito"
        else:
            for c in ordem[:2]:
                st[c] = "2º turno"
    return st


def candidatos(carg):
    """{número: (nome, votos apurados, destinação)} a partir do arquivo oficial."""
    out = {}
    def visitar(o):
        if isinstance(o, dict):
            if "cand" in o:
                for c in o["cand"]:
                    out[int(c["n"])] = (c.get("nmu") or c.get("nm"), n(c.get("vap")), c.get("dvt", ""))
            for v in o.values():
                visitar(v)
        elif isinstance(o, list):
            for v in o:
                visitar(v)
    visitar(carg)
    return out


def contar_bus(dados):
    """Nº de BUs (seções distintas) por UF nos pacotes .uvpk, lendo só os cabeçalhos."""
    out = {}
    for arq in sorted(glob.glob(os.path.join(dados, "*.uvpk"))):
        uf = os.path.basename(arq)[:2]
        vistos = set()
        with open(arq, "rb") as f:
            f.seek(8)
            while True:
                h = f.read(52)
                if len(h) < 52:
                    break
                tam, _pl, mu, zn, se = struct.unpack_from("<IIIHH", h, 0)
                vistos.add((mu, zn, se))
                f.seek(tam, 1)
        out[uf] = len(vistos)
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--totais", required=True, help="CSV de uv-coletor totalizar --totais (uf,cargo,tipo,codigo,votos)")
    ap.add_argument("--oficial", required=True, help="pasta com <abr>-c<cargo>-e<eleicao>-u.json")
    ap.add_argument("--dados", help="pasta dos .uvpk (para contar BUs por UF)")
    ap.add_argument("--saida", help="relatório Markdown")
    ap.add_argument("--cargos", default="1,3,5", help="cargos a conferir (1 Presidente, 3 Governador, 5 Senador)")
    ap.add_argument("--metricas", default="", help="CSVs de métricas da coleta (vírgula): seções 'Não instalada' contam como totalizadas sem BU")
    a = ap.parse_args()
    # seções não instaladas: o TSE as conta em s.st (totalizadas, com zero voto), mas não há BU
    nao_inst = collections.Counter()
    vistas = set()
    for p in [x for x in a.metricas.split(",") if x and os.path.exists(x)]:
        for r in csv.DictReader(open(p, encoding="utf-8")):
            if r.get("erro", "").startswith("sem BU (situação Nao instalada"):
                k = (r["uf"], r["municipio"], r["zona"], r["secao"])
                if k not in vistas:
                    vistas.add(k)
                    nao_inst[r["uf"]] += 1
    nao_inst["br"] = sum(nao_inst.values())

    nosso = collections.defaultdict(collections.Counter)  # (abr, cargo) -> Counter
    with open(a.totais) as f:
        for r in csv.DictReader(f):
            c = int(r["cargo"])
            if c not in CARGOS:
                continue
            k = r["tipo"] if r["tipo"] != "nominal" else ("nominal", int(r["codigo"]))
            nosso[(r["uf"], c)][k] += int(r["votos"])
            if c == 1:
                nosso[("br", 1)][k] += int(r["votos"])
    bus = contar_bus(a.dados) if a.dados else {}
    bus["br"] = sum(bus.values()) if bus else 0

    linhas, difs_por_abr, notas = [], [], []
    situacoes = collections.Counter()  # (cargo, "ok"/"dif") -> nº de candidatos
    total_dif = 0
    pedidos = {int(x) for x in a.cargos.split(",") if x.strip()}
    for cargo, (nome, ele) in CARGOS.items():
        if cargo not in pedidos:
            continue
        abrs = ["br"] + UFS + ["zz"] if cargo == 1 else UFS
        for abr in abrs:
            p = os.path.join(a.oficial, f"{abr}-c{cargo:04d}-e00{ele}-u.json")
            if not os.path.exists(p):
                linhas.append((nome, abr, "arquivo oficial ausente", "", "", "", "", "", "❓"))
                total_dif += 1
                continue
            o = json.load(open(p, encoding="utf-8"))
            carg = o["carg"][0]
            of_c = candidatos(carg)
            v, e, s = o.get("v", {}), o.get("e", {}), o.get("s", {})
            t = nosso.get((abr, cargo), collections.Counter())
            difs = []
            # candidatos
            nom_nosso = {k[1]: q for k, q in t.items() if isinstance(k, tuple)}
            for num, (nm, vap, dvt) in sorted(of_c.items()):
                q = nom_nosso.get(num, 0)
                if q != vap:
                    difs.append(f"candidato {num} {nm} ({dvt}): oficial {vap}, BUs {q} (dif {q - vap:+d})")
            # situação de cada candidato (eleito / 2º turno / não eleito) calculada com os votos do Urna Velox
            # A situação vale para a circunscrição do cargo: Presidente é decidido no Brasil (os arquivos das UFs
            # e do exterior repetem a situação nacional); Governador e Senador, na UF.
            info = detalhes(carg)
            base = nosso.get(("br", 1), collections.Counter()) if cargo == 1 else t
            votos_circ = {c: base.get(("nominal", c), 0) for c in of_c}
            calc = situacao(cargo, max(1, n(carg.get("nv"))), votos_circ, info)
            for c, st_nosso in sorted(calc.items()):
                st_of = info[c]["st"]
                if st_of == st_nosso:
                    situacoes[(cargo, "ok")] += 1
                else:
                    situacoes[(cargo, "dif")] += 1
                    difs.append(f"situação do candidato {c} {of_c[c][0]}: oficial '{st_of}', Urna Velox '{st_nosso}'")
            # votos digitados em número de candidato que não está apto (fora da lista oficial): o TSE os soma
            # como "nulos técnicos" (vnt); nos BUs aparecem como votos nominais nesse número
            sobra = {num: q for num, q in nom_nosso.items() if num not in of_c and q}
            if sobra:
                notas.append((nome, abr.upper(), ", ".join(f"nº {k}: {q}" for k, q in sorted(sobra.items())), n(v.get("vnt"))))
            comps = [("brancos (vb)", n(v.get("vb")), t["branco"]),
                     ("nulos (vn)", n(v.get("vn")), t["nulo"]),
                     ("nulos técnicos (vnt) = votos em números sem candidato apto", n(v.get("vnt")), sum(sobra.values())),
                     ("comparecimento (e.c)", n(e.get("c")), t["comparecimento"])]
            for rot, of, nos in comps:
                if of != nos:
                    difs.append(f"{rot}: oficial {of}, BUs {nos} (dif {nos - of:+d})")
            if t["sem_candidato"] or t["legenda"]:
                difs.append(f"atenção: legenda {t['legenda']}, sem_candidato {t['sem_candidato']} nos BUs")
            st = n(s.get("st"))
            nb = bus.get(abr, 0)
            ni = nao_inst.get(abr, 0) if cargo == 1 or abr != "br" else 0
            if bus and nb + ni != st:
                difs.append(f"seções totalizadas (s.st): oficial {st}, BUs coletados {nb} + não instaladas {ni} (dif {nb + ni - st:+d})")
            nominais_of = sum(x[1] for x in of_c.values())
            nominais_nos = sum(nom_nosso.get(k, 0) for k in of_c)
            ok = "✅" if not difs else "❌"
            total_dif += bool(difs)
            linhas.append((nome, abr.upper(), f"{nominais_of:,}".replace(",", "."), f"{nominais_nos:,}".replace(",", "."),
                           f"{n(v.get('vb')):,}".replace(",", "."), f"{n(v.get('tvn')):,}".replace(",", "."),
                           f"{n(e.get('c')):,}".replace(",", "."), (f"{st} / {nb}" + (f" + {ni} n.i." if ni else "")) if bus else str(st), ok))
            if difs:
                difs_por_abr.append((nome, abr.upper(), difs))

    out = ["# Comparação: totalização dos BUs × divulgação oficial do TSE", "",
           "| cargo | abrangência | votos nominais (oficial) | votos nominais (BUs) | brancos | nulos | comparecimento | seções totalizadas (oficial) / BUs | |",
           "|---|---|---:|---:|---:|---:|---:|---:|:-:|"]
    out += ["| " + " | ".join(l) + " |" for l in linhas]
    out += ["", f"**Abrangências com divergência: {total_dif}**", ""]
    out += ["**Situação dos candidatos** (eleito / 2º turno / não eleito, calculada com os votos do Urna Velox × campo `st` oficial):", ""]
    for cargo in sorted({c for c, _ in situacoes}):
        ok, dif = situacoes[(cargo, "ok")], situacoes[(cargo, "dif")]
        out.append(f"* {CARGOS[cargo][0]}: {ok} de {ok + dif} candidatos com a mesma situação" + (f" — **{dif} divergentes**" if dif else ""))
    out.append("")
    if notas:
        out += ["## Votos em números sem candidato apto (nulos técnicos)", "",
                "Nos BUs esses votos aparecem como nominais no número digitado; na divulgação o TSE os soma em `vnt`. "
                "A comparação acima já confere `vnt` com a soma desses números.", "",
                "| cargo | abrangência | números (votos nos BUs) | vnt oficial |", "|---|---|---|---:|"]
        out += [f"| {a} | {b} | {c} | {d} |" for a, b, c, d in notas]
        out.append("")
    for nome, abr, difs in difs_por_abr:
        out.append(f"## {nome} — {abr}")
        out += [f"* {d}" for d in difs]
        out.append("")
    txt = "\n".join(out)
    print(txt)
    if a.saida:
        open(a.saida, "w", encoding="utf-8").write(txt + "\n")
    sys.exit(1 if total_dif else 0)


if __name__ == "__main__":
    main()
