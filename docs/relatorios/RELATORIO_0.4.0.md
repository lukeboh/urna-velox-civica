# Relatório da Etapa 4 — Publicação em tempo real (preparação para `0.4.0`)

**Data:** 13/10/2026 · **Versão:** `0.3.4` (código medido: `0.3.3`) · **Status:** itens 1 a 6 do
[HANDOFF_0.4.0.md](/mnt/d/git/lukeboh/urna-velox/docs/handoffs/HANDOFF_0.4.0.md) entregues e critérios D1–D4 e os limites do
item 5 executados com o binário final. **Etapa concluída na `v0.4.0` (13/10/2026) por decisão do Luciano, sem a
validação independente** descrita no fim do handoff (o roteiro dela continua no §8, para quem quiser fazê-la depois).

**Resumo:** D1 (1º turno de 2026 a 60×, 1.000 clientes) — **BU → cliente p99 7,7 ms** (meta < 50 ms), 0 conexões
perdidas, 1.500/1.500 últimas mensagens idênticas ao arquivo publicado, eventos = `eventos.csv`, conferência 83/83 e
832/832; com 0 clientes a difusão não muda o BU → publicação (p99 4,8 × 4,3 ms sem `--http`); D2 clientes lentos
juntados e cortados sem afetar os outros; D3 `kill -9`/429/reenvio/anulação com reconexão correta; D4 51/51 pelo
WebSocket; limites do item 5 testados; até **10.000 clientes com p99 35,6 ms**. O motor não mudou.

Caminhos: repositório = `D:\git\lukeboh\urna-velox` (Windows) = `/mnt/d/git/lukeboh/urna-velox` (WSL `Ubuntu-24.04`);
dados = `/home/dev/uvreal`; ensaios desta etapa em `/home/dev/uvreal/etapa4/`.

## 1. O que foi feito

| item | onde | estado |
|---|---|---|
| 1. Protocolo | `/mnt/d/git/lukeboh/urna-velox/docs/arquitetura/PROTOCOLO_WS.md` | escrito antes do código; ajustado com o que os ensaios mostraram (§1.2) |
| 2. Camada HTTP/WS no serviço | `/mnt/d/git/lukeboh/urna-velox/aquisicao/src/difusao.rs`; ligações em `aquisicao/src/servico.rs` (`gravador`, `gravar_eventos`, `evento_apuracao`, `saude`, `executar`); opções `--http`, `--max-clientes`, `--http-threads` em `aquisicao/src/bin/uv-servico.rs` | ADR-026 |
| 3. Cliente de carga | `/mnt/d/git/lukeboh/urna-velox/aquisicao/src/bin/uv-carga.rs` | carga, `--lentos`, `--reconectar`, `--limites`, `--amostras`, conferência byte a byte e de eventos |
| 4. Critérios | §3 | D1–D4 com o simulador |
| 5. Exposição segura | §4 | limites testados |
| 6. Documentação | `PROTOCOLO_WS.md`, `OPERACAO.md` §7, `ARCHITECTURE.md` §2.7, ADR-026, ROADMAP | |

### 1.1. Como funciona (resumo; detalhes no ADR-026 e em `ARCHITECTURE.md` §2.7)

* **O motor não mudou** e continua sem rede e sem `tokio` (`cargo tree -p urna-velox -e normal`: nenhum `tokio`,
  `hyper` ou `axum`).
* A thread `uv-gravacao` já recebia do motor o JSON de cada abrangência × cargo que mudou. Agora ela o converte em
  `Bytes` (sem cópia) e, **antes de gravar o arquivo**, entrega uma referência a uma thread nova, `uv-difusao`, por
  canal sem limite (nunca espera). Sem `--http`, nada disso acontece.
* A `uv-difusao` junta o que se acumulou (só a versão mais nova de cada arquivo) e põe cada versão no
  `tokio::sync::watch` do tópico (**só o último valor**). A mensagem com o envelope (`tipo`, `topico`, `seq`,
  `secoes_apuradas`, `ver`, `t_bu`, `t_envio`, `resultado` = bytes do arquivo) é montada **uma vez por versão**, pela
  primeira conexão que precisar dela (`OnceLock`), e compartilhada (`Utf8Bytes`) por todas as outras.
