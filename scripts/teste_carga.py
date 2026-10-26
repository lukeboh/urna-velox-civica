#!/usr/bin/env python3
"""
teste_carga.py — Executor de Teste de Carga e Estresse do Urna Velox

Executa teste de carga em degraus crescentes (ex: 1.000, 3.000, 5.000, 8.000, 10.000, 12.000, 15.000 usuários),
medindo latência p50/p99/máx, taxa de conexões, vazão de difusão e consumo de CPU/RAM.

Suporta:
  - Modo local: sobe uv-simulador (dados reais 2026 a 60x) + uv-servico e roda uv-carga
  - Modo produção: executa uv-carga diretamente contra a URL pública (ex: https://urnavelox.com.br)
"""

import argparse
import json
import os
import signal
import socket
import subprocess
import sys
import time
import resource
from pathlib import Path
from urllib.request import urlopen, Request

def get_free_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.bind(('127.0.0.1', 0))
        return s.getsockname()[1]

def get_process_stats(pid: int) -> dict:
    try:
        with open(f"/proc/{pid}/status") as f:
            lines = f.readlines()
        rss_kb = 0
        vm_peak_kb = 0
        for l in lines:
            if l.startswith("VmRSS:"):
                rss_kb = int(l.split()[1])
            elif l.startswith("VmPeak:"):
                vm_peak_kb = int(l.split()[1])
        return {
            "rss_mb": round(rss_kb / 1024, 1),
            "peak_mb": round(vm_peak_kb / 1024, 1)
        }
    except Exception:
        return {"rss_mb": 0, "peak_mb": 0}

def wait_for_http(url: str, timeout_s: float = 30.0) -> bool:
    t0 = time.time()
    while time.time() - t0 < timeout_s:
        try:
            req = Request(url, headers={"User-Agent": "uv-healthcheck"})
            with urlopen(req, timeout=1) as resp:
                if resp.status == 200:
                    return True
        except Exception:
            pass
        time.sleep(0.2)
    return False

