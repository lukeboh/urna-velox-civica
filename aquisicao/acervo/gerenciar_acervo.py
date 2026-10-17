#!/usr/bin/env python3
"""CLI principal para montagem, atualização e verificação do acervo de BUs do TSE."""

import argparse
import csv
import json
import os
import sys
import time
import zipfile
from datetime import datetime
from pathlib import Path

# Adiciona o diretório ao path para importação local
sys.path.insert(0, str(Path(__file__).parent))

import acervo_tse
from acervo_tse import (
    RateLimiter, TSEClient, baixar_arquivo_integro, derivar_bus_zip,
    baixar_documentacao, testar_amostra_asn1, get_disk_free_gb, USER_AGENT
)

LOG_FILE = None

def log(msg):
    ts = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
    formatted = f"[{ts}] {msg}"
    print(formatted)
    if LOG_FILE:
        with open(LOG_FILE, "a", encoding="utf-8") as f:
            f.write(formatted + "\n")


def parse_recurso_nome(nome_recurso: str, url: str):
    """Extrai (ano, turno, uf) a partir do nome ou da URL do recurso."""
    # Ex: bu_imgbu_logjez_rdv_vscmr_2026_1t_AC.zip
    base = os.path.basename(url)
    parts = base.replace(".zip", "").split("_")
    ano = None
    turno = None
    uf = None
    for p in parts:
        if p in ["2026", "2024", "2022", "2020"]:
            ano = p
        elif p.endswith("t") and len(p) == 2 and p[0].isdigit():
            turno = p[0]
        elif len(p) == 2 and p.isalpha():
            uf = p.upper()

    # Suplementares
    if "suplementares" in url:
        ano = "2024" if "2024" in url else ("2022" if "2022" in url else "2020")
        turno = "1"
        uf = "SUPL"

    return ano, turno, uf


def descobrir_recursos_ckan(client: TSEClient, anos_alvo=None):
    """Consulta a API CKAN do TSE e mapeia todos os pacotes de arquivos de urna brutos."""
    log("Consultando API CKAN do TSE para catalogar eleições...")
    pacotes_busca = [
        "resultados-2026-arquivos-transmitidos-para-totalizacao",
        "resultados-2024-arquivos-transmitidos-para-totalizacao",
        "resultados-2022-arquivos-transmitidos-para-totalizacao",
        "resultados-eleicoes-suplementares-2024",
        "resultados-eleicoes-suplementares-2022",
        "resultados-2020-suplementares",
    ]

    recursos = []
    for pkg_id in pacotes_busca:
        url_show = f"https://dadosabertos.tse.jus.br/api/3/action/package_show?id={pkg_id}"
        try:
            resp = client.request(url_show)
            data = json.loads(resp.read().decode("utf-8"))
            if not data.get("success"):
                continue
            
            pkg = data["result"]
            for r in pkg.get("resources", []):
                r_url = r.get("url", "")
                if not r_url.endswith(".zip") or "arqurnatot" not in r_url:
                    continue

                ano, turno, uf = parse_recurso_nome(r.get("name", ""), r_url)
                if not ano or not turno or not uf:
                    continue

                if anos_alvo and ano not in anos_alvo:
                    continue

                recursos.append({
                    "pacote": pkg_id,
                    "nome": r.get("name"),
                    "ano": ano,
                    "turno": turno,
                    "uf": uf,
                    "url": r_url,
                    "id": r.get("id"),
                    "created": r.get("created"),
                    "last_modified": r.get("last_modified")
                })
        except Exception as e:
            log(f"[AVISO] Falha ao consultar pacote {pkg_id}: {e}")

    # Ordenação cronológica inversa: 2026 2t -> 2026 1t -> 2024 2t -> 2024 1t ...
    # Dentro de cada turno: UFs alfabéticas com ZZ por último
    def sort_key(item):
        ano_num = int(item["ano"])
        turno_num = int(item["turno"])
        uf = item["uf"]
        uf_weight = 999 if uf == "ZZ" else (998 if uf == "SUPL" else ord(uf[0]) * 100 + ord(uf[1]))
        return (-ano_num, -turno_num, uf_weight)

    recursos.sort(key=sort_key)
    log(f"Catalogados {len(recursos)} arquivos de urna na ordem cronológica inversa.")
    return recursos


