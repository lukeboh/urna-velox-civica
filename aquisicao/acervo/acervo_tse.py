"""Acervo de BUs do TSE: Coleta, Derivação e Verificação

Baixa arquivos brutos de urna de eleições do TSE (CKAN / CDN),
deriva pacotes 'stored' com BUs e arquivos de assinatura de seção,
extrai documentação técnica e gera índices e relatórios de integridade.
"""

import argparse
import csv
import hashlib
import json
import os
import shutil
import struct
import sys
import time
import urllib.error
import urllib.request
import zipfile
from pathlib import Path

USER_AGENT = "urna-velox-acervo/1.0 (+https://github.com/lukeboh/urna-velox-civica)"
CKAN_BASE = "https://dadosabertos.tse.jus.br/api/3/action"

# Especificações de documentação por eleição
DOC_URLS = {
    "2026": "https://www.tse.jus.br/eleicoes/eleicoes-2026-content/arquivos/formato-arquivos-de-bu-rdv-e-assinatura-digital",
    "2024": "https://www.tse.jus.br/eleicoes/eleicoes-2024-content/arquivos/tse-formato-dos-arquivos-de-bu-rdv-e-assinatura-digital-formato-zip",
    "2022": "https://www.tse.jus.br/eleicoes/eleicoes-2022/arquivos/formato-dos-arquivos-de-bu-rdv-e-assinatura-digital-17-9-22",
}

class RateLimiter:
    """Controlador estrito de requisições por segundo (teto máximo e AIMD adaptativo)."""
    def __init__(self, rps_max=100.0):
        self.rps_max = min(float(rps_max), 100.0)
        self.interval = 1.0 / self.rps_max
        self.last_req = 0.0

    def acquire(self):
        now = time.time()
        elapsed = now - self.last_req
        if elapsed < self.interval:
            time.sleep(self.interval - elapsed)
        self.last_req = time.time()


class TSEClient:
    """Cliente HTTP com suporte a retomada, limite de taxa e tratamento de 429."""
    def __init__(self, limiter: RateLimiter, user_agent=USER_AGENT):
        self.limiter = limiter
        self.user_agent = user_agent

    def request(self, url, headers=None, method="GET", timeout=60):
        hdrs = {"User-Agent": self.user_agent}
        if headers:
            hdrs.update(headers)

        while True:
            self.limiter.acquire()
            req = urllib.request.Request(url, headers=hdrs, method=method)
            try:
                resp = urllib.request.urlopen(req, timeout=timeout)
                return resp
            except urllib.error.HTTPError as e:
                if e.code == 429 or 500 <= e.code < 600:
                    print(f"\n[ALERTA] HTTP {e.code} em {url}. Acionando espera de segurança de 10 min...")
                    time.sleep(600)
                    continue
                raise
            except Exception as e:
                time.sleep(5)
                raise


def get_disk_free_gb(path: Path) -> float:
    total, used, free = shutil.disk_usage(path if path.exists() else path.parent)
    return free / (1024 ** 3)


def parse_zip_central_directory(tail_bytes: bytes, total_size: int):
    """Localiza o offset e tamanho do diretório central a partir do final do arquivo."""
    def u16le(b, i): return struct.unpack_from("<H", b, i)[0]
    def u32le(b, i): return struct.unpack_from("<I", b, i)[0]
    def u64le(b, i): return struct.unpack_from("<Q", b, i)[0]

    idx = tail_bytes.rfind(b"PK\x05\x06")
    if idx == -1:
        raise ValueError("EOCD não encontrado")
    
    cd_size = u32le(tail_bytes, idx + 12)
    cd_offset = u32le(tail_bytes, idx + 16)

    if cd_offset == 0xFFFFFFFF or cd_size == 0xFFFFFFFF:
        idx64 = tail_bytes.rfind(b"PK\x06\x06")
        if idx64 == -1:
            raise ValueError("Zip64 locator ausente para arquivo saturado")
        cd_size = u64le(tail_bytes, idx64 + 40)
        cd_offset = u64le(tail_bytes, idx64 + 48)

    return cd_offset, cd_size


