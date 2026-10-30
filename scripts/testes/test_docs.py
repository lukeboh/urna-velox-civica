#!/usr/bin/env python3
"""Testes do scripts/docs.py (organização de docs/).   python3 scripts/testes/test_docs.py"""
import importlib.util
import subprocess
import tempfile
import unittest
from pathlib import Path

AQUI = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('docs', AQUI.parent / 'docs.py')
docs = importlib.util.module_from_spec(spec)
spec.loader.exec_module(docs)


class Docs(unittest.TestCase):
    def repo(self, arquivos):
        d = Path(tempfile.mkdtemp(prefix='uv-docs-'))
        subprocess.run(['git', 'init', '-q'], cwd=d, check=True)
        base = {'docs/README.md': '# Índice\n\n* [OPERACAO.md](operacao/OPERACAO.md)\n', 'docs/operacao/OPERACAO.md': '# Operação\n',
                'docs/BUGS.md': '# Bugs\n', 'README.md': '# Projeto\n\n[operação](docs/operacao/OPERACAO.md)\n'}
        base.update(arquivos)
        for k, v in base.items():
            if v is None:
                continue
            (d / k).parent.mkdir(parents=True, exist_ok=True)
            (d / k).write_text(v, encoding='utf-8')
        return d

    def test_em_ordem(self):
        self.assertEqual(docs.verificar(self.repo({})), [])

    def test_recusas(self):
        casos = {
            'solto na raiz': {'docs/NOVO.md': '# x\n'},
            'fora da lista': {'docs/diversos/X.md': '# x\n'},
            'começa com HANDOFF_': {'docs/handoffs/NOTAS.md': '# x\n'},
            'nome fora do padrão': {'docs/operacao/notas.md': '# x\n'},
            'não está no índice': {'docs/operacao/PAINEL.md': '# x\n'},
            'link quebrado': {'docs/operacao/OPERACAO.md': '# Operação\n\n[x](../arquitetura/NAO_EXISTE.md)\n'},
            'imagem fora': {'docs/operacao/foto.png': 'x'},
            'só <id>.md': {'docs/itens/notas.md': '# x\n'},
        }
        for esperado, extra in casos.items():
            with self.subTest(esperado):
                erros = docs.verificar(self.repo(extra))
                self.assertTrue(any(esperado in e for e in erros), f'{esperado}: {erros}')

    def test_links_ignorados(self):
        d = self.repo({'docs/operacao/OPERACAO.md': '# Op\n\n[a](https://x.y) [b](#secao) `[c](nada.md)`\n\n```\n[d](nada.md)\n```\n'})
        self.assertEqual(docs.verificar(d), [])

    def test_repositorio(self):
        self.assertEqual(docs.verificar(AQUI.parent.parent), [])


if __name__ == '__main__':
    unittest.main(verbosity=1)
