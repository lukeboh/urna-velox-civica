"""M-005: o Caddyfile de cada nó (servidor/gerar_caddyfile.sh sobre o Caddyfile.template).

No nó A, o endereço antigo (urnavelox.duckdns.org) responde 301 (Moved Permanently) para o mesmo caminho em
https://urnavelox.com.br — inclusive /robots.txt e /sitemap.xml; no B o bloco não entra. Com o `caddy` no PATH (ou em
UV_CADDY), o arquivo gerado também passa pelo `caddy adapt` (e o JSON confere o 301 e o Location).

    python3 -m unittest discover -s deploy/oci/testes -p 'test_*.py'
"""
import json
import os
import shutil
import subprocess
import unittest

AQUI = os.path.dirname(os.path.abspath(__file__))
SERV = os.path.join(os.path.dirname(AQUI), 'servidor')
GERAR = os.path.join(SERV, 'gerar_caddyfile.sh')
TEMPLATE = os.path.join(SERV, 'Caddyfile.template')
CADDY = os.environ.get('UV_CADDY') or shutil.which('caddy')


def gerar(*args):
    r = subprocess.run(['bash', GERAR, TEMPLATE, *args], capture_output=True, text=True)
    return r.returncode, r.stdout, r.stderr


def rotas_do_host(cfg, host):
    for srv in cfg['apps']['http']['servers'].values():
        for rota in srv.get('routes', []):
            for m in rota.get('match', []):
                if host in m.get('host', []):
                    yield rota


def achar(o, chave):
    if isinstance(o, dict):
        for k, v in o.items():
            if k == chave:
                yield v
            yield from achar(v, chave)
    elif isinstance(o, list):
        for v in o:
            yield from achar(v, chave)


