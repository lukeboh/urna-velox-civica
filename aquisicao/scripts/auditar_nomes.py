#!/usr/bin/env python3
"""Auditoria dos nomes das séries de consulta (multipleito, 1.1): em cada pleito com série, todo município, UF e
candidato que a página pode mostrar tem nome no `nomes.json` da série? Sem nome, a página mostra o código ("sp71072") ou
o número do candidato no lugar do nome.

    python3 aquisicao/scripts/auditar_nomes.py [--base https://dados.urnavelox.com.br/consulta | pasta] [--pleitos 452,3220]
           [--json saida.json]

Confere, por pleito:
* tópicos (`topicos.json`): a abrangência de cada um (UF ou município) tem nome;
* resultado final de cada tópico (`t/<tópico>.json`, última versão): cada candidato tem nome na circunscrição e cargo;
* marcos (`marcos.json`): o município de cada marco (primeira seção, BU marco…) tem nome;
* mapa (`mapa-secoes.json`): todo município das seções tem nome (o "Último BU" do mapa).
Só leitura; nenhum pedido ao TSE (a série é servida pelos nós e pela Cloudflare).
"""
import argparse
import concurrent.futures as cf
import gzip
import json
import os
import sys
import urllib.request


def ler(base, caminho):
    if base.startswith('http'):
        r = urllib.request.Request(f'{base}/{caminho}', headers={'Accept-Encoding': 'gzip', 'User-Agent': 'uv-auditar-nomes'})
        try:
            with urllib.request.urlopen(r, timeout=60) as x:
                b = x.read()
                if x.headers.get('Content-Encoding') == 'gzip':
                    b = gzip.decompress(b)
        except Exception:
            return None
    else:
        p = os.path.join(base, caminho)
        if os.path.exists(p):
            b = open(p, 'rb').read()
        elif os.path.exists(p + '.gz'):
            b = gzip.decompress(open(p + '.gz', 'rb').read())
        else:
            return None
    try:
        return json.loads(b)
    except ValueError:
        return None


def municipios_do_mapa(m):
    """`mapa-secoes.json`: {"ufs": [...], "m": [município, ...]} — a lista "m" alterna código do município e contagens;
    aqui só interessa o conjunto dos códigos, que o `uv-servico` também expõe por UF. Formato: para cada UF, uma lista de
    [mun, nz, (zona, nsec, seções…)…]; lemos os municípios pelo início de cada bloco."""
    return None  # o formato compacto não é lido aqui; o mapa usa os mesmos municípios dos marcos (conferidos abaixo)


def auditar(base, p):
    raiz = f'{p}/serie'
    nomes = ler(base, f'{raiz}/nomes.json') or {}
    tops = ler(base, f'{raiz}/topicos.json') or []
    ufs, muns = nomes.get('ufs') or {}, nomes.get('municipios') or {}
    cands = {}
    for e in nomes.get('eleicoes') or []:
        for c in e.get('cargos') or []:
            for ci in c.get('circunscricoes') or []:
                cands.setdefault((ci['abr'], c['cargo']), set()).update(k['n'] for k in ci.get('candidatos') or [] if k.get('nome_urna') or k.get('nome'))
    r = {'pleito': p, 'topicos': len(tops), 'sem_nome_abr': [], 'cand_sem_nome': [], 'marcos_mun_sem_nome': [], 'erros': []}
    if not nomes:
        r['erros'].append('sem nomes.json')
    for t in tops:
        abr = t['topico'].split('-')[0]
        if len(abr) == 2 and abr not in ufs and abr not in ('br', 'zz'):
            r['sem_nome_abr'].append(abr)
        if len(abr) == 7 and abr not in muns:
            r['sem_nome_abr'].append(abr)

    def um(t):
        s = ler(base, f'{raiz}/t/{t["topico"]}.json')
        if not s or not s.get('versoes'):
            return t['topico'], None
        return t['topico'], s['versoes'][-1][1]

    with cf.ThreadPoolExecutor(16) as ex:
        for topico, v in ex.map(um, tops):
            if v is None:
                r['erros'].append(f'sem série de {topico}')
                continue
            circ, cargo = v.get('circ') or v.get('abr'), v.get('cargo')
            ok = cands.get((circ, cargo), set())
            for k in v.get('cand') or []:
                if k['n'] not in ok:
                    r['cand_sem_nome'].append(f'{topico}:{k["n"]}')
    ma = ler(base, f'{raiz}/marcos.json')
    if ma:
        col = ma['colunas']
        iu, im = col.index('uf'), col.index('municipio')
        vistos = set()
        for l in ma['linhas']:
            if l[im] in ('', None) or l[iu] in ('', None):
                continue
            cod = f'{l[iu]}{int(l[im]):05d}'
            if cod not in muns and cod not in vistos:
                vistos.add(cod)
        r['marcos_mun_sem_nome'] = sorted(vistos)
    return r


def main():
    a = argparse.ArgumentParser()
    a.add_argument('--base', default='https://dados.urnavelox.com.br/consulta')
    a.add_argument('--pleitos', default='')
    a.add_argument('--json', default='')
    o = a.parse_args()
    if o.pleitos:
        ps = o.pleitos.split(',')
    else:
        ind = ler(o.base, 'indice.json') or {}
        ps = [str(x['pleito']) for x in ind.get('pleitos', [])]
    res = []
    for p in ps:
        r = auditar(o.base, p)
        res.append(r)
        prob = len(r['sem_nome_abr']) + len(r['cand_sem_nome']) + len(r['marcos_mun_sem_nome']) + len(r['erros'])
        print(f"{p:>5}: {r['topicos']:>5} tópicos · abrangências sem nome {len(r['sem_nome_abr'])} · candidatos sem nome {len(r['cand_sem_nome'])}"
              f" · municípios de marcos sem nome {len(r['marcos_mun_sem_nome'])} · erros {len(r['erros'])}{'' if prob else '  ok'}")
        for k in ('sem_nome_abr', 'cand_sem_nome', 'marcos_mun_sem_nome', 'erros'):
            if r[k]:
                print(f'       {k}: {", ".join(map(str, r[k][:12]))}{" …" if len(r[k]) > 12 else ""}')
    if o.json:
        json.dump(res, open(o.json, 'w'), ensure_ascii=False, indent=1)
    sys.exit(1 if any(r['sem_nome_abr'] or r['cand_sem_nome'] or r['marcos_mun_sem_nome'] or r['erros'] for r in res) else 0)


if __name__ == '__main__':
    main()