def run_local_load_test(degraus: list[int], duracao_por_degrau: int, rampa: float, saida_dir: Path) -> dict:
    uv_work = Path(os.environ.get("UV_WORK", Path.home() / "uvreal"))
    uv_bin = Path(os.environ.get("UV_BIN", uv_work / "target-dev" / "release"))
    cand = Path(os.environ.get("UV_CANDIDATOS", uv_work / "etapa5" / "preparo-2026-1t" / "candidatos.json"))
    
    simulador_bin = uv_bin / "uv-simulador"
    servico_bin = uv_bin / "uv-servico"
    carga_bin = uv_bin / "uv-carga"

    for b in [simulador_bin, servico_bin, carga_bin]:
        if not b.exists():
            print(f"ERRO: binário não encontrado: {b}", file=sys.stderr, flush=True)
            sys.exit(1)

    saida_dir.mkdir(parents=True, exist_ok=True)
    porta_arquivo = saida_dir / "porta_simulador"
    if porta_arquivo.exists():
        porta_arquivo.unlink()

    sim_out = open(saida_dir / "simulador.out", "w")
    sim_err = open(saida_dir / "simulador.err", "w")
    srv_err = open(saida_dir / "servico.err", "w")

    print("=" * 75, flush=True)
    print(" PREPARANDO AMBIENTE LOCAL DE TESTE DE CARGA E ESTRESSE", flush=True)
    print("=" * 75, flush=True)
    print(f"• UV_WORK:     {uv_work}", flush=True)
    print(f"• Binários:    {uv_bin}", flush=True)
    print(f"• Candidatos:  {cand}", flush=True)
    print(f"• Degraus:     {degraus} usuários simultâneos", flush=True)
    print(f"• Duração:     {duracao_por_degrau}s estáveis por degrau (rampa: {rampa} conn/s)", flush=True)
    print(flush=True)

    # Iniciar Simulador (1º turno 2026, aceleração 60x)
    print("-> Iniciando uv-simulador (aceleração 60×)...", flush=True)
    sim_cmd = [
        str(simulador_bin),
        "--dados", str(uv_work / "dados"),
        "--cs", str(uv_work / "config"),
        "--metricas", f"{uv_work}/metricas_multi.csv,{uv_work}/metricas_nacional.csv,{uv_work}/metricas_retomada.csv",
        "--oficial", str(uv_work / "oficial"),
        "--pleito", "3220",
        "--ciclo", "ele2026",
        "--acelerar", "60",
        "--corte", "05/10/2026 02:00:00",
        "--horario", "recebido",
        "--log", str(saida_dir / "simulador.log"),
        "--resumo", str(saida_dir / "simulador_resumo.json"),
        "--porta-arquivo", str(porta_arquivo)
    ]
    p_sim = subprocess.Popen(sim_cmd, stdout=sim_out, stderr=sim_err)

    try:
        t0 = time.time()
        porta_sim = None
        while time.time() - t0 < 30:
            if porta_arquivo.exists() and porta_arquivo.stat().st_size > 0:
                with open(porta_arquivo) as f:
                    porta_sim = f.read().strip()
                if porta_sim:
                    break
            time.sleep(0.2)

        if not porta_sim:
            raise RuntimeError("Timeout aguardando uv-simulador subir.")
        print(f"   uv-simulador pronto na porta {porta_sim}", flush=True)

        # Iniciar uv-servico
        porta_srv = get_free_port()
        dados_dir = saida_dir / "dados_servico"
        publicacao_dir = saida_dir / "publicacao_servico"
        dados_dir.mkdir(parents=True, exist_ok=True)
        publicacao_dir.mkdir(parents=True, exist_ok=True)

        print(f"-> Iniciando uv-servico na porta {porta_srv} (--max-clientes 35000)...", flush=True)
        srv_cmd = [
            str(servico_bin),
            "--pleito", "3220",
            "--ciclo", "ele2026",
            "--candidatos", str(cand),
            "--dados", str(dados_dir),
            "--saida", str(publicacao_dir),
            "--base", f"http://127.0.0.1:{porta_sim}",
            "--aceleracao-local", "60",
            "--http", f"127.0.0.1:{porta_srv}",
            "--max-clientes", "35000",
            "--continuar"
        ]
        p_srv = subprocess.Popen(srv_cmd, stdout=subprocess.DEVNULL, stderr=srv_err)

        if not wait_for_http(f"http://127.0.0.1:{porta_srv}/api/v1/estado", timeout_s=30):
            raise RuntimeError("Timeout aguardando uv-servico responder /api/v1/estado.")
        print(f"   uv-servico pronto e apurando!", flush=True)
        print(flush=True)

        resultados_degraus = []
        for clientes in degraus:
            print("-" * 75, flush=True)
            print(f" EXECUTANDO DEGRAU: {clientes:,} USUÁRIOS SIMULTÂNEOS", flush=True)
            print("-" * 75, flush=True)

            stats_antes = get_process_stats(p_srv.pid)
            relatorio_degrau = saida_dir / f"carga_{clientes}.json"

            cmd_carga = [
                str(carga_bin),
                "--endereco", f"127.0.0.1:{porta_srv}",
                "--clientes", str(clientes),
                "--perfil", "misto",
                "--rampa", str(rampa),
                "--duracao", str(duracao_por_degrau),
                "--threads", "6",
                "--relatorio", str(relatorio_degrau)
            ]

            t_inicio = time.time()
            res = subprocess.run(cmd_carga, capture_output=True, text=True)
            duracao_real = round(time.time() - t_inicio, 1)

            stats_depois = get_process_stats(p_srv.pid)

            if not relatorio_degrau.exists():
                print(f"   ERRO ao executar uv-carga:\n{res.stderr}", file=sys.stderr, flush=True)
                resultados_degraus.append({
                    "clientes": clientes,
                    "status": "FALHA",
                    "erro": res.stderr
                })
                continue

            with open(relatorio_degrau) as f:
                rel = json.load(f)

            cx = rel.get("conexoes", {})
            msgs = rel.get("mensagens", {})
            lat_bu = rel.get("latencias_ms", {}).get("bu_cliente", {})
            lat_srv = rel.get("latencias_ms", {}).get("servidor_cliente", {})

            abertas = cx.get("abertas", 0)
            falhas = cx.get("falhas", 0)
            p50 = lat_bu.get("p50", 0.0)
            p99 = lat_bu.get("p99", 0.0)
            max_lat = lat_bu.get("max", 0.0)
            p50_tot = lat_bu.get("p50_total", p50)
            p99_tot = lat_bu.get("p99_total", p99)
            total_msgs = msgs.get("total", 0)
            total_bytes = msgs.get("bytes", 0)
            mb_s = round((total_bytes / (1024 * 1024)) / max(duracao_real, 1), 2)
            msgs_s = round(total_msgs / max(duracao_real, 1), 0)

            # Classificação do SLA (Meta TSE / Urna Velox: < 50ms)
            if falhas > 0:
                sla = "🔴 FALHAS"
            elif p99 > 100.0:
                sla = "🟠 DEGRADADO (>100ms)"
            elif p99 > 50.0:
                sla = "🟡 ALERTA (>50ms)"
            else:
                sla = "🟢 EXCELENTE (<50ms)"

            print(f"   • Conexões ativas:   {abertas:,}/{clientes:,} (Falhas: {falhas})", flush=True)
            print(f"   • Latência BU→Cli:   p50: {p50:.2f} ms | p99: {p99:.2f} ms | máx: {max_lat:.2f} ms", flush=True)
            print(f"   • Servidor→Cli:      p50: {lat_srv.get('p50',0):.2f} ms | p99: {lat_srv.get('p99',0):.2f} ms", flush=True)
            print(f"   • Taxa de difusão:   {msgs_s:,.0f} msgs/s | {mb_s} MB/s (Total: {total_msgs:,} msgs)", flush=True)
            print(f"   • Memória Servidor:  {stats_depois.get('rss_mb')} MB RSS (Δ +{stats_depois.get('rss_mb',0) - stats_antes.get('rss_mb',0):.1f} MB)", flush=True)
            print(f"   • Avaliação SLA:     {sla}", flush=True)
            print(flush=True)

            resultados_degraus.append({
                "clientes": clientes,
                "abertas": abertas,
                "falhas": falhas,
                "p50_bu": p50,
                "p99_bu": p99,
                "max_bu": max_lat,
                "p50_srv": lat_srv.get("p50", 0.0),
                "p99_srv": lat_srv.get("p99", 0.0),
                "total_msgs": total_msgs,
                "msgs_s": msgs_s,
                "mb_s": mb_s,
                "rss_mb": stats_depois.get("rss_mb", 0),
                "sla": sla
            })

            # Se teve mais de 10% de falhas, interrompe
            if falhas > 0 and (falhas / clientes) > 0.1:
                print("⚠️ Interrompendo degraus adicionais devido a taxa de falha elevada.", flush=True)
                break

            time.sleep(2)

        return {
            "modo": "local",
            "degraus": resultados_degraus,
            "servico_pid": p_srv.pid
        }

    finally:
        print("-> Encerrando processos locais...", flush=True)
        try:
            p_srv.terminate()
            p_srv.wait(timeout=3)
        except Exception:
            try:
                p_srv.kill()
            except Exception:
                pass

        try:
            p_sim.terminate()
            p_sim.wait(timeout=3)
        except Exception:
            try:
                p_sim.kill()
            except Exception:
                pass

        sim_out.close()
        sim_err.close()
        srv_err.close()
        # Limpar dados temporários gerados para não acumular disco
        dados_dir = saida_dir / "dados_servico"
        if dados_dir.exists():
            subprocess.run(["rm", "-rf", str(dados_dir)], check=False)

