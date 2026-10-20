#!/usr/bin/env python3
"""uv-ciclo-vigia — monitor de conclusão da totalização e orquestrador da rotação horária.

Acompanha a execução atual em /srv/urna-velox/ciclo/atual:
1. Para eleições simuladas (452, 453, 3220):
   Monitora dita/saida/saude.json até apuracao.encerrada == True.
   Quando concluir, calcula a próxima hora fechada (HH:00:00).
   Se faltar menos de 3 minutos para a hora fechada, agenda para a hora seguinte (+1h).
   Mantém o resultado exibido na tela até o momento do disparo.
2. Para o pleito 3221 (espera / sem coleta):
   Permanece em exibição até a próxima hora fechada (exibindo a tela do 2º turno de 2026).
3. Na hora fechada (até INTERVALO segundos depois):
   Dispara `/opt/urna-velox/bin/uv-ciclo.sh avancar <proximo_pleito> <proximo_modo>` numa unidade transitória
   própria (systemd-run), com CICLO_CARIMBO = hora do disparo, e sai.
A decisão (próximo pleito, modo e horário) fica em rotacao.json e é retomada se o vigia reiniciar.

Playlist de simulados (uv-playlist.py, no nó A): na hora do disparo, o vigia pergunta à playlist qual é o próximo
(o mais votado pelos visitantes; sem votos, a rotação de sempre). A playlist decide uma vez por execução, e os dois nós
recebem a mesma resposta. Sem resposta da playlist em ~2 min, vale a rotação de sempre calculada aqui.
Suspensão: de um dia antes até três dias depois de cada eleição oficial (ELEICOES_OFICIAIS, padrão 2026-10-25), o
vigia não dispara nenhum simulado: o resultado no ar fica até a suspensão acabar (ou até o operador intervir).
"""
import glob
import json
import os
import subprocess
import sys
import time
import urllib.request
from datetime import date, datetime, timedelta, timezone

C = sys.argv[1] if len(sys.argv) > 1 else '/srv/urna-velox/ciclo'
NO = sys.argv[2] if len(sys.argv) > 2 else 'no'
INTERVALO = float(sys.argv[3]) if len(sys.argv) > 3 else 10.0
BR = timezone(timedelta(hours=-3))
BIN = os.environ.get('UV_BIN', '/opt/urna-velox/bin')
SYSTEMD_RUN = os.environ.get('UV_SYSTEMD_RUN', 'systemd-run')   # sobrescrito só nos testes locais

def log(msg):
    t = datetime.now(BR).isoformat()
    linha = f"{t} [{NO}] [vigia] {msg}"
    print(linha, flush=True)
    for p in [os.path.join(C, 'ciclo.log'), os.path.join(C, 'atual', 'vigia.log')]:
        try:
            with open(p, 'a', encoding='utf-8') as f:
                f.write(linha + '\n')
            break
        except OSError:
            pass

def ler_json(path):
    try:
        with open(path, encoding='utf-8') as f:
            return json.load(f)
    except Exception:
        return None

def ler_params():
    p = os.path.join(C, 'atual', 'params')
    r = {}
    try:
        for l in open(p, encoding='utf-8'):
            k, _, v = l.rstrip('\n').partition('=')
            r[k] = v
    except OSError:
        pass
    return r

def proximo_pleito(p):
    seq = {'452': '453', '453': '3220', '3220': '3221', '3221': '452'}
    return seq.get(str(p), '452')

def carregar_rotacao():
    for p in [os.path.join(C, 'rotacao.json'), os.path.join(C, 'atual', 'rotacao.json')]:
        d = ler_json(p)
        if isinstance(d, dict):
            return d
    return {'historico_modos': {}}

def salvar_rotacao(dados):
    for p in [os.path.join(C, 'rotacao.json'), os.path.join(C, 'atual', 'rotacao.json')]:
        try:
            tmp = p + '.tmp'
            with open(tmp, 'w', encoding='utf-8') as f:
                json.dump(dados, f, ensure_ascii=False, indent=2)
            os.replace(tmp, p)
            return
        except OSError as e:
            log(f"erro ao salvar {p}: {e}")

