# Relatório da Etapa 3 — Operação automatizada (preparação para `0.3.0`)

**Data:** 13/10/2026 · **Versão:** `0.2.6` (relatório) → **`0.3.0`** · **Status:** itens 1 a 8 do
[HANDOFF_0.3.0.md](HANDOFF_0.3.0.md) entregues e critérios A, B, C e o teste real do 3221 executados com o binário final.

> **Atualização (`v0.3.0`, 13/10):** etapa concluída por decisão do Luciano **sem a validação independente** prevista
> no handoff. Depois deste relatório entraram, sem mudar a totalização: `--painel` (`0.2.7`), dois ritmos de releitura
> dos `cs.json` com intervalos em hora do site (`0.2.8`) e `--intervalo-cs-ocioso` controlando o gasto (`0.2.9`) —
> ver o [CHANGELOG](../CHANGELOG.md).

Caminhos: repositório = `D:\git\lukeboh\urna-velox` (Windows) = `/mnt/d/git/lukeboh/urna-velox` (WSL `Ubuntu-24.04`);
dados = `/home/dev/uvreal`; ensaios desta etapa em `/home/dev/uvreal/etapa3/`.

## 0. Quem fez o quê (histórico desta etapa)

A etapa foi feita em duas sessões. É importante para quem valida, porque parte do código entrou num commit com
mensagem e CHANGELOG de documentação.

| versão | commit | o quê |
|---|---|---|
| `0.2.2` | `21244a2` | 1ª sessão: itens 1 a 4 (motor retirar/substituir, `uv-simulador`, `uv-servico`, situações do `aux.json`), ensaios `--ignored` do serviço (6), ADR-019/020. Critérios A e B rodados em 12/10 com os binários `0.2.1`/`0.2.2` (`/home/dev/uvreal/etapa3/A_0.2.1`, `B_0.2.2`). |
| `0.2.3` | `81e2ed1` | documentação de escalabilidade (fora do escopo da etapa 3). |
| `0.2.4` | `14a0298` | "docs: relatório de auditoria" — **mas o commit também traz código do serviço que o CHANGELOG da 0.2.4 não cita**: thread de gravação da publicação (`uv-gravacao`), cache de totais por UF no `Publicador` (`ativar_cache_uf`/`invalidar_uf`, teste `publicador_cache_por_uf`), histograma da execução inteira nas latências (`p50_total`/`p99_total`), `M_ARENA_MAX=2` + `malloc_trim` depois de cada ciclo de `cs.json` (RSS), 4 threads de rede, sondagem só no exterior, `Situ::Bu` com a impressão do hash; e `docs/OPERACAO.md`, `docs/AQUISICAO.md` §5.1, `ARCHITECTURE.md`. Registrado agora no CHANGELOG da `0.2.6`. |
| `0.2.5` | `bd00c4a` | gráficos de desempenho (fora do escopo). |
| `0.2.6` | (este) | 2ª sessão (esta): verificação de tudo com o código atual, correções abaixo (§1), critérios A/B/C completos e teste real do 3221 com o binário final, este relatório. |

## 1. O que esta sessão mudou no código (`0.2.6`)

* **Seções pendentes pedidas de novo sozinhas** (`aquisicao/src/servico.rs`: `Pend`, `Motor::repedir_pendentes`,
  `Pedido::Recoletar`). **Defeito encontrado no ensaio C-queda com o binário 0.2.5:** uma seção publicada cujo
  `aux.json`/BU não vinha depois das novas tentativas virava "pendente" e só era pedida de novo se a disponibilização
  mudasse ou numa retomada; com o site fora do ar por 120 s reais a 60×, 160 seções ficaram pendentes e a apuração nunca
  encerrou (o ensaio automatizado de queda, de 10 s, não esgotava as tentativas). Agora o motor as pede de novo a cada
  2, 4, 8, 16, 32 min (hora do site), até 12 vezes (alerta `pendente_sem_solucao`), só as que têm disponibilização
  (a regra "seção sem disponibilização não gera requisição" continua valendo); `secoes.repedidas` na saúde. Provado
  pelo caso C5′ (§3.3).
* **Sincronia do disco fora da thread do motor** (`aquisicao/src/servico.rs` `sincronizador`, `motor/src/pacote.rs`
  `Escritor::descarregar_e_copiar`). A cada 2 s o motor fazia `sync_data` em ~30 arquivos (28 `.uvpk` + diário) na
  própria thread; os BUs que chegavam nesse intervalo esperavam. Agora o motor só esvazia os buffers para o SO e manda
  cópias dos descritores a uma thread `uv-sincronia` (canal de capacidade 1: se a anterior não terminou, essa rodada é
  pulada). Contra `kill -9` nada muda (os dados já estão no SO); contra falta de energia, um BU que o diário cite e não
  esteja no disco é pedido de novo na retomada (`retomada_bu_ausente`, já existente). Efeito medido no §4.
