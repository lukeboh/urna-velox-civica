#!/usr/bin/env python3
"""B-067: procura a regra da borda que explica os 429 de 25/10/2026 no registro de um ensaio sem limite.

    python3 aquisicao/scripts/b067_regras.py <pasta do ensaio> [--saida <pasta>]

Lê o `simulador.log` (`t_ms,simulado,tipo,status,bytes,caminho,ip`) de um `ensaio_b067.sh` rodado sem
`--limite-ip-rps` (nenhum 429, o comportamento dos nós é o da noite até o primeiro 429 real) e, para cada regra
candidata, calcula quando cada IP passaria do limite pela primeira vez (hora da noite). A regra que der o primeiro
429 do nó B perto de 19:47:57 e o do nó A perto de 19:53:50 (horários reais de 25/10) é a candidata a confirmar
com o simulador punindo de verdade. Grava também, por IP e por segundo da noite, as requisições por tipo e os 404
(`por_segundo.csv`), e os picos (`picos.json`).

Regras: blocos fixos e janela deslizante de W s (teto em req/s × W), 404 com peso P, atraso aleatório de rede na
chegada (a borda conta na chegada; o coletor espaça na saída) e "N 404 em 60 s" (CLAUDE.md: milhares de 404
seguidos provocam 429 mesmo abaixo de 100 req/s).
"""
import json
import sys
from datetime import datetime, timedelta
from pathlib import Path

import numpy as np

REAIS = {'127.0.0.3': '19:47:57', '127.0.0.2': '19:53:50'}   # nó B e nó A (1º 429 de cada um em 25/10)
NOME = {'127.0.0.2': 'no-a', '127.0.0.3': 'no-b'}


def ler(p):
    por_ip = {}
    with open(p, encoding='utf-8', errors='replace') as f:
        for ln in f:
            if ln.startswith('#'):
                continue
            c = ln.rstrip('\n').split(',')
            if len(c) < 7:
                continue
            ip = c[-1]
            d = por_ip.setdefault(ip, {'t': [], 'sim': [], 'tipo': [], 'st': [], 'uf': []})
            d['t'].append(float(c[0]) / 1000.0)
            d['sim'].append(c[1])
            d['tipo'].append(c[2])
            d['st'].append(int(c[3]))
            seg = c[5].split('/')
            d['uf'].append(seg[6] if len(seg) > 6 else '')
    out = {}
    for ip, d in por_ip.items():
        t = np.array(d['t'])
        o = np.argsort(t, kind='stable')
        out[ip] = {'t': t[o], 'sim': [d['sim'][i] for i in o], 'tipo': np.array(d['tipo'])[o], 'st': np.array(d['st'])[o],
                   'uf': np.array(d['uf'])[o]}
    return out


def relogio(d):
    """Função real (s) → hora da noite (datetime): o simulador anda a 1×; ancora no primeiro registro."""
    s0 = datetime.strptime(d['sim'][0], '%d/%m/%Y %H:%M:%S')
    t0 = d['t'][0]
    # corrige a fração: a hora simulada é truncada no segundo; usa a mediana da diferença das primeiras 2000
    n = min(2000, len(d['t']))
    difs = [(datetime.strptime(d['sim'][i], '%d/%m/%Y %H:%M:%S') - s0).total_seconds() - (d['t'][i] - t0) for i in range(n)]
    ajuste = float(np.median(difs)) + 0.5
    return lambda t: s0 + timedelta(seconds=(t - t0) + ajuste)


def pesos(d, p404):
    return np.where(d['st'] == 404, p404, 1.0)


def primeiro_excesso(t, w, regra, janela, teto):
    """Índice da primeira requisição em que o peso na janela passa de teto × janela (ou None)."""
    c = np.concatenate([[0.0], np.cumsum(w)])
    if regra == 'deslizante':
        j = np.searchsorted(t, t - janela, side='right')
        soma = c[1:] - c[j]
    else:
        b = np.floor(t / janela)
        ini = np.searchsorted(b, b, side='left')
        soma = c[1:] - c[ini]
    i = np.argmax(soma > teto * janela + 1e-9)
    return (int(i) if soma[i] > teto * janela + 1e-9 else None), float(soma.max())