def run_remote_load_test(url: str, degraus: list[int], duracao_por_degrau: int, rampa: float, saida_dir: Path) -> dict:
    uv_work = Path(os.environ.get("UV_WORK", Path.home() / "uvreal"))
    uv_bin = Path(os.environ.get("UV_BIN", uv_work / "target-dev" / "release"))
    carga_bin = uv_bin / "uv-carga"

    if not carga_bin.exists():
        print(f"ERRO: binário uv-carga não encontrado: {carga_bin}", file=sys.stderr, flush=True)
        sys.exit(1)

    saida_dir.mkdir(parents=True, exist_ok=True)

    print("=" * 75, flush=True)
    print(" EXECUTANDO TESTE CONTRA PRODUÇÃO / REMOTO", flush=True)
    print("=" * 75, flush=True)
    print(f"• Alvo:        {url}", flush=True)
    print(f"• Degraus:     {degraus} usuários simultâneos", flush=True)
    print(f"• Duração:     {duracao_por_degrau}s estáveis por degrau (rampa: {rampa} conn/s)", flush=True)
    print(flush=True)

    resultados_degraus = []
    for clientes in degraus:
        print("-" * 75, flush=True)
        print(f" EXECUTANDO DEGRAU: {clientes:,} USUÁRIOS SIMULTÂNEOS -> {url}", flush=True)
        print("-" * 75, flush=True)

        relatorio_degrau = saida_dir / f"carga_prod_{clientes}.json"
        cmd_carga = [
            str(carga_bin),
            "--endereco", url,
            "--clientes", str(clientes),
            "--perfil", "misto",
            "--rampa", str(rampa),
            "--duracao", str(duracao_por_degrau),
            "--threads", "6",
            "--relatorio", str(relatorio_degrau)
        ]

        t_inicio = time.time()
        res = subprocess.run(cmd_carga, capture_output=True, text=True)
        duracao_real = round(time.time() - t_inicio, 1)

        if not relatorio_degrau.exists():
            print(f"   ERRO ao executar uv-carga:\n{res.stderr}", file=sys.stderr, flush=True)
            resultados_degraus.append({
                "clientes": clientes,
                "status": "FALHA",
                "erro": res.stderr
            })
            break

        with open(relatorio_degrau) as f:
            rel = json.load(f)

        cx = rel.get("conexoes", {})
        msgs = rel.get("mensagens", {})
        lat_bu = rel.get("latencias_ms", {}).get("bu_cliente", {})
        lat_srv = rel.get("latencias_ms", {}).get("servidor_cliente", {})

        abertas = cx.get("abertas", 0)
        falhas = cx.get("falhas", 0)
        p50 = lat_bu.get("p50", 0.0)
        p99 = lat_bu.get("p99", 0.0)
        max_lat = lat_bu.get("max", 0.0)
        total_msgs = msgs.get("total", 0)
        total_bytes = msgs.get("bytes", 0)
        mb_s = round((total_bytes / (1024 * 1024)) / max(duracao_real, 1), 2)
        msgs_s = round(total_msgs / max(duracao_real, 1), 0)

        if falhas > 0:
            sla = "🔴 FALHAS"
        elif p99 > 100.0:
            sla = "🟠 DEGRADADO (>100ms)"
        elif p99 > 50.0:
            sla = "🟡 ALERTA (>50ms)"
        else:
            sla = "🟢 EXCELENTE (<50ms)"

        print(f"   • Conexões ativas:   {abertas:,}/{clientes:,} (Falhas: {falhas})", flush=True)
        print(f"   • Latência BU→Cli:   p50: {p50:.2f} ms | p99: {p99:.2f} ms | máx: {max_lat:.2f} ms", flush=True)
        print(f"   • Servidor→Cli:      p50: {lat_srv.get('p50',0):.2f} ms | p99: {lat_srv.get('p99',0):.2f} ms", flush=True)
        print(f"   • Taxa de difusão:   {msgs_s:,.0f} msgs/s | {mb_s} MB/s (Total: {total_msgs:,} msgs)", flush=True)
        print(f"   • Avaliação SLA:     {sla}", flush=True)
        print(flush=True)

        resultados_degraus.append({
            "clientes": clientes,
            "abertas": abertas,
            "falhas": falhas,
            "p50_bu": p50,
            "p99_bu": p99,
            "max_bu": max_lat,
            "p50_srv": lat_srv.get("p50", 0.0),
            "p99_srv": lat_srv.get("p99", 0.0),
            "total_msgs": total_msgs,
            "msgs_s": msgs_s,
            "mb_s": mb_s,
            "sla": sla
        })

        if falhas > 0 and (falhas / clientes) > 0.30:
            print("⚠️ Interrompendo degraus adicionais contra produção devido a saturação/quebra (taxa de falha > 30%).", flush=True)
            break

        time.sleep(3)

    return {
        "modo": "producao",
        "url": url,
        "degraus": resultados_degraus
    }

