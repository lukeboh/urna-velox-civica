# Sondas — o padrão *Test Message* no Urna Velox

Desde a `1.0.24`. Código: `/mnt/d/git/lukeboh/urna-velox/aquisicao/src/sonda.rs` (lógica, sem rede),
`servico.rs` (injetor, motor, gravação), `difusao.rs` (`op: sonda`), `federacao/no.rs` (eco entre os nós),
`bin/uv-sonda.rs` (verificador de fora) e `painel/` (cartão e gráfico). Decisão: ADR-032 em
`/mnt/d/git/lukeboh/urna-velox/docs/arquitetura/ARCHITECTURE_DECISIONS.md`. Protocolo: `/mnt/d/git/lukeboh/urna-velox/docs/arquitetura/PROTOCOLO_WS.md` §4.9.

## Para que serve

A saúde que já existia (`saude.json` a cada 2 s, *ping* do WebSocket, *heartbeat* da federação) diz que **cada
processo está vivo**. Ela não diz que **o caminho dos BUs anda**: com o motor preso num lote, a gravação engasgada, a
difusão sem entregar ou um proxy no meio retendo as mensagens, o processo continua "vivo". E na noite há duas
situações em que não dá para saber, só olhando os BUs, se o problema é nosso:

* **antes do primeiro BU** (17h): o serviço está pronto ou só quieto?
* **quando os BUs param de chegar**: parou o TSE ou parou o Urna Velox?

A sonda responde às duas: **se as sondas chegam e os BUs não, quem parou foi o TSE; se as sondas também param, o
problema é nosso.**

## Como funciona

O padrão (Hohpe & Woolf, *Enterprise Integration Patterns*, cap. 11) tem quatro peças:

| peça | no Urna Velox |
|---|---|
| **gerador / injetor** | a thread `uv-sonda` do `uv-servico`: a cada `--sonda-intervalo` s (padrão 30; 0 desliga) põe uma `Msg::Sonda` no **mesmo canal dos BUs** do motor (e, federado, um registro `sonda` no próximo lote do nó) |
| **o caminho** | FIFO, atrás dos BUs: o motor passa a sonda adiante depois do lote e da publicação que estavam na frente; a thread de gravação, depois de gravar os arquivos do mesmo lote; a difusão a entrega a quem pediu |
| **separador** | a sonda **não toca** no estado, no diário (`secoes.log`), nos pacotes `.uvpk` nem nos arquivos publicados; na difusão só vai para quem pede `{"op":"sonda"}` (a página pede, para a telemetria) |
| **verificador** | (1) o `Vigia` dentro do serviço: sonda que não volta da gravação no limite → alerta `sonda_atrasada`; (2) o `uv-sonda`, cliente WebSocket de fora: o caminho inteiro, até a rede e o proxy |

```text
uv-sonda (thread) ──Msg::Sonda──▶ canal do motor (FIFO, atrás dos BUs) ──▶ motor ──Gravar::Sonda──▶ gravação
        │                                                                                         │
        │ (federado) registro "sonda" no lote ──▶ par: consumidor ──▶ motor do par ──▶ … (eco)     ▼
        └─ Vigia: confere o atraso das pendentes (alerta)                    Vigia.concluir ◀── difusão ──▶ WebSocket
                                                                                                    (op: sonda) ──▶ uv-sonda
```

**Garantias.** Nenhuma requisição ao TSE (a sonda nasce no serviço; os 100 req/s e a regra do 429 não mudam). A
totalização não muda: o teste `sonda_nao_muda_a_publicacao` (dados reais, `--ignored`) compara a publicação final do
2º turno de 2024 (AM e RO) sem sondas e com uma a cada 50 ms — idênticas, com o mesmo `checksum` do motor. Custo: uma
mensagem a cada 30 s no canal do motor; na thread do motor, só um carimbo de tempo.

## Opções

