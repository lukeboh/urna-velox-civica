#!/usr/bin/env python3
"""Bugs e melhorias × roadmap (regra do CLAUDE.md, seção "Bugs", de 27/10/2026).

    python3 scripts/itens.py gerar       # reescreve no ROADMAP.md as listas de itens de cada etapa
    python3 scripts/itens.py verificar   # confere a coerência (o pre-commit roda este)
    python3 scripts/itens.py listar [destino]

Fonte única: as tabelas de `docs/BUGS.md` (bugs `B-nnn` e melhorias `M-nnn`). Cada item tem um destino na coluna
`roadmap`:

* uma etapa do roadmap (`1.0.x` = manutenção da 1.0, `1.1`, `1.2`, `1.x`, `2.0`...): a chave de um bloco
  `<!-- itens:CHAVE -->` … `<!-- /itens:CHAVE -->` do ROADMAP.md, onde a lista daquela etapa é gerada;
* a série em que um item já fechado entrou (`0.8.x`, `0.9.x`...), para o histórico;
* `fora`: descartado (com o motivo no estado).

O `verificar` recusa: item sem destino; destino de item aberto que não é bloco do ROADMAP; item fechado (corrigido,
feito, resolvido) sem versão; item descartado com destino diferente de `fora`; link de detalhe para arquivo que não
existe; etapa marcada ✅ no ROADMAP com item aberto; e bloco do ROADMAP diferente do que o `gerar` escreveria.
"""
import re
import sys
from pathlib import Path

RAIZ = Path(__file__).resolve().parent.parent
RX_ITEM = re.compile(r'^\| ([BM]-\d{3,}) \|')
RX_BLOCO = re.compile(r'<!-- itens:([^ >]+) -->\n.*?<!-- /itens:\1 -->', re.S)
RX_SERIE = re.compile(r'^\d+\.\d+\.x$')
FECHADO = re.compile(r'^(corrigido|feito|resolvido)', re.I)
DESCARTADO = re.compile(r'^descartado', re.I)


def celulas(linha):
    return [c.strip() for c in linha.strip().strip('|').split('|')]


def ler_itens(bugs_md):
    """Itens das tabelas do BUGS.md: dicionários com id, tipo, onde, resumo, roadmap, estado, ordem, versao, detalhe."""
    itens, cab = [], None
    for n, ln in enumerate(bugs_md.read_text(encoding='utf-8').splitlines(), 1):
        if ln.startswith('| id |'):
            cab = [c.lower() for c in celulas(ln)]
            continue
        m = RX_ITEM.match(ln)
        if not m or not cab:
            continue
        c = celulas(ln)
        if len(c) != len(cab):
            itens.append({'id': m.group(1), 'linha': n, 'erro': f'{len(c)} colunas, o cabeçalho tem {len(cab)}'})
            continue
        d = dict(zip(cab, c))
        itens.append({'id': m.group(1), 'linha': n, 'tipo': 'bug' if m.group(1)[0] == 'B' else 'melhoria',
                      'onde': d.get('onde', ''), 'resumo': d.get('sintoma', d.get('pedido', '')),
                      'roadmap': d.get('roadmap', '').strip('` '), 'estado': d.get('estado', ''), 'ordem': d.get('ordem', ''),
                      'versao': d.get('versão', d.get('versao', '')), 'detalhe': d.get('detalhe', '')})
    return itens


def situacao(it):
    e = it.get('estado', '').strip('* ')
    if FECHADO.match(e):
        return 'fechado'
    if DESCARTADO.match(e):
        return 'descartado'
    return 'aberto'


def resumo_curto(t, n=150):
    t = re.sub(r'\*\*|`', '', t)
    t = re.sub(r'\[([^\]]*)\]\([^)]*\)', r'\1', t)
    t = t.split(' [detalhe]')[0]
    m = re.match(r'(.+?[.!?])(\s|$)', t)
    s = m.group(1) if m and len(m.group(1)) >= 25 else t
    return s if len(s) <= n else s[:n - 1].rstrip() + '…'


def versao_de(it):
    m = re.search(r'\d+\.\d+\.\d+', it.get('versao', ''))
    return m.group(0) if m else ''


def lista_do_bloco(chave, itens, blocos=()):
    """Texto Markdown gerado para o bloco `chave` do ROADMAP (`blocos`: as chaves dos blocos, que o histórico não repete)."""
    if chave == 'historico':
        series = {}
        for it in itens:
            if RX_SERIE.match(it.get('roadmap', '')) and it['roadmap'] not in blocos:
                series.setdefault(it['roadmap'], []).append(it)
        if not series:
            return '_(nenhum)_'
        def chave_serie(s):
            return tuple(int(x) for x in s.split('.')[:2])
        out = ['| série | bugs | melhorias | itens |', '|---|---|---|---|']
        for s in sorted(series, key=chave_serie):
            v = series[s]
            ids = ', '.join(x['id'] for x in sorted(v, key=lambda x: x['id']))
            out.append(f"| `{s}` | {sum(x['tipo'] == 'bug' for x in v)} | {sum(x['tipo'] == 'melhoria' for x in v)} | {ids} |")
        return '\n'.join(out)
    sel = [it for it in itens if it.get('roadmap') == chave]
    if not sel:
        return '_(nenhum item de [docs/BUGS.md](docs/BUGS.md) com este destino)_'
    def ordem(it):
        o = re.match(r'\d+', it.get('ordem', '') or '')
        return (situacao(it) != 'aberto', int(o.group(0)) if o else 9999, it['id'])
    out = []
    for it in sorted(sel, key=ordem):
        st = situacao(it)
        marca = {'aberto': '⬜', 'fechado': '✅', 'descartado': '✖️'}[st]
        det = re.search(r'\]\((itens/[^)]+)\)', it.get('detalhe', ''))
        ref = f"[{it['id']}](docs/{det.group(1)})" if det else it['id']
        estado = re.sub(r'\s+', ' ', it['estado']).strip()
        estado = estado if len(estado) <= 60 else estado[:59].rstrip() + '…'
        extra = f" · {versao_de(it)}" if st == 'fechado' and versao_de(it) else (f" · ordem {it['ordem']}" if it.get('ordem') else '')
        out.append(f"- {marca} **{ref}** ({it['tipo']}) — {resumo_curto(it['resumo'])} · *{estado}*{extra}")
    return '\n'.join(out)


