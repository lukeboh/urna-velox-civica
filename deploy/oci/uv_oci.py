#!/usr/bin/env python3
"""Operações nos nós da OCI sem SSH (usado por `lib.sh`, função `_oci_op`): `em` (executa um script pelo Run Command),
`para` (envia um arquivo pelo bucket) e `de` (baixa um arquivo pelo bucket).

    uv_oci.py em <instância> <tem_stdin 0|1> <comando>      (o script vem pelo descritor 3 quando tem_stdin = 1)
    uv_oci.py para <instância> <origem local> <destino no nó>
    uv_oci.py de <instância> <origem no nó> <destino local>
    uv_oci.py liberar-portas <origem: CIDR ou "sub-rede"> <porta[,porta…]> <instância>…   (Security List, B-019)

Ambiente: OCI_COMPARTIMENTO, OCI_BUCKET, OCI_NAMESPACE (opcional). Saiu do `lib.sh` (heredoc) na 0.8.15 para ter teste
(`deploy/oci/testes/test_uv_oci.py`, B-004); a lógica é a mesma de antes.
"""
import base64, os, sys, time, uuid

# clientes e parâmetros (definidos por `iniciar`; os testes põem falsos no lugar)
ia_client = os_client = cfg = None
comp = bucket = ns = ""
ESPERA = 1.5
# B-004: consultas a mais quando o Run Command termina "SUCCEEDED" sem o texto da saída (a linha UV_RES)
TENTATIVAS_TEXTO = 10
SEM_SAIDA = b"Run Command sem saida"


def iniciar():
    global ia_client, os_client, comp, bucket, ns, cfg
    import oci
    comp = os.environ["OCI_COMPARTIMENTO"]
    bucket = os.environ["OCI_BUCKET"]
    ns = os.environ.get("OCI_NAMESPACE", "")
    cfg = oci.config.from_file()
    ia_client = oci.compute_instance_agent.ComputeInstanceAgentClient(cfg)
    os_client = oci.object_storage.ObjectStorageClient(cfg)
    if not ns:
        ns = os_client.get_namespace().data


def del_obj(nome):
    try:
        os_client.delete_object(ns, bucket, nome)
    except Exception:
        pass


def detalhes_comando(inst_id, wrapper, timeout_s, tok):
    import oci
    return oci.compute_instance_agent.models.CreateInstanceAgentCommandDetails(
        compartment_id=comp,
        execution_time_out_in_seconds=timeout_s,
        display_name=f"uv-{tok[:20]}",
        target=oci.compute_instance_agent.models.InstanceAgentCommandTarget(instance_id=inst_id),
        content=oci.compute_instance_agent.models.InstanceAgentCommandContent(
            source=oci.compute_instance_agent.models.InstanceAgentCommandSourceViaTextDetails(text=wrapper),
            output=oci.compute_instance_agent.models.InstanceAgentCommandOutputViaTextDetails()))


def ler_resultado(texto):
    """(rc, out, err) da linha UV_RES do texto do Run Command, ou None se ela não está lá."""
    for linha in reversed((texto or "").splitlines()):
        if linha.startswith("UV_RES:"):
            partes = linha.split(":", 4)
            rc = int(partes[1])
            if partes[2] == "INLINE":
                out_b = base64.b64decode(partes[3]) if partes[3] else b""
                err_b = base64.b64decode(partes[4]) if partes[4] else b""
            else:
                o_obj, e_obj = partes[3], partes[4]
                try:
                    out_b = os_client.get_object(ns, bucket, o_obj).data.content
                finally:
                    del_obj(o_obj)
                try:
                    err_b = os_client.get_object(ns, bucket, e_obj).data.content
                finally:
                    del_obj(e_obj)
            return rc, out_b, err_b
    return None