| | |
|---|---|
| `uv-servico --sonda-intervalo S` | segundos reais entre duas sondas (padrão 30; 0 desliga) |
| `uv-servico --sonda-limite S` | sonda pendente há mais que isto → alerta `sonda_atrasada` (0 = o dobro do intervalo, no mínimo 5 s) |
| `uv-sonda --url ws://host:porta/ws` | verificador de fora (repetível; aceita `host:porta`); `--limite` (padrão 75 s), `--saida <arquivo>` (relatório a cada segundo), `--duracao S` + `--min-sondas N` (sai com 0 se todos os alvos terminarem `ok` com N sondas, senão 1) |

## O que olhar

* **`saude.json` → `sonda`**: `injetadas`, `concluidas`, `pendentes`, `perdidas` (deve ficar 0: o caminho é FIFO e só
  atrasa), `atrasada`, `episodios_atraso`, `atraso_s` (idade da pendente mais antiga), `ultima` (a última sonda que
  voltou, com os `ms`), `latencias_ms.{motor,gravacao,total}` e `remotas` (federado: as sondas de cada par, com
  `puladas`, `reinicios`, `federacao_ms` entre os relógios e `atrasada` se o par sumiu por mais de 2 × o limite).
* **`saude.json` → `difusao`**: `sondas_enviadas` e `latencias_ms.sonda_envio` (injeção → socket).
* **`saude.json` → `federacao`**: `sondas_publicadas` e `sondas_recebidas`.
* **Alertas**: `sonda_atrasada` (uma vez por episódio, gravado pela thread `uv-sonda`, que continua viva com o motor
  parado: aparece no `alertas.csv` e no `servico.log` mesmo sem `saude.json` nova); no fim do episódio,
  `sonda_normalizada` no `servico.log` com a duração.
* **Painel dos nós**: o cartão "Sondas (caminho dos BUs)" (injeção → gravação, pendente mais antiga, episódios e o
  verificador externo), o gráfico "Sonda: injeção → gravação" e a situação do nó em vermelho com o caminho parado. O
  painel continua só lendo arquivos: o relatório do `uv-sonda` entra se ele gravar em `<saida do serviço>/sonda.json`.
* **`GET /api/v1/sonda`**: as últimas 20 sondas e a latência até o socket.
* **Na página**: o chip de BUs/min alterna, discretamente (3,5 s a cada 9 s), com a confirmação da
  sonda — "✓ estável · 4 ms" ("✓ 4 ms" no celular), o atraso injeção → difusão da última sonda, que é o que um BU
  recebido agora leva para ser processado e publicado. Sem sonda nova por mais de 2,5 × o intervalo (75 s com o padrão
  de 30 s), o chip fica fixo em "⚠ sem sonda há …". Tocar no chip abre a **telemetria**: velocímetros do ritmo
  (BUs/min), do atraso de processamento (sonda), de "BU recebido → publicado" (p50 do `estado.latencia_bu_ms`) e da
  idade da última confirmação, com o p99, o intervalo das sondas e quantas chegaram na visita. Servidor sem sonda: o
  chip não alterna e a telemetria diz que o servidor não envia sondas. Código: `site/src/sonda.js` (lógica) e
  `site/src/telemetria.js` (janela); teste: `site/testes/telemetria.mjs`.

### Na noite

Rodar o verificador ao lado de cada nó, com o relatório na pasta de saída do serviço (o painel mostra):

```bash
uv-sonda --url 127.0.0.1:8080 --saida /srv/urna-velox/saida/sonda.json
```

Para conferir também a borda (proxy reverso, CDN), um segundo `uv-sonda` de fora, apontado para o `wss://` público.

