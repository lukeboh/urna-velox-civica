#!/usr/bin/env python3
"""uv-ciclo-resumo — RESUMO.json de uma execução do ciclo diário (docs/operacao/CICLO_DIARIO.md).

  uv-ciclo-resumo.py <pasta-da-execução> <pasta-do-histórico>   → JSON na saída padrão

Veredito "aprovado" quando: a apuração encerrou, a conferência com os -u.json oficiais deu tudo idêntico, nenhuma
situação divergente, nenhuma revogação do "matematicamente", nenhum BU duplicado e nenhum HTTP 429. Mede também a ordem
de totalização × recebimento no TSE (diário de seções), o tempo até encerrar e os picos da telemetria.
"""
import csv, gzip, json, os, sys
from datetime import datetime

N, H = sys.argv[1], sys.argv[2]


def jler(p):
    try:
        with open(p, encoding='utf-8') as f: return json.load(f)
    except Exception: return None


def params():
    r = {}
    try:
        for l in open(os.path.join(N, 'params'), encoding='utf-8'):
            k, _, v = l.rstrip('\n').partition('=')
            r[k] = v
    except OSError: pass
    return r


def seg(x):
    x = (x or '').strip()
    if not x: return None
    try: return float(x) if x.replace('.', '', 1).isdigit() else datetime.strptime(x, '%d/%m/%Y %H:%M:%S').timestamp()
    except ValueError:
        try: return datetime.fromisoformat(x).timestamp()
        except ValueError: return None


def pct(v, p):
    if not v: return None
    s = sorted(v); return s[min(len(s) - 1, int(round(p / 100 * (len(s) - 1))))]


def ordem():
    p = os.path.join(N, 'saida', 'secoes.log')
    if not os.path.exists(p): return None
    # B-002: o BU recebido no TSE depois do corte (dias depois da noite) entra na simulação no corte (`disponivel` antes
    # do `recebido`). Não é da noite: fica fora da medida de ordem (contado à parte). Dentro dela, um só aplicado no meio
    # empurrava o "maior até aqui" para o corte ou para dias à frente, e todo BU seguinte parecia atrasado.
    rec, tardios = [], 0
    with open(p, encoding='utf-8', errors='replace') as f:
        for r in csv.DictReader(f):
            if r.get('acao') == 'aplicado':
                t, d = seg(r.get('recebido')), seg(r.get('disponivel'))
                if t is not None and d is not None and d < t: tardios += 1; continue
                if t is not None: rec.append(t)
    if len(rec) < 2: return {'aplicados_com_recebido': len(rec), 'recebidos_depois_do_corte': tardios}
    atraso, mx, trocas = [], rec[0], 0
    for i, t in enumerate(rec):
        if i and t < rec[i - 1]: trocas += 1
        mx = max(mx, t); atraso.append(mx - t)
    pos = sorted(range(len(rec)), key=lambda i: rec[i])
    desl = [0] * len(rec)
    for alvo, i in enumerate(pos): desl[i] = abs(i - alvo)
    return {'aplicados_com_recebido': len(rec), 'recebidos_depois_do_corte': tardios,
            'atraso_recebimento_s': {'p50': pct(atraso, 50), 'p99': pct(atraso, 99), 'max': max(atraso)},
            'deslocamento_posicao': {'p50': pct(desl, 50), 'p99': pct(desl, 99), 'max': max(desl)},
            'fora_de_ordem_pct': round(100.0 * sum(1 for a in atraso if a > 0) / len(rec), 2),
            'trocas_vizinhas_pct': round(100.0 * trocas / (len(rec) - 1), 2)}


def telemetria():
    p = os.path.join(N, 'telemetria.jsonl')
    if not os.path.exists(p): return None
    r = {'amostras': 0, 'pico_rss_servico_mb': 0, 'pico_rss_simulador_mb': 0, 'pico_cpu_pct': 0, 'min_disco_livre_gb': None,
         'min_mem_disponivel_mb': None, 'pico_conexoes_8080': 0, 'pico_fila': 0, 'reinicios_servico': 0, 'reinicios_intencionais': 0,
         'encerrou_em': None, 'primeira': None}
    pid_ant = None
    # B-003: `63_ciclo.sh --reiniciar` anota a hora em atual/reinicios_intencionais antes de reiniciar; a troca de pid
    # vista na telemetria até 60 s depois de uma anotação é intencional (cada anotação desculpa uma troca só)
    marcas = []
    try:
        for l in open(os.path.join(N, 'reinicios_intencionais'), encoding='utf-8'):
            m = seg(l)
            if m is not None: marcas.append(m)
    except OSError: pass
    marcas.sort()
    for l in open(p, encoding='utf-8'):
        try: a = json.loads(l)
        except ValueError: continue
        r['amostras'] += 1; r['primeira'] = r['primeira'] or a['t']
        pr = a.get('processos') or {}
        sv = pr.get('uv-servico') or {}
        r['pico_rss_servico_mb'] = max(r['pico_rss_servico_mb'], sv.get('rss_mb') or 0)
        r['pico_rss_simulador_mb'] = max(r['pico_rss_simulador_mb'], (pr.get('uv-simulador') or {}).get('rss_mb') or 0)
        if sv.get('pid') and pid_ant and sv['pid'] != pid_ant:
            r['reinicios_servico'] += 1
            ta = seg(a.get('t'))
            m = next((m for m in marcas if ta is not None and m <= ta <= m + 60), None)
            if m is not None: marcas.remove(m); r['reinicios_intencionais'] += 1
        pid_ant = sv.get('pid') or pid_ant
        si = a.get('sistema') or {}
        r['pico_cpu_pct'] = max(r['pico_cpu_pct'], si.get('cpu_pct') or 0)
        d = si.get('disco_livre_gb'); r['min_disco_livre_gb'] = d if r['min_disco_livre_gb'] is None else min(r['min_disco_livre_gb'], d)
        m = (si.get('memoria_mb') or {}).get('MemAvailable')
        if m is not None: r['min_mem_disponivel_mb'] = m if r['min_mem_disponivel_mb'] is None else min(r['min_mem_disponivel_mb'], m)
        r['pico_conexoes_8080'] = max(r['pico_conexoes_8080'], a.get('conexoes_8080') or 0)
        s = a.get('saude') or {}
        fi = s.get('fila')
        if isinstance(fi, dict): fi = fi.get('total', fi.get('tamanho'))
        if isinstance(fi, (int, float)): r['pico_fila'] = max(r['pico_fila'], fi)
        if not r['encerrou_em'] and (s.get('apuracao') or {}).get('encerrada'): r['encerrou_em'] = a['t']
    return r


