"""Playlist de simulados (servidor/uv-playlist.py): votos por IP, decisão única por execução, agenda e suspensão.

    python3 -m unittest discover -s deploy/oci/testes -p 'test_*.py'
"""
import importlib.util
import json
import os
import shutil
import tempfile
import threading
import unittest
import urllib.error
import urllib.request
from datetime import date, datetime, timedelta, timezone

AQUI = os.path.dirname(os.path.abspath(__file__))
SERV = os.path.join(os.path.dirname(AQUI), 'servidor')
_spec = importlib.util.spec_from_file_location('uv_playlist', os.path.join(SERV, 'uv-playlist.py'))
up = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(up)
BR = timezone(timedelta(hours=-3))


class Relogio:
    def __init__(self, t):
        self.t = t

    def __call__(self):
        return self.t


def ciclo(pleito='452', modo='rapido', inicio='2026-10-19T14:01:00-03:00', encerrada=False, rotacao=None):
    c = tempfile.mkdtemp(prefix='uv-playlist-')
    os.makedirs(os.path.join(c, 'atual', 'saida'))
    with open(os.path.join(c, 'atual', 'id'), 'w') as f:
        f.write(f'20261019-1400-{pleito}-{modo}\n')
    with open(os.path.join(c, 'atual', 'params'), 'w') as f:
        f.write(f'pleito={pleito}\nmodo={modo}\ninicio={inicio}\n')
    json.dump({'apuracao': {'encerrada': encerrada}}, open(os.path.join(c, 'atual', 'saida', 'saude.json'), 'w'))
    json.dump(rotacao or {'historico_modos': {}}, open(os.path.join(c, 'rotacao.json'), 'w'))
    return c