* A camada HTTP é o `axum` 0.8 num runtime próprio (`--http-threads`, padrão 4), com as threads em prioridade menor
  (nice 10). Cada conexão é uma tarefa que observa os `watch` dos seus tópicos; enquanto não consegue escrever, as
  versões se juntam (ela recebe a mais nova, com um aviso `seq_pulado` no máximo a cada 5 s por tópico); 30 s sem
  conseguir escrever, cai (`cliente_lento`).
* REST como arquivo estático: `ETag` = `seq`, 304, `Cache-Control: public, max-age=1`, CORS `*`, só a tabela de
  tópicos.
* Partida: a camada só escuta depois da publicação a frio (todo tópico já tem valor; na retomada depois de `kill -9`,
  o valor recalculado do diário). Parada limpa: publicação final entregue → 200 ms → `aviso desligamento` + close 1001
  a todos → saída 0.

### 1.2. Mudanças feitas por causa das medidas (dentro desta etapa)

| achado | medida | correção | depois |
|---|---|---|---|
| com `--http` e **0 clientes**, a `uv-difusao` montava a mensagem de toda versão de tópico (845 mil, com *parse*, cópia e alocação) e disputava CPU e o alocador (`M_ARENA_MAX=2`) com o motor | B com `--http` e 0 clientes: BU → publicação p99 **8,4 ms** (sem `--http`: 4,3), máx. 65 ms (`/home/dev/uvreal/etapa4/B4-http0_v1`) | montagem preguiçosa: a `uv-difusao` só guarda o `Arc` dos bytes; a mensagem é montada pela primeira conexão que a usa, nas threads HTTP | p99 **4,8 ms**, máx. 45 ms (`B4-http0`) |
| um aviso `seq_pulado` **por versão pulada**: com o `br-c0001` mudando ~100 vezes por segundo a 60×, quase toda conexão pula versões | D1 (código da `0.3.2`): 21,9 M avisos em 50,9 M mensagens (`D1_v1`) | no máximo um aviso por tópico a cada 5 s por conexão, com a soma das juntadas | D1: 96,6 mil avisos em 60,1 M mensagens |
| as 4 threads HTTP (~200 % de CPU com 1.000 clientes) nos mesmos núcleos do motor | D1 `0.3.2`: lote do motor p50 0,055 → 0,335 ms | threads `uv-http` com nice 10 | D1: lote p50 0,205 ms (§3.1) |
| o bloco `difusao` da saúde (percentis de janelas de 50 mil amostras) era calculado na thread do motor | +1 ms na `saude()` a cada 2 s | calculado pela `uv-difusao`; o motor copia o último | |

## 2. Como os ensaios foram rodados

Binários de `/home/dev/uvreal/target-dev/release/` compilados do código do commit `0.3.3` (`d04a7f5`; a versão impressa
nos JSON é `0.3.2` porque o *bump* foi feito depois da compilação — o código é o mesmo), por
`bash /mnt/d/git/lukeboh/urna-velox/aquisicao/scripts/compilar_dev.sh` sem avisos.

Roteiro: `/mnt/d/git/lukeboh/urna-velox/.git/uv_tmp_e4_ensaio.sh` (fora do repositório, como os da etapa 3). É o
roteiro do §3 do [RELATORIO_0.3.0.md](/mnt/d/git/lukeboh/urna-velox/docs/relatorios/RELATORIO_0.3.0.md) com três diferenças:
`--http 127.0.0.1:<porta livre>` no serviço, o `uv-carga` ao mesmo tempo, e **cada processo em núcleos próprios**
(`taskset`: serviço 0–11, simulador 12–17, carga 18–27; i7-14700, 28 threads; `ulimit -n 65536`). Os comandos:

```bash
taskset -c 12-17 uv-simulador --dados <D> --cs <CS> --metricas <MET> --oficial <OF> --pleito <P> --ciclo <C> --acelerar <A> \
  --log <W>/simulador.log --porta-arquivo <W>/porta [--corte "05/10/2026 02:00:00" --horario recebido] [falhas]
taskset -c 18-27 uv-carga --endereco 127.0.0.1:<H> --clientes <N> --lentos <K> --perfil <misto|todos> \
  --publicacao <W>/saida/publicacao --eventos-csv <W>/saida/eventos.csv --relatorio <W>/carga.json --threads 8 [...]
taskset -c 0-11 uv-servico --pleito <P> --ciclo <C> --candidatos <CAND> --dados <W>/dados --saida <W>/saida \
  --base http://127.0.0.1:<porta> --aceleracao-local <A> --intervalo-cs <30/A> --http 127.0.0.1:<H>
```