def determinar_proximo_modo(prox_p, hist_modos):
    prox_p = str(prox_p)
    if prox_p == '3221':
        return 'espera'
    # 1. Verifica no historico em memoria / rotacao.json
    ult = hist_modos.get(prox_p)
    if ult == 'rapido':
        return 'cadencia'
    if ult == 'cadencia':
        return 'rapido'
    # 2. Fallback: procurar no historico em disco
    pastas = sorted(glob.glob(os.path.join(C, 'historico', f"*-{prox_p}-*")), reverse=True)
    for p in pastas:
        base = os.path.basename(p)
        if 'rapido' in base:
            return 'cadencia'
        if 'cadencia' in base:
            return 'rapido'
    # Sem histórico: rápido (o mesmo padrão de modo_do_pleito no uv-ciclo.sh)
    return 'rapido'

def proxima_hora_fechada(agora, margem_s=180):
    base = agora.replace(minute=0, second=0, microsecond=0) + timedelta(hours=1)
    if (base - agora).total_seconds() < margem_s:
        base += timedelta(hours=1)
    return base

def executar_avancar(prox_pleito, prox_modo, disparo):
    """Dispara `uv-ciclo.sh avancar` numa unidade transitória própria.

    O avancar para a unidade uv-ciclo-vigia (parar_unidades). Com KillMode=control-group, um filho direto do vigia
    morreria junto, no meio da troca, deixando os nós parados. Em unidade própria, ele sobrevive à parada do vigia.
    O carimbo (hora do disparo) é o mesmo nos dois nós, para o ID da execução ser igual em A e B.
    """
    carimbo = disparo.strftime('%Y%m%d-%H%M')
    unidade = f"uv-ciclo-avancar-{int(time.time())}"
    cmd = [SYSTEMD_RUN, "--quiet", "--collect", f"--unit={unidade}", f"--setenv=CICLO_CARIMBO={carimbo}",
           f"{BIN}/uv-ciclo.sh", "avancar", str(prox_pleito), str(prox_modo)]
    if os.geteuid() != 0 and 'UV_SYSTEMD_RUN' not in os.environ:
        cmd = ["sudo", "-n"] + cmd
    try:
        log(f"executando transição: {' '.join(cmd)}")
        res = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, timeout=60)
        estado = "disparada" if res.returncode == 0 else "NÃO disparada"
        log(f"transição {estado} na unidade {unidade} (rc={res.returncode}) {res.stdout.strip()} {res.stderr.strip()}".rstrip())
        return res.returncode == 0
    except Exception as e:
        log(f"erro ao disparar avancar: {e}")
        return False

def ler_ciclo_env():
    r = {}
    try:
        for l in open(os.environ.get('UV_CICLO_ENV', '/etc/urna-velox/ciclo.env'), encoding='utf-8'):
            l = l.strip()
            if l and not l.startswith('#') and '=' in l:
                k, _, v = l.partition('=')
                r[k.strip()] = v.strip().strip('"').strip("'")
    except OSError:
        pass
    return r

ENV = ler_ciclo_env()

def eleicoes_oficiais():
    txt = os.environ.get('UV_ELEICOES_OFICIAIS', ENV.get('ELEICOES_OFICIAIS', '2026-10-25'))
    r = []
    for s in txt.replace(';', ',').split(','):
        try:
            r.append(date.fromisoformat(s.strip()))
        except ValueError:
            pass
    return r

def suspensao_em(t):
    """Fim da suspensão (00:00 do 4º dia depois da eleição) se `t` cai de um dia antes a três dias depois; senão None."""
    for d in eleicoes_oficiais():
        dia = datetime(d.year, d.month, d.day, tzinfo=BR)
        if dia - timedelta(days=1) <= t < dia + timedelta(days=4):
            return dia + timedelta(days=4), d
    return None

