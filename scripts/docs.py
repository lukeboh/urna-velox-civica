#!/usr/bin/env python3
"""Organização da pasta docs/ (regras em docs/README.md, seção "Regras"; CLAUDE.md, "Onde fica cada coisa").

    python3 scripts/docs.py verificar        # o pre-commit roda este
    python3 scripts/docs.py links [--todos]  # só os links relativos quebrados (padrão: dos .md de docs/ e da raiz)

O `verificar` recusa:
* arquivo solto na raiz de docs/ (lá só ficam README.md e BUGS.md) e pasta fora da lista;
* nome fora do padrão da pasta (handoffs/: HANDOFF_ ou PROMPT_; relatorios/: RELATORIO_, ANALISE_ ou VALIDACAO_;
  propostas/: PROPOSTA_; os .md em MAIÚSCULAS_COM_SUBLINHADO; imagens em img/, itens/img/ ou marca/);
* documento de docs/ que não está no índice (docs/README.md);
* link relativo quebrado num .md de docs/ ou da raiz do repositório.
"""
import os
import re
import subprocess
import sys
from pathlib import Path

RAIZ = Path(__file__).resolve().parent.parent
RAIZ_DOCS = {'README.md', 'BUGS.md'}
PASTAS = {
    'arquitetura': None, 'operacao': None, 'processo': None, 'legal': None,
    'handoffs': ('HANDOFF_', 'PROMPT_'), 'relatorios': ('RELATORIO_', 'ANALISE_', 'VALIDACAO_'), 'propostas': ('PROPOSTA_',),
    'itens': None, 'img': None, 'marca': None,
}
RX_NOME_MD = re.compile(r'^[A-Z0-9][A-Z0-9_.\-]*\.md$')
RX_ITEM = re.compile(r'^[BM]-\d{3,}\.md$')
IMAGEM = ('.png', '.jpg', '.jpeg', '.svg', '.gif', '.webp')
RX_LINK = re.compile(r'\]\(([^)\s]+)\)')


def arquivos_git(raiz):
    r = subprocess.run(['git', 'ls-files', '--cached', '--others', '--exclude-standard'], cwd=raiz, capture_output=True, text=True)
    return [a for a in r.stdout.split('\n') if a and (raiz / a).exists()]


def links_quebrados(raiz, mds):
    erros = []
    for a in mds:
        txt = (raiz / a).read_text(encoding='utf-8', errors='replace')
        # tira os blocos de código (exemplos com colchetes e parênteses)
        txt = re.sub(r'```.*?```', '', txt, flags=re.S)
        txt = re.sub(r'`[^`\n]*`', '', txt)
        for alvo in RX_LINK.findall(txt):
            if re.match(r'(https?:|mailto:|file:|#|/|data:)', alvo) or '{' in alvo:
                continue
            cam = alvo.split('#')[0]
            if not cam:
                continue
            p = os.path.normpath(os.path.join(os.path.dirname(a), cam))
            if not (raiz / p).exists():
                erros.append(f'{a}: link quebrado para {alvo}')
    return erros


def verificar(raiz=RAIZ):
    erros = []
    todos = arquivos_git(raiz)
    docs = [a for a in todos if a.startswith('docs/')]
    for a in docs:
        partes = a.split('/')[1:]
        nome = partes[-1]
        if len(partes) == 1:
            if nome not in RAIZ_DOCS:
                erros.append(f'{a}: solto na raiz de docs/ (escolha uma pasta: {", ".join(sorted(p for p in PASTAS if p not in ("img", "itens", "marca")))})')
            continue
        pasta = partes[0]
        if pasta not in PASTAS:
            erros.append(f'{a}: pasta docs/{pasta}/ fora da lista (docs/README.md, "Regras")')
            continue
        if pasta in ('img', 'marca') or (pasta == 'itens' and len(partes) > 2):
            continue
        if pasta == 'itens':
            if not RX_ITEM.match(nome):
                erros.append(f'{a}: em docs/itens/ só <id>.md (ex.: B-067.md); imagens em docs/itens/img/')
            continue
        if nome.lower().endswith(IMAGEM):
            erros.append(f'{a}: imagem fora de docs/img/ (ou docs/itens/img/, docs/marca/)')
            continue
        if nome.endswith('.md') and not RX_NOME_MD.match(nome):
            erros.append(f'{a}: nome fora do padrão (MAIÚSCULAS_COM_SUBLINHADO.md)')
        pre = PASTAS[pasta]
        if pre and not nome.startswith(pre):
            erros.append(f'{a}: em docs/{pasta}/ o nome começa com {" ou ".join(pre)}')
    indice = (raiz / 'docs' / 'README.md')
    if not indice.exists():
        erros.append('docs/README.md (índice) não existe')
    else:
        itxt = indice.read_text(encoding='utf-8')
        listados = {os.path.normpath(os.path.join('docs', x.split('#')[0])) for x in RX_LINK.findall(itxt)}
        for a in docs:
            p = a.split('/')
            if a.endswith('.md') and len(p) > 2 and p[1] in ('arquitetura', 'operacao', 'processo', 'legal', 'handoffs', 'relatorios', 'propostas', 'marca') \
                    and os.path.normpath(a) not in listados:
                erros.append(f'{a}: não está no índice docs/README.md')
    mds = [a for a in todos if a.endswith('.md') and (a.startswith('docs/') or '/' not in a)]
    erros += links_quebrados(raiz, mds)
    return erros


def main(argv):
    acao = argv[1] if len(argv) > 1 else 'verificar'
    if acao == 'links':
        todos = arquivos_git(RAIZ)
        mds = [a for a in todos if a.endswith('.md') and ('--todos' in argv or a.startswith('docs/') or '/' not in a)]
        mds = [a for a in mds if 'node_modules/' not in a]
        e = links_quebrados(RAIZ, mds)
        print('\n'.join(e) or 'nenhum link quebrado')
        return 1 if e else 0
    erros = verificar()
    for e in erros:
        print(f'docs: {e}', file=sys.stderr)
    if erros:
        print(f'docs: {len(erros)} problema(s) na organização de docs/ (regras em docs/README.md)', file=sys.stderr)
        return 1
    print('docs: organização de docs/ em ordem')
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv))
