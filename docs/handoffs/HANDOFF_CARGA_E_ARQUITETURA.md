# Handoff — Testes de Carga, Estresse e Subsídios para Refatoração Arquitetural

*Preparado em 20/10/2026 para balizar a discussão de capacidade, limites e refatoração arquitetural de distribuição e difusão em tempo real do Urna Velox antes do 2º turno de 2026 (25/10/2026).*

> **Revisado em 20/10/2026, 19h (Claude), com o painel dos nós do teste das 18h21.** Partes das conclusões abaixo não se
> sustentam nas evidências: a "capacidade máxima" é o limite configurado (não uma medida), as latências de produção
> têm de 0 a 37 amostras, a pasta de evidências citada era a de outra rodada, e o gargalo real dos nós é a **CPU**
> (2 OCPUs), não memória nem kernel. O que vale para o 2º turno está na **[§7](#7-revisão-de-2010-19h-o-que-vale-para-o-2º-turno)**;
> o plano de trabalho, no [HANDOFF_MODELO_BORDA_E_CONSISTENCIA.md](HANDOFF_MODELO_BORDA_E_CONSISTENCIA.md).

**Caminhos:**
* Repositório: `D:\git\lukeboh\urna-velox` (Windows) = `/mnt/d/git/lukeboh/urna-velox` (WSL `Ubuntu-24.04`)
* Binários: `/home/dev/uvreal/target-dev/release/{uv-servico, uv-simulador, uv-carga}`
* Scripts de teste: `/mnt/d/git/lukeboh/urna-velox/scripts/{teste_carga.py, teste_carga.sh}`
* Evidências locais: `/home/dev/uvreal/ensaios/carga_local_20261020_111259/`
* Evidências produção: `/home/dev/uvreal/ensaios/carga_producao_20261020_182157/` (0.8.41; é dela a tabela da §3.2). *Correção de 20/10, 19h:* o texto citava a `carga_producao_20261020_114835/` (0.8.37), em que o degrau de 12.000 falhou do lado do **cliente** (`Too many open files`: 1.769; `Connection timed out`: 1.130) e nenhum degrau tem amostra de latência (`n: 0`).

---

## 1. Contexto e Motivação

O **Urna Velox** adota uma arquitetura em que o núcleo de totalização (motor em Rust puro, síncrono e sem I/O) entrega versões de resultados a uma thread de difusão (`uv-difusao`), que publica os dados via **WebSocket** (`/ws`) e REST (`/api/v1/resultado/<topico>`) através do `axum 0.8` no binário `uv-servico`. Em produção (OCI Ampere A1), a terminação TLS/HTTPS e o balanceamento entre os nós A e B são realizados pelo **Caddy** via `least_conn`.

O objetivo desta sessão foi:
1. Submeter o sistema a testes de carga em degraus progressivos (de 500 a 15.000 conexões simultâneas).
2. Determinar a capacidade máxima com SLA estrito (**latência BU → cliente < 50 ms no p99**).
3. Executar o teste de estresse contra a infraestrutura de **produção** (`https://urnavelox.com.br`) até encontrar o **ponto de quebra/saturação**, identificando os gargalos exatos da arquitetura atual para subsidiar eventuais refatorações.

---

## 2. O que foi Implementado e Instrumentado

1. **Suporte a TLS/WSS no `uv-carga` ([`aquisicao/src/bin/uv-carga.rs`](/mnt/d/git/lukeboh/urna-velox/aquisicao/src/bin/uv-carga.rs)):**
   - Habilitada a feature `rustls-tls-webpki-roots` em `tokio-tungstenite` no [`aquisicao/Cargo.toml`](/mnt/d/git/lukeboh/urna-velox/aquisicao/Cargo.toml).
   - Implementada a função `urls(end)` para desambiguação automática entre endpoints locais (`http://` e `ws://`) e endpoints de produção com terminação TLS (`https://` e `wss://`).
   - Corrigido o cálculo de tempo de encerramento do cliente (`fim_em = t0 + duracao + tempo_rampa`) para que conexões abertas em rampas longas permaneçam abertas e ativas durante toda a janela de medição estável.

2. **Automação de Carga e Estresse ([`scripts/teste_carga.py`](/mnt/d/git/lukeboh/urna-velox/scripts/teste_carga.py) / [`scripts/teste_carga.sh`](/mnt/d/git/lukeboh/urna-velox/scripts/teste_carga.sh)):**
   - Suporta execução nos modos `--local` (orquestrando `uv-simulador` com dados reais de 2026 a 60× + `uv-servico`) e `--producao` (atingindo `urnavelox.com.br`).
   - Mede: conexões abertas, falhas de handshake/conexão, latência BU → Cliente (p50/p99/máx), latência Servidor → Cliente (p50/p99), vazão em mensagens/s e MB/s, e consumo de memória (RSS) do processo servidor.

---

## 3. Resultados Obtidos

### 3.1. Teste Local (WSL, 1º Turno 2026 acelerado a 60×, loopback)

| Usuários Simultâneos | Conexões Abertas | Falhas | BU→Cli p50 | BU→Cli p99 | Servidor→Cli p99 | Vazão (msgs/s) | Banda (MB/s) | RAM RSS Servidor | SLA (< 50ms) |
|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|
| **1.000** | 1.000 | 0 | 2,54 ms | **6,34 ms** | 3,26 ms | 37.106 msgs/s | 45,1 MB/s | 87,9 MB | 🟢 Excelente |
| **3.000** | 3.000 | 0 | 4,98 ms | **28,17 ms** | 26,86 ms | 136.269 msgs/s | 166,6 MB/s | 494,3 MB | 🟢 Excelente |
| **5.000** | 5.000 | 0 | 6,40 ms | **20,76 ms** | 14,39 ms | 291.343 msgs/s | 353,5 MB/s | 783,2 MB | 🟢 Excelente |
| **8.000** | 8.000 | 0 | 3,12 ms | **118,08 ms** | 114,79 ms | 288.965 msgs/s | 345,3 MB/s | 1.252,5 MB | 🟠 Degradado |
| **10.000** | 10.000 | 0 | 73,80 ms | **246,06 ms** | 203,97 ms | 331.053 msgs/s | 396,2 MB/s | 1.563,5 MB | 🟠 Degradado |
| **12.000** | 10.228 | 1.772 | 48,80 ms | **74,77 ms** | 36,05 ms | 241.534 msgs/s | 292,5 MB/s | — | 🔴 Limite de Sockets |

*Nota da revisão (20/10, 19h):* o limite dos 12.000 é do **gerador de carga**, não do servidor: o `ulimit -n` do WSL é
10.240 e as conexões pararam em 10.228. No local, simulador a 60×, `uv-servico` e `uv-carga` disputam a mesma máquina,
o que explica a degradação a partir de 8.000.

### 3.2. Teste em Produção (`urnavelox.com.br`, Cluster OCI A1 + Caddy TLS)

| Usuários Simultâneos | Conexões Abertas | Falhas | BU→Cli p99 | Servidor→Cli p99 | Vazão (msgs/s) | Volume | Avaliação |
|:---:|:---:|:---:|:---:|:---:|:---:|:---:|
| **500** | 500 | 0 | 0,00 ms | 0,00 ms | 159 msgs/s | 0,08 MB/s | 🟢 Estável / SLA Atendido |
| **1.000** | 1.000 | 0 | 51,56 ms | 6,61 ms | 309 msgs/s | 0,16 MB/s | 🟢 Estável / SLA Atendido |
| **2.000** | 2.000 | 0 | 0,00 ms | 0,00 ms | 585 msgs/s | 0,31 MB/s | 🟢 Estável / SLA Atendido |
| **3.000** | 3.000 | 0 | 14,04 ms | 8,91 ms | 857 msgs/s | 0,46 MB/s | 🟢 Estável / SLA Atendido |
| **5.000** | 5.000 | 0 | 19,66 ms | 7,14 ms | 1.281 msgs/s | 0,69 MB/s | 🟢 Estável / SLA Atendido |
| **7.500** | 7.500 | 0 | 132,32 ms | 21,84 ms | 1.683 msgs/s | 0,91 MB/s | 🟡 Estável (Cauda p99 WAN) |
| **10.000** | **10.000** | **0** | **22,56 ms** | **16,14 ms** | **1.010 msgs/s** | **0,55 MB/s** | 🟢 **Capacidade Máxima Estável** |
| **12.000** | 10.543 | **1.457** | 33,98 ms | 6,40 ms | 1.038 msgs/s | 0,57 MB/s | 🔴 **Ponto de Quebra (HTTP 503)** |

*Nota da revisão (20/10, 19h):* "0,00 ms" significa **nenhuma amostra** (`n: 0`), não latência excelente; no degrau de
12.000 houve 37 amostras, então o "p99" é o máximo de 37 valores (o mesmo vale para os 132 ms de 7.500). As latências
desta tabela não servem para afirmar SLA. O que a tabela mostra com segurança: conexões abertas, falhas e o motivo das
falhas. O gerador de carga é uma máquina só (o WSL do Luciano, um IP de saída).

---

## 4. Diagnóstico Detalhado do Ponto de Quebra

No degrau de 12.000 conexões contra produção:
1. **Conexões atendidas com sucesso:** **10.543 clientes simultâneos** mantiveram streaming contínuo sem quedas, com latência Servidor→Cliente p99 de apenas **6,40 ms**.
2. **Causa exata das 1.457 falhas:**
   - Erro registrado no cliente: `falha_conexao: HTTP error: 503 Service Unavailable: 1457`.
   - Diagnóstico: Cada nó (`uv-no-a` e `uv-no-b`) está configurado por padrão com `--max-clientes 5000` (+256 conexões TCP de margem). O Caddy balanceou a carga 50/50 entre os dois nós via rede privada (10.0.1.10 e 10.0.1.11). Quando a soma de conexões atingiu **10.543 conexões**, o disjuntor de aplicação do `uv-servico` entrou em ação e **recusou as conexões excedentes com HTTP 503 no handshake**, preservando a estabilidade total das 10.543 conexões ativas!
3. **Comportamento do Motor e do Serviço:**
   - O `uv-servico` não caiu, não teve vazamento de memória e seguiu apurando os BUs em tempo real normalmente (passou de seq 445.281 para seq 472.949 durante a bateria).
   - O painel em `https://urnavelox.com.br/painel/` refletiu os picos de conexões e vazão em tempo real.

---

## 5. Subsídios e Propostas para a Sessão de Refatoração Arquitetural

A arquitetura atual sustenta com folga **10.000 usuários simultâneos** por cluster (5.000 por nó). Caso o objetivo seja escalar para **50.000, 100.000 ou 500.000 usuários simultâneos**, a sessão de refatoração deve considerar os seguintes pontos críticos:

### A. O Gargalo do Modelo "WebSocket com Conexão Persistente Direta"
* Cada conexão WebSocket mantém um socket TCP aberto, buffers de envio/recepção no kernel (~32–64 KB cada) e estruturas de estado em memória (~125–150 KB por cliente).
* Em 50.000 conexões, apenas o estado de conexões exigiria ~7,5 GB de RAM e dezenas de milhares de descritores no nó, além de milhares de interrupções de CPU para gerenciar frames WebSocket.
* **Alternativa / Refatoração (ADR-021):**
  - Mudar o modelo do usuário comum no navegador de **WebSocket direto no nó** para **REST / arquivos estáticos com cache agressivo em CDN / borda** (Cloudflare, Fastly, CloudFront ou Caddy em borda independente).
  - Como os dados mudam em blocos (a cada poucos segundos), a CDN pode cachear `/api/v1/resultado/br-c0001` com `max-age=1, stale-while-revalidate=2` e `ETag`. O nó do Urna Velox responde apenas **1 requisição por segundo para a CDN**, e a CDN atende centenas de milhares de usuários.
  - Reservar o **WebSocket** (`/ws`) exclusivamente para integradores e telas de alta prioridade.

### B. Ajustes no Caddy e no Kernel Linux dos Nós da OCI
Para que os nós da OCI absorvam rajadas sem entrar em `SYN-SENT` (descarte de SYN):
1. **Backlog TCP no `/etc/sysctl.conf`:**
   ```sysctl
   net.core.somaxconn = 65535
   net.ipv4.tcp_max_syn_backlog = 65535
   net.ipv4.tcp_syncookies = 1
   net.ipv4.tcp_tw_reuse = 1
   net.ipv4.ip_local_port_range = 1024 65535
   ```
2. **Descritores nos serviços systemd (`caddy.service` e `urna-velox.service`):**
   ```ini
   LimitNOFILE=1048576
   LimitNPROC=512000
   ```
3. **Tuning do Caddy:**
   - Adicionar limites explícitos de buffer de streaming e ajustar `read_timeout` / `write_timeout` nas diretivas `reverse_proxy`.

### C. Isolamento de Threads HTTP vs. Motor
* Nos testes locais acima de 8.000 clientes, observou-se contenção de CPU entre o runtime HTTP do `axum` e o motor de totalização na mesma máquina.
* No `uv-servico`, o runtime HTTP roda em `--http-threads` (padrão 4).
* Recomenda-se fixar afinidade de CPU (`taskset` ou configuração de thread pool): deixar 2 ou 4 núcleos Ampere dedicados à difusão/Caddy e os demais núcleos dedicados exclusivamente ao leitor de BU, motor e gravação.

---

## 6. Arquivos e Comandos para Reproduzir

1. **Repetir o teste local:**
   ```bash
   bash /mnt/d/git/lukeboh/urna-velox/scripts/teste_carga.sh --local --degraus "1000,3000,5000,8000,10000" --duracao 15
   ```

2. **Repetir o teste em produção:**
   ```bash
   bash /mnt/d/git/lukeboh/urna-velox/scripts/teste_carga.sh --producao --url urnavelox.com.br --degraus "1000,3000,5000,7500,10000,12000" --duracao 15
   ```

3. **Conferência de saúde do serviço após testes:**
   ```bash
   curl -s https://urnavelox.com.br/api/v1/estado | python3 -m json.tool
   ```

---

## 7. Revisão de 20/10, 19h: o que vale para o 2º turno

*Claude, com o código (`aquisicao/src/difusao.rs`, `site/src/conexao.js`, `deploy/oci/`), os JSONs das duas rodadas
de produção e o painel dos nós do teste das 18h21 (janela de 15 min).*

### 7.1. O que o painel dos nós mostrou (2 OCPUs e 12 GB por nó, `deploy/oci/00_criar_oci.sh`)

| Medida | Repouso | Pico (~5.000 clientes por nó) | Leitura |
|---|---|---|---|
| Clientes WebSocket por nó | 0 | ~5.000 (os dois nós iguais) | o corte é o `max_clientes` (5.000, padrão de `difusao.rs:71`) |
| Memória do `uv-servico` (RSS) | ~420 MB | ~1.080 MB | **~130 KB por conexão**; ~9 % da máquina; volta ao repouso quando os clientes saem (sem vazamento) |
| CPU da máquina | < 5 % | **40–50 %** (picos junto das subidas de conexões) | **é o recurso que limita**; os 2 núcleos são os mesmos do motor, da gravação e do Caddy (TLS) |
| BUs aplicados/s, federação | ~10–35 BUs/s em rajadas | sem mudança | a totalização não sentiu a carga |

### 7.2. Correções às conclusões das §§ 3–5

1. **"Capacidade máxima de 10.000" é configuração, não medida.** 2 nós × `max_clientes 5000`. Não se mediu até onde
   a máquina vai. As 10.543 conexões (acima de 10.000) não estão explicadas: o limite TCP é `max_clientes + 256` por
   nó e o de WebSocket é `max_clientes`; vale conferir o `no.env` dos nós.
2. **A memória tem folga; a CPU não.** Os 40–50 % de CPU com 5.000 clientes por nó foram medidos com um replay de
   ritmo baixo (~10–35 BUs/s). No pico de uma noite real chegam bem mais BUs por segundo, e cada atualização vira
   mensagens para todos os clientes inscritos. Dobrar os clientes por nó com os 2 OCPUs atuais tende a saturar a
   máquina. Não convém subir o `max_clientes` além de ~7–8 mil por nó sem um novo teste olhando a CPU.
3. **Latência:** ver as notas das tabelas (0 a 37 amostras). E 50 ms não é a meta certa para o público: abaixo de
   1–2 s ninguém percebe. A meta útil é "todo mundo que abre a página vê a apuração".
4. **§5.B (kernel e Caddy):** nenhum descarte de SYN foi observado. As falhas foram 503 da aplicação (produção) e
   `ulimit` do gerador de carga (local). `tcp_tw_reuse` e `ip_local_port_range` só afetam conexões que **saem** do nó.
   Subir o `LimitNOFILE` dos serviços não faz mal (conferir o valor atual), mas não é o gargalo.
5. **§5.C (fixar núcleos):** a disputa de CPU vista no teste local vem de simulador, servidor e gerador estarem na
   mesma máquina. O `uv-servico` já baixa a prioridade das threads HTTP (`prioridade_baixa`, nice 10). Não é para agora.
6. **Balanceamento:** o texto fala em `least_conn`; o `02_configurar_borda.sh` descreve 50/50 e o DNS aponta para os
   dois IPs. Isso pesa na consistência (o "voltar no tempo"), não na carga; está no outro handoff.

### 7.3. O risco real de domingo (não estava no relatório)

**Quem passa do limite não vê nada.** O `site/src/conexao.js` não tem caminho alternativo: com o 503 do `/ws`, a
página só tenta reconectar (1, 2, 4… até 30 s) e a apuração não aparece. Registrado como **B-026** em
[BUGS.md](../BUGS.md) (só o sintoma; ordem de ataque com o Luciano).

O servidor já tem o que um caminho alternativo precisa: `GET /api/v1/resultado/{topico}` com `ETag`, `304` e
`Cache-Control: public, max-age=1` (`difusao.rs:648`).

### 7.4. Capacidade estimada para domingo (a medir, não medida)

| Cenário | Quem vê a apuração |
|---|---|
| Hoje (2 × 5.000, sem alternativa ao WebSocket) | ~10 mil simultâneos em tempo real; **acima disso, ninguém** |
| + consulta HTTP periódica na página (intervalo longo, `ETag`) | os mesmos ~10 mil em tempo real; o excedente com alguns segundos de atraso, até a CPU dos nós (provavelmente dezenas de milhares, a medir) |
| + mais CPU nos nós (A1 flexível, 4–8 OCPUs, pago) | sobe o teto das duas linhas acima, mais ou menos em proporção aos núcleos |
| + cache na borda (Cloudflare ou equivalente) na frente do HTTP | o excedente deixa de depender dos nós (~1 req/s por tópico chegando à origem) |

**Metas fixadas pelo Luciano (20/10, 19h20):** 10.000 WebSocket em tempo real e 100.000 por HTTP com até 2–3 min de
atraso; decididos mais CPU nos dois nós e a Cloudflare só no subdomínio `dados.urnavelox.com.br`. Plano em
[HANDOFF_MODELO_BORDA_E_CONSISTENCIA.md](HANDOFF_MODELO_BORDA_E_CONSISTENCIA.md) (§0 e §3).

### 7.5. Próximos testes de carga (para que meçam o que importa)

* **Gerador em 2 ou mais máquinas** (ou `ulimit -n` alto e mais de um IP de saída): uma máquina só limita o teste.
* **Medir os nós**, não só o cliente: CPU e RSS do `uv-servico` e do Caddy por degrau (o painel já mostra; guardar
  junto do relatório).
* **Subida rápida de propósito:** milhares de conexões em poucos segundos (a TV cita o site; um nó reinicia e os
  clientes dele reconectam juntos).
* **Replay com ritmo de pico** (BUs/s de uma noite real, não ~10–35/s), para a CPU refletir domingo.
* **Latência com amostras suficientes** (centenas ou mais por degrau) ou, melhor, a meta "toda página recebe
  atualização em até N s".
* Depois do caminho HTTP pronto: saturar de propósito acima do `max_clientes` e confirmar que **toda** página
  continua mostrando a apuração.
