"""Multipleito (1.1): playlist, vigia e uv-ciclo.sh a partir do pleitos.json (servidor/uv_pleitos.py), sem a lista fixa.

O pleitos.json de fixture saiu do `uv-coletor pleitos` sobre o ele-c.json real de 29/10/2026
(aquisicao/tests/fixtures/ele-c-2026-10-29.json), com acervo pronto para 452, 453, 3220, 3221 e os suplementares 3080
(Paranhos/MS) e 3040 (Neves Paulista e Eldorado/SP), montados de verdade pelo montar_acervo.sh.

    python3 -m unittest discover -s deploy/oci/testes -p 'test_*.py'
"""
import copy
import importlib.util
import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from datetime import date, datetime, timedelta, timezone

AQUI = os.path.dirname(os.path.abspath(__file__))
SERV = os.path.join(os.path.dirname(AQUI), 'servidor')
FIXTURE = os.path.join(AQUI, 'fixtures', 'pleitos-2026-10-29.json')
BR = timezone(timedelta(hours=-3))


def carregar(nome, arquivo):
    spec = importlib.util.spec_from_file_location(nome, os.path.join(SERV, arquivo))
    m = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(m)
    return m


sys.path.insert(0, SERV)
import uv_pleitos  # noqa: E402

up = carregar('uv_playlist_dinamica', 'uv-playlist.py')


class Relogio:
    def __init__(self, t):
        self.t = t

    def __call__(self):
        return self.t


def pasta_com(pleitos_extra=(), sem_acervo=()):
    """Pasta temporária com o pleitos.json da fixture (mais `pleitos_extra`, menos o acervo de `sem_acervo`)."""
    d = tempfile.mkdtemp(prefix='uv-pleitos-')
    j = json.load(open(FIXTURE, encoding='utf-8'))
    for p in j['pleitos']:
        if str(p['codigo']) in sem_acervo:
            p['replay'] = False
    j['pleitos'] = list(pleitos_extra) + j['pleitos']
    json.dump(j, open(os.path.join(d, 'pleitos.json'), 'w', encoding='utf-8'), ensure_ascii=False)
    os.makedirs(os.path.join(d, 'acervo'))
    return d


def suplementar_agendado(codigo=3250, data='06/12/2026', iso='2026-12-06', local='Vila Nova (SP)', situacao='agendado'):
    return {'codigo': codigo, 'ciclo': 'ele2024', 'data': data, 'data_iso': iso, 'ano': int(iso[:4]), 'turno': 1,
            'situacao': situacao, 'nome': f'Suplementar de {local}', 'titulo': f'Eleição suplementar · {local}',
            'complementares': 0, 'ufs': ['sp'], 'replay': False,
            'eleicoes': [{'codigo': 6300, 'nome': 'Eleição Suplementar - Vila Nova', 'local': local, 'turno': 1,
                          'tipo': 'suplementar', 'cargos': [{'codigo': 11, 'nome': 'Prefeito'}], 'ufs': ['sp'],
                          'municipios': [99999]}]}


