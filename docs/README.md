# Documentação do Urna Velox

Índice de tudo o que está em `docs/`. **Regras de organização** no fim desta página (conferidas no pre-commit por
`python3 scripts/docs.py verificar`).

* **[BUGS.md](BUGS.md)** — bugs (`B-nnn`) e melhorias (`M-nnn`), cada um com destino no [ROADMAP](../ROADMAP.md);
  o detalhe de cada item em [itens/](itens/) (imagens em `itens/img/`).
* Imagens dos documentos: [img/](img/).

## 🏗️ Arquitetura e formatos — `arquitetura/`

Como o sistema é feito: decisões (ADRs), aquisição, escalabilidade, protocolo do WebSocket, formatos de arquivo, glossário.

* [AQUISICAO.md](arquitetura/AQUISICAO.md) — Fase de aquisição: como o Urna Velox obtém os BUs
* [ARCHITECTURE_DECISIONS.md](arquitetura/ARCHITECTURE_DECISIONS.md) — Registro de Decisões de Arquitetura (ADR) — Urna Velox
* [ESCALABILIDADE_SEGURA.md](arquitetura/ESCALABILIDADE_SEGURA.md) — 🌐 Módulo de Escalabilidade Segura: Federação Colaborativa e Distribuição Massiva
* [ESTIMATIVAS_MEMORIA_E_TEMPO.md](arquitetura/ESTIMATIVAS_MEMORIA_E_TEMPO.md) — Estimativas de Memória e Tempo de Totalização
* [FORMATO_CANDIDATOS.md](arquitetura/FORMATO_CANDIDATOS.md) — Formato do arquivo `candidatos.json` (Versão 1)
* [FORMATO_RESULTADO.md](arquitetura/FORMATO_RESULTADO.md) — Formato do resultado (versão 1)
* [GLOSSARIO.md](arquitetura/GLOSSARIO.md) — Glossário
* [PROTOCOLO_WS.md](arquitetura/PROTOCOLO_WS.md) — Protocolo de difusão em tempo real — WebSocket e REST (versão 1)
* [SONDA.md](arquitetura/SONDA.md) — Sondas — o padrão *Test Message* no Urna Velox

## 🛠️ Operação — `operacao/`

Como operar: serviço da noite, ciclo diário de replays, painel dos nós, borda (Caddy/Cloudflare), checklists.

* [CHECKLIST_CLOUDFLARE.md](operacao/CHECKLIST_CLOUDFLARE.md) — Checklist — subdomínio `dados` na Cloudflare (passo 3 do plano de borda)
* [CHECKPOINT_BORDA.md](operacao/CHECKPOINT_BORDA.md) — Checkpoint — plano de borda do 2º turno (para retomar de outra sessão)
* [CICLO_DIARIO.md](operacao/CICLO_DIARIO.md) — Ciclo diário de simulações com dados reais (18 a 23/10/2026)
* [GUIA_URL_E_BALANCEAMENTO.md](operacao/GUIA_URL_E_BALANCEAMENTO.md) — Guia de Configuração: URL Pública e Balanceamento para o Urna Velox
* [OPERACAO.md](operacao/OPERACAO.md) — Operação da noite — `uv-servico` (etapa 3)
* [PAINEL.md](operacao/PAINEL.md) — Painel dos nós (`uv-painel`)

## 📐 Processo — `processo/`

Regras de trabalho do projeto (versionamento).

* [VERSIONAMENTO.md](processo/VERSIONAMENTO.md) — Versionamento

## ⚖️ Legal — `legal/`

Licença explicada e privacidade (também publicados no site, em /termos e /privacidade).

* [LICENCIAMENTO.md](legal/LICENCIAMENTO.md) — Modelo de Licenciamento: Urna Velox
* [PRIVACIDADE.md](legal/PRIVACIDADE.md) — Privacidade, acesso e retenção de dados: Urna Velox

## 📊 Relatórios e análises — `relatorios/`

