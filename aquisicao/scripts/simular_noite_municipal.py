#!/usr/bin/env python3
"""Simulação da noite de uma eleição municipal, BU a BU: em que instante o Urna Velox define o resultado de cada
município, comparado com o horário de totalização do TSE no arquivo oficial.

Para cada seção, a partir das métricas de uma coleta real pela API (`uv-coletor baixar --metricas`):
* `recebido_tse`: quando o BU chegou ao TSE (dr/hr do aux.json);
* `disponivel_cs`: quando a seção apareceu como disponível no cs.json do site público (da/ha);
* latências medidas de verdade nessa coleta para o aux.json e o BU.

Dois cenários de aquisição:
* **site** (o que um observador externo consegue): o coletor relê o cs.json de cada UF a cada `--intervalo-cs`
  segundos, descobre as seções disponíveis e as baixa em fila FIFO com teto de `--rps` requisições/s
  (2 por seção, descontadas as releituras do cs.json); cada seção leva as latências medidas dela;
* **rede** (Urna Velox recebendo os BUs junto com o TSE): o BU está no Urna Velox no instante do recebimento.

Ao instante em que o BU está no Urna Velox soma-se `--latencia-uv-ms` (totalização + publicação, medida no
replay). Por município:
* **matematicamente eleito**: primeiro instante em que a diferença entre o 1º e o 2º colocado (candidatos com destinação
  "Válido") supera o eleitorado ainda não apurado (`e.te` do arquivo oficial menos os aptos dos BUs já
  totalizados) — ninguém mais alcança o líder, mesmo que todos os eleitores restantes votem no 2º;
* **fim**: último BU do município totalizado.
O TSE é representado pela data/hora de totalização do arquivo oficial (`dt`/`ht`), que está na **hora local** da
UF (convertida para a de Brasília, a dos `aux.json` e `cs.json`). Municípios cuja data de totalização não é o dia
da eleição foram **retotalizados** depois: entram só na contagem. Quando o recebimento do `aux.json` é posterior à
disponibilização no `cs.json` (lotes recarregados, reenvios), vale a disponibilização.

    python3 simular_noite_municipal.py --dados ~/uvreal/2024-2t/dados --metricas ~/uvreal/2024-2t/metricas_2t.csv \
        --oficial ~/uvreal/2024-2t/oficial --eleicao 620 --cargo 11 --dia 27/10/2024 --saida ~/uvreal/2024-2t/sim
"""
import argparse, collections, csv, glob, json, math, os, statistics, struct, sys
from datetime import datetime

import asn1tools

AQUI = os.path.dirname(os.path.abspath(__file__))
CARGOS = {"prefeito": 11, "vereador": 13}
# diferença da hora local para a de Brasília (sem horário de verão desde 2019)
FUSO = {"ac": -2, "am": -1, "mt": -1, "ms": -1, "ro": -1, "rr": -1}


def seg(s):
    return datetime.strptime(s.strip(), "%d/%m/%Y %H:%M:%S").timestamp()


def hms(t):
    return datetime.fromtimestamp(t).strftime("%H:%M:%S")


def ler_bus(dados, spec, eleicao, cargo):
    """{(uf, mun, zona, seção): (aptos, {número: votos})} só com os votos nominais do cargo."""
    conv = asn1tools.compile_files(spec, codec="ber")
    out = {}
    for arq in sorted(glob.glob(os.path.join(dados, "*.uvpk"))):
        uf = os.path.basename(arq)[:2]
        b = open(arq, "rb").read()
        p = 8
        while p + 52 <= len(b):
            n, _pl, mu, zn, se = struct.unpack_from("<IIIHH", b, p)
            bu = b[p + 52:p + 52 + n]
            p += 52 + n
            ent = conv.decode("EntidadeBoletimUrna", conv.decode("EntidadeEnvelopeGenerico", bu)["conteudo"])
            aptos, votos = 0, collections.Counter()
            for el in ent["resultadosVotacaoPorEleicao"]:
                if el["idEleicao"] != eleicao:
                    continue
                aptos = el["qtdEleitoresAptos"]
                for rv in el["resultadosVotacao"]:
                    for tc in rv["totaisVotosCargo"]:
                        k, v = tc["codigoCargo"]
                        c = CARGOS.get(v, v) if k == "cargoConstitucional" else v
                        if c != cargo:
                            continue
                        for vv in tc["votosVotaveis"]:
                            if vv["tipoVoto"] == "nominal":
                                votos[vv["identificacaoVotavel"]["codigo"]] += vv["quantidadeVotos"]
            out[(uf, mu, zn, se)] = (aptos, votos)
    return out