def gerado(roadmap_txt, itens):
    blocos = set(RX_BLOCO.findall(roadmap_txt))
    return RX_BLOCO.sub(lambda m: f"<!-- itens:{m.group(1)} -->\n{lista_do_bloco(m.group(1), itens, blocos)}\n<!-- /itens:{m.group(1)} -->", roadmap_txt)


def etapas_concluidas(roadmap_txt):
    """Chaves dos blocos que ficam numa seção (## ...) com ✅ no título."""
    feitas, titulo = set(), ''
    for ln in roadmap_txt.splitlines():
        if ln.startswith('## '):
            titulo = ln
        for m in re.finditer(r'<!-- itens:([^ >]+) -->', ln):
            if '✅' in titulo:
                feitas.add(m.group(1))
    return feitas


def verificar(raiz=RAIZ):
    bugs, roadmap = raiz / 'docs' / 'BUGS.md', raiz / 'ROADMAP.md'
    itens = ler_itens(bugs)
    rtxt = roadmap.read_text(encoding='utf-8')
    blocos = set(RX_BLOCO.findall(rtxt)) - {'historico'}
    erros = []
    vistos = {}
    for it in itens:
        p = f"docs/BUGS.md:{it['linha']} {it['id']}"
        if 'erro' in it:
            erros.append(f"{p}: {it['erro']}"); continue
        if it['id'] in vistos:
            erros.append(f"{p}: id repetido (linha {vistos[it['id']]})")
        vistos[it['id']] = it['linha']
        r, st = it['roadmap'], situacao(it)
        if not r:
            erros.append(f"{p}: sem destino na coluna roadmap"); continue
        if st == 'aberto' and r not in blocos:
            erros.append(f"{p}: aberto com destino '{r}', que não é uma etapa do ROADMAP ({', '.join(sorted(blocos))})")
        if st == 'fechado':
            if not versao_de(it):
                erros.append(f"{p}: fechado sem versão")
            if r not in blocos and not RX_SERIE.match(r):
                erros.append(f"{p}: fechado com destino '{r}' (use uma etapa do ROADMAP ou a série, ex.: 0.8.x)")
        if st == 'descartado' and r != 'fora':
            erros.append(f"{p}: descartado com destino '{r}' (deve ser fora)")
        if st != 'descartado' and r == 'fora':
            erros.append(f"{p}: destino fora sem estar descartado")
        for alvo in re.findall(r'\]\((itens/[^)#]+)', it['detalhe']):
            if not (raiz / 'docs' / alvo).exists():
                erros.append(f"{p}: detalhe aponta para docs/{alvo}, que não existe")
    for chave in etapas_concluidas(rtxt):
        abertos = [it['id'] for it in itens if it.get('roadmap') == chave and situacao(it) == 'aberto']
        if abertos:
            erros.append(f"ROADMAP.md: etapa '{chave}' marcada ✅ com itens abertos: {', '.join(abertos)}")
    if gerado(rtxt, itens) != rtxt:
        erros.append("ROADMAP.md: as listas de itens estão desatualizadas (rode: python3 scripts/itens.py gerar)")
    return erros, itens


def main(argv):
    acao = argv[1] if len(argv) > 1 else 'verificar'
    raiz = Path(argv[argv.index('--raiz') + 1]) if '--raiz' in argv else RAIZ
    if acao == 'gerar':
        roadmap = raiz / 'ROADMAP.md'
        itens = ler_itens(raiz / 'docs' / 'BUGS.md')
        txt = roadmap.read_text(encoding='utf-8')
        novo = gerado(txt, itens)
        if novo != txt:
            roadmap.write_text(novo, encoding='utf-8', newline='\n')
        print(f"ROADMAP.md: {len(RX_BLOCO.findall(novo))} blocos, {len(itens)} itens ({'atualizado' if novo != txt else 'já estava em dia'})")
        return 0
    if acao == 'listar':
        filtro = argv[2] if len(argv) > 2 and not argv[2].startswith('--') else None
        for it in ler_itens(raiz / 'docs' / 'BUGS.md'):
            if not filtro or it.get('roadmap') == filtro:
                print(f"{it['id']}\t{it.get('roadmap', '')}\t{situacao(it)}\t{resumo_curto(it.get('resumo', ''), 90)}")
        return 0
    erros, itens = verificar(raiz)
    for e in erros:
        print(f"itens: {e}", file=sys.stderr)
    if erros:
        print(f"itens: {len(erros)} problema(s) entre docs/BUGS.md e ROADMAP.md", file=sys.stderr)
        return 1
    abertos = sum(situacao(i) == 'aberto' for i in itens)
    print(f"itens: {len(itens)} itens ({abertos} abertos) coerentes com o ROADMAP.md")
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv))