class TestCatalogo(unittest.TestCase):
    def setUp(self):
        self.d = pasta_com()
        self.cat = uv_pleitos.Catalogo(os.path.join(self.d, 'pleitos.json'), os.path.join(self.d, 'acervo'))

    def tearDown(self):
        shutil.rmtree(self.d, ignore_errors=True)

    def test_rotacao_do_mais_antigo_para_o_mais_novo(self):
        self.assertEqual(self.cat.replays(), ['452', '453', '3040', '3080', '3220', '3221'])
        self.assertEqual(self.cat.seguinte('453'), '3040')
        self.assertEqual(self.cat.seguinte('3221'), '452', 'a rotação dá a volta')
        self.assertEqual(self.cat.seguinte('3237'), '3220', 'fora da rotação: o primeiro depois dele na ordem das datas')

    def test_info_de_um_suplementar(self):
        i = self.cat.info('3080')
        self.assertEqual((i['ciclo'], i['nome'], i['data_br'], i['corte'], i['tipo']),
                         ('ele2024', 'Suplementar de Paranhos (MS)', '06/04/2025', '07/04/2025 02:00:00', 'suplementar'))
        self.assertEqual(i['tempo_cadencia_s'], 3380, 'do acervo.json')
        self.assertEqual(i['eleicao'], 'Eleição suplementar · Paranhos (MS)')

    def test_info_dos_fixos_continua_igual(self):
        i = self.cat.info('3221')
        self.assertEqual((i['ciclo'], i['nome'], i['data_br'], i['corte'], i['tempo_cadencia_s'], i['eleicao']),
                         ('ele2026', '2º turno de 2026', '25/10/2026', '26/10/2026 02:00:00', 15300, 'Eleições Gerais 2026'))
        self.assertEqual(self.cat.info('452')['tempo_cadencia_s'], 13500)

    def test_sem_arquivo_vale_a_lista_fixa(self):
        c = uv_pleitos.Catalogo(os.path.join(self.d, 'nao-existe.json'), os.path.join(self.d, 'acervo'))
        self.assertEqual(c.replays(), ['452', '453', '3220', '3221'])
        self.assertIsNone(c.info('3080'))
        self.assertFalse(c.tem_lista())

    def test_relido_quando_muda(self):
        p = os.path.join(self.d, 'pleitos.json')
        j = json.load(open(p, encoding='utf-8'))
        for x in j['pleitos']:
            if x['codigo'] == 3237:
                x['replay'] = True
        tmp = p + '.tmp'
        json.dump(j, open(tmp, 'w', encoding='utf-8'))
        os.utime(tmp, ns=(os.stat(p).st_mtime_ns + 10**9,) * 2)
        os.replace(tmp, p)
        self.assertIn('3237', self.cat.replays(), 'pleito novo com acervo entra sem reiniciar nada')

    def test_acervo_json_local_basta_no_no_b(self):
        # o nó B: sem o pleitos.json, mas com o acervo montado (acervo.json)
        os.makedirs(os.path.join(self.d, 'acervo', '3080'))
        shutil.copy(os.path.join(AQUI, 'fixtures', 'acervo-3080.json'), os.path.join(self.d, 'acervo', '3080', 'acervo.json'))
        c = uv_pleitos.Catalogo(os.path.join(self.d, 'nao-existe.json'), os.path.join(self.d, 'acervo'))
        i = c.info('3080')
        self.assertEqual((i['ciclo'], i['corte'], i['tempo_cadencia_s']), ('ele2024', '07/04/2025 02:00:00', 3380))

    def test_linha_de_comando_do_uv_ciclo(self):
        env = dict(os.environ, UV_PLEITOS=os.path.join(self.d, 'pleitos.json'), UV_ACERVO=os.path.join(self.d, 'acervo'))
        r = subprocess.run([sys.executable, os.path.join(SERV, 'uv_pleitos.py'), 'info', '3080'], env=env,
                           capture_output=True, text=True, check=True)
        kv = dict(l.split('=', 1) for l in r.stdout.strip().splitlines())
        self.assertEqual((kv['ciclo'], kv['data_br'], kv['corte'], kv['tempo_cadencia_s']),
                         ('ele2024', '06/04/2025', '07/04/2025 02:00:00', '3380'))
        r = subprocess.run([sys.executable, os.path.join(SERV, 'uv_pleitos.py'), 'seguinte', '3221'], env=env,
                           capture_output=True, text=True, check=True)
        self.assertEqual(r.stdout.strip(), '452')
        r = subprocess.run([sys.executable, os.path.join(SERV, 'uv_pleitos.py'), 'info', '9999'], env=env,
                           capture_output=True, text=True)
        self.assertEqual(r.returncode, 2)


