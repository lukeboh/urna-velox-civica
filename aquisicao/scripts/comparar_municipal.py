#!/usr/bin/env python3
"""Compara a totalização por município do uv-coletor (a partir dos BUs) com os arquivos oficiais de divulgação.

Cargos de circunscrição municipal: **Prefeito** (cargo 11) e **consultas populares** (perguntas, cargos 25..99).
Para cada município confere: votos de cada candidato/opção (`vap`), brancos (`vb`), nulos (`vn`), nulos técnicos
(`vnt`), comparecimento (`e.c`), BUs × seções totalizadas (`s.st`) e a **situação** de cada candidato/opção
(`st`), calculada com os votos do Urna Velox:

* Prefeito, município com mais de 200 mil eleitores (`e.te` do próprio arquivo oficial): maioria absoluta dos
  votos válidos elege no 1º turno; senão, os dois mais votados vão ao 2º turno (CF, art. 29, II).
* Prefeito, demais municípios, e 2º turno (`t` = 2 no arquivo oficial): o mais votado é eleito.
* Empate: o mais idoso. Só concorrem candidatos com destinação "Válido".
* Consulta: a opção mais votada recebe `st` "Eleito"; as demais, "Não eleito".

    python3 comparar_municipal.py --totais totais_municipio.csv --oficial ~/uvreal/2024-1t/oficial \
        --dados ~/uvreal/2024-1t/dados --eleicoes 619:11,633:55,634:53,635:51,636:54,637:52 --saida comparacao.md
"""
import argparse, collections, csv, glob, json, os, struct, sys

from comparar_oficial import n, nascimento

LIMITE_2T = 200_000


def contar_bus_municipio(dados):
    """Nº de BUs (seções distintas) por (uf, município) nos pacotes .uvpk, lendo só os cabeçalhos."""
    out = collections.Counter()
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
        for mu, _, _ in vistos:
            out[(uf, mu)] += 1
    return out


def lista_oficial(o):
    """[(número, nome, votos, destinação, situação, nascimento)] de Prefeito (`carg`) ou consulta (`perg`)."""
    out = []
    if "perg" in o:
        for r in o["perg"][0].get("resp", []):
            out.append((int(r["n"]), r.get("ds", ""), n(r.get("vap")), "Válido", r.get("st", ""), ""))
        return out
    def visitar(x):
        if isinstance(x, dict):
            for c in x.get("cand", []):
                out.append((int(c["n"]), c.get("nmu") or c.get("nm"), n(c.get("vap")), c.get("dvt", ""), c.get("st", ""), c.get("dt", "")))
            for v in x.values():
                visitar(v)
        elif isinstance(x, list):
            for v in x:
                visitar(v)
    visitar(o["carg"][0])
    return out