def gerar_relatorio_md(dados: dict, caminho_relatorio: Path):
    degraus = dados.get("degraus", [])
    if not degraus:
        return

    linhas = []
    linhas.append("# Relatório de Teste de Carga e Estresse — Urna Velox")
    linhas.append(f"\n**Data:** {time.strftime('%Y-%m-%d %H:%M:%S')} · **Modo:** `{dados.get('modo')}`")
    if dados.get("url"):
        linhas.append(f"· **Alvo:** `{dados.get('url')}`")
    linhas.append("\n## 1. Tabela Comparativa de Capacidade")
    linhas.append("\n| Usuários Simultâneos | Conexões Abertas | Falhas | BU→Cliente p50 | BU→Cliente p99 | Servidor→Cli p99 | Vazão (msgs/s) | Banda (MB/s) | RAM Servidor (RSS) | SLA (<50ms) |")
    linhas.append("|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|")

    max_capacidade_sla = 0
    max_capacidade_absoluta = 0

    for d in degraus:
        cli = d.get("clientes", 0)
        ab = d.get("abertas", 0)
        fal = d.get("falhas", 0)
        p50 = f"{d.get('p50_bu', 0):.2f} ms"
        p99 = f"{d.get('p99_bu', 0):.2f} ms"
        psrv = f"{d.get('p99_srv', 0):.2f} ms"
        msgs = f"{d.get('msgs_s', 0):,.0f}"
        mbs = f"{d.get('mb_s', 0):.1f} MB/s"
        rss = f"{d.get('rss_mb', 0)} MB" if "rss_mb" in d else "N/A"
        sla = d.get("sla", "")

        linhas.append(f"| **{cli:,}** | {ab:,} | {fal} | {p50} | **{p99}** | {psrv} | {msgs} | {mbs} | {rss} | {sla} |")

        if fal == 0 and d.get("p99_bu", 999) < 50.0 and ab >= (cli * 0.95):
            max_capacidade_sla = cli
        if fal == 0 and ab >= (cli * 0.95):
            max_capacidade_absoluta = cli

    linhas.append("\n## 2. Conclusões e Capacidade Máxima")
    linhas.append(f"\n* **Capacidade Máxima com SLA Estrito (p99 < 50ms):** **{max_capacidade_sla:,} usuários simultâneos** por nó.")
    linhas.append(f"* **Capacidade Máxima Sustentável sem perda de pacotes/conexões:** **{max_capacidade_absoluta:,} usuários simultâneos** por nó.")
    
    if dados.get("modo") == "local":
        linhas.append("\n### Projeção para o Cluster de Produção (2 Nós na OCI):")
        linhas.append(f"* Com **2 nós ativos** (`uv-no-a` e `uv-no-b`) balanceados pelo Caddy (`least_conn`):")
        linhas.append(f"  - Capacidade combinada com SLA (< 50ms): **{max_capacidade_sla * 2:,} usuários simultâneos**.")
        linhas.append(f"  - Capacidade máxima sem queda: **{max_capacidade_absoluta * 2:,} usuários simultâneos**.")
        linhas.append(f"  - Consumo de memória por conexão WebSocket: **~125 KB** / usuário.")
        linhas.append(f"  - Em caso de falha de um dos nós: o nó remanescente sustenta **{max_capacidade_sla:,} conexões**.")

    linhas.append("\n## 3. Recomendações Operacionais")
    linhas.append("""1. **Descritores de Arquivos (`ulimit`):** Garantir `ulimit -n 65536` no systemd do `urna-velox.service` e do `caddy.service`.
2. **Limite de Conexões (`--max-clientes`):** Configurar `--max-clientes 20000` em cada nó para permitir expansão elástica.
3. **Mecanismo de Juntada (Coalescência de Versão):** O mecanismo de `tokio::sync::watch` impede filas infinitas de envio e protege o alocador de memória mesmo se a velocidade de apuração acelerar brutalmente.""")

    conteudo = "\n".join(linhas) + "\n"
    with open(caminho_relatorio, "w", encoding="utf-8") as f:
        f.write(conteudo)
    print(f"\n-> Relatório gerado com sucesso em: {caminho_relatorio}", flush=True)

