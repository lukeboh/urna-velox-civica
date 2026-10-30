# Análise de uma noite de apuração

Scripts da [análise do atraso de 25/10/2026](../../../docs/relatorios/ANALISE_ATRASO_2026-10-25.md). Só leem dados: nenhuma
requisição ao TSE e nada de totalização.

Entrada: a pasta do pleito no D: (`/mnt/d/dados/urna-velox/pleitos/<AAAA-MM-DD-código>`), com `nos/<nó>/saida/secoes.log`
de cada nó de produção e, se houver, `validacao-espelho/saida/secoes.log`. A data da noite vem do nome da pasta.
Saída: `<pasta>/analise/`.

| script | faz | grava |
|---|---|---|
| `curvas.py --pleito P` | por seção: recebimento no TSE, disponibilização e primeira aplicação em cada instância | `minuto_a_minuto.csv`, `percentis.json`, `marcos_pct.json` |
| `fila.py --pleito P` | fila (recebidos e não aplicados), picos e vazão máxima | `fila.csv` |
| `turnos.py --pleito P --anterior Q` | chegada ao TSE nas duas noites (a anterior pela `linha-do-tempo.csv`): picos de 1/10/60 min, marcos, pausas | `chegada_turnos.csv` |
| `graficos.py --pleito P --saida docs/img` | os três gráficos do documento (precisa do `matplotlib`) | `analise_<data>_{chegada_turnos,curvas,fila}.png` |

```bash
P=/mnt/d/dados/urna-velox/pleitos
python3 aquisicao/scripts/analise_noite/curvas.py --pleito $P/2026-10-25-3221
python3 aquisicao/scripts/analise_noite/fila.py   --pleito $P/2026-10-25-3221
python3 aquisicao/scripts/analise_noite/turnos.py --pleito $P/2026-10-25-3221 --anterior $P/2026-10-04-3220
python3 aquisicao/scripts/analise_noite/graficos.py --pleito $P/2026-10-25-3221 --saida docs/img
```

`curvas.py` lê os `secoes.log` inteiros (~140 MB cada): leva 1–2 min no WSL lendo do D:.
