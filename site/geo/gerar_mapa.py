#!/usr/bin/env python3
"""Gera site/src/mapa.json: a geografia do cartão "Mapa das seções" (site/src/mapa.js).

O servidor não sabe de geografia: ele numera as seções do catálogo (GET /api/v1/mapa/secoes) e diz quais já
acenderam. Este arquivo diz à página ONDE desenhar cada uma:

* contorno de cada UF, já projetado no plano do mapa (equiretangular com cos 15°, largura 1000);
* centro de cada município pelo código do TSE (a seção sem local de votação conhecido fica em volta dele);
* opcional, com --locais: os locais de votação com coordenada (arquivo "Eleitorado - local de votação" dos dados
  abertos do TSE, uma linha por seção): para cada local, zona, primeira seção, nº de seções e posição. A página põe a
  seção (zona z, seção s) no local da mesma zona com a maior "primeira seção" ≤ s;
* malha de pontos dos continentes (o globo) e a posição das cidades do exterior pelo nome (a UF "zz" do TSE).

Uso (fontes em docs: site/geo/README.md):
  python3 site/geo/gerar_mapa.py --municipios municipios.csv --tse-ibge municipios_brasileiros_tse.csv \
      --ufs geodata-br-states/geojson/br_states --mundo countries.geo.json [--locais eleitorado_local_votacao_2026.csv]
"""
import argparse
import csv
import glob
import json
import math
import os
import sys
import unicodedata

# plano do mapa: lon -74,2 a -28,6 (inclui Noronha), lat 5,5 a -34; x = 0..1000
LON0, LON1, LAT1, LAT0 = -74.2, -28.6, 5.5, -34.0
SPAN = LON1 - LON0
LARG = 1000.0


def px(lon, lat):
    return ((lon - LON0) * LARG / SPAN, (LAT1 - lat) * LARG / SPAN)


def dp(pts, eps):
    """Douglas-Peucker iterativo."""
    if len(pts) < 3:
        return pts
    manter = [False] * len(pts)
    manter[0] = manter[-1] = True
    pilha = [(0, len(pts) - 1)]
    while pilha:
        i, j = pilha.pop()
        a, b = pts[i], pts[j]
        dx, dy = b[0] - a[0], b[1] - a[1]
        L = math.hypot(dx, dy)
        im, dm = -1, 0.0
        for k in range(i + 1, j):
            p = pts[k]
            d = abs(dy * p[0] - dx * p[1] + b[0] * a[1] - b[1] * a[0]) / L if L > 1e-9 else math.hypot(p[0] - a[0], p[1] - a[1])
            if d > dm:
                im, dm = k, d
        if dm > eps and im > 0:
            manter[im] = True
            pilha += [(i, im), (im, j)]
    return [p for p, m in zip(pts, manter) if m]


def area(p):
    return abs(sum(p[i][0] * p[i + 1][1] - p[i + 1][0] * p[i][1] for i in range(len(p) - 1))) / 2


def norm(s):
    s = unicodedata.normalize('NFD', s or '')
    s = ''.join(c for c in s if unicodedata.category(c) != 'Mn')
    return ' '.join(s.upper().replace('-', ' ').replace("'", ' ').split())


def ufs_contornos(pasta):
    saida, centros = [], {}
    for f in sorted(glob.glob(os.path.join(pasta, '*.json'))):
        d = json.load(open(f, encoding='utf-8'))
        g = d['features'][0]['geometry']
        uf = d['features'][0]['id'].lower()
        polis = g['coordinates'] if g['type'] == 'MultiPolygon' else [g['coordinates']]
        caminho, maior = '', (0, None)
        for p in polis:
            anel = [px(*c[:2]) for c in p[0]]
            a = area(anel)
            if a < 1.5:
                continue
            if a > maior[0]:
                maior = (a, anel)
            pts = dp(anel, 0.9)
            caminho += 'M' + 'L'.join(f'{x:.1f},{y:.1f}' for x, y in pts) + 'Z'
        saida.append([uf, caminho])
        anel = maior[1]
        centros[uf] = [round(sum(x for x, _ in anel) / len(anel), 1), round(sum(y for _, y in anel) / len(anel), 1)]
    return saida, centros


