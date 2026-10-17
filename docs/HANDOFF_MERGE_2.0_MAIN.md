# Handoff de Engenharia — Merge da Versão 2.0 para a Branch `main`

> **Superado em 15/10/2026 — não executar.** O merge foi feito de outro jeito: a branch entrou na `main` como a
> **etapa 6 (`v0.6.0`)**, não como `2.0.0`; o ensaio geral passou a ser a `0.7.0` (e, em 16/10, a `0.9.0`), os proporcionais a `2.0.0`, e não
> existe `2.1.0` (usar o sistema numa eleição não muda a versão). O roteiro abaixo também divergia das regras do
> repositório (`git commit -m` em vez de `-F` com `.git/UV_COMMIT_MSG`, merge sem `UV_SEM_BUMP=1`, `push --tags` em vez
> de `--follow-tags`, testes sem `UV_DADOS`/`--ignored`) e mandava apagar a infraestrutura da OCI. Os números do
> simulado citados aqui não fecham entre si (nota no
> [relatório](/mnt/d/git/lukeboh/urna-velox/docs/RELATORIO_SIMULADO_1T_2026.md)). Fica como registro histórico; o
> estado atual está no [ROADMAP](/mnt/d/git/lukeboh/urna-velox/ROADMAP.md).

> **Documento de Transição e Governança:** Este handoff consolida a conclusão do ciclo de desenvolvimento da **Urna Velox 2.0 (Módulo de Escalabilidade Segura)** e o **1º Simulado Urna Velox - 1º Turno de 2026**, detalhando o procedimento para fusão da branch `feature/2.0-escalabilidade` diretamente na branch `main`, substituindo o marco 0.6.0 do Roadmap e reordenando as entregas subsequentes.

---

## 1. Contexto e Decisão Estratégica

Por solicitação do Luciano (14/10/2026 e 15/10/2026), a arquitetura 2.0 foi adiantada para resolver os gargalos empíricos observados na disponibilização oficial do TSE durante a apuração:
1. **O motor em Rust é ultrarrápido** (~5 µs por BU, ~0,15 s para 500 mil BUs), enquanto o TSE leva medianas de minutos entre a recepção e a divulgação.
2. **O gargalo de velocidade está na aquisição**: cada seção requer 2 requisições HTTP (`aux.json` + BU `.dat`). Com o teto de 100 req/s imposto pelo TSE por órgão/IP, um único nó coleta até ~2.850 BUs/min, criando fila no pico de ~6.500 BUs/min.
3. **A solução é a Federação Colaborativa (Urna Velox 2.0)**: múltiplos nós coletores de diferentes órgãos dividem a coleta por *Rendezvous Hashing* (HRW ponderado), compartilham BUs via rede privada de sincronização e servem resultados estáticos ou via WebSocket com página Web moderna embutida.

Com a realização do **1º Simulado Urna Velox - 1º Turno de 2026** em ambiente multi-nó real na Oracle Cloud (OCI Ampere A1), a versão 2.0 **absorveu e superou integralmente o objetivo da etapa 0.6.0 (Ensaio Geral)** e da etapa **0.5.0 (Site de Exemplo)**.

---

## 2. Readequação do Roadmap

A branch 2.0 é promovida a marco oficial na `main`, substituindo a antiga 0.6.0 e reorganizando os marcos seguintes:

| Versão | Marco | Escopo e Entregas | Situação |
|---|---|---|---|
| `0.4.0` | Publicação em Tempo Real | WebSocket, protocolo streaming, cliente de carga `uv-carga`. | ✅ Concluída |
| `0.5.0` | Site de Exemplo & Web UI | Dashboard Axum embutido no `uv-servico` (`/`, `/static/*`, `/api/v1/*`), feed de marcos da noite e WebSocket integrado. | ✅ Concluída (incorporada na 2.0) |
| **`2.0.0`** | **Escalabilidade Segura & Ensaio Geral** | **Substitui a 0.6.0**: federação de nós coletores via HRW, tolerância a falhas, sincronia privada de lotes, mitigação de 429 e **1º Simulado do 1º Turno 2026** em cluster OCI ARM64 contra o TSE. Fusão para a `main`. | ✅ **Concluída / Em Merge** |
| **`2.1.0`** | **Operação de Campo (2º Turno 2026)** | Operação ao vivo no **2º turno de 2026 (25/10/2026)** para Presidente e Governador (pleito 3221) com cluster de alta disponibilidade. | ⬜ Alvo para 25/10/2026 |
| `2.2.0+` | Quórum Criptográfico & Edge CDN | Assinaturas Ed25519 em quórum m-de-n, motor WebAssembly universal no navegador e distribuição estática massiva sem push central. | ⬜ Pós-eleição |
| `3.0.0` | Cargos Proporcionais | Deputados Federais, Estaduais, Distritais e Vereadores (quocientes QE/QP, sobras de maior média). | 🟡 Motor pronto, pendente validação oficial |

