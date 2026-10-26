# Operação da noite — `uv-servico` (etapa 3)

Como preparar e rodar o serviço da noite da eleição, o que olhar enquanto ele roda, o que fazer em cada alerta e como
retomar. É o embrião do runbook do ensaio geral (etapa 9). Todos os caminhos são absolutos: repositório = `D:\git\lukeboh\urna-velox`
no Windows = `/mnt/d/git/lukeboh/urna-velox` no WSL (`Ubuntu-24.04`, usuário `dev`); dados = `/home/dev/uvreal`.

> **Regras de rede (CLAUDE.md), que o próprio serviço impõe:** contra o TSE, no máximo 100 req/s somando tudo (padrão
> 95) e, depois de **qualquer** HTTP 429, nenhuma requisição por pelo menos 10 minutos. Isso está no código
> (`aquisicao/src/coletor.rs`, `limites`/`Rede::para`) e não pode ser afrouxado por opção: só um servidor em
> `127.0.0.1`/`localhost` (o site simulado) aceita aceleração. Um processo só, um IP só. **Na noite de 25/10 e no dia 26/10,
> nada roda contra o site real sem combinar com o Luciano.** Já combinada: a busca dos arquivos de resultado do TSE
> pela comparação (etapa 7), só da tela aberta e no máximo a cada 30–60 s (regra no CLAUDE.md).

## 1. Antes da noite

1. **Compilar** (sem tocar num binário em uso):
   ```bash
   bash /mnt/d/git/lukeboh/urna-velox/aquisicao/scripts/compilar_dev.sh
   ```
   Binários em `/home/dev/uvreal/target-dev/release/` (`uv-servico`, `uv-coletor`, `uv-motor`, `uv-simulador`). Para a
   noite, copie-os para uma pasta própria (ex.: `/home/dev/uvreal/noite/bin/`), para que uma compilação durante a noite
   não troque o executável.
2. **Candidatos** (≤ 5 req/s, cache em `--fontes`). Regere perto da eleição, e de novo na véspera:
   ```bash
   /home/dev/uvreal/target-dev/release/uv-coletor preparar --pleito 3221 --saida /home/dev/uvreal/2026-2t/preparo \
     --fontes /home/dev/uvreal/2026-2t/preparo/fontes --atualizar
   ```
   * Código 0 = completo. Código 2 = `incompleto` (em 12/10: `cs.json` de SE e TO em 404; em 13/10 12:46 o de SE já
     existia — 500.111 seções — e só o de TO continuava em 404). O serviço **recusa** um
     `candidatos.json` incompleto sem `--aceitar-incompleto`; na noite, rodar incompleto significa que os BUs dessas UFs
     caem em "fora do catálogo" (alerta `fora_catalogo`).
   * `--atualizar` baixa de novo todas as fontes, inclusive os `-u.json` pré-apuração (desde a 0.9.7, B-055; até a 0.9.6 só forçava `ele-c.json` e `cs.json` e, para reler os `-u.json`, era preciso mover a pasta `fontes/u`). Na 0.9.6 e antes:
     relê-los, mova a pasta `fontes/u` antes. **Isso importou**: os de 12/10 trazem `s.ts` (seções esperadas) e eleitorado
     parciais nas abrangências por UF de Presidente (SP com 33.403 seções e 10,8 milhões de eleitores) — o serviço avisa
     com o alerta `secoes_esperadas_incoerentes` (ver §4). O eleitorado das **circunscrições** (Brasil; Governador em AC,
     AM, DF, ES, RN) estava coerente em 12/10, mas não ficou: o TSE só publicou o eleitorado do TO em 15/10 e o `candidatos.json` de 18/10 saiu com o Brasil 1.182.041 abaixo e o Governador do ES 448.654 abaixo (B-055). O serviço avisa com o alerta `eleitorado_divergente` quando a comparação com o TSE baixa um `-u.json` com outro eleitorado.
   * Destinação dos candidatos: os `-u.json` pré-apuração não trazem `dvt`; tudo é presumido "Válido" (alerta
     `destinacao_presumida` na partida). Confira cassações e indeferimentos antes da noite.
