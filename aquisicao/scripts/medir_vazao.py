#!/usr/bin/env python3
"""M-002: onde vai o tempo de uma eleição do ciclo diário, por nó, a partir das evidências arquivadas.

    python3 aquisicao/scripts/medir_vazao.py <pasta da execução> [--json saida.json]

<pasta da execução> é a de `63_ciclo.sh --telemetria` (com no-a.tar.gz/no-b.tar.gz) ou uma já extraída (no-a/…/,
no-b/…/ com telemetria.jsonl.gz, secoes.log.gz, saude.json, servico.log, RESUMO.json). Para cada nó:

* fases: partida (serviço no ar → 1º BU aplicado), 50 %, 90 %, 99 % e 100 % dos BUs aplicados, cauda (99 → 100 %);
* vazão: BUs aplicados por segundo (média e pico em janelas de 10 s), próprios × recebidos do par (secoes.log);
* CPU: segundos de CPU de cada processo (uv-servico, uv-simulador, caddy) entre a partida e o encerramento, em
  núcleos médios, e a CPU do sistema (telemetria a cada 15 s);
* rede: requisições por tipo (cs/aux/bu, 304), MB; federação: publicados, recebidos, já aplicados (trabalho repetido),
  assumidas/delegadas;
* custos internos do serviço (custos_ms/latencias_ms do saude.json: n, p50, p99, máx) e a estimativa n × p50.
"""
import gzip
import json
import os
import sys
import tarfile
import tempfile
from datetime import datetime


def ler_json(p):
    try:
        with open(p, encoding='utf-8') as f:
            return json.load(f)
    except (OSError, ValueError):
        return None


def iso(s):
    return datetime.fromisoformat(s).timestamp()


def pasta_do_no(base, no):
    """Pasta com os arquivos do nó (extrai o tar.gz se for o caso)."""
    tgz = os.path.join(base, f'{no}.tar.gz')
    d = os.path.join(base, no)
    if not os.path.isdir(d) and os.path.isfile(tgz):
        d = tempfile.mkdtemp(prefix=f'uv-m002-{no}-')
        with tarfile.open(tgz) as t:
            t.extractall(d, filter='data')
    if not os.path.isdir(d):
        return None
    for raiz, _, arqs in os.walk(d):
        if 'telemetria.jsonl.gz' in arqs or 'telemetria.jsonl' in arqs:
            return raiz
    return None


def telemetria(d):
    p = os.path.join(d, 'telemetria.jsonl.gz')
    abrir = gzip.open if os.path.exists(p) else open
    p = p if os.path.exists(p) else os.path.join(d, 'telemetria.jsonl')
    out = []
    with abrir(p, 'rt', encoding='utf-8') as f:
        for linha in f:
            try:
                out.append(json.loads(linha))
            except ValueError:
                pass
    return out


def aplicados_ms(d):
    """Instantes (ms) de cada BU aplicado, do secoes.log."""
    p = os.path.join(d, 'secoes.log.gz')
    if not os.path.exists(p):
        return []
    ts = []
    with gzip.open(p, 'rt', encoding='utf-8', errors='replace') as f:
        next(f, None)
        for linha in f:
            i = linha.find(',')
            j = linha.find(',', i + 1)
            if linha[i + 1:j] == 'aplicado':
                ts.append(int(linha[:i]))
    ts.sort()
    return ts


def quantil_t(ts, q):
    return ts[min(len(ts) - 1, max(0, int(round(q * len(ts))) - 1))] if ts else None


