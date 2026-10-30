"""uv_pleitos — os pleitos do `ele-c.json` para a playlist, o vigia e o uv-ciclo.sh (multipleito, etapa 1.1).

Lê o `pleitos.json` do `uv-coletor pleitos --acervo /srv/urna-velox/acervo` (gravado pelo `uv-pleitos.sh`, no nó A) e
o relê quando ele muda. Os pleitos encerrados com o acervo pronto (`replay`) são os da playlist e da rotação, do mais
antigo para o mais novo; os agendados e em curso dão as datas da suspensão da playlist e da "próxima eleição ao vivo".
Sem o arquivo (nó antigo, desenvolvimento), vale a lista fixa de antes (452, 453, 3220, 3221).

Também é um utilitário de linha de comando para o uv-ciclo.sh:

    python3 uv_pleitos.py info <pleito>       ciclo, nome, data, corte e cadência, uma linha "chave=valor" cada
    python3 uv_pleitos.py seguinte <pleito>   o pleito seguinte da rotação
    python3 uv_pleitos.py replays             os pleitos da rotação, um por linha
    python3 uv_pleitos.py ufs <pleito>        UFs do pleito para o --ufs ("" = todas)
    python3 uv_pleitos.py aovivo              o pleito em curso a totalizar ao vivo agora (sai com 1 se nenhum)

Só biblioteca padrão do Python.
"""
import json
import os
import sys
import threading
from datetime import date, datetime, timedelta, timezone

BR = timezone(timedelta(hours=-3))
CAMINHO = '/srv/urna-velox/pleitos/pleitos.json'
ACERVO = '/srv/urna-velox/acervo'

# a lista de antes da 1.1 (acervos montados à mão): vale sem o pleitos.json e completa o que ele não diz
FIXOS = {
    '452': {'turno': 1, 'ano': 2024, 'data': '2024-10-06', 'eleicao': 'Eleições Municipais 2024', 'nome': '1º turno de 2024',
            'ciclo': 'ele2024', 'tempo_cadencia_s': 13500, 'tipo': 'ordinaria'},
    '453': {'turno': 2, 'ano': 2024, 'data': '2024-10-27', 'eleicao': 'Eleições Municipais 2024', 'nome': '2º turno de 2024',
            'ciclo': 'ele2024', 'tempo_cadencia_s': 7200, 'tipo': 'ordinaria'},
    '3220': {'turno': 1, 'ano': 2026, 'data': '2026-10-04', 'eleicao': 'Eleições Gerais 2026', 'nome': '1º turno de 2026',
             'ciclo': 'ele2026', 'tempo_cadencia_s': 13500, 'tipo': 'ordinaria'},
    '3221': {'turno': 2, 'ano': 2026, 'data': '2026-10-25', 'eleicao': 'Eleições Gerais 2026', 'nome': '2º turno de 2026',
             'ciclo': 'ele2026', 'tempo_cadencia_s': 15300, 'tipo': 'ordinaria'},
}


def _corte(data_iso):
    """02:00 do dia seguinte ao da eleição ("dd/mm/aaaa hh:mm:ss"), como o --corte do uv-simulador."""
    d = date.fromisoformat(data_iso) + timedelta(days=1)
    return d.strftime('%d/%m/%Y') + ' 02:00:00'


def _ler_json(p):
    try:
        with open(p, encoding='utf-8') as f:
            return json.load(f)
    except Exception:
        return None