class TestPlaylistDinamica(unittest.TestCase):
    def novo(self, extra=(), agora=datetime(2026, 10, 30, 14, 5, tzinfo=BR), pleito='3221', modo='rapido'):
        self.d = pasta_com(extra)
        self.cat = uv_pleitos.Catalogo(os.path.join(self.d, 'pleitos.json'), os.path.join(self.d, 'acervo'))
        self.c = tempfile.mkdtemp(prefix='uv-playlist-din-')
        os.makedirs(os.path.join(self.c, 'atual', 'saida'))
        open(os.path.join(self.c, 'atual', 'id'), 'w').write(f'20261030-1400-{pleito}-{modo}\n')
        open(os.path.join(self.c, 'atual', 'params'), 'w').write(f'pleito={pleito}\nmodo={modo}\ninicio=2026-10-30T14:01:00-03:00\n')
        json.dump({'apuracao': {'encerrada': False}}, open(os.path.join(self.c, 'atual', 'saida', 'saude.json'), 'w'))
        json.dump({'historico_modos': {}}, open(os.path.join(self.c, 'rotacao.json'), 'w'))
        self.rel = Relogio(agora)
        self.pl = up.Playlist(self.c, eleicoes=[date(2026, 10, 25)], relogio=self.rel, catalogo=self.cat)
        return self.pl

    def tearDown(self):
        for x in (getattr(self, 'd', None), getattr(self, 'c', None)):
            if x:
                shutil.rmtree(x, ignore_errors=True)

    def test_suplementares_com_acervo_sao_votaveis(self):
        pl = self.novo()
        v = pl.votaveis()
        self.assertIn('3080-rapido', v)
        self.assertIn('3040-cadencia', v)
        self.assertNotIn('3237-rapido', v, 'sem acervo, fora da playlist')
        self.assertEqual(len(v), 12)
        k = up.chave_ip('200.1.2.3', pl.e['sal'])
        self.assertEqual(pl.votar(k, '3080-cadencia')[0], 200)
        self.assertEqual(pl.votar(k, '3237-rapido')[0], 400)

    def test_descricao_e_duracao_de_um_suplementar(self):
        pl = self.novo()
        d = pl.descrever('3080-cadencia')
        self.assertEqual((d['nome'], d['titulo'], d['tipo'], d['data_eleicao']),
                         ('Suplementar de Paranhos (MS)', 'Suplementar de Paranhos (MS)', 'suplementar', '2025-04-06'))
        self.assertEqual(pl.descrever('3221-rapido')['titulo'], '2º turno · 2026', 'ordinária: como antes')
        self.assertEqual(pl.duracao('3080-cadencia'), 3380 + 600)
        self.assertEqual(pl.duracao('3080-rapido'), 120)
        self.assertEqual(pl.duracao('3220-rapido'), 600, 'medidas de antes para os fixos')

    def test_rotacao_sem_votos_passa_pelos_suplementares(self):
        pl = self.novo(pleito='453', modo='cadencia')
        p, m = pl.padrao_seguinte('453', {})
        self.assertEqual((p, m), ('3040', 'rapido'))
        f = pl.fila()
        self.assertEqual([x['pleito'] for x in f[:5]], ['3040', '3080', '3220', '3221', '452'])
        dec = pl.decidir('20261030-1400-453-cadencia', '453', 'cadencia')
        self.assertEqual((dec['pleito'], dec['origem']), ('3040', 'rotacao'))

    def test_suplementar_agendado_suspende_so_a_noite(self):
        pl = self.novo([suplementar_agendado()], agora=datetime(2026, 12, 1, 10, 0, tzinfo=BR))
        self.assertEqual(pl.curtas, [date(2026, 12, 6)])
        susp, prox = pl.suspensao()
        self.assertIsNone(susp)
        self.assertEqual((prox['eleicao'], prox['nome']), ('2026-12-06', 'Eleição suplementar · Vila Nova (SP)'))
        p = pl.publico()['proxima_ao_vivo']
        self.assertEqual((p['eleicao'], p['nome'], p['ao_vivo']), ('2026-12-06', 'Eleição suplementar · Vila Nova (SP)', False))
        self.assertIsNone(up.suspensao_em(datetime(2026, 12, 5, 23, 59, tzinfo=BR), pl.eleicoes, pl.curtas),
                          'suplementar: sem o dia anterior')
        self.assertIsNotNone(up.suspensao_em(datetime(2026, 12, 6, 0, 0, tzinfo=BR), pl.eleicoes, pl.curtas))
        self.assertIsNotNone(up.suspensao_em(datetime(2026, 12, 7, 7, 59, tzinfo=BR), pl.eleicoes, pl.curtas))
        self.assertIsNone(up.suspensao_em(datetime(2026, 12, 7, 8, 0, tzinfo=BR), pl.eleicoes, pl.curtas))
        self.rel.t = datetime(2026, 12, 6, 18, 0, tzinfo=BR)
        k = up.chave_ip('200.1.2.3', pl.e['sal'])
        self.assertEqual(pl.votar(k, '3080-rapido')[0], 423)
        self.assertTrue(pl.decidir('x', '3221', 'rapido').get('suspensa'))

    def test_ordinaria_agendada_suspende_como_antes(self):
        ord_ = suplementar_agendado(codigo=3300, data='04/10/2028', iso='2028-10-04', local='x')
        ord_['eleicoes'][0]['tipo'] = 'ordinaria'
        ord_['nome'], ord_['titulo'] = '1º turno de 2028', 'Eleições Municipais 2028 · 1º turno'
        pl = self.novo([ord_], agora=datetime(2028, 10, 1, 10, 0, tzinfo=BR))
        self.assertIn(date(2028, 10, 4), pl.eleicoes)
        self.assertIsNotNone(up.suspensao_em(datetime(2028, 10, 3, 0, 0, tzinfo=BR), pl.eleicoes, pl.curtas))
        self.assertIsNotNone(up.suspensao_em(datetime(2028, 10, 7, 23, 59, tzinfo=BR), pl.eleicoes, pl.curtas))
        self.assertEqual(pl.suspensao()[1]['nome'], '1º turno das Eleições Municipais 2028')


