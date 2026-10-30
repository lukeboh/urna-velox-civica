# Glossário

Termos técnicos usados na documentação e no código do Urna Velox, explicados em linguagem direta. Cada termo diz **o que é** e, quando ajuda, **por que importa neste projeto**. Os termos estão agrupados por assunto e em ordem alfabética dentro de cada grupo.

**Grupos:** [1. Documentação e processo](#1-documentação-e-processo) · [2. Domínio eleitoral](#2-domínio-eleitoral) · [3. Formato dos dados (ASN.1/BER)](#3-formato-dos-dados-asn1ber) · [4. Memória](#4-memória) · [5. Processador e concorrência](#5-processador-e-concorrência) · [6. Estruturas de dados](#6-estruturas-de-dados) · [7. Linguagens, runtimes e bibliotecas](#7-linguagens-runtimes-e-bibliotecas) · [8. Ambiente, arquivos e distribuição](#8-ambiente-arquivos-e-distribuição) · [9. Medição e estatística](#9-medição-e-estatística) · [10. Unidades](#10-unidades) · [11. Rede e aquisição](#11-rede-e-aquisição)

---

## 1. Documentação e processo

| Termo | Significado |
|---|---|
| **ADR** (*Architecture Decision Record*, "registro de decisão de arquitetura") | Um texto curto que registra **uma** decisão técnica importante: o contexto, o que foi decidido, as alternativas descartadas e as consequências. O arquivo [ARCHITECTURE_DECISIONS.md](ARCHITECTURE_DECISIONS.md) reúne os ADRs do projeto, numerados (ADR-000, ADR-001...). Serve para que, meses depois, alguém entenda *por que* o sistema é como é, e não só *como* é. |
| **Status de um ADR** | *Proposta*: ainda em discussão ou a validar. *Aceita*: em vigor. *Substituída por ADR-NNN*: revogada por outra decisão; o texto antigo é mantido para histórico. |
| **Alternativas consideradas** | Seção do ADR com as opções que foram avaliadas e não escolhidas, e o motivo. |
| **Critério de pivot** | Condição objetiva que, se acontecer, justifica trocar uma decisão (por exemplo, trocar de linguagem). |
| **Fase / marco** | Etapa do [ROADMAP.md](../../ROADMAP.md) com entregas e um critério de sucesso mensurável. |
| **Plano A / B / C** | As três opções de linguagem comparadas: A = Rust (escolhida), B = JavaScript/TypeScript (Node.js ou Bun), C = Java. |
| **Requisito não funcional** | Exigência sobre *como* o sistema funciona (velocidade, memória, portabilidade), e não sobre *o que* ele faz. |
| **Roadmap** | Plano de desenvolvimento em fases. |
| **Replay** | Uma eleição que já aconteceu, apurada de novo no site com os dados reais (os BUs publicados pelo TSE), como a reprise de um jogo: a totalização recomeça do zero, na ordem de chegada dos boletins, e no fim é conferida com o oficial. Até a 0.8.37 o site dizia "simulado"/"simulação"; nos nomes internos (`uv-simulador`, `--simulacao`, campo `simulacao` do protocolo) o termo antigo continua. A **Playlist de Replays** (`/playlist`) mostra a agenda e deixa o público votar no próximo. Ver [CICLO_DIARIO.md](../operacao/CICLO_DIARIO.md). |

## 2. Domínio eleitoral

| Termo | Significado |
|---|---|
| **Aptos** | Eleitores habilitados a votar numa seção. |
| **BU (Boletim de Urna)** | Arquivo emitido por cada urna ao fim da votação com o total de votos da seção, por cargo e por candidato. É a entrada do Urna Velox. O TSE publica os BUs em formato binário ASN.1/BER (extensão `.bu`). |
| **Cargo** | Posição em disputa. Códigos do TSE: 1 Presidente, 3 Governador, 5 Senador, 6 Deputado Federal, 7 Deputado Estadual, 8 Deputado Distrital, 11 Prefeito, 13 Vereador. |
| **Comparecimento** | Eleitores que efetivamente votaram na seção. |
| **Consolidação** | Somar os totais municipais para obter os totais de cada UF e do Brasil. |
| **Eleição geral / municipal** | Geral (anos pares, como 2026): Presidente, Governador, Senador, Deputados. Municipal (como 2024): Prefeito e Vereador. Um BU de eleição geral contém **duas eleições** (federal e estadual). |
| **Pleito / turno** | Pleito é a eleição de um ano; turno é o 1º ou o 2º turno dela. Os dois entram na chave que identifica um BU. |
| **Seção eleitoral** | Menor unidade de votação: um conjunto de eleitores que vota numa mesma urna. Uma eleição geral tem ~500 mil seções. |
| **Seções agregadas / urna de contingência / SA** | Casos especiais: uma urna atendendo mais de uma seção; uma urna substituta após falha; BU gerado pelo Sistema de Apuração (SA) quando a urna não pôde emiti-lo. Exigem tratamento explícito na deduplicação. |
| **Totalização** | Somar os votos de todos os BUs para chegar ao resultado por município, UF e Brasil. |
| **TSE** | Tribunal Superior Eleitoral, que organiza as eleições e publica os dados abertos, inclusive os BUs. |
| **UF** | Unidade da Federação (26 estados + DF). O voto no exterior aparece como a "UF" `ZZ`. |
| **Vagas** | Quantas cadeiras de um cargo estão em disputa. Em 2026 o Senado renova 2/3: cada eleitor vota em **2** senadores, então os votos do cargo somam 2 × o comparecimento (ADR-008). |
| **Votável** | Qualquer opção que pode receber voto num cargo: um candidato (voto nominal), um partido (voto de legenda), branco ou nulo. Um BU de eleição geral tem em média ~340 votáveis. |
| **Voto de legenda** | Voto dado só ao número do partido, nos cargos proporcionais (deputados, vereadores). |
| **Voto nominal / branco / nulo** | Nominal: no número de um candidato. Branco: opção "branco". Nulo: número inexistente digitado. |
| **Zona eleitoral** | Divisão administrativa da Justiça Eleitoral que agrupa seções. |

## 3. Formato dos dados (ASN.1/BER)

| Termo | Significado |
|---|---|
| **ASN.1** (*Abstract Syntax Notation One*) | Linguagem padronizada (norma ITU-T X.680) para descrever a estrutura de dados binários: que campos existem, de que tipo e em que ordem. O TSE descreve o BU num arquivo de esquema ASN.1 (`bu.asn1`). |
| **Assinatura digital** | Código criptográfico que prova que um arquivo foi gerado por quem diz ter gerado e que não foi alterado. Os BUs são assinados pela Justiça Eleitoral. |
| **BER** (*Basic Encoding Rules*) | A regra (norma ITU-T X.690) que diz como transformar dados descritos em ASN.1 em bytes. Os BUs são gravados em BER. |
| **DER / CER** | Variantes mais restritas do BER; o DER garante uma única codificação possível para cada valor. |
| **Decodificador / parser** | Programa que lê os bytes e extrai os campos. O Urna Velox tem um parser **manual** (escrito à mão, rápido) e usa uma **biblioteca** (`rasn`) como referência para conferir o manual. |
| **Esquema** | A descrição formal dos campos do BU, em ASN.1. Muda de uma eleição para outra. |
| **Hash** | Resumo de tamanho fixo calculado a partir de um arquivo; qualquer alteração no arquivo muda o hash. Usado para detectar BUs repetidos com conteúdo diferente. |
| **ICP-Brasil / PKCS#7** | ICP-Brasil é a infraestrutura brasileira de certificados digitais; PKCS#7 é um formato padrão de "envelope" para dados assinados. Aparecem na verificação de assinaturas (Fase 4). |
| **INTEGER / OCTET STRING / SEQUENCE / SEQUENCE OF** | Tipos básicos do ASN.1: número inteiro; sequência de bytes arbitrária; registro com campos em ordem; lista de elementos do mesmo tipo. |
| **SHA-512** | Função de hash criptográfica que produz um resumo de 64 bytes. Medido: calcular o SHA-512 de cada BU custa ~7 × a totalização (ADR-011). |
| **Tag (de contexto, implícita)** | Primeiro byte de cada campo BER; identifica qual campo é. Uma tag de contexto implícita (ex.: `[3]`, byte `0xA3` ou `0x83`) substitui a tag do tipo original pelo número do campo. |
| **TLV** (*Tag–Length–Value*) | A forma como o BER grava cada campo: **tag** (qual campo), **length** (quantos bytes), **value** (o conteúdo, que pode conter outros TLVs). O parser manual é essencialmente um leitor de TLV. |
| **Zero-copy** ("sem cópia") | Técnica em que o programa lê os campos diretamente dos bytes originais, sem copiá-los para novas estruturas na memória. Evita alocações e é o que torna o parser manual rápido. |

## 4. Memória

| Termo | Significado |
|---|---|
| **Alocação / alocador** | Pedir um pedaço de memória ao sistema durante a execução. O alocador é o componente que atende esses pedidos. Alocar muito e com frequência custa tempo. O `membench` usa um **alocador contador**, que soma os bytes pedidos para medir exatamente a memória de cada estrutura. |
| **Boxing ("encaixotamento")** | Em Java (e de forma parecida em JS), guardar um número simples dentro de um objeto (`Integer`, `AtomicLong`). Cada objeto carrega um cabeçalho e um ponteiro, então um contador de 4 bytes pode ocupar 70–80 bytes. É a causa do alto consumo das versões "ingênuas" em Java. |
| **Coletor de lixo (GC, *garbage collector*)** | Parte do runtime de Java e JavaScript que encontra e libera automaticamente a memória que o programa não usa mais. Rust não tem GC: a memória é liberada em pontos definidos na compilação. |
| **G1 / ZGC** | Coletores de lixo da JVM. G1 é o padrão; ZGC tem pausas abaixo de 1 ms. |
| **Heap** | Área de memória onde ficam os dados criados durante a execução. Em Java e JS é gerenciada pelo GC. "Heap vivo" = o que ainda está em uso após uma coleta. |
| **Memória anônima / privada** | Memória do processo que não corresponde a nenhum arquivo em disco. É a que o orçamento do motor considera (ADR-000). |
| **mmap / `memmap2`** | Mapear um arquivo na memória: o programa acessa o arquivo como se fosse um vetor de bytes e o sistema operacional carrega as páginas sob demanda. Essas páginas aparecem no RSS, mas não são memória "consumida" pelo motor. |
| **Pausa de GC** | Momento em que o programa para para o coletor de lixo trabalhar. |
| **Pico de memória** | O maior uso de memória do processo durante a execução. |
| **RSS / VmHWM / maxRSS / Working Set** | Medidas de quanta memória física um processo ocupa. RSS (*Resident Set Size*) é a atual no Linux; VmHWM (*high water mark*) e `maxRSS` são o pico; Working Set é o equivalente no Windows. Todas incluem páginas de arquivos mapeados. |
| **`-Xmx`** | Opção da JVM que define o tamanho máximo do heap Java (ex.: `-Xmx3g` = 3 GB). |

## 5. Processador e concorrência

| Termo | Significado |
|---|---|
| **Aceleração (*speedup*)** | Quantas vezes mais rápido fica com N threads em relação a 1 thread. O ideal seria N ×; na prática fica abaixo. |
| **Banda de memória** | Quantos bytes por segundo a memória RAM consegue entregar ao processador. Com muitas threads, vira o limite (ESTIMATIVAS §3.2). |
| **Cache (L1, L2, L3)** | Memórias pequenas e muito rápidas dentro do processador que guardam dados usados recentemente. L3 é a maior e mais lenta das três (33 MB no i7-14700). Dados que não cabem no cache precisam vir da RAM, bem mais devagar. |
| **Condição de corrida (*data race*)** | Erro em que duas threads acessam o mesmo dado ao mesmo tempo, sem coordenação, e pelo menos uma escreve; o resultado fica imprevisível. O compilador do Rust impede esse erro. |
| **Disputa (contenção)** | Quando várias threads tentam usar o mesmo recurso (uma trava, um contador, uma linha de cache) ao mesmo tempo e ficam esperando umas pelas outras. |
| **Época** | Número que aumenta a cada lote de BUs aplicado; permite saber exatamente quais BUs estão incluídos numa resposta (ADR-005). |
| **`fetch_add` / `Atomics.add` / `getAndAdd`** | Operação atômica de "somar e guardar" em Rust, JavaScript e Java, respectivamente. |
| **Linha de cache** | Bloco de 64 bytes, a menor unidade que o processador traz da memória. Se duas threads escrevem em contadores na mesma linha, elas disputam essa linha. |
| **Lock / trava / mutex / RwLock** | Mecanismo que deixa só uma thread (ou várias leitoras e uma escritora, no caso do RwLock) acessar um dado por vez. Seguro, mas faz as threads esperarem. |
| **Lock-free / lockless ("sem trava")** | Técnica em que as threads coordenam o acesso sem travas, usando operações atômicas. A versão 1 dos documentos usava o termo incorretamente para o `DashMap`, que usa travas. |
| **Núcleo (*core*)** | Unidade de processamento independente do processador. |
| **Núcleos P e E** | Processadores Intel recentes têm núcleos de desempenho (P, *Performance*), mais rápidos, e de eficiência (E, *Efficient*), mais econômicos e lentos. O i7-14700 tem 8 P + 12 E. |
| **Hyper-threading / thread lógica** | Cada núcleo P executa duas threads ao mesmo tempo, dividindo seus recursos. Por isso o i7-14700 tem 20 núcleos e 28 threads lógicas (8 × 2 + 12). |
| **`nproc`** | Comando Linux que informa o número de threads lógicas disponíveis. |
| **Operação atômica** | Operação que o processador garante executar inteira, sem que outra thread veja um estado intermediário. Permite que várias threads somem no mesmo contador sem trava. |
| **Ordenação de memória `Relaxed`** | Modo mais barato de operação atômica: garante que a soma não se perde, mas não impõe ordem em relação a outras operações. Suficiente para contadores. |
| **Pool de threads / bloco (*chunk*)** | Conjunto fixo de threads que vão pegando tarefas. No benchmark, cada thread pega **blocos de 64 BUs** de cada vez. |
| **Seqlock** | Técnica de leitura consistente: o leitor anota a época, copia os dados e confere se a época não mudou; se mudou, lê de novo (ADR-005). |
| **Snapshot** | Cópia consistente do estado num instante. |
| **Thread** | Linha de execução independente dentro de um programa. Várias threads permitem usar vários núcleos ao mesmo tempo. |
| **Work-stealing** | Estratégia de pool em que uma thread ociosa "rouba" tarefas da fila de outra (usada pelo `rayon`). |
| **Worker (`worker_threads`)** | No Node.js, uma thread separada com seu próprio ambiente JavaScript. Cada worker custa ~10 MB de memória. |

## 6. Estruturas de dados

| Termo | Significado |
|---|---|
| **`AtomicU32` / `AtomicIntegerArray` / `Uint32Array` + `SharedArrayBuffer`** | O mesmo conceito nas três linguagens: um vetor de inteiros de 32 bits que várias threads podem incrementar com operações atômicas. O `SharedArrayBuffer` é a memória compartilhada entre workers do JavaScript. |
| **Bitset** | Vetor de bits, um por item. Na deduplicação, o bit *n* indica se a seção de ordinal *n* já foi contada: 500 mil seções cabem em 61 KB. |
| **Capacidade / fator de carga** | Uma tabela hash reserva mais espaço do que usa (capacidade) para continuar rápida; o fator de carga é a fração ocupada. Por isso ela gasta ~30 bytes por item mesmo guardando 12. |
| **`ConcurrentHashMap` / `LongAdder` / `AtomicLong`** | Estruturas concorrentes do Java: mapa que várias threads podem alterar, e contadores atômicos (o `LongAdder` espalha a soma em várias células para reduzir disputa). |
| **Contador `u32` / `u64`** | Inteiro sem sinal de 32 bits (até 4,29 bilhões, 4 bytes) ou 64 bits (8 bytes). Por município `u32` basta; UF e Brasil são somados em `u64`. |
| **Deduplicação / idempotência** | Garantir que o mesmo BU, se chegar duas vezes, seja contado uma só. Uma operação é idempotente quando repeti-la não muda o resultado. |
| **Delta / varint** | Formas de compactar números: delta grava a diferença em relação ao anterior; varint usa 1 byte para números pequenos e mais bytes só quando necessário. Usadas na estimativa de retenção por seção (ADR-009). |
| **`DashMap` / partição (*shard*)** | `DashMap` é um mapa concorrente do Rust dividido em várias **partições**, cada uma com sua trava. Por padrão cria `4 × threads` partições; cada uma ocupa memória mesmo vazia. Era o desenho da versão 1 e foi substituído (ADR-002). |
| **`hashbrown` / tabela hash** | Tabela hash é uma estrutura que encontra um valor a partir de uma chave calculando um "endereço" (hash) da chave. `hashbrown` é a implementação usada pelo Rust. |
| **Layout (denso)** | O "mapa" que diz em que posição do vetor fica o contador de cada (município, cargo, votável). É calculado antes da eleição a partir das listas públicas de candidatos. |
| **`Map` (JavaScript)** | Mapa chave → valor nativo do JavaScript. Não pode ser compartilhado entre workers. |
| **Mapa aninhado** | Mapa cujos valores são outros mapas (ex.: município → cargo → candidato → votos). Simples de escrever, caro em memória. |
| **Ordinal** | Número sequencial (0, 1, 2…) atribuído a cada item de uma lista conhecida, como as seções. Permite usar um vetor em vez de uma tabela hash. |
| **Região** | Trecho contíguo do vetor de contadores dedicado a um par (município, cargo). |
| **Vetor denso** | Vetor em que cada posição corresponde a um item conhecido de antemão, sem buracos e sem chaves. É a decisão do ADR-002: 9,35 M contadores × 4 bytes = 35,7 MB para a eleição geral. |

## 7. Linguagens, runtimes e bibliotecas

| Termo | Significado |
|---|---|
| **AOT (*ahead-of-time*) / GraalVM Native Image** | Compilar Java diretamente para código de máquina antes da execução, gerando um executável sem JVM. |
| **`asn1js`** | Biblioteca JavaScript para ASN.1; monta uma árvore de objetos para cada BU. Medida: ~186 × mais lenta que o parser manual em JS. |
| **`axum` / `tokio`** | `tokio` é o runtime assíncrono mais usado em Rust (bom para muitas conexões de rede); `axum` é um servidor HTTP sobre ele. Previstos para a API da Fase 4 (ADR-012). |
| **Bouncy Castle** | Biblioteca Java de criptografia e ASN.1, muito usada em sistemas bancários e governamentais. Medida: ~37 × mais lenta que o parser manual em Java. |
| **Bun / JavaScriptCore** | Bun é um runtime JavaScript alternativo ao Node.js; usa o motor JavaScriptCore (do Safari). |
| **Cargo / crate / `Cargo.lock`** | Cargo é a ferramenta de compilação e de dependências do Rust; *crate* é um pacote de código Rust; `Cargo.lock` fixa as versões exatas das dependências para a compilação ser reproduzível. |
| **`clap` / `serde` / `notify` / `rayon` / `sha2` / `memmap2`** | Crates Rust: argumentos de linha de comando; conversão para JSON/CSV; aviso de arquivos novos numa pasta; paralelismo com work-stealing; SHA-512; mapeamento de arquivos em memória. |
| **Edição do Rust (*edition* 2021)** | Versão das regras da linguagem usada no código. |
| **Event loop** | Modelo do Node.js em que uma única thread trata eventos em sequência; para usar vários núcleos é preciso criar workers. |
| **JIT (*just-in-time*) / aquecimento (*warm-up*)** | Java e JavaScript compilam o código para instruções de máquina durante a execução, depois de observar como ele é usado. As primeiras execuções são mais lentas (aquecimento). |
| **JVM / JDK / JRE** | JVM é a máquina virtual que executa Java; JRE é o pacote para rodar programas Java; JDK inclui também o compilador (`javac`). |
| **LTO / `codegen-units = 1` / `opt-level = 3` / *release*** | Opções de compilação do Rust para gerar o código mais rápido possível: *release* liga as otimizações; LTO (*link-time optimization*) otimiza o programa inteiro de uma vez; `codegen-units = 1` evita dividir a compilação; `opt-level = 3` é o nível máximo de otimização. |
| **N-API** | Interface para chamar código C/C++ a partir do Node.js. |
| **Node.js / V8 / npm** | Node.js é o runtime JavaScript fora do navegador; V8 é o motor JavaScript que ele usa (o mesmo do Chrome); npm é o gerenciador de pacotes. |
| **`rasn`** | Biblioteca Rust para ASN.1 que gera código a partir do esquema e decodifica para structs. Segura e fiel ao esquema, mas aloca memória; ~9 × mais lenta que o parser manual. É a referência dos testes (ADR-007). |
| **Runtime** | O ambiente que executa o programa: a JVM para Java, o Node.js para JavaScript. Rust compila para código de máquina e praticamente não tem runtime. |
| **Rust / rustup / rustc** | Rust é a linguagem do Plano A; `rustup` instala e atualiza o Rust; `rustc` é o compilador. |
| **Struct** | Registro com campos nomeados (equivalente a uma classe só com dados). |
| **WebAssembly (WASM)** | Formato que permite executar código compilado (como Rust) dentro do navegador, em velocidade próxima da nativa. Permite uma versão de auditoria no navegador sem reescrever o motor. |

## 8. Ambiente, arquivos e distribuição

| Termo | Significado |
|---|---|
| **API / REST / SSE / WebSocket** | API é uma interface para outros programas consultarem o motor; REST é o estilo de API HTTP usado; SSE e WebSocket são formas de o servidor enviar atualizações em tempo real ao cliente. |
| **Binário único / estático / musl** | Um só arquivo executável que roda sem instalar nada além dele. Binário estático não depende de bibliotecas do sistema; musl é uma biblioteca C usada para gerar binários estáticos no Linux. |
| **CLI** | Interface de linha de comando (programa usado pelo terminal). |
| **Cold start / inicialização** | Tempo entre chamar o programa e ele começar a trabalhar. |
| **Corpus** | O conjunto de BUs sintéticos gerado para o teste (20 mil BUs distintos, percorridos em ciclo até somar 500 mil). Não é versionado; é gerado por `bench/scripts/gerar_bus.sh`. |
| **Deflate / zip** | Algoritmo de compressão usado em arquivos `.zip`. |
| **Docker / imagem `scratch`** | Docker empacota um programa com seu ambiente; `scratch` é uma imagem vazia, usada para imagens mínimas. |
| **NTFS** | Sistema de arquivos do Windows. Abrir centenas de milhares de arquivos pequenos nele (com antivírus ativo) é lento, por isso a leitura direta dos `.zip` (ADR-010). |
| **Parquet / Arrow / CSV / JSON** | Formatos de exportação: Parquet e Arrow são colunares, para análise de dados em volume; CSV e JSON são textuais. |
| **SSD NVMe** | Disco de estado sólido rápido, ligado direto ao barramento PCIe. |
| **venv** | Ambiente Python isolado; o benchmark cria um em `~/uvbench/.venv` para instalar o `matplotlib` sem mexer no Python do sistema. |
| **WSL2** | *Windows Subsystem for Linux*: roda um Linux completo dentro do Windows. Os benchmarks rodam nele. |

## 9. Medição e estatística

| Termo | Significado |
|---|---|
| **Benchmark** | Programa que mede desempenho (tempo, memória) de forma repetível. |
| **Checksum** | Número calculado a partir de todos os totais; se duas implementações chegam ao mesmo checksum, totalizaram igual. `bench/scripts/verificar.sh` exige o mesmo checksum em todas as variantes. |
| **Determinístico / semente** | O gerador usa números pseudoaleatórios a partir de uma semente fixa: os mesmos parâmetros geram exatamente os mesmos BUs em qualquer máquina. |
| **Distribuição de Pareto (cauda pesada)** | Modelo estatístico em que poucos itens são muito grandes e a maioria é pequena. Usado para distribuir as seções entre municípios (poucas capitais enormes, muitos municípios pequenos). |
| **Extrapolação linear** | Estimar o tempo de 500 mil BUs a partir de uma amostra menor, multiplicando proporcionalmente. Usada nas variantes muito lentas (Bouncy Castle, `asn1js`). |
| **Fixture** | Arquivo de dados fixo usado nos testes (ex.: um BU real de uma eleição passada). |
| **Mediana** | Valor do meio de uma lista ordenada; menos sensível a medições discrepantes que a média. Os relatórios usam a mediana das repetições. |
| **Teste de verificação (*smoke test*)** | Teste rápido para confirmar que tudo funciona antes de medir. |
| **Teste diferencial** | Rodar duas implementações independentes com a mesma entrada e exigir resultados idênticos (parser manual × `rasn`). |
| **Throughput / vazão** | Quantidade processada por unidade de tempo (BUs por segundo). |
| **Variante** | Uma combinação de linguagem + parser + estrutura de estado medida pelo benchmark (ex.: `rust manual-dense`). Lista em [ESTIMATIVAS §1.2](ESTIMATIVAS_MEMORIA_E_TEMPO.md#12-variantes-medidas). |

## 10. Unidades

| Unidade | Significado |
|---|---|
| **µs, ms, s** | Microssegundo (milionésimo de segundo), milissegundo (milésimo), segundo. |
| **B, KB, MB, GB** | Byte, kilobyte, megabyte, gigabyte. Nas medições, 1 MB = 1.048.576 bytes (base 2). |
| **M** | Milhões (ex.: 9,35 M contadores). |
| **BUs/s** | Boletins de urna processados por segundo. |
| **×** | "vezes" (ex.: 9 × mais lento). |

## 11. Rede e aquisição

Termos da fase de aquisição ([AQUISICAO.md](AQUISICAO.md), ADR-013 a ADR-018).

| Termo | Significado |
|---|---|
| **429 (Too Many Requests)** | Resposta HTTP que um servidor devolve quando o cliente passou do limite de requisições. Na borda do CDN do TSE vem com `cdn-cache-status: Error from child` e põe o IP numa caixa de penalidade. |
| **AIMD** (*additive increase, multiplicative decrease*) | Regra de controle de taxa: sobe devagar (soma um pouco por segundo) enquanto tudo vai bem e corta pela metade quando o servidor reclama (429). É a mesma ideia do controle de congestionamento do TCP. Usada pelo `uv-coletor --adaptativo`, sempre abaixo do teto de 100 req/s. |
| **ALPN** | Parte do aperto de mão TLS em que cliente e servidor combinam o protocolo (HTTP/2 ou HTTP/1.1). |
| **aux.json** | Arquivo JSON de cada seção no site de resultados: diz a situação da seção e lista os arquivos de urna publicados (BU, RDV, log, assinaturas) e o *hash* do diretório onde estão. |
| **Borda / origem (CDN)** | A borda é o servidor do CDN mais próximo do cliente, que guarda cópias (cache); a origem é o servidor do TSE de onde a borda busca o que não tem. |
| **Cache hit / miss** | *Hit*: a borda já tinha o arquivo (resposta em ~0,1 s). *Miss*: precisou buscar na origem (~0,5 s). Quase todo aux.json e BU é *miss*, porque cada um é pedido por poucas pessoas. |
| **Caixa de penalidade** (*penalty box*) | Período em que a borda do CDN recusa (429) todas as requisições de um IP que passou do limite. Medido: ~8 minutos. |
| **CDN** (*Content Delivery Network*) | Rede de servidores espalhados que entrega arquivos em nome do site (aqui, Akamai). Aguenta milhões de acessos na noite da eleição. |
| **CKAN** | Software do portal de dados abertos do TSE (`dadosabertos.tse.jus.br`); tem uma API para listar conjuntos de dados e seus arquivos. |
| **Concorrência (de rede)** | Quantas requisições estão em andamento ao mesmo tempo. |
| **cs.json** | Lista de municípios, zonas e seções de uma UF num pleito, publicada pelo site de resultados. O Brasil inteiro tem 517.179 seções no 1º turno de 2026. |
| **Dados abertos (zip por UF)** | Pacote publicado dias depois da eleição com todos os arquivos de urna de uma UF (`bu_imgbu_logjez_rdv_vscmr_<ano>_<turno>t_<UF>.zip`). |
| **Diretório central (zip)** | Índice no fim de um arquivo zip com o nome, o tamanho e a posição de cada entrada. Lendo só ele, dá para baixar entradas avulsas com HTTP Range. |
| **ETag / If-None-Match / 304** | ETag é uma "impressão digital" de uma versão do arquivo. Quem já tem a versão pergunta com `If-None-Match`; se nada mudou, o servidor responde `304 Not Modified`, sem corpo. É como o `acompanhar` relê os `cs.json` sem baixar tudo de novo. |
| **HTTP/1.1 × HTTP/2** | No HTTP/1.1, cada requisição em andamento ocupa uma conexão TCP. O HTTP/2 multiplexa muitas requisições numa conexão só. |
| **HTTP Range / 206** | Pedido de só um pedaço de um arquivo (`Range: bytes=a-b`); o servidor responde `206 Partial Content`. |
| **Latência** | Tempo entre enviar uma requisição e receber a resposta. |
| **Lei de Little** | Numa fila estável, vazão = itens em andamento ÷ tempo de cada um. Para a coleta: req/s ≈ concorrência ÷ latência. Com latência de 0,5 s, 50 requisições em andamento dão ~100 req/s. |
| **Limitado por latência** | Situação em que o que limita a vazão é a espera pela resposta, não a CPU nem a banda. É o caso da coleta de BUs. |
| **NAT / IP de saída** | Numa rede corporativa, muitos computadores saem para a internet pelo mesmo IP público. Um limite "por IP" no CDN vale para todos eles juntos. |
| **Pleito** | Conjunto de eleições realizadas no mesmo dia (ex.: 3220 = 1º turno de 2026, com as eleições federal 6257 e estadual 6259). Um BU traz os resultados de todas as eleições do pleito naquela seção. |
| **Teto de requisições** | Limite de 100 requisições por segundo aos servidores do TSE, imposto no código do `uv-coletor` (`TETO_RPS`). |
| **Totalização em fluxo** | Totalizar cada BU no momento em que ele chega, em vez de esperar todos chegarem. |
| **`.uvpk`** | Pacote de BUs do Urna Velox: um arquivo por UF, só de acréscimo, com um cabeçalho de 52 bytes por BU e o BU intocado (ADR-016). |
| **`.vsc` / hash do diretório** | Arquivo de assinaturas digitais da seção. O nome do diretório de cada BU no site é o SHA-256 desse arquivo (em base64, escrito em hexadecimal). |
| **Comparação Urna Velox × TSE** | Modo da página de resultados (etapa 7, `0.7.0`) que mostra lado a lado a totalização do Urna Velox e a do site de resultados do TSE, com um controle "Urna Velox · Ambos · TSE". O servidor busca só o arquivo da tela, no máximo a cada 30–60 s ([HANDOFF_0.7.0](../handoffs/HANDOFF_0.7.0.md)). |
| **Previsão (do matematicamente / do fim)** | Estimativa, com intervalo P10–P90, do horário em que sai o matematicamente eleito / 2º turno e do fim da apuração, calculada pela velocidade de chegada dos BUs (eleitores aptos por minuto) e pela tendência dos votos por estrato (etapa 8, `0.8.0`, [HANDOFF_0.8.0](../handoffs/HANDOFF_0.8.0.md)). Nunca é resultado. |
| **P10 / P50 / P90** | Percentis de uma estimativa: 10 % dos cenários simulados ficam abaixo do P10, metade abaixo do P50 (a estimativa central) e 90 % abaixo do P90. A faixa P10–P90 é o intervalo mostrado na previsão. |