class Catalogo:
    """Os pleitos do `pleitos.json`, relido quando muda (`caminho`; padrão: $UV_PLEITOS ou CAMINHO)."""

    def __init__(self, caminho=None, acervo=None):
        self.caminho = caminho or os.environ.get('UV_PLEITOS', CAMINHO)
        self.acervo = acervo or os.environ.get('UV_ACERVO', ACERVO)
        self._trava = threading.Lock()
        self._modif = None
        self._pleitos = None   # None = sem arquivo: só os FIXOS

    def _atual(self):
        with self._trava:
            try:
                m = os.stat(self.caminho).st_mtime_ns
            except OSError:
                self._modif, self._pleitos = None, None
                return None
            if m != self._modif:
                d = _ler_json(self.caminho)
                lista = d.get('pleitos') if isinstance(d, dict) else None
                if isinstance(lista, list):
                    self._pleitos = {str(p['codigo']): p for p in lista if isinstance(p, dict) and 'codigo' in p}
                    self._modif = m
            return self._pleitos

    def tem_lista(self):
        return self._atual() is not None

    def info(self, p):
        """O que a playlist e o uv-ciclo.sh precisam de um pleito, ou None se ele não é conhecido."""
        p = str(p)
        lista = self._atual() or {}
        x, fixo = lista.get(p), FIXOS.get(p)
        # o acervo.json do acervo local completa (o uv-ciclo.sh roda nos dois nós; o B pode não ter o pleitos.json)
        ac = _ler_json(os.path.join(self.acervo, p, 'acervo.json')) or {}
        if x is None and fixo is None and not ac:
            return None
        r = dict(fixo or {})
        if x:
            eleicoes = x.get('eleicoes') or []
            ordinaria = any(e.get('tipo') == 'ordinaria' for e in eleicoes)
            titulo = x.get('titulo') or ''
            r.update({'turno': x.get('turno', r.get('turno', 1)), 'ano': x.get('ano', r.get('ano')),
                      'data': x.get('data_iso', r.get('data')), 'nome': x.get('nome') or r.get('nome'),
                      'ciclo': x.get('ciclo') or r.get('ciclo'), 'situacao': x.get('situacao'), 'replay': bool(x.get('replay')),
                      'tipo': 'ordinaria' if ordinaria else (eleicoes[0].get('tipo') if eleicoes else 'outra')})
            # "Eleições Municipais 2024 · 1º turno" → "Eleições Municipais 2024"; suplementar: o título inteiro
            r['eleicao'] = (fixo or {}).get('eleicao') or (titulo.split(' · ')[0] if ordinaria else titulo)
            ac = x.get('acervo') or {}
            if ac.get('tempo_cadencia_s'):
                r['tempo_cadencia_s'] = int(ac['tempo_cadencia_s'])
            if ac.get('secoes'):
                r['secoes'] = int(ac['secoes'])
        for k in ('ciclo', 'nome', 'tempo_cadencia_s', 'secoes'):
            if ac.get(k) and not r.get(k):
                r[k] = ac[k]
        if ac.get('data') and not r.get('data'):
            r['data'] = datetime.strptime(ac['data'], '%d/%m/%Y').date().isoformat()
        if ac.get('titulo') and not r.get('eleicao'):
            r['eleicao'] = ac['titulo']
        if not r.get('data') or not r.get('ciclo'):
            return None
        r.setdefault('turno', 1)
        r.setdefault('ano', int(r['data'][:4]))
        r.setdefault('nome', f"eleição de {date.fromisoformat(r['data']).strftime('%d/%m/%Y')}")
        r.setdefault('eleicao', r['nome'])
        r.setdefault('tipo', 'outra')
        r['corte'] = ac.get('corte') or _corte(r['data'])
        r['data_br'] = date.fromisoformat(r['data']).strftime('%d/%m/%Y')
        return r

    def replays(self):
        """Os pleitos da playlist e da rotação, do mais antigo para o mais novo: encerrados com o acervo pronto."""
        lista = self._atual()
        if lista is None:
            return list(FIXOS)
        r = [p for p, x in lista.items() if x.get('replay') and x.get('situacao') == 'encerrado']
        return sorted(r, key=lambda p: (lista[p].get('data_iso', ''), int(p)))

    def seguinte(self, p):
        """O pleito seguinte da rotação (o primeiro, se `p` não está nela)."""
        r = self.replays()
        if not r:
            return None
        p = str(p)
        if p not in r:
            # um pleito que saiu da rotação (ou nunca entrou): o primeiro depois dele na ordem das datas
            ip = self.info(p)
            if ip:
                depois = [q for q in r if (self.info(q) or {}).get('data', '') > ip['data']]
                if depois:
                    return depois[0]
            return r[0]
        return r[(r.index(p) + 1) % len(r)]

    def em_curso(self, agora=None):
        """Os pleitos em curso agora (do início do dia da eleição às 8 h do dia seguinte, como o `uv-coletor pleitos`,
        calculado aqui na hora: o pleitos.json pode ter até 15 min), ordinárias primeiro, depois o código."""
        agora = agora or datetime.now(BR)
        r = []
        for p, x in (self._atual() or {}).items():
            try:
                d = date.fromisoformat(x.get('data_iso', ''))
            except ValueError:
                continue
            ini = datetime(d.year, d.month, d.day, tzinfo=BR)
            if ini <= agora < ini + timedelta(hours=32):
                ordi = any(e.get('tipo') == 'ordinaria' for e in x.get('eleicoes') or [])
                r.append((0 if ordi else 1, int(p), p))
        return [p for _, _, p in sorted(r)]

    def ao_vivo_agora(self, agora=None):
        """O pleito a totalizar ao vivo agora (o motor totaliza um por vez), ou None."""
        e = self.em_curso(agora)
        return e[0] if e else None

    def ufs(self, p):
        """UFs do pleito para o --ufs do uv-servico ("ms", "sp,rj"); vazio = todas (pleito com "br")."""
        x = (self._atual() or {}).get(str(p)) or {}
        u = x.get('ufs') or []
        return '' if 'br' in u else ','.join(u)

    def ao_vivo(self):
        """As eleições com noite de apuração ao vivo: [(dia, ordinaria, nome)] dos pleitos agendados e em curso (e dos
        encerrados há pouco, para a janela da suspensão), em ordem de data."""
        lista = self._atual() or {}
        hoje = datetime.now(BR).date()
        r = []
        for p, x in lista.items():
            try:
                d = date.fromisoformat(x.get('data_iso', ''))
            except ValueError:
                continue
            if x.get('situacao') == 'encerrado' and (hoje - d).days > 7:
                continue
            i = self.info(p) or {}
            ordinaria = i.get('tipo') == 'ordinaria'
            nome = f"{i.get('turno', 1)}º turno das {i.get('eleicao')}" if ordinaria else (i.get('eleicao') or i.get('nome'))
            r.append((d, ordinaria, nome))
        return sorted(r)


def main(argv):
    c = Catalogo()
    if len(argv) >= 3 and argv[1] == 'info':
        i = c.info(argv[2])
        if not i:
            print(f'pleito {argv[2]} desconhecido (sem pleitos.json, sem acervo.json)', file=sys.stderr)
            return 2
        for k in ('ciclo', 'nome', 'data_br', 'corte', 'tempo_cadencia_s', 'eleicao', 'tipo'):
            print(f'{k}={i.get(k, "")}')
        return 0
    if len(argv) >= 3 and argv[1] == 'seguinte':
        s = c.seguinte(argv[2])
        if not s:
            return 2
        print(s)
        return 0
    if len(argv) >= 3 and argv[1] == 'ufs':
        print(c.ufs(argv[2]))
        return 0
    if len(argv) >= 2 and argv[1] == 'aovivo':
        p = c.ao_vivo_agora()
        if not p:
            return 1
        print(p)
        return 0
    if len(argv) >= 2 and argv[1] == 'replays':
        print('\n'.join(c.replays()))
        return 0
    print(__doc__, file=sys.stderr)
    return 2


if __name__ == '__main__':
    sys.exit(main(sys.argv))