pa = params()
sa = jler(os.path.join(N, 'saida', 'saude.json')) or {}
co = jler(os.path.join(N, 'saida', 'conferencia.json'))
te = telemetria()
o = ordem()
res = {'id': open(os.path.join(N, 'id')).read().strip() if os.path.exists(os.path.join(N, 'id')) else None, 'parametros': pa,
       'estado_final': sa.get('estado'), 'encerrada': (sa.get('apuracao') or {}).get('encerrada'),
       'secoes': sa.get('secoes'), 'motor': sa.get('motor'),
       'rede': {k: (sa.get('rede') or {}).get(k) for k in ('http429', 'aux', 'bus', 'erros', 'req_s_atual')},
       'federacao': {k: (sa.get('federacao') or {}).get(k) for k in ('publicados', 'recebidos', 'ja_aplicados', 'delegadas', 'assumidas', 'rejeitados')},
       'recursos': sa.get('recursos'), 'matematicamente': sa.get('matematicamente'), 'alertas': sa.get('alertas'),
       'conferencia': None if co is None else {'identicos': co.get('identicos'), 'arquivos': co.get('arquivos'),
           'situacoes_divergentes': (co.get('situacoes') or {}).get('divergentes'), 'revogacoes': (co.get('matematicamente') or {}).get('revogacoes')},
       'ordem': o, 'telemetria': te}
ini = seg(pa.get('inicio'))
if te and te.get('encerrou_em') and ini: res['minutos_ate_encerrar'] = round((seg(te['encerrou_em']) - ini) / 60, 1)
if pa.get('modo') == 'espera':
    res['veredito'] = 'espera'
    res['resumo'] = '3221 sem coleta (candidatos carregados)'
else:
    falhas = []
    if not res['encerrada']: falhas.append('apuração não encerrou')
    obs = []
    if co is None: falhas.append('sem conferência')
    else:
        # votos × situação: a situação diverge do oficial quando o candidatos.json pré-eleição não tem a destinação
        # final (candidato depois indeferido/cassado: "destinação presumida Válido"); isso é observação, não falha
        difs = [d for x in (co.get('divergentes') or []) for d in (x.get('difs') or [])]
        votos = [d for d in difs if not d.lower().startswith('situa')]
        res['divergencias_votos'] = len(votos)
        if votos: falhas.append(f"{len(votos)} divergências de votos/totais ({co.get('identicos')}/{co.get('arquivos')} idênticos)")
        sd = (co.get('situacoes') or {}).get('divergentes')
        if sd: obs.append(f"{sd} situações divergentes (destinação presumida 'Válido')")
        if co.get('nao_finalizados'): obs.append(f"{len(co['nao_finalizados'])} oficiais não finalizados")
        if co.get('faltando'): falhas.append(f"{len(co['faltando'])} oficiais faltando")
    res['observacoes'] = obs
    if co and (co.get('matematicamente') or {}).get('revogacoes'): falhas.append('revogações')
    if (sa.get('motor') or {}).get('duplicados'): falhas.append('BUs duplicados')
    if (sa.get('rede') or {}).get('http429'): falhas.append('HTTP 429')
    if te and te.get('reinicios_intencionais'): obs.append(f"{te['reinicios_intencionais']} reinício(s) intencional(is) (63_ciclo.sh --reiniciar)")
    quedas = (te or {}).get('reinicios_servico', 0) - (te or {}).get('reinicios_intencionais', 0)
    if quedas > 0: falhas.append(f"{quedas} reinícios do serviço")
    res['veredito'] = 'aprovado' if not falhas else 'falhou'
    res['falhas'] = falhas
    c = res['conferencia'] or {}
    res['resumo'] = (f"conferência {c.get('identicos')}/{c.get('arquivos')} · aplicadas {(sa.get('secoes') or {}).get('aplicadas')} · "
                     f"encerrou em {res.get('minutos_ate_encerrar')} min · ordem × recebimento no TSE: atraso p50/p99 "
                     f"{((o or {}).get('atraso_recebimento_s') or {}).get('p50')}/{((o or {}).get('atraso_recebimento_s') or {}).get('p99')} s da noite"
                     + (f" · falhas: {'; '.join(falhas)}" if falhas else '') + (f" · obs.: {'; '.join(obs)}" if obs else ''))
print(json.dumps(res, ensure_ascii=False, indent=1))