* **`saude.json` mais barata**: uma ordenação por amostra (eram duas), contagem por UF sem uma `String` por seção.
* **Custos novos na `saude.json`** (`custos_ms`): `lote` (lote inteiro do motor), `saude`, `verificacao`,
  `sincronia_disco` (medido na thread de sincronia).
* **`uv-simulador` recusa reenvio/anulação marcados para antes da disponibilização da seção** — antes o pedido virava
  em silêncio "seção publicada já com dois hashes" (ou "anulada desde o início") e o ensaio não provava o que dizia.
  Foi o que aconteceu com `sp:71072/1/1@18:30` no roteiro de C da 1ª sessão (a seção só é publicada às 18:36:50).
* **Alerta `secoes_esperadas_incoerentes`**: comparava Brasil × soma das UFs só quando **todas** as UFs tinham `s.ts` e
  só acusava `s.ts` **maior** que a lista do `cs.json`. No 3221 de hoje (TO sem `s.ts`) ele dizia só "1 UF sem s.ts",
  com SP em 33.403 de 91.326 seções. Agora compara sempre e acusa `s.ts` < 90 % da lista para Presidente e Governador
  (fora o exterior, onde metade das seções é agregada; consultas e suplementares cobrem só alguns municípios), até 30
  itens na mensagem.
* **Ensaio automatizado `queda_do_simulador`**: queda de 60 s reais no pico de chegada de AM e RO (era 10 s às 17:40,
  quando já não havia seção em coleta), exigindo 0 pendentes no fim.
* **Teste `segundo_turno_2026_candidatos_pre_apuracao`** (motor `--ignored`): lia os `-u.json` pré-apuração da raiz de
  `fontes/`, mas o `preparar` os guarda em `fontes/u/`; falhava desde que o `candidatos.json` do 3221 foi regerado
  (12/10 21:31, depois do commit da `0.2.2`). Agora lê as duas pastas e exige pelo menos um arquivo. Nenhum número mudou.

## 2. O que existe (itens 1 a 8 do handoff)

| item | onde | estado |
|---|---|---|
| 1. Motor: aplicar, retirar, substituir | `motor/src/estado.rs` (`retirar`, `substituir`, `aplicar_secao`, `aplicada`), `acompanhamento.rs` (`bu_alterado`, motivo nas revogações), ADR-019 | testes `motor/tests/retirada.rs` (aplicar → retirar = estado zero, inclusive `aptos`, `secoes` e checksum; substituir A por B = só B; anular depois = anulada desde o início; caso real MA 07315/0023/0354; cache por UF do `Publicador`) — rodam sempre |
| 2. `uv-simulador` | `aquisicao/src/simulador.rs`, `src/bin/uv-simulador.rs`, ADR-020 | `cs.json` reconstruído no instante simulado com `ETag`/304, `aux.json` no formato real, `-bu.dat` dos `.uvpk`, `-u.json` só depois do último BU, relógio acelerado, falhas (429, 5xx, latência, reenvio, anulação, queda), `simulador.log` |
| 3. `uv-servico` | `aquisicao/src/servico.rs`, `src/bin/uv-servico.rs` | partida (recusa `incompleto` sem `--aceitar-incompleto`; retomada pelo diário + `.uvpk`), acompanhamento com `If-None-Match` e sem requisição para seção sem disponibilização, coleta com o limitador do `coletor.rs`, pausa global depois de 429, totalização em fluxo, matematicamente a cada lote, publicação atômica só do que mudou, `eventos.csv`, diário `secoes.log`, conferência automática, `saude.json` a cada 2 s, `servico.log`, `alertas.csv`, parada limpa |
| 4. Situações do `aux.json` | `servico.rs` (`coletar`, `aplicar_bu`) | §6 |
| 5. Critérios | `aquisicao/tests/servico.rs` (6 ensaios `--ignored`) + execuções completas | §3 |
| 6. Desempenho | `saude.json` (`latencias_ms`, `custos_ms`, `recursos`) | §4 |
| 7. Teste real 3221 | — | §5 |
| 8. Documentação | `docs/OPERACAO.md`, `ARCHITECTURE.md` §2.6, `docs/AQUISICAO.md` §5.1, ADR-019/020, `ROADMAP.md` | revisada nesta versão |

**Teto de taxa no código** (`aquisicao/src/coletor.rs` `limites`, `base_local`, `Rede::para`): conferido pela linha de
comando com o binário final (host `.invalid`, nenhuma requisição sai):

