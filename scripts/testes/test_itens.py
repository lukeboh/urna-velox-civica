#!/usr/bin/env python3
"""Testes do scripts/itens.py (bugs e melhorias × roadmap).   python3 scripts/testes/test_itens.py"""
import importlib.util
import sys
import tempfile
import unittest
from pathlib import Path

AQUI = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('itens', AQUI.parent / 'itens.py')
itens = importlib.util.module_from_spec(spec)
spec.loader.exec_module(itens)

CAB = '| id | registrado em | quem viu | onde | roadmap | sintoma | estado | ordem | teste | versão | detalhe |\n|---|---|---|---|---|---|---|---|---|---|---|\n'
ROAD = """# Roadmap

## 🔧 Manutenção da 1.0 (`1.0.x`)

<!-- itens:1.0.x -->
<!-- /itens:1.0.x -->

## 🛡️ `1.2.0` — Coleta robusta

<!-- itens:1.2 -->
<!-- /itens:1.2 -->

## ✅ `1.1.0` — Multipleito ✅

<!-- itens:1.1 -->
<!-- /itens:1.1 -->

## 📚 Histórico

<!-- itens:historico -->
<!-- /itens:historico -->
"""


def linha(i, roadmap, estado, versao='', ordem='', detalhe=''):
    return f'| {i} | 27/10/2026 | Claude | serviço | `{roadmap}` | Algo deu errado no serviço. | {estado} | {ordem} | t | {versao} | {detalhe} |\n'


class Itens(unittest.TestCase):
    def montar(self, linhas, roadmap=ROAD):
        d = Path(tempfile.mkdtemp(prefix='uv-itens-'))
        (d / 'docs' / 'itens').mkdir(parents=True)
        (d / 'docs' / 'BUGS.md').write_text('# Bugs\n\n' + CAB + ''.join(linhas), encoding='utf-8')
        (d / 'ROADMAP.md').write_text(roadmap, encoding='utf-8')
        return d

    def gerar_e_verificar(self, d):
        self.assertEqual(itens.main(['itens.py', 'gerar', '--raiz', str(d)]), 0)
        return itens.verificar(d)[0]

    def test_coerente_e_idempotente(self):
        d = self.montar([linha('B-001', '1.2', 'registrado'), linha('B-002', '0.8.x', 'corrigido', '0.8.9'),
                         linha('M-001', '1.0.x', 'feito', '1.0.15')])
        self.assertEqual(self.gerar_e_verificar(d), [])
        txt = (d / 'ROADMAP.md').read_text(encoding='utf-8')
        self.assertIn('⬜ **B-001**', txt.split('<!-- itens:1.2 -->')[1].split('<!-- /itens:1.2 -->')[0])
        self.assertIn('| `0.8.x` | 1 | 0 | B-002 |', txt)
        self.assertNotIn('`1.0.x` |', txt.split('<!-- itens:historico -->')[1], 'o histórico não repete a série que é etapa')
        itens.main(['itens.py', 'gerar', '--raiz', str(d)])
        self.assertEqual((d / 'ROADMAP.md').read_text(encoding='utf-8'), txt, 'gerar duas vezes não muda nada')

    def test_recusas(self):
        casos = {
            'sem destino': linha('B-001', '', 'registrado'),
            "que não é uma etapa": linha('B-001', '3.0', 'registrado'),
            'fechado sem versão': linha('B-001', '1.0.x', 'corrigido'),
            'deve ser fora': linha('B-001', '1.0.x', 'descartado (duplicado)'),
            'sem estar descartado': linha('B-001', 'fora', 'registrado'),
            'que não existe': linha('B-001', '1.2', 'registrado', detalhe='[B-001](itens/B-001.md)'),
            'marcada ✅ com itens abertos': linha('B-001', '1.1', 'registrado'),
        }
        for esperado, l in casos.items():
            with self.subTest(esperado):
                erros = self.gerar_e_verificar(self.montar([l]))
                self.assertTrue(any(esperado in e for e in erros), f'{esperado}: {erros}')

    def test_lista_desatualizada(self):
        d = self.montar([linha('B-001', '1.2', 'registrado')])
        self.assertEqual(self.gerar_e_verificar(d), [])
        b = d / 'docs' / 'BUGS.md'
        b.write_text(b.read_text(encoding='utf-8') + linha('B-002', '1.2', 'registrado'), encoding='utf-8')
        self.assertTrue(any('desatualizadas' in e for e in itens.verificar(d)[0]))

    def test_repositorio(self):
        """O próprio repositório está coerente."""
        erros, its = itens.verificar(AQUI.parent.parent)
        self.assertEqual(erros, [])
        self.assertGreater(len(its), 80)


if __name__ == '__main__':
    unittest.main(verbosity=1)
