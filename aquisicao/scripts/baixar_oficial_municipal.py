#!/usr/bin/env python3
"""Baixa os arquivos oficiais de divulgação **por município** (Prefeito, Vereador, consultas populares).

Endereço (2024): resultados.tse.jus.br/oficial/<ciclo>/<eleição>/dados/<uf>/<uf><mun:05>-c<cargo:04>-e<eleição:06>-u.json
A lista de municípios vem de <ciclo>/<eleição>/config/mun-e<eleição:06>-cm.json (1 requisição, guardada em <dest>).

Ritmo: teto de --rps requisições por segundo (padrão 15, nunca acima de 20), sem rajadas (cada requisição
reserva o próximo horário livre), --workers requisições em andamento. Arquivos já baixados não são pedidos de
novo. **Disjuntor:** a primeira resposta HTTP 429 para tudo (esperar pelo menos 10 min antes de repetir).

    python3 baixar_oficial_municipal.py --ciclo ele2024 --eleicao 619 --cargo 11 --dest ~/uvreal/2024-1t/oficial
"""
import argparse, gzip, json, os, sys, threading, time, urllib.error, urllib.request
from concurrent.futures import ThreadPoolExecutor

UA = "urna-velox (+https://github.com/lukeboh/urna-velox-civica)"
TETO = 20.0


class Limitador:
    def __init__(self, rps):
        self.dt = 1.0 / min(rps, TETO)
        self.prox = time.monotonic()
        self.trava = threading.Lock()

    def esperar(self):
        with self.trava:
            agora = time.monotonic()
            t = max(agora, self.prox)
            self.prox = t + self.dt
        if t > agora:
            time.sleep(t - agora)


def get(url, timeout=30):
    req = urllib.request.Request(url, headers={"User-Agent": UA, "Accept-Encoding": "gzip"})
    try:
        with urllib.request.urlopen(req, timeout=timeout) as r:
            corpo = r.read()
            if r.headers.get("Content-Encoding") == "gzip":
                corpo = gzip.decompress(corpo)
            return r.status, corpo
    except urllib.error.HTTPError as e:
        return e.code, b""


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--base", default="https://resultados.tse.jus.br/oficial")
    ap.add_argument("--ciclo", default="ele2024")
    ap.add_argument("--eleicao", type=int, required=True)
    ap.add_argument("--cargo", type=int, required=True)
    ap.add_argument("--dest", required=True)
    ap.add_argument("--rps", type=float, default=15.0)
    ap.add_argument("--workers", type=int, default=8)
    ap.add_argument("--ufs", default="", help="só estas UFs (vírgula)")
    a = ap.parse_args()
    os.makedirs(a.dest, exist_ok=True)
    B = f"{a.base}/{a.ciclo}/{a.eleicao}"
    lim = Limitador(a.rps)
    cm_arq = os.path.join(a.dest, f"mun-e{a.eleicao:06d}-cm.json")
    if not os.path.exists(cm_arq):
        lim.esperar()
        st, corpo = get(f"{B}/config/mun-e{a.eleicao:06d}-cm.json")
        if st != 200:
            sys.exit(f"cm.json: HTTP {st}")
        open(cm_arq, "wb").write(corpo)
    cm = json.load(open(cm_arq, encoding="utf-8"))
    so = {x.strip().lower() for x in a.ufs.split(",") if x.strip()}
    alvos = []
    for ab in cm["abr"]:
        uf = ab["cd"].lower()
        if so and uf not in so:
            continue
        for m in ab.get("mu", []):
            nome = f"{uf}{m['cd']}-c{a.cargo:04d}-e{a.eleicao:06d}-u.json"
            if not os.path.exists(os.path.join(a.dest, nome)):
                alvos.append((uf, nome))
    print(f"eleição {a.eleicao}, cargo {a.cargo}: {sum(len(x.get('mu', [])) for x in cm['abr'])} municípios no cm.json, "
          f"{len(alvos)} a baixar, teto {min(a.rps, TETO):.0f} req/s", flush=True)
    parar = threading.Event()
    cont = {}
    trava = threading.Lock()
    t0 = time.monotonic()

    def um(alvo):
        uf, nome = alvo
        if parar.is_set():
            return
        lim.esperar()
        st, corpo = 0, b""
        for tentativa in range(4):
            try:
                st, corpo = get(f"{B}/dados/{uf}/{nome}")
            except Exception:
                st = -1
            if st in (200, 404, 429):
                break
            time.sleep(2 ** tentativa)
            lim.esperar()
        with trava:
            cont[st] = cont.get(st, 0) + 1
            n = sum(cont.values())
        if st == 429:
            parar.set()
            print(f"HTTP 429 em {nome}: parando tudo (esperar >= 10 min)", flush=True)
        elif st == 200:
            tmp = os.path.join(a.dest, nome + ".tmp")
            open(tmp, "wb").write(corpo)
            os.replace(tmp, os.path.join(a.dest, nome))
        else:
            print(f"HTTP {st} {nome}", flush=True)
        if n % 500 == 0:
            print(f"  {n}/{len(alvos)} em {time.monotonic() - t0:.0f} s ({n / (time.monotonic() - t0):.1f} req/s) {cont}", flush=True)

    with ThreadPoolExecutor(a.workers) as ex:
        list(ex.map(um, alvos))
    dt = time.monotonic() - t0
    print(f"fim: {sum(cont.values())} requisições em {dt:.0f} s ({sum(cont.values()) / max(dt, 1e-9):.1f} req/s): {cont}", flush=True)
    sys.exit(2 if parar.is_set() else 0)


if __name__ == "__main__":
    main()