def oficial(pasta, eleicao, cargo):
    out = {}
    for p in glob.glob(os.path.join(pasta, f"*-c{cargo:04d}-e{eleicao:06d}-u.json")):
        o = json.load(open(p, encoding="utf-8"))
        nome = os.path.basename(p)
        cands = {}
        def vis(x):
            if isinstance(x, dict):
                for c in x.get("cand", []):
                    cands[int(c["n"])] = (c.get("nmu") or c.get("nm"), c.get("dvt", ""), c.get("st", ""), int(c.get("vap") or 0))
                for v in x.values():
                    vis(v)
            elif isinstance(x, list):
                for v in x:
                    vis(v)
        vis(o["carg"][0])
        out[(nome[:2], int(nome[2:7]))] = {"te": int(o["e"]["te"]), "dt": o["dt"], "ht": o["ht"], "cand": cands, "tf": o.get("tf")}
    return out


def simular_site(secoes, rps, intervalo_cs, n_ufs, t0):
    """secoes: [(chave, t_disp, lat_s)] → {chave: instante em que o BU está no coletor}."""
    cap = (rps - n_ufs / intervalo_cs) / 2.0  # seções por segundo
    fila = sorted((t0 + math.ceil(max(0.0, td - t0) / intervalo_cs) * intervalo_cs, td, k, lat) for k, td, lat in secoes)
    out, livre = {}, t0
    for tdesc, _td, k, lat in fila:
        ini = max(tdesc, livre)
        livre = ini + 1.0 / cap
        out[k] = ini + lat
    return out


