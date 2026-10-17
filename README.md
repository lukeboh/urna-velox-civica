# ⚡ Urna Velox

[![Rust](https://img.shields.io/badge/rust-stable%20%7C%20edition%202021%2B-orange.svg?logo=rust)](https://www.rust-lang.org)
[![License: PolyForm Noncommercial 1.0.0](https://img.shields.io/badge/license-PolyForm%20Noncommercial%201.0.0-blue.svg)](LICENSE)
[![Source-Available](https://img.shields.io/badge/modelo-source--available%20%7C%20dupla%20licen%C3%A7a-lightgrey.svg)](docs/LICENCIAMENTO.md)
[![Status](https://img.shields.io/badge/status-active%20development-brightgreen.svg)](#roadmap)
[![Engine](https://img.shields.io/badge/engine-zero--copy%20%7C%20concurrent-purple.svg)](#arquitetura)

> **Motor de totalização eleitoral independente de alta performance, projetado em Rust para processamento contínuo de Boletins de Urna (BUs) do TSE com agregação analítica instantânea em nível Municipal, Estadual e Nacional.**

---

## 🎯 Visão e Missão

O **Urna Velox** nasceu para resolver o desafio de auditoria cívica e totalização em escala do processo eleitoral brasileiro. Em eleições gerais e municipais, a Justiça Eleitoral (TSE) disponibiliza centenas de milhares de arquivos brutos de Boletim de Urna (`.bu`) codificados em ASN.1/BER.

Diferente de abordagens baseadas em scripts em lote (*batch*) sequenciais ou cargas pesadas em bancos de dados relacionais que demoram horas para consolidar resultados, o **Urna Velox** opera como um **motor analítico in-memory contínuo e concorrente**:

- **Agregação Virtualmente Instantânea:** À medida que cada arquivo `.bu` ou lote compactado é lido do disco ou recebido via rede, os totais consolidados para **Município**, **UF** e **Brasil** são atualizados em microssegundos.
- **Zero-Copy & Parser ASN.1 Nativo:** Decodificação de baixo nível dos formatos oficiais do TSE sem alocações supérfluas de memória.
- **Eficiência de Recursos Extrema:** O estado completo de uma eleição geral (~500.000 BUs) ocupa ~36 MB num vetor denso de contadores pré-dimensionado a partir dos dados públicos de candidatura; o motor inteiro fica na casa de 50 MB, saturando todos os núcleos de CPU disponíveis ([estimativas medidas](docs/ESTIMATIVAS_MEMORIA_E_TEMPO.md)).
- **Auditoria Cívica Aberta (*Source-Available*):** Código-fonte público e determinístico (licença [PolyForm Noncommercial 1.0.0](LICENSE)), viabilizando a conferência pública e independente dos resultados das urnas eletrônicas brasileiras por pesquisadores, partidos, entidades fiscalizadoras e a sociedade civil.

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

Como o projeto tem dupla licença (ver abaixo), ao enviar uma contribuição você concorda que ela seja distribuída pelo autor tanto sob a PolyForm Noncommercial 1.0.0 quanto sob as licenças comerciais do projeto.

---

## ⚖️ Licença e Termos de Uso

O Urna Velox é **Source-Available / PolyForm Noncommercial**: o código-fonte é público para garantir a transparência cívica, mas **não é *open source*** segundo os critérios da OSI. O projeto adota **dupla licença** (*dual licensing*):

| Licença | Para quem | Custo |
| :--- | :--- | :--- |
| [**PolyForm Noncommercial License 1.0.0**](LICENSE) ([em português](LICENCA.pt-BR.md)) | qualquer uso **sem finalidade comercial** | gratuito |
| **Licença Comercial** (contrato específico com o autor) | qualquer uso **com finalidade econômica ou lucrativa** | royalties, sob contrato |

Copyright © 2026 Luciano Soares Bohnert. O texto que vale juridicamente é o do [LICENSE](LICENSE) (texto oficial e integral da PolyForm Noncommercial 1.0.0, em inglês), com tradução para o português do Brasil de conteúdo idêntico em [LICENCA.pt-BR.md](LICENCA.pt-BR.md); esta seção apenas os explica.

### ✅ Uso não comercial — gratuito e livre

Qualquer pessoa tem liberdade irrestrita para **auditar, compilar, modificar, estudar e utilizar** o Urna Velox, sem pagar nada e sem pedir autorização, para:

- **pesquisa acadêmica** e científica (TCC, dissertação, tese, artigo);
- **auditoria cidadã** e conferência independente dos resultados eleitorais;
- **fins educacionais** (aulas, cursos, material didático);
- **uso pessoal privado sem fins lucrativos** (estudo, experimento, hobby).

A licença também permite o uso por instituições de ensino, organizações de pesquisa pública, entidades beneficentes e instituições governamentais, qualquer que seja a fonte do seu financiamento. Quem redistribuir o software (com ou sem modificações) deve repassar o texto da licença (ou o seu endereço) e as linhas `Required Notice:` do [LICENSE](LICENSE). O software é fornecido **"no estado em que se encontra" (*as is*)**, sem garantia de qualquer tipo e sem responsabilidade do autor por danos, nos limites da lei.

### 🚫 Uso comercial — exige licença comercial e royalties

Qualquer exploração, **direta ou indireta**, com finalidade econômica ou lucrativa **não é coberta** pela licença PolyForm Noncommercial e é vedada sem autorização prévia e por escrito do autor. Isso inclui, mas não se limita a:

- **venda, licenciamento, sublicenciamento ou oferta como serviço** (SaaS, API paga, *feed* de dados, painel com assinatura, anúncios ou patrocínio);
- **incorporação em soluções ou coberturas remuneradas** de consultorias, veículos de mídia (TV, rádio, portais, agências de notícias), campanhas eleitorais ou partidos políticos;
- **monetização em transmissões ao vivo, palestras remuneradas ou conteúdos audiovisuais comerciais** que utilizem o software como atrativo econômico.

Na dúvida sobre se um uso é comercial, consulte o autor antes de usar.

### 💼 Contato para licenciamento comercial

Empresas, consultorias, veículos de comunicação, campanhas e criadores de conteúdo que desejem fazer uso comercial do Urna Velox devem **entrar em contato formal com o autor para firmar um contrato específico de licenciamento comercial e pagamento de royalties** antes de qualquer uso. Detalhes no [Guia de Licenciamento](docs/LICENCIAMENTO.md).

- **Titular dos direitos autorais:** Luciano Soares Bohnert
- **E-mail:** `luciano.bohnert@gmail.com`
