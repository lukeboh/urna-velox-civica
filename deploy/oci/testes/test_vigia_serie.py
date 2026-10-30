"""Multipleito (1.1): na noite ao vivo (modo aovivo), o vigia refaz a série de consulta do pleito em curso a cada
SERIE_INTERVALO (uv-serie --em-curso) e o índice geral (gerar_consultas.sh --indice): o slider da página vai até o
último minuto consolidado. Fora do aovivo, nada (as séries dos encerrados são do passo 3 do uv-pleitos.sh)."""
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from datetime import datetime, timedelta, timezone

AQUI = os.path.dirname(os.path.abspath(__file__))
VIGIA = os.path.join(os.path.dirname(AQUI), 'servidor', 'uv-ciclo-vigia.py')
BR = timezone(timedelta(hours=-3))


class TestVigiaSerie(unittest.TestCase):
    def preparar(self, modo):
        self.c = tempfile.mkdtemp(prefix='uv-vigia-serie-')
        self.addCleanup(shutil.rmtree, self.c, True)
        n = os.path.join(self.c, 'atual')
        os.makedirs(os.path.join(n, 'saida', 'serie'))
        os.makedirs(os.path.join(n, 'preparo'))
        open(os.path.join(n, 'preparo', 'candidatos.json'), 'w').write('{}')
        open(os.path.join(n, 'id'), 'w').write(f'20261206-0800-3250-{modo}\n')
        open(os.path.join(n, 'params'), 'w').write(f'pleito=3250\nmodo={modo}\ninicio={datetime.now(BR).isoformat()}\n')
        # binários falsos: anotam a chamada
        self.bin = os.path.join(self.c, 'bin')
        os.makedirs(self.bin)
        self.anot = os.path.join(self.c, 'chamadas.txt')
        for nome in ('uv-serie', 'gerar_consultas.sh'):
            f = os.path.join(self.bin, nome)
            open(f, 'w').write(f'#!/bin/sh\necho "{nome} $@" >> {self.anot}\n')
            os.chmod(f, 0o755)
        self.cons = os.path.join(self.c, 'consulta')

    def rodar(self, timeout=4):
        e = dict(os.environ, UV_BIN=self.bin, UV_CONSULTA_DIR=self.cons, UV_SERIE_INTERVALO='0.3', UV_CICLO_ENV='/nao/existe',
                 UV_SYSTEMD_RUN='/bin/true', UV_PLAYLIST_URL='', UV_ELEICOES_OFICIAIS='2030-01-01')
        try:
            subprocess.run([sys.executable, VIGIA, self.c, 'no-a', '0.1'], env=e, timeout=timeout, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        except subprocess.TimeoutExpired:
            pass
        return open(self.anot).read() if os.path.exists(self.anot) else ''

    def test_aovivo_refaz_a_serie_em_curso(self):
        self.preparar('aovivo')
        s = self.rodar()
        self.assertIn('uv-serie --em-curso', s, 'o vigia não refez a série da noite em curso')
        self.assertIn(f'--destino {self.cons}/3250/serie', s)
        self.assertIn(f'--candidatos {self.c}/atual/preparo/candidatos.json', s)
        self.assertIn(f'gerar_consultas.sh --destino {self.cons} --indice', s)
        self.assertGreaterEqual(s.count('uv-serie --em-curso'), 2, 'a série é refeita a cada intervalo')

    def test_replay_nao_mexe_na_serie(self):
        self.preparar('rapido')
        self.assertNotIn('uv-serie', self.rodar(timeout=2))


if __name__ == '__main__':
    unittest.main()