def rodar_remoto(inst_id, script_bytes, timeout_s=1800):
    tok = f"{int(time.time()*1000)}-{os.getpid()}-{uuid.uuid4().hex[:8]}"
    in_obj = None
    try:
        if len(script_bytes) <= 2000:
            b64_in = base64.b64encode(script_bytes).decode("ascii")
            prep = f"echo '{b64_in}' | base64 -d > /tmp/uv-in-{tok}.sh"
        else:
            in_obj = f"in-{tok}.sh"
            os_client.put_object(ns, bucket, in_obj, script_bytes)
            prep = f"sudo /opt/urna-velox/bin/uv-bucket get '{bucket}' '{in_obj}' /tmp/uv-in-{tok}.sh '{ns}'"

        wrapper = f"""#!/usr/bin/env bash
{prep}
chmod 0700 /tmp/uv-in-{tok}.sh
bash /tmp/uv-in-{tok}.sh >/tmp/uv-out-{tok} 2>/tmp/uv-err-{tok}
RC=$?
OUT_SZ=$(wc -c </tmp/uv-out-{tok} 2>/dev/null || echo 0)
ERR_SZ=$(wc -c </tmp/uv-err-{tok} 2>/dev/null || echo 0)
if [ $((OUT_SZ + ERR_SZ)) -le 450 ]; then
  B_OUT=$(base64 -w0 /tmp/uv-out-{tok})
  B_ERR=$(base64 -w0 /tmp/uv-err-{tok})
  printf 'UV_RES:%d:INLINE:%s:%s\\n' "$RC" "$B_OUT" "$B_ERR"
else
  sudo /opt/urna-velox/bin/uv-bucket put '{bucket}' 'out-{tok}' /tmp/uv-out-{tok} '{ns}'
  sudo /opt/urna-velox/bin/uv-bucket put '{bucket}' 'err-{tok}' /tmp/uv-err-{tok} '{ns}'
  printf 'UV_RES:%d:BUCKET:out-%s:err-%s\\n' "$RC" "{tok}" "{tok}"
fi
rm -f /tmp/uv-in-{tok}.sh /tmp/uv-out-{tok} /tmp/uv-err-{tok}
exit 0
"""
        cmd_id = ia_client.create_instance_agent_command(detalhes_comando(inst_id, wrapper, timeout_s, tok)).data.id
        t0 = time.time()
        sem_texto = 0
        while time.time() - t0 < timeout_s + 60:
            ex = ia_client.get_instance_agent_command_execution(cmd_id, inst_id).data
            st = ex.lifecycle_state
            if st in ("SUCCEEDED", "FAILED", "TIMED_OUT", "CANCELED"):
                texto = (ex.content.text or "") if ex.content else ""
                r = ler_resultado(texto)
                if r is not None:
                    return r
                # B-004: o estado final pode chegar antes do texto da saída; consulta de novo antes de desistir
                if st == "SUCCEEDED" and sem_texto < TENTATIVAS_TEXTO:
                    sem_texto += 1
                    time.sleep(ESPERA * 2)
                    continue
                rc = ex.content.exit_code if (ex.content and ex.content.exit_code is not None) else 1
                return (rc if rc != 0 else 1), b"", SEM_SAIDA + f" ({st}): {texto}\n".encode("utf-8")
            time.sleep(ESPERA)
        return 124, b"", b"Run Command: tempo limite esgotado\n"
    finally:
        if in_obj:
            del_obj(in_obj)


def em(inst_id, tem_stdin, cmd_str):
    if tem_stdin:
        with os.fdopen(3, "rb", closefd=False) as fd3:
            stdin_bytes = fd3.read()
        if cmd_str.strip() in ("", "bash -s", "bash"):
            script_bytes = stdin_bytes
        else:
            tok_eof = f"UV_EOF_{uuid.uuid4().hex}"
            script_bytes = cmd_str.encode("utf-8") + f" <<'{tok_eof}'\n".encode("utf-8") + stdin_bytes + f"\n{tok_eof}\n".encode("utf-8")
    else:
        script_bytes = cmd_str.encode("utf-8") + b"\n"
    rc, out_b, err_b = rodar_remoto(inst_id, script_bytes)
    if out_b:
        sys.stdout.buffer.write(out_b)
        sys.stdout.buffer.flush()
    if err_b:
        sys.stderr.buffer.write(err_b)
        sys.stderr.buffer.flush()
    return rc


def para(inst_id, origem, destino):
    tok = f"para-{int(time.time()*1000)}-{os.getpid()}-{uuid.uuid4().hex[:8]}"
    try:
        with open(origem, "rb") as f:
            os_client.put_object(ns, bucket, tok, f)
        cmd = (
            f"sudo /opt/urna-velox/bin/uv-bucket get '{bucket}' '{tok}' '/tmp/{tok}' '{ns}' && "
            f"(mkdir -p \"$(dirname '{destino}')\" 2>/dev/null || sudo mkdir -p \"$(dirname '{destino}')\") && "
            f"(cp '/tmp/{tok}' '{destino}' 2>/dev/null || sudo cp '/tmp/{tok}' '{destino}') && "
            f"sudo rm -f '/tmp/{tok}'"
        )
        rc, out_b, err_b = rodar_remoto(inst_id, cmd.encode("utf-8"))
        if rc != 0 and err_b.startswith(SEM_SAIDA):
            # B-004: o Run Command acabou sem dizer como; o arquivo no nó com o tamanho certo é a resposta
            local = os.path.getsize(origem)
            rc2, out2, _ = rodar_remoto(inst_id, f"stat -c %s '{destino}' 2>/dev/null || sudo stat -c %s '{destino}'\n".encode("utf-8"))
            no_no = out2.decode("utf-8", "replace").strip()
            if rc2 == 0 and no_no == str(local):
                sys.stderr.write(f"aviso: o Run Command não devolveu a saída, mas {destino} chegou inteiro ({local} bytes)\n")
                return 0
            err_b += f"no nó: {no_no or 'sem o arquivo'} bytes; local: {local}\n".encode("utf-8")
        if err_b:
            sys.stderr.buffer.write(err_b)
        return rc
    finally:
        del_obj(tok)