Os conjuntos (`<D>`, `<CS>`, `<MET>`, `<OF>`, `<CAND>`) são os da tabela do §3 do RELATORIO_0.3.0. Perfil `misto`
(D1/D2): metade das conexões só `br-c0001`, metade `br-c0001` + o `c0001` ou `c0003` de uma UF sorteada (semente 42).
Em cada pasta: `ensaio.log`, `simulador.log`, `servico.err`, `carga.json` (relatório do `uv-carga`, com histogramas),
`carga.err`, `time.txt` e `saida/{saude.json,conferencia.*,eventos.csv,alertas.csv,publicacao/,…}`. Resumos tirados com
`/mnt/d/git/lukeboh/urna-velox/.git/uv_tmp_e4_resumo.py`.

## 3. Critérios com o site simulado (item 4)

### 3.1. D1 — 1º turno de 2026 a 60× com 1.000 clientes, e a comparação com e sem `--http`

Pastas `/home/dev/uvreal/etapa4/{B4-sem-http,B4-http0,D1}` (código final) e `D1_v1`, `B4-http0_v1` (código da `0.3.2`).
Os três primeiros rodaram em sequência, sozinhos na máquina.

| | B sem `--http` | B com `--http`, 0 clientes | **D1: 1.000 clientes** | (D1 com o código da `0.3.2`) |
|---|---|---|---|---|
| duração do serviço | 538,4 s | 538,3 s | 538,4 s | 537,8 s |
| conferência automática | 83/83, 832/832 | 83/83, 832/832 | **83/83, 832/832** | 83/83, 832/832 |
| `uv-motor conferir-publicacao` | código 0 | código 0 | código 0 | código 0 |
| **BU → cliente** (`uv-carga`, `agora − t_bu`) p50 / **p99** / máx. | — | — | 3,22 / **7,70** / 47,1 ms | 3,66 / 9,26 / 93,2 ms |
| servidor → cliente (`agora − t_envio`) p50 / p99 / máx. | — | — | 1,15 / 3,11 / 8,1 ms | 1,18 / 4,39 / 24,9 ms |
| BU → envio (medido no serviço) p50 / p99 | — | — | 2,90 / 7,42 ms | 3,58 / 9,08 ms |
| BU → publicação gravada p50 / p99 / máx. | 0,815 / 4,34 / 44,3 ms | 0,805 / 4,80 / 45,4 ms | 2,93 / 9,97 / 58,2 ms | 3,30 / 10,9 / 104 ms |
| lote do motor p50 / p99 | 0,055 / 0,375 ms | 0,055 / 0,415 ms | 0,205 / 1,62 ms | 0,335 / 1,98 ms |
| `saude()` na thread do motor p50 / p99 | 21,9 / 30,2 ms | 23,7 / 31,6 ms | 24,4 / 33,1 ms | 23,8 / 30,6 ms |
| CPU média do serviço (100 % = 1 núcleo) | 68 % | 69 % | 214 % | 195 % |
| RSS do serviço pico / final | 304 / 243 MB | 294 / 227 MB | 429 / 359 MB | 440 / 370 MB |
| conexões abertas / perdidas | — | — | 1.000 / **0** (todas fecharam com `1001 desligamento`) | 1.000 / 0 |
| mensagens recebidas (atualizações + avisos) | — | — | 60,1 M (59,97 M + 96,6 mil); 77,7 GB, ~144 MB/s | 50,9 M (28,9 M + 21,9 M) |
| regressões de `seq` | — | — | **0** | 0 |
| última mensagem = arquivo (por conexão e tópico) | — | — | **1.500/1.500** (56 tópicos) | 1.500/1.500 |
| eventos = `eventos.csv` (por conexão) | — | — | **1.000/1.000**; 144 eventos na noite, os **3 de Presidente** (`br-c0001`) em todas as conexões | 1.000/1.000 |
| CPU média do `uv-carga` | — | — | 184 % | 117 % |