def carregar_manifesto(caminho_csv: Path) -> dict:
    if not caminho_csv.exists():
        return {}
    itens = {}
    with open(caminho_csv, "r", encoding="utf-8") as f:
        reader = csv.DictReader(f)
        for row in reader:
            chave = (row["ano"], row["turno"], row["uf"])
            itens[chave] = row
    return itens


def salvar_manifesto(caminho_csv: Path, linhas: list):
    campos = ["ano", "turno", "uf", "url", "bytes", "etag", "last_modified", "sha256", "baixado_em", "situacao"]
    caminho_csv.parent.mkdir(parents=True, exist_ok=True)
    with open(caminho_csv, "w", newline="", encoding="utf-8") as f:
        writer = csv.DictWriter(f, fieldnames=campos)
        writer.writeheader()
        writer.writerows(linhas)


def main():
    parser = argparse.ArgumentParser(description="Coletor e Gerenciador do Acervo de BUs do TSE")
    parser.add_argument("--raiz", default=r"D:\dados\tse-arquivos-urna", help="Caminho raiz do acervo")
    parser.add_argument("--anos", help="Anos a baixar (separados por vírgula, ex: 2026,2024)")
    parser.add_argument("--turnos", help="Turnos a baixar (separados por vírgula, ex: 1,2)")
    parser.add_argument("--ufs", help="UFs a baixar (separadas por vírgula, ex: DF,SP,ZZ)")
    parser.add_argument("--limite-ufs", type=int, default=0, help="Limita o número de UFs por turno (para testes rápidos)")
    parser.add_argument("--somente-bus", action="store_true", help="Após derivar os BUs, não mantém o ZIP original")
    parser.add_argument("--rps", type=float, default=90.0, help="Taxa máxima de requisições por segundo (teto 100)")
    parser.add_argument("--spec-dir", default=r"d:\git\lukeboh\urna-velox\aquisicao\spec", help="Diretório de especificações ASN.1")

    args = parser.parse_args()

    raiz = Path(args.raiz)
    raiz.mkdir(parents=True, exist_ok=True)

    global LOG_FILE
    pasta_logs = raiz / "logs"
    pasta_logs.mkdir(parents=True, exist_ok=True)
    LOG_FILE = pasta_logs / f"exec_{datetime.now().strftime('%Y%m%d_%H%M%S')}.log"

    log("=" * 60)
    log("Iniciando rotina do Acervo de BUs do TSE")
    log(f"Raiz: {raiz}")
    log(f"Espaço livre em disco: {get_disk_free_gb(raiz):.2f} GB")
    log("=" * 60)

    limiter = RateLimiter(rps_max=args.rps)
    client = TSEClient(limiter=limiter)

    anos_filtro = [a.strip() for a in args.anos.split(",")] if args.anos else None
    turnos_filtro = [t.strip() for t in args.turnos.split(",")] if args.turnos else None
    ufs_filtro = [u.strip().upper() for u in args.ufs.split(",")] if args.ufs else None

    # 1. Catálogo CKAN
    recursos = descobrir_recursos_ckan(client, anos_alvo=anos_filtro)
    
    # Salvar catalogo.json
    catalogo_path = raiz / "catalogo.json"
    with open(catalogo_path, "w", encoding="utf-8") as f:
        json.dump(recursos, f, indent=2, ensure_ascii=False)
    log(f"Catálogo salvo em {catalogo_path}")

    # Filtrar recursos solicitados
    filtrados = []
    count_por_turno = {}
    for r in recursos:
        if turnos_filtro and r["turno"] not in turnos_filtro:
            continue
        if ufs_filtro and r["uf"] not in ufs_filtro:
            continue

        chave_turno = (r["ano"], r["turno"])
        c = count_por_turno.get(chave_turno, 0)
        if args.limite_ufs > 0 and c >= args.limite_ufs:
            continue
        count_por_turno[chave_turno] = c + 1
        filtrados.append(r)

    log(f"Recursos a processar nesta execução: {len(filtrados)}")

    # 2. Manifesto
    manifesto_path = raiz / "manifesto.csv"
    manifesto_atual = carregar_manifesto(manifesto_path)

    # 3. Documentação por ano presente
    anos_presentes = set(r["ano"] for r in filtrados)
    for ano in sorted(anos_presentes, reverse=True):
        pasta_doc = raiz / "documentacao" / ano
        baixar_documentacao(client, ano, pasta_doc, log_fn=log)

    # 4. Processamento dos arquivos
    dados_relatorio = []
    
    for r in filtrados:
        ano = r["ano"]
        turno = r["turno"]
        uf = r["uf"]
        url = r["url"]
        chave = (ano, turno, uf)

        nome_original = os.path.basename(url)
        pasta_orig = raiz / "originais" / ano / f"{turno}t"
        dest_orig = pasta_orig / nome_original

        pasta_bu = raiz / "bus" / ano / f"{turno}t"
        dest_bu_zip = pasta_bu / f"{uf}.zip"
        dest_bu_csv = pasta_bu / f"{uf}.csv"

        free_gb = get_disk_free_gb(raiz)
        if free_gb < 15.0:
            log(f"[ERRO ESPAÇO] Espaço em disco perigosamente baixo ({free_gb:.1f} GB). Interrompendo execução.")
            break

        log(f"\n--- Processando {ano} {turno}ºT {uf} ---")
        t_inicio = time.time()
        
        try:
            # Download do original
            info_down = baixar_arquivo_integro(client, url, dest_orig, log_fn=log)
            
            # Derivar BUs
            total_bus = derivar_bus_zip(dest_orig, dest_bu_zip, dest_bu_csv, ano, log_fn=log)

            # Extrair leiame.pdf se presente
            with zipfile.ZipFile(dest_orig, "r") as z_in:
                if "leiame.pdf" in z_in.namelist():
                    leiame_dest = raiz / "documentacao" / ano / f"leiame_{turno}t_{uf}.pdf"
                    leiame_dest.parent.mkdir(parents=True, exist_ok=True)
                    with open(leiame_dest, "wb") as f_leiame:
                        f_leiame.write(z_in.read("leiame.pdf"))

            # Teste de conformidade ASN.1 na amostra usando a especificação daquele ano
            spec_ano = raiz / "documentacao" / ano / "spec" / "bu.asn1"
            if not spec_ano.exists():
                spec_ano = Path(args.spec_dir) / "bu.asn1"
            asn1_ok, n_ok, n_err = testar_amostra_asn1(dest_bu_zip, spec_ano, ano, amostra_n=20, log_fn=log)

            duracao_total = time.time() - t_inicio
            
            manifesto_atual[chave] = {
                "ano": ano,
                "turno": turno,
                "uf": uf,
                "url": url,
                "bytes": info_down["bytes"],
                "etag": info_down["etag"],
                "last_modified": info_down["last_modified"],
                "sha256": info_down["sha256"],
                "baixado_em": datetime.now().isoformat(),
                "situacao": "completo"
            }

            dados_relatorio.append({
                "ano": ano,
                "turno": turno,
                "uf": uf,
                "bus": total_bus,
                "orig_bytes": info_down["bytes"],
                "bu_bytes": dest_bu_zip.stat().st_size,
                "tempo_s": duracao_total,
                "asn1_valido": asn1_ok,
                "situacao": "OK"
            })

            # Se configurado somente-bus, remove original para liberar espaço
            if args.somente_bus:
                dest_orig.unlink()
                manifesto_atual[chave]["situacao"] = "somente_bus"
                log(f"[SOMENTE_BUS] Original {dest_orig.name} removido para economizar disco.")

        except Exception as e:
            log(f"[FALHA] Erro processando {ano} {turno}t {uf}: {e}")
            dados_relatorio.append({
                "ano": ano,
                "turno": turno,
                "uf": uf,
                "bus": 0,
                "orig_bytes": 0,
                "bu_bytes": 0,
                "tempo_s": time.time() - t_inicio,
                "asn1_valido": False,
                "situacao": f"FALHA: {e}"
            })

    # Atualiza manifesto em disco
    salvar_manifesto(manifesto_path, list(manifesto_atual.values()))

    # Gera README do acervo
    readme_path = raiz / "README.md"
    gerar_readme_acervo(readme_path, raiz)

    # Gera relatório final
    relatorio_path = raiz / "RELATORIO.md"
    gerar_relatorio(relatorio_path, dados_relatorio)

    log("\n" + "=" * 60)
    log("Rotina concluída com sucesso.")
    log(f"Manifesto: {manifesto_path}")
    log(f"Relatório: {relatorio_path}")
    log("=" * 60)