3. **Ensaio com o site simulado** (prova o binário da noite sem tocar no TSE):
   ```bash
   cd /mnt/d/git/lukeboh/urna-velox
   UV_DADOS=/home/dev/uvreal cargo test --release -p uv-aquisicao --test servico -- --ignored --test-threads 1
   ```
   (6 ensaios, ~5 min; o critério A completo e o B estão no [relatório](RELATORIO_0.3.0.md) §3).
4. **Disco**: os BUs de uma eleição geral somam ~6 GB (499 mil BUs no 1º turno de 2026, `docs/AQUISICAO.md` §4.3.1);
   o serviço alerta (`disco`) com menos de 5 GB livres na partida, e a `saude.json` mostra `recursos.disco_livre_gb`.

## 2. A noite

```bash
B=/home/dev/uvreal/target-dev/release      # ou a cópia em /home/dev/uvreal/noite/bin
N=/home/dev/uvreal/noite-3221              # pasta da noite
mkdir -p $N
nohup $B/uv-servico --pleito 3221 --ciclo ele2026 \
  --candidatos /home/dev/uvreal/2026-2t/preparo/candidatos.json \
  --dados $N/dados --saida $N/saida \
  --intervalo-cs 30 --rps 95 \
  > $N/servico.out 2> $N/servico.err &
```

* Padrões: `--base https://resultados.tse.jus.br`, `--ambiente oficial`, `--rps 95` (teto 100), `--concorrencia 64`.
* Pode começar a qualquer hora antes das 17h: antes da eleição só relê os `cs.json` (quase todos 304) e **não pede
  nenhuma seção sem disponibilização**.
* **Exterior: os BUs começam a sair de madrugada.** No 1º turno de 2026 o TSE publicou a primeira seção do exterior às
  01:49 de Brasília (Wellington) e 781 das 1.310 antes das 17h (`zz-p003220-cs.json`). O serviço não tem trava de
  horário: totaliza cada BU assim que ele aparece no `cs.json`. **No dia da eleição, ele precisa estar no ar antes da
  1h** (de preferência na véspera). A página mostra essa totalização e explica que a oficial começa às 17h (chip
  "Oficial às 17h"; B-050, 0.8.57). Nos **replays** a madrugada não é reproduzida: o `uv-simulador --piso` (padrão
  `17:00:00`) põe às 17h tudo o que saiu antes, e esses BUs entram juntos (B-049, 0.8.58).
* **Dois ritmos de releitura dos `cs.json`** (desde a v0.2.8; `saude.json` → `ritmo_cs`):
  * **ocioso** (fila vazia e nada novo no último ciclo — antes da eleição, nos intervalos da noite): um ciclo por
    `--intervalo-cs-ocioso` (padrão 1 s, mínimo 1 s contra o TSE); contra o TSE cada ciclo lê 4 UFs em rodízio, então o
    intervalo decide o gasto: 1 s = 4 req/s (cada UF relida a cada ~7 s), 2 s = 2 req/s (~14 s), 5 s = 0,8 req/s
    (~35 s). Contra o simulador cada ciclo lê todas as UFs. Percebe a primeira seção nova em segundos;
  * **trabalhando** (seções chegando ou na fila): todas as UFs a cada `--intervalo-cs` (padrão 30 s, 28 requisições por
    ciclo), para sobrar taxa para os `aux.json` e BUs. Ao achar seção nova no rodízio, o ciclo seguinte é completo e
    imediato; quando um ciclo completo não traz nada e a fila esvazia, volta a ocioso.
  * Os dois são em hora do site: contra o simulador, `--aceleracao-local` já os divide (não passe `--intervalo-cs`
    ajustado à mão). Orçamento antes da eleição, com o padrão: ~4 `cs.json` por segundo (quase todos
    304), ~14 mil requisições por hora; num teste longo contra o site real, `--intervalo-cs-ocioso 5` baixa para ~3 mil
    por hora.
* Contra o site real o serviço **não sai sozinho** depois da conferência final: continua acompanhando (um BU atrasado
  reabre a publicação). Pare com `kill <pid>` (SIGTERM) ou Ctrl+C: termina o lote, grava publicação, diário e saúde, e
  sai com código 0.
