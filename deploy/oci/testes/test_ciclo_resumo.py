"""B-002 e B-003: RESUMO.json do ciclo (servidor/uv-ciclo-resumo.py) com execuções sintéticas.

B-002: os BUs recebidos no TSE dias depois da noite entram na simulação no corte (02:00; `disponivel` = corte, `recebido`
= a data real). Medidos pelo `recebido`, empurravam o "maior recebido até aqui" para dias à frente e todo BU aplicado
depois parecia atrasado dias (p99 de 2,8 dias no 3220).
B-003: um reinício intencional (`63_ciclo.sh --reiniciar`, que marca `atual/reinicios_intencionais`) reprovava a
execução como uma queda; uma queda de verdade (sem a marca) continua reprovando.

    python3 -m unittest discover -s deploy/oci/testes -p 'test_*.py'
"""
import json
import os
import subprocess
import sys
import tempfile
import unittest
from datetime import datetime, timedelta

AQUI = os.path.dirname(os.path.abspath(__file__))
RESUMO = os.path.join(os.path.dirname(AQUI), 'servidor', 'uv-ciclo-resumo.py')
NOITE = datetime(2024, 10, 6, 17, 0, 0)
CORTE = datetime(2024, 10, 7, 2, 0, 0)
F = '%d/%m/%Y %H:%M:%S'


def execucao(bus, amostras, marcas=None):
    """Pasta de uma execução: params, saida/{secoes.log,saude.json,conferencia.json}, telemetria.jsonl."""
    n = tempfile.mkdtemp(prefix='uv-resumo-')
    os.makedirs(os.path.join(n, 'saida'))
    with open(os.path.join(n, 'id'), 'w') as f:
        f.write('20261018-1900-452-rapido\n')
    with open(os.path.join(n, 'params'), 'w') as f:
        f.write('pleito=452\nmodo=rapido\ninicio=2026-10-18T19:00:00-03:00\n')
    with open(os.path.join(n, 'saida', 'secoes.log'), 'w') as f:
        f.write('t_ms,acao,uf,municipio,zona,secao,hash,sha256,situacao,anulada,disponivel,recebido,arquivo,pos,tam,seq\n')
        for i, (rec, disp) in enumerate(bus, 1):
            f.write(f'{1792360000000 + i},aplicado,sp,1,1,{i},h,s,Totalizado,0,{disp:{F}},{rec:{F}},sp.uvpk,0,1,{i}\n')
    json.dump({'estado': 'encerrada', 'apuracao': {'encerrada': True}, 'secoes': {'aplicadas': len(bus)},
               'motor': {'duplicados': 0}, 'rede': {'http429': 0}}, open(os.path.join(n, 'saida', 'saude.json'), 'w'))
    json.dump({'identicos': 1, 'arquivos': 1, 'divergentes': [], 'situacoes': {'divergentes': 0}, 'matematicamente': {'revogacoes': 0}},
              open(os.path.join(n, 'saida', 'conferencia.json'), 'w'))
    with open(os.path.join(n, 'telemetria.jsonl'), 'w') as f:
        for t, pid in amostras:
            f.write(json.dumps({'t': t.isoformat(), 'processos': {'uv-servico': {'pid': pid, 'rss_mb': 100}},
                                'sistema': {'cpu_pct': 10, 'disco_livre_gb': 50}, 'saude': {'apuracao': {'encerrada': False}}}) + '\n')
    if marcas:
        with open(os.path.join(n, 'reinicios_intencionais'), 'w') as f:
            for t in marcas:
                f.write(t.isoformat() + '\n')
    return n


def resumo(n):
    h = tempfile.mkdtemp(prefix='uv-resumo-h-')
    r = subprocess.run([sys.executable, RESUMO, n, h], capture_output=True, text=True, check=True)
    return json.loads(r.stdout)


AGORA = datetime(2026, 10, 18, 19, 0, 0).astimezone()
ESTAVEL = [(AGORA + timedelta(seconds=15 * k), 100) for k in range(10)]


class OrdemB002(unittest.TestCase):
    def test_recebidos_depois_do_corte_nao_dominam_o_atraso(self):
        # 1.000 BUs na ordem da noite (1 por segundo); 20 recebidos dias depois (no corte) aplicados no meio
        noite = [(NOITE + timedelta(seconds=k), NOITE + timedelta(seconds=k)) for k in range(1000)]
        tarde = [(datetime(2024, 10, 13, 0, 0, 7), CORTE)] * 20
        r = resumo(execucao(noite[:500] + tarde + noite[500:], ESTAVEL))
        o = r['ordem']
        # os 500 BUs depois dos tardios estão na ordem da noite: atraso 0 para eles
        self.assertLessEqual(o['atraso_recebimento_s']['p99'], 60, o)
        self.assertEqual(o.get('recebidos_depois_do_corte'), 20, o)

    def test_ordem_normal_nao_muda(self):
        noite = [(NOITE + timedelta(seconds=k), NOITE + timedelta(seconds=k)) for k in range(100)]
        noite[10], noite[20] = noite[20], noite[10]          # uma troca: o BU de 10 s sai depois do de 20 s
        o = resumo(execucao(noite, ESTAVEL))['ordem']
        self.assertEqual(o['atraso_recebimento_s']['max'], 10.0, o)


class ReinicioB003(unittest.TestCase):
    def test_reinicio_intencional_nao_reprova(self):
        am = ESTAVEL[:5] + [(t, 200) for t, _ in ESTAVEL[5:]]  # pid muda entre a 5ª e a 6ª amostra
        r = resumo(execucao([(NOITE, NOITE)], am, marcas=[ESTAVEL[4][0] + timedelta(seconds=5)]))
        self.assertEqual(r['veredito'], 'aprovado', r)
        self.assertEqual(r['telemetria']['reinicios_servico'], 1)
        self.assertEqual(r['telemetria']['reinicios_intencionais'], 1)
        self.assertTrue(any('intencional' in x for x in r['observacoes']), r['observacoes'])

    def test_queda_continua_reprovando(self):
        am = ESTAVEL[:5] + [(t, 200) for t, _ in ESTAVEL[5:]]
        r = resumo(execucao([(NOITE, NOITE)], am))
        self.assertEqual(r['veredito'], 'falhou', r)
        self.assertIn('1 reinícios do serviço', ' '.join(r['falhas']))

    def test_marca_antiga_nao_desculpa_queda_posterior(self):
        # marca às 19h00m05s, queda às 19h02m (mais de 1 min depois): reprova
        am = ESTAVEL[:8] + [(t, 200) for t, _ in ESTAVEL[8:]]
        r = resumo(execucao([(NOITE, NOITE)], am, marcas=[AGORA + timedelta(seconds=5)]))
        self.assertEqual(r['veredito'], 'falhou', r)


if __name__ == '__main__':
    unittest.main()