def ponto_no_poligono(x, y, polis):
    for a, b, c, d, r in polis:
        if x < a or x > b or y < c or y > d:
            continue
        dentro, j = False, len(r) - 1
        for i in range(len(r)):
            xi, yi = r[i][0], r[i][1]
            xj, yj = r[j][0], r[j][1]
            if (yi > y) != (yj > y) and x < (xj - xi) * (y - yi) / (yj - yi) + xi:
                dentro = not dentro
            j = i
        if dentro:
            return True
    return False


def malha_mundo(arq, passo=1.15):
    d = json.load(open(arq, encoding='utf-8'))
    polis = []
    for f in d['features']:
        g = f['geometry']
        for p in (g['coordinates'] if g['type'] == 'MultiPolygon' else [g['coordinates']]):
            r = p[0]
            xs, ys = [c[0] for c in r], [c[1] for c in r]
            polis.append((min(xs), max(xs), min(ys), max(ys), r))
    out, lat = [], -58.0
    while lat < 84:
        n = max(1, int(360 * math.cos(math.radians(lat)) / passo))
        for i in range(n):
            lon = -180 + (i + 0.5) * 360 / n
            if ponto_no_poligono(lon, lat, polis):
                out += [round(lon * 10), round(lat * 10)]
        lat += passo
    return out


def municipios(arq_mun, arq_tse):
    ll = {}
    for r in csv.DictReader(open(arq_mun, encoding='utf-8-sig')):
        ll[r['codigo_ibge']] = (float(r['longitude']), float(r['latitude']))
    out, faltam = [], []
    for r in csv.DictReader(open(arq_tse, encoding='utf-8-sig')):
        c = ll.get(r['codigo_ibge'])
        if not c:
            faltam.append(r['codigo_tse'])
            continue
        x, y = px(*c)
        out += [int(r['codigo_tse']), round(x * 10), round(y * 10)]
    return out, faltam


def coord_ok(lat, lon):
    return lat is not None and lon is not None and lat not in (0, -1) and lon not in (0, -1) and LAT0 <= lat <= LAT1 and LON0 <= lon <= LON1


def locais(arq):
    """Arquivo do TSE (uma linha por seção, ';', latin-1): agrupa por local de votação."""
    grupos = {}
    with open(arq, encoding='latin-1', newline='') as f:
        rd = csv.DictReader(f, delimiter=';')
        for r in rd:
            try:
                mu, z, s = int(r['CD_MUNICIPIO']), int(r['NR_ZONA']), int(r['NR_SECAO'])
                loc = int(r.get('NR_LOCAL_VOTACAO') or 0)
                lat = float((r.get('NR_LATITUDE') or '0').replace(',', '.'))
                lon = float((r.get('NR_LONGITUDE') or '0').replace(',', '.'))
            except (KeyError, ValueError):
                continue
            if r.get('SG_UF', '').upper() == 'ZZ':
                continue
            g = grupos.setdefault((mu, z, loc), [lon, lat, set()])
            g[2].add(s)
    return fronteiras(grupos)


def locais_eleitoral(arq):
    """Base já agregada por local (cache do mcp-eleitoral, `locais-votacao-<ano>.v1.json`, tirada do mesmo arquivo
    "Eleitorado - local de votação" do TSE, 1º turno): {"metadados", "locais": [{uf, codigoMunicipio, zona, numero,
    latitude, longitude, secoes}, …]}. Devolve (locais no formato da página, metadados, cidades do exterior com seções)."""
    d = json.load(open(arq, encoding='utf-8'))
    grupos, zz = {}, {}
    for l in d['locais']:
        if l.get('uf', '').upper() == 'ZZ':
            zz[l['municipio']] = zz.get(l['municipio'], 0) + len(l.get('secoes') or [])
            continue
        g = grupos.setdefault((int(l['codigoMunicipio']), int(l['zona']), int(l['numero'])), [l.get('longitude'), l.get('latitude'), set()])
        g[2].update(int(s) for s in l.get('secoes') or [])
    return fronteiras(grupos), d.get('metadados', {}), zz


# Posição dos locais em unidades de 1/LOCAIS_DIV do plano (1 unidade do plano ≈ 5 km; 1/5 ≈ 0,9 km, abaixo de um
# pixel mesmo com o cartão maximizado). Local sem coordenada = (0, 0): a página põe a seção em volta do centro do
# município, como sem a base dos locais.
LOCAIS_DIV = 5