**Critério D1 (BU → cliente < 50 ms no p99): atingido — 7,7 ms**, com 0 conexões perdidas, todas as últimas mensagens
iguais ao arquivo, os eventos iguais ao `eventos.csv` e a conferência automática 83/83 e 832/832.

**Comparação com o B da etapa 3** (`/home/dev/uvreal/etapa3/B_final`, binário `0.2.6`: BU → publicação p50 0,675 / p99
4,3 ms, CPU 52 %, RSS 301/211 MB):

* **Sem `--http`, o binário de hoje dá p50 0,815 ms (era 0,675) e CPU 68 % (era 52 %)** — e isso não vem da difusão
  (ela está desligada): vem do ritmo ocioso dos `cs.json` da `0.2.8` (posterior ao B da etapa 3), que contra o simulador
  relê as 28 UFs a cada ciclo ocioso; foram **1,227 M requisições contra 1,030 M** e mais interpretação de `cs.json`
  na mesma máquina. O p99 (4,34 ms) e o máximo (44 ms) são os mesmos. A referência justa para a difusão é esta coluna.
* **Com `--http` e 0 clientes** o custo da difusão é guardar um `Arc` por versão de tópico (868 mil na noite) numa thread
  à parte: p50 igual (0,805), p99 4,80 ms (+0,46 ms, dentro da variação entre execuções), máximo igual.
* **Com 1.000 clientes** a totalização fica mais lenta: lote do motor p50 0,055 → 0,205 ms e BU → publicação p50
  0,8 → 2,9 ms, p99 4,3 → 10 ms. O motor não espera nenhum cliente (nenhum lock ou canal com eles); o que pesa é a
  máquina dividida: o serviço envia ~111 mil mensagens/s (~144 MB/s) pelo *loopback*, o `uv-carga` (184 % de CPU)
  recebe e interpreta todas, e o simulador serve ~2.300 req/s — tudo no mesmo processador e na mesma memória (as
  threads HTTP já estão com nice 10 e em núcleos lógicos diferentes do simulador e da carga, mas dividem núcleos
  físicos, cache e banda de memória). Mesmo assim BU → cliente fica em 7,7 ms no p99, 6,5 vezes abaixo da meta. Numa
  noite real, a 1×, o volume é 60 vezes menor.
* **RSS** sobe ~125 MB com 1.000 conexões (buffers do WebSocket e do kernel, tarefas, `StreamMap`): pico 429 MB.

### 3.2. D2 — D1 com mais 100 conexões que não leem (`/home/dev/uvreal/etapa4/D2`)

| | D1 | **D2 (1.000 + 100 lentas)** |
|---|---|---|
| BU → cliente das 1.000 normais p50 / p99 / máx. | 3,22 / 7,70 / 47,1 ms | 3,20 / **8,02** / 52,2 ms |
| BU → publicação gravada p50 / p99 / máx. | 2,93 / 9,97 / 58,2 ms | 2,91 / **10,5** / 53,8 ms |
| pico de RSS do serviço | 429 MB | **432 MB** |
| conferência / últimas mensagens / eventos | 83/83, 1.500/1.500, 1.000/1.000 | 83/83, 1.500/1.500, 1.000/1.000 |
| as 100 lentas | — | **todas cortadas pelo servidor** (`desconexoes.cliente_lento` = 100; no cliente: "Connection reset by peer") |

As lentas se inscrevem no `br-c0001` e não leem nada: o buffer do socket enche, a escrita para, o `watch` passa a
juntar as versões dessas conexões (nenhuma fila cresce: o RSS fica no mesmo pico de D1) e, 30 s depois de a escrita
parar, o servidor derruba cada uma com o motivo `cliente_lento` na saúde. **As outras 1.000 e a totalização não pioram
de forma significativa** (+0,3 ms no p99 de BU → cliente; BU → publicação dentro da variação).

### 3.3. D3 — falhas (2º turno de 2024, 20 UFs, a 60×, 200 clientes inscritos nos 51 tópicos e nos eventos, com `--reconectar`)

