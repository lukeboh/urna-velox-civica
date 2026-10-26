"""Teste do B-019 (o painel do nó A não alcança o do nó B: "error sending request for url (http://10.0.1.11:8095/...)").
A porta 8095 tem de estar aberta para a sub-rede na Security List da VCN; o 02_configurar_borda.sh procurava a lista
pelo nome (`uv-seguranca`) com a OCI CLI e, quando não achava, seguia em silêncio. Agora `uv_oci.py liberar-portas`
chega à lista pelo caminho que a instância usa de fato (VNIC → sub-rede → Security Lists), só acrescenta as regras que
faltam (não apaga nenhuma) e falha alto quando não acha a lista. Clientes falsos da OCI; nada sai da máquina.

    python3 -m unittest discover -s deploy/oci/testes -p 'test_*.py' -v
"""
import os, sys, types, unittest

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
import uv_oci  # noqa: E402


def ns(**kw):
    return types.SimpleNamespace(**kw)


def regra(fonte, porta=None, proto="6"):
    tcp = ns(destination_port_range=ns(min=porta, max=porta), source_port_range=None) if porta else None
    return ns(source=fonte, protocol=proto, tcp_options=tcp, udp_options=None, icmp_options=None, is_stateless=False,
              source_type="CIDR_BLOCK", description=None)


class Computa:
    def __init__(self, vnics): self.vnics = vnics
    def list_vnic_attachments(self, compartment_id, instance_id=None):
        return ns(data=[ns(vnic_id=v, lifecycle_state="ATTACHED") for v in self.vnics.get(instance_id, [])])


class Rede:
    def __init__(self, listas, sub_listas):
        self.listas, self.sub_listas, self.atualizacoes = listas, sub_listas, []
    def get_vnic(self, vnic_id): return ns(data=ns(subnet_id="sub1"))
    def get_subnet(self, sid): return ns(data=ns(id=sid, cidr_block="10.0.1.0/24", security_list_ids=self.sub_listas))
    def get_security_list(self, sl): return ns(data=ns(id=sl, display_name=sl, ingress_security_rules=list(self.listas[sl]),
                                                        egress_security_rules=[ns(destination="0.0.0.0/0", protocol="all")]))
    def update_security_list(self, sl, detalhes):
        self.atualizacoes.append((sl, detalhes))
        self.listas[sl] = list(detalhes.ingress_security_rules)


class LiberarPortas(unittest.TestCase):
    def setUp(self):
        uv_oci.comp = "comp"
        uv_oci._modelos = lambda: (lambda **k: ns(**k), lambda **k: ns(**k), lambda **k: ns(**k), lambda **k: ns(**k))

    def rodar(self, listas, sub_listas, *args):
        rede = Rede(listas, sub_listas)
        rc = uv_oci.liberar_portas(Computa({"A": ["v1"], "B": ["v2"]}), rede, *args)
        return rc, rede

    def portas(self, rede, sl, fonte):
        return sorted(r.tcp_options.destination_port_range.min for r in rede.listas[sl]
                      if r.source == fonte and r.tcp_options)

    def test_acrescenta_so_o_que_falta_sem_apagar(self):
        existentes = [regra("0.0.0.0/0", 22), regra("10.0.1.0/24", 8080), regra("10.0.1.0/24", 7700)]
        rc, rede = self.rodar({"sl1": existentes}, ["sl1"], "sub-rede", [8080, 8095, 7700, 7701], ["A", "B"])
        self.assertEqual(rc, 0)
        self.assertEqual(self.portas(rede, "sl1", "10.0.1.0/24"), [7700, 7701, 8080, 8095])
        self.assertIn(22, self.portas(rede, "sl1", "0.0.0.0/0"))      # nada foi apagado
        self.assertEqual(len(rede.atualizacoes), 1)                    # A e B na mesma sub-rede: uma atualização só

    def test_idempotente(self):
        lista = [regra("10.0.1.0/24", p) for p in (8080, 8095)]
        rc, rede = self.rodar({"sl1": lista}, ["sl1"], "sub-rede", [8080, 8095], ["A", "B"])
        self.assertEqual(rc, 0)
        self.assertEqual(rede.atualizacoes, [])

    def test_regra_ampla_ja_cobre(self):
        # "todo protocolo" da VCN inteira já cobre a 8095 da sub-rede
        rc, rede = self.rodar({"sl1": [regra("10.0.0.0/16", proto="all")]}, ["sl1"], "sub-rede", [8095], ["A"])
        self.assertEqual((rc, rede.atualizacoes), (0, []))

    def test_porta_publica(self):
        rc, rede = self.rodar({"sl1": [regra("0.0.0.0/0", 443)]}, ["sl1"], "0.0.0.0/0", [80, 443], ["A"])
        self.assertEqual(rc, 0)
        self.assertEqual(self.portas(rede, "sl1", "0.0.0.0/0"), [80, 443])

    def test_sem_security_list_falha_alto(self):
        rc, _ = self.rodar({}, [], "sub-rede", [8095], ["A"])
        self.assertNotEqual(rc, 0)

    # B-025: uma regra pública (0.0.0.0/0 → 8080, criada à mão) "cobria" a da sub-rede, que nunca foi criada; tirar a
    # pública derrubaria o balanceamento entre os nós. A da sub-rede tem de existir por conta própria.
    def test_b025_sub_rede_nao_conta_com_regra_publica(self):
        rc, rede = self.rodar({"sl1": [regra("0.0.0.0/0", 8080), regra("10.0.1.0/24", 8095)]}, ["sl1"],
                              "sub-rede", [8080, 8095], ["A", "B"])
        self.assertEqual(rc, 0)
        self.assertEqual(self.portas(rede, "sl1", "10.0.1.0/24"), [8080, 8095])