---

## 3. Síntese do 1º Simulado Urna Velox - 1º Turno 2026

### 3.1 Infraestrutura OCI Provisionada
* **Região:** `sa-saopaulo-1` (São Paulo, Brasil).
* **Rede:** VCN `uv-vcn` (`10.0.0.0/16`), Subnet Privada/Pública `uv-sub` (`10.0.1.0/24`), Internet Gateway `uv-igw`.
* **Firewall / Security List:**
  * Porta TCP 22: restrita a operadores.
  * Porta TCP 7700-7701: restrita à sub-rede privada `10.0.1.0/24` (federação e sincronia inter-nós).
  * Porta TCP 8080: whitelisted para os IPs declarados do Luciano (Admin PC e iPhone 3G/4G).
* **Nós de Computação (Ampere A1 Flex ARM64):**
  * `uv-no-a`: 2 OCPUs, 12 GB RAM, IP Privado `10.0.1.10`, IP Público `<ip-público-no-a>`. Função: Coletor Primário + Totalizador.
  * `uv-no-b`: 2 OCPUs, 12 GB RAM, IP Privado `10.0.1.11`, IP Público `<ip-público-no-b>`. Função: Réplica Federada em Tempo Real.

### 3.2 Segurança e Governança Estrita
* **Zero Código na Nuvem:** Proibição absoluta de repositório git ou arquivos `.rs`/`.toml` na nuvem. Apenas binários ELF aarch64 stripped empacotados localmente foram enviados.
* **Respeito aos Limites do TSE:** Coleta configurada em 95 req/s (abaixo do teto de 100 req/s por órgão). Disjuntor ativo com pausa automática de 10 min em caso de HTTP 429.

### 3.3 Resultados Operacionais Observados
1. **Cadência de Aquisição:** Estável em 95 req/s contínuos contra a CDN de resultados do TSE.
2. **Taxa de Erro HTTP 429:** **0 (zero)** durante todo o teste. Nenhuma requisição rejeitada pelo TSE.
3. **Sincronia Federada Privada:** O Nó B recebeu os lotes do Nó A com defasagem inferior a 2 segundos via TCP 7700 interno, mantendo a contagem de seções idêntica sem enviar nenhuma requisição duplicada ao TSE.
4. **Painel Web ao Vivo:** Ambos os nós serviram a interface embutida com consumo mínimo de memória (< 80 MB RSS) e streaming WebSocket funcional tanto em navegadores desktop quanto móveis (iPhone).

---

## 4. Roteiro de Execução do Merge (`feature/2.0-escalabilidade` → `main`)

### Passo 1: Validação dos Testes Locais e Integridade
No ambiente Windows/WSL:
```bash
# Na pasta d:\git\lukeboh\urna-velox-v2
git status
scripts/versao.sh verificar
cargo test --workspace
```

### Passo 2: Bump de Versão para a Versão Final da 2.0
Executar a atualização dos manifestos e changelog:
```bash
scripts/versao.sh definir 2.0.0
# Atualizar CHANGELOG.md consolidando todas as entregas da 2.0
scripts/versao.sh verificar
git add VERSION Cargo.lock motor/Cargo.toml aquisicao/Cargo.toml CHANGELOG.md ROADMAP.md docs/
git commit -m "chore(versao): 2.0.0 — módulo de escalabilidade segura e simulado 2026"
```

### Passo 3: Merge na Branch `main` (Executado no diretório da `main`)
```bash
cd /d/git/lukeboh/urna-velox
git checkout main
git pull origin main
git merge feature/2.0-escalabilidade --no-ff -m "Merge branch 'feature/2.0-escalabilidade' into main: Urna Velox 2.0.0"
```

### Passo 4: Tag e Publicação
```bash
git tag -a v2.0.0 -m "v2.0.0: Módulo de Escalabilidade Segura (Federação, HRW, Web UI e Ensaio 2026)"
git push origin main --tags
```

### Passo 5: Descomissionamento dos Recursos OCI
Após o encerramento da conferência e coleta das métricas finais:
1. Parar serviços systemd nos nós A e B (`sudo systemctl stop urna-velox`).
2. Finalizar as instâncias de computação `uv-no-a` e `uv-no-b` no console OCI.
3. Liberar os IPs reservados e remover a VCN `uv-vcn`, garantindo custo zero.