Resultados medidos: relatórios de etapa, de eleição, de simulados, de capacidade, análises de incidente.

* [ANALISE_ATRASO_2026-10-25.md](relatorios/ANALISE_ATRASO_2026-10-25.md) — Por que o Urna Velox ficou atrás do TSE em 25/10/2026 (2º turno)
* [RELATORIO_0.2.0.md](relatorios/RELATORIO_0.2.0.md) — Relatório da Etapa 2 — Catálogo e Candidatos Pré-Eleição (Preparação para 0.2.0)
* [RELATORIO_0.3.0.md](relatorios/RELATORIO_0.3.0.md) — Relatório da Etapa 3 — Operação automatizada (preparação para `0.3.0`)
* [RELATORIO_0.4.0.md](relatorios/RELATORIO_0.4.0.md) — Relatório da Etapa 4 — Publicação em tempo real (preparação para `0.4.0`)
* [RELATORIO_0.5.0.md](relatorios/RELATORIO_0.5.0.md) — Relatório da etapa 5 — site de exemplo (página de resultados embutida)
* [RELATORIO_0.7.0.md](relatorios/RELATORIO_0.7.0.md) — Relatório da etapa 7 — comparação Urna Velox × TSE (`0.7.0`)
* [RELATORIO_0.8.0.md](relatorios/RELATORIO_0.8.0.md) — Relatório da etapa 8 — previsão do matematicamente e do fim da apuração (`0.8.0`)
* [RELATORIO_DEPLOY_SEM_SSH.md](relatorios/RELATORIO_DEPLOY_SEM_SSH.md) — Relatório — Implantação e Operação nos Nós da OCI sem a Porta 22 e sem o Portal
* [RELATORIO_ELEICAO_2026-10-25.md](relatorios/RELATORIO_ELEICAO_2026-10-25.md) — Relatório Consolidado de Acessos e Operação — Eleição de 25/10/2026
* [RELATORIO_ESTRESSE_E_CAPACIDADE_BORDA.md](relatorios/RELATORIO_ESTRESSE_E_CAPACIDADE_BORDA.md) — Relatório de Carga, Estresse e Capacidade da Borda (Urna Velox)
* [RELATORIO_M002_VAZAO.md](relatorios/RELATORIO_M002_VAZAO.md) — M-002 — Vazão de ponta a ponta e topologia (medição de 18/10/2026)
* [RELATORIO_SIMULADO_1T_2026.md](relatorios/RELATORIO_SIMULADO_1T_2026.md) — Relatório Oficial — 1º Simulado Urna Velox: 1º Turno de 2026
* [RELATORIO_SIMULADO_3220_ORDEM.md](relatorios/RELATORIO_SIMULADO_3220_ORDEM.md) — Simulado do 3220 na ordem de recebimento do TSE (16/10/2026)
* [VALIDACAO_0.2.0.md](relatorios/VALIDACAO_0.2.0.md) — Validação da etapa 2 (catálogo e candidatos pré-eleição) — 12/10/2026
* [VALIDACAO_1.1_IMPLANTACAO_2026-10-30.md](relatorios/VALIDACAO_1.1_IMPLANTACAO_2026-10-30.md) — Validação da 1.1 (multipleito) na implantação: T1–T14 — 30/10/2026

## 🤝 Handoffs — `handoffs/`

Passagens de trabalho entre sessões ou para outra IA/pessoa: contexto, estado e próximos passos de um assunto.

