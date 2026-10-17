#!/usr/bin/env python3
"""Compara a ordem em que o uv-servico aplicou os BUs (diário `secoes.log`) com a ordem de recebimento no TSE.

    python3 ordem_totalizacao.py <saida>/secoes.log [--ordem ordem_3220.csv] [--md relatorio.md]

A ordem aplicada é a das linhas `aplicado` do diário (a primeira de cada seção). O recebimento no TSE vem da coluna
`recebido` do diário (lida do aux.json na coleta) ou, com --ordem, de um CSV de métricas (`uf,municipio,zona,secao,
recebido_tse`, o mesmo do `uv-servico --ordem`). Mede:

* **fora de ordem**: BUs aplicados depois de um BU recebido mais tarde no TSE, e por quanto (segundos de recebimento);
* **deslocamento**: distância entre a posição na ordem aplicada e na ordem de recebimento (p50, p99, máximo);
* **trocas vizinhas**: pares consecutivos na ordem aplicada que estão invertidos no recebimento;
* a ordem em que as UFs começaram a ser totalizadas, ao lado da ordem em que o TSE recebeu o primeiro BU de cada uma.
"""
import argparse
import sys
from datetime import datetime


def segundos(txt):
    txt = (txt or '').strip()
    if not txt:
        return None
    try:
        return int(datetime.strptime(txt, '%d/%m/%Y %H:%M:%S').timestamp())
    except ValueError:
        return None


def pct(v, p):
    if not v:
        return 0
    s = sorted(v)
    return s[min(len(s) - 1, int(round(p / 100 * (len(s) - 1))))]


def ler_ordem(caminho):
    ref = {}
    with open(caminho, encoding='utf-8') as f:
        cab = f.readline().strip().split(',')
        c = {n: i for i, n in enumerate(cab)}
        for linha in f:
            x = linha.rstrip('\n').split(',')
            t = segundos(x[c['recebido_tse']])
            if t is not None:
                ref[(x[c['uf']], int(x[c['municipio']]), int(x[c['zona']]), int(x[c['secao']]))] = t
    return ref


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('diario', nargs='+', help='secoes.log de um nó (vários: um relatório por nó)')
    ap.add_argument('--ordem', help='CSV de métricas com recebido_tse (referência; padrão: a coluna do diário)')
    ap.add_argument('--md', help='grava o relatório em Markdown neste arquivo (padrão: só na tela)')
    a = ap.parse_args()
    ref = ler_ordem(a.ordem) if a.ordem else None
    saida = []
    for caminho in a.diario:
        vistos, aplic = set(), []  # (chave, recebido)
        sem = 0
        with open(caminho, encoding='utf-8', errors='replace') as f:
            for linha in f:
                x = linha.rstrip('\n').split(',')
                if len(x) < 16 or x[1] != 'aplicado':
                    continue
                k = (x[2], int(x[3]), int(x[4]), int(x[5]))
                if k in vistos:
                    continue
                vistos.add(k)
                t = ref.get(k) if ref is not None else segundos(x[11])
                if t is None:
                    sem += 1
                    continue
                aplic.append((k, t))
        n = len(aplic)
        saida.append(f'## {caminho}\n')
        if n == 0:
            saida.append('Nenhum BU aplicado com horário de recebimento.\n')
            continue
        # fora de ordem: recebido antes do maior recebimento já aplicado
        maior, atrasos = None, []
        for _, t in aplic:
            if maior is not None and t < maior:
                atrasos.append(maior - t)
            maior = t if maior is None else max(maior, t)
        # deslocamento de posição (empate no recebimento: a ordem estável do próprio recebimento)
        pos_ref = {k: i for i, (k, _) in enumerate(sorted(aplic, key=lambda z: (z[1], z[0])))}
        desl = [abs(i - pos_ref[k]) for i, (k, _) in enumerate(aplic)]
        trocas = sum(1 for i in range(1, n) if aplic[i][1] < aplic[i - 1][1])
        # UFs: começo da totalização × primeiro recebimento no TSE
        ini_aplic, ini_tse = {}, {}
        for i, (k, t) in enumerate(aplic):
            ini_aplic.setdefault(k[0], i)
            ini_tse[k[0]] = min(ini_tse.get(k[0], t), t)
        ufs_aplic = sorted(ini_aplic, key=ini_aplic.get)
        ufs_tse = sorted(ini_tse, key=lambda u: (ini_tse[u], u))
        fora = len(atrasos)
        saida += [
            f'- BUs aplicados com horário de recebimento: **{n:,}**'.replace(',', '.') + (f' ({sem} sem horário, fora da conta)' if sem else ''),
            f'- **Fora de ordem**: {fora:,} ({100 * fora / n:.2f} %)'.replace(',', '.')
            + (f'; atraso no recebimento p50 {pct(atrasos, 50)} s, p99 {pct(atrasos, 99)} s, máximo {max(atrasos)} s' if atrasos else ''),
            f'- **Deslocamento** de posição: p50 {pct(desl, 50)}, p99 {pct(desl, 99)}, máximo {max(desl)}',
            f'- **Trocas vizinhas**: {trocas:,} de {n - 1:,} pares ({100 * trocas / max(1, n - 1):.2f} %)'.replace(',', '.'),
            f'- UFs na ordem em que começaram a ser totalizadas: {" ".join(u.upper() for u in ufs_aplic)}',
            f'- UFs na ordem do primeiro BU recebido no TSE:     {" ".join(u.upper() for u in ufs_tse)}',
            '',
        ]
    txt = '\n'.join(saida)
    print(txt)
    if a.md:
        with open(a.md, 'w', encoding='utf-8') as f:
            f.write('# Ordem de totalização × recebimento no TSE\n\n' + txt + '\n')


if __name__ == '__main__':
    sys.exit(main())