def situacao(cargo, eleitorado, votos, info, turno=1):
    """Situação calculada com os votos do Urna Velox. `votos`: todos os candidatos/opções, inclusive os de
    votação anulada; `info`: {número: (destinação, nascimento)}.

    Prefeito (regras observadas nos arquivos oficiais de 2024, que seguem o Código Eleitoral, art. 224):
    * ninguém é eleito (haverá eleição suplementar) se o candidato com a maior votação nominal teve os votos
      anulados (destinação diferente de "Válido"), ou se os votos anulados passam de 50 % da votação nominal;
    * a maioria absoluta (municípios com mais de 200 mil eleitores) é medida sobre a votação nominal inteira,
      inclusive a de candidatos anulados depois da eleição (o 2º turno foi decidido com esses votos).
    """
    apto = lambda c: info[c][0].startswith("Válido")
    ordem_todos = sorted(votos, key=lambda c: (-votos[c], nascimento(info[c][1])))
    aptos = [c for c in ordem_todos if apto(c)]
    st = {c: "Não eleito" for c in votos}
    if not aptos:
        return st
    nominal = sum(votos.values())
    if cargo == 11:
        anulados = sum(q for c, q in votos.items() if not apto(c))
        if not apto(ordem_todos[0]) or 2 * anulados > nominal:
            return st
        if turno == 1 and eleitorado > LIMITE_2T and 2 * votos[aptos[0]] <= nominal:
            for c in aptos[:2]:
                st[c] = "2º turno"
            return st
    st[aptos[0]] = "Eleito"
    return st


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--totais", required=True, help="CSV de uv-coletor totalizar --totais-municipio")
    ap.add_argument("--oficial", required=True, help="pasta com <uf><mun>-c<cargo>-e<eleição>-u.json")
    ap.add_argument("--dados", help="pasta dos .uvpk (BUs por município)")
    ap.add_argument("--eleicoes", default="619:11", help="eleição:cargo, separados por vírgula")
    ap.add_argument("--metricas", default="", help="CSVs de métricas da coleta: seções 'Não instalada' contam como totalizadas sem BU")
    ap.add_argument("--saida", help="relatório Markdown")
    a = ap.parse_args()

    nao_inst = collections.Counter()
    vistas = set()
    for p in [x for x in a.metricas.split(",") if x and os.path.exists(x)]:
        for r in csv.DictReader(open(p, encoding="utf-8")):
            if r.get("erro", "").startswith("sem BU (situação Nao instalada"):
                k = (r["uf"], r["municipio"], r["zona"], r["secao"])
                if k not in vistas:
                    vistas.add(k)
                    nao_inst[(r["uf"], int(r["municipio"]))] += 1

    nosso = collections.defaultdict(collections.Counter)  # (uf, mun, cargo) -> Counter
    with open(a.totais) as f:
        for r in csv.DictReader(f):
            k = r["tipo"] if r["tipo"] != "nominal" else ("nominal", int(r["codigo"]))
            nosso[(r["uf"], int(r["municipio"]), int(r["cargo"]))][k] += int(r["votos"])
    bus = contar_bus_municipio(a.dados) if a.dados else None

    resumo, difs_por_abr, notas = [], [], []
    for ec in a.eleicoes.split(","):
        ele, cargo = (int(x) for x in ec.split(":"))
        arqs = sorted(glob.glob(os.path.join(a.oficial, f"*-c{cargo:04d}-e{ele:06d}-u.json")))
        cm = os.path.join(a.oficial, f"mun-e{ele:06d}-cm.json")
        esperados = sum(len(x.get("mu", [])) for x in json.load(open(cm, encoding="utf-8"))["abr"]) if os.path.exists(cm) else len(arqs)
        cont = collections.Counter()
        sit = collections.Counter()
        sem_dados, pendentes = [], []
        for p in arqs:
            o = json.load(open(p, encoding="utf-8"))
            nome = os.path.basename(p)
            uf, mu = nome[:2], int(nome[2:7])
            of = lista_oficial(o)
            v, e, s = o.get("v", {}), o.get("e", {}), o.get("s", {})
            t = nosso.get((uf, mu, cargo), collections.Counter())
            if n(e.get("c")) == 0 and not t:
                sem_dados.append(f"{uf.upper()} {mu:05d}")  # sem eleição ou sem votação no município
            nom = {k[1]: q for k, q in t.items() if isinstance(k, tuple)}
            difs = []
            for num, nm, vap, dvt, _, _ in of:
                q = nom.get(num, 0)
                if q != vap:
                    difs.append(f"{num} {nm} ({dvt}): oficial {vap}, BUs {q} (dif {q - vap:+d})")
            info = {num: (dvt, dt) for num, _, _, dvt, _, dt in of}
            calc = situacao(cargo, n(e.get("te")), {num: nom.get(num, 0) for num in info}, info, int(o.get("t", "1") or 1))
            if o.get("tf") == "n":
                # totalização não finalizada no oficial (ex.: "Aguarde reprocessamento da eleição"): `st` vem vazio
                pendentes.append(f"{uf.upper()} {mu:05d}" + (f" ({o['mntf']})" if o.get("mntf") else ""))
                calc = {}
            for num, nm, _, _, st_of, _ in of:
                if num not in calc:
                    continue
                if calc[num] == st_of:
                    sit["ok"] += 1
                else:
                    sit["dif"] += 1
                    difs.append(f"situação de {num} {nm}: oficial '{st_of}', Urna Velox '{calc[num]}'")
            sobra = {num: q for num, q in nom.items() if num not in info and q}
            if sobra:
                notas.append((cargo, f"{uf.upper()} {mu:05d}", ", ".join(f"nº {k}: {q}" for k, q in sorted(sobra.items())), n(v.get("vnt"))))
            for rot, x, y in [("brancos (vb)", n(v.get("vb")), t["branco"]), ("nulos (vn)", n(v.get("vn")), t["nulo"]),
                              ("nulos técnicos (vnt)", n(v.get("vnt")), sum(sobra.values())),
                              ("comparecimento (e.c)", n(e.get("c")), t["comparecimento"])]:
                if x != y:
                    difs.append(f"{rot}: oficial {x}, BUs {y} (dif {y - x:+d})")
            if t["legenda"] or t["sem_candidato"]:
                difs.append(f"atenção: legenda {t['legenda']}, sem_candidato {t['sem_candidato']} nos BUs")
            if bus is not None:
                nb, ni = bus.get((uf, mu), 0), nao_inst.get((uf, mu), 0)
                if nb + ni != n(s.get("st")):
                    difs.append(f"seções totalizadas (s.st): oficial {n(s.get('st'))}, BUs {nb} + não instaladas {ni}")
            cont["ok" if not difs else "dif"] += 1
            if difs:
                difs_por_abr.append((ele, cargo, f"{uf.upper()} {mu:05d}", difs))
        resumo.append((ele, cargo, esperados, len(arqs), cont["ok"], cont["dif"], sit["ok"], sit["dif"], sem_dados, pendentes))

    out = ["# Comparação por município: totalização dos BUs × divulgação oficial do TSE", "",
           "| eleição | cargo | municípios no cm.json | arquivos oficiais | idênticos | com divergência | situação idêntica | situação divergente |",
           "|---|---|---:|---:|---:|---:|---:|---:|"]
    total_dif = 0
    for ele, cargo, esp, narq, ok, dif, sok, sdif, _, _ in resumo:
        total_dif += dif + (esp - narq)
        out.append(f"| {ele} | {cargo} | {esp} | {narq} | {ok} | {dif} | {sok} | {sdif} |")
    out += ["", f"**Municípios com divergência ou sem arquivo oficial: {total_dif}**", ""]
    for ele, cargo, _, _, _, _, _, _, sem, pend in resumo:
        if sem:
            out.append(f"* Eleição {ele}, cargo {cargo}: {len(sem)} município(s) sem comparecimento no oficial e sem BUs: {', '.join(sem[:30])}")
        if pend:
            out.append(f"* Eleição {ele}, cargo {cargo}: {len(pend)} município(s) com a totalização não finalizada no oficial "
                       f"(`tf` = n, situação vazia; votos conferidos, situação não): {', '.join(pend)}")
    if notas:
        out += ["", "## Votos em números sem candidato apto (nulos técnicos)", "",
                "| cargo | município | números (votos nos BUs) | vnt oficial |", "|---|---|---|---:|"]
        out += [f"| {c} | {m} | {x} | {y} |" for c, m, x, y in notas]
    for ele, cargo, abr, difs in difs_por_abr:
        out += ["", f"## Eleição {ele}, cargo {cargo} — {abr}"] + [f"* {d}" for d in difs]
    txt = "\n".join(out) + "\n"
    print(txt if len(txt) < 20000 else txt[:20000] + "\n… (relatório completo em --saida)")
    if a.saida:
        open(a.saida, "w", encoding="utf-8").write(txt)
    sys.exit(1 if total_dif else 0)


if __name__ == "__main__":
    main()
