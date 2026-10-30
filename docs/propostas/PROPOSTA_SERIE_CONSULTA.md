# Proposta: Série de Consulta de Cada Pleito (a base do slider)

> **Origem:** visão do multipleito alinhada com o Luciano em 29/10/2026 ([HANDOFF_1.1.0.md](../handoffs/HANDOFF_1.1.0.md)
> §3.1); formato aprovado por ele em 29/10/2026 ("sim e pode implementar").
> **Etapa:** `1.1` · Multipleito ([ROADMAP.md](../../ROADMAP.md)).

## 1. O problema

O visitante consulta qualquer pleito encerrado, a qualquer instante da noite, com tudo o que a página mostra (contadores,
cartões, mapas, curva, marcos, "matematicamente", selos, previsão, coluna do TSE), servido **só por arquivos estáticos
atrás da CDN**, sem chegar aos nós. O handoff sugeria uma fotografia completa a cada N minutos com as diferenças entre
elas. As medidas abaixo mostram que dá para fazer mais simples.

## 2. Medidas (29/10/2026)

| o quê | medida |
|---|---|
| tópico publicado (abrangência × cargo) | 250 a 600 B (`br-c0001` do 3221: 607 B) |
| publicação inteira, comprimida | 3221 (44 arquivos): ~5 KB · 3220 (83): ~8 KB · 2024 1º turno (5.574): ~290 KB |
| mudanças por minuto da noite (hora de recebimento de cada seção) | 3220: 6.259 (UF × minuto), 100.560 (município × minuto) · 3221: 3.613 e 66.143 |
| seções com hora | 499.207 no 3220 e no 3221 (os dois turnos nacionais: as mesmas seções instaladas) |
| cauda da noite | 3220: 496.231 seções em 04/10, as últimas em 07/10 |

Estimativa do 3220: a série de resultados por UF × cargo fica em ~10 MB brutos e 1–2 MB comprimida; o mapa (seção →
minuto) em ~1,5 MB comprimido. **Um pleito inteiro cabe em poucos MB**; o visitante baixa só o tópico da tela (dezenas
de KB) e o mapa.

## 3. A proposta

**Duas camadas, como na regra de 29/10 ("guardar toda versão; a série das consultas pode ser processada depois"):**

1. **Acervo de versões (no `uv-servico`, sempre ligado).** Toda versão de todo arquivo publicado vai para
   `<saida>/serie/versoes-<partida>.jsonl.gz`, só acréscimo, uma linha por versão:
   `{"t":<hora da noite, s>,"s":<relógio do sistema, ms>,"a":"<arquivo>","j":<o JSON publicado, byte a byte>}`.
   Gzip contínuo com descarga a cada lote do gravador (legível até a última descarga mesmo numa queda); cada partida
   (retomada) abre um arquivo novo. O eixo é a **hora da noite** (o `ultimo_bu`, recebimento no TSE), não a do sistema:
   a mesma noite tem a mesma série ao vivo, no replay a 500× e na retotalização.
2. **Série de consulta (o `uv-serie`, depois ou no fim da noite).** Lê o acervo de versões e o resto da saída
   (`marcos.csv`, `eventos.csv`, `previsoes.csv`; as seções acesas vêm do próprio acervo, com a hora da noite) e grava
   os estáticos do pleito na pasta `--destino` (ex.: `/srv/urna-velox/consulta/<pleito>/serie/`):

| arquivo | conteúdo |
|---|---|
| `indice.json` | pleito, ciclo, eleições, início e fim da noite, tópicos (com nº de versões e tamanho), versão do gerador |
| `t/<tópico>.json` | `{"topico","versoes":[[t, <JSON publicado>], …]}`: **a última versão de cada minuto** da noite; a última da lista é a publicação final, byte a byte |
| `mapa-secoes.json` | a lista das seções (o mesmo corpo de `GET /api/v1/mapa/secoes`) |
| `mapa.json` | para cada índice do mapa, o minuto da noite em que a seção acendeu (0 = não acendeu), em base64 (2 bytes por seção) |
| `marcos.json`, `eventos.json`, `previsoes.json` | as linhas do `.csv` correspondente, com a hora da noite |

Cada arquivo também em `.json.gz` (a CDN/Caddy serve o pré-comprimido). Os arquivos de um pleito encerrado são
imutáveis: cache longo.

**Por que assim e não fotografia + diferença:**

* a página precisa de um tópico por vez: uma série por tópico é exatamente o que ela baixa, sem montar diferenças;
* mapa, marcos, eventos e previsões já são sequências no tempo: o estado em `t` é "tudo o que tem hora ≤ t";
* não há passo vazio: o slider mostra a versão em vigor em `t`. Nos suplementares de 1, 2 ou 3 lotes, os pontos do
  slider são os próprios lotes (o índice traz os instantes em que algo mudou);
* a curva é uma amostragem das versões: sai da própria série.

**Eixo e cauda.** O slider anda pelos instantes do índice; a cauda depois do fim da apuração da maioria (o 3220 vai até
07/10) fica no fim da escala sem esticar a noite (decisão da página, §3.4 do handoff).

**Lado do TSE (§3.2 do handoff).** Mesmo formato, em `tse/<tópico>.json`: das parciais oficiais quando houver
(`tse-parciais/`), senão a reconstrução pelo `aux.json` (marcada `"reconstrucao": true`). Fica para o passo seguinte.

**Pasta do acervo.** `--serie <dir>` no `uv-servico` (padrão `<saida>/serie`; `--sem-serie` desliga). No reinício do
`--controle-replay` a saída é apagada: a série de um encerrado sai de um replay rápido próprio do acervo
(`replay_acervo.sh`), não do replay compartilhado da playlist.

## 4. Conferência

A última versão de cada `t/<tópico>.json` tem de ser idêntica, byte a byte, ao arquivo final em `<saida>/publicacao/`,
que o `uv-servico` já confere com os `-u.json` oficiais (`conferencia.json`). O `uv-serie` recusa gerar (código 1) se
alguma não for. Teste automatizado no crate e prova com um replay de acervo (um geral e um suplementar).

## 5. Medido na implementação (`1.0.43`, 29/10/2026)

Replay rápido do acervo (`replay_acervo.sh`, 500×) com o acervo de versões ligado, e o `uv-serie` em seguida:

| | 3237 (suplementar de RR, 1.483 seções) | 3221 (2º turno de 2026, 499.207 seções) |
|---|---|---|
| acervo de versões (gzip) | 67 KB, 1.331 linhas | 11,1 MB, 132.672 linhas |
| série: tópicos, instantes | 1, 143 | 45, 209 |
| série inteira (`.json` / `.json.gz`) | 141 KB / 21 KB | 8,1 MB / 1,4 MB |
| maiores (`.json.gz`) | tópico 8,9 KB, previsões 8,2 KB | mapa 466 KB, previsões 447 KB, lista do mapa 159 KB, `br-c0001` 14 KB |
| conferência da série | 1/1 idêntico | 45/45 idênticos |
| `uv-serie` | < 1 s | 1 s |

O visitante baixa, para abrir um pleito geral, o índice, o tópico da tela (≤ 14 KB) e o mapa (~620 KB na primeira
vez). A 500× um lote do replay pode avançar mais de um minuto da noite (209 instantes para 298 minutos com chegada no
3221): para a série mais fina, gerar de um replay mais lento ou da noite ao vivo.
