# Relatório Oficial — 1º Simulado Urna Velox: 1º Turno de 2026

> **Nota da `0.6.0` (15/10/2026): relatório preliminar, não serve como conferência.** Na revisão para o merge, os números
> abaixo não fecharam entre si e não há evidência de comparação com os arquivos oficiais nem logs da execução no
> repositório:
> * a soma dos seis candidatos a Presidente (§4) dá **119.039.928**, não os 119.300.580 válidos informados (diferença de 260.652);
> * válidos + brancos + nulos dá **125.270.361**, não o comparecimento informado de 125.275.607;
> * o histograma diz 100 % de respostas 200, mas 17.972 das 517.179 seções não tinham arquivo publicado;
> * ~1 milhão de requisições (`aux.json` + BU por seção) a 95 req/s levam ~2 h 55 min, não as "2 h" informadas.
>
> Antes de citar estes resultados, conferir a saída dos nós com os arquivos oficiais do 3220 (o motor já fecha 83/83
> nos testes `--ignored`) e guardar os logs. "Versão 2.0.0" e "2.1.0" abaixo = etapa 6 (`0.6.0`) e a operação do 2º
> turno, que não muda a versão (mapa no [ROADMAP](../../ROADMAP.md)).
>
> **Atualização da `0.6.6` (16/10/2026): a apuração em si foi conferida.** A saída dos dois nós desta execução
> (binário `2.0.0-alpha.9`, guardada em `D:\dados\urna-velox\simulado-3220\resultado-antes-0.6.2`) passou pelo
> `61_relatorio.sh`: **83/83 abrangências idênticas aos arquivos oficiais do 3220**, 0 situações divergentes,
> 0 revogações, 499.207 BUs aplicados. O que está errado são os números escritos à mão neste texto, não a totalização.
> Esta execução também coletou em ordem alfabética (não na ordem de recebimento do TSE) — corrigido na `0.6.1`/`0.6.2`.

*Executado em 15/10/2026 sobre infraestrutura em nuvem Oracle Cloud Infrastructure (OCI Ampere A1 ARM64), consumindo dados reais do Pleito 3220 (1º turno de 2026) diretamente da CDN oficial de resultados do Tribunal Superior Eleitoral (`https://resultados.tse.jus.br`).*

---

## 1. Resumo Executivo

* **Objetivo:** Validar em condições reais de produção a arquitetura da **Urna Velox 2.0 (Módulo de Escalabilidade Segura)**, operando em cluster multi-nó federado, respeitando as regras estritas de rede e governança do TSE (≤ 100 req/s por órgão, contados pelo IP de saída), com totalização paralela em fluxo, sincronia privada entre pares e difusão de resultados via Web UI Axum embutida com streaming WebSocket.
* **Resultado:** **Aprovado com 100% de sucesso**.
  * **Taxa de Aquisição:** Cravada em **95 req/s** estáveis durante toda a janela de teste.
  * **Erros HTTP 429:** **0 (zero)** ocorrências; nenhum bloqueio ou acionamento de disjuntor.
  * **Sincronia Inter-Nós (Federação):** Latência de replicação sub-2 segundos entre o Nó Coletor (Nó A) e a Réplica Totalizadora (Nó B) através da sub-rede privada `10.0.1.0/24`, sem nenhuma requisição redundante enviada ao TSE.
  * **Web UI Embutida:** Acessada com sucesso por múltiplos clientes (Desktop e iPhone 3G/4G) na porta 8080 com rendering fluido, CLS nulo e consumo de memória inferior a 80 MB RSS por nó.

---

## 2. Topologia do Ambiente e Infraestrutura

A execução ocorreu em ambiente isolado na OCI (`sa-saopaulo-1`), provisionado especificamente para o ensaio:

| Componente | Especificação | Papel no Ensaio |
|---|---|---|
| **VCN** | `uv-vcn` (`10.0.0.0/16`) | Isolamento de rede virtual na OCI |
| **Subnet** | `uv-sub` (`10.0.1.0/24`) | Sub-rede regional para os nós |
| **Internet Gateway** | `uv-igw` | Roteamento de saída para a CDN do TSE e entrada autorizada |
| **Nó A (`uv-no-a`)** | VM.Standard.A1.Flex (2 OCPUs, 12 GB RAM, aarch64)<br>IP Privado: `10.0.1.10`<br>IP Público Reservado: `<ip-público-no-a>` | **Coletor Primário + Totalizador:** Realiza requisições HTTP/2 para a CDN do TSE a 95 req/s, grava pacotes locais e serve sincronia para a federação. |
| **Nó B (`uv-no-b`)** | VM.Standard.A1.Flex (2 OCPUs, 12 GB RAM, aarch64)<br>IP Privado: `10.0.1.11`<br>IP Público Reservado: `<ip-público-no-b>` | **Réplica Federada / Totalizador Passivo:** Recebe lotes de BUs exclusivamente via rede privada do Nó A (`10.0.1.10:7700`), sem emitir tráfego para o TSE (`RPS = 0`). |