class TestVigiaDinamico(unittest.TestCase):
    def setUp(self):
        self.d = pasta_com()
        os.environ['UV_PLEITOS'] = os.path.join(self.d, 'pleitos.json')
        os.environ['UV_ACERVO'] = os.path.join(self.d, 'acervo')
        self.c = tempfile.mkdtemp(prefix='uv-vigia-din-')
        sys_argv = sys.argv
        sys.argv = ['uv-ciclo-vigia.py', self.c, 'no-a', '1']
        try:
            self.v = carregar('uv_vigia_dinamico', 'uv-ciclo-vigia.py')
        finally:
            sys.argv = sys_argv

    def tearDown(self):
        os.environ.pop('UV_PLEITOS', None)
        os.environ.pop('UV_ACERVO', None)
        shutil.rmtree(self.d, ignore_errors=True)
        shutil.rmtree(self.c, ignore_errors=True)

    def test_rotacao_local_pelo_catalogo(self):
        self.assertEqual(self.v.proximo_pleito('453'), '3040')
        self.assertEqual(self.v.proximo_pleito('3221'), '452')

    def test_virada_a_cada_meia_hora(self):
        t = lambda h, m: datetime(2026, 10, 30, h, m, tzinfo=BR)
        self.assertEqual(self.v.proxima_hora_fechada(t(14, 9)), t(14, 30))
        self.assertEqual(self.v.proxima_hora_fechada(t(14, 28)), t(15, 0), 'menos de 3 min: a virada seguinte')
        self.assertEqual(self.v.proxima_hora_fechada(t(14, 40), margem_s=120), t(15, 0))
        os.environ['UV_PASSO_MIN'] = '60'
        try:
            self.assertEqual(self.v.proxima_hora_fechada(t(14, 9)), t(15, 0), 'PASSO_MIN=60: a hora fechada de antes')
        finally:
            os.environ.pop('UV_PASSO_MIN')

    def test_suspensao_curta_de_suplementar_agendado(self):
        p = os.path.join(self.d, 'pleitos.json')
        j = json.load(open(p, encoding='utf-8'))
        hoje = datetime.now(BR).date() + timedelta(days=2)
        j['pleitos'].insert(0, suplementar_agendado(data=hoje.strftime('%d/%m/%Y'), iso=hoje.isoformat()))
        json.dump(j, open(p, 'w', encoding='utf-8'))
        dia = datetime(hoje.year, hoje.month, hoje.day, tzinfo=BR)
        self.assertIsNone(self.v.suspensao_em(dia - timedelta(minutes=1)))
        fim, d = self.v.suspensao_em(dia + timedelta(hours=20))
        self.assertEqual((fim, d), (dia + timedelta(hours=32), hoje))