def de(inst_id, origem, destino_local):
    tok = f"de-{int(time.time()*1000)}-{os.getpid()}-{uuid.uuid4().hex[:8]}"
    try:
        cmd = f"sudo /opt/urna-velox/bin/uv-bucket put '{bucket}' '{tok}' '{origem}' '{ns}'"
        rc, out_b, err_b = rodar_remoto(inst_id, cmd.encode("utf-8"))
        if rc != 0:
            if err_b:
                sys.stderr.buffer.write(err_b)
            return rc
        os.makedirs(os.path.dirname(os.path.abspath(destino_local)), exist_ok=True)
        resp = os_client.get_object(ns, bucket, tok)
        with open(destino_local, "wb") as f:
            for chunk in resp.data.raw.stream(1024 * 1024, decode_content=False):
                f.write(chunk)
        return 0
    finally:
        del_obj(tok)


# ---------------------------------------------------------------- Security List (B-019)

def _modelos():
    """Classes do SDK para montar a atualização (os testes põem construtores falsos no lugar)."""
    from oci.core.models import IngressSecurityRule, PortRange, TcpOptions, UpdateSecurityListDetails
    return IngressSecurityRule, TcpOptions, PortRange, UpdateSecurityListDetails


def _cobre(r, fonte, porta):
    """A regra de entrada `r` já deixa passar TCP de `fonte` para `porta`?"""
    import ipaddress
    if str(r.protocol) not in ("6", "all"):
        return False
    try:
        if not ipaddress.ip_network(fonte).subnet_of(ipaddress.ip_network(r.source)):
            return False
    except (ValueError, TypeError):
        return False
    tcp = getattr(r, "tcp_options", None)
    if str(r.protocol) == "all" or tcp is None or tcp.destination_port_range is None:
        return True
    pr = tcp.destination_port_range
    return pr.min <= porta <= pr.max


def liberar_portas(computa, rede, origem, portas, instancias):
    """Abre `portas` (TCP) vindas de `origem` ("sub-rede" = o CIDR da sub-rede da instância) na Security List da
    sub-rede de cada instância. Só acrescenta o que falta; nunca apaga regra. Devolve 0, ou 1 se não achou a lista."""
    In, Tcp, Faixa, Upd = _modelos()
    feitas = set()
    for inst in instancias:
        anexos = [a for a in computa.list_vnic_attachments(comp, instance_id=inst).data if a.lifecycle_state == "ATTACHED"]
        if not anexos:
            print(f"liberar-portas: instância {inst} sem VNIC anexada", file=sys.stderr)
            return 1
        sub = rede.get_subnet(rede.get_vnic(anexos[0].vnic_id).data.subnet_id).data
        if not sub.security_list_ids:
            print(f"liberar-portas: a sub-rede {sub.id} não tem Security List", file=sys.stderr)
            return 1
        fonte = sub.cidr_block if origem == "sub-rede" else origem
        if (sub.id, fonte) in feitas:
            continue
        feitas.add((sub.id, fonte))
        listas = [rede.get_security_list(i).data for i in sub.security_list_ids]
        faltam = [p for p in portas if not any(_cobre(r, fonte, p) for sl in listas for r in sl.ingress_security_rules)]
        if not faltam:
            print(f"liberar-portas: {fonte} → {','.join(map(str, portas))} já liberadas ({sub.id})")
            continue
        # a lista do projeto, se a sub-rede usa ela; senão a primeira
        alvo = next((sl for sl in listas if sl.display_name == "uv-seguranca"), listas[0])
        novas = list(alvo.ingress_security_rules) + [
            In(source=fonte, protocol="6", source_type="CIDR_BLOCK", is_stateless=False,
               description=f"urna-velox: porta {p}", tcp_options=Tcp(destination_port_range=Faixa(min=p, max=p)))
            for p in faltam]
        rede.update_security_list(alvo.id, Upd(ingress_security_rules=novas, egress_security_rules=alvo.egress_security_rules))
        print(f"liberar-portas: {fonte} → {','.join(map(str, faltam))} acrescentadas em {alvo.display_name}")
    return 0


def main(argv):
    iniciar()
    acao = argv[0]
    if acao == "em":
        return em(argv[1], argv[2] == "1", argv[3])
    if acao == "para":
        return para(argv[1], argv[2], argv[3])
    if acao == "de":
        return de(argv[1], argv[2], argv[3])
    if acao == "liberar-portas":
        import oci
        portas = [int(p) for p in argv[2].split(",") if p]
        return liberar_portas(oci.core.ComputeClient(cfg), oci.core.VirtualNetworkClient(cfg), argv[1], portas, argv[3:])
    print(f"ação desconhecida: {acao}", file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