| caso | falha | resultado |
|---|---|---|
| **`kill -9` + reinício** (`/home/dev/uvreal/etapa4/D3-kill`) | `kill -9` no `uv-servico` com 51.691 BUs aplicados; reinício com o mesmo comando (mesma porta HTTP) | as 200 conexões caíram ("Connection reset") e **reconectaram (200 reconexões)**; **nenhum `instantaneo` com `seq` menor que o último recebido**, 0 regressões de `seq`; conferência 51/51 e 102/102; **publicação final idêntica à do A da etapa 3** (`/home/dev/uvreal/etapa3/A_final`: 51/51 arquivos iguais fora `ultimo_bu` e o instante do matematicamente); diário com 97.392 "aplicado", nenhuma seção duas vezes; 10.200/10.200 últimas mensagens = arquivo; eventos = `eventos.csv` em 200/200 conexões (o histórico reenviado na reinscrição é descartado pelo cliente por chave); BU → cliente p99 2,2 ms |
| **rajada de 429** (`D3-429`) | `--rajada-429 18:00:00+120` no simulador | uma pausa global; a conexão 0 recebeu `estado` com **`pausa_429.ativa: true`**, `retomada_em` e o alerta `http429` aos 60,6 s, e **`ativa: false` aos 70,7 s** (10 s reais = 600 s × 1/60); conferência 51/51 e 102/102; 10.200/10.200; 0 regressões |
| **reenvio idêntico e anulação depois de publicada** (`D3-correcoes`) | `--reenviar am:2550/1/550@18:40:00 --anular sp:71072/1/1@18:45:00` | a conexão 0 recebeu `estado` com o alerta `reenvio_identico` (101 s) e depois `anulacao` (105 s); as atualizações de São Paulo seguintes trazem os totais corrigidos e **a última mensagem de São Paulo é o arquivo corrigido** (10.200/10.200 iguais); conferência 50/51 — **só São Paulo diverge, e é o esperado** (no oficial a seção não foi anulada): Boulos −97, brancos −12, nulos +256, os mesmos números do C4 da etapa 3; 0 revogações, por isso nenhum `evento` de revogação (o `eventos.csv` não tem nenhum) |

### 3.4. D4 — o A pelo WebSocket (2º turno de 2024 a 10×, um cliente em todos os tópicos e nos eventos; `/home/dev/uvreal/etapa4/D4`)

| | resultado |
|---|---|
| duração do serviço | 1.384,9 s (o A da etapa 3: 1.384,8 s) |
| conferência automática | **51/51 e 102/102** (situação final = conferência); `uv-motor conferir-publicacao` código 0 |
| `seq` de cada tópico | **só sobe** (0 regressões em 92.232 atualizações e 51 instantâneos) |
| última mensagem = arquivo | **51/51** (os 51 municípios; nesta eleição só há Prefeito, então não há tópico de UF nem de Brasil) |
| eventos = `eventos.csv` | **102/102**, mais o `apuracao_encerrada` |
| BU → cliente p50 / p99 / máx. | 0,185 / 1,49 / 9,0 ms |
| `simulador.log` | 371.275 requisições: 97.392 `aux` 200, 97.392 BU, 13.421 `cs` 200 + 163.019 `cs` 304, 51 oficiais; 0 HTTP 429; pico 951 req/s (teto local 950) |
| mensagens reais | `/home/dev/uvreal/etapa4/D4/amostras.jsonl`, usadas em [PROTOCOLO_WS.md](/mnt/d/git/lukeboh/urna-velox/docs/arquitetura/PROTOCOLO_WS.md) §9 |

### 3.5. Contagens do `simulador.log` (D1–D3)

| ensaio | requisições | `aux` 200 / 404 | BU | `cs` 200 / 304 | 429 | pico/s |
|---|---|---|---|---|---|---|
| B sem `--http` | 1.227.458 | 499.248 / 1.364 | 499.207 | 15.438 / 212.118 | 0 | 5.738 |
| B com `--http`, 0 clientes | 1.227.794 | 499.248 / 1.364 | 499.207 | 15.605 / 212.287 | 0 | 5.736 |
| D1 | 1.210.449 | 499.248 / 1.364 | 499.207 | 10.839 / 199.708 | 0 | 5.711 |
| D2 | 1.210.098 | 499.248 / 1.364 | 499.207 | 10.799 / 199.397 | 0 | 5.757 |
| D3 `kill -9` | 284.055 | 97.392 / 0 | 97.392 | 12.413 / 76.807 | 0 | 5.701 |
| D3 429 | 281.436 | 97.392 / 0 (+1 com 429) | 97.392 | 11.679 / 74.921 | 1 | 5.705 |
| D3 correções | 285.439 | 97.394 / 0 | 97.394 | 12.929 / 77.671 | 0 | 5.269 |

