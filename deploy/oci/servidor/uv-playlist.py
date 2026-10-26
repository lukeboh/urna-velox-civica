#!/usr/bin/env python3
"""uv-playlist — Playlist de Replays das Eleições (antes "de Simulados"; M-004) (docs/CICLO_DIARIO.md, seção "Playlist de simulados").

Roda só no nó A (o do uv-simulador), em 127.0.0.1:8090, atrás do Caddy (rotas /playlist e /api/v1/playlist).

* Mostra a agenda dos simulados: o que está no ar, a fila prevista (início na hora fechada e previsão de fim) e as
  seis opções votáveis (1º/2º turno de 2024 e 1º turno de 2026, cada um "tempo real" ou "mesma ordem, acelerado").
* Um voto por IP (IPv6: por /64), que a pessoa pode trocar ou retirar. O mais votado é o próximo; empate: quem chegou
  primeiro a esse número de votos. Sem votos, vale a rotação de sempre (452 → 453 → 3220 → 3221, modos alternados).
  Os votos do simulado escolhido são consumidos quando ele entra na fila de execução.
* A decisão do próximo é tomada **uma vez por execução** (`POST /api/v1/playlist/decisao`, com o segredo do ciclo),
  pelo primeiro vigia que pedir na virada da hora; o outro nó recebe a mesma resposta. Assim A e B sobem o mesmo
  simulado mesmo que votos cheguem entre os dois pedidos.
* Suspensão: de um dia antes até três dias depois de cada eleição oficial (`UV_ELEICOES_OFICIAIS`, padrão
  2026-10-25), nada é decidido nem votado; a página explica e diz quando a playlist volta.

Só biblioteca padrão do Python. Estado em `playlist.json` (gravação atômica); IPs guardados só como hash com sal.
"""
import hashlib
import hmac
import ipaddress
import json
import os
import secrets
import statistics
import sys
import threading
import time
from datetime import date, datetime, timedelta, timezone
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

BR = timezone(timedelta(hours=-3))

PLEITOS = {
    '452': {'turno': 1, 'ano': 2024, 'data': '2024-10-06', 'eleicao': 'Eleições Municipais 2024', 'nome': '1º turno de 2024'},
    '453': {'turno': 2, 'ano': 2024, 'data': '2024-10-27', 'eleicao': 'Eleições Municipais 2024', 'nome': '2º turno de 2024'},
    '3220': {'turno': 1, 'ano': 2026, 'data': '2026-10-04', 'eleicao': 'Eleições Gerais 2026', 'nome': '1º turno de 2026'},
    '3221': {'turno': 2, 'ano': 2026, 'data': '2026-10-25', 'eleicao': 'Eleições Gerais 2026', 'nome': '2º turno de 2026'},
}
MODOS = {
    'cadencia': {'nome': 'Tempo real',
                 'descricao': 'Os boletins de urna entram no mesmo ritmo e na mesma ordem em que chegaram à Justiça '
                              'Eleitoral na noite da eleição.'},
    'rapido': {'nome': 'Mesma ordem, acelerado',
               'descricao': 'Os boletins de urna entram na mesma ordem em que chegaram à Justiça Eleitoral, o mais '
                            'rápido que o Urna Velox consegue totalizar.'},
    'espera': {'nome': 'Vitrine do 2º turno de 2026',
               'descricao': 'Os candidatos do 2º turno de 2026 já carregados, à espera da apuração oficial.'},
}
VOTAVEIS = [f'{p}-{m}' for p in ('452', '453', '3220') for m in ('cadencia', 'rapido')]
SEQUENCIA = {'452': '453', '453': '3220', '3220': '3221', '3221': '452'}
# duração até a apuração encerrar (s), enquanto não houver medida (CICLO_DIARIO.md, execuções de prova de 17/10)
DURACAO_PADRAO = {'452-rapido': 480, '453-rapido': 120, '3220-rapido': 600,
                  '452-cadencia': 14100, '453-cadencia': 7380, '3220-cadencia': 14100}
