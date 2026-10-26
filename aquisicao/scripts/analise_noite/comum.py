"""Funções comuns da análise de uma noite de apuração (docs/ANALISE_ATRASO_2026-10-25.md).

Pasta de um pleito (D:\\dados\\urna-velox\\pleitos\\<AAAA-MM-DD-código>):
  nos/<A|B|...>/saida/secoes.log   saída de cada nó de produção
  validacao-espelho/saida/secoes.log   instância de validação lendo um servidor espelho (opcional)
  linha-do-tempo.csv                    recebimento de cada seção (pleitos anteriores)
  analise/                              o que estes scripts gravam
"""
import csv, datetime as dt, os, re

FUSO = dt.timezone(dt.timedelta(hours=-3))


def data_do_pleito(pasta):
    """AAAA-MM-DD do nome da pasta (convenção AAAA-MM-DD-código)."""
    m = re.match(r"(\d{4})-(\d{2})-(\d{2})-\d+$", os.path.basename(os.path.normpath(pasta)))
    if not m:
        raise SystemExit(f"{pasta}: o nome da pasta do pleito deve ser AAAA-MM-DD-código")
    return dt.date(int(m[1]), int(m[2]), int(m[3]))


def instancias(pasta):
    """{nome: caminho do secoes.log}: nós de produção (A, B, ...) e, se houver, o espelho (L)."""
    out = {}
    nos = os.path.join(pasta, "nos")
    for n in sorted(os.listdir(nos)) if os.path.isdir(nos) else []:
        c = os.path.join(nos, n, "saida", "secoes.log")
        if os.path.isfile(c):
            out[n] = c
    c = os.path.join(pasta, "validacao-espelho", "saida", "secoes.log")
    if os.path.isfile(c):
        out["L"] = c
    if not out:
        raise SystemExit(f"{pasta}: nenhum secoes.log em nos/*/saida nem em validacao-espelho/saida")
    return out


def hora(s):
    if not s:
        return None
    return dt.datetime.strptime(s, "%d/%m/%Y %H:%M:%S").replace(tzinfo=FUSO).timestamp()


def ler_secoes(caminhos):
    """recebido, disponivel: {seção: epoch}; aplicado: {instância: {seção: primeiro epoch}}."""
    rec, disp, apl = {}, {}, {}
    for nome, cam in caminhos.items():
        a = apl.setdefault(nome, {})
        with open(cam, newline="") as f:
            for r in csv.DictReader(f):
                if r["acao"] != "aplicado":
                    continue
                k = (r["uf"], r["municipio"], r["zona"], r["secao"])
                t = int(r["t_ms"]) / 1000
                if k not in a or t < a[k]:
                    a[k] = t
                if r["recebido"]:
                    rec.setdefault(k, hora(r["recebido"]))
                if r["disponivel"]:
                    disp.setdefault(k, hora(r["disponivel"]))
    return rec, disp, apl


def minuto(t):
    return dt.datetime.fromtimestamp(t, FUSO).strftime("%Y-%m-%d %H:%M")


def pasta_analise(pasta):
    p = os.path.join(pasta, "analise")
    os.makedirs(p, exist_ok=True)
    return p