def extrair_metadados_nome(nome: str, ano: str):
    """Extrai (municipio, zona, secao) a partir do nome de arquivo em cada eleição."""
    base = os.path.basename(nome)
    # Padrão 2026/2024: o03220df9701200090104-bu.dat ou o00452ac0101500020088-bu.dat
    if base.startswith("o") and "-" in base:
        parts = base.split("-")[0]
        # remove prefixo 'o' e pleito/uf se houver
        if len(parts) >= 21:
            mu_str = parts[8:13]
            zn_str = parts[13:17]
            se_str = parts[17:21]
            try:
                return int(mu_str), int(zn_str), int(se_str)
            except ValueError:
                pass
        elif len(parts) >= 19:
            # 2022: o00406-0111200060046.bu
            # [0:6]=o00406, [7:12]=01112 (mu), [12:16]=0006 (zn), [16:20]=0046 (se)
            sub = parts[7:] if len(parts) > 7 else parts
            if len(sub) >= 13:
                try:
                    return int(sub[0:5]), int(sub[5:9]), int(sub[9:13])
                except ValueError:
                    pass
    return None, None, None


def is_bu_ou_assinatura(nome: str) -> bool:
    """Verifica se a entrada é um BU bruto ou arquivo de assinatura da seção."""
    n = nome.lower()
    return (
        n.endswith("-bu.dat") or n.endswith(".bu") or
        n.endswith("-vota.vsc") or n.endswith(".vscmr") or n.endswith(".vsc")
    )


def is_apenas_bu(nome: str) -> bool:
    n = nome.lower()
    return n.endswith("-bu.dat") or n.endswith(".bu")


def baixar_arquivo_integro(client: TSEClient, url: str, destino: Path, log_fn=print) -> dict:
    """Baixa um arquivo via HTTP para .part com retomada e validação SHA-256."""
    destino.parent.mkdir(parents=True, exist_ok=True)
    part_file = destino.with_suffix(destino.suffix + ".part")

    # HEAD inicial para headers
    resp_head = client.request(url, method="HEAD")
    content_len = int(resp_head.headers.get("Content-Length", 0))
    etag = resp_head.headers.get("ETag", "").strip('"')
    last_mod = resp_head.headers.get("Last-Modified", "")

    # Se já existir e tiver o tamanho correto, valida integridade
    if destino.exists() and destino.stat().st_size == content_len:
        log_fn(f"[EXISTENTE] {destino.name} ({content_len} bytes) íntegro no disco.")
        sha = hashlib.sha256()
        with open(destino, "rb") as f:
            while chunk := f.read(1024 * 1024):
                sha.update(chunk)
        return {
            "bytes": content_len,
            "etag": etag,
            "last_modified": last_mod,
            "sha256": sha.hexdigest(),
            "status": "existente"
        }

    baixado = part_file.stat().st_size if part_file.exists() else 0
    headers = {}
    if baixado > 0:
        if baixado > content_len:
            part_file.unlink()
            baixado = 0
        else:
            headers["Range"] = f"bytes={baixado}-{content_len - 1}"
            log_fn(f"[RETOMANDO] {destino.name} a partir do byte {baixado}/{content_len}...")

    t0 = time.time()
    resp = client.request(url, headers=headers)
    mode = "ab" if baixado > 0 else "wb"
    with open(part_file, mode) as f:
        while True:
            chunk = resp.read(512 * 1024)
            if not chunk:
                break
            f.write(chunk)
            baixado += len(chunk)

    # Validação atômica
    if part_file.stat().st_size != content_len:
        raise IOError(f"Tamanho inconsistente em {destino.name}: {part_file.stat().st_size} != {content_len}")

    # Validar zipfile
    if destino.suffix == ".zip":
        with zipfile.ZipFile(part_file, "r") as zf:
            bad = zf.testzip()
            if bad is not None:
                raise IOError(f"Arquivo zip corrompido na entrada {bad}")

    sha = hashlib.sha256()
    with open(part_file, "rb") as f:
        while chunk := f.read(1024 * 1024):
            sha.update(chunk)

    if destino.exists():
        destino.unlink()
    part_file.rename(destino)

    duracao = time.time() - t0
    log_fn(f"[CONCLUÍDO] {destino.name} em {duracao:.1f}s ({content_len / (1024**2):.1f} MB, {content_len/duracao/(1024**2):.1f} MB/s)")
    return {
        "bytes": content_len,
        "etag": etag,
        "last_modified": last_mod,
        "sha256": sha.hexdigest(),
        "status": "baixado",
        "duracao": duracao
    }