MARGEM_RAPIDA_S = 180    # o vigia: encerrou a menos de 3 min da virada, fica para a hora seguinte
MARGEM_ESPERA_S = 120    # o vigia, no 3221
PARTIDA_S = 60           # o uv-ciclo.sh espera o minuto seguinte ao disparo antes de subir
MAX_FILA = 8
LIMITE_POSTS = (20, 600)  # por IP: 20 escritas a cada 10 min


def agora_br():
    return datetime.now(BR)


def proxima_hora(t, margem_s):
    base = t.replace(minute=0, second=0, microsecond=0) + timedelta(hours=1)
    if (base - t).total_seconds() < margem_s:
        base += timedelta(hours=1)
    return base


def eleicoes_oficiais(texto=None):
    texto = texto if texto is not None else os.environ.get('UV_ELEICOES_OFICIAIS', '2026-10-25')
    r = []
    for s in texto.replace(';', ',').split(','):
        s = s.strip()
        if s:
            r.append(date.fromisoformat(s))
    return sorted(r)


def janelas_suspensao(eleicoes):
    """[início, fim) de cada suspensão: 00:00 do dia anterior até 00:00 do 4º dia depois (3 dias depois inclusive)."""
    r = []
    for d in eleicoes:
        ini = datetime(d.year, d.month, d.day, tzinfo=BR) - timedelta(days=1)
        fim = datetime(d.year, d.month, d.day, tzinfo=BR) + timedelta(days=4)
        r.append((ini, fim, d))
    return r


def suspensao_em(t, eleicoes):
    for ini, fim, d in janelas_suspensao(eleicoes):
        if ini <= t < fim:
            return ini, fim, d
    return None


def fora_da_suspensao(t, eleicoes):
    """O instante, ou o fim da suspensão em que ele cai (a playlist retoma na virada desse dia)."""
    s = suspensao_em(t, eleicoes)
    return s[1] if s else t


def inicio_apuracao(texto=None):
    """Hora do início da totalização ao vivo no dia de cada eleição oficial (Brasília). Padrão 17:00."""
    texto = texto if texto is not None else os.environ.get('UV_INICIO_APURACAO', '17:00')
    h, m = (texto.strip() or '17:00').split(':')[:2]
    return int(h), int(m)


DURACAO_AO_VIVO = timedelta(hours=15)   # a noite da apuração ao vivo: das 17h às 8h do dia seguinte


def proxima_ao_vivo(agora, eleicoes, hora=None):
    """A próxima eleição oficial com totalização ao vivo (ou a que está acontecendo agora): M-004, a contagem
    regressiva da página. `ao_vivo` = a noite da apuração já começou e ainda não acabou."""
    h, m = hora or inicio_apuracao()
    for d in eleicoes:
        ini = datetime(d.year, d.month, d.day, h, m, tzinfo=BR)
        fim = ini + DURACAO_AO_VIVO
        if agora < fim:
            return {'eleicao': d.isoformat(), 'nome': nome_eleicao_oficial(d), 'inicio': iso(ini), 'fim': iso(fim),
                    'ao_vivo': ini <= agora}
    return None


def nome_eleicao_oficial(d):
    for info in PLEITOS.values():
        if info['data'] == d.isoformat():
            return f"{info['turno']}º turno das {info['eleicao']}"
    return f"eleição de {d.strftime('%d/%m/%Y')}"


def chave_ip(ip_txt, sal):
    """Hash com sal do IP (IPv6 agrupado por /64: um aparelho troca de endereço dentro do mesmo /64)."""
    try:
        ip = ipaddress.ip_address(ip_txt.strip())
        if ip.version == 6 and ip.ipv4_mapped:
            ip = ip.ipv4_mapped
        norm = str(ipaddress.ip_network(f'{ip}/64', strict=False)) if ip.version == 6 else str(ip)
    except ValueError:
        norm = ip_txt.strip()
    return hashlib.sha256((sal + '|' + norm).encode()).hexdigest()[:32]