class TestCaddyfile(unittest.TestCase):
    def test_no_a_redireciona_o_endereco_antigo(self):
        rc, out, err = gerar('urnavelox.com.br', '10.0.1.11', 'urnavelox.duckdns.org')
        self.assertEqual(rc, 0, err)
        self.assertIn('urnavelox.duckdns.org {', out)
        self.assertIn('redir https://urnavelox.com.br{uri} permanent', out)
        self.assertIn('urnavelox.com.br, www.urnavelox.com.br {', out)
        self.assertIn('10.0.1.11:8080', out)
        for marca in ('__DOMINIO_ANTIGO__', '{$DOMINIO:', '{$PAR_IP:'):   # (o comentário do topo cita {$PAR_IP})
            self.assertNotIn(marca, out)

    def test_no_b_sem_o_bloco(self):
        rc, out, err = gerar('urnavelox.com.br', '10.0.1.10')
        self.assertEqual(rc, 0, err)
        self.assertNotIn('duckdns', out)
        self.assertNotIn('redir ', out)
        self.assertNotIn('__DOMINIO_ANTIGO__', out)
        self.assertIn('urnavelox.com.br, www.urnavelox.com.br {', out)

    def test_nomes_invalidos_recusados(self):
        self.assertNotEqual(gerar('urnavelox.com.br', '10.0.1.11', 'x/y;rm')[0], 0)
        self.assertNotEqual(gerar('urnavelox.com.br', '10.0.1.11', 'urnavelox.com.br')[0], 0)
        self.assertNotEqual(gerar('a b', '10.0.1.11')[0], 0)

    # B-024: o uv-playlist só roda no A, mas o domínio também leva visitantes ao B: o Caddy do B tem de levar a
    # playlist ao A (antes: 127.0.0.1:8090 no B, que não existe → 503, e a página ficava sem a contagem e sem o "▶")
    def test_b024_no_b_leva_a_playlist_ao_a(self):
        rc, out, err = gerar('urnavelox.com.br', '10.0.1.10', '', 'remoto:203.0.113.10')
        self.assertEqual(rc, 0, err)
        trecho = out[out.index('@playlist path'):out.index('# 2. Painel')]
        self.assertNotIn('127.0.0.1:8090', trecho)
        self.assertIn('reverse_proxy https://203.0.113.10', trecho)
        self.assertIn('tls_server_name urnavelox.com.br', trecho)

    def test_b024_no_a_confia_no_b_para_o_ip_do_visitante(self):
        rc, out, err = gerar('urnavelox.com.br', '10.0.1.11', 'urnavelox.duckdns.org', 'local:198.51.100.20,10.0.1.11')
        self.assertEqual(rc, 0, err)
        self.assertIn('trusted_proxies static 198.51.100.20 10.0.1.11', out)
        trecho = out[out.index('@playlist path'):out.index('# 2. Painel')]
        self.assertIn('reverse_proxy 127.0.0.1:8090', trecho)
        self.assertIn('header_up X-Real-IP {client_ip}', trecho)

    def test_b024_roteiros_passam_o_papel_da_playlist(self):
        for r in ('02_configurar_borda.sh', '63_ciclo.sh'):
            s = open(os.path.join(os.path.dirname(AQUI), r), encoding='utf-8').read()
            self.assertIn('remoto:', s, r)
            self.assertIn('local:', s, r)

    @unittest.skipUnless(CADDY, 'sem o caddy (defina UV_CADDY)')
    def test_b024_caddy_aceita_os_dois_papeis(self):
        import tempfile
        for args, upstream in ((('urnavelox.com.br', '10.0.1.10', '', 'remoto:203.0.113.10'), '203.0.113.10:443'),
                               (('urnavelox.com.br', '10.0.1.11', 'urnavelox.duckdns.org', 'local:198.51.100.20,10.0.1.11'), '127.0.0.1:8090')):
            rc, out, err = gerar(*args)
            self.assertEqual(rc, 0, err)
            with tempfile.NamedTemporaryFile('w', suffix='.Caddyfile', delete=False) as f:
                f.write(out)
            try:
                r = subprocess.run([CADDY, 'adapt', '--config', f.name, '--adapter', 'caddyfile'], capture_output=True, text=True)
                self.assertEqual(r.returncode, 0, r.stderr)
                self.assertIn(upstream, [d for d in achar(json.loads(r.stdout), 'dial')])
            finally:
                os.unlink(f.name)

    @unittest.skipUnless(CADDY, 'sem o caddy (defina UV_CADDY)')
    def test_caddy_aceita_e_o_redirecionamento_e_301(self):
        import tempfile
        for args in (('urnavelox.com.br', '10.0.1.11', 'urnavelox.duckdns.org'), ('urnavelox.com.br', '10.0.1.10')):
            rc, out, err = gerar(*args)
            self.assertEqual(rc, 0, err)
            with tempfile.NamedTemporaryFile('w', suffix='.Caddyfile', delete=False) as f:
                f.write(out)
            try:
                r = subprocess.run([CADDY, 'adapt', '--config', f.name, '--adapter', 'caddyfile'], capture_output=True, text=True)
                self.assertEqual(r.returncode, 0, r.stderr)
                cfg = json.loads(r.stdout)
            finally:
                os.unlink(f.name)
            rotas = list(rotas_do_host(cfg, 'urnavelox.duckdns.org'))
            if len(args) == 2:
                self.assertEqual(rotas, [], 'o nó B não pode ter o endereço antigo')
                continue
            self.assertTrue(rotas, 'sem rota para o endereço antigo')
            status = [str(s) for s in achar(rotas, 'status_code')]
            locais = [l for h in achar(rotas, 'headers') if isinstance(h, dict) for l in h.get('Location', [])]
            self.assertIn('301', status)
            self.assertIn('https://urnavelox.com.br{http.request.uri}', locais)