* [HANDOFF_0.1.0.md](handoffs/HANDOFF_0.1.0.md) — Handoff — Urna Velox: Etapa 1, o motor (`0.1.0`, alvo 14/10/2026)
* [HANDOFF_0.2.0.md](handoffs/HANDOFF_0.2.0.md) — Handoff — Urna Velox: Etapa 2, catálogo e candidatos pré-eleição (`0.2.0`, alvo 16/10/2026)
* [HANDOFF_0.3.0.md](handoffs/HANDOFF_0.3.0.md) — Handoff — Urna Velox: Etapa 3, operação automatizada (`0.3.0`, alvo 19/10/2026)
* [HANDOFF_0.4.0.md](handoffs/HANDOFF_0.4.0.md) — Handoff — Urna Velox: Etapa 4, publicação em tempo real (`0.4.0`, alvo 21/10/2026)
* [HANDOFF_0.4.10.md](handoffs/HANDOFF_0.4.10.md) — Handoff — sessão de 14/10/2026 (0.4.10 a 0.4.12), etapa 5 (site de exemplo)
* [HANDOFF_0.5.0.md](handoffs/HANDOFF_0.5.0.md) — Handoff — Urna Velox: Etapa 5, site de exemplo (`0.5.0`, alvo 22/10/2026)
* [HANDOFF_0.6.0.md](handoffs/HANDOFF_0.6.0.md) — Handoff — Urna Velox 2.0: módulo de escalabilidade segura (`2.0.0-alpha.N`)
* [HANDOFF_0.7.0.md](handoffs/HANDOFF_0.7.0.md) — Handoff — Urna Velox: Etapa 7, comparação Urna Velox × TSE (`0.7.0`, alvo 19/10/2026)
* [HANDOFF_0.8.0.md](handoffs/HANDOFF_0.8.0.md) — Handoff — Urna Velox: Etapa 8, previsão do matematicamente e do fim da apuração (`0.8.0`, alvo 21/10/2026)
* [HANDOFF_0.9.0.md](handoffs/HANDOFF_0.9.0.md) — Handoff — Urna Velox: Etapa 6, ensaio geral (`0.6.0`, alvo 23/10/2026)
* [HANDOFF_1.1.0.md](handoffs/HANDOFF_1.1.0.md) — Handoff — Urna Velox: etapa 1.1, multipleito (`1.1.0`): a visão de 29/10/2026 e o que falta
* [HANDOFF_2024.md](handoffs/HANDOFF_2024.md) — Handoff — Urna Velox: totalizar as eleições municipais de outubro de 2024 (Prefeito e consultas populares)
* [HANDOFF_B056.md](handoffs/HANDOFF_B056.md) — Handoff B-056 · BUs do exterior no ar antes do `cs.json` (25/10/2026, madrugada do 2º turno)
* [HANDOFF_BUGS_2026-10-18.md](handoffs/HANDOFF_BUGS_2026-10-18.md) — Handoff — bugs e melhorias do ciclo diário (18/10/2026)
* [HANDOFF_CARGA_E_ARQUITETURA.md](handoffs/HANDOFF_CARGA_E_ARQUITETURA.md) — Handoff — Testes de Carga, Estresse e Subsídios para Refatoração Arquitetural
* [HANDOFF_DEPLOY_SEM_SSH.md](handoffs/HANDOFF_DEPLOY_SEM_SSH.md) — Handoff — Implantação nos nós da OCI sem a porta 22 e sem o portal
* [HANDOFF_FS_COMPARTILHADO.md](handoffs/HANDOFF_FS_COMPARTILHADO.md) — Handoff — N instâncias de coleta com sistema de arquivos compartilhado (para revisão crítica)
* [HANDOFF_IMPLANTACAO_OCI.md](handoffs/HANDOFF_IMPLANTACAO_OCI.md) — Handoff — implantação de dois nós do Urna Velox em servidores Arm da OCI
* [HANDOFF_MARCA.md](handoffs/HANDOFF_MARCA.md) — Handoff — Urna Velox: logo e ícone de aplicativo (para o Claude Design)
* [HANDOFF_MERGE_2.0_MAIN.md](handoffs/HANDOFF_MERGE_2.0_MAIN.md) — Handoff de Engenharia — Merge da Versão 2.0 para a Branch `main`
* [HANDOFF_MODELO_BORDA_E_CONSISTENCIA.md](handoffs/HANDOFF_MODELO_BORDA_E_CONSISTENCIA.md) — Handoff — Modelo de Borda, Consistência Canônica e Alta Escala
* [HANDOFF_OOM_RETOMADA_NO_A.md](handoffs/HANDOFF_OOM_RETOMADA_NO_A.md) — Handoff — falta de memória (OOM) do `uv-servico` na retomada, nó A da OCI
* [HANDOFF_SESSAO_2026-10-14.md](handoffs/HANDOFF_SESSAO_2026-10-14.md) — Handoff da sessão de 14/10/2026 — início da 2.0 (escalabilidade segura)
* [PROMPT_CABECALHO_1.1.md](handoffs/PROMPT_CABECALHO_1.1.md) — Prompt para a melhoria do cabeçalho da página (etapa 1.1) — 30/10/2026
* [PROMPT_ACERVO_BUS.md](handoffs/PROMPT_ACERVO_BUS.md) — Prompt: acervo local de BUs (dados abertos do TSE)