O pico por segundo de calendário passa do teto local (5.700) em até 1 % — a mesma borda de contagem explicada no §4
do RELATORIO_0.3.0 (o limitador espaça 1/rps; contra o TSE o teto é 95). No D3 `kill -9`, 97.392 `aux.json` para
97.392 BUs: **depois do reinício nenhuma seção foi pedida de novo** (o diário diz o que já estava aplicado).

## 4. Exposição segura (item 5)

Escuta em `127.0.0.1` em todos os exemplos e ensaios; expor é com proxy reverso com TLS
([OPERACAO.md](/mnt/d/git/lukeboh/urna-velox/docs/operacao/OPERACAO.md) §7: Caddy e nginx). Limites testados com o serviço de
verdade apurando (`/mnt/d/git/lukeboh/urna-velox/.git/uv_tmp_e4_limites.sh`: AM e RO do 2º turno de 2024 a 10×,
`--max-clientes 5000`; `/home/dev/uvreal/etapa4/L5`) e o `uv-carga --limites`, além do teste de unidade
`difusao::testes::protocolo_ponta_a_ponta`, que roda sempre:

| tentativa | resultado |
|---|---|
| **6.000 conexões com `--max-clientes 5000`** (rampa de 1.000/s) | **5.000 atendidas, exatamente 1.000 recusadas com HTTP 503** no *handshake* (`recusadas_max_clientes` = 1.000); as 5.000 receberam normalmente (877 mil mensagens em 60 s, BU → cliente p99 25,6 ms); o serviço seguiu apurando (BU → publicação p99 0,3 ms nesse instante); RSS do serviço 677 MB com 5.000 conexões (pico 680 MB) |
| **mensagem de 1 MB** | a conexão cai com **1009 `mensagem_grande`** no próprio leitor do WebSocket (o cliente vê "Broken pipe" no envio); nada chega ao servidor de aplicação |
| **1.000 mensagens/s de um cliente** | 39 mensagens `erro` `taxa` e a conexão cai com **1008 `taxa`** depois de 93 mensagens em 0,2 s |
| ***handshake* incompleto** (cabeçalho HTTP pela metade) | a conexão TCP é fechada pelo servidor em **10,0 s** |
| caminhos vindos do cliente no REST (`/api/v1/resultado/..%2F..%2Fetc%2Fpasswd`, `/api/v1/resultado/../../saude.json`, `/api/v1/resultado/br-c0001-e006257.json`, `/api/v1/../../saude.json`, `/publicacao/…`) | **404** em todos (só a tabela de tópicos; nada chega ao disco) |
| `POST /api/v1/resultado/br-c0001` | **405** (só leitura) |
| **219 tópicos numa conexão** (limite 200; MA do 1º turno de 2024 a 60×, um tópico por município, em mensagens de 40; `/home/dev/uvreal/etapa4/L5b`, `uv_tmp_e4_limites2.sh`) | **200 `instantaneo`** e `erro limite_topicos` para o que passou; depois `erro json_invalido`, `erro op_desconhecida` e `erro binario` — e **a conexão continua aberta** recebendo (25 atualizações no intervalo); tópico com caminho (`../../etc/passwd`) dá `topico_desconhecido` (teste de unidade) |
| REST com `ETag` (`/api/v1/resultado/ma07005-c0011`) | 200 com `ETag: "0"`, `Access-Control-Allow-Origin: *`, `Cache-Control: public, max-age=1`; com `If-None-Match: "0"` → **304** |
| uma conexão que viola um limite | cai sozinha; as outras seguem (no teste de unidade, a conexão principal continua recebendo depois de outras três caírem por taxa, tamanho e máximo de clientes) |

Nos dois ensaios a apuração seguiu normal enquanto os limites eram atacados: L5 (AM e RO) e L5b (MA do 1º turno de
2024, 317,9 s a 60×) terminaram com a conferência automática idêntica ao oficial — L5: **2/2 e 4/4**; L5b: **219/219
arquivos e 608/608 situações** — e `uv-motor conferir-publicacao` código 0 nos dois.