def pedir_decisao(exec_id, pleito, modo, inicio, concluido_em,
                  tentativas=int(os.environ.get('UV_PLAYLIST_TENTATIVAS', '12')),
                  espera=float(os.environ.get('UV_PLAYLIST_ESPERA', '10'))):
    """Próximo simulado segundo a playlist: (pleito, modo, origem), 'suspensa' ou None (sem playlist/sem resposta)."""
    url = os.environ.get('UV_PLAYLIST_URL', ENV.get('PLAYLIST_URL', ''))
    seg = os.environ.get('UV_PLAYLIST_SEGREDO', '')
    if not seg:
        try:
            seg = open(os.environ.get('UV_PLAYLIST_SEGREDO_ARQ', '/etc/urna-velox/playlist.segredo'), encoding='utf-8').read().strip()
        except OSError:
            seg = ''
    if not url or not seg:
        return None
    corpo = json.dumps({'id': exec_id, 'pleito': str(pleito), 'modo': str(modo or ''), 'inicio': inicio,
                        'concluido_em': concluido_em, 'no': NO}).encode()
    for i in range(tentativas):
        try:
            req = urllib.request.Request(url.rstrip('/') + '/decisao', data=corpo, method='POST',
                                         headers={'Content-Type': 'application/json', 'X-UV-Segredo': seg})
            with urllib.request.urlopen(req, timeout=8) as r:
                d = json.loads(r.read())
            if d.get('suspensa'):
                return 'suspensa'
            if str(d.get('pleito')) in ('452', '453', '3220', '3221') and d.get('modo') in ('rapido', 'cadencia', 'espera'):
                return str(d['pleito']), str(d['modo']), d.get('origem', '?') + (f" ({d.get('votos')} votos)" if d.get('origem') == 'votos' else '')
            log(f"playlist respondeu algo inesperado: {d}")
            return None
        except Exception as e:
            log(f"playlist sem resposta ({i + 1}/{tentativas}): {e}")
            if i + 1 < tentativas:
                time.sleep(espera)
    return None

def disparar(rot, exec_id, pleito, modo, inicio_iso, concluido_em, prox_pleito, prox_modo, disparo_alvo):
    """Na hora fechada: suspensão? playlist? então o avancar. Devolve True quando a transição foi disparada."""
    agora = datetime.now(BR)
    s = suspensao_em(agora)
    if s:
        if not getattr(disparar, 'avisou', False):
            log(f"playlist suspensa (eleição oficial de {s[1].strftime('%d/%m/%Y')}) até {s[0].strftime('%d/%m %H:%M')}: "
                f"nenhum simulado novo; o {pleito} segue no ar")
            disparar.avisou = True
        return 'suspensa'
    dec = pedir_decisao(exec_id, pleito, modo, inicio_iso, concluido_em)
    if dec == 'suspensa':
        log("a playlist informa suspensão: nenhum simulado novo")
        return 'suspensa'
    origem = 'rotação (sem playlist)'
    if dec:
        prox_pleito, prox_modo, origem = dec
    rot['proximo'] = {'pleito': prox_pleito, 'modo': prox_modo, 'disparo': disparo_alvo.isoformat(), 'origem': origem}
    salvar_rotacao(rot)
    log(f"próximo simulado: {prox_pleito} ({prox_modo}) · escolha: {origem}")
    return executar_avancar(prox_pleito, prox_modo, disparo_alvo)