def fronteiras(grupos):
    """Formato `locais` v2 de mapa.json: por (município, zona), em ordem: mu, zona, m e m trios (Δs, x, y) — a seção
    s0 (em diferença da anterior) a partir da qual vale a posição (x, y). As seções de locais diferentes vêm intercaladas
    na numeração, então só se grava onde a posição muda; a página pega, na zona, a maior s0 ≤ seção."""
    zonas = {}
    for (mu, z, _), (lon, lat, ss) in grupos.items():
        if coord_ok(lat, lon):
            x, y = px(lon, lat)
            pos = (max(1, round(x * LOCAIS_DIV)), max(1, round(y * LOCAIS_DIV)))
        else:
            pos = (0, 0)
        zonas.setdefault((mu, z), []).extend((s, pos) for s in ss)
    out = []
    for (mu, z), v in sorted(zonas.items()):
        v.sort()
        ent, ant = [], None
        for s, p in v:
            if p != ant:
                ent.append((s, p))
                ant = p
        out += [mu, z, len(ent)]
        s_ant = 0
        for s, (x, y) in ent:
            out += [s - s_ant, x, y]
            s_ant = s
    return out


def contar_locais(loc):
    """(zonas, fronteiras, fronteiras com posição) do formato v2."""
    i, zonas, fr, com = 0, 0, 0, 0
    while i < len(loc):
        m = loc[i + 2]
        zonas += 1
        fr += m
        com += sum(1 for j in range(m) if loc[i + 3 + 3 * j + 1] or loc[i + 3 + 3 * j + 2])
        i += 3 + 3 * m
    return zonas, fr, com