| opções | resultado |
|---|---|
| `--base https://resultados.exemplo.invalid --aceleracao-local 10` | `Error: aceleração 10 recusada: só vale contra um servidor local` |
| `--base https://resultados.exemplo.invalid --rps 500` | `100 req/s (teto 100), pausa de 600 s depois de um 429` |
| `--base http://127.0.0.1:9 --rps 500` (sem aceleração) | `100 req/s (teto 100), pausa de 600 s` |
| `--base http://127.0.0.1:9 --rps 500 --aceleracao-local 10` | `1000 req/s (teto 1000), pausa de 60 s — servidor local` |

Testes de unidade `servidor_local` e `limites_nao_afrouxam_contra_o_tse` cobrem o mesmo.

## 3. Critérios com o site simulado (item 5)

Todos com o binário final **`0.2.6`** (`/home/dev/uvreal/target-dev/release/`), compilado por
`bash /mnt/d/git/lukeboh/urna-velox/aquisicao/scripts/compilar_dev.sh` sem avisos. Roteiro de cada ensaio (simulador +
serviço + `uv-motor conferir-publicacao` no fim), com as pastas e opções exatas: o comando do serviço é

```bash
uv-servico --pleito <P> --ciclo <ciclo> --candidatos <candidatos.json> --dados <W>/dados --saida <W>/saida \
  --base http://127.0.0.1:<porta> --aceleracao-local <A> --intervalo-cs <30/A>
```

e o do simulador

```bash
uv-simulador --dados <pacotes> --cs <cs.json> --metricas <métricas> --oficial <oficiais> --pleito <P> --ciclo <ciclo> \
  --acelerar <A> --log <W>/simulador.log --porta-arquivo <W>/porta [falhas]
```

| conjunto | `--dados` | `--cs` | `--metricas` | `--oficial` | `--candidatos` | P / ciclo |
|---|---|---|---|---|---|---|
| 2024 2º t. | `/home/dev/uvreal/2024-2t/dados` | `…/2024-2t/dados/config` | `/home/dev/uvreal/2024-2t/metricas_2t.csv` | `/home/dev/uvreal/2024-2t/oficial` | `/home/dev/uvreal/2024-2t/preparo/candidatos.json` | 453 / ele2024 |
| 2026 1º t. | `/home/dev/uvreal/dados` | `/home/dev/uvreal/config` | `/home/dev/uvreal/metricas_{multi,nacional,retomada}.csv` | `/home/dev/uvreal/oficial` | `/home/dev/uvreal/preparo_2026_1t/candidatos.json` | 3220 / ele2026; simulador com `--horario recebido --corte "05/10/2026 02:00:00"` (o `cs.json` de 2026 foi coletado dias depois da noite) |

### 3.1. Critério B — 1º turno de 2026 a 60× (`/home/dev/uvreal/etapa3/B_final`, rodou sozinho na máquina)

| | resultado |
|---|---|
| duração | 537,8 s (noite de 04/10 17:07 a 05/10 02:05, hora do site) |
| conferência automática do serviço | **83/83 arquivos idênticos; 832/832 situações** (cargo 1: 29/29, 348/348; cargo 3: 27/27, 183/183; cargo 5: 27/27, 301/301) |
| `uv-motor conferir-publicacao --publicacao …/saida/publicacao --oficial …/saida/oficial` | código 0 |
| matematicamente | 55 circunscrições, 55 decididas, **0 revogações**, 0 diferentes do oficial; **3 eventos de Presidente** (2 × `2t`, 1 × `matematicamente_2t`) |
| BU → publicação | **p50 0,675 ms**, p99 4,3 ms, máx. 55 ms (§4) |
| BUs | 499.207 aplicados, 0 duplicados, 0 erros, 0 fora do catálogo; 41 não instaladas e 1.364 agregadas pela sondagem do exterior |
| rede (`simulador.log`) | 1.029.834 requisições: 499.248 `aux` 200, 1.364 `aux` 404 (só a sondagem do exterior), 499.207 BU, 8.429 `cs` 200 + 21.503 `cs` 304, 83 oficiais; **0 HTTP 429**; pico 5.714 req/s (teto local 5.700, §4) |
| alertas | só `destinacao_presumida` |

### 3.2. Critério A — 2º turno de 2024 a 10× (`/home/dev/uvreal/etapa3/A_final`)