def main():
    pasta = Path(sys.argv[1])
    saida = Path(sys.argv[sys.argv.index('--saida') + 1]) if '--saida' in sys.argv else pasta / 'analise-b067'
    saida.mkdir(parents=True, exist_ok=True)
    dados = ler(pasta / 'simulador.log')
    # --desde hh:mm:ss: ignora o começo do ensaio (a partida dos nós com a fila do início do relógio não existiu na
    # noite real, em que os nós estavam no ar havia horas)
    if '--desde' in sys.argv:
        desde = sys.argv[sys.argv.index('--desde') + 1]
        for ip, d in dados.items():
            keep = np.array([s[11:] >= desde for s in d['sim']])
            for k in ('t', 'tipo', 'st', 'uf'):
                d[k] = d[k][keep]
            d['sim'] = [s for s, m in zip(d['sim'], keep) if m]
    rng = np.random.default_rng(2026)
    linhas, picos = [], {}
    for ip, d in sorted(dados.items()):
        if ip not in NOME:
            continue
        h = relogio(d)
        t = d['t']
        real = datetime.strptime(d['sim'][0][:10] + ' ' + REAIS[ip], '%d/%m/%Y %H:%M:%S')
        # por segundo da noite: total, por tipo, 404, aux do exterior
        seg = np.floor(t - t[0]).astype(int)
        n = seg.max() + 1
        tab = {'total': np.bincount(seg, minlength=n)}
        for tp in ('cs', 'aux', 'bu', 'oficial', 'outro'):
            tab[tp] = np.bincount(seg[d['tipo'] == tp], minlength=n)
        tab['http404'] = np.bincount(seg[d['st'] == 404], minlength=n)
        tab['aux_zz'] = np.bincount(seg[(d['tipo'] == 'aux') & (d['uf'] == 'zz')], minlength=n)
        with open(saida / f'por_segundo_{NOME[ip]}.csv', 'w') as f:
            f.write('hora,' + ','.join(tab) + '\n')
            for s in range(n):
                f.write(h(t[0] + s).strftime('%H:%M:%S') + ',' + ','.join(str(int(v[s])) for v in tab.values()) + '\n')
        picos[NOME[ip]] = {'requisicoes': int(len(t)), 'http404': int((d['st'] == 404).sum()), 'max_seg_fixo': int(tab['total'].max()),
                           'max_1s_deslizante': primeiro_excesso(t, np.ones(len(t)), 'deslizante', 1.0, 1e9)[1],
                           'max_10s_deslizante_por_s': primeiro_excesso(t, np.ones(len(t)), 'deslizante', 10.0, 1e9)[1] / 10,
                           'max_404_60s': primeiro_excesso(t, (d['st'] == 404).astype(float), 'deslizante', 60.0, 1e9)[1]}
        for jit in (0.0, 0.05, 0.15):
            # chegada à borda = saída + atraso de rede uniforme em [0, jit] s (reordena as requisições)
            chegada = t + rng.uniform(0, jit, len(t)) if jit else t
            ordem = np.argsort(chegada, kind='stable')
            tj = chegada[ordem]
            # 5 s e 120 s: as janelas de "rajada" e de "média" dos controles de taxa da Akamai (a borda do TSE)
            for regra, janela in (('fixa', 1.0), ('deslizante', 1.0), ('deslizante', 0.5), ('fixa', 10.0), ('deslizante', 10.0),
                                  ('deslizante', 5.0), ('deslizante', 120.0)):
                for teto in (80.0, 85.0, 90.0, 95.0, 100.0):
                    for p404 in (1.0, 2.0, 5.0):
                        w = pesos(d, p404)[ordem]
                        i, mx = primeiro_excesso(tj, w, regra, janela, teto)
                        quando = h(tj[i]) if i is not None else None
                        linhas.append({'no': NOME[ip], 'atraso_rede_s': jit, 'regra': f'{regra}:{janela:g}', 'teto': teto, 'peso_404': p404,
                                       'primeiro_429': quando.strftime('%H:%M:%S') if quando else '', 'max_por_s': round(mx / janela, 1),
                                       'dif_real_s': round((quando - real).total_seconds()) if quando else None})
        so404 = (d['st'] == 404).astype(float)
        for k in (60, 100, 200, 300, 500, 1000):
            i, mx = primeiro_excesso(t, so404, 'deslizante', 60.0, k / 60.0)
            quando = h(t[i]) if i is not None else None
            linhas.append({'no': NOME[ip], 'atraso_rede_s': 0.0, 'regra': '404 em 60 s', 'teto': k, 'peso_404': None,
                           'primeiro_429': quando.strftime('%H:%M:%S') if quando else '', 'max_por_s': mx,
                           'dif_real_s': round((quando - real).total_seconds()) if quando else None})
    json.dump({'picos': picos, 'regras': linhas}, open(saida / 'regras.json', 'w'), ensure_ascii=False, indent=1)
    print(json.dumps(picos, ensure_ascii=False, indent=1))
    # regras em que os dois nós passam do limite a menos de 3 min do real
    ok = {}
    for x in linhas:
        k = (x['atraso_rede_s'], x['regra'], x['teto'], x['peso_404'])
        ok.setdefault(k, {})[x['no']] = x
    print('\nregra · teto · peso 404 · atraso rede → 1º 429 no-a (dif) · no-b (dif)')
    for k, v in sorted(ok.items(), key=lambda kv: sum(abs(x['dif_real_s']) if x['dif_real_s'] is not None else 1e6 for x in kv[1].values())):
        a, b = v.get('no-a', {}), v.get('no-b', {})
        print(f"{k[1]:14} {k[2]:6g} {str(k[3]):4} {k[0]:4g} → {a.get('primeiro_429') or '—':8} ({a.get('dif_real_s')}) · {b.get('primeiro_429') or '—':8} ({b.get('dif_real_s')})")


if __name__ == '__main__':
    main()