def main():
    log(f"vigia iniciado em {C} (intervalo {INTERVALO}s, uid={os.geteuid()})")
    exec_id_path = os.path.join(C, 'atual', 'id')
    if not os.path.exists(exec_id_path):
        log("nenhuma execução atual em andamento; encerrando vigia")
        return

    exec_id = open(exec_id_path, encoding='utf-8').read().strip()
    pa = ler_params()
    pleito = pa.get('pleito')
    modo = pa.get('modo')
    inicio_iso = pa.get('inicio')
    log(f"acompanhando {exec_id} (pleito {pleito}, modo {modo})")

    rot = carregar_rotacao()
    concluido = False
    disparo_alvo = None
    prox_pleito = None
    prox_modo = None

    at = rot.get('atual') or {}
    pr = rot.get('proximo') or {}
    if at.get('id') == exec_id and at.get('concluido') and pr.get('disparo'):
        # reinício do vigia (Restart=on-failure ou 63_ciclo.sh --scripts) depois da conclusão: mantém a decisão já
        # tomada, em vez de recalcular o horário (o 3221 ganharia mais uma hora e o fim seria registrado de novo)
        try:
            disparo_alvo = datetime.fromisoformat(pr['disparo'])
            prox_pleito, prox_modo = str(pr['pleito']), str(pr['modo'])
            concluido = True
            log(f"retomando decisão salva: {prox_pleito} ({prox_modo}) às {disparo_alvo.strftime('%H:%M:%S')}")
        except (KeyError, ValueError, TypeError):
            disparo_alvo = None
    elif at.get('id') != exec_id:
        # execução nova: o rotacao.json passa a mostrar a execução em curso (o --estado mostrava a anterior)
        rot['atual'] = {'id': exec_id, 'pleito': pleito, 'modo': modo, 'inicio': inicio_iso, 'concluido': False}
        rot['proximo'] = {}
        salvar_rotacao(rot)

    while True:
        agora = datetime.now(BR)

        # Se a pasta atual mudou ou foi arquivada, encerra este vigia
        if not os.path.exists(exec_id_path):
            log(f"execução {exec_id} não está mais em atual; vigia saindo")
            break
        id_atual = open(exec_id_path, encoding='utf-8').read().strip()
        if id_atual != exec_id:
            log(f"execução mudou de {exec_id} para {id_atual}; vigia saindo")
            break

        # Caso 1: Pleito 3221 (espera / sem coleta)
        if str(pleito) == '3221':
            if not concluido:
                concluido = True
                disparo_alvo = proxima_hora_fechada(agora, margem_s=120)
                prox_pleito = proximo_pleito(pleito)
                prox_modo = determinar_proximo_modo(prox_pleito, rot.get('historico_modos', {}))
                rot['atual'] = {
                    'id': exec_id, 'pleito': pleito, 'modo': modo,
                    'inicio': inicio_iso, 'concluido': True, 'concluido_em': agora.isoformat()
                }
                rot['proximo'] = {
                    'pleito': prox_pleito, 'modo': prox_modo,
                    'disparo': disparo_alvo.isoformat()
                }
                salvar_rotacao(rot)
                minutos_espera = round((disparo_alvo - agora).total_seconds() / 60, 1)
                log(f"3221 em vitrine até {disparo_alvo.strftime('%H:%M:%S')} ({minutos_espera} min); próximo: {prox_pleito} ({prox_modo})")

            if agora >= disparo_alvo:
                log(f"hora fechada atingida ({disparo_alvo.strftime('%H:%M:%S')}); consultando a playlist (padrão: {prox_pleito} {prox_modo})")
                r = disparar(rot, exec_id, pleito, modo, inicio_iso, (rot.get('atual') or {}).get('concluido_em'),
                             prox_pleito, prox_modo, disparo_alvo)
                if r is True:
                    break
                if r == 'suspensa':
                    time.sleep(300)
                    continue
                log("disparo falhou; nova tentativa em 1 min")
                time.sleep(60)
            time.sleep(INTERVALO)
            continue

        # Caso 2: Pleitos simulados (452, 453, 3220)
        saude_path = os.path.join(C, 'atual', 'saida', 'saude.json')
        saude = ler_json(saude_path)
        encerrada = bool(saude and (saude.get('apuracao') or {}).get('encerrada'))

        if encerrada and not concluido:
            concluido = True
            concluido_em = agora
            disparo_alvo = proxima_hora_fechada(concluido_em, margem_s=180)
            prox_pleito = proximo_pleito(pleito)
            prox_modo = determinar_proximo_modo(prox_pleito, rot.get('historico_modos', {}))
            
            # Atualiza historico de modos do pleito encerrado
            hist = rot.get('historico_modos', {})
            hist[str(pleito)] = modo
            rot['historico_modos'] = hist
            rot['atual'] = {
                'id': exec_id, 'pleito': pleito, 'modo': modo,
                'inicio': inicio_iso, 'concluido': True, 'concluido_em': concluido_em.isoformat()
            }
            rot['proximo'] = {
                'pleito': prox_pleito, 'modo': prox_modo,
                'disparo': disparo_alvo.isoformat()
            }
            salvar_rotacao(rot)
            espera_min = round((disparo_alvo - agora).total_seconds() / 60, 1)
            log(f"TOTALIZAÇÃO CONCLUÍDA ({exec_id}) às {concluido_em.strftime('%H:%M:%S')}! "
                f"Resultado em exibição até {disparo_alvo.strftime('%H:%M:%S')} ({espera_min} min). "
                f"Próxima eleição: {prox_pleito} ({prox_modo})")

        if concluido and disparo_alvo:
            if agora >= disparo_alvo:
                log(f"hora fechada atingida ({disparo_alvo.strftime('%H:%M:%S')}); consultando a playlist (padrão: {prox_pleito} {prox_modo})")
                r = disparar(rot, exec_id, pleito, modo, inicio_iso, (rot.get('atual') or {}).get('concluido_em'),
                             prox_pleito, prox_modo, disparo_alvo)
                if r is True:
                    break
                if r == 'suspensa':
                    time.sleep(300)
                    continue
                log("disparo falhou; nova tentativa em 1 min")
                time.sleep(60)

        time.sleep(INTERVALO)

if __name__ == '__main__':
    main()