| | resultado |
|---|---|
| duração | 1.384,8 s (27/10/2024 16:59:33 → 20:50:09, hora do site) |
| conferência automática do serviço | **51/51 arquivos idênticos; 102/102 situações** |
| `uv-motor conferir-publicacao` | código 0 |
| HTTP 429 provocado | **0** |
| duplicados / erros | **0 / 0** (97.392 BUs, 97.392 "aplicado" no diário, nenhuma seção duas vezes) |
| revogações | **0** (51 circunscrições, 51 decididas, 0 diferentes do oficial) |
| seção sem disponibilização pedida | **0** (`aux.json` 404: 0; sondagem: 0) |
| `simulador.log` | 204.075 requisições: 97.392 `aux`, 97.392 BU, 2.259 `cs` 200 + 6.981 `cs` 304, 51 oficiais; média 147 req/s, pico 951 num segundo (teto local 950) |
| antecedência do matematicamente sobre a totalização do TSE (49 municípios sem retotalização) | mín. 78 s, **mediana 1.562 s (26 min)**, máx. 10.071 s, 0 negativos |
| antecedência da apuração completa | mediana 285 s, 1 negativo (−2 s) |
| BU → publicação | p50 0,115 ms, p99 0,415 ms, máx. 8,8 ms |

**Comparação com o cenário "site" de `/mnt/d/git/lukeboh/urna-velox/aquisicao/results/PREFEITO_2024_2T.md`:** lá,
mediana do matematicamente 1.561 s (mín. 85 s, máx. 10.071 s) e da apuração completa 285 s, com 1 município em que o
TSE chegou antes (Santarém/PA, por 2 s: o último BU só apareceu no `cs.json` 8 min depois do recebimento). Aqui: 1.562
s e 285 s, e o mesmo negativo de −2 s. **Diferença de 1 s na mediana**: o cenário "site" era uma simulação do coletor
(releitura a cada 30 s + latências medidas na coleta real, ~0,5 s por requisição) e aqui é o serviço de verdade contra
o simulador, com releitura a cada 30 s simulados (3 s reais a 10×) e latência local de ~0,1 ms. Os dois são dominados
pelo mesmo fator — o horário de disponibilização no `cs.json` e o ciclo de releitura —, por isso quase coincidem; a
latência da rede real (~1 s por seção, duas requisições) não aparece aqui. O A da 1ª sessão (0.2.1) deu 1.561 s / 284
s, mas com 32 sondagens fora do exterior (corrigido na 0.2.4); o A com o binário 0.2.5 hoje deu 1.562 s / 285 s, 51/51,
e BU → publicação p99 de 48 ms (o `fsync`, §4).

### 3.3. Critério C — falhas (2º turno de 2024, 20 UFs, 60×; `/home/dev/uvreal/etapa3/C-<caso>_final`)

Roteiro: `/mnt/d/git/lukeboh/urna-velox/.git/uv_tmp_C2.sh <caso> final` (mesmo comando do serviço de §3; as falhas são
opções do simulador). O A rodou ao mesmo tempo (a carga não muda resultados, só tempos).

| caso | falha (opções) | resultado |
|---|---|---|
| **C1 `kill -9` × 3** | `kill -9` no `uv-servico` com 20.336, 51.555 e 80.103 BUs aplicados, reinício com o mesmo comando | 51/51, 102/102; **publicação final idêntica à do A** (51 arquivos, fora `ultimo_bu` e o instante do matematicamente); diário com 97.392 "aplicado", nenhuma seção duas vezes; checksum do estado igual ao do A (133009645324); o último reinício só pediu as 17.156 seções que faltavam |
| **C2 rajada de 429** | `--rajada-429 18:00:00+120` (2 s reais) | 6 respostas 429 no mesmo instante (requisições já em voo), **uma pausa global de 10,00 s reais** (= 600 s × 1/60) e **0 requisições durante a pausa** pelo `simulador.log`; retomada sem perda (97.392/97.392); 51/51, 102/102; alerta `http429` |
| **C3 reenvio idêntico** | `--reenviar sp:71072/1/1@18:45:00 --reenviar am:2550/1/550@18:40:00` | 2 alertas `reenvio_identico` (hash novo, mesmo SHA-256), totais inalterados — publicação idêntica à do C1; 51/51, 102/102 |
| **C4 anulação depois de publicada** | `--anular sp:71072/1/1@18:45:00` (a seção é publicada às 18:36:50) | alerta `anulacao`, ação `anulado`: o BU sai e volta com os votos como nulos (Nunes −147, Boulos −97, brancos −12, nulos +256); publicação corrigida; conferência 50/51 — **só o arquivo de São Paulo diverge, e é o esperado** (no oficial essa seção não foi anulada); 102/102 situações, 0 revogações. A conferência **com uma seção anulada no oficial** é o ensaio automatizado `anulacao_depois_de_publicada` (MA 07315/0023/0354, 1º turno de 2024, passou hoje): ele exige que o arquivo do município da seção (`ma07315-c0011-e000619-u.json`) bata com o oficial, que tem a seção anulada, e só admite divergências de situação nos outros (destinação presumida); alerta `anulacao` 1, ação `anulado` 1, 0 duplicados |
| **C5 queda do site** | `--queda 18:00:00+120` (120 s reais = 2 h simuladas) | retomada sozinha: 97.392/97.392, 0 pendentes, 51/51, 102/102. **Com o binário 0.2.5 a mesma queda deixou 160 seções pendentes e a apuração não encerrou** (`/home/dev/uvreal/etapa3/C-queda_0.2.5`, `saude_ao_parar.json`): defeito corrigido na 0.2.6 (§1) |
| **C5′ queda com `--timeout 2`** (determinístico) | `--queda 18:00:00+60` no simulador e `--timeout 2` no serviço (as 8 tentativas somam ~25 s < 60 s) | **45 seções pendentes**, pedidas de novo 135 vezes (3 rodadas; as duas primeiras ainda com o site fora) e **resolvidas**: 0 pendentes, 97.392/97.392, 51/51, 102/102. Na C5 sem `--timeout 2` nenhuma seção esgota as tentativas (cada uma espera 30 s penduradas na queda), por isso este caso existe |
| **C6 rede ruim** (capacidade) | `--latencia-ms 20-400 --erros-5xx 0.02` | 2.295 respostas 503 (2 %), todas recuperadas pelas novas tentativas (2.293), 0 pendentes, 0 erros, 0 duplicados, BU → publicação p99 0,7 ms. **Vazão limitada a ~311 req/s** pela concorrência (64 seções ÷ 0,211 s de latência ≈ 303 req/s), abaixo dos ~2.000 req/s que a noite a 60× pede: formou fila (46 mil) e foi encerrado com `SIGTERM` aos 372 s, às 23:11 simuladas — **saída 0**, parada limpa sob carga, diário com 51.255 "aplicado" = 51.255 BUs no estado. Na noite real (1×, teto 95 req/s) essa latência não forma fila: 64 ÷ 0,5 s (latência medida no TSE em 2024) = 128 req/s > 95 |


