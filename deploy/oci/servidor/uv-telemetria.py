#!/usr/bin/env python3
"""uv-telemetria — amostra a cada N segundos o estado do nó durante o ciclo diário (docs/CICLO_DIARIO.md).

Uma linha JSON por amostra em <pasta>/telemetria.jsonl:
  t (ISO), no, saude (a saude.json inteira do uv-servico: fila, aplicadas, rede, latências, custos, federação,
  matematicamente, apuração, recursos, difusão, previsão), sistema (carga, memória, disco, CPU), processos (RSS e CPU do
  uv-servico e do uv-simulador), conexoes_8080 (conexões estabelecidas na porta da página: Caddy e clientes) e
  simulador (relógio e aceleração, no nó A).

  uv-telemetria.py <pasta-da-execução> <no> [intervalo_s]
"""
import json, os, sys, time, urllib.request
from datetime import datetime, timezone, timedelta

PASTA, NO = sys.argv[1], sys.argv[2]
INTERVALO = float(sys.argv[3]) if len(sys.argv) > 3 else 15.0
BR = timezone(timedelta(hours=-3))
TICK = os.sysconf('SC_CLK_TCK')


def ler(p):
    try:
        with open(p, encoding='utf-8') as f: return f.read()
    except OSError: return None


def memoria():
    m = {}
    for l in (ler('/proc/meminfo') or '').splitlines():
        k, _, v = l.partition(':')
        if k in ('MemTotal', 'MemAvailable', 'SwapTotal', 'SwapFree', 'Cached'): m[k] = int(v.split()[0]) // 1024
    return m


def cpu_total():
    p = (ler('/proc/stat') or '').splitlines()[0].split()[1:]
    v = [int(x) for x in p]
    return sum(v), v[3] + (v[4] if len(v) > 4 else 0)


def processos():
    r = {}
    for pid in os.listdir('/proc'):
        if not pid.isdigit(): continue
        nome = (ler(f'/proc/{pid}/comm') or '').strip()
        if nome not in ('uv-servico', 'uv-simulador', 'caddy'): continue
        st = ler(f'/proc/{pid}/status') or ''
        rss = next((int(l.split()[1]) // 1024 for l in st.splitlines() if l.startswith('VmRSS')), None)
        hwm = next((int(l.split()[1]) // 1024 for l in st.splitlines() if l.startswith('VmHWM')), None)
        s = (ler(f'/proc/{pid}/stat') or '').rsplit(')', 1)[-1].split()
        cpu_s = (int(s[11]) + int(s[12])) / TICK if len(s) > 12 else None
        thr = next((int(l.split()[1]) for l in st.splitlines() if l.startswith('Threads')), None)
        r[nome] = {'pid': int(pid), 'rss_mb': rss, 'pico_rss_mb': hwm, 'cpu_s': cpu_s, 'threads': thr}
    return r


def conexoes(porta=8080):
    n = 0
    for arq in ('/proc/net/tcp', '/proc/net/tcp6'):
        for l in (ler(arq) or '').splitlines()[1:]:
            c = l.split()
            if len(c) > 3 and c[3] == '01' and int(c[1].rsplit(':', 1)[1], 16) == porta: n += 1
    return n


def simulador():
    try:
        with urllib.request.urlopen('http://127.0.0.1:18080/controle', timeout=2) as f: return json.loads(f.read())
    except Exception: return None


ant = cpu_total()
while True:
    t0 = time.time()
    try:
        s = None
        txt = ler(os.path.join(PASTA, 'saida', 'saude.json'))
        if txt:
            try: s = json.loads(txt)
            except ValueError: s = None
        tot, ocioso = cpu_total()
        dt, di = tot - ant[0], ocioso - ant[1]; ant = (tot, ocioso)
        vfs = os.statvfs('/srv')
        linha = {
            't': datetime.now(BR).isoformat(timespec='seconds'), 'no': NO,
            'sistema': {'carga': (ler('/proc/loadavg') or '').split()[:3], 'cpu_pct': round(100.0 * (dt - di) / dt, 1) if dt else None,
                        'memoria_mb': memoria(), 'disco_livre_gb': round(vfs.f_bavail * vfs.f_frsize / 1e9, 2)},
            'processos': processos(), 'conexoes_8080': conexoes(), 'simulador': simulador(), 'saude': s,
        }
        with open(os.path.join(PASTA, 'telemetria.jsonl'), 'a', encoding='utf-8') as f:
            f.write(json.dumps(linha, ensure_ascii=False, separators=(',', ':')) + '\n')
    except Exception as e:  # a telemetria nunca derruba nada: anota e segue
        print(f'{datetime.now(BR).isoformat()} erro: {e!r}', file=sys.stderr, flush=True)
    time.sleep(max(1.0, INTERVALO - (time.time() - t0)))