O limite de conexões TCP (`--max-clientes` + 256) vale para conexões HTTP ainda sem *upgrade* (handshakes e REST):
depois do *upgrade* para WebSocket, a conexão passa a contar em `clientes` (e no `--max-clientes`).

## 5. Quantos clientes a máquina aguenta (D1 com 1.000, 3.000, 6.000 e 10.000)

Mesmo D1 (1º turno de 2026 a 60×, perfil `misto`, `--max-clientes 20000`), só mudando o número de clientes
(`/home/dev/uvreal/etapa4/{D1,D1-3000,D1-6000,D1-10000}`; serviço, simulador e carga na mesma máquina, em núcleos
separados):

| clientes | **BU → cliente p50 / p99 / máx.** | servidor → cliente p99 | BU → publicação p50 / p99 | lote do motor p50 / p99 | `saude()` p99 | CPU serviço / carga | RSS pico | mensagens (total) | últimas = arquivo | eventos = csv |
|---|---|---|---|---|---|---|---|---|---|---|
| 1.000 | 3,22 / **7,70** / 47 ms | 3,1 ms | 2,93 / 9,97 ms | 0,205 / 1,62 ms | 33 ms | 214 % / 184 % | 429 MB | 60,1 M | 1.500/1.500 | 1.000/1.000 |
| 3.000 | 3,52 / **11,7** / 71 ms | 8,5 ms | 2,82 / 12,6 ms | 0,215 / 1,77 ms | 31 ms | 234 % / 200 % | 710 MB | 62,8 M | 4.500/4.500 | 3.000/3.000 |
| 6.000 | 5,07 / **21,3** / 73 ms | 17,7 ms | 3,91 / 18,1 ms | 0,245 / 2,84 ms | 36 ms | 248 % / 214 % | 1.122 MB | 67,7 M | 9.000/9.000 | 6.000/6.000 |
| 10.000 | 5,77 / **35,6** / 94 ms | 30,6 ms | 4,03 / 20,7 ms | 0,255 / 3,20 ms | 47 ms | 263 % / 225 % | 1.652 MB | 72,9 M | 15.000/15.000 | 10.000/10.000 |

Em todos: 0 conexões perdidas, 0 regressões de `seq`, conferência 83/83 e 832/832, todas fecharam com
`1001 desligamento`.

* **Até 10.000 clientes o p99 de BU → cliente ficou abaixo de 50 ms** (35,6 ms) — o limite pedido no handoff ("suba até
  achar o limite ou 10.000") não foi alcançado. Pela tendência (p99 cresce ~3,5 ms a cada mil clientes depois de
  3.000), o limite desta máquina, com a carga na própria máquina, deve ficar perto de **14–15 mil clientes**.
* O total de mensagens quase não cresce (60 → 73 M) enquanto os clientes decuplicam: o envio satura em ~110–135 mil
  mensagens/s (~145–175 MB/s) e cada conexão passa a receber menos versões do `br-c0001` (mais juntadas: 50 M → 686 M).
  É o "último valor por tópico" funcionando: ninguém recebe um valor velho; quem não acompanha pula para o mais novo.
* **Memória: ~125 KB por conexão** (1.652 MB com 10.000; 429 MB com 1.000). Escolha `--max-clientes` pela RAM livre.
* A `saude()` na thread do motor (21–28 ms a cada 2 s no B) chega a 47 ms no p99 com 10.000 clientes (contenção de
  CPU): continua sendo a maior cauda da thread do motor (§6).
* A 1× (noite real) chegam ~60 vezes menos versões por segundo: o envio fica muito abaixo da saturação.

## 6. Riscos para 25/10

1. **Uma máquina só para tudo.** Coleta, motor, difusão (e, nos ensaios, a carga e o simulador) dividem CPU, cache e
   memória. Com 1.000 clientes a 60× o BU → publicação vai de p50 0,8 para 2,9 ms; a 1× o efeito é muito menor, mas se
   houver muitos clientes externos, ponha a difusão atrás de um proxy/CDN (REST com cache, ADR-021) em vez de
   aumentar `--max-clientes`.
2. **Rede de verdade.** Os ensaios são em *loopback*: clientes na internet têm latência e perdas que o servidor não
   controla (o servidor mede até o socket: `latencias_ms.bu_envio`). Clientes ruins são juntados e, se pararem de
   ler, cortados; isso não atrasa a totalização (D2).
