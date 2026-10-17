#!/usr/bin/env python3
"""Confere a totalização do uv-coletor com um decodificador ASN.1 independente.

Lê os pacotes .uvpk, decodifica cada BU com `asn1tools` a partir da especificação
oficial do TSE (aquisicao/spec/bu.asn1), soma os votos por UF/cargo/tipo/votável e
compara, linha a linha, com o CSV de totais gravado por `uv-coletor totalizar --totais`.

    python3 conferir_referencia.py --dados ~/uvreal/dados --totais totais_uf.csv [--max N]
"""
import argparse, collections, csv, glob, os, struct, sys, time

import asn1tools

AQUI = os.path.dirname(os.path.abspath(__file__))
CAB = 52
TIPOS = {"nominal": "nominal", "branco": "branco", "nulo": "nulo", "legenda": "legenda", "cargoSemCandidato": "sem_candidato"}
CARGOS = {"presidente": 1, "vicePresidente": 2, "governador": 3, "viceGovernador": 4, "senador": 5, "deputadoFederal": 6,
          "deputadoEstadual": 7, "deputadoDistrital": 8, "prefeito": 11, "vicePrefeito": 12, "vereador": 13}


def registros(caminho):
    b = open(caminho, "rb").read()
    assert b[:4] == b"UVPK", caminho
    p = 8
    while p + CAB <= len(b):
        n, = struct.unpack_from("<I", b, p)
        uf = b[p + 16:p + 18].decode()
        yield uf, b[p + CAB:p + CAB + n]
        p += CAB + n


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--dados", required=True)
    ap.add_argument("--totais", required=True, help="CSV de uv-coletor totalizar --totais")
    ap.add_argument("--spec", default=os.path.join(AQUI, "..", "spec", "bu.asn1"))
    ap.add_argument("--ufs", default="", help="só estes pacotes, ex.: df,ac (padrão: todos)")
    ap.add_argument("--max", type=int, default=0, help="limita o nº de BUs (0 = todos; a comparação só vale com todos)")
    a = ap.parse_args()
    conv = asn1tools.compile_files(a.spec, codec="ber")
    tot = collections.Counter()
    n = 0
    t0 = time.time()
    ufs = {u.strip() for u in a.ufs.split(",") if u.strip()}
    for arq in sorted(glob.glob(os.path.join(a.dados, "*.uvpk"))):
        if ufs and os.path.basename(arq)[:2] not in ufs:
            continue
        for uf, bu in registros(arq):
            env = conv.decode("EntidadeEnvelopeGenerico", bu)
            ent = conv.decode("EntidadeBoletimUrna", env["conteudo"])
            for el in ent["resultadosVotacaoPorEleicao"]:
                for rv in el["resultadosVotacao"]:
                    comp = rv["qtdComparecimento"]
                    for tc in rv["totaisVotosCargo"]:
                        k, v = tc["codigoCargo"]
                        cargo = CARGOS.get(v, v) if k == "cargoConstitucional" else v
                        tot[(uf, cargo, "comparecimento", "")] += comp
                        for vv in tc["votosVotaveis"]:
                            tipo = TIPOS[vv["tipoVoto"]]
                            ident = vv.get("identificacaoVotavel")
                            cod = "" if tipo in ("branco", "nulo", "sem_candidato") else str(ident["codigo"] if tipo == "nominal" else ident["partido"])
                            tot[(uf, cargo, tipo, cod)] += vv["quantidadeVotos"]
            n += 1
            if a.max and n >= a.max:
                break
        if a.max and n >= a.max:
            break
    dt = time.time() - t0
    nosso = collections.Counter()
    with open(a.totais) as f:
        for r in csv.DictReader(f):
            if ufs and r["uf"] not in ufs:
                continue
            nosso[(r["uf"], int(r["cargo"]), r["tipo"], r["codigo"])] += int(r["votos"])
    chaves = {k for k in set(tot) | set(nosso) if tot[k] or nosso[k]}
    dif = [(k, tot[k], nosso[k]) for k in sorted(chaves, key=str) if tot[k] != nosso[k]]
    print(f"referência (asn1tools): {n} BUs decodificados em {dt:.1f} s ({n / dt:.0f} BUs/s); {len(chaves)} totais comparados; divergências: {len(dif)}")
    for k, r, o in dif[:20]:
        print("  ", k, "referência", r, "uv-coletor", o)
    sys.exit(1 if dif and not a.max else 0)


if __name__ == "__main__":
    main()