### 3.4. Testes automatizados

* Motor: `cargo test --release -p urna-velox` (unidade e fixtures, inclusive `motor/tests/retirada.rs`) e
  `UV_DADOS=/home/dev/uvreal cargo test --release -p urna-velox -- --ignored --test-threads 1`: **13/13** (7 de
  integração — 2026 83/83, 2024 5.574/5.574 e 51/51, os três pré-eleição e o 3221 a frio — e 6 do matematicamente), sem
  mudança de número. Testes de unidade do workspace: 46, todos passando; `compilar_dev.sh` sem avisos (0 avisos também
  com `--all-targets`).
* Serviço × simulador (`aquisicao/tests/servico.rs`, processos filhos com porta livre):
  `UV_DADOS=/home/dev/uvreal cargo test --release -p uv-aquisicao --test servico -- --ignored --test-threads 1`:
  **6/6** em 352 s — `criterio_a_reduzido` (AM e RO a 120×: 2/2 idênticos, 5.102 BUs, 0 HTTP 429, 0 `aux.json` 404),
  `kill9_tres_vezes`, `rajada_de_429` (nenhuma requisição na pausa, pelo `simulador.log`), `reenvio_identico`,
  `anulacao_depois_de_publicada` (caso real MA 07315/0023/0354, 1º turno de 2024), `queda_do_simulador` (queda de 60 s
  reais no pico de AM e RO; ver a nota no próprio teste: o caminho "pendente pedida de novo" só aparece com fila, e é
  provado pelo C-queda completo).

## 4. Desempenho e recursos (item 6, medidos no B)

i7-14700 (28 threads), WSL2, 15 GB. Números da `saude.json` final do serviço (`p50_total`/`p99_total` = execução
inteira, pelo histograma; `max`), simulador e serviço na mesma máquina.

| medida | B 0.2.2 (12/10, 1ª sessão) | B 0.2.5 (código da outra sessão, hoje) | **B 0.2.6 (final)** |
|---|---|---|---|
| BU recebido → arquivo publicado gravado, p50 | 1,032 ms (janela) | 0,825 ms | **0,675 ms** |
| idem, p99 | 314,6 ms (janela) | 172,5 ms | **4,3 ms** |
| idem, máximo | 384,8 ms | 240,1 ms | **55,4 ms** |
| lote do motor (pacotes, estado, diário, matematicamente, eventos), p50 / p99 / máx | — | — | 0,055 / 0,345 / 52,1 ms |
| matematicamente por lote, p50 / p99 | 0,003 / 0,081 ms | 0,005 / 0,075 ms | 0,005 / 0,055 ms |
| gerar os JSON da publicação por lote, p50 / p99 | 0,41 / 2,98 ms | 0,285 / 1,495 ms | 0,245 / 1,135 ms |
| gravar os arquivos (thread `uv-gravacao`), p50 / p99 | (no motor) | 0,095 / 0,435 ms | 0,095 / 0,375 ms |
| `sync_data` de ~30 arquivos a cada 2 s, p50 / p99 / máx | (no motor, não medido) | (no motor, não medido) | 54,9 / 176,5 / 224,7 ms (thread `uv-sincronia`) |
| `saude.json` a cada 2 s na thread do motor, p50 / p99 | — | — | 21,3 / 28,4 ms |
| RSS do serviço: pico / final | 364,9 / 305,6 MB | 287,5 / 242,3 MB | 300,8 / 210,6 MB |
| CPU média do serviço (100 % = 1 núcleo) | 58,1 % | 52,4 % | 51,9 % |
| duração (noite de 9 h a 60×) | 538,5 s | 538,1 s | 537,8 s |