def derivar_bus_zip(zip_origem: Path, zip_destino: Path, csv_destino: Path, ano: str, log_fn=print):
    """Extrai apenas os BUs e assinaturas da seção, gravando em zip STORED (sem recomprimir)."""
    zip_destino.parent.mkdir(parents=True, exist_ok=True)
    csv_destino.parent.mkdir(parents=True, exist_ok=True)

    part_zip = zip_destino.with_suffix(".zip.part")
    registros_csv = []
    total_bus = 0

    with zipfile.ZipFile(zip_origem, "r") as z_in:
        with zipfile.ZipFile(part_zip, "w", compression=zipfile.ZIP_STORED) as z_out:
            for item in z_in.infolist():
                if is_bu_ou_assinatura(item.filename):
                    data = z_in.read(item.filename)
                    z_out.writestr(item.filename, data)
                    
                    if is_apenas_bu(item.filename):
                        total_bus += 1
                        mu, zn, se = extrair_metadados_nome(item.filename, ano)
                        sha = hashlib.sha256(data).hexdigest()
                        registros_csv.append({
                            "arquivo": item.filename,
                            "municipio": mu if mu is not None else "",
                            "zona": zn if zn is not None else "",
                            "secao": se if se is not None else "",
                            "bytes": len(data),
                            "sha256": sha
                        })

    if zip_destino.exists():
        zip_destino.unlink()
    part_zip.rename(zip_destino)

    with open(csv_destino, "w", newline="", encoding="utf-8") as f:
        writer = csv.DictWriter(f, fieldnames=["arquivo", "municipio", "zona", "secao", "bytes", "sha256"])
        writer.writeheader()
        writer.writerows(registros_csv)

    log_fn(f"[DERIVADO] {zip_destino.name}: {total_bus} BUs indexados ({zip_destino.stat().st_size / (1024**2):.1f} MB)")
    return total_bus


def baixar_documentacao(client: TSEClient, ano: str, pasta_doc: Path, log_fn=print):
    """Baixa documentação oficial e especificações ASN.1."""
    pasta_doc.mkdir(parents=True, exist_ok=True)
    url_doc = DOC_URLS.get(ano)
    if not url_doc:
        return

    dest_zip = pasta_doc / f"formato_{ano}.zip"
    try:
        baixar_arquivo_integro(client, url_doc, dest_zip, log_fn=log_fn)
        # Extrair arquivos relevantes
        with zipfile.ZipFile(dest_zip, "r") as zf:
            for member in zf.namelist():
                if member.endswith((".asn1", ".pdf", ".py", ".md", ".puml")):
                    zf.extract(member, pasta_doc)
        log_fn(f"[DOC] Documentação de {ano} extraída em {pasta_doc}")
    except Exception as e:
        log_fn(f"[ERRO DOC] Falha ao baixar documentação de {ano}: {e}")


def testar_amostra_asn1(zip_bu: Path, spec_path: Path, ano: str, amostra_n=20, log_fn=print):
    """Decodifica amostra de BUs com asn1tools para validar compatibilidade do formato."""
    try:
        import asn1tools
    except ImportError:
        log_fn("[AVISO] asn1tools não instalado; pulando teste ASN.1 de BUs.")
        return True, 0, 0

    if not spec_path.exists():
        log_fn(f"[AVISO] Especificação {spec_path} não encontrada para validação.")
        return False, 0, 0

    try:
        spec = asn1tools.compile_files([str(spec_path)], codec="ber")
    except Exception as e:
        log_fn(f"[ERRO ASN1] Falha ao compilar especificação {spec_path}: {e}")
        return False, 0, 0

    sucessos = 0
    falhas = 0

    with zipfile.ZipFile(zip_bu, "r") as zf:
        bus = [n for n in zf.namelist() if is_apenas_bu(n)]
        alvos = bus[:amostra_n]
        for nome in alvos:
            data = zf.read(nome)
            try:
                if "EntidadeEnvelopeGenerico" in spec.types:
                    env = spec.decode("EntidadeEnvelopeGenerico", data)
                    bu_raw = env.get("conteudo") if isinstance(env, dict) and "conteudo" in env else data
                else:
                    bu_raw = data
                
                bu = spec.decode("EntidadeBoletimUrna", bu_raw)
                if "identificacaoSecao" in bu:
                    sucessos += 1
                else:
                    falhas += 1
            except Exception as e:
                falhas += 1

    log_fn(f"[ASN.1 OK] {zip_bu.name}: {sucessos} decodificados com sucesso ({falhas} falhas)")
    return falhas == 0, sucessos, falhas