def descrever(item):
    p, m = item.split('-', 1)
    info = PLEITOS[p]
    return {
        'item': item, 'pleito': p, 'modo': m, 'turno': info['turno'], 'ano': info['ano'],
        'eleicao': info['eleicao'], 'nome': info['nome'], 'titulo': f"{info['turno']}º turno · {info['ano']}",
        'data_eleicao': info['data'], 'modo_nome': MODOS[m]['nome'], 'modo_descricao': MODOS[m]['descricao'],
        'votavel': item in VOTAVEIS,
    }


def iso(t):
    return t.isoformat(timespec='seconds') if t else None


def ler_json(p):
    try:
        with open(p, encoding='utf-8') as f:
            return json.load(f)
    except Exception:
        return None


class Playlist:
    """Votos, decisões e agenda. Toda mudança de estado passa pelo `trava` e é gravada em `caminho` na hora."""

    def __init__(self, ciclo_dir, caminho=None, eleicoes=None, relogio=agora_br):
        self.c = ciclo_dir
        self.caminho = caminho or os.path.join(ciclo_dir, 'playlist.json')
        self.eleicoes = eleicoes if eleicoes is not None else eleicoes_oficiais()
        self.relogio = relogio
        self.trava = threading.Lock()
        self.posts = {}
        e = ler_json(self.caminho) or {}
        self.e = {
            'sal': e.get('sal') or secrets.token_hex(16),
            'votos': e.get('votos') or {},          # chave do IP -> {item, em}
            'decisoes': e.get('decisoes') or {},    # ID da execução -> {pleito, modo, origem, votos, em}
            'duracoes': e.get('duracoes') or {},    # item -> [segundos até encerrar], as últimas 10
            'tocados': e.get('tocados') or [],      # últimas decisões, para a página
        }
        self._gravar()

    # ------------------------------------------------------------------ estado em disco
    def _gravar(self):
        tmp = self.caminho + '.tmp'
        with open(tmp, 'w', encoding='utf-8') as f:
            json.dump(self.e, f, ensure_ascii=False, indent=1)
        os.replace(tmp, self.caminho)

    def _rotacao(self):
        for p in (os.path.join(self.c, 'rotacao.json'), os.path.join(self.c, 'atual', 'rotacao.json')):
            d = ler_json(p)
            if isinstance(d, dict):
                return d
        return {}

    def _params(self):
        r = {}
        try:
            with open(os.path.join(self.c, 'atual', 'params'), encoding='utf-8') as f:
                for linha in f:
                    k, _, v = linha.rstrip('\n').partition('=')
                    r[k] = v
        except OSError:
            pass
        return r

    # ------------------------------------------------------------------ regras
    def duracao(self, item):
        med = self.e['duracoes'].get(item) or []
        return int(statistics.median(med[-5:])) if med else DURACAO_PADRAO.get(item, 3600)

    def contagem(self, excluir=None):
        """item -> (votos, instante em que chegou a essa contagem)."""
        r = {}
        for v in self.e['votos'].values():
            it = v.get('item')
            if it not in VOTAVEIS or it == excluir:
                continue
            n, t = r.get(it, (0, ''))
            r[it] = (n + 1, max(t, v.get('em', '')))
        return r

    def ranking(self, excluir=None):
        """Votados, do mais para o menos votado; empate: quem chegou antes à contagem; depois, a ordem da lista."""
        c = self.contagem(excluir)
        return sorted(c, key=lambda it: (-c[it][0], c[it][1], VOTAVEIS.index(it)))

    @staticmethod
    def padrao_seguinte(pleito, hist):
        """A rotação de sempre (sem votos): pleito seguinte da sequência, modo alternado em relação à última vez."""
        p = SEQUENCIA.get(str(pleito), '452')
        if p == '3221':
            return p, 'espera'
        ult = hist.get(p)
        return p, ('rapido' if ult == 'cadencia' else 'cadencia' if ult == 'rapido' else 'rapido')

    def no_ar(self):
        pa, rot = self._params(), self._rotacao()
        p, m = pa.get('pleito'), pa.get('modo')
        if p not in PLEITOS or m not in MODOS:
            return None
        at, pr = rot.get('atual') or {}, rot.get('proximo') or {}
        try:
            inicio = datetime.fromisoformat(pa.get('inicio') or at.get('inicio'))
        except (TypeError, ValueError):
            inicio = None
        saude = ler_json(os.path.join(self.c, 'atual', 'saida', 'saude.json')) or {}
        encerrada = bool((saude.get('apuracao') or {}).get('encerrada')) or (p == '3221')
        conc = None
        if at.get('concluido') and at.get('concluido_em'):
            try:
                conc = datetime.fromisoformat(at['concluido_em'])
            except ValueError:
                conc = None
        troca = None
        if at.get('concluido') and pr.get('disparo'):
            try:
                troca = datetime.fromisoformat(pr['disparo'])
            except ValueError:
                troca = None
        item = f'{p}-{m}'
        prev_fim = conc
        if prev_fim is None and inicio is not None:
            prev_fim = inicio + timedelta(seconds=(3600 - PARTIDA_S if p == '3221' else self.duracao(item)))
        d = descrever(item)
        exec_id = None
        try:
            with open(os.path.join(self.c, 'atual', 'id'), encoding='utf-8') as f:
                exec_id = f.read().strip()
        except OSError:
            pass
        d.update({'id': exec_id,
                  'inicio': iso(inicio), 'previsao_fim': iso(prev_fim), 'encerrada': encerrada,
                  'concluido_em': iso(conc), 'proxima_troca': iso(troca),
                  'relogio_eleicao': saude.get('relogio_site')})
        return d

    def _hist(self):
        return dict(self._rotacao().get('historico_modos') or {})

    def fila(self, agora=None, no_ar=None):
        """A agenda prevista: votados primeiro (na ordem dos votos), depois a rotação de sempre."""
        agora = agora or self.relogio()
        no_ar = no_ar if no_ar is not None else self.no_ar()
        hist = self._hist()
        if no_ar:
            ult = no_ar['pleito']
            if no_ar['modo'] != 'espera':
                hist[ult] = no_ar['modo']
            if no_ar.get('proxima_troca'):
                inicio = datetime.fromisoformat(no_ar['proxima_troca'])
            else:
                fim = datetime.fromisoformat(no_ar['previsao_fim']) if no_ar.get('previsao_fim') else agora
                fim = max(fim, agora)
                inicio = proxima_hora(fim, MARGEM_ESPERA_S if ult == '3221' else MARGEM_RAPIDA_S)
        else:
            ult, inicio = '3221', proxima_hora(agora, MARGEM_RAPIDA_S)
        cont = self.contagem(excluir=no_ar['item'] if no_ar else None)
        sequencia = [(it, 'votos') for it in self.ranking(excluir=no_ar['item'] if no_ar else None)]
        r = []
        while len(r) < MAX_FILA:
            if sequencia:
                it, origem = sequencia.pop(0)
                p, m = it.split('-', 1)
            else:
                p, m = self.padrao_seguinte(ult, hist)
                it, origem = f'{p}-{m}', 'rotacao'
            inicio = fora_da_suspensao(inicio, self.eleicoes)
            if m == 'espera':
                fim = proxima_hora(inicio + timedelta(seconds=PARTIDA_S), MARGEM_ESPERA_S)
                prox = fim
            else:
                fim = inicio + timedelta(seconds=PARTIDA_S + self.duracao(it))
                prox = proxima_hora(fim, MARGEM_RAPIDA_S)
            d = descrever(it)
            d.update({'posicao': len(r) + 1, 'origem': origem, 'votos': cont.get(it, (0, ''))[0] if origem == 'votos' else 0,
                      'inicio_previsto': iso(inicio), 'fim_previsto': iso(fim),
                      'duracao_estimada_s': None if m == 'espera' else self.duracao(it)})
            r.append(d)
            ult = p
            if m != 'espera':
                hist[p] = m
            inicio = prox
        return r

    def suspensao(self, agora=None):
        agora = agora or self.relogio()
        s = suspensao_em(agora, self.eleicoes)
        if not s:
            prox = [j for j in janelas_suspensao(self.eleicoes) if j[0] > agora]
            if not prox:
                return None, None
            ini, fim, d = prox[0]
            return None, {'eleicao': d.isoformat(), 'nome': nome_eleicao_oficial(d), 'de': iso(ini), 'ate': iso(fim)}
        ini, fim, d = s
        return {'eleicao': d.isoformat(), 'nome': nome_eleicao_oficial(d), 'de': iso(ini), 'ate': iso(fim),
                'retoma_em': iso(fim)}, None

    def publico(self, chave=None):
        agora = self.relogio()
        with self.trava:
            susp, prox_susp = self.suspensao(agora)
            no_ar = self.no_ar()
            cont = self.contagem()
            opcoes = []
            for it in VOTAVEIS:
                d = descrever(it)
                d.update({'votos': cont.get(it, (0, ''))[0], 'no_ar': bool(no_ar and no_ar['item'] == it),
                          'duracao_estimada_s': self.duracao(it)})
                opcoes.append(d)
            meu = self.e['votos'].get(chave, {}).get('item') if chave else None
            return {
                'agora': iso(agora), 'suspensao': susp, 'proxima_suspensao': prox_susp,
                'proxima_ao_vivo': proxima_ao_vivo(agora, self.eleicoes),
                'no_ar': no_ar, 'fila': [] if susp else self.fila(agora, no_ar), 'opcoes': opcoes,
                'meu_voto': meu, 'total_votos': sum(n for n, _ in cont.values()),
                'tocados': self.e['tocados'][-6:][::-1],
            }

    # ------------------------------------------------------------------ escrita
    def pode_postar(self, chave):
        janela, lim = LIMITE_POSTS[1], LIMITE_POSTS[0]
        t = time.monotonic()
        lst = [x for x in self.posts.get(chave, []) if t - x < janela]
        if len(lst) >= lim:
            self.posts[chave] = lst
            return False
        lst.append(t)
        self.posts[chave] = lst
        if len(self.posts) > 50000:
            self.posts.clear()
        return True

    def votar(self, chave, item):
        """(status, mensagem). Um voto por IP: votar de novo troca o voto; item None retira."""
        agora = self.relogio()
        with self.trava:
            if suspensao_em(agora, self.eleicoes):
                return 423, 'A playlist está suspensa durante o período da eleição oficial.'
            if item is None:
                self.e['votos'].pop(chave, None)
                self._gravar()
                return 200, 'Voto retirado.'
            if item not in VOTAVEIS:
                return 400, 'Replay desconhecido.'
            na = self.no_ar()
            if na and na['item'] == item:
                return 409, 'Este replay está no ar agora: escolha outro para o próximo.'
            ant = self.e['votos'].get(chave)
            if ant and ant.get('item') == item:
                return 200, 'Você já votou neste replay.'
            self.e['votos'][chave] = {'item': item, 'em': iso(agora)}
            self._gravar()
            return 200, 'Voto registrado.' if not ant else 'Voto trocado.'

    def decidir(self, exec_id, pleito, modo, inicio=None, concluido_em=None, no=None):
        """O próximo simulado depois da execução `exec_id`: tomado uma vez; os pedidos seguintes recebem o mesmo."""
        agora = self.relogio()
        with self.trava:
            if exec_id in self.e['decisoes']:
                return dict(self.e['decisoes'][exec_id], repetida=True)
            s = suspensao_em(agora, self.eleicoes)
            if s:
                return {'suspensa': True, 'ate': iso(s[1]), 'eleicao': s[2].isoformat()}
            item_atual = f'{pleito}-{modo}'
            if item_atual in VOTAVEIS and inicio and concluido_em:
                try:
                    dur = (datetime.fromisoformat(concluido_em) - datetime.fromisoformat(inicio)).total_seconds()
                    if 0 < dur < 12 * 3600:
                        self.e['duracoes'][item_atual] = (self.e['duracoes'].get(item_atual, []) + [int(dur)])[-10:]
                except ValueError:
                    pass
            cont = self.contagem(excluir=item_atual)
            rk = self.ranking(excluir=item_atual)
            if rk:
                it = rk[0]
                p, m = it.split('-', 1)
                votos = cont[it][0]
                origem = 'votos'
                for k in [k for k, v in self.e['votos'].items() if v.get('item') == it]:
                    del self.e['votos'][k]
            else:
                hist = self._hist()
                if modo in ('rapido', 'cadencia'):
                    hist[str(pleito)] = modo
                p, m = self.padrao_seguinte(pleito, hist)
                votos, origem = 0, 'rotacao'
            dec = {'pleito': p, 'modo': m, 'origem': origem, 'votos': votos, 'em': iso(agora), 'apos': exec_id,
                   'pedido_por': no}
            self.e['decisoes'][exec_id] = dec
            for k in sorted(self.e['decisoes'], key=lambda k: self.e['decisoes'][k].get('em', ''))[:-50]:
                del self.e['decisoes'][k]
            self.e['tocados'] = (self.e['tocados'] + [dict(descrever(f'{p}-{m}'), origem=origem, votos=votos,
                                                           decidido_em=iso(agora))])[-20:]
            self._gravar()
            return dict(dec)