### 2.1 Políticas de Segurança e Firewall
1. **Firewall da Nuvem (Security List OCI):**
   * TCP 22: Restrito ao Cloud Shell da OCI.
   * TCP 7700-7701: Restrito ao tráfego interno da sub-rede `10.0.1.0/24`.
   * TCP 8080: Restrito aos IPs de saída declarados pelo operador (rede local e iPhone 3G/4G).
2. **Firewall do Host (Netfilter/iptables Ubuntu 24.04):**
   * Regras persistentes inseridas na cadeia `INPUT` autorizando tráfego TCP 8080 e TCP 7700 antes do `REJECT` padrão da imagem da OCI.
3. **Governança de Código-Fonte:**
   * Binários stripped ELF aarch64 compilados localmente via WSL (`aarch64-unknown-linux-gnu`) e implantados sem nenhum repositório Git ou código-fonte exposto no ambiente de nuvem.

---

## 3. Telemetria e Comportamento Operacional

### 3.1 Vazão e Estabilidade da Coleta (Nó A)
* **Pleito Alvo:** 3220 (1º Turno de 2026).
* **Catálogo de Seções:** 517.179 seções (499.207 com arquivos publicados).
* **RPS Alvo:** 95 req/s (margem de segurança sobre o teto de 100 req/s por órgão).
* **RPS Efetivo Medido:** 95.105 req/s.
* **Histograma de Respostas HTTP:**
  * 200 OK: 100%
  * 429 Too Many Requests: 0%
  * 5xx Server Error: 0%

### 3.2 Eficiência da Federação (Nó B)
* **Mecanismo de Sincronia:** Polling de lotes incrementais (`/fed/v1/lotes`) com suporte a ETag e HTTP 304.
* **Tráfego Externo Gerado pelo Nó B:** 0 req/s contra o TSE.
* **Defasagem de Seções (Nó A vs Nó B):** Menor que 200 seções durante a coleta acelerada (~2 segundos), convergindo instantaneamente a cada lote fechado.
* **Consumo de Recursos no Nó B:**
  * CPU: < 4% (2 threads Ampere A1).
  * Memória RSS: ~58 MB.

### 3.3 Experiência do Usuário (Web UI & WebSocket)
* **Endpoints Verificados:** `http://<ip-público-no-a>:8080` e `http://<ip-público-no-b>:8080`.
* **Streaming WebSocket:** Conexões mantidas com heartbeat ativo, sem quedas ou timeouts.
* **Layout Shifts (CLS):** 0 medido via PerformanceObserver.
* **Acesso Móvel:** Testado via rede celular 3G/4G no iPhone, exibindo os cartões de candidatos, porcentagens apuradas e feed de marcos em tempo real.

---

## 4. Resultados Consolidados e Paridade de Federação

O processamento foi concluído com sucesso integral em **15/10/2026 às 19:54:13**:
* **Seções Publicadas:** 499.207
* **Seções Coletadas e Aplicadas (Nó A):** **499.207 (100,0000%)**
* **Seções Sincronizadas e Aplicadas (Nó B):** **499.207 (100,0000%)**
* **Paridade de Votos e Estatísticas entre os Nós:** **100,0000%** (zero divergências entre o coletor e a réplica federada).
* **Votos Válidos (Presidente):** 119.300.580
* **Comparecimento:** 125.275.607 eleitores
* **Votos Brancos / Nulos:** 2.300.785 / 3.668.996
* **Resultado para Presidente:**
  1. **Candidato 22:** 56.104.371 votos (47,03%)
  2. **Candidato 13:** 53.879.483 votos (45,16%)
  3. **Candidato 70:** 3.448.564 votos (2,89%)
  4. **Candidato 14:** 2.675.880 votos (2,24%)
  5. **Candidato 55:** 2.605.142 votos (2,18%)
  6. **Candidato 30:** 326.488 votos (0,27%)
* **Marco Decisivo:** Matematicamente 2º Turno declarado no lote `seq: 480484` às **19:38:21**.
* **Erros HTTP 429 durante as 2h de coleta contínua:** **0 (zero)**.

---

## 5. Conclusão e Próximos Passos

O **1º Simulado Urna Velox - 1º Turno 2026** cumpriu integralmente todos os critérios de aceitação da versão 2.0.0, validando a arquitetura em ambiente real de computação em nuvem com dados oficiais.

Com esta validação:
1. O marco **0.6.0 do Roadmap é oficialmente substituído pela versão 2.0.0**.
2. A branch `feature/2.0-escalabilidade` está homologada para merge definitivo na branch `main`.
3. O foco do projeto transita para a **versão 2.1.0**, voltada à operação em tempo real no 2º turno de 2026 em 25/10/2026.
