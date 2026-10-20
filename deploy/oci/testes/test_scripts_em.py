"""B-007/B-015: nenhum script de deploy/oci manda comando ao nó por heredoc sem "bash -s".

No transporte OCI (Run Command), o `em` de lib.sh só repassa a entrada padrão ao nó quando o comando é "bash -s";
`em "$N" <<CMD … CMD` chega vazio ao nó e o roteiro segue como se tivesse feito (B-015: 02_configurar_borda.sh não
instalava o Caddy). Também confere a sintaxe de todos os scripts (bash -n, compile).

    python3 -m unittest discover -s deploy/oci/testes -p 'test_*.py'
"""
import glob
import os
import re
import subprocess
import unittest

AQUI = os.path.dirname(os.path.abspath(__file__))
OCI = os.path.dirname(AQUI)
SH = sorted(glob.glob(os.path.join(OCI, '*.sh')) + glob.glob(os.path.join(OCI, 'servidor', '*.sh')))
PY = sorted(glob.glob(os.path.join(OCI, '*.py')) + glob.glob(os.path.join(OCI, 'servidor', '*.py')))
# `em <nó> <<` com um só argumento antes do heredoc (sem o comando "bash -s")
EM_HEREDOC = re.compile(r'(?:^|[;&|(]|\s)em\s+("[^"]*"|\S+)\s*<<')


class ScriptsDoDeploy(unittest.TestCase):
    def test_em_com_heredoc_tem_bash_s(self):
        achados = []
        for p in SH:
            with open(p, encoding='utf-8', errors='replace') as f:
                for i, linha in enumerate(f, 1):
                    if linha.lstrip().startswith('#'):
                        continue
                    if EM_HEREDOC.search(linha):
                        achados.append(f'{os.path.relpath(p, OCI)}:{i}: {linha.strip()}')
        self.assertEqual(achados, [], 'heredoc para "em" sem "bash -s" (chega vazio ao nó):\n' + '\n'.join(achados))

    def test_sintaxe_bash(self):
        ruins = []
        for p in SH:
            r = subprocess.run(['bash', '-n', p], capture_output=True, text=True)
            if r.returncode:
                ruins.append(f'{os.path.relpath(p, OCI)}: {r.stderr.strip()}')
        self.assertEqual(ruins, [])

    def test_sintaxe_python(self):
        ruins = []
        for p in PY:
            try:
                with open(p, encoding='utf-8') as f:
                    compile(f.read(), p, 'exec')
            except SyntaxError as e:
                ruins.append(f'{os.path.relpath(p, OCI)}: {e}')
        self.assertEqual(ruins, [])


if __name__ == '__main__':
    unittest.main()