* **O p99 de 172 ms era o `fsync`**: a mesma medida que agora aparece na thread de sincronia (55 ms no p50, 177 ms no
  p99) ficava na thread do motor a cada 2 s; os BUs que chegavam nesse intervalo esperavam. Com ela fora, o p99 caiu
  ~40×. O que sobra de cauda é a própria `saude()` (21–28 ms a cada 2 s, percorre ~500 mil seções e as abrangências) e
  lotes grandes depois de uma rajada (máx. 52 ms) — para a meta da etapa 4 (BU → cliente < 50 ms no p99 a 60×) já
  cabe; uma `saude()` incremental é a próxima melhoria se for preciso.
* **Meta do ROADMAP, BU → publicação < 1 ms no p50: atingida** (0,675 ms) — na 0.2.2 não estava (1,03 ms).
* **Memória**: o motor (estado + catálogo) cabe na meta de < 150 MB; o serviço inteiro (rede, filas, cache de totais,
  amostras de 50 mil, buffers de 1 MB por pacote) tem pico de ~300 MB. O `M_ARENA_MAX`/`malloc_trim` da 0.2.4 é o que
  segura: sem ele o RSS subia sem parar a 60× (cada ciclo interpreta até 31,5 MB de `cs.json`).
* **Publicação só do que mudou** (desde a 0.2.2) e cache de totais por UF (0.2.4): gerar os JSON custa 0,25 ms no p50
  por lote; não é gargalo.
* **Rede no simulador**: pico de 5.714 req/s num segundo de relógio para um teto de 5.700 (0,25 % acima: o limitador
  espaça 1/rps, e a contagem por segundo de calendário pega a borda); no A, 951 para 950. Contra o TSE o teto é 95 por
  padrão, então o pior caso é ~96 num segundo, abaixo do limite de 100.

## 5. Teste real, sem BUs: pleito 3221 antes da eleição (item 7)

**Preparo** (13/10 12:46, binário 0.2.5; `-u.json` pré-apuração relidos: `fontes/u` movida para `fontes/u_antes_20261013_1246`, cópia do preparo anterior em `/home/dev/uvreal/2026-2t/preparo_antes_20261013_1246`):

```bash
/home/dev/uvreal/target-dev/release/uv-coletor preparar --pleito 3221 --saida /home/dev/uvreal/2026-2t/preparo \
  --fontes /home/dev/uvreal/2026-2t/preparo/fontes --atualizar
```

Código 2 em 19 s, **72 requisições**: 28 UFs (27 com seções), 5.757 municípios, **500.111 seções** (eram 494.044: o
`cs.json` de **SE** foi publicado), 11 eleições, 33 candidatos; **`to-p003221-cs.json` ainda 404** → `incompleto`.
`s.ts` de Presidente continua parcial (§7.2).

**Serviço contra o site real** (`/home/dev/uvreal/etapa3/real3221_final`, binário 0.2.6 copiado para `…/bin`):

```bash
uv-servico --pleito 3221 --ciclo ele2026 --candidatos /home/dev/uvreal/2026-2t/preparo/candidatos.json \
  --aceitar-incompleto --dados …/dados --saida …/saida --intervalo-cs 60 --rps 95
# depois de 1.860 s: kill -TERM <pid>; wait <pid>
```