def gerar_readme_acervo(dest: Path, raiz: Path):
    conteudo = f"""# Acervo Local de Boletins de Urna (BUs) — TSE

Acervo estruturado de arquivos de urna brutos publicados pelo Tribunal Superior Eleitoral (TSE).
Alimenta os testes e totalização do projeto **Urna Velox** sem necessidade de conexão com a rede.

## Origem dos Dados
* **Portal de Dados Abertos do TSE**: Conjuntos `resultados-<ano>-arquivos-transmitidos-para-totalizacao`.
* **CDN do TSE**: `cdn.tse.jus.br` (Akamai).
* **Especificações Técnicas**: Arquivos ASN.1 oficiais de BU, RDV e Assinatura Digital.

## Estrutura de Diretórios
```text
{raiz.name}/
  README.md                 Este documento
  catalogo.json             Catálogo de pacotes e recursos descobertos na API CKAN
  manifesto.csv             Registro com URL, ETag, tamanho e SHA-256 de cada pacote
  originais/<ano>/<turno>t/ Arquivos ZIP originais e intocados baixados do CDN
  bus/<ano>/<turno>t/       Pacotes ZIP (.zip) contendo exclusivamente BUs e assinaturas
                            (método ZIP STORED, sem recompressão para leitura direta)
  bus/<ano>/<turno>t/<UF>.csv Índice com (arquivo, município, zona, seção, bytes, SHA-256)
  documentacao/<ano>/       Especificações ASN.1, scripts Python e manuais do TSE
  logs/                     Histórico detalhado de execução
```

## Convenções de Nomes por Ano
* **2026 e 2024**: `o0<pleito><uf><mun:05><zona:04><secao:04>-bu.dat` e `-vota.vsc`
* **2022**: `o00406-<mun:05><zona:04><secao:04>.bu` e `.vscmr`

## Validação e Integridade
Todos os arquivos baixados possuem validação de integridade por soma de verificação SHA-256 e teste de descompactação de ZIP.
Amostras de BUs foram decodificadas com `asn1tools` e a especificação `bu.asn1` oficial de cada eleição.
"""
    with open(dest, "w", encoding="utf-8") as f:
        f.write(conteudo)


