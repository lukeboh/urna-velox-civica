# Registro de Decisões de Arquitetura (ADR) — Urna Velox

> **O que é um ADR?** *Architecture Decision Record* (registro de decisão de arquitetura) é um texto curto que documenta **uma** decisão técnica importante: o contexto, o que foi decidido, as alternativas descartadas e as consequências. Este arquivo reúne todos os ADRs do projeto, numerados. Termos técnicos usados aqui estão explicados no [Glossário](GLOSSARIO.md).

* **Revisão:** 2 (11 de outubro de 2026) — substitui a versão 1 do mesmo dia.
* **Motivo da revisão:** a versão 1 escolheu o Rust com base em estimativas de memória e desempenho que não tinham sido medidas. A revisão mediu as três opções com o mesmo corpus sintético (ver [ESTIMATIVAS_MEMORIA_E_TEMPO.md](ESTIMATIVAS_MEMORIA_E_TEMPO.md)). O Rust continua escolhido, mas por motivos diferentes, e o modelo de dados mudou.
* **Revisão 3 (11/10/2026):** acrescenta ADR-013 a ADR-018, da fase de aquisição, com base em testes reais contra os servidores do TSE ([AQUISICAO.md](AQUISICAO.md), código em [`aquisicao/`](../aquisicao/)).
* **Evidências:** código e resultados brutos em [`bench/`](../bench/) (para repetir: `./bench/benchmark.sh`); gráficos em [`docs/img/`](img/).

Cada decisão abaixo segue o formato: **Contexto → Decisão → Alternativas consideradas → Consequências → Evidência**. Decisões futuras devem ser acrescentadas como novas entradas numeradas; uma decisão revogada é marcada como **Substituída por ADR-NNN**, nunca apagada.

## Índice