| | resultado (14:28:05 → 14:59:05, **1.860 s**) |
|---|---|
| requisições | **868, todas `cs.json`**: 810 com 304, 31 com 404 (TO), o resto 200 (primeira leitura de cada UF e mudanças) |
| `aux.json` / BU | **0 / 0** — nenhuma seção publicada (500.111 listadas, `publicadas_cs` 0): seção sem disponibilização não gera requisição |
| HTTP 429 | **0**; teto 95 req/s, pausa 600 s; média real 0,47 req/s |
| `saude.json` | coerente: `estado` acompanhando → parado, `relogio_site` = hora de Brasília, `fila` 0, 34 abrangências esperadas, 0 completas, alertas `candidatos_incompleto`, `destinacao_presumida`, `secoes_esperadas_incoerentes` (Brasil × soma das UFs e 9 UFs parciais, §7.2), `cs_ausente` (TO) |
| recursos | RSS 46 MB (pico 65 MB), CPU 0,15 % |
| parada | `SIGTERM` → `servico.log` "sinal: parada limpa" → "parada"; **código de saída 0** |
| publicação a frio | 43 arquivos, os mesmos nomes do `uv-motor totalizar --candidatos /home/dev/uvreal/2026-2t/preparo/candidatos.json --dados <vazia> --saida …/frio`, **conteúdo idêntico** fora o campo `encerrada` (o serviço publica a apuração em curso, `false`; o `totalizar` fecha o lote, `true`); todos `"estado": "apurando"` |

Antes, a mesma coisa com o binário 0.2.5 por 2.100 s (13:44–14:19, `/home/dev/uvreal/etapa3/real3221`): 980
requisições, todas `cs.json` (918 com 304, 35 com 404 do TO), **0 `aux.json`, 0 HTTP 429**, 0,47 req/s em média, RSS
41 MB, CPU 0,14 %; publicação a frio com os 43 arquivos e os mesmos nomes do `uv-motor totalizar --candidatos`,
idênticos a ele fora o campo `encerrada` (o serviço publica a apuração em curso, `false`; o `totalizar` fecha o lote,
`true`), todos "apurando". O código de saída registrado ali (127) foi do `timeout` que o roteiro usava; o
`servico.log` mostra a parada limpa (`sinal` → `parada`). Por isso o teste final manda o `SIGTERM` direto e lê o código
com `wait`.

Orçamento de rede real desta sessão: 72 (preparar) + 980 (0.2.5) + ~112 (duas partidas de ~1 min interrompidas para
corrigir o alerta do `s.ts`, nunca simultâneas a outra) + 868 (final) ≈ **2.030 requisições**, todas `ele-c`/`cs.json`/
`-u.json`, abaixo das ~5 mil do handoff. Nenhum BU baixado do TSE.

## 6. Valores de situação (`st`) encontrados e tratamento (item 4)

Contados nas métricas das coletas (coluna `situacao` = `st` do hash escolhido no `aux.json`;
`/home/dev/uvreal/2024-1t/metricas_aux.csv`, `/home/dev/uvreal/2024-2t/metricas_2t.csv`,
`/home/dev/uvreal/metricas_{multi,nacional,retomada}.csv`) e nos `aux.json` reais de exemplo
(`/home/dev/uvreal/2024-1t/descoberta/`):

| onde | valor | 2024 1º t. | 2024 2º t. | 2026 1º t. | tratamento no `uv-servico` |
|---|---|---|---|---|---|
| BU (`hashes[].st`) | `Totalizado` | 478.025 | 97.392 | 489.561 | aplica |
| BU | `Recebido` | — | — | 2.677 | aplica como totalizado (o resultado bate com o oficial nos 83 arquivos) |
| BU | `Excluído` | 1 (MA 07315/0023/0354) | — | — | seção anulada: votos como nulos (`secao_anulada`); se chegar depois de aplicada, `anulacao` (retira e reaplica) |
| BU | (sem BU: `aux.json` 404 ou sem hash) | 22.315 | 1.471 | 19.058 | sem disponibilização no `cs.json`: não é pedida (salvo a sondagem do exterior); publicada e 404 persistente: pendente, pedida de novo (§1) |
| seção (`st`) | `Totalizada` | nos exemplos | | | normal |
| seção | `Nao instalada` | — | — | 41 (exterior) | conta em `s.st`, sem BU (`nao_instalada`) |
| seção | `Anulada` | | | | votos como nulos (`secao_anulada`/`anulacao`) |
| qualquer | outro valor | 0 | 0 | 0 | seção pendente com alerta `situacao_desconhecida` (não entra na totalização) |

`Anulada` e `Nao instalada` como `st` da seção vêm do formato do site (e do simulador); nos dados locais as anulações
aparecem como BU `Excluído` e as não instaladas como seção sem BU nem disponibilização.

## 7. Riscos para a noite de 25/10 e o que ficou pendente

1. **`cs.json` de TO ainda em 404** (13/10 12:46). Enquanto faltar, o `candidatos.json` sai `incompleto` e os BUs de TO
   cairiam em `fora_catalogo`. Regerar com `uv-coletor preparar --pleito 3221 … --atualizar` perto da eleição e na
   véspera ([OPERACAO.md](OPERACAO.md) §1.2).