class TestRegras(unittest.TestCase):
    def setUp(self):
        self.c = ciclo()
        self.rel = Relogio(datetime(2026, 10, 19, 14, 5, tzinfo=BR))
        self.pl = up.Playlist(self.c, eleicoes=[date(2026, 10, 25)], relogio=self.rel)

    def tearDown(self):
        shutil.rmtree(self.c, ignore_errors=True)

    def test_um_voto_por_ip_e_troca(self):
        k = up.chave_ip('200.1.2.3', self.pl.e['sal'])
        self.assertEqual(self.pl.votar(k, '453-cadencia')[0], 200)
        self.assertEqual(self.pl.votar(k, '3220-rapido'), (200, 'Voto trocado.'))
        self.assertEqual(self.pl.contagem(), {'3220-rapido': (1, self.pl.e['votos'][k]['em'])})
        self.assertEqual(self.pl.votar(k, None)[0], 200)
        self.assertEqual(self.pl.contagem(), {})

    def test_ipv6_conta_por_64_e_ip_nao_fica_em_claro(self):
        s = self.pl.e['sal']
        self.assertEqual(up.chave_ip('2804:14c::1', s), up.chave_ip('2804:14c::abcd:1', s))
        self.assertNotEqual(up.chave_ip('2804:14c:0:1::1', s), up.chave_ip('2804:14c:0:2::1', s))
        self.assertEqual(up.chave_ip('::ffff:200.1.2.3', s), up.chave_ip('200.1.2.3', s))
        self.pl.votar(up.chave_ip('200.1.2.3', s), '453-rapido')
        self.assertNotIn('200.1.2.3', open(self.pl.caminho).read())

    def test_no_ar_e_desconhecido_recusados(self):
        k = up.chave_ip('1.1.1.1', self.pl.e['sal'])
        self.assertEqual(self.pl.votar(k, '452-rapido')[0], 409)   # está no ar
        self.assertEqual(self.pl.votar(k, '3221-espera')[0], 400)
        self.assertEqual(self.pl.votar(k, 'x')[0], 400)

    def test_mais_votado_e_o_proximo_e_votos_sao_consumidos(self):
        s = self.pl.e['sal']
        for i in range(3):
            self.pl.votar(up.chave_ip(f'10.0.0.{i}', s), '3220-cadencia')
        for i in range(2):
            self.pl.votar(up.chave_ip(f'10.0.1.{i}', s), '453-rapido')
        f = self.pl.fila()
        self.assertEqual([x['item'] for x in f[:2]], ['3220-cadencia', '453-rapido'])
        self.assertEqual((f[0]['origem'], f[0]['votos']), ('votos', 3))
        d = self.pl.decidir('20261019-1400-452-rapido', '452', 'rapido')
        self.assertEqual((d['pleito'], d['modo'], d['origem'], d['votos']), ('3220', 'cadencia', 'votos', 3))
        self.assertEqual(self.pl.contagem(), {'453-rapido': (2, self.pl.contagem()['453-rapido'][1])})

    def test_decisao_unica_por_execucao(self):
        s = self.pl.e['sal']
        self.pl.votar(up.chave_ip('10.0.0.1', s), '453-rapido')
        a = self.pl.decidir('X', '452', 'rapido', no='no-a')
        # um voto chega entre o pedido do nó A e o do nó B: o B recebe a mesma decisão
        self.pl.votar(up.chave_ip('10.0.0.2', s), '3220-cadencia')
        self.pl.votar(up.chave_ip('10.0.0.3', s), '3220-cadencia')
        b = self.pl.decidir('X', '452', 'rapido', no='no-b')
        self.assertEqual((a['pleito'], a['modo']), (b['pleito'], b['modo']))
        self.assertTrue(b['repetida'])

    def test_empate_vence_quem_chegou_antes(self):
        s = self.pl.e['sal']
        self.pl.votar(up.chave_ip('10.0.0.1', s), '453-cadencia')
        self.rel.t += timedelta(minutes=1)
        self.pl.votar(up.chave_ip('10.0.0.2', s), '3220-rapido')
        self.assertEqual(self.pl.ranking()[0], '453-cadencia')

    def test_sem_votos_segue_a_rotacao_de_sempre(self):
        d = self.pl.decidir('A1', '452', 'rapido')
        self.assertEqual((d['pleito'], d['modo'], d['origem']), ('453', 'rapido', 'rotacao'))
        json.dump({'historico_modos': {'453': 'rapido'}}, open(os.path.join(self.c, 'rotacao.json'), 'w'))
        d = self.pl.decidir('A2', '452', 'cadencia')
        self.assertEqual((d['pleito'], d['modo']), ('453', 'cadencia'))
        d = self.pl.decidir('A3', '3220', 'rapido')
        self.assertEqual((d['pleito'], d['modo']), ('3221', 'espera'))

    def test_duracao_medida_entra_na_previsao(self):
        self.pl.decidir('D1', '452', 'rapido', '2026-10-19T14:01:00-03:00', '2026-10-19T14:21:00-03:00')
        self.assertEqual(self.pl.duracao('452-rapido'), 1200)

    def test_agenda_em_horas_fechadas(self):
        f = self.pl.fila()
        # 452 rápido iniciou 14:01, ~8 min: termina 14:09; o próximo começa às 15:00
        self.assertEqual(f[0]['inicio_previsto'], '2026-10-19T15:00:00-03:00')
        self.assertEqual(f[0]['item'], '453-rapido')
        for a, b in zip(f, f[1:]):
            ti = datetime.fromisoformat(b['inicio_previsto'])
            self.assertEqual((ti.minute, ti.second), (0, 0))
            self.assertGreaterEqual(ti, datetime.fromisoformat(a['fim_previsto']))   # a vitrine vai até a virada
        self.assertIn('3221-espera', [x['item'] for x in f])

    def test_suspensao_um_dia_antes_ate_tres_depois(self):
        el = [date(2026, 10, 25)]
        self.assertIsNone(up.suspensao_em(datetime(2026, 10, 23, 23, 59, tzinfo=BR), el))
        self.assertIsNotNone(up.suspensao_em(datetime(2026, 10, 24, 0, 0, tzinfo=BR), el))
        self.assertIsNotNone(up.suspensao_em(datetime(2026, 10, 28, 23, 59, tzinfo=BR), el))
        self.assertIsNone(up.suspensao_em(datetime(2026, 10, 29, 0, 0, tzinfo=BR), el))

    def test_suspensa_nao_vota_nao_decide_e_agenda_pula(self):
        self.rel.t = datetime(2026, 10, 23, 21, 30, tzinfo=BR)
        f = self.pl.fila()
        inicios = [datetime.fromisoformat(x['inicio_previsto']) for x in f]
        self.assertFalse(any(datetime(2026, 10, 24, tzinfo=BR) <= t < datetime(2026, 10, 29, tzinfo=BR) for t in inicios))
        self.rel.t = datetime(2026, 10, 25, 10, 0, tzinfo=BR)
        k = up.chave_ip('1.2.3.4', self.pl.e['sal'])
        self.assertEqual(self.pl.votar(k, '453-rapido')[0], 423)
        self.assertTrue(self.pl.decidir('S1', '452', 'rapido')['suspensa'])
        p = self.pl.publico(k)
        self.assertEqual(p['suspensao']['retoma_em'], '2026-10-29T00:00:00-03:00')
        self.assertEqual(p['fila'], [])