class FecharPortas(unittest.TestCase):
    """B-025: `uv_oci.py fechar-portas 0.0.0.0/0 8080 …` tira da Security List as regras públicas dessas portas (e só
    elas); uma regra ampla (todo protocolo, faixa de portas) não é retalhada: falha alto para o operador decidir."""
    def setUp(self):
        uv_oci.comp = "comp"
        uv_oci._modelos = lambda: (lambda **k: ns(**k), lambda **k: ns(**k), lambda **k: ns(**k), lambda **k: ns(**k))

    def rodar(self, regras, *args):
        rede = Rede({"sl1": regras}, ["sl1"])
        rc = uv_oci.fechar_portas(Computa({"A": ["v1"], "B": ["v2"]}), rede, *args)
        return rc, rede

    def test_b025_tira_so_a_publica(self):
        regras = [regra("0.0.0.0/0", 22), regra("0.0.0.0/0", 8080), regra("10.0.1.0/24", 8080), regra("0.0.0.0/0", 443)]
        rc, rede = self.rodar(regras, "0.0.0.0/0", [8080, 8095], ["A", "B"])
        self.assertEqual(rc, 0)
        fica = sorted((r.source, r.tcp_options.destination_port_range.min) for r in rede.listas["sl1"])
        self.assertEqual(fica, [("0.0.0.0/0", 22), ("0.0.0.0/0", 443), ("10.0.1.0/24", 8080)])
        self.assertEqual(len(rede.atualizacoes), 1)

    def test_b025_idempotente(self):
        rc, rede = self.rodar([regra("10.0.1.0/24", 8080)], "0.0.0.0/0", [8080], ["A", "B"])
        self.assertEqual((rc, rede.atualizacoes), (0, []))

    def test_b025_regra_ampla_falha_alto(self):
        rc, rede = self.rodar([regra("0.0.0.0/0", proto="all")], "0.0.0.0/0", [8080], ["A"])
        self.assertNotEqual(rc, 0)
        self.assertEqual(rede.atualizacoes, [])

    def test_b025_borda_libera_a_sub_rede_e_fecha_a_internet(self):
        s = open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "02_configurar_borda.sh"), encoding="utf-8").read()
        self.assertIn("fechar-portas 0.0.0.0/0", s)
        i, j = s.index("sl sub-rede"), s.index("fechar-portas 0.0.0.0/0")
        self.assertLess(i, j, "a regra da sub-rede tem de vir antes de fechar a pública")


if __name__ == "__main__":
    unittest.main()
