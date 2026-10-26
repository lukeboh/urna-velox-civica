"""Teste do B-004 (`para` devolve erro com arquivo grande que chegou): o Run Command da OCI termina "SUCCEEDED" mas o
texto da saída vem vazio (sem a linha UV_RES) — com o tar de 3,7 GB de 17/10 o arquivo tinha chegado inteiro ao nó e o
`para` dizia que falhou. Clientes falsos da OCI no lugar dos reais; nada sai da máquina.

    python3 -m unittest discover -s deploy/oci/testes -p 'test_*.py' -v
"""
import base64, os, re, sys, tempfile, types, unittest

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
import uv_oci  # noqa: E402


def ns(**kw):
    return types.SimpleNamespace(**kw)


class IaFalso:
    """Cada comando criado responde conforme `responder(script) -> lista de (estado, texto)` (um item por consulta;
    o último se repete)."""
    def __init__(self, responder):
        self.responder, self.cmds, self.scripts = responder, {}, []

    def create_instance_agent_command(self, wrapper):
        m = re.search(r"echo '([A-Za-z0-9+/=]+)' \| base64 -d", wrapper)
        script = base64.b64decode(m.group(1)).decode() if m else ""
        self.scripts.append(script)
        cid = f"cmd{len(self.cmds)}"
        self.cmds[cid] = list(self.responder(script))
        return ns(data=ns(id=cid))

    def get_instance_agent_command_execution(self, cid, inst):
        fila = self.cmds[cid]
        st, texto = fila.pop(0) if len(fila) > 1 else fila[0]
        return ns(data=ns(lifecycle_state=st, content=ns(text=texto, exit_code=0)))


class OsFalso:
    def put_object(self, *a): pass
    def delete_object(self, *a): pass
    def get_object(self, *a): raise AssertionError("não devia buscar no bucket")


def res(rc, out=b"", err=b""):
    return f"UV_RES:{rc}:INLINE:{base64.b64encode(out).decode()}:{base64.b64encode(err).decode()}"


class Para(unittest.TestCase):
    def setUp(self):
        uv_oci.os_client, uv_oci.ESPERA = OsFalso(), 0
        uv_oci.detalhes_comando = lambda inst, wrapper, t, tok: wrapper
        self.arq = tempfile.NamedTemporaryFile(delete=False)
        self.arq.write(b"x" * 12345)
        self.arq.close()

    def tearDown(self):
        os.unlink(self.arq.name)

    def rodar(self, responder):
        uv_oci.ia_client = IaFalso(responder)
        return uv_oci.para("inst", self.arq.name, "/srv/destino.tar")

    def test_saida_vazia_mas_o_arquivo_chegou(self):
        # a cópia termina "SUCCEEDED" sem texto (o sintoma); conferir o tamanho no nó mostra o arquivo inteiro
        def r(script):
            if "stat -c" in script:
                return [("SUCCEEDED", res(0, b"12345\n"))]
            return [("IN_PROGRESS", ""), ("SUCCEEDED", "")]
        self.assertEqual(self.rodar(r), 0)

    def test_saida_vazia_e_o_arquivo_incompleto(self):
        def r(script):
            if "stat -c" in script:
                return [("SUCCEEDED", res(0, b"999\n"))]
            return [("SUCCEEDED", "")]
        self.assertNotEqual(self.rodar(r), 0)

    def test_saida_normal(self):
        self.assertEqual(self.rodar(lambda s: [("SUCCEEDED", res(0))]), 0)

    def test_erro_de_verdade(self):
        self.assertEqual(self.rodar(lambda s: [("SUCCEEDED", res(3, b"", b"sem espaco\n"))]), 3)


class SaidaAtrasada(unittest.TestCase):
    def test_texto_chega_depois_do_estado(self):
        # o estado vira SUCCEEDED antes de o texto da saída aparecer: consultar de novo, não falhar
        uv_oci.os_client, uv_oci.ESPERA = OsFalso(), 0
        uv_oci.detalhes_comando = lambda inst, wrapper, t, tok: wrapper
        uv_oci.ia_client = IaFalso(lambda s: [("SUCCEEDED", ""), ("SUCCEEDED", ""), ("SUCCEEDED", res(0, b"ok\n"))])
        rc, out, err = uv_oci.rodar_remoto("inst", b"echo ok\n")
        self.assertEqual((rc, out), (0, b"ok\n"))


class LimiteDoTexto(unittest.TestCase):
    """B-063: o Run Command da OCI recusa texto com mais de 4.096 caracteres (`content.source.text size must be between
    1 and 4096`). O script em linha vai em base64 (4/3) dentro do wrapper: com o limite em bytes do script (3.500) o
    30_verificar.sh (~2.500 bytes) passava disso e o comando nem era criado."""
    def test_comando_cabe_no_limite_da_oci(self):
        textos, objetos = [], []

        class Ia(IaFalso):
            def create_instance_agent_command(self, wrapper):
                textos.append(wrapper)
                return super().create_instance_agent_command(wrapper)

        class Os(OsFalso):
            def put_object(self, ns_, bucket, nome, dados): objetos.append(nome)

        uv_oci.os_client, uv_oci.ESPERA = Os(), 0
        uv_oci.ns, uv_oci.bucket = "ns", "balde"
        uv_oci.detalhes_comando = lambda inst, wrapper, t, tok: wrapper
        uv_oci.ia_client = Ia(lambda s: [("SUCCEEDED", res(0, b"ok\n"))])
        for n in (100, 2000, 2600, 3000, 3500, 6000):
            antes = len(objetos)
            rc, out, _ = uv_oci.rodar_remoto("inst", b"#" * (n - 1) + b"\n")
            self.assertEqual((rc, out), (0, b"ok\n"), n)
            self.assertLessEqual(len(textos[-1]), 4096, f"script de {n} bytes: comando com {len(textos[-1])} caracteres")
            if n == 100:
                self.assertEqual(len(objetos), antes, "script pequeno vai em linha, sem bucket")
            if n == 6000:
                self.assertEqual(len(objetos), antes + 1, "script grande vai pelo bucket")


if __name__ == "__main__":
    unittest.main()
