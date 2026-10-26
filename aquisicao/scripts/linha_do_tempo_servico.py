#!/usr/bin/env python3
"""Linha do tempo de uma noite real a partir do diário de seções do uv-servico (saida/secoes.log).

  python3 linha_do_tempo_servico.py <secoes.log> <linha-do-tempo.csv>

Grava no formato das métricas da coleta (o mesmo das linhas do tempo do 452, 453 e 3220, lido pelo
`uv-simulador --metricas` e pelo `uv-servico --ordem`), uma linha por seção, em ordem de recebimento no TSE:
  t_s,uf,municipio,zona,secao,lat_aux_ms,lat_bu_ms,bytes_bu,tentativas,situacao,recebido_tse,disponivel_cs,erro,fonte_recebido
* recebido_tse / disponivel_cs / situacao / bytes_bu: do primeiro BU aplicado da seção (o da noite);
* não instaladas (acao nao_instalada): erro "sem BU (situação Nao instalada)", como o uv-coletor grava;
* t_s: segundos do primeiro aplicado desde o início do diário (as latências da coleta não existem aqui: 0).
"""
import csv, datetime as dt, sys

ent, sai = sys.argv[1], sys.argv[2]
FUSO = dt.timezone(dt.timedelta(hours=-3))
sec, ordem, t0 = {}, [], None
with open(ent, newline="") as f:
    for r in csv.DictReader(f):
        k = (r["uf"], r["municipio"], r["zona"], r["secao"])
        t = int(r["t_ms"]) / 1000
        t0 = t if t0 is None else min(t0, t)
        if r["acao"] == "nao_instalada" and k not in sec:
            sec[k] = dict(t=t, situacao="Nao instalada", rec="", disp="", tam="0", erro="sem BU (situação Nao instalada)")
        elif r["acao"] == "aplicado" and (k not in sec or not sec[k]["rec"]):
            sit = "Excluído" if r.get("anulada") == "1" else r["situacao"]
            sec[k] = dict(t=t, situacao=sit, rec=r["recebido"], disp=r["disponivel"], tam=r["tam"], erro="")


def chave_ordem(item):
    k, v = item
    if v["rec"]:
        return (0, dt.datetime.strptime(v["rec"], "%d/%m/%Y %H:%M:%S").timestamp(), k)
    return (1, v["t"], k)


n = 0
with open(sai, "w", newline="") as f:
    f.write("t_s,uf,municipio,zona,secao,lat_aux_ms,lat_bu_ms,bytes_bu,tentativas,situacao,recebido_tse,disponivel_cs,erro,fonte_recebido\n")
    for (uf, mu, zn, se), v in sorted(sec.items(), key=chave_ordem):
        f.write(f"{v['t'] - t0:.4f},{uf},{mu},{zn},{se},0,0,{v['tam']},1,{v['situacao']},{v['rec']},{v['disp']},{v['erro']},"
                "secoes.log do uv-servico\n")
        n += 1
sit = {}
for v in sec.values():
    sit[v["situacao"]] = sit.get(v["situacao"], 0) + 1
print(f"{n} seções gravadas em {sai}; situações: {sit}; com recebimento: {sum(1 for v in sec.values() if v['rec'])}")
