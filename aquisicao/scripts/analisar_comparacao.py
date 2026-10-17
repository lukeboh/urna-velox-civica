#!/usr/bin/env python3
"""Análise de um ensaio da comparação Urna Velox × TSE (etapa 7; aquisicao/scripts/ensaio_comparacao.sh).

    python3 analisar_comparacao.py ~/uvreal/ensaios/etapa7-<nome>

Critérios do handoff (docs/HANDOFF_0.7.0.md):
 1. no fim, coluna do TSE = oficial = coluna do Urna Velox nas telas das abas, posições iguais, CLS 0;
 2. rede: ≤ 1 busca por (abrangência, cargo) por intervalo (fora a conferência do fim, que baixa todos), 0 buscas da
    comparação depois que a página fechou, 0 HTTP 429; BU → publicação p99;
 4. o marco "TSE confirmou o matematicamente" tem o horário do Urna Velox igual ao do eventos.csv.
Grava RESUMO.json na pasta do ensaio e imprime o resumo.
"""
import csv, datetime, json, os, sys
from collections import defaultdict

W = sys.argv[1]
INTERVALO = float(os.environ.get("UV_TSE_INTERVALO", "30"))

def seg(s):
    """dd/mm/aaaa hh:mm:ss → segundos (hora local lida como UTC)."""
    return int(datetime.datetime.strptime(s, "%d/%m/%Y %H:%M:%S").replace(tzinfo=datetime.timezone.utc).timestamp())

# conferências pedidas pelo serviço (cada uma baixa todos os -u.json, uma vez)
confs = []
with open(os.path.join(W, "saida/servico.log"), encoding="utf-8") as f:
    for l in f:
        try:
            j = json.loads(l)
        except Exception:
            continue
        if j.get("evento") == "conferencia_pedida":
            confs.append(datetime.datetime.fromisoformat(j["t"]).timestamp())
# requisições de -u.json no simulador
req = []  # (t_real, t_site, status, arquivo)
n429 = 0
with open(os.path.join(W, "simulador.log"), encoding="utf-8") as f:
    for l in f:
        if l.startswith("#"):
            continue
        x = l.rstrip("\n").split(",")
        if len(x) < 6:
            continue
        if x[3] == "429":
            n429 += 1
        if x[2] != "oficial":
            continue
        req.append((float(x[0]) / 1000.0, seg(x[1]), int(x[3]), x[5].rsplit("/", 1)[-1]))
# cada conferência pedida baixa cada -u.json uma vez, nos segundos seguintes ao pedido. A requisição dela não se
# distingue no log do simulador; entre as do arquivo nessa janela, é a que, tirada, deixa o menor número de intervalos
# curtos (a comparação segue a própria cadência; a conferência cai no meio dela)
def curtas_de(ts):
    return sum(1 for x, y in zip(ts, ts[1:]) if y - x < 0.95 * INTERVALO)
da_conf = set()
idx_arq = defaultdict(list)
for i, (tr, ts, st, a) in enumerate(req):
    idx_arq[a].append(i)
for tc in confs:
    for a, ids in idx_arq.items():
        cand = [i for i in ids if i not in da_conf and tc - 0.002 <= req[i][0] <= tc + 5.0]
        if not cand:
            continue
        resto = [i for i in ids if i not in da_conf]
        melhor = min(cand, key=lambda c: (curtas_de([req[i][1] for i in resto if i != c]), req[c][0]))
        da_conf.add(melhor)
comp = [r for i, r in enumerate(req) if i not in da_conf]
por_arq = defaultdict(list)
for tr, ts, st, a in comp:
    por_arq[a].append(ts)
curtas = {a: sum(1 for x, y in zip(v, v[1:]) if y - x < 0.95 * INTERVALO) for a, v in por_arq.items()}
fechada = float(open(os.path.join(W, "pagina_fechada")).read()) if os.path.exists(os.path.join(W, "pagina_fechada")) else None
depois = sum(1 for tr, ts, st, a in comp if fechada and tr > fechada + 2.0)

