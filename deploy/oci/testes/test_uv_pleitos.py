"""uv-pleitos.sh (multipleito, 1.1): pleitos.json a cada rodada, acervos que faltam, nunca na noite de uma eleição nem
no dia seguinte, falha com espera. Sem rede: uv-coletor e montar_acervo.sh falsos, que só anotam as chamadas.

    python3 -m unittest discover -s deploy/oci/testes -p 'test_*.py'
"""
import json
import os
import shutil
import stat
import subprocess
import tempfile
import unittest
from datetime import datetime, timedelta, timezone

AQUI = os.path.dirname(os.path.abspath(__file__))
SCRIPT = os.path.join(os.path.dirname(AQUI), 'servidor', 'uv-pleitos.sh')
FIXTURE = os.path.join(AQUI, 'fixtures', 'pleitos-2026-10-29.json')
BR = timezone(timedelta(hours=-3))

COLETOR = '''#!/usr/bin/env bash
echo "uv-coletor $*" >> "$ANOT"
if [ "$1" = preparar ]; then
  while [ $# -gt 0 ]; do [ "$1" = --saida ] && saida="$2"; shift; done
  [ "${PREPARAR_RC:-0}" = 1 ] && { echo "cs.json ainda não publicado (404)"; exit 1; }
  echo '{"pleito":1}' > "$saida/candidatos.json"; exit "${PREPARAR_RC:-0}"
fi
saida=""; while [ $# -gt 0 ]; do [ "$1" = --saida ] && saida="$2"; [ "$1" = --guardar ] && echo '{}' > "$2"; shift; done
[ -n "$saida" ] && cp "$PLEITOS_FIXO" "$saida"
exit 0
'''
MONTAR = '''#!/usr/bin/env bash
echo "montar $1" >> "$ANOT"
case " $MONTAR_OK " in *" $1 "*) mkdir -p "$2"; touch "$2/.completo"; exit 0 ;; esac
echo "falhou de propósito" >&2; exit 3
'''


def executavel(caminho, texto):
    with open(caminho, 'w') as f:
        f.write(texto)
    os.chmod(caminho, os.stat(caminho).st_mode | stat.S_IEXEC)