* `--conferir-a-cada 30` confere com os oficiais a cada 30 min (além do fim); `touch $N/saida/CONFERIR` força uma
  conferência na hora (o arquivo é apagado quando ela começa).
* `--sem-sondagem`: não pede o `aux.json` das seções sem disponibilização no fim da noite (ver §3, "sondagem").
* `--painel` (desde a v0.2.7): mostra no terminal a totalização ao vivo de Presidente no Brasil — seções apuradas,
  comparecimento, brancos/nulos, cada candidato com nome, votos, % e barra, o "matematicamente" no instante em que
  acontece e a situação final. `--painel sp-c0003` mostra outra abrangência/cargo (prefixo do arquivo em
  `publicacao/`). Até 4 quadros por segundo, desenhados pela thread de gravação (não atrasa o motor). Rode com
  `2> $N/servico.err` para os avisos do serviço não se misturarem ao painel; com a saída redirecionada para arquivo,
  escreve uma linha de resumo por quadro. Exemplo, o 1º turno de 2026 inteiro em ~80 s contra o site simulado:
  ```bash
  B=/home/dev/uvreal/target-dev/release; U=/home/dev/uvreal; W=/home/dev/uvreal/etapa3/manual/painel
  M="$U/metricas_multi.csv,$U/metricas_nacional.csv,$U/metricas_retomada.csv"
  rm -rf $W; mkdir -p $W
  $B/uv-simulador --dados $U/dados --cs $U/config --metricas $M --oficial $U/oficial --pleito 3220 --ciclo ele2026 \
    --acelerar 500 --horario recebido --corte "05/10/2026 02:00:00" --log $W/simulador.log --porta-arquivo $W/porta 2> $W/simulador.err &
  sleep 5
  $B/uv-servico --pleito 3220 --ciclo ele2026 --candidatos $U/preparo_2026_1t/candidatos.json --dados $W/dados --saida $W/saida \
    --base http://127.0.0.1:$(cat $W/porta) --aceleracao-local 500 --painel 2> $W/servico.err
  pkill -f "uv-simulador.*manual/painel/"
  ```

### Pastas e arquivos

| caminho | o quê |
|---|---|
| `$N/dados/<uf>.uvpk` | BUs recebidos, só de acréscimo (nunca reescritos) |
| `$N/saida/secoes.log` | **diário de seções**, só de acréscimo: `t_ms,acao,uf,municipio,zona,secao,hash,sha256,situacao,anulada,disponivel,recebido,arquivo,pos,tam,seq`; `acao` ∈ aplicado, substituido, anulado, reenvio_identico, confirmado, nao_instalada, agregada, pendente, fora_catalogo, erro_bu. Decide na retomada qual BU vale em cada seção |
| `$N/saida/publicacao/<abr>-c<cargo>-e<eleição>.json` | resultado por abrangência ([FORMATO_RESULTADO.md](FORMATO_RESULTADO.md)), gravação atômica, `seq` = BUs aplicados |
| `$N/saida/eventos.csv` | `seq,horario,abrangencia,cargo,candidato,estado` — matematicamente eleito / 2º turno, revogações (mesmo formato do `uv-motor noite`) |
| `$N/saida/saude.json` | saúde, regravada a cada 2 s (§3) |
| `$N/saida/servico.log` | log estruturado (uma linha JSON por evento; sem uma linha por BU) |
| `$N/saida/alertas.csv` | `t,tipo,texto` — os mesmos alertas do log, um por linha |
| `$N/saida/oficial/`, `conferencia.md`, `conferencia.json` | `-u.json` oficiais baixados (≤ 5 req/s) e o resultado da conferência |

## 3. O que olhar na `saude.json`

```bash
python3 -c "import json;s=json.load(open('$N/saida/saude.json'));[print(k,json.dumps(s[k],ensure_ascii=False)) for k in ['estado','relogio_site','secoes','fila','rede','latencias_ms','alertas','matematicamente','apuracao','recursos']]"
```

