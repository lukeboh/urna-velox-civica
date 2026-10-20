"""M-002: testes das medidas (fila_noite.py e medir_vazao.py) com dados sintéticos de resposta conhecida.

    python3 -m unittest discover -s aquisicao/scripts/testes -p 'test_*.py'
"""
import gzip
import importlib.util
import json
import os
import tempfile
import unittest
from collections import Counter
from datetime import datetime, timedelta

AQUI = os.path.dirname(os.path.abspath(__file__))


def carregar(nome):
    spec = importlib.util.spec_from_file_location(nome, os.path.join(os.path.dirname(AQUI), f'{nome}.py'))
    m = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(m)
    return m


fila = carregar('fila_noite')
medir = carregar('medir_vazao')
T0 = datetime(2024, 10, 6, 18, 0, 0)


class FilaNoite(unittest.TestCase):
    def test_capacidade_folgada_nao_atrasa(self):
        chegada = Counter({T0 + timedelta(seconds=s): 10 for s in range(600)})  # 10 BU/s por 10 min
        r = fila.simular(chegada, 1, 2, 0)                                       # 50 BU/s
        self.assertEqual((r['capacidade_bu_s'], r['atraso_max_s'], r['atraso_p99_s']), (50.0, 0, 0))

    def test_rajada_acima_da_capacidade(self):
        # 6.000 BUs de uma vez; 50 BU/s (50 já saem no próprio segundo da chegada: fila máx. 5.950) → o último sai
        # 119 s depois, e a fila zera nesse mesmo segundo
        r = fila.simular(Counter({T0: 6000}), 1, 2, 0)
        self.assertEqual((r['fila_max'], r['atraso_max_s']), (5950, 119))
        self.assertEqual(r['fila_zera_em'], f'{T0 + timedelta(seconds=119):%d/%m %H:%M}')
        # dois nós: metade do tempo
        self.assertEqual(fila.simular(Counter({T0: 6000}), 2, 2, 0)['atraso_max_s'], 59)

    def test_cs_json_desconta_da_capacidade(self):
        self.assertAlmostEqual(fila.simular(Counter({T0: 1}), 1, 2, 30)['capacidade_bu_s'], 49.5)


class MedirVazao(unittest.TestCase):
    def test_execucao_sintetica(self):
        """Serviço no ar em t0; 1.000 BUs aplicados entre t0+10 s e t0+20 s (100 BU/s); 4 s de CPU em 20 s."""
        d = tempfile.mkdtemp(prefix='uv-m002-teste-')
        no = os.path.join(d, 'no-a', 'exec')
        os.makedirs(no)
        t0 = datetime(2026, 10, 18, 13, 0, 0).astimezone()
        ms0 = int(t0.timestamp() * 1000)
        with gzip.open(os.path.join(no, 'secoes.log.gz'), 'wt') as f:
            f.write('t_ms,acao,uf\n')
            for i in range(1000):
                f.write(f'{ms0 + 10000 + i * 10},aplicado,sp\n')
            f.write(f'{ms0 + 21000},confirmado,sp\n')

        def amostra(s, cpu, fila_):
            t = (t0 + timedelta(seconds=s)).isoformat()
            return {'t': t, 'sistema': {'cpu_pct': 50.0}, 'processos': {'uv-servico': {'pid': 7, 'cpu_s': cpu}},
                    'saude': {'agora': t, 'ativo_s': s, 'fila': fila_, 'versao': 't',
                              'rede': {'aux': 500}, 'federacao': {'recebidos': 500, 'ja_aplicados': 5},
                              'custos_ms': {'lote': {'n': 10, 'p50': 2.0, 'p99': 9.0, 'max': 9.0}}}}
        with gzip.open(os.path.join(no, 'telemetria.jsonl.gz'), 'wt') as f:
            for s, cpu, fl in ((5, 1.0, 0), (15, 3.0, 400), (25, 5.0, 0)):
                f.write(json.dumps(amostra(s, cpu, fl)) + '\n')
        r = medir.medir_no(no)
        self.assertEqual(r['bus_aplicados'], 1000)
        self.assertEqual(r['fases_s']['partida_ate_1o_bu'], 10.0)
        self.assertEqual(r['fases_s']['100%'], 20.0)
        self.assertAlmostEqual(r['vazao_bu_s']['media'], 100.1, delta=0.5)
        self.assertEqual(r['cpu']['processos']['uv-servico'], {'cpu_s': 4.0, 'nucleos': 0.2})
        self.assertEqual(r['fila_max'], 400)
        self.assertEqual(r['federacao']['repetidos_pct'], 1.0)
        self.assertEqual(r['custos_ms']['lote']['n_x_p50_s'], 0.0)
        self.assertEqual(medir.pasta_do_no(d, 'no-a'), no)


if __name__ == '__main__':
    unittest.main()
