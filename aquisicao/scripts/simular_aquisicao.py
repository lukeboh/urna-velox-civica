#!/usr/bin/env python3
"""Simula a aquisição na noite da eleição com uma curva de recebimento por minuto (CSV
`minuto;recebimento`, por exemplo contada a partir do `recebido_tse` dos `aux.json`).

Modelo: fila FIFO (fluido, resolução de 1 s). Os BUs chegam ao TSE na taxa de cada minuto (distribuída
uniformemente no minuto) e ficam disponíveis no CDN depois de `--atraso-cdn` segundos (tempo até o BU
aparecer no CDN). O coletor descobre a seção ao reler o cs.json a cada `--intervalo-cs` segundos e baixa
2 requisições por seção (aux.json + BU) com teto de `--rps` req/s. A totalização do Urna Velox é somada
como `--latencia-uv-ms` (medida no replay).

    python3 simular_aquisicao.py --minutos chegada_por_minuto.csv --rps 100 --saida sim.json
"""
import argparse, csv, json, math


def percentil(v, q):
    if not v:
        return 0.0
    k = (len(v) - 1) * q / 100
    i = int(k)
    return v[i] if i + 1 >= len(v) else v[i] + (v[i + 1] - v[i]) * (k - i)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--minutos", required=True)
    ap.add_argument("--rps", type=float, default=100.0)
    ap.add_argument("--req-por-secao", type=float, default=2.0)
    ap.add_argument("--eficiencia", type=float, default=0.95, help="fração do teto efetivamente usada (medido: 95 de 100 req/s)")
    ap.add_argument("--atraso-cdn", type=float, default=0.0, help="s entre o recebimento no TSE e o BU no CDN")
    ap.add_argument("--intervalo-cs", type=float, default=30.0, help="s entre releituras do cs.json")
    ap.add_argument("--latencia-uv-ms", type=float, default=1.0)
    ap.add_argument("--saida")
    a = ap.parse_args()

    linhas = list(csv.DictReader(open(a.minutos, encoding="utf-8"), delimiter=";"))
    cap = a.rps * a.eficiencia / a.req_por_secao  # seções por segundo
    # chegada por segundo (uniforme dentro do minuto), deslocada pelo atraso do CDN e pela descoberta via cs.json
    T = len(linhas) * 60 + int(a.atraso_cdn) + int(a.intervalo_cs) + 10
    chega = [0.0] * (T + 1)
    for i, l in enumerate(linhas):
        n = int(l["recebimento"])
        for s in range(60):
            t = i * 60 + s + a.atraso_cdn
            # descoberta no próximo ciclo de releitura do cs.json
            t = math.ceil(t / a.intervalo_cs) * a.intervalo_cs if a.intervalo_cs > 0 else t
            chega[min(int(t), T)] += n / 60
    # fila fluida: atende até cap seções por segundo
    fila, sai = 0.0, [0.0] * (T * 3)
    t = 0
    while t < len(sai):
        if t <= T:
            fila += chega[t]
        x = min(fila, cap)
        sai[t] = x
        fila -= x
        t += 1
        if t > T and fila <= 1e-9:
            break
    # atraso por BU: compara as curvas acumuladas de recebimento no TSE e de coleta (FIFO)
    rec = []
    for i, l in enumerate(linhas):
        n = int(l["recebimento"])
        rec += [i * 60 + (s + 0.5) * 60 / max(n, 1) for s in range(n)]
    atrasos, acum, k = [], 0.0, 0
    for tt, x in enumerate(sai):
        acum += x
        while k < len(rec) and k + 1 <= acum + 1e-6:
            atrasos.append(tt + 1 - rec[k] + a.latencia_uv_ms / 1000)
            k += 1
    atrasos.sort()
    # série por minuto (coletados = disponíveis no Urna Velox)
    por_min = [0.0] * (len(sai) // 60 + 1)
    for tt, x in enumerate(sai):
        por_min[tt // 60] += x
    inicio = linhas[0]["minuto"]
    backlog_max, b, tb = 0.0, 0.0, 0
    cum_rec = cum_sai = 0.0
    for m in range(len(por_min)):
        cum_rec += int(linhas[m]["recebimento"]) if m < len(linhas) else 0
        cum_sai += por_min[m]
        if cum_rec - cum_sai > backlog_max:
            backlog_max, tb = cum_rec - cum_sai, m
    r = {
        "parametros": vars(a) | {"capacidade_secoes_s": round(cap, 2), "capacidade_bus_min": round(cap * 60)},
        "bus": len(rec), "inicio": inicio,
        "pico_recebimento_bus_min": max(int(l["recebimento"]) for l in linhas),
        "backlog_max_bus": round(backlog_max), "backlog_max_minuto": tb,
        "atraso_s": {q: round(percentil(atrasos, p), 1) for q, p in [("p50", 50), ("p90", 90), ("p95", 95), ("p99", 99), ("max", 100)]}
                    | {"media": round(sum(atrasos) / len(atrasos), 1)},
        "minutos_coleta": [round(x) for x in por_min],
    }
    print(json.dumps({k: v for k, v in r.items() if k != "minutos_coleta"}, ensure_ascii=False, indent=1))
    if a.saida:
        json.dump(r, open(a.saida, "w", encoding="utf-8"), ensure_ascii=False)


if __name__ == "__main__":
    main()