def definir(bus_ord, te, aptos_cand, uv_ms):
    """bus_ord: [(t, aptos, votos)] em ordem de chegada → (t_matematicamente_eleito, líder, t_fim)."""
    tot = collections.Counter()
    apurado = 0
    mat_eleito = lider = None
    for t, aptos, votos in bus_ord:
        for c, q in votos.items():
            if c in aptos_cand:
                tot[c] += q
        apurado += aptos
        if mat_eleito is None:
            top = sorted(aptos_cand, key=lambda c: -tot[c])
            l, s = tot[top[0]], (tot[top[1]] if len(top) > 1 else 0)
            if l - s > max(0, te - apurado):
                mat_eleito, lider = t + uv_ms / 1000, top[0]
    return mat_eleito, lider, bus_ord[-1][0] + uv_ms / 1000


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--dados", required=True)
    ap.add_argument("--metricas", required=True)
    ap.add_argument("--oficial", required=True)
    ap.add_argument("--eleicao", type=int, required=True)
    ap.add_argument("--cargo", type=int, default=11)
    ap.add_argument("--dia", required=True, help="dia da eleição, dd/mm/aaaa")
    ap.add_argument("--inicio", default="17:00:00", help="início da coleta (fim da votação), hh:mm:ss")
    ap.add_argument("--rps", type=float, default=95.0)
    ap.add_argument("--intervalo-cs", type=float, default=30.0)
    ap.add_argument("--latencia-uv-ms", type=float, default=5.0)
    ap.add_argument("--spec", default=os.path.join(AQUI, "..", "spec", "bu.asn1"))
    ap.add_argument("--saida", required=True)
    a = ap.parse_args()
    os.makedirs(a.saida, exist_ok=True)
    t0 = seg(f"{a.dia} {a.inicio}")

    bus = ler_bus(a.dados, a.spec, a.eleicao, a.cargo)
    ofi = oficial(a.oficial, a.eleicao, a.cargo)
    met = {}
    for r in csv.DictReader(open(a.metricas, encoding="utf-8")):
        if r["erro"] or not r["recebido_tse"].strip():
            continue
        k = (r["uf"], int(r["municipio"]), int(r["zona"]), int(r["secao"]))
        met[k] = r
    # disponibilização: a do cs.json quando é do dia da eleição; senão (seção republicada), recebimento + atraso mediano
    atrasos = [seg(r["disponivel_cs"]) - seg(r["recebido_tse"]) for r in met.values()
               if r["disponivel_cs"].startswith(a.dia) and r["recebido_tse"].startswith(a.dia)]
    atraso_med = statistics.median(atrasos) if atrasos else 0.0
    # O recebimento do aux.json é o do conjunto de arquivos escolhido; em reenvios e recargas ele pode ser
    # posterior à disponibilização (ex.: ES 56995, zona 53: recebimento 20:12:24 em lote, disponível às 17:45).
    # O BU já estava no TSE quando ficou disponível: recebimento efetivo = o menor dos dois.
    secoes, rec, estimadas, corrigidas = [], {}, 0, 0
    for k, r in met.items():
        if k not in bus:
            continue
        tr = seg(r["recebido_tse"])
        if r["disponivel_cs"].startswith(a.dia):
            td = seg(r["disponivel_cs"])
            if td < tr:
                tr, corrigidas = td, corrigidas + 1
        else:
            td, estimadas = tr + atraso_med, estimadas + 1
        rec[k] = tr
        lat = (float(r["lat_aux_ms"] or 0) + float(r["lat_bu_ms"] or 0)) / 1000
        secoes.append((k, td, lat))
    n_ufs = len({k[0] for k in bus})
    site = simular_site(secoes, a.rps, a.intervalo_cs, n_ufs, t0)

    # CSV para o replay (horário em que o BU está no Urna Velox, cenário site)
    with open(os.path.join(a.saida, "chegadas_site.csv"), "w") as f:
        f.write("uf,municipio,zona,secao,recebido_tse\n")
        for (uf, mu, zn, se), t in site.items():
            f.write(f"{uf},{mu},{zn},{se},{datetime.fromtimestamp(t).strftime('%d/%m/%Y %H:%M:%S')}\n")

    linhas, cont = [], collections.Counter()
    porm = collections.defaultdict(list)
    for k in bus:
        if k in site:
            porm[(k[0], k[1])].append(k)
    for (uf, mu), o in sorted(ofi.items()):
        ks = porm.get((uf, mu), [])
        aptos_cand = [c for c, x in o["cand"].items() if x[1].startswith("Válido")]
        eleito = [c for c, x in o["cand"].items() if x[2] == "Eleito"]
        retot = o["dt"] != a.dia
        if retot:
            cont["retotalizados"] += 1
        if not ks or len(aptos_cand) < 1:
            cont["sem_dados"] += 1
            continue
        res = {}
        for cen, tempo in (("site", site), ("rede", rec)):
            ordem = sorted((tempo[k], bus[k][0], bus[k][1]) for k in ks)
            res[cen] = definir(ordem, o["te"], aptos_cand, a.latencia_uv_ms)
        # dt/ht do arquivo oficial estão na hora local da UF; recebimento e disponibilização, na de Brasília
        tse = seg(f"{o['dt']} {o['ht']}") - 3600 * FUSO.get(uf, 0)
        ok = bool(eleito) and res["site"][1] == eleito[0] and res["rede"][1] == eleito[0]
        cont["lider_ok" if ok else "lider_dif"] += 1
        linhas.append({"uf": uf, "municipio": mu, "eleitorado": o["te"], "secoes": len(ks), "retotalizado": retot,
                       "tse_totalizacao": f"{o['dt']} {o['ht']}", "tse_brasilia": hms(tse), "ultimo_recebimento": hms(max(rec[k] for k in ks)),
                       "site_mat_eleito": hms(res["site"][0]) if res["site"][0] else "", "site_fim": hms(res["site"][2]),
                       "rede_mat_eleito": hms(res["rede"][0]) if res["rede"][0] else "", "rede_fim": hms(res["rede"][2]),
                       "antecedencia_site_mat_eleito_s": None if retot or not res["site"][0] else round(tse - res["site"][0]),
                       "antecedencia_site_fim_s": None if retot else round(tse - res["site"][2]),
                       "antecedencia_rede_mat_eleito_s": None if retot or not res["rede"][0] else round(tse - res["rede"][0]),
                       "antecedencia_rede_fim_s": None if retot else round(tse - res["rede"][2]),
                       "lider_confere": ok})
    with open(os.path.join(a.saida, "municipios.csv"), "w") as f:
        w = csv.DictWriter(f, fieldnames=list(linhas[0].keys()))
        w.writeheader()
        w.writerows(linhas)

    def est(campo):
        v = sorted(l[campo] for l in linhas if l[campo] is not None)
        if not v:
            return {}
        return {"n": len(v), "min": v[0], "p50": statistics.median(v), "max": v[-1], "media": round(sum(v) / len(v)),
                "negativos": sum(1 for x in v if x < 0)}
    resumo = {"municipios_oficiais": len(ofi), "municipios_simulados": len(linhas), "retotalizados": cont["retotalizados"],
              "lider_confere": cont["lider_ok"], "lider_diverge": cont["lider_dif"], "secoes": len(site),
              "secoes_com_disponibilizacao_estimada": estimadas, "secoes_recebimento_corrigido_pela_disponibilizacao": corrigidas, "atraso_mediano_recebimento_disponibilizacao_s": atraso_med,
              "parametros": {"rps": a.rps, "intervalo_cs": a.intervalo_cs, "latencia_uv_ms": a.latencia_uv_ms, "inicio": a.inicio},
              "ultimo_bu_no_urna_velox_site": hms(max(site.values())),
              "antecedencia_s": {c: est(c) for c in ("antecedencia_site_mat_eleito_s", "antecedencia_site_fim_s",
                                                      "antecedencia_rede_mat_eleito_s", "antecedencia_rede_fim_s")}}
    json.dump(resumo, open(os.path.join(a.saida, "resumo.json"), "w"), ensure_ascii=False, indent=1)
    print(json.dumps(resumo, ensure_ascii=False, indent=1))


if __name__ == "__main__":
    main()