class TestHttp(unittest.TestCase):
    def setUp(self):
        self.c = ciclo()
        self.pl = up.Playlist(self.c, eleicoes=[date(2030, 1, 1)])
        html = os.path.join(self.c, 'p.html')
        open(html, 'w').write('<!doctype html><title>Playlist</title>')
        self.srv = up.ThreadingHTTPServer(('127.0.0.1', 0), up.criar_tratador(self.pl, html, 'segredo'))
        threading.Thread(target=self.srv.serve_forever, daemon=True).start()
        self.base = f'http://127.0.0.1:{self.srv.server_address[1]}'

    def tearDown(self):
        self.srv.shutdown()
        shutil.rmtree(self.c, ignore_errors=True)

    def pedir(self, metodo, caminho, corpo=None, cab=None):
        cab = dict(cab or {})
        dados = None
        if corpo is not None:
            dados = json.dumps(corpo).encode()
            cab['Content-Type'] = 'application/json'
        r = urllib.request.Request(self.base + caminho, data=dados, headers=cab, method=metodo)
        try:
            with urllib.request.urlopen(r) as resp:
                return resp.status, resp.read()
        except urllib.error.HTTPError as e:
            return e.code, e.read()

    def test_fluxo(self):
        st, b = self.pedir('GET', '/playlist')
        self.assertEqual(st, 200)
        self.assertIn(b'Playlist', b)
        # dois visitantes atrás do Caddy (IP no X-Real-IP)
        st, b = self.pedir('POST', '/api/v1/playlist/voto', {'item': '453-cadencia'}, {'X-Real-IP': '200.0.0.1'})
        self.assertEqual(st, 200)
        self.assertEqual(json.loads(b)['meu_voto'], '453-cadencia')
        st, b = self.pedir('POST', '/api/v1/playlist/voto', {'item': '453-cadencia'}, {'X-Real-IP': '200.0.0.2'})
        st, b = self.pedir('GET', '/api/v1/playlist', cab={'X-Real-IP': '200.0.0.3'})
        j = json.loads(b)
        self.assertIsNone(j['meu_voto'])
        self.assertEqual(j['fila'][0]['item'], '453-cadencia')
        self.assertEqual(j['fila'][0]['votos'], 2)
        # sem o segredo, nada de decisão
        self.assertEqual(self.pedir('POST', '/api/v1/playlist/decisao', {'id': 'X', 'pleito': '452'})[0], 403)
        st, b = self.pedir('POST', '/api/v1/playlist/decisao', {'id': 'X', 'pleito': '452', 'modo': 'rapido'},
                           {'X-UV-Segredo': 'segredo'})
        self.assertEqual((st, json.loads(b)['pleito'], json.loads(b)['modo']), (200, '453', 'cadencia'))
        # voto sem JSON (formulário de outro site) é recusado
        r = urllib.request.Request(self.base + '/api/v1/playlist/voto', data=b'item=452-rapido', method='POST')
        with self.assertRaises(urllib.error.HTTPError):
            urllib.request.urlopen(r)


class TestProximaAoVivo(unittest.TestCase):
    """M-004: a contagem da página para a próxima eleição ao vivo (2º turno de 2026, domingo 25/10, 17h)."""
    ELEI = [date(2026, 10, 25)]

    def test_antes_durante_e_depois(self):
        p = up.proxima_ao_vivo(datetime(2026, 10, 20, 15, 0, tzinfo=BR), self.ELEI, (17, 0))
        self.assertEqual(p['inicio'], '2026-10-25T17:00:00-03:00')
        self.assertEqual(p['nome'], '2º turno das Eleições Gerais 2026')
        self.assertFalse(p['ao_vivo'])
        p = up.proxima_ao_vivo(datetime(2026, 10, 25, 18, 0, tzinfo=BR), self.ELEI, (17, 0))
        self.assertTrue(p['ao_vivo'])
        self.assertEqual(p['fim'], '2026-10-26T08:00:00-03:00')
        self.assertIsNone(up.proxima_ao_vivo(datetime(2026, 10, 26, 8, 0, tzinfo=BR), self.ELEI, (17, 0)))

    def test_hora_configuravel_e_no_publico(self):
        self.assertEqual(up.inicio_apuracao('18:30'), (18, 30))
        self.assertEqual(up.inicio_apuracao(''), (17, 0))
        c = ciclo()
        try:
            pl = up.Playlist(c, eleicoes=self.ELEI, relogio=Relogio(datetime(2026, 10, 20, 14, 40, tzinfo=BR)))
            j = pl.publico()
            self.assertEqual(j['proxima_ao_vivo']['eleicao'], '2026-10-25')
            self.assertIn('inicio', j['proxima_ao_vivo'])
        finally:
            shutil.rmtree(c, ignore_errors=True)


if __name__ == '__main__':
    unittest.main()
