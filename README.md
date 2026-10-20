# [⚡ Urna Velox](https://urnavelox.com.br)

![Urna Velox: totalização eleitoral independente, BU por BU](docs/img/capa.png)

> ### 🌐 Acompanhe a apuração ao vivo: [urnavelox.com.br](https://urnavelox.com.br)

[![Rust](https://img.shields.io/badge/rust-stable%20%7C%20edition%202021%2B-orange.svg?logo=rust)](https://www.rust-lang.org)
[![Status](https://img.shields.io/badge/status-active%20development-brightgreen.svg)](#roadmap)
[![Engine](https://img.shields.io/badge/engine-zero--copy%20%7C%20concurrent-purple.svg)](#arquitetura)

> **Motor de totalização eleitoral independente de alta performance, projetado em Rust para processamento contínuo de Boletins de Urna (BUs) do TSE com agregação analítica instantânea em nível Municipal, Estadual e Nacional.**

---

## ✨ Para quem acompanha a eleição

Abra a página na noite da eleição e veja a apuração do jeito que ela acontece. Nada de instalar programa ou criar
conta: funciona no navegador do celular e do computador.

- **⚡ Resultado em tempo quase real.** Cada Boletim de Urna é somado assim que o TSE o recebe. Entre o BU chegar e o
  número mudar na sua tela passam milissegundos, e o placar anda junto com as urnas, sem esperar o ciclo do site
  oficial.
- **🏁 O momento em que a eleição se decide.** O Urna Velox calcula, BU a BU, quando o resultado **não pode mais
  mudar**, nem que todos os eleitores ainda não apurados votassem no adversário. É o *matematicamente eleito* (ou
  *matematicamente 2º turno*), com a hora exata e até a seção eleitoral que decidiu a disputa.
- **⏳ Quantos eleitores faltam para decidir.** Um contador regressivo mostra quantos eleitores ainda precisam ser
  apurados até o resultado ficar garantido. Dá para ver a disputa "fechando" ao vivo.
- **🔮 A que horas deve sair.** Uma estimativa, sempre com faixa ("entre 20h10 e 20h35"), do horário em que o
  resultado fica definido e de quando a apuração termina, recalculada ao longo da noite. Depois, a página mostra se
  acertou. Se a chegada dos BUs parar, ela avisa e vai adiando a estimativa, em vez de fingir certeza.
- **📰 Letreiro de notícias da apuração.** Uma faixa que corre no pé da tela, como os letreiros dos canais de notícias
  e de economia, com o que vai acontecendo:
  - a primeira seção apurada do país e de cada estado;
  - o primeiro governador ou prefeito eleito;
  - as viradas, quando o 1º colocado muda;
  - quantos eleitores faltam para decidir as disputas mais perto do fim;
  - os marcos de 25%, 50%, 90% e 100% das seções.

  O ritmo de BUs por minuto fica à vista o tempo todo, num chip logo abaixo do topo.

  Tocou num item, ele abre o detalhe. Perdeu algo, é só abrir o histórico com tudo o que já passou, com busca.
- **🕐 Dois relógios, sem confusão.** A página separa o horário do Urna Velox do horário do TSE. "Totalizado pelo
  Urna Velox às…" é a hora real em que o sistema somou o último BU; logo abaixo, "último BU no TSE: … de dd/mm" é a
  hora em que esse boletim chegou à Justiça Eleitoral. A curva, a previsão e o letreiro usam o relógio do Urna Velox;
  o horário do TSE de cada item fica no detalhe.
- **⚖️ Lado a lado com o TSE.** Um botão põe a contagem do Urna Velox ao lado da do site oficial do TSE, candidato
  por candidato, com a diferença e a idade do arquivo do TSE. Quando o TSE confirma o resultado, a página mostra
  quanto tempo antes o Urna Velox já tinha chegado lá, medido no mesmo relógio. Num simulado de outro dia (ou
  acelerado), o horário do TSE é trazido para o dia e a velocidade do simulado antes da comparação: assim a distância
  entre as duas apurações continua certa.
- **📈 A curva da noite.** Um gráfico com o avanço das seções e o percentual dos dois primeiros colocados desde o
  começo da apuração.
- **🗺️ Do Brasil ao seu município.** Presidente no Brasil, Governador e Senador no seu estado, Prefeito na sua cidade:
  cada abrangência e cada cargo têm a sua tela, com comparecimento, abstenção, brancos e nulos.
- **🔍 Aberto e conferível.** Os números vêm dos BUs públicos do TSE, e o código é aberto. Em todas as eleições já
  reproduzidas (2024 e 2026), o resultado final foi idêntico ao oficial, candidato por candidato. O Urna Velox não
  substitui a divulgação oficial do TSE: é uma forma independente de acompanhá-la e conferi-la.

### 🎬 Simulados com dados reais e a Playlist de Simulados

Fora da noite da eleição, o site fica totalizando de novo, sem parar, eleições que já aconteceram: o 1º e o 2º turno
de 2024 e o 1º turno de 2026. **Os dados são os reais**: os boletins de urna publicados pelo TSE, nenhum voto
inventado. O que se repete é a **totalização**: a cada ciclo o Urna Velox começa do zero e soma de novo todos os
boletins, na ordem em que chegaram à Justiça Eleitoral. Assim dá para ver a totalização funcionando e a **auditoria**
acontecendo: no fim de cada ciclo, o resultado é conferido com o oficial do TSE, candidato por candidato.

- **Na página da apuração:** o chip "Dados reais do TSE · retotalização" diz qual eleição está passando (com a data em
  que ela aconteceu) e em que modo. Tocou nele, abre a explicação, que também aparece sozinha na primeira visita do
  dia. O selo do topo leva a marca "simulação".
- **Dois modos:**
  - **tempo real:** os boletins entram no mesmo ritmo e na mesma ordem da noite da eleição (leva horas);
  - **mesma ordem, acelerado:** a mesma ordem de chegada, o mais rápido que o sistema consegue totalizar (alguns
    minutos).
- **▶ Playlist de Simulados das Eleições** ([/playlist](https://urnavelox.com.br/playlist)), para escolher o que
  passa a seguir:
  - mostra o que está no ar e a agenda, com o horário previsto de início (sempre numa hora cheia) e de fim, a eleição,
    a data em que ela aconteceu e o modo;
  - qualquer pessoa vota no simulado que quer ver a seguir, com um voto por conexão (endereço IP), que pode ser
    trocado ou retirado;
  - o mais votado é sempre o próximo; sem votos, segue a rotação automática.
- **Pausa nas eleições oficiais:** de um dia antes até três dias depois de uma eleição oficial, a playlist fica
  suspensa e o site acompanha a eleição de verdade.

Como funciona por dentro (rotação, vigias dos nós, votação e suspensão): [docs/CICLO_DIARIO.md](docs/CICLO_DIARIO.md).

---

## 🎯 Visão e Missão

O **Urna Velox** nasceu para resolver o desafio de auditoria cívica e totalização em escala do processo eleitoral brasileiro. Em eleições gerais e municipais, a Justiça Eleitoral (TSE) disponibiliza centenas de milhares de arquivos brutos de Boletim de Urna (`.bu`) codificados em ASN.1/BER.

Diferente de abordagens baseadas em scripts em lote (*batch*) sequenciais ou cargas pesadas em bancos de dados relacionais que demoram horas para consolidar resultados, o **Urna Velox** opera como um **motor analítico in-memory contínuo e concorrente**:

- **Agregação Virtualmente Instantânea:** À medida que cada arquivo `.bu` ou lote compactado é lido do disco ou recebido via rede, os totais consolidados para **Município**, **UF** e **Brasil** são atualizados em microssegundos.
- **Zero-Copy & Parser ASN.1 Nativo:** Decodificação de baixo nível dos formatos oficiais do TSE sem alocações supérfluas de memória.
- **Eficiência de Recursos Extrema:** O estado completo de uma eleição geral (~500.000 BUs) ocupa ~36 MB num vetor denso de contadores pré-dimensionado a partir dos dados públicos de candidatura; o motor inteiro fica na casa de 50 MB, saturando todos os núcleos de CPU disponíveis ([estimativas medidas](docs/ESTIMATIVAS_MEMORIA_E_TEMPO.md)).
- **Auditoria Cívica Aberta:** Código-fonte público e determinístico, viabilizando a conferência pública e independente dos resultados das urnas eletrônicas brasileiras por pesquisadores, partidos, entidades fiscalizadoras e a sociedade civil.

---

## 🏛️ Granularidade e Modelo de Agregação

O Urna Velox mantém os totais municipais num vetor denso de contadores atômicos; UF e Brasil são somas derivadas dos municípios, calculadas sob demanda:

```
[ Boletim de Urna (.bu) ] ──> [ Parser ASN.1/BER ] ──> [ Validador & Deduplicador ]
                                                                │
                                      ┌─────────────────────────┴─────────────────────────┐
                                      ▼                                                   ▼
                          [ Dimensão Eleitoral ]                               [ Dimensão Geográfica ]
                     (Eleição / Cargo / Candidato)                        (Município ──> UF ──> Brasil)
                                      │                                                   │
                                      └─────────────────────────┬─────────────────────────┘
                                                                ▼
                                                [ Tabela Agregada em Memória ]
                                             (Totalização Instantânea Multiescopo)
```

1. **Nível Município:** Consolidação local imediata de votos nominais, votos de legenda, brancos e nulos por cargo.
2. **Nível Unidade da Federação (UF):** Soma dos municípios de cada estado (e Distrito Federal / Votação no Exterior).
3. **Nível Brasil (Nacional):** Soma das UFs para cargos nacionais (Presidência da República) e indicadores gerais de comparecimento e abstenção. A consolidação completa leva poucos milissegundos.

---

## 🛠️ Stack Tecnológica

| Componente | Tecnologia | Justificativa |
|---|---|---|
| **Linguagem Principal** | **Rust** (Edição 2021+) | Compilação nativa, previsibilidade total de latência, ausência de coletor de lixo (GC) e garantias formais de segurança de memória e ausência de *data races*. |
| **Parser ASN.1 / BER** | Parser BER manual zero-copy + `rasn` como referência | Caminho rápido lê os bytes do `.bu` sem alocar; o esquema oficial compilado com `rasn` confere o resultado nos testes (ADR-007). |
| **Paralelismo Massivo** | threads nativas / `rayon` | Uma thread por núcleo lógico, divisão do trabalho em blocos de BUs. |
| **Estado de Totalização** | `Vec<AtomicU32>` denso | Um contador por (município, cargo, votável), layout pré-computado; incremento atômico relaxado, sem travas (ADR-002, ADR-003). |
| **Motor** | crate `urna-velox` (`motor/`) | Leitor de BU, catálogo, estado denso, situação dos candidatos, matematicamente eleito / 2º turno, publicação em JSON e conferência com o oficial ([ARCHITECTURE §2.5](ARCHITECTURE.md)). |
| **Aquisição** | `tokio` + `reqwest` (HTTP/2, `rustls`), crate `aquisicao/` | Coleta dos BUs no site de resultados e nos dados abertos do TSE, com teto de 100 req/s e controle adaptativo; entrega ao núcleo por canal e pacotes `.uvpk` (ADR-013 a ADR-016). |
| **I/O & Streaming** | `zip` & `notify` (Fases 1–3); `tokio` + `axum` (Fase 4) | Leitura direta dos pacotes compactados do TSE; runtime assíncrono só na camada HTTP e na aquisição (ADR-010, ADR-012, ADR-014). |
| **Interface CLI** | `clap` (v4) | CLI robusta, tipada e ergonômica para operações, benchmarks e diagnósticos via terminal. |
| **Serialização de Saída** | `serde` (`serde_json`, `csv`) | Exportação instantânea de snapshots e relatórios consolidados em formatos abertos. |

---

## 🚀 Guia de Início Rápido

### Pré-requisitos
- **Rust Toolchain**: Rust 1.82+ com Cargo instalado ([rustup.rs](https://rustup.rs/)).

### Compilação

Clone o repositório e compile o binário com otimizações de release:

```bash
git clone https://github.com/lukeboh/urna-velox-civica.git
cd urna-velox
cargo build --release
```

O binário otimizado estará disponível em `target/release/uv-motor` (ou `target/release/uv-motor.exe` no Windows); todos os executáveis do projeto começam com `uv-`.

### Comandos do motor (`v0.1.0`)

O workspace tem dois binários: **`uv-motor`** (o motor, crate `urna-velox` em `motor/`) e **`uv-coletor`** (a aquisição, `aquisicao/`).
Os dados (pacotes `.uvpk` de BUs, arquivos oficiais `-u.json`, métricas da coleta) ficam fora do repositório.

```bash
# totaliza em lote e grava o resultado de cada cargo e abrangência (JSON, docs/FORMATO_RESULTADO.md)
./target/release/uv-motor totalizar --dados ~/uvreal/dados --oficial ~/uvreal/oficial --threads 28 --saida resultado/

# confere votos e situação de cada candidato com a divulgação oficial do TSE (código 1 se divergir)
./target/release/uv-motor conferir --dados ~/uvreal/dados --oficial ~/uvreal/oficial --metricas ~/uvreal/metricas_nacional.csv --vagas 5=2

# reproduz a noite em ordem de chegada dos BUs: quando cada circunscrição fica matematicamente decidida
./target/release/uv-motor noite --dados ~/uvreal/2024-2t/dados --oficial ~/uvreal/2024-2t/oficial \
  --chegadas ~/uvreal/2024-2t/metricas_2t.csv --saida noite/

# conteúdo de um BU
./target/release/uv-motor inspecionar o03220df9701200010003-bu.dat
```

Coleta dos BUs, replay da noite com publicação contínua e os demais utilitários: [aquisicao/README.md](aquisicao/README.md).
Testes: `cargo test --release -p urna-velox` (unidade + fixtures) e, com os dados locais,
`UV_DADOS=~/uvreal cargo test --release -p urna-velox -- --ignored` (as conferências completas). Detalhes em
[motor/README.md](motor/README.md).

---

## 📚 Documentação Técnica

- [docs/AQUISICAO.md](docs/AQUISICAO.md): Fase de aquisição: como os BUs são obtidos do TSE (API de resultados e dados abertos), arquitetura do coletor, limites do CDN, medições reais e gráficos. Código em [aquisicao/](aquisicao/) (`uv-coletor`); para repetir: `./aquisicao/teste-real.sh` ou `.\aquisicao\teste-real.ps1`.
- [docs/FORMATO_RESULTADO.md](docs/FORMATO_RESULTADO.md): Formato JSON do resultado por cargo e abrangência (situação, matematicamente eleito), contrato do WebSocket e do site.
- [docs/CICLO_DIARIO.md](docs/CICLO_DIARIO.md): Simulados contínuos com dados reais nos dois nós da OCI: rotação por hora cheia, modos tempo real e acelerado, Playlist de Simulados (votação por IP, decisão única nos dois nós, suspensão nas eleições oficiais), telemetria e conferência de cada ciclo.
- [ARCHITECTURE.md](ARCHITECTURE.md): Detalhamento do pipeline de parsing, modelo de concorrência, estruturas em memória e garantias de idempotência e integridade.
- [docs/ARCHITECTURE_DECISIONS.md](docs/ARCHITECTURE_DECISIONS.md): Registro de decisões (ADR-000 a ADR-031): linguagem (Rust vs. JavaScript vs. Java), modelo de estado, concorrência, deduplicação, validação, orçamento de memória, critérios de pivot, federação e licenciamento (ADR-031: Source-Available / PolyForm Noncommercial com dupla licença).
- [docs/ESTIMATIVAS_MEMORIA_E_TEMPO.md](docs/ESTIMATIVAS_MEMORIA_E_TEMPO.md): Cálculos de memória e tempo de totalização de uma eleição real nas três linguagens, medidos com o mesmo corpus sintético, com gráficos.
- [bench/](bench/): Benchmarks reproduzíveis (Rust, Java, Node.js) e resultados brutos. Para repetir tudo: `./bench/benchmark.sh` (Linux/WSL2) ou `.\bench\benchmark.ps1` (Windows).
- [docs/GLOSSARIO.md](docs/GLOSSARIO.md): Significado de todos os termos técnicos usados na documentação (ADR, BER, zero-copy, operação atômica, CDN, HTTP 429, AIMD etc.).
- [ROADMAP.md](ROADMAP.md): Planejamento de engenharia por etapas e versões (motor, catálogo, operação automatizada, WebSocket, site, federação, comparação com o TSE, previsão, ensaio geral → `1.0.0`; proporcionais → `2.0.0`).

---

## 🤝 Contribuição e Transparência Cívica

O **Urna Velox** é um projeto de soberania informacional, auditoria cidadã e pesquisa eleitoral. Contribuições da comunidade técnica são muito bem-vindas para fins cívicos e acadêmicos!

1. Faça um Fork do projeto.
2. Crie uma branch para sua funcionalidade ou correção: `git checkout -b feature/minha-feature`.
3. Garanta que o código passa no linter e testes: `cargo fmt --check && cargo clippy && cargo test`.
4. Envie suas alterações via Pull Request com descrição clara dos objetivos técnicos.

Ao contribuir, valem as condições do [licenciamento](docs/LICENCIAMENTO.md#contribuições).

---

## ⚖️ Licença e Termos de Uso

O Urna Velox é **livre para qualquer uso não comercial** — pesquisa, auditoria cidadã, ensino e uso pessoal —, sem custo e sem pedir autorização. Detalhes em [docs/LICENCIAMENTO.md](docs/LICENCIAMENTO.md).