| o que se vê | significa |
|---|---|
| sondas chegando, BUs parados | o TSE parou de publicar (ou só não há BU novo): nada a fazer do nosso lado |
| `sonda_atrasada`, `saude.json` atualizando | o motor está ocupado com um lote enorme ou a gravação/difusão engasgou; acompanhar `custos_ms` |
| `sonda_atrasada`, `saude.json` parada | a thread do motor parou: é o caso de reiniciar (a retomada pelo diário não perde BU) |
| `uv-sonda` local `ok`, o de fora `atrasada`/`desconectado` | o problema está no proxy, na CDN ou na rede |
| `remotas.<par>.atrasada` | as sondas do par pararam de chegar pelos lotes: conferir a sincronia com ele (o *heartbeat* pode estar vivo) |

## Falha provocada para os ensaios: `TRAVAR_MOTOR`

Só contra o site simulado local (contra qualquer outro servidor o arquivo é ignorado): criar `<saida>/TRAVAR_MOTOR`
faz a thread do motor dormir até o arquivo sumir (registro `motor_travado`/`motor_destravado` no `servico.log`). É o
caso que a sonda existe para pegar: o processo vivo, o caminho parado. O teste `motor_travado_dispara_o_alerta`
usa exatamente isso.

## Compatibilidade

* Clientes que não pedem `op: sonda` (a página, o `uv-carga`, integradores) não recebem nada novo.
* Federação: todos os nós devem ter a `1.0.24` ou mais nova. Um nó antigo que recebe um registro `sonda` num lote o
  conta em `rejeitados` (não tem seção) e não aplica nada; o resto do lote segue normal.
* Com sondas ligadas e federado, cada nó grava um lote pequeno a cada intervalo mesmo sem BU novo (só a sonda).

## Testes

| teste | onde | o que prova |
|---|---|---|
| `sonda::testes::*` | `aquisicao/src/sonda.rs` | formato e leitura da mensagem; vigia: caminho andando sem alerta, motor parado com um alerta por episódio e normalização, teto das pendentes, sondas dos pares (pulos, repetidas, reinício); verificador: situações, pulos, repetidas, reinício, quedas, separação das sondas dos pares |
| `difusao::testes::sondas_guardadas_e_rest`, `sonda_pelo_websocket` | `aquisicao/src/difusao.rs` | só quem pede recebe; instantâneo na inscrição; ordem; desligar; as dos pares sem entrar na latência local; REST |
| `federacao::no::testes::sonda_no_lote_e_no_par` | `aquisicao/src/federacao/no.rs` | registro `sonda` no lote sem contar como resultado; volta como sonda remota no par; registros ilegíveis |
| `painel::amostras::testes::*` | `aquisicao/src/painel/amostras.rs` | colunas `sonda_ms`/`sonda_atraso_s`; relatório do `uv-sonda` no resumo, com a idade do arquivo |
| `tests/sonda.rs` (3 testes, sem dados reais) | `aquisicao/tests/sonda.rs` | serviço de verdade (`--sem-coleta`): sondas de ponta a ponta com carimbos em ordem, nada na publicação nem no diário, zero requisições; `uv-sonda` com código 0/1; motor travado → `sonda_atrasada` uma vez, `uv-sonda` `atrasada`, destravado → `sonda_normalizada` e nada perdido |
| `site/testes/telemetria.mjs` (sem servidor) | `site/testes/` | lógica da página (sonda nova, de inscrição, repetida, pulada, de outro nó, sessão nova, limite); a 1366 e a 390 px, nos dois temas: a página pede `op: sonda`, o chip alterna e o texto cabe, a janela abre abaixo do chip com 4 velocímetros e os valores certos, Esc e tocar fora fecham, caminho parado → chip fixo e janela em alerta, volta a estável, servidor sem sonda não alterna, CLS 0 |
| `sonda_nao_muda_a_publicacao` (`--ignored`) | `aquisicao/tests/servico.rs` | publicação final idêntica com e sem sondas (dados reais do 2º turno de 2024) |
| `tres_orgaos` (`--ignored`) | `aquisicao/tests/federacao.rs` | cada nó recebe e ecoa as sondas dos outros dois |
