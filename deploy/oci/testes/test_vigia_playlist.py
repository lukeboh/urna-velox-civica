"""uv-ciclo-vigia.py com a playlist: na virada, o vigia sobe o simulado mais votado; suspenso perto de uma eleição
oficial, não sobe nada; sem playlist, segue a rotação de sempre. O `systemd-run` é trocado por um script que anota.

    python3 -m unittest discover -s deploy/oci/testes -p 'test_*.py'
"""
import importlib.util
import json
import os
import shutil
import subprocess
import sys
import tempfile
import threading
import unittest
from datetime import date, datetime, timedelta, timezone

AQUI = os.path.dirname(os.path.abspath(__file__))
SERV = os.path.join(os.path.dirname(AQUI), 'servidor')
VIGIA = os.path.join(SERV, 'uv-ciclo-vigia.py')
_spec = importlib.util.spec_from_file_location('uv_playlist', os.path.join(SERV, 'uv-playlist.py'))
up = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(up)
BR = timezone(timedelta(hours=-3))


class TestVigiaPlaylist(unittest.TestCase):
    def setUp(self):
        self.c = tempfile.mkdtemp(prefix='uv-vigia-')
        os.makedirs(os.path.join(self.c, 'atual', 'saida'))
        agora = datetime.now(BR)
        ini = (agora - timedelta(minutes=30)).isoformat()
        self.id = '20261019-1400-452-rapido'
        open(os.path.join(self.c, 'atual', 'id'), 'w').write(self.id + '\n')
        open(os.path.join(self.c, 'atual', 'params'), 'w').write(f'pleito=452\nmodo=rapido\ninicio={ini}\n')
        json.dump({'apuracao': {'encerrada': True}}, open(os.path.join(self.c, 'atual', 'saida', 'saude.json'), 'w'))
        # vigia reiniciado depois da conclusão, com o disparo já vencido: dispara na primeira volta
        json.dump({'historico_modos': {'452': 'rapido'},
                   'atual': {'id': self.id, 'pleito': '452', 'modo': 'rapido', 'inicio': ini, 'concluido': True,
                             'concluido_em': (agora - timedelta(minutes=20)).isoformat()},
                   'proximo': {'pleito': '453', 'modo': 'rapido', 'disparo': (agora - timedelta(seconds=5)).isoformat()}},
                  open(os.path.join(self.c, 'rotacao.json'), 'w'))
        self.anot = os.path.join(self.c, 'systemd-run.txt')
        self.falso = os.path.join(self.c, 'systemd-run')
        open(self.falso, 'w').write(f'#!/bin/sh\necho "$@" >> {self.anot}\n')
        os.chmod(self.falso, 0o755)
        self.srv = None

    def tearDown(self):
        if self.srv:
            self.srv.shutdown()
        shutil.rmtree(self.c, ignore_errors=True)

    def playlist(self, eleicoes=(date(2030, 1, 1),)):
        pl = up.Playlist(self.c, caminho=os.path.join(self.c, 'pl.json'), eleicoes=list(eleicoes))
        self.srv = up.ThreadingHTTPServer(('127.0.0.1', 0), up.criar_tratador(pl, '/dev/null', 'seg'))
        threading.Thread(target=self.srv.serve_forever, daemon=True).start()
        return pl, f'http://127.0.0.1:{self.srv.server_address[1]}/api/v1/playlist'

    def rodar(self, env, timeout=20):
        e = dict(os.environ, UV_SYSTEMD_RUN=self.falso, UV_CICLO_ENV='/nao/existe', UV_PLAYLIST_TENTATIVAS='2',
                 UV_PLAYLIST_ESPERA='0.2', UV_ELEICOES_OFICIAIS='2030-01-01')
        e.update(env)
        try:
            subprocess.run([sys.executable, VIGIA, self.c, 'no-a', '0.2'], env=e, timeout=timeout,
                           stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        except subprocess.TimeoutExpired:
            pass
        return open(self.anot).read() if os.path.exists(self.anot) else ''

    def test_sobe_o_mais_votado(self):
        pl, url = self.playlist()
        for i in range(2):
            pl.votar(up.chave_ip(f'10.0.0.{i}', pl.e['sal']), '3220-cadencia')
        pl.votar(up.chave_ip('10.0.0.9', pl.e['sal']), '453-cadencia')
        saida = self.rodar({'UV_PLAYLIST_URL': url, 'UV_PLAYLIST_SEGREDO': 'seg'})
        self.assertIn('avancar 3220 cadencia', saida)
        rot = json.load(open(os.path.join(self.c, 'rotacao.json')))
        self.assertEqual((rot['proximo']['pleito'], rot['proximo']['modo']), ('3220', 'cadencia'))
        self.assertTrue(rot['proximo']['origem'].startswith('votos'))

    def test_sem_playlist_segue_a_rotacao(self):
        saida = self.rodar({'UV_PLAYLIST_URL': 'http://127.0.0.1:9/api/v1/playlist', 'UV_PLAYLIST_SEGREDO': 'seg'})
        self.assertIn('avancar 453 rapido', saida)

    def test_suspenso_nao_sobe_nada(self):
        hoje = datetime.now(BR).date().isoformat()
        pl, url = self.playlist()
        saida = self.rodar({'UV_PLAYLIST_URL': url, 'UV_PLAYLIST_SEGREDO': 'seg', 'UV_ELEICOES_OFICIAIS': hoje}, timeout=4)
        self.assertEqual(saida, '')
        self.assertIn('playlist suspensa', open(os.path.join(self.c, 'ciclo.log')).read())


if __name__ == '__main__':
    unittest.main()