| campo | normal | atenção |
|---|---|---|
| `estado` | `acompanhando` → `encerrada` | `parado` sem você ter parado = o processo saiu (veja `servico.err`) |
| `relogio_site` | hora do `Date` do site, avançando | parado há minutos = o site não responde (ver `rede.erros`, `cs_erros`) |
| `secoes.publicadas_cs` × `aplicadas` | `aplicadas` acompanha `publicadas_cs` com atraso de segundos | diferença crescendo com `fila` alta = coleta atrás do site |
| `fila` | 0 a poucas centenas no pico | crescendo sem parar = taxa insuficiente (o teto é 95–100 req/s: ~47 seções/s) |
| `rede.req_s_atual` | ≤ 95 | — (o teto está no código) |
| `rede.http429`, `pausas_429`, `pausa_restante_s`, `retomada_em` | 0 | **qualquer 429**: o serviço pára todas as requisições por 10 min sozinho; não reinicie, não "ajude" |
| `latencias_ms.bu_publicacao` | p50 < 1 ms no simulador; na noite, o tempo do motor + espera do lote (`p50`/`p99` = últimas 50 mil; `p50_total`/`p99_total` = execução inteira) | p99 de segundos = publicação lenta (ver `custos_ms`) |
| `custos_ms` | `lote`, `matematicamente`, `publicacao` (gerar os JSON), `gravacao`, `saude`, `verificacao`, `sincronia_disco` (fora da thread do motor) | `lote` ou `saude` com p99 alto atrasam os BUs; `sincronia_disco` alto = disco lento (não atrasa o motor, mas reduz a proteção contra falta de energia) |
| `secoes.pendentes` | 0 | > 0: seções que não se resolveram (ver alertas `aux_404`, `erro_rede`, `situacao_desconhecida`) |
| `matematicamente.revogacoes` | 0 | > 0: alerta `revogacao` com o motivo |
| `motor.duplicados`, `motor.erros` | 0 | > 0: investigar (BU ilegível, duplicado) |
| `apuracao.abrangencias_completas` / `esperadas` | sobe até o total; então `encerrada` e conferência | ver `secoes_esperadas_incoerentes` |
| `recursos.rss_mb`, `cpu_pct_medio`, `disco_livre_gb` | ~130–200 MB, < 30 % de um núcleo | disco < 5 GB |

**Sondagem (não instaladas).** Seções não instaladas não têm disponibilização no `cs.json` (41 no exterior no 1º turno
de 2026, com 423 eleitores: `s.sni` e `e.esni` do oficial; nenhuma em 2024), mas o TSE as conta em `s.st`. Regra
(16/10/2026): enquanto o TSE não disser que a seção não foi instalada, ela continua esperada; quando o `aux.json` dela
disser "Nao instalada", sai das que faltam e conta como totalizada, como no oficial — a qualquer hora da noite. Por UF,
quando toda seção com disponibilização da UF já está resolvida e faltam poucas para o `s.ts` (exterior: ≤ 5 % ou 50;
Brasil: ≤ 50), o serviço pede o `aux.json` das seções da UF ainda sem BU, **fora as agregadas** (`nsp` no `cs.json`,
que não têm `aux.json`), cada uma de novo a cada 5 min (hora do site) enquanto não se resolver, ≤ 2 req/s, ≤ 1.500
seções por UF. 404 ou "sem BU" mantêm a seção esperada. No 1º turno de 2026 isso são as 41 do exterior (antes da
`0.6.10`, ~1.400 pedidos, quase todos 404 de agregadas, e só depois de 30 min sem BU). Sem `nsp` no `cs.json` de uma UF
(arquivo antigo), vale o comportamento anterior: só o exterior, uma vez cada, 404 = agregada. É a única exceção a
"seção sem disponibilização não gera requisição"; `saude.json`: `secoes.sondadas`, `agregadas`, `nao_instaladas`.