class TestUvPleitos(unittest.TestCase):
    def setUp(self):
        self.t = tempfile.mkdtemp(prefix='uv-pleitos-sh-')
        self.bin = os.path.join(self.t, 'bin')
        os.makedirs(self.bin)
        executavel(os.path.join(self.bin, 'uv-coletor'), COLETOR)
        executavel(os.path.join(self.bin, 'montar_acervo.sh'), MONTAR)
        self.anot = os.path.join(self.t, 'chamadas')
        self.fixo = os.path.join(self.t, 'fixo.json')
        self.pd, self.ac = os.path.join(self.t, 'pleitos'), os.path.join(self.t, 'acervo')

    def tearDown(self):
        shutil.rmtree(self.t, ignore_errors=True)

    def pleitos(self, extra=()):
        j = json.load(open(FIXTURE, encoding='utf-8'))
        j['pleitos'] = list(extra) + j['pleitos']
        json.dump(j, open(self.fixo, 'w', encoding='utf-8'))

    def rodar(self, ok='', **env):
        e = dict(os.environ, UV_BIN=self.bin, PLEITOS_DIR=self.pd, ACERVO_DIR=self.ac, ANOT=self.anot,
                 PLEITOS_FIXO=self.fixo, MONTAR_OK=ok, **env)
        r = subprocess.run(['bash', SCRIPT], env=e, capture_output=True, text=True, timeout=60)
        self.assertEqual(r.returncode, 0, r.stderr)
        try:
            return open(self.anot).read().splitlines(), r.stderr
        finally:
            if os.path.exists(self.anot):
                os.remove(self.anot)

    def test_monta_os_que_faltam_do_mais_novo_para_o_mais_antigo(self):
        self.pleitos()
        ch, _ = self.rodar(ok='3240 3239')
        self.assertTrue(ch[0].startswith('uv-coletor pleitos --base https://resultados.tse.jus.br --saida'), ch[0])
        self.assertIn('--acervo', ch[0])
        self.assertEqual([c for c in ch if c.startswith('montar')], ['montar 3240', 'montar 3239'], 'MAX_POR_RODADA=2')
        self.assertIn('--ele-c', ch[-1], 'depois de um acervo novo, o pleitos.json é regravado sem pedir o ele-c.json')
        self.assertTrue(os.path.exists(os.path.join(self.ac, '3240', '.completo')))

    def test_falha_espera_antes_de_tentar_de_novo(self):
        self.pleitos()
        ch, err = self.rodar(ok='3239', MAX_POR_RODADA='2')
        self.assertEqual([c for c in ch if c.startswith('montar')], ['montar 3240', 'montar 3239'])
        self.assertIn('acervo do 3240 falhou', err)
        self.assertTrue(os.path.exists(os.path.join(self.ac, '3240', 'falhou')))
        ch, _ = self.rodar(ok='', MAX_POR_RODADA='1')
        self.assertEqual([c for c in ch if c.startswith('montar')], ['montar 3239'],
                         '3240 espera ESPERA_FALHA_H (o fixo continua dizendo que o 3239 não tem acervo)')

    def test_nada_de_coleta_na_noite_nem_no_dia_seguinte(self):
        ontem = datetime.now(BR).date() - timedelta(days=1)
        self.pleitos([{'codigo': 3250, 'ciclo': 'ele2024', 'data': ontem.strftime('%d/%m/%Y'), 'data_iso': ontem.isoformat(),
                       'situacao': 'encerrado', 'replay': False, 'eleicoes': []}])
        ch, err = self.rodar(ok='3250 3240 3239')
        self.assertEqual([c for c in ch if c.startswith('montar')], [], 'eleição ontem: só o pleitos.json')
        self.assertIn('noite ou dia seguinte de eleição (3250', err)
        self.assertEqual(len(ch), 1)

    def test_em_curso_tambem_segura(self):
        hoje = datetime.now(BR).date()
        self.pleitos([{'codigo': 3251, 'ciclo': 'ele2024', 'data': hoje.strftime('%d/%m/%Y'), 'data_iso': hoje.isoformat(),
                       'situacao': 'em_curso', 'replay': False, 'eleicoes': []}])
        ch, _ = self.rodar(ok='3240')
        self.assertEqual([c for c in ch if c.startswith('montar')], [])

    def test_so_o_pleitos_json_quando_desligado(self):
        self.pleitos()
        ch, _ = self.rodar(ok='3240', MONTAR_ACERVOS='0')
        self.assertEqual(len(ch), 1)
        self.assertTrue(os.path.exists(os.path.join(self.pd, 'pleitos.json')))

    # §3.6: catálogo pré-eleição dos agendados, sozinho, assim que os cs.json saírem
    def agendado(self, dias, codigo=3260):
        d = datetime.now(BR).date() + timedelta(days=dias)
        return {'codigo': codigo, 'ciclo': 'ele2026', 'data': d.strftime('%d/%m/%Y'), 'data_iso': d.isoformat(),
                'situacao': 'agendado', 'replay': False, 'eleicoes': []}

    def test_agendado_ganha_o_catalogo_pre_eleicao(self):
        self.pleitos([self.agendado(10)])
        ch, err = self.rodar(ok='3240 3239')
        prep = [c for c in ch if c.startswith('uv-coletor preparar')]
        self.assertEqual(len(prep), 1, ch)
        self.assertIn('--pleito 3260', prep[0])
        self.assertIn('--ciclo ele2026', prep[0])
        self.assertIn(f'--fontes {self.ac}/3260/fontes-aovivo', prep[0], 'as mesmas fontes do modo aovivo')
        self.assertTrue(os.path.getsize(os.path.join(self.ac, '3260', 'preparo', 'candidatos.json')) > 0)
        self.assertIn('catálogo pré-eleição do 3260 pronto', err)
        ch, _ = self.rodar(ok='')
        self.assertEqual([c for c in ch if c.startswith('uv-coletor preparar')], [], 'pronto: não pede de novo')

    def test_agendado_sem_cs_json_espera(self):
        self.pleitos([self.agendado(10)])
        ch, err = self.rodar(ok='', PREPARAR_RC='1')
        self.assertEqual(len([c for c in ch if c.startswith('uv-coletor preparar')]), 1)
        self.assertIn('ainda não saiu', err)
        self.assertTrue(os.path.exists(os.path.join(self.ac, '3260', 'preparo', 'falhou')))
        ch, _ = self.rodar(ok='')
        self.assertEqual([c for c in ch if c.startswith('uv-coletor preparar')], [], 'espera PREPARO_H')
        ch, _ = self.rodar(ok='', PREPARO_H='0')
        self.assertEqual(len([c for c in ch if c.startswith('uv-coletor preparar')]), 1, 'passado o prazo, tenta de novo')

    def test_agendado_incompleto_tenta_completar_depois(self):
        self.pleitos([self.agendado(10)])
        _, err = self.rodar(ok='', PREPARAR_RC='2')
        self.assertIn('fontes incompletas', err)
        ch, _ = self.rodar(ok='')
        self.assertEqual([c for c in ch if c.startswith('uv-coletor preparar')], [])
        ch, _ = self.rodar(ok='', PREPARO_H='0')
        self.assertEqual(len([c for c in ch if c.startswith('uv-coletor preparar')]), 1)
        ch, _ = self.rodar(ok='', PREPARO_H='0')
        self.assertEqual([c for c in ch if c.startswith('uv-coletor preparar')], [], 'completo: acabou')

    def test_agendado_longe_ou_na_noite_nao(self):
        self.pleitos([self.agendado(90)])
        ch, _ = self.rodar(ok='')
        self.assertEqual([c for c in ch if c.startswith('uv-coletor preparar')], [], 'além de PREPARO_DIAS')
        hoje = datetime.now(BR).date()
        self.pleitos([self.agendado(10), {'codigo': 3251, 'ciclo': 'ele2024', 'data': hoje.strftime('%d/%m/%Y'),
                      'data_iso': hoje.isoformat(), 'situacao': 'em_curso', 'replay': False, 'eleicoes': []}])
        ch, _ = self.rodar(ok='')
        self.assertEqual([c for c in ch if c.startswith('uv-coletor preparar')], [], 'noite de eleição: nada')


if __name__ == '__main__':
    unittest.main()