# ---------------------------------------------------------------------------------------------------- HTTP
def ler_segredo():
    s = os.environ.get('UV_PLAYLIST_SEGREDO')
    if s:
        return s.strip()
    try:
        with open(os.environ.get('UV_PLAYLIST_SEGREDO_ARQ', '/etc/urna-velox/playlist.segredo'), encoding='utf-8') as f:
            return f.read().strip()
    except OSError:
        return ''


def criar_tratador(pl, html_path, segredo):
    class Tratador(BaseHTTPRequestHandler):
        server_version = 'uv-playlist'
        sys_version = ''

        def log_message(self, fmt, *args):   # sem IPs no log
            sys.stderr.write(f"{agora_br().isoformat(timespec='seconds')} {self.command} {self.path.split('?')[0]} {args[1] if len(args) > 1 else ''}\n")

        def ip(self):
            # atrás do Caddy (127.0.0.1): o IP do visitante vem no X-Real-IP / X-Forwarded-For que o Caddy põe
            remoto = self.client_address[0]
            if ipaddress.ip_address(remoto).is_loopback:
                xr = self.headers.get('X-Real-IP')
                if xr:
                    return xr.strip()
                xf = self.headers.get('X-Forwarded-For')
                if xf:
                    return xf.split(',')[0].strip()
            return remoto

        def responder(self, status, corpo, tipo='application/json; charset=utf-8', cache='no-store'):
            b = corpo if isinstance(corpo, bytes) else json.dumps(corpo, ensure_ascii=False).encode()
            self.send_response(status)
            self.send_header('Content-Type', tipo)
            self.send_header('Content-Length', str(len(b)))
            self.send_header('Cache-Control', cache)
            self.end_headers()
            if self.command != 'HEAD':
                self.wfile.write(b)

        def corpo_json(self):
            n = int(self.headers.get('Content-Length') or 0)
            if n > 4096:
                return None
            if 'application/json' not in (self.headers.get('Content-Type') or ''):
                return None
            try:
                return json.loads(self.rfile.read(n) or b'{}')
            except ValueError:
                return None

        def caminho(self):
            p = self.path.split('?')[0].rstrip('/')
            return p or '/'

        def do_HEAD(self):
            self.do_GET()

        def do_GET(self):
            p = self.caminho()
            if p in ('/playlist', '/playlist/index.html'):
                try:
                    with open(html_path, 'rb') as f:
                        return self.responder(200, f.read(), 'text/html; charset=utf-8', 'public, max-age=60')
                except OSError:
                    return self.responder(500, {'erro': 'página ausente'})
            if p == '/api/v1/playlist':
                return self.responder(200, pl.publico(chave_ip(self.ip(), pl.e['sal'])))
            if p == '/api/v1/playlist/saude':
                return self.responder(200, {'ok': True})
            self.responder(404, {'erro': 'não encontrado'})

        def do_POST(self):
            p = self.caminho()
            if p == '/api/v1/playlist/voto':
                chave = chave_ip(self.ip(), pl.e['sal'])
                if not pl.pode_postar(chave):
                    return self.responder(429, {'erro': 'Muitas tentativas em pouco tempo. Tente de novo em alguns minutos.'})
                c = self.corpo_json()
                if not isinstance(c, dict) or not isinstance(c.get('item'), str):
                    return self.responder(400, {'erro': 'Pedido inválido.'})
                st, msg = pl.votar(chave, c['item'])
                corpo = pl.publico(chave)
                corpo['mensagem' if st == 200 else 'erro'] = msg
                return self.responder(st, corpo)
            if p == '/api/v1/playlist/decisao':
                dado = self.headers.get('X-UV-Segredo') or ''
                if not segredo or not hmac.compare_digest(dado.encode(), segredo.encode()):
                    return self.responder(403, {'erro': 'proibido'})
                c = self.corpo_json()
                if not isinstance(c, dict) or not c.get('id') or str(c.get('pleito')) not in PLEITOS:
                    return self.responder(400, {'erro': 'pedido inválido'})
                return self.responder(200, pl.decidir(str(c['id']), str(c['pleito']), str(c.get('modo') or ''),
                                                      c.get('inicio'), c.get('concluido_em'), c.get('no')))
            self.responder(404, {'erro': 'não encontrado'})

        def do_DELETE(self):
            if self.caminho() == '/api/v1/playlist/voto':
                chave = chave_ip(self.ip(), pl.e['sal'])
                if not pl.pode_postar(chave):
                    return self.responder(429, {'erro': 'Muitas tentativas em pouco tempo. Tente de novo em alguns minutos.'})
                st, msg = pl.votar(chave, None)
                corpo = pl.publico(chave)
                corpo['mensagem' if st == 200 else 'erro'] = msg
                return self.responder(st, corpo)
            self.responder(404, {'erro': 'não encontrado'})

    return Tratador


def main():
    import argparse
    ap = argparse.ArgumentParser(description='Playlist de Replays das Eleições (Urna Velox)')
    ap.add_argument('--ciclo', default='/srv/urna-velox/ciclo')
    ap.add_argument('--escuta', default='127.0.0.1:8090')
    ap.add_argument('--html', default=os.path.join(os.path.dirname(os.path.abspath(__file__)), 'uv-playlist.html'))
    a = ap.parse_args()
    pl = Playlist(a.ciclo)
    segredo = ler_segredo()
    if not segredo:
        print('uv-playlist: sem segredo (/etc/urna-velox/playlist.segredo): as decisões ficam desligadas; '
              'os vigias seguem a rotação de sempre', file=sys.stderr)
    host, _, porta = a.escuta.rpartition(':')
    srv = ThreadingHTTPServer((host or '127.0.0.1', int(porta)), criar_tratador(pl, a.html, segredo))
    srv.daemon_threads = True
    print(f'uv-playlist em http://{a.escuta} (ciclo {a.ciclo}; eleições oficiais: '
          f"{', '.join(d.isoformat() for d in pl.eleicoes)})", file=sys.stderr, flush=True)
    srv.serve_forever()


if __name__ == '__main__':
    main()