**Comparação com o TSE (etapa 7, `0.6.11`).** Com `--http`, a página (ou qualquer cliente) pode pedir a totalização do
TSE ao lado da do Urna Velox: a inscrição `tse:<tópico>` (`docs/PROTOCOLO_WS.md` §4.6) faz o serviço buscar o
`-u.json` daquele tópico no site do TSE, no máximo uma vez a cada `--tse-intervalo` (30 s; de 30 a 60, hora do site),
com cache para todos os clientes e **nada sem inscrito** — combinado com o Luciano em 16/10/2026, inclusive na noite da
apuração. Mesma rede da coleta (100 req/s do órgão, 429 → pausa), com prioridade abaixo dela; até `--tse-max-topicos`
(200) tópicos ao mesmo tempo. `--sem-tse` desliga. `saude.json`, bloco `tse_resultados`: `topicos_ativos`, `inscritos`,
`esperando_vaga`, `estados`, `buscas`, `req_s`, `r200`/`r304`/`r404`/`erros`, `fora_do_intervalo` (deve ser 0) e
`idade_max_s` (o arquivo do TSE mais velho entre os tópicos inscritos).

**Só a comparação, sem coleta (`--sem-coleta`, `0.6.12`).** Para abrir a página contra o site real do TSE sem baixar BU
nem sondar seção (ex.: o pleito 3220 já encerrado, com os BUs no acervo, ou o 3221 antes da apuração): o serviço não pede
`cs.json`, `aux.json` nem BU, publica o que carregou na partida e só faz as buscas da comparação (uma por tópico inscrito a cada 30–60 s). Requisições ao TSE: valem as
regras do `CLAUDE.md` (horário combinado com o Luciano durante uma apuração).

**TSE simulado (`uv-simulador --tse-simulado <candidatos.json>`, `0.6.12`).** O simulador passa a servir também os
`-u.json` progressivos de cada abrangência e cargo (com `ETag`/304), regerados a cada `--tse-passo` (15 s da noite):
cada seção entra no arquivo do TSE com um atraso de divulgação sorteado (log-normal, mediana e p90 de `Opcoes::padrao`
em `tse_simulado.rs`) depois do recebimento do BU; as não instaladas e as agregadas contam como no oficial; depois do
último atraso, o arquivo servido é **byte a byte o oficial**. Sem a opção, nada muda (os `-u.json` servidos são os
oficiais, para a conferência). Ensaio: `aquisicao/scripts/ensaio_comparacao.sh <nome> <2024-2t|2026-1t> <aceleração>
[abas]` (simulador + serviço + abas da página; `RESUMO.json` com os critérios do `docs/HANDOFF_0.7.0.md`).

A conferência automática do fim da noite, quando os arquivos do TSE ainda não estão completos (`secoes_totalizadas` <
`secoes`), tenta de novo a cada minuto (até 60 vezes; sem arquivo, até 10) e só alerta `conferencia_divergente` com os
arquivos completos; `conferencia.json` ganhou `incompletos`.

## 4. Alertas e o que fazer