2. **`s.ts` pré-apuração do 3221 parcial** em 9 UFs para Presidente (SP 33.403 de 91.326; RJ 13.661 de 38.738…) e
   Brasil 494.864 × soma das UFs 358.396. Com isso o fim automático da apuração pode vir cedo para essas UFs (é
   reaberto se chegar BU) e o do Brasil pode não vir — o serviço avisa na partida; na noite, `touch …/CONFERIR`. Mover
   `fontes/u` e regerar perto da eleição; se o TSE não atualizar esses arquivos, a conferência final é a manual.
3. **Destinação presumida "Válido"** (os `-u.json` pré-apuração não trazem `dvt`): conferir cassações/indeferimentos
   antes da noite.
4. **Sondagem do exterior**: no fim da noite ~1.400 `aux.json` de seções sem disponibilização (≤ 2 req/s, quase todos
   404, ~12 min), a única exceção à regra. No B: 1.405 sondadas, 1.364 agregadas, 41 não instaladas, 0 HTTP 429 (no
   simulador, que não tem a borda do TSE). Se preferir não arriscar 404 em sequência no TSE, `--sem-sondagem` (as 41
   não instaladas ficam fora de `s.st` do exterior e a conferência mostra a diferença).
5. **Disco do WSL**: o `sync_data` de ~30 arquivos leva 55 ms no p50 e 177 ms no p99 aqui (B final); agora fora do motor, mas
   num disco mais lento a proteção contra falta de energia fica proporcionalmente mais espaçada.
6. **IP de saída**: a regra de 100 req/s e a pausa de 10 min depois de 429 valem para o IP; o serviço não sabe de outro
   tráfego da mesma rede para o TSE.
7. **O ensaio automatizado de queda não exercita "pendente pedida de novo"**: com só AM e RO não há fila no limitador,
   então nenhuma seção está em coleta no instante da queda (testado com 10, 30 e 60 s). Quem prova a correção é o caso
   C5′ (20 UFs, `--timeout 2`). Fica como melhoria: um ensaio automatizado com fila (mais UFs ou `--concorrencia` baixa).
8. **Timeout da rede não escala com a aceleração** (`--timeout 30`): no simulador uma queda de 2 h simuladas dura 120 s
   reais, e cada tentativa espera 30 s pendurada; por isso a C5 não chega a esgotar as tentativas. Na noite real (1×) é
   o comportamento certo; só os ensaios acelerados ficam mais brandos que a realidade nesse ponto.
9. **Concorrência × latência** (C6): com 64 seções simultâneas a vazão é ~64 ÷ latência; com a latência do TSE em 2024
   (~0,5 s) dá ~128 req/s, acima do teto de 95. Se a latência na noite passar de ~0,65 s, a coleta fica abaixo do teto:
   subir `--concorrencia` (a taxa continua limitada a 95 pelo limitador).
10. Não verificado aqui (fora do escopo, ver auditoria): assinaturas do BU, percentuais por truncamento, limites de
   descompressão.

## 8. Como validar (sugestão) e commits/tags

Pastas dos ensaios desta sessão (WSL): `/home/dev/uvreal/etapa3/{A_final,B_final,C-kill9_final,C-429_final,C-reenvio_final,C-anulacao_final,C-queda_final,C-queda-timeout_final,C-rede_final,real3221_final}`
(binário 0.2.6) e, para comparação, `{A_0.2.5,B_0.2.5,C-*_0.2.5,real3221}` (0.2.5) e `{A_0.2.1,B_0.2.2}` (1ª sessão). Em
cada uma: `ensaio.log`, `simulador.log`, `simulador_resumo.json`, `servico.err` e `saida/{saude.json,conferencia.md,conferencia.json,secoes.log,servico.log,alertas.csv,eventos.csv,publicacao/}`.
Roteiros usados: `/mnt/d/git/lukeboh/urna-velox/.git/uv_tmp_{ensaio2,Bexp,C2,C3,final,real3221}.sh` (fora do
repositório).

| tag | commit | assunto |
|---|---|---|
| `v0.2.1` | `c921f09` | docs: handoff da etapa 3 |
| `v0.2.2` | `21244a2` | servico: uv-servico, uv-simulador e retirar/substituir BU no motor (etapa 3) |
| `v0.2.3` | `81e2ed1` | docs: módulo de escalabilidade segura (fora do escopo da etapa 3) |
| `v0.2.4` | `14a0298` | docs: relatório de auditoria — **com código do serviço não registrado no CHANGELOG** (§0) |
| `v0.2.5` | `bd00c4a` | docs: gráficos de desempenho (fora do escopo) |
| `v0.2.6` | (este commit) | servico: pendentes pedidas de novo, fsync fora do motor, alerta do s.ts; critérios A/B/C e teste real com o binário final |