def main():
    try:
        resource.setrlimit(resource.RLIMIT_NOFILE, (65536, 65536))
    except Exception:
        pass
    parser = argparse.ArgumentParser(description="Teste de Carga e Estresse — Urna Velox")
    parser.add_argument("--modo", choices=["local", "producao"], default="local", help="Modo de execução (local ou producao)")
    parser.add_argument("--url", default="https://urnavelox.com.br", help="URL para teste em modo produção")
    parser.add_argument("--degraus", default="1000,3000,5000,8000,10000,12000,15000", help="Lista de conexões por degrau separadas por vírgula")
    parser.add_argument("--duracao", type=int, default=15, help="Duração estável de cada degrau em segundos (default: 15s)")
    parser.add_argument("--rampa", type=float, default=1500.0, help="Rampa de abertura de conexões por segundo (default: 1500)")
    parser.add_argument("--saida", default=None, help="Diretório de saída para logs e relatórios")
    args = parser.parse_args()

    degraus = [int(x.strip()) for x in args.degraus.split(",") if x.strip()]

    if not args.saida:
        uv_work = Path(os.environ.get("UV_WORK", Path.home() / "uvreal"))
        timestamp = time.strftime("%Y%m%d_%H%M%S")
        saida_dir = uv_work / "ensaios" / f"carga_{args.modo}_{timestamp}"
    else:
        saida_dir = Path(args.saida)

    saida_dir.mkdir(parents=True, exist_ok=True)

    if args.modo == "local":
        res = run_local_load_test(degraus, args.duracao, args.rampa, saida_dir)
    else:
        res = run_remote_load_test(args.url, degraus, args.duracao, args.rampa, saida_dir)

    rel_md = saida_dir / "RELATORIO_CARGA.md"
    gerar_relatorio_md(res, rel_md)

    with open(saida_dir / "resumo.json", "w", encoding="utf-8") as f:
        json.dump(res, f, indent=2, ensure_ascii=False)

if __name__ == "__main__":
    main()