| tipo | significa | o que fazer |
|---|---|---|
| `destinacao_presumida` | candidatos sem `dvt` (todos "Válido") | conferir cassações/indeferimentos antes da noite; situação pode divergir do oficial nesses casos |
| `candidatos_incompleto` | rodando com `--aceitar-incompleto` | regerar o `candidatos.json` assim que as fontes existirem e reiniciar (a retomada mantém tudo) |
| `secoes_esperadas_incoerentes` | `s.ts` pré-apuração **maior** que as seções do catálogo (listadas no `cs.json` menos as agregadas) numa abrangência | o fim automático dessa abrangência espera por seções que o `cs.json` não lista: regerar o `candidatos.json` (§1.2) ou usar `CONFERIR`. Desde a 0.9.2 (B-051) o fim usa o total do catálogo, o mesmo da página, e o `s.ts` só como piso: um `s.ts` **menor** (desatualizado; no 3221 em 23/10, SP 33.403 de 103.904) não muda mais nada e fica só no `servico.log` (`sts_desatualizado`), sem alerta |
| `disco` | < 5 GB livres | liberar espaço antes da noite |
| `cs_ausente`, `cs_ilegivel` | `cs.json` de uma UF com erro (uma vez por UF até voltar) | normal antes da publicação (SE/TO em 12/10); na noite, conferir no navegador |
| `http429` | o servidor respondeu 429; pausa global de 10 min | **não reiniciar** (reiniciar não encurta a penalidade da borda e o IP é compartilhado); esperar `retomada_em` |
| `aux_404`, `erro_rede`, `bu_indisponivel`, `aux_ilegivel` | seção publicada que não se resolveu depois das novas tentativas | fica pendente e o serviço a pede de novo sozinho (2, 4, 8, 16, 32 min… na hora do site, até 12 vezes; `secoes.repedidas` na saúde), além de na próxima mudança da disponibilização ou na retomada. Desde a v0.2.6: antes, uma queda do site mais longa que as novas tentativas deixava a apuração sem fim |
| `pendente_sem_solucao` | seção pedida 12 vezes sem resolver | conferir a seção no site; a apuração não encerra sozinha com ela pendente — use `CONFERIR` |
| `sem_bu` | `aux.json` sem BU e sem "Nao instalada" | pendente; conferir a seção no site |
| `situacao_desconhecida` | `st` do BU fora de Totalizado/Recebido/Excluído | **a seção não entra**; decidir à mão (e registrar o valor novo) |
| `secao_anulada` | BU "Excluído"/seção "Anulada" na primeira coleta | votos da seção contados como nulos (como o TSE) |
| `anulacao`, `anulacao_desfeita` | a seção foi anulada (ou deixou de ser) depois de totalizada | automático: BU retirado e reaplicado com votos nulos; publicação corrigida |
| `reenvio_identico` | hash novo, mesmo BU (SHA-256) | nada: totais inalterados |
| `reenvio_diferente` | BU novo com conteúdo diferente | automático: o antigo sai, o novo entra; conferir a seção |
| `revogacao` | um "matematicamente" foi desfeito (com o motivo) | conferir; nos ensaios nunca aconteceu |
| `fora_catalogo` | BU de seção que não está no `candidatos.json` | regerar o `candidatos.json` (UF faltando?) e reiniciar: o BU já está no `.uvpk` e é reaproveitado |
| `erro_bu` | BU ilegível | a seção fica sem BU; conferir |
| `nao_instalada_com_bu`, `bu_em_nao_instalada` | contradição entre coletas | mantido o primeiro; conferir |
| `apuracao_reaberta` | BU depois da apuração encerrada | automático: publicação volta a "apurando" e encerra de novo |
| `conferencia_divergente`, `oficiais_ausentes` | conferência com divergência ou `-u.json` faltando | ver `conferencia.md`; oficiais faltando são pedidos de novo (até 10 vezes) |
| `sondagem_limitada` | abrangência com seções sem disponibilização demais para sondar | as não instaladas dela ficam de fora de `s.st`; aceitar a diferença ou conferir pelo oficial |
| `retomada_bu_ausente`, `retomada_bu_invalido` | o diário cita um BU que não está no pacote | a seção é pedida de novo ao site |
| `erro_interno` | substituição impossível | anotar e reportar; o resto continua |

## 5. Queda, reinício e retomada

* **Queda do serviço (inclusive `kill -9`, falta de luz):** rode o **mesmo comando**. Na partida o serviço relê
  `secoes.log`, aplica o BU que vale em cada seção a partir dos `.uvpk` (sem rede), reaproveita os BUs gravados sem linha
  no diário (órfãos), publica e volta a acompanhar; as seções já resolvidas não são pedidas de novo (só se a
  disponibilização mudar). Testado com três `kill -9` (resultado idêntico ao da execução sem queda).
* **Queda do site ou da rede:** nada a fazer; as requisições falham, são repetidas com espera crescente e o
  acompanhamento continua (testado com o site simulado fora do ar).
* **Nunca** apague `secoes.log` ou os `.uvpk` de uma noite em andamento: sem eles a retomada pede tudo de novo ao site.
* **Pasta nova = noite nova:** para recomeçar do zero, use outras `--dados`/`--saida`.

## 6. Forçar a conferência

```bash
touch /home/dev/uvreal/noite-3221/saida/CONFERIR
```

Baixa os `-u.json` de todas as abrangências do `candidatos.json` (≤ 5 req/s; 43 arquivos no 3221) e confere votos,
`vnt`, comparecimento, seções e situação com a publicação calculada naquele instante; resultado em `conferencia.md` e
`conferencia.json`. Durante a apuração os oficiais também estão parciais, então divergência no meio da noite é esperada;
a conferência que vale é a do fim.

