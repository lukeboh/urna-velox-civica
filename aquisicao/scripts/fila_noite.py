#!/usr/bin/env python3
"""M-002: fila da noite real para uma capacidade de coleta dada, a partir da chegada dos BUs ao TSE (linha do tempo).

    python3 aquisicao/scripts/fila_noite.py <linha-do-tempo.csv> [--json saida.json]

Cada BU entra na fila no `recebido_tse` (a hora em que chegou ao TSE; disponível no cs.json pouco depois) e sai à
capacidade da federação: nós × teto por IP (100 req/s) ÷ requisições por BU (2 pela API: aux.json + BU; 1 se o
aux.json deixar de ser preciso), descontado o cs.json (UFs ÷ 30 s por nó). Para cada cenário: atraso máximo de um
BU (chegada ao TSE → coletado), quando acontece, atraso p50/p99 e a hora em que a fila zera depois do pico.
Simulação em passos de 1 s, só a coleta (a totalização é ~instantânea: ADR/AQUISICAO).
"""
import csv
import json
import sys
from collections import Counter, deque
from datetime import datetime, timedelta

CENARIOS = [  # (nome, nós com IP próprio, requisições por BU)
    ('1 nó, API (aux + BU)', 1, 2),
    ('2 nós, API (aux + BU)', 2, 2),
    ('3 nós, API (aux + BU)', 3, 2),
    ('4 nós, API (aux + BU)', 4, 2),
    ('1 nó, 1 req por BU', 1, 1),
    ('2 nós, 1 req por BU', 2, 1),
]
TETO = 100.0      # req/s por IP de saída (ADR-015/ADR-028)
UFS = 28          # cs.json por UF a cada 30 s, em cada nó


def ler(p):
    chegada, ufs = Counter(), set()
    with open(p, encoding='utf-8') as f:
        r = csv.DictReader(f)
        for row in r:
            v = (row.get('recebido_tse') or '').strip()
            if not v:
                continue
            try:
                t = datetime.strptime(v, '%d/%m/%Y %H:%M:%S')
            except ValueError:
                continue
            chegada[t] += 1
            ufs.add(row.get('uf'))
    return chegada, len(ufs) or UFS


def simular(chegada, nos, req_bu, ufs):
    cap = nos * (TETO - ufs / 30.0) / req_bu          # BUs/s que a federação coleta
    tempos = sorted(chegada)
    fila = deque()                                    # [instante de chegada, quantos]
    atrasos = Counter()                               # atraso em s → BUs
    t, i, credito = tempos[0], 0, 0.0
    maior_fila, t_maior_fila, maior_atraso, t_maior_atraso = 0, t, 0, t
    tamanho, zerou_depois_do_pico = 0, None
    while i < len(tempos) or fila:
        if not fila and i < len(tempos) and tempos[i] > t:
            t, credito = tempos[i], 0.0               # nada na fila: salta até a próxima chegada
        while i < len(tempos) and tempos[i] <= t:
            fila.append([tempos[i], chegada[tempos[i]]])
            tamanho += chegada[tempos[i]]
            i += 1
        credito += cap
        while fila and credito >= 1:
            ent = fila[0]
            k = min(ent[1], int(credito))
            a = int((t - ent[0]).total_seconds())
            atrasos[a] += k
            if a > maior_atraso:
                maior_atraso, t_maior_atraso = a, t
            ent[1] -= k
            tamanho -= k
            credito -= k
            if ent[1] == 0:
                fila.popleft()
        if not fila:
            credito = 0.0                             # capacidade ociosa não acumula
        if tamanho > maior_fila:
            maior_fila, t_maior_fila, zerou_depois_do_pico = tamanho, t, None
        elif tamanho == 0 and zerou_depois_do_pico is None:
            zerou_depois_do_pico = t
        t += timedelta(seconds=1)
    total = sum(atrasos.values())

    def q(p):
        acc = 0
        for a in sorted(atrasos):
            acc += atrasos[a]
            if acc >= p * total:
                return a
        return None
    return {'capacidade_bu_s': round(cap, 1), 'fila_max': maior_fila, 'fila_max_em': f'{t_maior_fila:%d/%m %H:%M}',
            'atraso_max_s': maior_atraso, 'atraso_max_em': f'{t_maior_atraso:%d/%m %H:%M}',
            'atraso_p50_s': q(0.5), 'atraso_p99_s': q(0.99),
            'fila_zera_em': f'{zerou_depois_do_pico:%d/%m %H:%M}' if zerou_depois_do_pico else None}


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(2)
    chegada, ufs = ler(sys.argv[1])
    pico = max(chegada.values())
    por_min = Counter()
    for t, n in chegada.items():
        por_min[t.replace(second=0)] += n
    tm, nm = por_min.most_common(1)[0]
    res = {'arquivo': sys.argv[1], 'bus': sum(chegada.values()), 'ufs': ufs,
           'pico_1min_bu_s': round(nm / 60, 1), 'pico_1min_em': f'{tm:%d/%m %H:%M}', 'pico_1s': pico, 'cenarios': {}}
    print(f"{res['bus']} BUs; pico de chegada {res['pico_1min_bu_s']} BU/s (1 min, {res['pico_1min_em']}); teto {TETO:.0f} req/s por IP")
    for nome, nos, req in CENARIOS:
        r = simular(chegada, nos, req, ufs)
        res['cenarios'][nome] = r
        print(f"  {nome:24} {r['capacidade_bu_s']:6.1f} BU/s · fila máx. {r['fila_max']:6d} ({r['fila_max_em']}) · atraso máx. "
              f"{r['atraso_max_s'] / 60:5.1f} min ({r['atraso_max_em']}) · p50 {r['atraso_p50_s']} s · p99 {r['atraso_p99_s'] / 60:.1f} min · fila zera {r['fila_zera_em']}")
    if '--json' in sys.argv:
        with open(sys.argv[sys.argv.index('--json') + 1], 'w', encoding='utf-8') as f:
            json.dump(res, f, ensure_ascii=False, indent=1)


if __name__ == '__main__':
    main()