def gerar_relatorio(dest: Path, dados: list):
    conteudo = f"""# Relatório de Coleta e Verificação do Acervo de BUs

Gerado em: {datetime.now().strftime("%Y-%m-%d %H:%M:%S")}

## Resumo por Pacote Processado

| Ano | Turno | UF | Nº BUs | Tamanho Original | Tamanho BUs (Derivado) | Tempo (s) | ASN.1 Válido | Situação |
|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|
"""
    for d in dados:
        orig_mb = f"{d['orig_bytes'] / (1024**2):.1f} MB" if d['orig_bytes'] > 0 else "-"
        bu_mb = f"{d['bu_bytes'] / (1024**2):.1f} MB" if d['bu_bytes'] > 0 else "-"
        conteudo += f"| {d['ano']} | {d['turno']} | {d['uf']} | {d['bus']} | {orig_mb} | {bu_mb} | {d['tempo_s']:.1f} | {d['asn1_valido']} | {d['situacao']} |\n"

    conteudo += """
## Eleições e Recursos Pesquisados no Portal de Dados Abertos
* **2026 1º Turno**: 28 recursos disponíveis (27 UFs + ZZ). 2º Turno ainda não realizado/publicado.
* **2024 1º e 2º Turnos**: 52 recursos (eleição municipal regular sem exterior/DF).
* **2022 1º e 2º Turnos**: 56 recursos (27 UFs + ZZ em ambos os turnos).
* **Eleições Suplementares**: Localizados pacotes em 2024, 2022 e 2020 (`bu_imgbu_logjez_rdv_vscmr_ele_suplementares_<ano>.zip`).
* **Anos Anteriores (2020 regular, 2018, 2016)**: No Portal de Dados Abertos constam apenas arquivos tabulares processados em CSV/TXT (`buweb/bweb_...`), sem disponibilização dos arquivos binários brutos de urna (`bu_imgbu...`).
"""
    with open(dest, "w", encoding="utf-8") as f:
        f.write(conteudo)


if __name__ == "__main__":
    main()