pag = json.load(open(os.path.join(W, "pagina/resumo.json")))
abas = json.load(open(os.path.join(W, "pagina/abas.json")))
telas = sorted({a["arquivo"] for a in abas if a.get("arquivo")})
fora_da_tela = sorted(a for a in por_arq if a not in telas)
conf = json.load(open(os.path.join(W, "saida/conferencia.json")))
saude = json.load(open(os.path.join(W, "saida/saude.json")))

# critério 4: o horário do Urna Velox no marco "TSE confirmou" = o do eventos.csv
ev = defaultdict(list)
with open(os.path.join(W, "saida/eventos.csv"), encoding="utf-8") as f:
    for r in csv.DictReader(l for l in f if not l.startswith("#")):
        if r["estado"] in ("eleito", "2t") and r["candidato"]:
            ev[(r["abrangencia"], int(r["cargo"]))].append(r["horario"][11:19])
marcos = []
for c in pag.get("confirmados", []):
    topico = c["t"]
    abr, cargo = topico.split("-c")[0], int(topico.split("-c")[1])
    uv = c["uv"][11:19]
    csvh = min(ev.get((abr, cargo), ["?"]))
    marcos.append({"aba": c["aba"], "topico": topico, "uv": uv, "eventos_csv": csvh, "tse": c["tse"], "min_antes": c["min"], "bate": uv == csvh})

r = {
    "criterio_1": {"abas": pag["abas"], "conferidas": pag["conferidas"], "identicas": pag["identicas"], "cls_max": pag["cls_max"],
                   "rolagem_horizontal": pag["rolagem_horizontal"], "externas": len(pag["externas"]), "erros": pag["erros"],
                   "divergencias": {a["aba"]: a["divergencias"] for a in abas if a["divergencias"]}},
    "criterio_2": {"intervalo_s": INTERVALO, "buscas_comparacao": len(comp), "por_arquivo": {a: len(v) for a, v in por_arq.items()},
                   "curtas": {a: n for a, n in curtas.items() if n}, "arquivos_fora_das_telas": fora_da_tela,
                   "buscas_depois_de_fechar": depois, "conferencias": len(confs), "http429": n429,
                   "bu_publicacao_p99_ms": saude["latencias_ms"]["bu_publicacao"].get("p99_total"),
                   "tse_resultados": saude.get("tse_resultados")},
    "conferencia_servico": {"arquivos": conf["arquivos"], "identicos": conf["identicos"], "incompletos": conf.get("incompletos"), "faltando": conf.get("faltando")},
    "criterio_4": {"marcos": marcos, "todos_batem": all(m["bate"] for m in marcos) and bool(marcos)},
}
json.dump(r, open(os.path.join(W, "RESUMO.json"), "w"), ensure_ascii=False, indent=1)
c1, c2, c4 = r["criterio_1"], r["criterio_2"], r["criterio_4"]
print(f"1. telas: {c1['identicas']}/{c1['conferidas']} idênticas ao oficial nas duas colunas · CLS máx {c1['cls_max']} · rolagem horizontal {c1['rolagem_horizontal']} · externas {c1['externas']} · erros {len(c1['erros'])}")
print(f"2. rede: {c2['buscas_comparacao']} buscas da comparação em {len(c2['por_arquivo'])} arquivos ({', '.join(f'{a}: {n}' for a, n in sorted(c2['por_arquivo'].items()))}); "
      f"curtas {sum(c2['curtas'].values())}; fora das telas {len(c2['arquivos_fora_das_telas'])}; depois de fechar {c2['buscas_depois_de_fechar']}; 429 {c2['http429']}; "
      f"BU → publicação p99 {c2['bu_publicacao_p99_ms']} ms; fora_do_intervalo {(c2['tse_resultados'] or {}).get('fora_do_intervalo')}")
print(f"   conferência do serviço: {r['conferencia_servico']['identicos']}/{r['conferencia_servico']['arquivos']} (incompletos {r['conferencia_servico']['incompletos']})")
print(f"4. marcos 'TSE confirmou': {len(c4['marcos'])}, todos com o horário do eventos.csv: {c4['todos_batem']}")
for m in c4["marcos"]:
    print(f"   aba {m['aba']} {m['topico']}: Urna Velox {m['uv']} (eventos.csv {m['eventos_csv']}), TSE {m['tse']}, {m['min_antes']} min antes")