def medir_no(d):
    tel = telemetria(d)
    com_saude = [x for x in tel if x.get('saude')]
    if not com_saude:
        return None
    s0 = com_saude[0]['saude']
    t_svc = iso(s0['agora']) - s0['ativo_s']
    ts = aplicados_ms(d)
    r = {'servico_no_ar': datetime.fromtimestamp(t_svc).isoformat(timespec='seconds'), 'bus_aplicados': len(ts)}
    if not ts:
        return r
    t1, tfim = ts[0] / 1000, ts[-1] / 1000
    r['fases_s'] = {'partida_ate_1o_bu': round(t1 - t_svc, 1),
                    **{f'{int(q * 100)}%': round(quantil_t(ts, q) / 1000 - t_svc, 1) for q in (0.5, 0.9, 0.99)},
                    '100%': round(tfim - t_svc, 1), 'cauda_99_100': round(tfim - quantil_t(ts, 0.99) / 1000, 1)}
    jan = {}
    for t in ts:
        jan[t // 10000] = jan.get(t // 10000, 0) + 1
    dur = max(tfim - t1, 1e-3)
    r['vazao_bu_s'] = {'media': round(len(ts) / dur, 1), 'pico_10s': round(max(jan.values()) / 10, 1),
                       'media_ate_99': round(0.99 * len(ts) / max(quantil_t(ts, 0.99) / 1000 - t1, 1e-3), 1)}
    # CPU de cada processo entre a última amostra antes do 1º BU e a 1ª depois do último
    antes = [x for x in tel if iso(x['t']) <= t1] or tel[:1]
    depois = [x for x in tel if iso(x['t']) >= tfim] or tel[-1:]
    a, b = antes[-1], depois[0]
    wall = iso(b['t']) - iso(a['t'])
    cpu, reinicios = {}, []
    for nome, pb in (b.get('processos') or {}).items():
        pa = (a.get('processos') or {}).get(nome)
        if not pa or not pb:
            continue
        if pa.get('pid') != pb.get('pid'):
            reinicios.append(nome)
            continue
        cpu[nome] = {'cpu_s': round(pb['cpu_s'] - pa['cpu_s'], 1), 'nucleos': round((pb['cpu_s'] - pa['cpu_s']) / wall, 2) if wall > 0 else None}
    janela = [x for x in tel if iso(a['t']) <= iso(x['t']) <= iso(b['t'])]
    sis = [x['sistema']['cpu_pct'] for x in janela if (x.get('sistema') or {}).get('cpu_pct') is not None]
    r['cpu'] = {'janela_s': round(wall, 1), 'processos': cpu, 'reinicios': reinicios,
                'sistema_pct_media': round(sum(sis) / len(sis), 1) if sis else None, 'sistema_pct_max': max(sis) if sis else None}
    filas = [x['saude'].get('fila') for x in janela if x.get('saude') and x['saude'].get('fila') is not None]
    r['fila_max'] = max(filas) if filas else None
    fim = next((x['saude'] for x in tel if x.get('saude') and iso(x['t']) >= tfim), None) or ler_json(os.path.join(d, 'saude.json')) or {}
    rede = fim.get('rede') or {}
    r['rede'] = {k: rede.get(k) for k in ('requisicoes', 'cs', 'cs_304', 'aux', 'bu_baixados', 'mb', 'req_s_media', 'http429', 'erros')}
    fe = fim.get('federacao') or {}
    r['federacao'] = {k: fe.get(k) for k in ('publicados', 'recebidos', 'ja_aplicados', 'assumidas', 'delegadas_total', 'lotes_gravados', 'lotes_lidos', 'puladas')}
    if fe.get('recebidos'):
        r['federacao']['repetidos_pct'] = round(100 * (fe.get('ja_aplicados') or 0) / fe['recebidos'], 1)
    custos = {}
    for grupo in ('custos_ms', 'latencias_ms'):
        for k, v in (fim.get(grupo) or {}).items():
            if isinstance(v, dict) and v.get('n'):
                custos[k] = {'n': v['n'], 'p50': v.get('p50'), 'p99': v.get('p99'), 'max': v.get('max'),
                             'n_x_p50_s': round(v['n'] * (v.get('p50') or 0) / 1000, 1)}
    r['custos_ms'] = custos
    r['versao'] = fim.get('versao')
    return r


def texto(nome, r):
    if not r:
        return f'## {nome}\n(sem telemetria)\n'
    f, v, c = r.get('fases_s', {}), r.get('vazao_bu_s', {}), r.get('cpu', {})
    linhas = [f"## {nome} (versão {r.get('versao')}, serviço no ar {r['servico_no_ar']})",
              f"BUs aplicados: {r['bus_aplicados']} · fila máx. {r.get('fila_max')}",
              'fases (s desde o serviço no ar): ' + ', '.join(f'{k} {x}' for k, x in f.items()),
              f"vazão (BU/s): média {v.get('media')}, até 99 % {v.get('media_ate_99')}, pico em 10 s {v.get('pico_10s')}",
              f"CPU na janela de {c.get('janela_s')} s: " + ', '.join(f"{k} {x['nucleos']} núcleos ({x['cpu_s']} s)" for k, x in c.get('processos', {}).items())
              + f"; sistema {c.get('sistema_pct_media')} % (máx. {c.get('sistema_pct_max')} %)"
              + (f"; REINÍCIO: {', '.join(c['reinicios'])}" if c.get('reinicios') else ''),
              'rede: ' + ', '.join(f'{k} {x}' for k, x in r.get('rede', {}).items()),
              'federação: ' + ', '.join(f'{k} {x}' for k, x in r.get('federacao', {}).items()),
              'custos internos (n · p50 · p99 · máx ms · n×p50 s):']
    for k, x in sorted(r.get('custos_ms', {}).items(), key=lambda kv: -kv[1]['n_x_p50_s']):
        linhas.append(f"  {k}: {x['n']} · {x['p50']} · {x['p99']} · {x['max']} · {x['n_x_p50_s']}")
    return '\n'.join(linhas) + '\n'


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(2)
    base = sys.argv[1]
    res = {'execucao': os.path.basename(os.path.normpath(base))}
    for no in ('no-a', 'no-b'):
        d = pasta_do_no(base, no)
        res[no] = medir_no(d) if d else None
        if d:
            resumo = ler_json(os.path.join(d, 'RESUMO.json')) or {}
            if res[no] is not None:
                res[no]['resumo'] = resumo.get('resumo')
                res[no]['parametros'] = resumo.get('parametros')
    print(f"# {res['execucao']}\n")
    for no in ('no-a', 'no-b'):
        print(texto(no, res[no]))
    if '--json' in sys.argv:
        with open(sys.argv[sys.argv.index('--json') + 1], 'w', encoding='utf-8') as f:
            json.dump(res, f, ensure_ascii=False, indent=1)


if __name__ == '__main__':
    main()