3. **Exposição.** O binário não tem TLS nem autenticação; qualquer um que alcance a porta conecta. Escutar em
   `127.0.0.1` e publicar só por proxy reverso (OPERACAO.md §7). Os limites (clientes, tópicos, taxa, tamanho,
   *handshake*) estão testados (§4).
4. **Descritores e memória:** `ulimit -n 65536` no shell que inicia o serviço (o padrão pode ser 1.024); ~125 KB por
   conexão.
5. **`saude()` na thread do motor** é a maior cauda que sobrou (p99 31–47 ms a cada 2 s, conforme a carga). Se for
   preciso, a próxima melhoria é uma `saude()` incremental ou fora da thread do motor (o handoff já apontava).
6. **Ordem evento × atualização:** um `evento` pode chegar antes da `atualizacao` com o mesmo `seq` (caminhos
   diferentes); documentado no protocolo e no handoff da etapa 5.
7. **`relogio_site` = `01/01/1970 00:00:00`** no `estado` até a primeira resposta do site (cosmético; documentado no
   protocolo §9).
8. Os riscos da etapa 3 continuam (RELATORIO_0.3.0 §7): `cs.json` de TO, `s.ts` parcial em 9 UFs (a difusão repassa
   o alerta `secoes_esperadas_incoerentes` no `estado`), destinação presumida "Válido".

## 7. O que ficou pendente

* **Validação independente** (fim do handoff) e o `bump minor` para `0.4.0`.
* **`uv-coletor preparar --pleito 3221`** (para ver se o `cs.json` de TO saiu): não foi repetido hoje — a sessão da
  etapa 3 já o rodou em 13/10 às 12:46 (TO ainda 404) e o handoff pede uma vez por dia; nenhuma requisição ao TSE nesta
  sessão. Repetir em 14/10 e nos dias seguintes até 24/10.
* `saude()` incremental (§6.5).
* Marcos/curiosidades e nomes de municípios e partidos para o site: especificados no
  [HANDOFF_0.5.0.md](/mnt/d/git/lukeboh/urna-velox/docs/handoffs/HANDOFF_0.5.0.md) (servidor, por acréscimo ao protocolo).

## 8. Como validar (sugestão)

Pastas dos ensaios: `/home/dev/uvreal/etapa4/{B4-sem-http,B4-http0,D1,D2,D3-kill,D3-429,D3-correcoes,D4,L5,L5b,D1-3000,D1-6000,D1-10000}`
e, do código da `0.3.2`, `{D1_v1,B4-http0_v1}`. Em cada uma: `ensaio.log`, `simulador.log`, `servico.err`, `carga.json`,
`carga.err`, `saida/{saude.json,conferencia.md,conferencia.json,eventos.csv,secoes.log,alertas.csv,publicacao/}`.
Roteiros (fora do repositório): `/mnt/d/git/lukeboh/urna-velox/.git/uv_tmp_e4_{ensaio,D12,D34,limites,limites2,cadeia,final}.sh`;
resumos: `uv_tmp_e4_resumo.py`, `uv_tmp_e4_simlog.py`, `uv_tmp_e4_comparar.py` (publicação final × A).

Ensaios automatizados: `UV_DADOS=/home/dev/uvreal cargo test --release -p uv-aquisicao --test servico -- --ignored --test-threads 1`
(9: os 6 da etapa 3 e `difusao_d4_reduzido`, `difusao_cliente_lento`, `difusao_kill9_reconexao`) e
`cargo test --release -p uv-aquisicao --lib` (inclui `difusao::testes::protocolo_ponta_a_ponta`).

### Commits desta etapa

| versão | commit | assunto |
|---|---|---|
| `0.3.2` | `d5fbc1d` | servico: difusão em tempo real por WebSocket e REST (`--http`) e `uv-carga` |
| `0.3.3` | `d04a7f5` | difusao: mensagem montada sob demanda, avisos limitados e HTTP com prioridade menor |
| `0.3.4` | `ec07fa7` | docs: relatório da etapa 4, exemplos reais no protocolo, `uv-carga --limites` com mais de 200 tópicos, handoff da etapa 5 |
| `0.4.0` | tag `v0.4.0` | etapa 4 concluída (por decisão do Luciano, sem a validação independente) |