class TestDados(unittest.TestCase):
    """Passo 3 do plano de borda (docs/handoffs/HANDOFF_MODELO_BORDA_E_CONSISTENCIA.md): o subdomínio `dados`, atrás da
    Cloudflare (cache de 30 s), só com a leitura do resultado e do estado — o modo HTTP da página (B-026). O site
    principal fica idêntico; sem o 6º argumento (DOMINIO_DADOS vazio no nos.env), o bloco não entra (o Caddy não tenta
    o certificado de um nome que ainda não está no DNS)."""
    B = ('urnavelox.com.br', '10.0.1.10', '', 'remoto:203.0.113.10')
    DADOS = 'dados.urnavelox.com.br'

    def acrescimo(self, args, dados):
        import difflib
        rc, sem, err = gerar(*args)
        self.assertEqual(rc, 0, err)
        rc, com, err = gerar(*args, dados)
        self.assertEqual(rc, 0, err)
        dif = list(difflib.ndiff(sem.splitlines(), com.splitlines()))
        tirado = [l for l in dif if l.startswith('- ')]
        self.assertEqual(tirado, [], 'o site principal mudou com o dados')
        return com, '\n'.join(l[2:] for l in dif if l.startswith('+ '))

    def test_dados_serve_a_consulta_com_o_cache_do_servico(self):
        """Multipleito (1.1): as séries de consulta (`/consulta/*`) também pelo `dados`, com o Cache-Control do próprio
        uv-servico (5 min no índice, 1 h no resto), sem a troca para 30 s das rotas ao vivo."""
        _, novo = self.acrescimo(self.B, self.DADOS)
        self.assertIn('path /consulta/*', novo)
        i = novo.index('path /consulta/*')
        bloco = novo[i:novo.index('}\n    }', novo.index('handle @consulta', i)) + 6]
        self.assertIn('reverse_proxy 127.0.0.1:8080 10.0.1.10:8080', bloco)
        self.assertNotIn('header_down Cache-Control', bloco)

    def test_sem_dados_nada_muda(self):
        rc, out, err = gerar(*self.B)
        self.assertEqual(rc, 0, err)
        self.assertNotIn('dados.', out)
        self.assertNotIn('__DOMINIO_DADOS__', out)

    def test_dados_so_le_resultado_e_estado_com_cache_de_30s(self):
        # 0.8.48 tinha passado para 120-150 s; de volta a 30 s em 24/10/2026 (Luciano)
        for args in (self.B, ('urnavelox.com.br', '10.0.1.11', 'urnavelox.duckdns.org', 'local:198.51.100.20,10.0.1.11')):
            _, novo = self.acrescimo(args, self.DADOS)
            self.assertIn(f'{self.DADOS} {{', novo)
            self.assertIn('method GET HEAD', novo)
            self.assertIn('path /api/v1/resultado/* /api/v1/estado', novo)
            self.assertIn('header_down Cache-Control "public, max-age=30, s-maxage=30"', novo)
            self.assertIn(f'reverse_proxy 127.0.0.1:8080 {args[1]}:8080', novo)
            self.assertIn('respond `{"erro":"nao_encontrado"}` 404', novo)
            self.assertIn('Cache-Control "no-store"', novo)   # erro do serviço não fica no cache da borda
            for fora in ('/ws', 'playlist', '/painel', 'forward_auth', '__DOMINIO_DADOS__'):
                self.assertNotIn(fora, novo)

    def test_dados_nomes_invalidos_recusados(self):
        self.assertNotEqual(gerar(*self.B, 'x;rm -rf')[0], 0)
        self.assertNotEqual(gerar(*self.B, 'urnavelox.com.br')[0], 0)   # o dados não pode ser o domínio principal

    def test_roteiros_passam_o_dominio_dados(self):
        for r in ('02_configurar_borda.sh', '63_ciclo.sh', 'nos.env.exemplo'):
            s = open(os.path.join(os.path.dirname(AQUI), r), encoding='utf-8').read()
            self.assertIn('DOMINIO_DADOS', s, r)
        s = open(os.path.join(SERV, 'instalar_caddy.sh'), encoding='utf-8').read()
        self.assertIn('"$DOMINIO_DADOS"', s)

    @unittest.skipUnless(CADDY, 'sem o caddy (defina UV_CADDY)')
    def test_caddy_aceita_o_dados(self):
        import tempfile
        rc, out, err = gerar(*self.B, self.DADOS)
        self.assertEqual(rc, 0, err)
        with tempfile.NamedTemporaryFile('w', suffix='.Caddyfile', delete=False) as f:
            f.write(out)
        try:
            r = subprocess.run([CADDY, 'adapt', '--config', f.name, '--adapter', 'caddyfile'], capture_output=True, text=True)
            self.assertEqual(r.returncode, 0, r.stderr)
            cfg = json.loads(r.stdout)
        finally:
            os.unlink(f.name)
        rotas = list(rotas_do_host(cfg, self.DADOS))
        self.assertTrue(rotas, 'sem rota para o dados')
        self.assertEqual(sorted(set(d for d in achar(rotas, 'dial'))), ['10.0.1.10:8080', '127.0.0.1:8080'])
        self.assertIn(['GET', 'HEAD'], list(achar(rotas, 'method')))
        # o site principal continua com o WebSocket
        self.assertIn('/ws', [p for m in achar(list(rotas_do_host(cfg, 'urnavelox.com.br')), 'path') for p in m])


if __name__ == '__main__':
    unittest.main()