# Cidades do exterior (UF "zz" do TSE) → [lat, lon]. A página casa pelo nome do município de /api/v1/nomes, sem acento
# e em maiúsculas; nomes alternativos apontam para a mesma posição. Cidade que não estiver aqui fica fora do globo (a
# página lista as que faltaram em window.__uv.mapa.semPosicao, para completar esta tabela).
EXTERIOR = {
    # Europa
    'LISBOA': (38.72, -9.14), 'PORTO': (41.15, -8.61), 'FARO': (37.02, -7.93), 'COIMBRA': (40.21, -8.43), 'BRAGA': (41.55, -8.42),
    'LONDRES': (51.51, -0.13), 'EDIMBURGO': (55.95, -3.19), 'DUBLIN': (53.35, -6.26), 'PARIS': (48.86, 2.35), 'LYON': (45.76, 4.84),
    'MARSELHA': (43.30, 5.37), 'CAIENA': (4.92, -52.31), 'SAINT GEORGES DE L OYAPOCK': (3.89, -51.80), 'SAINT GEORGES': (3.89, -51.80),
    'MADRI': (40.42, -3.70), 'MADRID': (40.42, -3.70), 'BARCELONA': (41.39, 2.17), 'ROMA': (41.90, 12.50), 'MILAO': (45.46, 9.19),
    'GENEBRA': (46.20, 6.14), 'ZURIQUE': (47.37, 8.54), 'BERNA': (46.95, 7.45), 'BERLIM': (52.52, 13.40), 'MUNIQUE': (48.14, 11.58),
    'FRANKFURT': (50.11, 8.68), 'HAMBURGO': (53.55, 9.99), 'AMSTERDA': (52.37, 4.90), 'ROTERDA': (51.92, 4.48), 'HAIA': (52.08, 4.30),
    'BRUXELAS': (50.85, 4.35), 'LUXEMBURGO': (49.61, 6.13), 'VIENA': (48.21, 16.37), 'ESTOCOLMO': (59.33, 18.07), 'OSLO': (59.91, 10.75),
    'COPENHAGUE': (55.68, 12.57), 'HELSINQUE': (60.17, 24.94), 'VARSOVIA': (52.23, 21.01), 'PRAGA': (50.08, 14.44), 'BUDAPESTE': (47.50, 19.04),
    'BUCARESTE': (44.43, 26.10), 'SOFIA': (42.70, 23.32), 'ATENAS': (37.98, 23.73), 'MOSCOU': (55.76, 37.62), 'KIEV': (50.45, 30.52),
    'KIIV': (50.45, 30.52), 'ANCARA': (39.93, 32.86), 'ISTAMBUL': (41.01, 28.98), 'LIUBLIANA': (46.06, 14.51), 'ZAGREB': (45.81, 15.98),
    'BELGRADO': (44.79, 20.45), 'BRATISLAVA': (48.15, 17.11), 'TALLINN': (59.44, 24.75), 'RIGA': (56.95, 24.11), 'VILNIUS': (54.69, 25.28),
    'REYKJAVIK': (64.15, -21.94), 'NICOSIA': (35.19, 33.38), 'VALLETTA': (35.90, 14.51), 'TIRANA': (41.33, 19.82), 'ANDORRA LA VELLA': (42.51, 1.52),
    'MONACO': (43.74, 7.42), 'SANTA SE': (41.90, 12.45), 'VATICANO': (41.90, 12.45), 'TBILISI': (41.72, 44.79), 'IEREVAN': (40.18, 44.51), 'BAKU': (40.41, 49.87),
    # América do Norte e Central
    'NOVA YORK': (40.71, -74.01), 'NOVA IORQUE': (40.71, -74.01), 'NEW YORK': (40.71, -74.01), 'NEWARK': (40.74, -74.17),
    'BOSTON': (42.36, -71.06), 'HARTFORD': (41.77, -72.67), 'MIAMI': (25.76, -80.19), 'ORLANDO': (28.54, -81.38), 'ATLANTA': (33.75, -84.39),
    'WASHINGTON': (38.91, -77.04), 'CHICAGO': (41.88, -87.63), 'HOUSTON': (29.76, -95.37), 'DALLAS': (32.78, -96.80), 'LOS ANGELES': (34.05, -118.24),
    'SAO FRANCISCO': (37.77, -122.42), 'SAN FRANCISCO': (37.77, -122.42), 'SEATTLE': (47.61, -122.33), 'DENVER': (39.74, -104.99),
    'FILADELFIA': (39.95, -75.17), 'PHILADELPHIA': (39.95, -75.17), 'TORONTO': (43.65, -79.38), 'MONTREAL': (45.50, -73.57), 'VANCOUVER': (49.28, -123.12),
    'OTTAWA': (45.42, -75.70), 'CALGARY': (51.05, -114.07), 'CIDADE DO MEXICO': (19.43, -99.13), 'MEXICO': (19.43, -99.13), 'GUATEMALA': (14.63, -90.51),
    'CIDADE DA GUATEMALA': (14.63, -90.51), 'SAN SALVADOR': (13.69, -89.22), 'TEGUCIGALPA': (14.07, -87.19), 'MANAGUA': (12.11, -86.24),
    'SAO JOSE': (9.93, -84.08), 'SAN JOSE': (9.93, -84.08), 'PANAMA': (8.98, -79.52), 'CIDADE DO PANAMA': (8.98, -79.52), 'HAVANA': (23.11, -82.37),
    'SANTO DOMINGO': (18.49, -69.93), 'SAO DOMINGOS': (18.49, -69.93), 'PORTO PRINCIPE': (18.54, -72.34), 'KINGSTON': (17.97, -76.79),
    'PORTO DE ESPANHA': (10.66, -61.51), 'BRIDGETOWN': (13.10, -59.61), 'NASSAU': (25.04, -77.35), 'BELMOPAN': (17.25, -88.77),
    'SAO JOAO': (17.12, -61.85), 'SAINT JOHN S': (17.12, -61.85), 'CASTRIES': (14.01, -60.99), 'ROSEAU': (15.30, -61.39),
    'SAINT GEORGE S': (12.06, -61.75), 'BASSETERRE': (17.30, -62.73), 'KINGSTOWN': (13.16, -61.22), 'PARAMARIBO': (5.85, -55.20),
    'GEORGETOWN': (6.80, -58.16), 'LETHEM': (3.38, -59.80),
    # América do Sul
    'BUENOS AIRES': (-34.60, -58.38), 'CORDOBA': (-31.42, -64.18), 'MENDOZA': (-32.89, -68.83), 'ROSARIO': (-32.95, -60.65),
    'PASO DE LOS LIBRES': (-29.71, -57.09), 'PUERTO IGUAZU': (-25.60, -54.57), 'MONTEVIDEU': (-34.90, -56.16), 'MONTEVIDEO': (-34.90, -56.16),
    'RIVERA': (-30.90, -55.55), 'CHUY': (-33.70, -53.46), 'ARTIGAS': (-30.40, -56.47), 'RIO BRANCO': (-32.59, -53.38), 'ASSUNCAO': (-25.26, -57.58),
    'ASUNCION': (-25.26, -57.58), 'CIUDAD DEL ESTE': (-25.51, -54.61), 'ENCARNACION': (-27.33, -55.87), 'PEDRO JUAN CABALLERO': (-22.55, -55.73),
    'SALTO DEL GUAIRA': (-24.06, -54.31), 'CONCEPCION': (-23.41, -57.43), 'SANTIAGO': (-33.45, -70.67), 'SANTIAGO DO CHILE': (-33.45, -70.67),
    'LIMA': (-12.05, -77.04), 'IQUITOS': (-3.75, -73.25), 'BOGOTA': (4.71, -74.07), 'LETICIA': (-4.22, -69.94), 'QUITO': (-0.18, -78.47),
    'GUAIAQUIL': (-2.19, -79.89), 'GUAYAQUIL': (-2.19, -79.89), 'CARACAS': (10.48, -66.90), 'SANTA ELENA DE UAIREN': (4.60, -61.11),
    'PUERTO AYACUCHO': (5.66, -67.63), 'CIUDAD GUAYANA': (8.35, -62.64), 'LA PAZ': (-16.50, -68.15), 'SANTA CRUZ DE LA SIERRA': (-17.78, -63.18),
    'SANTA CRUZ': (-17.78, -63.18), 'COCHABAMBA': (-17.39, -66.16), 'COBIJA': (-11.03, -68.77), 'GUAYARAMERIN': (-10.83, -65.36),
    'PUERTO QUIJARRO': (-17.78, -57.77), 'PUERTO SUAREZ': (-18.95, -57.80), 'RIBERALTA': (-11.01, -66.07),
    # África e Oriente Médio
    'LUANDA': (-8.84, 13.23), 'MAPUTO': (-25.97, 32.57), 'PRETORIA': (-25.75, 28.19), 'JOANESBURGO': (-26.20, 28.05), 'CIDADE DO CABO': (-33.92, 18.42),
    'PRAIA': (14.93, -23.51), 'BISSAU': (11.86, -15.60), 'SAO TOME': (0.34, 6.73), 'DACAR': (14.69, -17.44), 'ABUJA': (9.08, 7.40), 'LAGOS': (6.52, 3.38),
    'ACRA': (5.60, -0.19), 'ABIDJA': (5.36, -4.01), 'NAIROBI': (-1.29, 36.82), 'ADIS ABEBA': (9.03, 38.74), 'CAIRO': (30.04, 31.24), 'ARGEL': (36.75, 3.06),
    'RABAT': (34.02, -6.84), 'TUNIS': (36.81, 10.18), 'TRIPOLI': (32.89, 13.19), 'CARTUM': (15.50, 32.56), 'DAR ES SALAAM': (-6.79, 39.21),
    'HARARE': (-17.83, 31.05), 'LUSACA': (-15.39, 28.32), 'GABORONE': (-24.63, 25.92), 'WINDHOEK': (-22.56, 17.08), 'KINSHASA': (-4.44, 15.27),
    'BRAZZAVILLE': (-4.27, 15.28), 'LIBREVILLE': (0.42, 9.47), 'MALABO': (3.75, 8.78), 'IAUNDE': (3.85, 11.50), 'COTONOU': (6.37, 2.39),
    'LOME': (6.13, 1.22), 'OUAGADOUGOU': (12.37, -1.52), 'BAMAKO': (12.64, -8.00), 'CONACRI': (9.64, -13.58), 'FREETOWN': (8.48, -13.23),
    'MONROVIA': (6.30, -10.80), 'NOUAKCHOTT': (18.08, -15.98), 'TEL AVIV': (32.09, 34.78), 'JERUSALEM': (31.77, 35.21), 'RAMALA': (31.90, 35.20),
    'BEIRUTE': (33.89, 35.50), 'AMA': (31.95, 35.93), 'DAMASCO': (33.51, 36.28), 'BAGDA': (33.31, 44.37), 'TEERA': (35.69, 51.39), 'RIAD': (24.71, 46.68),
    'JEDA': (21.49, 39.19), 'KUWAIT': (29.38, 47.99), 'CIDADE DO KUWAIT': (29.38, 47.99), 'DOHA': (25.29, 51.53), 'ABU DHABI': (24.45, 54.38),
    'DUBAI': (25.20, 55.27), 'MASCATE': (23.59, 58.41), 'MANAMA': (26.23, 50.59),
    # Ásia e Oceania
    'TOQUIO': (35.68, 139.69), 'NAGOIA': (35.18, 136.91), 'NAGOYA': (35.18, 136.91), 'HAMAMATSU': (34.71, 137.73), 'OTA': (36.29, 139.38),
    'OIZUMI': (36.25, 139.40), 'TOYOTA': (35.08, 137.16), 'SUZUKA': (34.88, 136.58), 'GIFU': (35.42, 136.76), 'TSU': (34.73, 136.51),
    'OSAKA': (34.69, 135.50), 'KOBE': (34.69, 135.20), 'SEUL': (37.57, 126.98), 'PEQUIM': (39.90, 116.40), 'XANGAI': (31.23, 121.47),
    'CANTAO': (23.13, 113.26), 'GUANGZHOU': (23.13, 113.26), 'HONG KONG': (22.32, 114.17), 'MACAU': (22.20, 113.54), 'TAIPE': (25.03, 121.57),
    'CINGAPURA': (1.35, 103.82), 'SINGAPURA': (1.35, 103.82), 'BANGCOC': (13.76, 100.50), 'KUALA LUMPUR': (3.14, 101.69), 'JACARTA': (-6.21, 106.85),
    'MANILA': (14.60, 120.98), 'HANOI': (21.03, 105.85), 'NOVA DELHI': (28.61, 77.21), 'MUMBAI': (19.08, 72.88), 'BOMBAIM': (19.08, 72.88),
    'DACA': (23.81, 90.41), 'COLOMBO': (6.93, 79.86), 'CATMANDU': (27.72, 85.32), 'ISLAMABAD': (33.68, 73.05), 'CABUL': (34.56, 69.21),
    'ASTANA': (51.17, 71.45), 'TASHKENT': (41.30, 69.24), 'ULAN BATOR': (47.89, 106.91), 'DILI': (-8.56, 125.57), 'YANGON': (16.84, 96.17),
    'SYDNEY': (-33.87, 151.21), 'MELBOURNE': (-37.81, 144.96), 'BRISBANE': (-27.47, 153.03), 'PERTH': (-31.95, 115.86), 'CAMBERRA': (-35.28, 149.13),
    'CANBERRA': (-35.28, 149.13), 'WELLINGTON': (-41.29, 174.78), 'AUCKLAND': (-36.85, 174.76), 'PORT MORESBY': (-9.44, 147.18),
    # grafias do TSE na base de locais de votação de 2026 (as que faltavam acima)
    'RAMALLAH': (31.90, 35.20), 'BANGKOK': (13.76, 100.50), 'SARAJEVO': (43.86, 18.41), 'KINGSTON JAMAICA': (17.97, -76.79),
    'ACCRA': (5.60, -0.19), 'PORT OF SPAIN': (10.66, -61.51), 'ISLAMABADE': (33.68, 73.05), 'KATMANDU': (27.72, 85.32),
    'PYONGYANG': (39.04, 125.76), 'UAGADUGU': (12.37, -1.52), 'SAO SALVADOR': (13.69, -89.22), 'LILONGUE': (-13.96, 33.79),
    'BAREIN': (26.23, 50.59), 'DACCA': (23.81, 90.41), 'KUAITE': (29.38, 47.99), 'ST GEORGES DE LOYAPOCK': (3.89, -51.80),
    'SAINT JOHNS': (17.12, -61.85), 'TALIN': (59.44, 24.75), 'PASO LOS LIBRES': (-29.71, -57.09), 'RIADE': (24.71, 46.68),
}


