#!/usr/bin/env python3
"""Confere o marcos.csv de uma saída do uv-servico com o diário de seções (secoes.log) e o eventos.csv:
* primeira_secao (Brasil, cada UF, cada circunscrição municipal) = primeira linha "aplicado" do secoes.log da abrangência;
* secao_decisiva: uma por evento de circunscrição (matematicamente_eleito/2t), no seq do evento, com a última seção
  aplicada da circunscrição até aquele seq (a seção do lote do evento);
* faltam_para_mat chega a 0 exatamente no seq de cada evento de circunscrição e nunca sobe (fora reenvio/anulação);
* bu_marco: a seção cuja aplicação levou o seq ao marco.
Uso: conferir_marcos.py <saida> [--json arquivo]"""
import csv, json, sys
from collections import defaultdict

def main():
    d = sys.argv[1]
    diario = [l for l in csv.DictReader(open(f'{d}/secoes.log', encoding='utf-8')) if l.get('acao')]
    eventos = [l for l in csv.DictReader(l for l in open(f'{d}/eventos.csv', encoding='utf-8') if not l.startswith('#'))]
    marcos = list(csv.DictReader(open(f'{d}/marcos.csv', encoding='utf-8')))
    erros, ok = [], defaultdict(int)
    mudam = [l for l in diario if l['acao'] in ('aplicado', 'substituido', 'anulado')]
    aplic = [l for l in diario if l['acao'] == 'aplicado']
    chave = lambda l: (l['uf'], int(l['municipio']), int(l['zona']), int(l['secao']))
    # primeira seção
    primeira = {}
    for l in aplic:
        uf, mu = l['uf'], int(l['municipio'])
        for a in ('br', uf, f'{uf}{mu:05d}'):
            primeira.setdefault(a, l)
    for m in (x for x in marcos if x['classe'] == 'primeira_secao'):
        p = primeira.get(m['abrangencia'])
        if p and chave(p) == (m['uf'], int(m['municipio']), int(m['zona']), int(m['secao'])) and int(p['seq']) == int(m['seq']):
            ok['primeira_secao'] += 1
        else:
            erros.append(f"primeira_secao {m['abrangencia']}: marco {m['uf']} {m['municipio']}/{m['zona']}/{m['secao']} seq {m['seq']} × diário {p and chave(p)} seq {p and p['seq']}")
    abrs_esperadas = {m['abrangencia'] for m in marcos if m['classe'] == 'primeira_secao'}
    for a in ('br',) + tuple(sorted({l['uf'] for l in aplic})):
        if a not in abrs_esperadas:
            erros.append(f'primeira_secao ausente: {a}')
    # eventos de circunscrição
    # depois de uma retomada o serviço reavalia tudo e o eventos.csv repete os eventos já registrados (com o seq da
    # retomada): vale a primeira ocorrência de cada (abrangência, cargo, estado)
    ev_circ, vistos = [], set()
    for e in eventos:
        if e['candidato'] or not e['estado'].startswith('matematicamente'): continue
        k = (e['abrangencia'], e['cargo'], e['estado'])
        if k in vistos: continue
        vistos.add(k)
        ev_circ.append(e)
    dec = {(m['abrangencia'], m['cargo'], m['estado']): m for m in marcos if m['classe'] == 'secao_decisiva'}
    def dentro(a, l):
        return a == 'br' or a == l['uf'] or a == f"{l['uf']}{int(l['municipio']):05d}"
    for e in ev_circ:
        a, c, s = e['abrangencia'], e['cargo'], int(e['seq'])
        m = dec.get((a, c, e['estado']))
        ult = None
        for l in mudam:
            if int(l['seq']) > s: break
            if dentro(a, l): ult = l
        if not m:
            erros.append(f'secao_decisiva ausente: {a} cargo {c} {e["estado"]} seq {s}')
        elif int(m['seq']) != s or not ult or chave(ult) != (m['uf'], int(m['municipio']), int(m['zona']), int(m['secao'])):
            erros.append(f"secao_decisiva {a} c{c}: marco seq {m['seq']} seção {m['uf']} {m['municipio']}/{m['zona']}/{m['secao']} × evento seq {s}, última do diário {ult and chave(ult)}")
        else:
            ok['secao_decisiva'] += 1
    # faltam
    fal = defaultdict(list)
    for m in marcos:
        if m['classe'] == 'faltam_para_mat':
            fal[(m['abrangencia'], m['cargo'])].append((int(m['seq']), int(m['valor']) if m['valor'] else None))
    for e in ev_circ:
        k = (e['abrangencia'], e['cargo'])
        zeros = [s for s, v in fal.get(k, []) if v == 0]
        if zeros and zeros[0] == int(e['seq']):
            ok['faltam_zero_no_evento'] += 1
        else:
            erros.append(f'faltam {k}: primeiro 0 no seq {zeros[:1]} × evento seq {e["seq"]}')
    subidas = 0
    for k, v in fal.items():
        ant = None
        for s, x in v:
            if x is None: continue
            if ant is not None and x > ant: subidas += 1; erros.append(f'faltam {k} subiu {ant} → {x} no seq {s}') if subidas <= 10 else None
            ant = x
    ok['faltam_series'] = len(fal)
    ok['faltam_subidas'] = subidas
    # bu_marco
    porseq = {int(l['seq']): l for l in aplic}
    for m in (x for x in marcos if x['classe'] == 'bu_marco'):
        l = porseq.get(int(m['seq']))
        if l and chave(l) == (m['uf'], int(m['municipio']), int(m['zona']), int(m['secao'])): ok['bu_marco'] += 1
        else: erros.append(f"bu_marco {m['seq']}: × diário {l and chave(l)}")
    r = {'ok': dict(ok), 'eventos_circunscricao': len(ev_circ), 'erros': erros[:50], 'n_erros': len(erros),
         'classes': {c: sum(1 for m in marcos if m['classe'] == c) for c in sorted({m['classe'] for m in marcos})}}
    print(json.dumps(r, ensure_ascii=False, indent=1))
    if '--json' in sys.argv: json.dump(r, open(sys.argv[sys.argv.index('--json') + 1], 'w'), ensure_ascii=False, indent=1)
    sys.exit(1 if erros else 0)

if __name__ == '__main__':
    main()
