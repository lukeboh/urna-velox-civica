#!/usr/bin/env python3
"""Confere os números lidos da TELA (numeros.json do testes/rodar.mjs) com os arquivos oficiais de divulgação do TSE
(-u.json): votos de cada candidato (vap), seções apuradas/total (s.st/s.ts), comparecimento (e.c), brancos (v.vb) e
nulos (v.vn + v.vnt). Uso: conferir.py <numeros.json> <pasta dos -u.json> [--saida conferencia_pagina.json]"""
import glob, json, os, sys

def inteiro(x):
    try:
        return int(str(x).replace('.', '').strip() or 0)
    except ValueError:
        return None

def oficial(pasta, topico):
    abr, c = topico.rsplit('-c', 1)
    fs = glob.glob(os.path.join(pasta, f'{abr}-c{c}-e*-u.json'))
    if not fs:
        return None
    v = json.load(open(fs[0], encoding='utf-8'))
    cand = {}
    for cg in v.get('carg', []):
        for a in cg.get('agr', []):
            for p in a.get('par', []):
                for k in p.get('cand', []):
                    cand[int(k['n'])] = inteiro(k.get('vap'))
    for pg in v.get('perg', []):  # consultas
        for r in pg.get('resp', []):
            cand[int(r['n'])] = inteiro(r.get('vap'))
    s, e, vv = v.get('s', {}), v.get('e', {}), v.get('v', {})
    return {'arquivo': os.path.basename(fs[0]), 'cand': cand, 'secoes': inteiro(s.get('st')), 'total': inteiro(s.get('ts')),
            'comparecimento': inteiro(e.get('c')), 'brancos': inteiro(vv.get('vb')), 'nulos': (inteiro(vv.get('vn')) or 0) + (inteiro(vv.get('vnt')) or 0)}

def main():
    numeros = json.load(open(sys.argv[1], encoding='utf-8'))
    pasta = sys.argv[2]
    saida = sys.argv[sys.argv.index('--saida') + 1] if '--saida' in sys.argv else None
    iguais, difs, sem = 0, [], []
    for t, tela in sorted(numeros.items()):
        of = oficial(pasta, t)
        if of is None or tela is None:
            sem.append(t)
            continue
        d = []
        for k in ('secoes', 'total', 'comparecimento', 'brancos', 'nulos'):
            if tela.get(k) != of[k]:
                d.append(f'{k}: tela {tela.get(k)} × oficial {of[k]}')
        vt = {c['n']: c['v'] for c in tela['cand']}
        for n, v in of['cand'].items():
            if vt.get(n) != v:
                d.append(f'candidato {n}: tela {vt.get(n)} × oficial {v}')
        # a ordem na tela (posição de cada linha) segue os votos
        ordem = [c['n'] for c in sorted(tela['cand'], key=lambda c: c['pos'])]
        votos = [vt[n] for n in ordem]
        if votos != sorted(votos, reverse=True):
            d.append(f'ordem na tela não segue os votos: {ordem}')
        if d:
            difs.append({'topico': t, 'arquivo': of['arquivo'], 'difs': d})
        else:
            iguais += 1
    r = {'topicos': len(numeros), 'identicos': iguais, 'divergentes': difs, 'sem_oficial': sem}
    print(f'tela × oficial: {iguais}/{len(numeros) - len(sem)} idênticos; {len(difs)} divergentes; sem oficial: {len(sem)}')
    for x in difs[:20]:
        print(' ', x['topico'], '; '.join(x['difs'][:5]))
    if saida:
        json.dump(r, open(saida, 'w', encoding='utf-8'), ensure_ascii=False, indent=1)
    sys.exit(1 if difs else 0)

if __name__ == '__main__':
    main()