## 7. Difusão em tempo real (`--http`, desde a `0.3.2`)

O serviço pode publicar o resultado por **WebSocket** e por um **REST mínimo** para o site de exemplo e integradores.
Contrato: [PROTOCOLO_WS.md](PROTOCOLO_WS.md); decisão: ADR-026.

### Ligar

Acrescente ao comando da §2:

```bash
--http 127.0.0.1:8080          # WebSocket ws://127.0.0.1:8080/ws, REST http://127.0.0.1:8080/api/v1/
--max-clientes 5000            # padrão; acima disso o handshake recebe 503
--http-threads 4               # padrão; threads só da camada HTTP (não disputam com a coleta e o motor)
```

Sem `--http`, nada muda. A camada só começa a escutar depois da publicação a frio (todo tópico já tem valor) e, na
parada limpa (`SIGTERM` ou fim da conferência), avisa os clientes (`desligamento`) e fecha as conexões **depois** da
gravação final. Com porta `0` o serviço escolhe uma livre e grava em `<saida>/http_porta`.

**Descritores:** cada cliente é um socket; 1.000 clientes + o serviço na mesma máquina passam de 2.000. O padrão do
WSL (`ulimit -n` 1024 em alguns terminais) não basta: rode o serviço com `ulimit -n 65536` no mesmo shell (os roteiros
de ensaio já fazem isso). Para muitos clientes abrindo de uma vez, o `somaxconn` (4096 aqui) e a faixa de portas
efêmeras do cliente também limitam: abra em rampa.

### Expor fora da máquina (proxy reverso com TLS)

O binário **não tem TLS nem autenticação** e deve escutar em `127.0.0.1`. Para expor, ponha um proxy na frente.
Caddy (certificado automático):

```
resultados.exemplo.org {
    reverse_proxy /ws 127.0.0.1:8080
    reverse_proxy /api/v1/* 127.0.0.1:8080
}
```

nginx:

```nginx
location /ws {
    proxy_pass http://127.0.0.1:8080;
    proxy_http_version 1.1;
    proxy_set_header Upgrade $http_upgrade;
    proxy_set_header Connection "upgrade";
    proxy_read_timeout 120s;          # o servidor manda ping a cada 20 s
}
location /api/v1/ {
    proxy_pass http://127.0.0.1:8080;
    proxy_cache_valid 200 1s;         # o REST se comporta como arquivo estático (ETag = seq, 304)
}
```

### O que olhar na saúde (`saude.json` → `difusao`)

| campo | o que é |
|---|---|
| `clientes`, `inscricoes`, `conexoes_total` | conexões WebSocket abertas, tópicos inscritos somados, conexões desde a partida |
| `mensagens_s`, `bytes_s` | envio atual (o `br-c0001` vai para quase todos a cada lote: é o que mais pesa) |
| `latencias_ms.bu_envio` | BU recebido → mensagem escrita no socket, por mensagem (p50/p99/máx) — a meta é < 50 ms no p99 |
| `latencias_ms.bu_difusao` | BU recebido → mensagem montada (uma por versão de tópico); a diferença para `bu_envio` é o tempo de fila nas conexões |
| `juntadas`, `avisos_seq_pulado` | versões que conexões deixaram de receber por não acompanhar (receberam a mais nova) |
| `desconexoes` | por motivo: `cliente_fechou`, `cliente_lento`, `sem_pong`, `taxa`, `mensagem_grande`, `desligamento`… |
| `recusadas_max_clientes`, `tcp_recusadas` | conexões recusadas pelos limites |

### Quando os clientes lentos começam a ser cortados

`desconexoes.cliente_lento` subindo = conexões que ficaram 30 s sem conseguir receber (rede ruim do cliente, cliente
travado). **A totalização não é afetada** (a difusão nunca espera um cliente) e os outros clientes também não. Se
forem muitos de uma vez: (1) veja se a máquina está sem CPU (`recursos.cpu_pct_atual`, `top`) — o envio do `br-c0001`
a milhares de clientes custa; (2) veja `bytes_s` e a banda de saída; (3) se forem clientes externos atrás de um proxy,
o proxy deve ter buffer e `proxy_read_timeout` maior que 60 s; (4) diminua `--max-clientes` e mande os demais para o
REST (com cache no proxy), que custa quase nada por cliente.