def so_locais(a):
    """--so-locais: troca só os locais de votação (e a tabela do exterior) de um mapa.json já gerado, sem as fontes de
    geografia (contornos, municípios e continentes ficam como estão)."""
    d = json.load(open(a.saida, encoding='utf-8'))
    if a.locais.endswith('.json'):
        loc, meta, zz = locais_eleitoral(a.locais)
    else:
        loc, meta, zz = locais(a.locais), {}, {}
    d['locais'] = loc
    d['locais_v'] = 2
    d['locais_div'] = LOCAIS_DIV
    d['exterior'] = {norm(k): [round(v[0] * 100), round(v[1] * 100)] for k, v in EXTERIOR.items()}
    if meta:
        d['fonte_locais'] = {'ano': meta.get('ano'), 'eleicao': meta.get('dataEleicao'), 'gerado': meta.get('dataGeracao')}
    with open(a.saida, 'w', encoding='utf-8') as f:
        json.dump(d, f, ensure_ascii=False, separators=(',', ':'))
    faltam = sorted((n for n in zz if norm(n) not in d['exterior']), key=lambda n: -zz[n])
    zs, fr, com = contar_locais(loc)
    print(f'{a.saida}: locais de votação em {zs} zonas, {fr} fronteiras ({fr - com} sem coordenada), '
          f'fonte {meta or a.locais}; exterior: {len(zz)} cidades na base, {len(faltam)} fora da tabela'
          f'{": " + ", ".join(f"{n} ({zz[n]})" for n in faltam) if faltam else ""}; {os.path.getsize(a.saida) / 1024:.0f} KB')
    return 1 if faltam and a.exigir_exterior else 0


