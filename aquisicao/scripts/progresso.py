#!/usr/bin/env python3
"""Progresso da coleta por UF, sem rede: conta os BUs nos .uvpk (só cabeçalhos) e compara com
as seções do cs.json e com as seções totalizadas na divulgação oficial (s.st), quando baixada.

    python3 progresso.py [--dados ~/uvreal/dados] [--oficial ~/uvreal/oficial] [--log ~/uvreal/run_nacional.log] [--vigiar 60]
"""
import argparse, glob, json, os, re, struct, sys, time

UFS = "ac al am ap ba ce df es go ma mg ms mt pa pb pe pi pr rj rn ro rr rs sc se sp to zz".split()
H = os.path.expanduser


def secoes_cs(dados):
    out = {}
    for p in glob.glob(os.path.join(dados, "config", "*-cs.json")):
        uf = os.path.basename(p)[:2]
        c = json.load(open(p, encoding="utf-8"))
        out[uf] = sum(len(z.get("sec", [])) for a in c["abr"] for m in a.get("mu", []) for z in m.get("zon", []))
    return out


def secoes_oficial(oficial):
    out = {}
    for uf in UFS:
        p = os.path.join(oficial, f"{uf}-c0001-e006257-u.json")
        if os.path.exists(p):
            out[uf] = int(json.load(open(p, encoding="utf-8")).get("s", {}).get("st", 0) or 0)
    return out


def bus_por_uf(dados):
    out = {}
    for arq in glob.glob(os.path.join(dados, "*.uvpk")):
        uf = os.path.basename(arq)[:2]
        tam_arq = os.path.getsize(arq)
        vistos = set()
        with open(arq, "rb") as f:
            p = 8
            while p + 52 <= tam_arq:
                f.seek(p)
                h = f.read(52)
                tam, _pl, mu, zn, se = struct.unpack_from("<IIIHH", h, 0)
                if p + 52 + tam > tam_arq:  # registro ainda sendo gravado
                    break
                vistos.add((mu, zn, se))
                p += 52 + tam
        out[uf] = len(vistos)
    return out


def ultima_linha_log(log):
    try:
        with open(log, "rb") as f:
            f.seek(max(0, os.path.getsize(log) - 4000))
            linhas = [l for l in f.read().decode("utf-8", "replace").splitlines() if "seções (" in l or "==" in l]
        return linhas[-1].strip() if linhas else ""
    except OSError:
        return ""


def barra(frac, n=20):
    k = int(round(frac * n))
    return "█" * k + "·" * (n - k)


def relatorio(a):
    cs, of, bus = secoes_cs(a.dados), secoes_oficial(a.oficial), bus_por_uf(a.dados)
    tot_bus = tot_ref = 0
    print(f"{'UF':<3} {'BUs':>8} {'esperados':>9} {'%':>6}  {'':<20}  situação")
    for uf in UFS:
        ref = of.get(uf) or cs.get(uf, 0)  # esperados: seções totalizadas no oficial (senão, as listadas no cs.json)
        nb = bus.get(uf, 0)
        tot_bus += nb
        tot_ref += ref
        frac = nb / ref if ref else 0
        sit = "concluída" if ref and nb >= ref else ("em andamento" if nb else "aguardando")
        print(f"{uf.upper():<3} {nb:>8} {ref:>9} {100 * frac:>5.1f}%  {barra(min(frac, 1))}  {sit}")
    frac = tot_bus / tot_ref if tot_ref else 0
    print(f"{'BR':<3} {tot_bus:>8} {tot_ref:>9} {100 * frac:>5.1f}%  {barra(frac)}")
    l = ultima_linha_log(a.log)
    if l:
        print("\ncoletor:", l)
        m = re.search(r"\[\s*([\d.]+) s\] (\d+)/(\d+) seções \((\d+)/s", l)
        if m:
            t, feitas, total, taxa = float(m[1]), int(m[2]), int(m[3]), int(m[4])
            resta = (total - feitas) / max(taxa, 1)
            print(f"restam {total - feitas} seções desta execução; previsão: ~{resta / 60:.0f} min "
                  f"(término ~{time.strftime('%H:%M', time.localtime(time.time() + resta))})")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--dados", default=H("~/uvreal/dados"))
    ap.add_argument("--oficial", default=H("~/uvreal/oficial"))
    ap.add_argument("--log", default=H("~/uvreal/run_nacional.log"))
    ap.add_argument("--vigiar", type=int, default=0, help="repete a cada N segundos (0 = uma vez)")
    a = ap.parse_args()
    while True:
        if a.vigiar:
            print("\033[2J\033[H" + time.strftime("%H:%M:%S"))
        relatorio(a)
        if not a.vigiar:
            break
        time.sleep(a.vigiar)


if __name__ == "__main__":
    sys.exit(main())