| # | Decisão | Status |
|---|---|---|
| [ADR-000](#adr-000--requisitos-e-métricas-de-referência) | Requisitos e como cada métrica é medida | Aceita |
| [ADR-001](#adr-001--linguagem-rust-plano-a) | Linguagem: Rust (Plano A), com justificativa revisada | Aceita (revisada) |
| [ADR-002](#adr-002--estado-em-vetor-denso-de-contadores) | Estado em vetor denso de contadores `u32` | Aceita — substitui o `DashMap` aninhado |
| [ADR-003](#adr-003--concorrência-parsers-paralelos-e-incremento-atômico-relaxado) | Concorrência: parsers paralelos + incremento atômico relaxado | Aceita |
| [ADR-004](#adr-004--uf-e-brasil-derivados-dos-municípios) | UF e Brasil derivados dos municípios, não propagados a cada voto | Aceita |
| [ADR-005](#adr-005--consultas-consistentes-durante-a-ingestão-contínua) | Consultas consistentes durante a ingestão contínua | Proposta (Fase 3) |
| [ADR-006](#adr-006--deduplicação-por-bitset-e-conflito-como-alerta) | Deduplicação por bitset; BU conflitante gera alerta | Aceita |
| [ADR-007](#adr-007--parser-ber-manual-com-rasn-como-referência) | Parser BER manual; `rasn` como referência nos testes | Aceita |
| [ADR-008](#adr-008--validação-por-cargo-considerando-o-número-de-vagas) | Validação por cargo × número de vagas | Aceita |
| [ADR-009](#adr-009--orçamento-de-memória-e-retenção-por-seção) | Orçamento de memória; retenção por seção é modo opcional | Aceita |
| [ADR-010](#adr-010--ingestão-direto-dos-pacotes-zip) | Ingestão direto dos pacotes `.zip`, sem extrair arquivos | Aceita |
| [ADR-011](#adr-011--verificação-criptográfica-é-etapa-separada-e-opcional) | Verificação criptográfica é etapa separada e opcional | Aceita |
| [ADR-012](#adr-012--tokio-fora-do-núcleo-até-a-fase-4) | `tokio` fora do núcleo até a Fase 4 | Aceita |
| [ADR-013](#adr-013--fonte-dos-bus-api-de-resultados-na-noite-da-eleição-dados-abertos-depois) | Fonte dos BUs: API de resultados (noite da eleição) e zip de dados abertos (eleição encerrada) | Aceita |
| [ADR-014](#adr-014--coleta-assíncrona-http2-isolada-no-crate-de-aquisição) | Coleta assíncrona (HTTP/2) isolada no crate de aquisição | Aceita |
| [ADR-015](#adr-015--controle-de-taxa-teto-de-100-reqs-e-aimd-guiado-pelos-http-429) | Teto de 100 req/s em código + AIMD guiado por HTTP 429 | Aceita |
| [ADR-016](#adr-016--entrega-ao-núcleo-pacote-uvpk-por-uf-e-canal-em-memória) | Entrega ao núcleo: pacote `.uvpk` por UF + canal em memória | Aceita |
| [ADR-017](#adr-017--leitor-do-bu-real-envelope--conteúdo-leitura-verificada-conferido-com-decodificador-independente) | Leitor do BU real com leitura verificada, conferido com `asn1tools` | Aceita |
| [ADR-018](#adr-018--catálogo-layout-antes-da-eleição-nos-testes-extraído-dos-bus) | Catálogo antes da eleição; nos testes, extraído dos BUs | Aceita (testes) |
| [ADR-019](#adr-019--retirar-e-substituir-um-bu-no-estado-reenvio-e-anulação-depois-de-totalizada) | Retirar e substituir um BU no estado | Aceita |
| [ADR-020](#adr-020--site-simulado-local-uv-simulador-para-provar-o-serviço) | Site simulado local (`uv-simulador`) para provar o serviço | Aceita |
| [ADR-021](#adr-021--distribuição-massiva-via-static-file-edge-broadcast-sem-push-central) | Distribuição massiva via *Static-File Edge Broadcast* (sem push central) | Aceita (v2.0); storage revisto pela ADR-030 |
| [ADR-022](#adr-022--particionamento-dinâmico-por-rendezvous-hashing-sem-coordenação-central) | Particionamento dinâmico por *Rendezvous Hashing* sem coordenação central | Aceita (v2.0) |
| [ADR-023](#adr-023--zero-trust-e-validação-por-quórum-criptográfico-no-cliente-ed25519) | Zero-Trust e validação por quórum criptográfico no cliente (Ed25519) | Aceita (v2.0) |
| [ADR-024](#adr-024--webassembly-wasm-para-auditoria-e-consenso-no-navegador) | WebAssembly (Wasm) para auditoria e consenso no navegador | Aceita (v2.0) |
| [ADR-025](#adr-025--spa-pwa-em-vez-de-aplicativos-nativos-mobile) | SPA / PWA em vez de aplicativos nativos mobile (Android/iOS) | Aceita (v2.0) |
| [ADR-026](#adr-026--difusão-em-tempo-real-no-uv-servico-websocket-com-último-valor-por-tópico) | Difusão em tempo real no `uv-servico`: WebSocket com último valor por tópico | Aceita (etapa 4) |
| [ADR-027](#adr-027--página-de-resultados-embutida-no-binário-e-marcos-da-noite-calculados-no-servidor) | Página de resultados embutida no binário e marcos calculados no servidor | Aceita (etapa 5) |
| [ADR-028](#adr-028--federação-de-órgãos-um-nó-por-órgão-limite-por-ip-de-saída-hrw-ponderado-e-troca-de-bus-pelo-storage) | Federação de órgãos: limite por IP de saída, HRW ponderado e troca de BUs direto entre os nós | Aceita (v2.0, `2.0.0-alpha.3`; revista na `alpha.5`) |
| [ADR-029](#adr-029--vários-órgãos-na-mesma-infraestrutura-ip-de-saída-dedicado-por-órgão) | Vários órgãos na mesma infraestrutura: IP de saída dedicado por órgão | Aceita (v2.0, `2.0.0-alpha.8`) |
| [ADR-030](#adr-030--entrega-aos-leitores-cada-nó-é-origem-dos-próprios-arquivos-e-a-cdn-só-faz-cache) | Entrega aos leitores: cada nó é origem dos próprios arquivos e a CDN só faz cache | Aceita (v2.0, `2.0.0-alpha.8`) |
| [ADR-031](#adr-031--licença-source-available--polyform-noncommercial-com-dupla-licença) | Licença: Source-Available / PolyForm Noncommercial 1.0.0, com dupla licença | Aceita (`0.6.15`) |

---

## ADR-000 — Requisitos e métricas de referência

**Contexto.** A versão 1 trazia metas de RAM diferentes em cada documento (README: "poucos gigabytes"; ADR: < 500 MB; ARCHITECTURE: "< 200 a 500 MB"; ROADMAP: < 400 MB) e não dizia o que cada número mede. Com arquivos mapeados em memória, o RSS no Linux e o *Working Set* no Windows incluem as páginas do arquivo, então o mesmo programa pode "consumir" 300 MB ou 3 GB conforme a métrica.

**Decisão.** Ficam valendo estes requisitos e definições:

| Requisito | Meta | Como medir |
|---|---|---|
| Totalização de eleição geral (~500 mil BUs) | < 5 s em desktop de 8+ núcleos, BUs já disponíveis | tempo de parede do laço de ingestão + consolidação |
| Memória do motor | **< 150 MB** sem retenção por seção | pico de memória anônima/privada do processo, **excluindo** os bytes dos BUs de entrada e páginas de arquivos mapeados |
| Memória com retenção por seção | < 1 GB | idem, ver ADR-009 |
| Latência de atualização de um agregado | < 1 ms | do fim do parsing do BU até o valor visível para consulta |
| Inicialização da CLI | < 50 ms | `uv-motor --version` |

A eleição de referência é a **geral de 2026**: 500 mil seções, 5.570 municípios, 27 UFs, cargos Presidente, Governador, Senador (**2 vagas**), Deputado Federal e Deputado Estadual/Distrital. O exterior (UF `ZZ`) entra como uma "UF" adicional.

**Consequências.** Todos os documentos usam essas metas. Os benchmarks reportam o pico de RSS e o tamanho do corpus separadamente, para que a memória do motor possa ser isolada.

---

## ADR-001 — Linguagem: Rust (Plano A)

**Contexto.** A versão 1 justificava o Rust principalmente por memória ("ordens de magnitude" menos que Java) e por "concorrência perfeita". As medições mostraram que:

* com o estado em vetor denso (ADR-002), **o estado ocupa os mesmos ~36 MB nas três linguagens** — bytes são bytes;
* o que separa as linguagens é a **vazão do parsing** (Rust 3,3 µs por BU, Java 5,9 µs, Node 8,5 µs) e a **memória do runtime** (pico do processo sem o corpus: Rust 40 MB, Java 544 MB, Node 475 MB com 28 threads), não a estrutura de contagem;
* a forma "óbvia" em cada linguagem (mapas aninhados) estoura o orçamento **em qualquer uma delas**, inclusive em Rust (ver ADR-002).

**Decisão.** Manter **Rust** como linguagem do motor, pelos motivos abaixo, em ordem de peso:

1. **Vazão de parsing BER previsível e a mais alta medida** — parser manual sem cópia, sem pausas de coleta de lixo e sem aquecimento de JIT.
2. **Binário único** sem runtime instalado (Windows, Linux, macOS), o que importa para auditores e partidos que vão rodar a ferramenta em máquinas comuns.
3. **Mesmo código compilando para WebAssembly**, o que permite uma versão de auditoria no navegador sem reescrever o parser. A versão 1 tratava "rodar no navegador" como motivo para migrar ao JavaScript; isso não procede.
4. Memória do processo mais baixa, porque não há runtime gerenciado.

**Alternativas consideradas.**

| | Plano A — Rust | Plano B — JavaScript (Node/Bun) | Plano C — Java 21 |
|---|---|---|---|
| Parser recomendado | manual sobre `&[u8]` | manual sobre `Uint8Array` | manual sobre `byte[]` |
| Parser com biblioteca | `rasn` (aloca structs) | `asn1js` (árvore de objetos, muito lenta) | Bouncy Castle (árvore de objetos) |
| Estado recomendado | `Vec<AtomicU32>` | `Uint32Array` em `SharedArrayBuffer` + `Atomics.add` | `AtomicIntegerArray` |
| Paralelismo | threads nativas | `worker_threads` (~10 MB cada) | threads nativas |
| Distribuição | binário único | exige runtime (ou binário empacotado grande) | exige JRE (ou GraalVM Native Image) |
| Inicialização | ~ms | ~40 ms (Node) | ~100–500 ms (JVM) |
| **500 mil BUs, 1 thread (medido)** | **1,66 s** | 4,24 s (Node) | 2,95 s |
| **500 mil BUs, 28 threads (medido)** | **0,18 s** | 0,59 s (Node) | 0,28 s |
| Idem com a biblioteca ASN.1 | 1,16 s (`rasn`) | 139 s (`asn1js`) | 9,6 s (Bouncy Castle) |
| **Pico do processo sem o corpus, 28 threads** | **40 MB** | 475 MB | 544 MB (`-Xmx3g`) |

Os tempos e a memória de cada combinação estão em [ESTIMATIVAS_MEMORIA_E_TEMPO.md](ESTIMATIVAS_MEMORIA_E_TEMPO.md) §4 e §5. Correções em relação à versão 1:

* **Java:** a "pegada de memória em ordens de magnitude" só vale com objetos encaixotados (*boxing*). Com vetores de primitivos a diferença some no estado e fica no runtime (JVM + folga de heap). O ZGC tem pausas abaixo de 1 ms; pausa de GC não é o problema. O custo de inicialização da JVM pesa pouco num processamento de segundos.
* **JavaScript:** a concorrência não é "limitada": `SharedArrayBuffer` + `Atomics` permitem estado compartilhado entre workers sem cópia. Um worker do Node custa ~10 MB, não uma VM inteira. Não há necessidade de N-API para ler BER.
* **Rust:** o `rasn` **não** é zero-copy; ele decodifica para structs com `Vec` e buffers próprios (ver ADR-007).

**Critérios de pivot (revistos).**

* **Para o Plano C (Java):** necessidade de reutilizar componentes homologados distribuídos só como `.jar` (validação ICP-Brasil, bibliotecas oficiais do TSE). Nesse caso, manter o parser manual e o estado denso também em Java.
* **Para o Plano B (JavaScript):** **não** é mais critério "rodar no navegador" (coberto por WASM). Seria critério apenas se o projeto deixar de ter um motor de totalização e virar só uma camada de visualização.

**Consequências.** O ganho do Rust deve ser comprovado continuamente: os benchmarks de `bench/` fazem parte do repositório e rodam nas três linguagens.

---

## ADR-002 — Estado em vetor denso de contadores

**Contexto.** A versão 1 modelava o estado como `DashMap<(município, cargo), TallyBreakdown>`, onde cada `TallyBreakdown` continha **dois outros `DashMap`** (votos nominais e de legenda). Um `DashMap` cria por padrão `4 × núcleos lógicos` partições (arredondado para potência de 2), e cada partição ocupa memória mesmo vazia. Com ~27,8 mil pares município × cargo, isso dá 55,7 mil mapas.

Medições (eleição geral, ~4,5 M combinações município × cargo × votável):

| Desenho | Memória do estado |
|---|---|
| `DashMap` aninhado, 8 partições/mapa (máquina de 2 threads) | 188 MB |
| `DashMap` aninhado, 64 partições/mapa (16 threads) | 594 MB |
| `DashMap` aninhado, 128 partições/mapa (28–32 threads, ex.: i7-14700) | ~1.050 MB |
| só os 27,8 mil `TallyBreakdown` vazios, 64 partições | 438 MB |
| tabela hash plana (`hashbrown<u64,u32>`) | 136 MB |
| **vetor denso com todos os candidatos de cada UF em cada município (9,35 M contadores)** | **35,7 MB** |

O consumo do desenho original **cresce com o número de núcleos da máquina**, o oposto do desejado.

**Decisão.** O estado é um único vetor contíguo de contadores `AtomicU32`, indexado por um **layout pré-computado** a partir dos dados públicos de candidatura (publicados antes da eleição):

```
slot = base[município, cargo] + deslocamento
deslocamento = índice do candidato na UF/cargo        (voto nominal)
             | n_candidatos + (partido − 10)          (voto de legenda)
             | n_candidatos + 31                       (branco)
             | n_candidatos + 32                       (nulo)
             | n_candidatos + 33                       (comparecimento)
```

* Cada região (município, cargo) reserva espaço para **todos** os candidatos daquela UF/cargo, mesmo os que não recebem voto no município. Isso troca ~2× de espaço (9,35 M em vez de ~4,5 M contadores) por indexação sem hash e sem realocação.
* `u32` é suficiente: nenhum total por município se aproxima de 4,29 bilhões. Os totais de UF e Brasil são somados em `u64` na consolidação (ADR-004).
* O mapeamento "número do candidato → índice" é uma tabela por (UF, cargo), carregada com o layout.

**Alternativas consideradas.** `DashMap` aninhado (rejeitado: memória proporcional aos núcleos, travas por partição); tabela hash plana (aceitável, 4× mais memória e um hash por voto); vetor denso só com pares observados (metade da memória, mas exige conhecer de antemão quem recebe voto em cada município — não é possível antes da apuração).

**Consequências.** Memória do estado previsível e calculável antes da eleição (`Σ regiões × 4 bytes`). Candidatos não previstos no layout (dados de candidatura desatualizados) caem num contador de "votável desconhecido" por região e geram alerta. A eleição municipal usa o mesmo esquema (0,94 M contadores, 3,6 MB).

**Evidência.** `bench/rust/src/bin/membench.rs`, `bench/results/mem_rust.txt`; Figura 1 em [ESTIMATIVAS](ESTIMATIVAS_MEMORIA_E_TEMPO.md).

---

## ADR-003 — Concorrência: parsers paralelos e incremento atômico relaxado

**Contexto.** A versão 1 previa "centenas de threads" escrevendo em mapas concorrentes e chamava as leituras de *lockless*. O `DashMap` usa trava de leitura/escrita por partição, e o `u64` dentro dele não é atômico; as leituras não são livres de trava.

**Decisão.**

* **Modo lote (`ingest`):** um *pool* com uma thread por núcleo lógico. Cada thread pega blocos de 64 BUs de um contador atômico, faz o parsing e aplica `fetch_add(Relaxed)` diretamente no vetor denso. Como os 9,35 M contadores estão espalhados, a disputa por linha de cache é rara.
* **Sem cópia por thread:** acumular uma cópia do vetor em cada thread e somar no fim custaria 28 × 36 MB ≈ 1 GB numa máquina de 28 threads. Rejeitado.
* **`rayon`** continua sendo uma opção de implementação para o *pool*; o benchmark usa `std::thread::scope` com divisão por blocos para ficar idêntico nas três linguagens.

**Alternativas consideradas.** `DashMap` com propagação em três níveis (medido: 8,4 × mais lento que o vetor denso numa thread e 10,8 × com 28 threads; ver ESTIMATIVAS §3.3); thread agregadora única recebendo deltas por canal (adequada ao modo contínuo, ver ADR-005); particionar o trabalho por UF para que cada thread seja dona de uma região (elimina atômicos, mas desbalanceia: SP e MG concentram ~30% das seções).

**Consequências.** O termo *lockless* sai da documentação; o que existe são incrementos atômicos sem trava. A consistência entre níveis durante a ingestão é tratada no ADR-005.

---

## ADR-004 — UF e Brasil derivados dos municípios

**Contexto.** A versão 1 incrementava três agregados (município, UF, Brasil) a cada voto. Isso triplica as escritas e concentra todas as threads nos mesmos contadores nacionais de Presidente — exatamente o tipo de disputa que o projeto quer evitar.

**Decisão.** Só o nível municipal é escrito durante a ingestão. UF e Brasil são **somas sobre as regiões municipais**, calculadas sob demanda ou ao fim do lote. A consolidação completa da eleição geral levou **~6 ms numa thread** (Rust), dentro da meta de latência.

**Consequências.** Consultas de UF/Brasil custam alguns milissegundos em vez de um acesso direto; aceitável para CLI e API. Se a API da Fase 4 precisar de consultas nacionais com frequência muito alta, um *cache* recalculado a cada lote resolve.

---

## ADR-005 — Consultas consistentes durante a ingestão contínua

**Status:** proposta, a validar na Fase 3.

**Contexto.** Lendo o vetor enquanto threads escrevem, uma consulta pode ver o voto de um BU num município e ainda não na soma da UF, ou ver parte dos cargos de um BU. A versão 1 prometia "cópias atômicas do estado", o que não é possível com mapas concorrentes.

**Decisão proposta.** No modo `watch`, os parsers enviam o resultado de cada BU (lista compacta de `(slot, votos)`) para uma **thread agregadora única**, que aplica os deltas em lotes e publica uma **época** (`AtomicU64`) ao fim de cada lote. As consultas leem a época, copiam a região de interesse e conferem se a época não mudou (padrão *seqlock*); a API retorna sempre o número da época, ou seja, quantos BUs estão contidos na resposta.

**Justificativa de capacidade.** Uma eleição geral tem ~340 votáveis por BU; a 100 mil BUs/s isso dá ~34 M incrementos/s, ao alcance de uma única thread com acesso sequencial a memória.

---

## ADR-006 — Deduplicação por bitset e conflito como alerta

**Contexto.** A versão 1 usava `DashSet<SectionId>` com chave `(id_eleição, UF, município, zona, seção)`. Problemas: (a) um BU de eleição geral contém **duas eleições** (federal e estadual), então `id_eleição` não identifica o BU; (b) faltava o **turno**; (c) um BU repetido com **conteúdo diferente** era descartado em silêncio, o que esconde exatamente o caso que uma auditoria precisa ver.

**Decisão.**

* Chave do BU: `(pleito, turno, município, zona, seção)`.
* As seções são conhecidas antes da eleição, então cada uma recebe um **ordinal**; a deduplicação é um bitset de 500 mil bits (**61 KB**, contra 17 MB do `DashSet`).
* Junto do bit, guarda-se o hash (SHA-512 truncado em 8 bytes) do BU aceito. Um segundo BU da mesma seção com hash igual é descartado; com hash **diferente**, vira **alerta de conflito** com os dois arquivos citados, e o primeiro continua valendo até decisão humana.
* Seções agregadas, urnas de contingência e BUs emitidos pelo Sistema de Apuração (SA) recebem tratamento explícito no normalizador, documentado à medida que as especificações do TSE forem mapeadas na Fase 1.

**Consequências.** +4 MB para os hashes de 500 mil seções. Seções fora do cadastro previsto geram alerta em vez de entrar silenciosamente.

---

## ADR-007 — Parser BER manual com `rasn` como referência

**Contexto.** A versão 1 citava o `rasn` e "parsing zero-copy" como a mesma coisa. O `rasn` decodifica para structs com `Vec`, `String` e buffers próprios: é seguro e fiel ao esquema, mas aloca.

**Decisão.**

* **Caminho rápido:** leitor BER escrito à mão, que percorre `&[u8]` e entrega cada voto a um *sink* (agregador) sem alocar. Medido: **~3,3 µs por BU** numa thread (eleição geral, 17 KB/BU).
* **Referência:** o esquema ASN.1 oficial do TSE é compilado com `rasn`; os testes decodificam cada *fixture* com os dois e exigem resultado idêntico (teste diferencial). O benchmark já faz essa conferência por *checksum* do vetor de totais.
* Divergência entre os dois é falha de teste, nunca "escolher um".

**Consequências.** O parser manual precisa acompanhar mudanças do esquema entre eleições (2022, 2024, 2026); o teste diferencial detecta quando ele ficou para trás. O `rasn` custou **~29 µs por BU** (9 × mais lento) — aceitável como referência, caro como caminho principal.

---

## ADR-008 — Validação por cargo considerando o número de vagas

**Contexto.** A versão 1 validava `nominais + legenda + brancos + nulos = comparecimento`. Em 2026 o Senado renova **2/3** das cadeiras: cada eleitor vota em **dois** senadores, e a soma do cargo é `2 × comparecimento`. A regra original marcaria todos os BUs como inválidos.

**Decisão.** Para cada cargo: `Σ votos do cargo = comparecimento × vagas(cargo, pleito)`, com `vagas` vindo da configuração do pleito. BU que falha a regra é isolado em fila de auditoria sem interromper a ingestão.

**Consequências.** A configuração de cada pleito (cargos, vagas, códigos) passa a ser um arquivo versionado junto do layout.

---

## ADR-009 — Orçamento de memória e retenção por seção

**Contexto.** A auditoria da Fase 4 (comparecimento atípico, comparação seção a seção) exige guardar resultados **por seção**, o que a versão 1 não orçava. Com ~340 votáveis por BU na eleição geral, mesmo um formato compacto (`u16` índice + `u16` votos) ocupa ~1,3 KB por seção, ou **~0,65 GB** para 500 mil seções.

**Decisão.**

* **Modo padrão:** só agregados municipais → orçamento de **150 MB** para o processo (excluindo a entrada).
* **Modo `--por-secao`:** guarda os votos de cada seção em blocos compactos por município, com contagens em *varint* (a maioria das contagens por seção cabe em 1 byte), estimado em **0,35–0,65 GB**; orçamento de **1 GB**.
* Alternativa sem memória extra: manter apenas o índice `(seção → arquivo, posição)` e reler o BU sob demanda (~3 µs por BU).

**Consequências.** A meta "< 500 MB" da versão 1 é substituída por duas metas explícitas, uma por modo.

---

## ADR-010 — Ingestão direto dos pacotes `.zip`

**Contexto.** A meta de segundos para 500 mil BUs não depende do parser e sim do sistema de arquivos: abrir 500 mil arquivos soltos no NTFS, com o antivírus verificando cada um, leva dezenas de segundos ou minutos. Os dados abertos do TSE são distribuídos em pacotes por UF.

**Decisão.** Ler os BUs **de dentro dos pacotes**, sem extrair para disco, com leitura sequencial do arquivo e descompressão em paralelo por entrada. `memmap2` é opcional; quando usado, a memória mapeada não entra no orçamento do ADR-000.

**Consequências.** O benchmark de referência (§4 das estimativas) mede a totalização com os BUs já em memória; o tempo de leitura e descompressão é estimado à parte.

---

## ADR-011 — Verificação criptográfica é etapa separada e opcional

**Contexto.** Medido: calcular o SHA-512 de cada BU custa **~7 × mais** que fazer o parsing e a totalização (eleição geral: 11,8 s contra 1,7 s numa thread). A verificação de assinaturas tende a custar ainda mais.

**Decisão.** `ingest` totaliza sem hash por padrão (exceto o hash curto de deduplicação, ADR-006, que pode usar uma função não criptográfica rápida). `audit` é um subcomando separado que calcula SHA-512 e verifica assinaturas, também em paralelo.

**Consequências.** O resultado de `ingest` informa explicitamente se a integridade criptográfica foi verificada ou não.

---

## ADR-012 — `tokio` fora do núcleo até a Fase 4

**Contexto.** O trabalho do motor é CPU, não I/O assíncrono. Misturar o runtime do `tokio` com o *pool* de CPU nas Fases 1–3 só acrescenta complexidade.

**Decisão.** As Fases 1–3 usam threads nativas (e `notify` síncrono no modo `watch`). O `tokio` entra na Fase 4, isolado na camada HTTP (`axum`), conversando com o motor por canais.

**Consequências.** Binário menor e inicialização mais rápida nas fases iniciais.

---

## ADR-013 — Fonte dos BUs: API de resultados na noite da eleição, dados abertos depois

**Status:** aceita (11/10/2026), com base nas medições de [AQUISICAO.md](AQUISICAO.md).

**Contexto.** O TSE publica o BU bruto de cada seção em dois lugares: (1) no site de resultados (`resultados.tse.jus.br`), seção a seção, à medida que os BUs chegam na noite da eleição; (2) nos dados abertos (`cdn.tse.jus.br`), um zip por UF com BU, RDV, log e assinaturas de todas as seções, publicado dias depois. Os arquivos de resultado já totalizados também existem, mas o Urna Velox totaliza a partir dos BUs e não os usa.

**Decisão.** Na noite da eleição, a fonte é a **API de resultados**: `ele-c.json` (pleitos) → `<uf>-p<pleito>-cs.json` (seções da UF) → `…-aux.json` (arquivos e hash da seção) → `…-bu.dat`. Uma seção custa duas requisições. Para uma eleição encerrada, o caminho preferido é o **zip de dados abertos**: o pacote inteiro (uma requisição por UF; o DF, 1,27 GB, veio em 13 s) ou, com pouca banda, só as entradas `-bu.dat` por HTTP Range (7,3 % do pacote, uma requisição por BU). Para testes, o acervo local desses zips (prompt em [PROMPT_ACERVO_BUS.md](PROMPT_ACERVO_BUS.md)) elimina a rede.

**Alternativas.** (a) *Só dados abertos*: não serve para a noite da eleição (publicação dias depois). (b) *Raspar a aplicação web*: frágil e sem ganho, ela consome a mesma API. (c) *Arquivos de resultado*: violam a premissa do projeto (auditar a partir do BU).

**Consequências.** O coletor precisa da lista de seções antes de começar (`cs.json`, 31,5 MB para o Brasil) e da escolha do hash quando uma seção tem mais de um conjunto de arquivos (regra: o último "Totalizado"; sem nenhum, o último com BU). Seções listadas sem BU publicado (aux.json 404, ~1–3 %) são registradas, não são erro de coleta.

---

## ADR-014 — Coleta assíncrona (HTTP/2) isolada no crate de aquisição

**Status:** aceita. Complementa o ADR-012.

**Contexto.** A coleta é limitada por **latência**, não por CPU nem banda: cada requisição sem cache no CDN leva ~0,5 s (até a origem) e um BU tem ~16 KB. Pela lei de Little, vazão = requisições em andamento ÷ latência; para 100 req/s são precisas ~50 requisições simultâneas.

**Decisão.** O `uv-coletor` (crate `aquisicao/`) usa `tokio` + `reqwest` com HTTP/2 (ALPN, `rustls`), várias requisições multiplexadas por conexão e concorrência configurável. O núcleo continua síncrono (ADR-012): a aquisição conversa com ele por um canal (`crossbeam`) e por pacotes em disco (ADR-016), nunca compartilhando o runtime.

**Alternativas.** (a) *Threads bloqueantes*: 100–500 threads só para esperar rede. (b) *HTTP/1.1*: mesma vazão medida, mas uma conexão TCP+TLS por requisição em andamento. (c) *HTTP/3*: o CDN anuncia (`alt-svc: h3`), mas não há ganho mensurável num caminho limitado pela origem; fica para depois.

**Consequências.** Medido: a vazão cresce linearmente com a concorrência até o teto de 100 req/s, atingido com ~64 requisições em andamento; HTTP/1.1 e HTTP/2 empatam com a mesma concorrência; mais conexões HTTP/2 não mudam nada nessa faixa. O servidor dos dados abertos (`cdn.tse.jus.br`) só fala HTTP/1.1 e responde em ~5 ms da rede de referência.

---

## ADR-015 — Controle de taxa: teto de 100 req/s e AIMD guiado pelos HTTP 429

**Status:** aceita.

**Contexto.** As respostas trazem `x-ratelimit-limit: 2000, 2000;w=1`, mas esse limite é da **origem** e é **compartilhado por todos os clientes** (`x-ratelimit-remaining` varia com o tráfego de terceiros). O limite que vale para um coletor é o da **borda Akamai, por IP**: ao passar de ~200 req/s, a borda respondeu `429` ("Error from child") e manteve o IP numa caixa de penalidade por ~8 minutos, recusando até requisições isoladas. Em rede corporativa esse IP é o NAT de muita gente.

**Decisão.** (1) **Teto absoluto de 100 req/s** por processo, regra do projeto (acima disso a borda do TSE responde HTTP 429 e põe o IP numa caixa de penalidade), imposto em código (`TETO_RPS`, sem rajadas; `--rps` maior é reduzido; teste de unidade). (2) Nas coletas longas, controle **AIMD** abaixo do teto: começa em 40 req/s, soma 1 req/s a cada segundo sem 429 até 100 e, a cada 429, corta a taxa pela metade e pausa todas as requisições por 2 s. (3) Novas tentativas com espera exponencial para 429/5xx/erro de rede, passando pelo mesmo limitador. (4) `User-Agent` identificando o projeto. (5) O `setup` do teste real detecta a caixa de penalidade e espera, com uma tentativa por minuto.

**Alternativas.** (a) *Ir até os 2.000 req/s do cabeçalho*: derruba o IP inteiro por minutos (aconteceu nos primeiros testes, antes do teto). (b) *Vários IPs ou processos em paralelo*: contorna o limite; não fazemos. (c) *Sem teto, só com retentativas*: cada 429 renova a penalidade.

**Consequências.** A 100 req/s a API rende no máximo 50 seções/s (duas requisições por seção): o Brasil inteiro (517 mil seções) leva **~3 horas** por esse caminho. Na noite da eleição isso acompanha a chegada dos BUs (ver AQUISICAO.md), mas reprocessar uma eleição encerrada é muito mais rápido pelo zip de dados abertos (ADR-013), que custa uma requisição por UF.

---

## ADR-016 — Entrega ao núcleo: pacote `.uvpk` por UF e canal em memória

**Status:** aceita.

**Contexto.** O ADR-010 proíbe 500 mil arquivos soltos no disco. O núcleo precisa de duas formas de receber BUs: em lote (reprocessar) e em fluxo (noite da eleição).

**Decisão.** Cada BU recebido vira um `Recebido { registro, bytes }` num canal. Um consumidor (1) grava em `<uf>.uvpk` — arquivo só de acréscimo com cabeçalho fixo de 52 bytes por registro (pleito, município, zona, seção, UF, situação, origem, SHA-256 do `.vsc`) seguido do BU **intocado** — e/ou (2) repassa a threads de totalização. O totalizador em lote abre os pacotes com `mmap`. Ao reabrir, um registro incompleto no fim é descartado e a coleta retoma pulando as seções já gravadas.

**Alternativas.** (a) *Zip*: escrita incremental e retomada são ruins. (b) *Banco (SQLite)*: dependência e custo por inserção sem necessidade. (c) *Um arquivo por BU*: o problema do ADR-010.

**Consequências.** 6.969 BUs do DF = 108 MB num arquivo; o Brasil ≈ 8 GB em 28 arquivos. Os bytes originais permitem verificar hash e assinaturas depois (ADR-011).

---

## ADR-017 — Leitor do BU real: envelope + conteúdo, leitura verificada, conferido com decodificador independente

**Status:** aceita. Especializa o ADR-007 para o formato oficial de 2026.

**Contexto.** O `…-bu.dat` é um `EntidadeEnvelopeGenerico` cujo `conteudo` (OCTET STRING) contém o `EntidadeBoletimUrna` (especificação `bu.asn1`, `IMPLICIT TAGS`). Vários campos irmãos têm a mesma tag universal (`SEQUENCE`), e os bytes chegam da rede.

**Decisão.** Leitor TLV manual sem alocação (`aquisicao/src/bu.rs`): acha o `conteudo` no envelope, distingue os `SEQUENCE` pela posição (cabeçalho, urna, identificação, resultados) e entrega seção, cargos e votos a um `Sink`. **Todo acesso é verificado**: BU truncado ou malformado devolve erro, nunca pânico. A conferência usa `asn1tools` com o `bu.asn1` oficial (`scripts/conferir_referencia.py`), comparando total a total.

**Consequências.** A identificação lida no BU é conferida com a do endereço/nome do arquivo (contador de divergências). Mudanças de formato entre eleições ficam neste módulo.

---

## ADR-018 — Catálogo (layout) antes da eleição; nos testes, extraído dos BUs

**Status:** aceita para os testes; produção pendente (Fase 2).

**Contexto.** O ADR-002 exige conhecer municípios, seções e candidatos antes do primeiro BU.

**Decisão.** O catálogo (`catalogo.txt`) tem municípios e seções (do `cs.json`) e candidatos por (UF, cargo), com tabela direta número → índice. Nos testes, os candidatos são extraídos dos próprios BUs coletados (`uv-coletor catalogo`), em etapa medida à parte. Em produção virão do cadastro de candidaturas publicado antes da eleição.

**Consequências.** Voto em número fora do catálogo é contado como "desconhecido" e vai para auditoria, sem parar a totalização.

---

## ADR-019 — Retirar e substituir um BU no estado (reenvio e anulação depois de totalizada)

**Status:** aceita (etapa 3, `0.2.2`). Complementa o ADR-006 (idempotência).

**Contexto.** Até a `0.2.x` o estado só somava, e o bitset `vistos` tratava um segundo BU da mesma seção como duplicado. Na noite há dois casos que exigem desfazer: o `aux.json` da seção passa a apontar outro hash (BU reenviado, idêntico ou não) e a seção já totalizada vira "Anulada" / BU "Excluído" (caso real: MA 07315/0023/0354, 2024), quando os votos dela passam a contar como nulos.

**Decisão.** `Estado::retirar` percorre o BU com o mesmo `Agregador` e o mesmo caminho de leitura de `aplicar`, com `Sinal::Retirar` (`fetch_sub` em votos, comparecimento, `aptos`, `secoes`, `bus`, votáveis desconhecidos, cargos inválidos e identificação divergente) e limpa o bit de `vistos`. `substituir` = retirar o antigo (com a anulação com que foi aplicado) + aplicar o novo; anulação depois de totalizada = substituir o BU por ele mesmo com `anulada = true`. A anulação passa a ser dada por quem aplica (`aplicar_secao`, situação lida do `aux.json`), não só pelas métricas. Quem decide qual BU vale em cada seção é o serviço (diário de seções, `uv-servico`), não o motor: o motor não guarda bytes de BU. `Acompanhamento::bu_alterado` reavalia a circunscrição e anexa o motivo a qualquer revogação de "matematicamente", que vira alerta.

**Alternativas.** (a) *Recalcular tudo do zero a cada troca*: 0,1–0,2 s por troca na eleição geral; correto, mas desnecessário. (b) *Guardar no estado os votos de cada seção*: memória de ~500 mil seções × regiões; o próprio BU (no `.uvpk`) já é esse registro.

**Consequências.** Aplicar e retirar o mesmo BU volta o estado exatamente ao anterior, inclusive o checksum (testes `motor/tests/retirada.rs` com BUs reais das fixtures, e o teste unitário do agregador). O chamador precisa reler o BU antigo (posição no `.uvpk`) e informar a anulação com que ele foi aplicado.

---

## ADR-020 — Site simulado local (`uv-simulador`) para provar o serviço

**Status:** aceita (etapa 3, `0.2.2`).

**Contexto.** Os critérios da etapa 3 exigem o serviço da noite rodando do início ao fim, com falhas, várias vezes. Os BUs de 2024 e de 2026 já estão nos pacotes locais e **não podem ser baixados de novo** do TSE; rodar contra o site real também não testaria nada (fora de eleição não há BUs novos).

**Decisão.** Um binário separado, `uv-simulador` (crate `uv-aquisicao`, `hyper`; nunca no motor), serve em `127.0.0.1` **exatamente os caminhos do site real** (`cs.json` reconstruído no instante simulado com `ETag`/304, `aux.json` no formato real, `-bu.dat` com os bytes do `.uvpk`, `-u.json` oficiais depois do último BU), com relógio acelerado e cabeçalho `Date` simulado. O horário de cada seção vem das métricas da coleta (`disponivel_cs`; quando ela não é da noite — `cs.json` de 2024 republicado em 29/09/2026 — `recebido_tse`). Falhas por parâmetro: rajadas de 429, latência e 5xx com semente fixa, reenvio idêntico, anulação depois de publicada, queda do servidor. Cada requisição vai para o `simulador.log`. O `uv-servico` aponta para ele com `--base http://127.0.0.1:<porta>` e roda **o mesmo código da noite**. A aceleração só é aceita com servidor local, e isso está no código (`coletor::limites`, `Rede::para`): contra qualquer outro host, teto de 100 req/s e pausa ≥ 10 min depois de um 429, qualquer que seja a configuração.

**Alternativas.** (a) *Replay direto no motor* (`uv-coletor replay`): não exercita rede, `cs.json`, `aux.json`, retomada nem 429. (b) *Mock dentro do serviço*: um caminho de código que não roda na noite.

**Consequências.** O ensaio da noite fica reproduzível e barato (2º turno de 2024 em ~23 min a 10×, testes `--ignored` em ~1 min por caso a 120×). O que o simulador não reproduz: o atraso do CDN entre a origem e a borda, o formato de um `st` que nunca vimos, e a carga do site real.

---

## ADR-021 — Distribuição massiva via *Static-File Edge Broadcast* (sem push central)

**Status:** aceita (planejada para v2.0.0; ver [ESCALABILIDADE_SEGURA.md](ESCALABILIDADE_SEGURA.md)).

**Contexto.** Atingir centenas de milhares de cidadãos simultâneos na noite da eleição via conexões abertas permanentes (WebSockets ou SSE) a partir de um servidor central derruba o processo por esgotamento de descritores de arquivo (C10K/C100K) e saturação de banda de saída (*egress*).

**Decisão.** Inverter o modelo de distribuição: eliminar o *push* ativo do servidor dedicado. O totalizador consolida o resultado em memória e grava periodicamente (a cada 2–5 s) pequenos arquivos estáticos JSON (ex.: `br-presidente.json` ~15 KB) em storage conectado a uma CDN global com cache na borda (Cloudflare Pages/R2, GitHub Pages). Os navegadores (SPA) consomem via HTTP GET (*short-polling*) com cabeçalhos `ETag` / `If-None-Match`.

**Alternativas.** (a) *Cluster de servidores de WebSocket com pub/sub*: complexidade de infraestrutura, alto custo e ponto de estrangulamento. (b) *Cada cliente baixar direto do TSE*: bloqueio por CORS pelo navegador e esgotamento do teto de requisições da Akamai/TSE.

**Consequências.** O servidor do projeto conversa com exatamente 1 cliente (a API da CDN). Custo de tráfego de distribuição reduzido a praticamente R$ 0,00, com capacidade de absorver milhões de acessos simultâneos na borda.

**Revista pela ADR-030** (`2.0.0-alpha.8`, 15/10/2026): sem storage na borda — cada nó é a origem dos próprios arquivos e a CDN só faz cache na frente, com vários nós como origem. O *pull* com `ETag` continua como aqui.

---

## ADR-022 — Particionamento dinâmico por *Rendezvous Hashing* sem coordenação central

**Status:** aceita (planejada para v2.0.0; ver [ESCALABILIDADE_SEGURA.md](ESCALABILIDADE_SEGURA.md)).

**Contexto.** Uma divisão estática por regiões geográficas gera desequilíbrio severo de carga: o Sudeste concentra 43% do eleitorado no início da noite, enquanto o Norte (fusos horários posteriores e transporte fluvial de urnas) transmite mais tarde. Além disso, se um nó voluntário falhar, suas seções ficariam descobertas.

**Decisão.** Usar **Rendezvous Hashing (Highest Random Weight Hashing)** sobre a chave única da seção (`municipio`, `zona`, `secao`) e a lista de nós ativos. A presença de nós é anunciada via arquivos de *heartbeat* leves atualizados a cada 10 s na CDN pública. Cada nó calcula localmente e de forma determinística quais seções deve coletar. Nós inativos (> 30 s sem heartbeat) têm suas seções automaticamente reatribuídas aos nós sobreviventes pelo segundo maior peso do hash.

**Alternativas.** (a) *Particionamento geográfico estático*: gargalo inicial no Sudeste e ociosidade no Norte. (b) *Módulo simples (`secao % N`)*: a entrada ou saída de 1 nó reatribui quase 100% de todas as seções, invalidando caches. (c) *Fila centralizada com RabbitMQ/Kafka*: reintroduz um ponto único de falha e dependência de infraestrutura cara.

**Consequências.** A carga de coleta aos servidores do TSE é multiplicada de forma segura (ex.: 3 nós a 100 req/s atingem 300 req/s combinados), com balanceamento perfeito ao longo de toda a apuração e sem necessidade de protocolo de consenso complexo entre os nós.

**Complementada pela ADR-028** (`2.0.0-alpha.3`): cada nó é de um órgão diferente e o limite de 100 req/s é contado por IP de saída; por isso o HRW é **ponderado** pela capacidade de cada nó (nós que dividem um IP pesam menos), com hash inteiro e determinístico, e um nó em pausa por 429 sai da partição.

---

## ADR-023 — Zero-Trust e validação por quórum criptográfico no cliente (Ed25519)

**Status:** aceita (planejada para v2.0.0; ver [ESCALABILIDADE_SEGURA.md](ESCALABILIDADE_SEGURA.md)).

**Contexto.** Ao permitir que múltiplos nós voluntários coletem e consolidem dados de forma independente, surge o risco de nós maliciosos ou comprometidos injetarem números falsos na apuração.

**Decisão.** Adotar arquitetura *Zero-Trust*:
1. Os nós coletores não geram a verdade; apenas transportam BUs cujo conteúdo é assinado digitalmente pelo hardware das urnas do TSE (ECDSA / ICP-Brasil).
2. Cada nó sentinela autorizado possui um par de chaves **Ed25519** com chave pública embutida na SPA (*whitelist*).
3. A cada publicação de resultado, o nó assina o digest SHA-256 do arquivo consolidado e anexa a lista de hashes dos BUs que o compõem.
4. Os nós praticam amostragem cruzada (*spot-checking*) aleatória de 1% das seções dos parceiros.
5. A aplicação cliente (SPA) valida o quórum de concordância ($M$ de $N$ operadores confiáveis) antes de considerar a apuração convergida.

**Alternativas.** (a) *Confiança cega no IP do servidor*: vulnerável a invasões e adulterações. (b) *Blockchain ou consenso bizantino estilo PBFT*: excessivamente pesado e lento para atualizações em tempo real a cada poucos segundos.

**Consequências.** Imunidade matemática a falsificações: nenhum nó consegue adulterar o resultado sem ser exposto imediatamente por divergência com as assinaturas oficiais das urnas e rejeitado pelo quórum da SPA.

---

## ADR-024 — WebAssembly (Wasm) para auditoria e consenso no navegador

**Status:** aceita (planejada para v2.0.0; ver [ESCALABILIDADE_SEGURA.md](ESCALABILIDADE_SEGURA.md)).

**Contexto.** Permitir que o cidadão valide localmente os cálculos eleitorais, assinaturas ASN.1 e quórum matemático sem depender da palavra do servidor, garantindo máxima transparência cívica.

**Decisão.** Compilar a crate `urna-velox` para WebAssembly (`wasm32-unknown-unknown`) com bindings leves para a SPA. O motor roda em um Web Worker desacoplado da thread de renderização da UI, mantendo o mesmo vetor denso de contadores atômicos (~36 MB de RAM linear) e totalizando deltas em milissegundos.

**Alternativas.** (a) *Reescrever o motor em JavaScript*: de 3 a 10 vezes mais lento e mais propenso a desvios sutis de implementação. (b) *Cálculo apenas no servidor*: impede a conferência independente no dispositivo do usuário.

**Consequências.** Unificação da base de código: o mesmo código Rust roda no servidor de coleta, na CLI do auditor e dentro do navegador de qualquer smartphone ou computador pessoal.

---

## ADR-025 — SPA / PWA em vez de aplicativos nativos mobile (Android/iOS)

**Status:** aceita (planejada para v2.0.0; ver [ESCALABILIDADE_SEGURA.md](ESCALABILIDADE_SEGURA.md)).

**Contexto.** Acompanhar a apuração em celulares é o principal caso de uso para a ampla maioria dos cidadãos. Surge a dúvida se é necessário construir, empacotar e publicar aplicativos mobile nativos (Kotlin/Swift ou Flutter) com bindings para o motor em Rust nas lojas Google Play e Apple App Store.

**Decisão.** **Não desenvolver aplicativos nativos para lojas de aplicativo.** A interface do usuário para smartphones e desktops será uma **Single Page Application (SPA) progressiva (PWA)**, servida via web standard:
1. **Zero atrito de instalação:** Um evento de apuração eleitoral dura poucas horas. Links compartilhados via redes sociais/mensageiros abrem instantaneamente no navegador em < 500 ms sem exigir download de 50 MB na loja.
2. **Imunidade à burocracia de moderação das lojas:** Evita revisões lentas (2 a 7 dias) da Apple e do Google, regras de conteúdo político/eleitoral e permite correções críticas (hotfixes) publicadas em segundos.
3. **Instalação opcional como PWA:** Suporte a tela cheia e ícone na tela inicial sem passar pela App Store / Play Store.
4. **Capacidades modernas suficientes:** O navegador mobile moderno já oferece WebAssembly (Wasm) para execução de alta performance, Web Workers para threads em segundo plano e Web Crypto API para validação criptográfica por hardware.

**Alternativas.** (a) *Apps nativos nas lojas*: alto custo de publicação/manutenção, atrasos de aprovação e atrito severo de adoção pelo público. (b) *APK para download avulso no Android*: barreiras de segurança do sistema operacional e exclusão total de usuários de iPhone/iOS.

**Consequências.** Redução drástica do custo e do tempo de desenvolvimento. Uma única base de código web atende computadores, tablets e smartphones (Android e iOS) de forma universal e instantânea.



---

## ADR-026 — Difusão em tempo real no `uv-servico`: WebSocket com último valor por tópico

**Status:** aceita (etapa 4, `0.3.2`; contrato em [PROTOCOLO_WS.md](PROTOCOLO_WS.md)).

**Contexto.** Até a etapa 3 o resultado só existia como arquivos em `<saida>/publicacao/`. O site de exemplo (etapa 5),
integradores e jornais precisam recebê-lo no instante em que muda, muitos ao mesmo tempo, **sem que nenhum cliente
atrase a totalização** (meta do ROADMAP: BU → cliente < 50 ms no p99, noite de 2026 a 60×, 1.000 clientes). O motor
não faz rede e não usa `tokio` (ADR-012/014).

**Decisão.**
1. **A camada fica no `uv-servico`** (crate `uv-aquisicao`, módulo `aquisicao/src/difusao.rs`), ligada por
   `--http <end:porta>`; sem o parâmetro, nada muda. O motor continua sem rede e sem `tokio`.
2. **Mesmos bytes do disco, montados uma vez.** A thread `uv-gravacao` converte o JSON de cada publicação em `Bytes`
   (contagem de referência, sem cópia) e o entrega a uma thread nova, `uv-difusao`, **antes** de gravar o arquivo, por
   canal sem limite (nunca espera). A `uv-difusao` monta a mensagem do tópico uma vez (envelope + bytes do arquivo) e a
   compartilha com todas as conexões; nada é serializado por cliente. Por isso `t_envio` é o instante da montagem.
3. **Último valor por tópico** (`tokio::sync::watch`): cada conexão observa os `watch` dos seus tópicos (`StreamMap`);
   enquanto ela não consegue escrever, as versões se juntam e ela recebe a mais recente com um aviso `seq_pulado`.
   Nenhuma fila cresce por cliente; 30 s sem conseguir escrever ou 4 MB de buffer derrubam a conexão
   (`cliente_lento`). `broadcast` foi descartado: com receptor lento ele perde mensagens (`Lagged`) e guarda uma fila
   por canal.
4. **`axum` 0.8** (WebSocket sobre o `hyper 1`/`tokio` que o crate já usa), num **runtime próprio** com
   `--http-threads` threads (padrão 4), separado do runtime da coleta e da thread do motor; aceitação de conexões à
   mão para ter `TCP_NODELAY`, tempo máximo de *handshake* e limite de conexões TCP.
5. **REST como arquivo estático**: `ETag` = `seq`, `If-None-Match` → 304, `Cache-Control` curto, CORS `*`; só leitura e
   só pela tabela de tópicos (nenhum caminho vindo do cliente chega ao disco). Um proxy/CDN na frente funciona sem
   mudança — é o caminho para a distribuição massiva da 2.0 (ADR-021): WebSocket aqui atende o site de exemplo e
   integradores (milhares de conexões numa máquina), não centenas de milhares de navegadores.
6. **Sem TLS nem autenticação no binário**: escuta em `127.0.0.1` nos exemplos; expor é decisão de quem opera, com
   proxy reverso (exemplo em [OPERACAO.md](OPERACAO.md) §7).

**Alternativas.** (a) *Servidor separado lendo a pasta `publicacao/`* (inotify): mais um processo e um atraso de
sistema de arquivos, e sem como medir BU → cliente de ponta a ponta. (b) *`hyper` + `tokio-tungstenite` direto*: mais
código para as mesmas garantias; o `axum` já traz os limites de tamanho e de buffer do WebSocket. (c) *Fila por
cliente*: memória sem limite com clientes lentos. (d) *Serializar por cliente com o `t_envio` exato*: custo
proporcional ao número de clientes dentro do caminho crítico.

**Consequências.** Um lote publicado custa à totalização só a entrega de `Arc`s a outra thread; com 0 clientes, a
difusão não muda BU → publicação (medido no relatório da etapa 4). O conteúdo de toda mensagem de resultado é conferível
byte a byte com o arquivo publicado (o `uv-carga` faz isso). Clientes que reconectam recebem o estado atual no
`instantaneo` (`seq` ≥ o que tinham, inclusive depois de `kill -9` e retomada).

---

## ADR-027 — Página de resultados embutida no binário e marcos da noite calculados no servidor

**Status:** aceita (etapa 5, `0.4.1`; contrato em [PROTOCOLO_WS.md](PROTOCOLO_WS.md) §4.5, §6 e §10; plano em
`/mnt/d/git/lukeboh/urna-velox/docs/HANDOFF_0.5.0.md`).

**Contexto.** A etapa 5 pede um site de exemplo elegante, que acompanhe a apuração só pelo protocolo público, sem nada
externo em tempo de execução e sem nenhum pulo na tela, com marcos e curiosidades (primeira seção totalizada, seção que
decidiu o "matematicamente", eleitores que faltam…). Parte dessas curiosidades depende de saber **qual seção** entrou
em cada lote ou da conta do "matematicamente", que só a thread do motor tem.

**Decisão.**
1. **A página é estática e vai dentro do `uv-servico`.** Fontes em `site/src/` (JavaScript sem framework, CSS, fonte
   Archivo variável embutida), juntadas por `site/build.mjs` (esbuild) em `site/dist/`, que **vai para o repositório** e
   entra no binário com `include_bytes!` (`aquisicao/src/pagina.rs`). Servida pela mesma camada HTTP (`GET /`,
   `GET /static/<arquivo>`) a partir de uma tabela fixa (nenhum caminho do cliente chega ao disco), com CSP
   `default-src 'self'; connect-src 'self'`. Quem opera não precisa de Node nem de servidor web; o binário da noite é o
   site.
2. **Só o protocolo público.** O que faltava entrou por acréscimo à versão 1: `GET /api/v1/nomes` (nomes de UFs,
   municípios, partidos e vices, montados do `candidatos.json`, que passou a guardá-los — `uv-coletor preparar` os lê dos
   `cm.json` e `-u.json` que já baixava, sem requisição nova), a mensagem `marco` (`op: marcos`) e `GET /api/v1/marcos`.
3. **Marcos que exigem a seção ou a conta do "matematicamente" ficam no servidor** (`aquisicao/src/marcos.rs`, na
   thread do motor, baratos: p50 0,002 ms por lote), com histórico em `<saida>/marcos.csv` (relido na retomada):
   primeira seção, seção decisiva, `faltam_para_mat` (função pura nova no motor, `matematica::faltam_para_mat`, busca
   binária sobre `avaliar`), viradas de liderança, % das seções e contagem de BUs. **O que sai dos resultados e eventos
   a página calcula** (primeiro eleito por cargo, disputa mais apertada/folgada, comparecimento por UF, ritmo).
4. **Desenho por quadro, não por mensagem**: a página guarda o último valor de cada tópico e redesenha no máximo uma vez
   por `requestAnimationFrame`; animações só por `transform`/`opacity` em caixas de tamanho reservado (CLS = 0 medido).

**Alternativas.** (a) *Site separado (outro processo ou CDN)*: mais uma peça para operar na noite; fica para a 2.0
(ADR-021), e esta página funciona igual atrás de um proxy. (b) *Framework (React, Svelte…) e biblioteca de gráficos*:
redesenhos que "piscam" e mais peso; o pedido é transição contínua e redirecionável, mais fácil à mão. (c) *Calcular a
seção decisiva no cliente*: o cliente não sabe quais seções entraram em cada lote (o resultado é agregado). (d) *Fonte do
sistema*: a troca de métricas entre sistemas mudaria larguras reservadas; a fonte embutida garante os mesmos números
tabulares em toda tela.

**Consequências.** O binário cresce ~170 KB. Mudar a página exige `npm run build` em `site/` e recompilar. A seção
decisiva é a última do lote dentro da circunscrição: a 60× (lotes grandes) é uma aproximação documentada no protocolo.

---

## ADR-028 — Federação de órgãos: um nó por órgão, limite por IP de saída, HRW ponderado e troca de BUs pelo storage

**Status:** aceita (v2.0, `2.0.0-alpha.3`; plano em [HANDOFF_0.6.0.md](HANDOFF_0.6.0.md); código em
`aquisicao/src/federacao/`). Complementa a ADR-022 e o REQ-ESC-08 da [ESCALABILIDADE_SEGURA.md](ESCALABILIDADE_SEGURA.md).

**Contexto.** A ADR-022 dividia as seções igualmente entre os nós vivos e supunha 100 req/s por nó. Duas coisas
mudam isso. (1) **A regra de rede** (14/10; `CLAUDE.md`): cada órgão acessa pelo
seu IP e não passa de 100 req/s (acima disso, HTTP 429); órgãos diferentes podem colaborar; a CDN conta pelo **IP de saída**, então dois nós
atrás do mesmo IP são um só para ela. (2) **Um nó só coleta parte das seções**, mas cada um precisa do país inteiro
para publicar totais que o quórum possa comparar (ADR-023).

**Decisão.**
1. **Um nó por órgão**, declarado no `federacao.json` (id, órgão, IP de saída declarado, teto). A configuração é
   recusada se dois nós forem do mesmo órgão ou se o teto de um nó passar de 100 req/s sem autorização registrada (campo `acordo`).
2. **IP de saída medido**, não presumido: cada nó pergunta seu IP a um eco da federação e o publica no *heartbeat*.
   Medido ≠ declarado → o nó não coleta (só consome o storage). **Nós com o mesmo IP medido formam um grupo e dividem
   o teto de coleta do IP** (95 req/s nas coletas longas).
3. **HRW ponderado, só com inteiros.** O peso de um nó é a sua vazão dividida por uma unidade (padrão 5 req/s, peso
   mínimo 1); um nó de peso *w* concorre com *w* identidades virtuais e fica com a seção se uma delas tiver a maior
   nota. Hash: FNV-1a 64 dos bytes + finalização *splitmix64*, especificados no código e fixados por testes com
   valores conhecidos; empate decidido pelo id. Nada de ponto flutuante (o `ln` da fórmula logarítmica pode variar no
   último bit entre plataformas, e dois nós discordando da partição significam seção coletada duas vezes ou nenhuma).
   Um nó que entra só toma seções para si; um nó que sai só libera as suas (REQ-ESC-07).
4. **Quem participa da partição:** nós com *heartbeat* há ≤ 30 s pelo relógio do storage, não `em_pausa` e com IP
   medido = declarado. Um **HTTP 429 pausa só o IP**: os nós dele marcam `em_pausa` por ≥ 10 min e saem da partição;
   as seções deles passam aos outros IPs e voltam depois.
5. **Troca de BUs direto entre os nós, sem storage compartilhado** (revisão de 14/10, decisão do Luciano: um storage
   comum é mais uma peça que os órgãos teriam de montar e combinar). Quem coleta uma seção grava os bytes do BU e o
   `<hash>` do diretório (SHA-256 do `.vsc`) em lotes só de acréscimo **no seu próprio diretório**; cada nó serve o seu
   diretório por HTTP só de leitura (`GET /fed/v1/nos/<nó>.json`, `/fed/v1/bus/<nó>/ultimo`,
   `/fed/v1/bus/<nó>/<seq>.uvtr`, com `ETag`) e **puxa** dos pares o que falta — primeiro da própria origem; se ela não
   responde, de qualquer outro par que tenha a cópia (**repasse**, no estilo P2P). Assim cada nó termina com todos os
   BUs e totaliza o país inteiro com o mesmo motor, e o mesmo `uv-servico` que entra na federação já sincroniza. A
   vida de um nó é medida por cada um, localmente: o *heartbeat* muda a cada batida (contador) e a cópia local só é
   regravada quando muda. **Antes de aceitar BU de outro nó em produção, verificar a assinatura da urna** (ADR-011
   vira pré-requisito da etapa C); até lá, a troca só existe entre nós de teste e confere o SHA-256 declarado.
6. **Diretório local como abstração** (`Armazem`: gravar, ler com `ETag`, listar por prefixo), com os dados do
   próprio nó e as cópias dos pares. O motor continua sem rede. (A distribuição aos cidadãos pela borda de uma CDN,
   ADR-021, é outra coisa e continua na etapa B.)

**Alternativas.** (a) *Teto global dividido por K* (versão `alpha.1` do handoff): mais restritivo que a regra real e
sem ganho de coleta. (b) *HRW sem peso*: um nó que divide IP receberia a mesma fatia de um nó sozinho e ficaria para
trás. (c) *HRW ponderado logarítmico (`-w/ln u`)*: menos hashes, mas em ponto flutuante. (d) *Cada nó só publicar o
seu pedaço e o cliente somar*: o quórum deixaria de comparar resultados independentes. (e) *Storage compartilhado
(diretório comum ou R2)* entre os nós (versões `alpha.3`/`alpha.4`): exige uma infraestrutura que os órgãos teriam de
combinar e é um ponto único de falha. (f) *Empurrar (push) cada BU para os pares*: quem recebe não controla o ritmo e
quem esteve fora perde o que foi empurrado; puxar com `ETag` resolve os dois, e a retomada é só continuar puxando.

**Consequências.** A vazão da federação soma os IPs de saída distintos; com 2 órgãos o atraso cai abaixo de 1 min no
p90. O custo da partição ponderada é ~Σpesos hashes por seção (3 nós de peso 19: 57 por seção, ~29
milhões para o Brasil, recalculados só quando a lista de nós muda). Um erro de configuração de rede (NAT comum não
declarado) é detectado pela medida do IP e reduz a vazão em vez de violar o limite.

---

## ADR-029 — Vários órgãos na mesma infraestrutura: IP de saída dedicado por órgão

**Status:** aceita (v2.0, `2.0.0-alpha.8`; decisão do Luciano em 15/10/2026). Complementa a ADR-028 e o REQ-ESC-08.

**Contexto.** Vários órgãos podem rodar os seus nós na mesma infraestrutura de hospedagem. A regra do projeto é de
100 req/s por órgão, contados pelo IP de saída (acima disso, HTTP 429). Se dois órgãos saíssem pelo mesmo IP (uma NAT
comum, um gateway compartilhado), o código os juntaria num grupo e **dividiria** o teto (ADR-028, item 2): correto
para o TSE, mas cada órgão perderia metade da vazão por uma falha de quem opera. **Órgãos não podem se prejudicar por
rodarem na mesma infraestrutura.** A divisão no código protege o TSE; não protege o órgão.

**Decisão.**
1. **A garantia é operacional, não só de código.** Cada órgão tem **um nó numa máquina própria** (VM, não contêiner
   na mesma máquina), com **IP público reservado exclusivo** e saída direta por ele (sem NAT ou gateway compartilhado).
   CPU, memória e disco também ficam isolados: a carga de um órgão não atrasa outro.
2. **Inventário do operador** (órgão, nó, chave, IP reservado): a implantação recusa um IP já atribuído a
   outro órgão e um segundo nó **que coleta** do mesmo órgão (réplicas do mesmo órgão só com `"coleta": false`,
   `alpha.7`).
3. **Os órgãos formam uma federação** e colaboram na coleta: cada um pede ao TSE só a sua fatia (HRW ponderado), no
   seu IP, e recebe dos outros o resto. Quanto mais a federação colabora, **menor a pressão na CDN do tribunal** (cada
   seção é pedida uma vez pela federação, e não uma vez por órgão), além de todos ficarem mais rápidos (com 2 órgãos,
   p90 abaixo de 1 min).
4. **O código continua como rede de segurança:** nós de órgãos diferentes com o mesmo IP medido continuam dividindo o
   teto (o TSE nunca vê mais de 100 req/s num IP), e isso passa a gerar um **alerta grave de operação** ("IP de saída
   compartilhado entre órgãos"), no *heartbeat* e na `saude.json` — a implementar.

**Alternativas.** (a) *Contêineres de vários órgãos numa máquina, saída por NAT*: barato, mas é exatamente o caso
que divide o teto e mistura a carga. (b) *Cada órgão isolado, coletando tudo sozinho*: cada um fica dezenas de minutos
atrás no pico (100 req/s) e o TSE recebe K vezes as mesmas requisições. (c) *Confiar só na divisão do código*: protege o TSE,
mas degrada o serviço de todos sem ninguém perceber.

**Consequências.** Uma VM e um IP reservado por órgão (custo por órgão); a vazão da
federação cresce com cada órgão; a operação precisa do inventário e de conferir, na implantação, o IP medido de cada
nó (`10_implantar.sh` já mede pelo eco).

---

## ADR-030 — Entrega aos leitores: cada nó é origem dos próprios arquivos e a CDN só faz cache

**Status:** aceita (v2.0, `2.0.0-alpha.8`; decisão do Luciano em 15/10/2026, opção A). **Revê a ADR-021** na parte do
storage; o *pull* por arquivos estáticos continua.

**Contexto.** A ADR-021 previa que cada nó enviasse os arquivos de resultado a um storage na borda (Cloudflare R2/KV)
e a CDN servisse de lá. Na etapa A a federação ficou **sem storage** (ADR-028): cada nó serve o seu diretório. O
`uv-servico` já serve `GET /api/v1/...` com `ETag`/304 (ADR-026), e todo nó tem o resultado completo.

**Decisão.**
1. **Cada nó serve os próprios arquivos de resultado** (os bytes do [FORMATO_RESULTADO.md](FORMATO_RESULTADO.md)) e um
   `indice.json` por ciclo (seq, seções apuradas, hash de cada arquivo), com
   `Cache-Control: public, max-age=2, stale-while-revalidate=1`, `ETag` e compressão.
2. **Uma CDN em modo proxy fica na frente e só faz cache** (TTL de ~2 s): busca no nó só quando a cópia expira, então o
   nó atende poucas requisições por arquivo mesmo com milhões de leitores. **Vários nós como origem**: se um cair, a
   CDN busca em outro (todos têm o mesmo resultado).
3. **O nó só fica exposto pela CDN e só com os caminhos públicos de resultado.** A sincronização entre nós
   (`/fed/v1/...`) continua só na rede entre os pares; a origem aceita apenas a CDN (lista de IPs dela ou túnel).
4. **Domínio:** um por órgão ou um só do operador; o desenho serve aos dois.
5. O WebSocket da ADR-026 continua para poucos consumidores de baixa latência (painéis, integrações); a borda é
   para o público.

**Alternativas.** (a) *Storage na borda com upload* (ADR-021 original): mais uma conta, credenciais de escrita por nó e
uma peça a mais para cada órgão montar; o resultado sobreviveria à queda de todos os nós, mas isso também se resolve
com `stale-if-error` no cache. (b) *GitHub Pages para os dados*: já descartado (build lento, limites de push, cache de
10 min).

**Consequências.** Nada para montar além de apontar um domínio para os nós e configurar o cache. REQ-ESC-02 passa a
ser medido como "lote totalizado → arquivo novo visível através do cache < 200 ms + TTL (p99)"; REQ-ESC-04 (custo zero)
depende de a CDN escolhida cachear JSON no plano gratuito por regra de cache (conferir na etapa B). A origem precisa de
proteção (só caminhos públicos, só a CDN), que se soma à autenticação entre nós da etapa C.

## ADR-031 — Licença: Source-Available / PolyForm Noncommercial, com dupla licença

**Status:** aceita (`0.6.15`, 17/10/2026; decisão do Luciano). **Substitui** a licença própria "de Uso Restrito,
Cívico e Não Comercial (Source-Available)" adotada antes do 2º turno de 2026.

**Contexto.** O código precisa ser público: a razão de ser do Urna Velox é a conferência independente e reproduzível
dos resultados (auditoria cidadã, pesquisa, imprensa que confere o TSE). Ao mesmo tempo, o uso comercial deve depender de
licença do autor, o que uma licença permissiva inviabilizaria. A licença própria anterior cumpria esse papel, mas era um texto artesanal,
sem versão padronizada nem identificador SPDX, mais difícil de ser entendido por quem audita e por ferramentas.

**Decisão.**
1. **Classificação do projeto: Source-Available / PolyForm Noncommercial.** O [`LICENSE`](../LICENSE) é o texto
   oficial e integral da [PolyForm Noncommercial License 1.0.0](https://polyformproject.org/licenses/noncommercial/1.0.0/)
   (SPDX `PolyForm-Noncommercial-1.0.0`), sem alterações, com tradução de conteúdo idêntico em
   [`LICENCA.pt-BR.md`](../LICENCA.pt-BR.md) (prevalece o texto em inglês), precedido das linhas `Required Notice:` com o copyright
   (2026, Luciano Soares Bohnert) e o aviso de licença comercial. Isenção de garantia (*as is*) e limitação de
   responsabilidade são as da própria licença.
2. **Dupla licença.** Uso não comercial (pesquisa, auditoria cidadã, educação, uso pessoal; instituições de ensino,
   pesquisa pública, beneficentes e governamentais) é gratuito pela PolyForm. Uso com finalidade econômica, direta ou
   indireta, exige contrato de licenciamento comercial com royalties, firmado com o autor
   ([LICENCIAMENTO.md](LICENCIAMENTO.md)).
3. **Manifestos** declaram `license = "PolyForm-Noncommercial-1.0.0"` (crates `urna-velox`, `uv-aquisicao`,
   `urnabench`; `bench/js/package.json`).
4. **Contribuições** são aceitas com a condição de poderem ser distribuídas pelo autor sob as duas licenças (sem isso
   a dupla licença deixaria de valer para o código de terceiros).

**Alternativas.** (a) *Manter a licença própria*: descartada pelos motivos do contexto. (b) *Licença com
conversão futura para open source após um prazo*: deixaria de exigir licença para uso comercial com o tempo. (c) *Licença
open source com cláusula de reciprocidade*: permitiria o uso comercial gratuito desde que o código fosse aberto, o que
não exige licença para uso comercial. (d) *Licença que proíba modificações e redistribuição*: atrapalharia a auditoria e a
pesquisa.

**Consequências.** O projeto **não é *open source*** segundo a OSI (é *source-available*); a transparência cívica fica
preservada porque qualquer pessoa pode ler, compilar, modificar e rodar o código para fins não comerciais. A PolyForm
é mais branda que a licença anterior em um ponto: órgãos públicos e entidades sem fins lucrativos usam de graça
qualquer que seja o financiamento, o que combina com a federação de órgãos (ADR-028). O que conta como "comercial"
segue o texto da licença; o README e o LICENCIAMENTO só o explicam, e a dúvida se resolve consultando o autor.