def main():
    if '--so-locais' in sys.argv:
        ap = argparse.ArgumentParser(description=so_locais.__doc__.split('\n')[0])
        ap.add_argument('--so-locais', action='store_true')
        ap.add_argument('--locais', required=True, help='locais-votacao-<ano>.v1.json (mcp-eleitoral) ou eleitorado_local_votacao_<ano>.csv')
        ap.add_argument('--saida', default=os.path.join(os.path.dirname(__file__), '..', 'src', 'mapa.json'))
        ap.add_argument('--exigir-exterior', action='store_true', help='sai com erro se alguma cidade do exterior ficar sem posição')
        sys.exit(so_locais(ap.parse_args()))
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('--municipios', required=True, help='municipios.csv (kelvins/municipios-brasileiros: codigo_ibge, latitude, longitude)')
    ap.add_argument('--tse-ibge', required=True, help='municipios_brasileiros_tse.csv (codigo_tse, codigo_ibge)')
    ap.add_argument('--ufs', required=True, help='pasta com br_<uf>.json (geodata-br-states)')
    ap.add_argument('--mundo', required=True, help='countries.geo.json (world.geo.json)')
    ap.add_argument('--locais', help='eleitorado_local_votacao_<ano>.csv do TSE (opcional)')
    ap.add_argument('--saida', default=os.path.join(os.path.dirname(__file__), '..', 'src', 'mapa.json'))
    a = ap.parse_args()
    sys.setrecursionlimit(10000)
    ufs, centros = ufs_contornos(a.ufs)
    mun, faltam = municipios(a.municipios, a.tse_ibge)
    loc = locais(a.locais) if a.locais else []
    ext = {norm(k): [round(v[0] * 100), round(v[1] * 100)] for k, v in EXTERIOR.items()}
    dados = {
        'v': 1,
        'plano': {'lon0': LON0, 'lat1': LAT1, 'span': SPAN, 'largura': LARG, 'altura': round((LAT1 - LAT0) * LARG / SPAN, 1)},
        'ufs': ufs, 'centros': centros, 'mun': mun, 'locais': loc, 'locais_v': 2, 'locais_div': LOCAIS_DIV, 'mundo': malha_mundo(a.mundo), 'exterior': ext,
    }
    with open(a.saida, 'w', encoding='utf-8') as f:
        json.dump(dados, f, ensure_ascii=False, separators=(',', ':'))
    print(f'{a.saida}: {len(ufs)} UFs, {len(mun) // 3} municípios ({len(faltam)} sem coordenada: {",".join(faltam[:10])}), '
          f'{len(loc) // 6} trechos de locais de votação, {len(dados["mundo"]) // 2} pontos do mundo, {len(ext)} nomes do exterior; '
          f'{os.path.getsize(a.saida) / 1024:.0f} KB')


if __name__ == '__main__':
    main()