## 💡 Propostas — `propostas/`

Ideias ainda não decididas, para avaliação.

* [PROPOSTA_COMPARACAO_DIRETA_CDN_TSE.md](propostas/PROPOSTA_COMPARACAO_DIRETA_CDN_TSE.md) — Proposta: Comparação ao Vivo Direto da CDN do TSE
* [PROPOSTA_SERIE_CONSULTA.md](propostas/PROPOSTA_SERIE_CONSULTA.md) — Proposta: Série de Consulta de Cada Pleito (a base do slider, etapa 1.1)

## 🎨 Marca — `marca/`

Guia da marca e peças.

* [GUIA_MARCA.md](marca/GUIA_MARCA.md) — Guia Rápido de Uso da Marca — Urna Velox

## Regras

1. **Nada solto na raiz de `docs/`:** lá só ficam este `README.md` e o `BUGS.md`. Todo documento novo vai para uma
   das pastas acima; pasta nova só com decisão do Luciano (e entra em `scripts/docs.py` e aqui).
2. **Nome pelo tipo, em MAIÚSCULAS_COM_SUBLINHADO.md:**
   * `handoffs/`: `HANDOFF_<assunto>.md` (ou `PROMPT_<assunto>.md` para um prompt de trabalho);
   * `relatorios/`: `RELATORIO_<assunto>.md`, `ANALISE_<assunto>_<AAAA-MM-DD>.md` (incidente ou noite) ou `VALIDACAO_<versão>.md`;
   * `propostas/`: `PROPOSTA_<assunto>.md`;
   * as outras pastas: o nome do assunto (`OPERACAO.md`, `PROTOCOLO_WS.md`).
   Data no nome sempre `AAAA-MM-DD`; versão como `0.8.0`.
3. **Onde vai cada coisa:** como o sistema é feito → `arquitetura/`; como operar → `operacao/`; o que foi medido ou
   aconteceu → `relatorios/`; passagem de trabalho → `handoffs/`; ideia a decidir → `propostas/`; regra de trabalho
   → `processo/`; bug ou melhoria → uma linha no `BUGS.md` e, se precisar de texto, `itens/<id>.md` (regras no
   [CLAUDE.md](../CLAUDE.md), "Bugs").
4. **Todo documento entra neste índice** no mesmo commit em que é criado (uma linha com o que ele é).
5. **Imagens** em `img/` (com o prefixo do documento, ex.: `analise_2026-10-25_curvas.png`), em `itens/img/` (de um
   bug ou melhoria: `B-067-<assunto>.png`) ou em `marca/`. Nunca ao lado dos `.md`.
6. **Links relativos sempre válidos:** ao mover ou renomear, atualizar os links (o `verificar` recusa link quebrado
   nos `.md` de `docs/` e da raiz). Mover com `git mv`, para manter o histórico.
7. **Só dados públicos do TSE:** nada da estrutura interna do TSE (o verificador de segurança do pre-commit recusa).
8. **Documento superado não se apaga:** fica, com uma linha no topo apontando para o que o substitui.