## 8. A página de resultados (`/`, desde a `0.4.1`)

Com `--http`, o próprio binário serve a página de resultados de exemplo da etapa 5 em `http://<end:porta>/` (fontes e
regras em `/mnt/d/git/lukeboh/urna-velox/site/README.md`; ADR-027). Nada a instalar: os arquivos estão embutidos.
Atrás do proxy da §7, acrescente a raiz e os estáticos:

```
resultados.exemplo.org {
    reverse_proxy 127.0.0.1:8080          # /, /static/*, /ws e /api/v1/*
}
```

(no nginx, um `location / { proxy_pass http://127.0.0.1:8080; }` além dos blocos da §7; os estáticos têm
`Cache-Control: immutable` e podem ficar no cache do proxy.)

**O que olhar na página:**

| onde | o que diz |
|---|---|
| selo no alto, à direita | `ao vivo`; `conexão caiu · volta em Ns` (ela reconecta sozinha, com espera crescente — não precisa recarregar); `TSE pediu pausa` (pausa por HTTP 429); `apuração encerrada` |
| faixa abaixo do topo | ritmo da apuração (BUs por minuto); a pausa de rede com o horário da retomada; **avisos do serviço**: "o % de seções de algumas UFs pode estar errado" = alerta `secoes_esperadas_incoerentes` (§4); destinação presumida; BU reenviado ou seção anulada |
| cartão "Apuração" | % e número de seções apuradas, horário do último BU, **eleitores que faltam apurar para o matematicamente** (vira "matematicamente decidido às hh:mm:ss") |
| letreiro no rodapé | marcos e curiosidades (toque/clique abre o detalhe; passar o mouse pausa) |

**Ver a página numa simulação** (a noite do 1º turno de 2026, sem requisição ao TSE; desde a `0.4.8`):

```bash
bash /mnt/d/git/lukeboh/urna-velox/aquisicao/scripts/simular_site.sh [velocidade inicial, padrão 10] [porta, padrão 18080]
```

Abra `http://localhost:18080/` no Chrome do Windows. No pé da página, acima do letreiro, fica a barra **Replay**
(desde a `0.4.9`), como a de um vídeo: tocar/pausar, horário da simulação, a linha do tempo da noite (17h–2h, com as
horas marcadas) e a velocidade (1× a 300×). Arrastar a linha do tempo para um instante já visto — para trás, ou para a
frente até onde a simulação já chegou — faz o serviço **retotalizar do disco** até lá, em segundos (as abas mostram
"Indo para…" e se recarregam); além do que já foi visto, a simulação avança rápido até o ponto. Vale para todas as abas.
O terminal mostra a cada 30 s o relógio, as seções apuradas e a velocidade; `Ctrl+C` encerra o simulador e o serviço.
Cada execução começa do zero em `~/uvreal/simulacao-site`. Por baixo: `uv-simulador` e `uv-servico …
--controle-replay` (PROTOCOLO_WS §11; **ir a um instante já visto apaga `--saida`** — pastas descartáveis).

**Marcos no disco:** `<saida>/marcos.csv` (`seq,horario,classe,abrangencia,cargo,uf,municipio,zona,secao,estado,valor,lote_bus`),
só de acréscimo e relido na retomada. Conferência: `python3 /mnt/d/git/lukeboh/urna-velox/site/testes/conferir_marcos.py <saida>`
(primeira seção × `secoes.log`, seção decisiva e `faltam_para_mat` × `eventos.csv`). Saúde: `custos_ms.marcos` (por
lote, na thread do motor) e `custos_ms.faltam` (verificação de 1 s); `difusao.marcos`.

**Nomes:** a página mostra nomes de municípios, partidos e vices do `candidatos.json`; um arquivo gerado antes da
`0.4.1` não os tem (a página mostra códigos e números). Regere com `uv-coletor preparar … --offline` (usa só o cache em
`--fontes`, nenhuma requisição).
