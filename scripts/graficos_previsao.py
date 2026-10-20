#!/usr/bin/env python3
"""Gráficos do backtest da previsão (etapa 8): a faixa P10–P90 prevista para o "matematicamente" ao longo da noite.

    python3 scripts/graficos_previsao.py <previsoes.csv> <abrangencia> <cargo> <saida.svg> [título]

Eixo x: a hora em que a previsão foi feita; eixo y: o horário previsto. Área = P10–P90, linha = P50, tracejado verde =
horário real em que saiu o "matematicamente". SVG puro (sem dependências), claro e legível no GitHub.
"""
import csv
import sys

arq, abr, cargo, saida = sys.argv[1:5]
titulo = sys.argv[5] if len(sys.argv) > 5 else f"{abr} cargo {cargo}"
rows = [r for r in csv.DictReader(open(arq)) if r["abrangencia"] == abr and r["cargo"] == cargo and r["mat_p50"]]
if not rows:
    sys.exit(f"sem previsões do matematicamente para {abr} cargo {cargo}")
real = int(rows[0]["real_mat"]) if rows[0]["real_mat"] else None
rows = [r for r in rows if real is None or int(r["instante"]) < real]
pts = [(int(r["instante"]), int(r["mat_p10"]), int(r["mat_p50"]), int(r["mat_p90"]), float(r["apurado"])) for r in rows]
W, H, ML, MR, MT, MB = 760, 340, 64, 20, 40, 46
x0, x1 = pts[0][0], max(pts[-1][0], real or 0)
lo = min(min(p[1] for p in pts), real or 10**12)
hi = max(max(p[3] for p in pts), real or 0)
pad = max(300, (hi - lo) * 0.06)
lo, hi = lo - pad, hi + pad
X = lambda t: ML + (t - x0) / max(1, x1 - x0) * (W - ML - MR)
Y = lambda t: MT + (1 - (t - lo) / max(1, hi - lo)) * (H - MT - MB)
hm = lambda t: "%02d:%02d" % ((t % 86400) // 3600, (t % 3600) // 60)
o = [f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W} {H}" font-family="sans-serif" font-size="12">',
     f'<rect width="{W}" height="{H}" fill="#ffffff"/>',
     f'<text x="{ML}" y="22" font-size="15" font-weight="bold" fill="#1b2240">{titulo}</text>']
# grade: horas
passo = 1800 if hi - lo < 4 * 3600 else 3600
t = (int(lo) // passo + 1) * passo
while t < hi:
    o.append(f'<line x1="{ML}" x2="{W - MR}" y1="{Y(t):.1f}" y2="{Y(t):.1f}" stroke="#e3e6f0"/>')
    o.append(f'<text x="{ML - 6}" y="{Y(t) + 4:.1f}" text-anchor="end" fill="#5a6280">{hm(t)}</text>')
    t += passo
t = (x0 // passo + 1) * passo
while t < x1:
    o.append(f'<line y1="{MT}" y2="{H - MB}" x1="{X(t):.1f}" x2="{X(t):.1f}" stroke="#eef0f6"/>')
    o.append(f'<text x="{X(t):.1f}" y="{H - MB + 16}" text-anchor="middle" fill="#5a6280">{hm(t)}</text>')
    t += passo
cima = " L".join(f"{X(p[0]):.1f},{Y(p[3]):.1f}" for p in pts)
baixo = " L".join(f"{X(p[0]):.1f},{Y(p[1]):.1f}" for p in reversed(pts))
o.append(f'<path d="M{cima} L{baixo} Z" fill="#8c6bff" fill-opacity="0.25" stroke="none"/>')
o.append('<path d="M' + " L".join(f"{X(p[0]):.1f},{Y(p[2]):.1f}" for p in pts) + '" fill="none" stroke="#6a4cff" stroke-width="2"/>')
if real:
    o.append(f'<line x1="{ML}" x2="{W - MR}" y1="{Y(real):.1f}" y2="{Y(real):.1f}" stroke="#0b8a64" stroke-width="2" stroke-dasharray="6 4"/>')
    o.append(f'<text x="{W - MR - 4}" y="{Y(real) - 6:.1f}" text-anchor="end" fill="#0b8a64">saiu às {hm(real)}</text>')
# % apurado de tempos em tempos
for frac in (0.2, 0.5, 0.8):
    p = next((p for p in pts if p[4] >= frac), None)
    if p:
        o.append(f'<line x1="{X(p[0]):.1f}" x2="{X(p[0]):.1f}" y1="{MT}" y2="{H - MB}" stroke="#c9cde0" stroke-dasharray="2 3"/>')
        o.append(f'<text x="{X(p[0]) + 3:.1f}" y="{MT + 12}" fill="#7a83a6" font-size="11">{int(frac * 100)}% apurado</text>')
o.append(f'<text x="{(ML + W - MR) / 2:.0f}" y="{H - 8}" text-anchor="middle" fill="#5a6280">hora em que a previsão foi feita</text>')
o.append(f'<text transform="translate(14 {(MT + H - MB) / 2:.0f}) rotate(-90)" text-anchor="middle" fill="#5a6280">horário previsto</text>')
o.append('</svg>')
open(saida, "w").write("\n".join(o) + "\n")
print(saida, len(pts), "previsões")