if __name__ == '__main__':
    unittest.main()


class TestAoVivo(unittest.TestCase):
    """Pleito em curso no ele-c.json → totalização ao vivo, sozinha (autorizada pelo Luciano em 29/10/2026)."""

    def setUp(self):
        hoje = datetime.now(BR).date()
        self.hoje = hoje
        self.d = pasta_com([suplementar_agendado(codigo=3250, data=hoje.strftime('%d/%m/%Y'), iso=hoje.isoformat(), situacao='em_curso')])
        self.pj = os.path.join(self.d, 'pleitos.json')
        self.cat = uv_pleitos.Catalogo(self.pj, os.path.join(self.d, 'acervo'))

    def tearDown(self):
        shutil.rmtree(self.d, ignore_errors=True)

    def test_em_curso_calculado_na_hora(self):
        dia = datetime(self.hoje.year, self.hoje.month, self.hoje.day, tzinfo=BR)
        self.assertEqual(self.cat.ao_vivo_agora(dia + timedelta(hours=18)), '3250')
        self.assertIsNone(self.cat.ao_vivo_agora(dia - timedelta(minutes=1)), 'antes do dia: agendado')
        self.assertIsNone(self.cat.ao_vivo_agora(dia + timedelta(hours=32)), '8 h do dia seguinte: encerrado')
        self.assertEqual(self.cat.ufs('3250'), 'sp')
        self.assertEqual(self.cat.ufs('3221'), '', 'pleito com "br": todas as UFs')
        i = self.cat.info('3250')
        self.assertEqual((i['ciclo'], i['data_br']), ('ele2024', self.hoje.strftime('%d/%m/%Y')), 'em curso, sem acervo: dados do pleitos.json')

    def test_ordinaria_tem_preferencia(self):
        j = json.load(open(self.pj, encoding='utf-8'))
        o = suplementar_agendado(codigo=3300, data=self.hoje.strftime('%d/%m/%Y'), iso=self.hoje.isoformat(), situacao='em_curso')
        o['eleicoes'][0]['tipo'] = 'ordinaria'
        j['pleitos'].insert(0, o)
        json.dump(j, open(self.pj, 'w', encoding='utf-8'))
        self.assertEqual(self.cat.em_curso(datetime.combine(self.hoje, datetime.min.time(), BR) + timedelta(hours=18)), ['3300', '3250'])

    def vigia(self, pleito, modo, rot=None):
        c = tempfile.mkdtemp(prefix='uv-vigia-aovivo-')
        self.addCleanup(shutil.rmtree, c, True)
        os.makedirs(os.path.join(c, 'atual', 'saida'))
        open(os.path.join(c, 'atual', 'id'), 'w').write(f'20261030-1400-{pleito}-{modo}\n')
        open(os.path.join(c, 'atual', 'params'), 'w').write(f'pleito={pleito}\nmodo={modo}\ninicio={datetime.now(BR).isoformat()}\n')
        json.dump({'apuracao': {'encerrada': False}}, open(os.path.join(c, 'atual', 'saida', 'saude.json'), 'w'))
        json.dump(rot or {'historico_modos': {}}, open(os.path.join(c, 'rotacao.json'), 'w'))
        anot, falso = os.path.join(c, 'systemd-run.txt'), os.path.join(c, 'systemd-run')
        open(falso, 'w').write(f'#!/bin/sh\necho "$@" >> {anot}\n')
        os.chmod(falso, 0o755)
        env = dict(os.environ, UV_SYSTEMD_RUN=falso, UV_CICLO_ENV='/nao/existe', UV_ELEICOES_OFICIAIS='2030-01-01',
                   UV_PLEITOS=self.pj, UV_ACERVO=os.path.join(self.d, 'acervo'))
        try:
            subprocess.run([sys.executable, os.path.join(SERV, 'uv-ciclo-vigia.py'), c, 'no-a', '0.2'], env=env, timeout=4,
                           capture_output=True)
        except subprocess.TimeoutExpired:
            pass
        return (open(anot).read() if os.path.exists(anot) else ''), json.load(open(os.path.join(c, 'rotacao.json')))

    def test_vigia_troca_o_replay_pelo_ao_vivo_na_hora(self):
        saida, rot = self.vigia('452', 'cadencia')
        self.assertIn('avancar 3250 aovivo', saida, 'sem esperar a virada nem o fim do replay')
        self.assertEqual((rot['proximo']['pleito'], rot['proximo']['modo']), ('3250', 'aovivo'))

    def test_vigia_nao_repete_com_o_ao_vivo_no_ar(self):
        saida, _ = self.vigia('3250', 'aovivo')
        self.assertNotIn('avancar', saida)

    def test_tentativa_recente_espera_10_min(self):
        rot = {'historico_modos': {}, 'aovivo_tentativa': {'pleito': '3250', 'em': (datetime.now(BR) - timedelta(minutes=3)).isoformat()}}
        saida, _ = self.vigia('452', 'rapido', rot)
        self.assertNotIn('avancar', saida, 'a partida anterior não pegou há 3 min: espera')
        rot['aovivo_tentativa']['em'] = (datetime.now(BR) - timedelta(minutes=11)).isoformat()
        saida, _ = self.vigia('452', 'rapido', rot)
        self.assertIn('avancar 3250 aovivo', saida)

    def test_playlist_mostra_o_ao_vivo_no_ar(self):
        c = tempfile.mkdtemp(prefix='uv-playlist-aovivo-')
        self.addCleanup(shutil.rmtree, c, True)
        os.makedirs(os.path.join(c, 'atual', 'saida'))
        open(os.path.join(c, 'atual', 'id'), 'w').write('20261030-1800-3250-aovivo\n')
        open(os.path.join(c, 'atual', 'params'), 'w').write(f'pleito=3250\nmodo=aovivo\ninicio={datetime.now(BR).isoformat()}\n')
        pl = up.Playlist(c, eleicoes=[], catalogo=self.cat)
        na = pl.publico()['no_ar']
        self.assertEqual((na['pleito'], na['modo'], na['modo_nome'], na['votavel']), ('3250', 'aovivo', 'Ao vivo', False))
        self.assertIsNotNone(pl.publico()['suspensao'], 'suplementar em curso: playlist suspensa até as 8 h do dia seguinte')

    def test_linha_de_comando(self):
        env = dict(os.environ, UV_PLEITOS=self.pj, UV_ACERVO=os.path.join(self.d, 'acervo'))
        r = subprocess.run([sys.executable, os.path.join(SERV, 'uv_pleitos.py'), 'aovivo'], env=env, capture_output=True, text=True)
        self.assertEqual((r.returncode, r.stdout.strip()), (0, '3250'))
        r = subprocess.run([sys.executable, os.path.join(SERV, 'uv_pleitos.py'), 'ufs', '3250'], env=env, capture_output=True, text=True)
        self.assertEqual(r.stdout.strip(), 'sp')
