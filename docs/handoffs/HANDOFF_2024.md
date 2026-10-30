# Handoff — Urna Velox: totalizar as eleições municipais de outubro de 2024 (Prefeito e consultas populares)

*Preparado em 12/10/2026, na versão `v0.0.14`. Para iniciar uma nova sessão de trabalho, cole este documento (ou peça para lê-lo).*

## Contexto
Sou o Luciano. O Urna Velox é um motor de totalização eleitoral de código aberto em Rust (repo `D:\git\lukeboh\urna-velox`, GitHub `lukeboh/urna-velox`, branch `main`). Até agora o trabalho foi todo na eleição geral de 2026 (1º turno, pleito 3220): coletamos os 499.207 BUs do Brasil pelo site de resultados e totalizamos **Presidente, Governador e Senador idênticos à divulgação oficial em 83/83 abrangências**, com a situação de 794 candidatos idêntica (54/54 senadores eleitos).

**Objetivo desta etapa:** avançar o marco "todos os cargos majoritários" (Fase 2 do ROADMAP, `0.2.0`) com as **eleições municipais de outubro de 2024**: totalizar **Prefeito** e as **consultas populares** (plebiscitos/referendos municipais realizados junto com o pleito) em **todos os municípios**, conferindo **100 % com os arquivos oficiais de divulgação**, inclusive a situação de cada candidato (eleito / 2º turno / não eleito). Primeiro o 1º turno (06/10/2024); depois o 2º turno (27/10/2024). Vereador sai de brinde da totalização, mas a distribuição de vagas é a Fase 3 e não entra agora.

## Leia antes de começar (nesta ordem)
1. [`CLAUDE.md`](../../CLAUDE.md) — regras permanentes (versionamento, rede, verificação).
2. [`ROADMAP.md`](../../ROADMAP.md) — situação por fase e tabela de cobertura de cargos.
3. [`CHANGELOG.md`](../../CHANGELOG.md) (entradas 0.0.10 em diante).
4. [`aquisicao/README.md`](../../aquisicao/README.md) e [`docs/arquitetura/AQUISICAO.md`](../arquitetura/AQUISICAO.md) — coletor, API do TSE, armadilhas.
5. [`aquisicao/results/SENADOR.md`](../../aquisicao/results/SENADOR.md) — modelo do que entregar (conferência + desempenho).
6. Doc do projeto no Claude: `claude/aquisicao-bus-tse.md`.

## Regras inegociáveis
- **No máximo 100 requisições por segundo** ao TSE, somando tudo (coletas longas a 95 req/s). Nunca usar vários IPs, proxies ou processos para contornar o limite.
- **HTTP 429:** parar tudo e esperar pelo menos 10 minutos. O `uv-coletor` tem disjuntor (`--max-429`, padrão 20). Milhares de 404 seguidos provocam 429 mesmo a ~70 req/s; na retomada, use `--pular-sem-bu <metricas.csv>`.
- **BUs já salvos no acervo local** (`D:\dados\tse-arquivos-urna`, regras em `aquisicao/acervo/README.md`) não podem ser baixados de novo. De 2024, o acervo já tem `bus/2024/1t/AC.zip` (e `bus/2024/2t/AC.zip` está vazio, 22 bytes).
- **Toda implementação deve ser verificada contra os arquivos oficiais de divulgação** antes do commit (não há testes automatizados suficientes; a cobertura é de 10,6 % e fica para depois). O resultado da conferência vai no CHANGELOG.
- **Versionamento a cada commit:** `scripts/versao.sh bump patch` (sem resumo na linha de comando; o PowerShell corta argumentos com espaço). Escreva o resumo direto no `CHANGELOG.md`, depois `git add`, `git commit`; o hook confere a versão e cria a tag `vX.Y.Z`. Publique com `git push --follow-tags`. O MINOR só sobe ao concluir uma fase do roadmap. Marque no `ROADMAP.md` o que for atingido, com a versão.
- Identidade git do repo: `Luciano Bohnert <luciano.bohnert@gmail.com>`. Assinatura dos commits conforme o sistema indicar.

## Ambiente (como trabalhar na máquina)
- Windows + WSL2 `Ubuntu-24.04`, usuário `dev`, i7-14700 (28 threads). Dados grandes em `~/uvreal` (WSL): `dados/*.uvpk` (2026), `oficial/`, `resultado/`, métricas. **Use pastas separadas para 2024** (ex.: `~/uvreal/2024-1t/dados`, `~/uvreal/2024-1t/oficial`), sem misturar com 2026.
- Os comandos rodam pelo Desktop Commander (`start_process`, shell PowerShell). **Aspas são um problema:** escreva o comando num script `.sh` no repo (ou em `.git/`, se for temporário) e rode `wsl -d Ubuntu-24.04 -- bash /mnt/d/git/lukeboh/urna-velox/...sh`. Jobs longos: `setsid nohup bash script.sh > ~/uvreal/x.log 2>&1 < /dev/null &`. Cada chamada ao computador expira em ~60 s.
- Compile com `aquisicao/scripts/compilar_dev.sh` (gera `~/uvreal/target-dev`, sem tocar no binário de uma coleta em andamento).
- Git e push pelo PowerShell (`cd D:\git\lukeboh\urna-velox; git ...`); a mensagem de commit vai em arquivo (`git commit -F .git\UV_COMMIT_MSG`).

## O que já existe e deve ser reaproveitado
- `uv-coletor` (`aquisicao/`): `eleicoes`, `secoes`, `baixar` (API: cs.json → aux.json → BU; aceita `-bu.dat` e `-busa.dat`), `zip` (dados abertos, `--local`), `catalogo`, `totalizar [--cargos]`, `replay [--cargos]`, `inspecionar`. Parâmetros `--ciclo` (padrão `ele2026`) e `--pleito` (padrão 3220).
- Scripts em `aquisicao/scripts/`: `coleta_nacional.sh`, `progresso.py`, `erros_coleta.py`, `baixar_oficial.sh`, `comparar_oficial.py` (votos, brancos, nulos, nulos técnicos `vnt`, comparecimento, seções `s.st` e situação `st` de cada candidato), `fechar_nacional.sh`, `desempenho_cargo.sh`, `replay_noite.sh`.
- Lições de 2026 que provavelmente valem para 2024:
  - seções apuradas pelo Sistema de Apuração trazem `-busa.dat`;
  - votos em número sem candidato apto vêm nominais no BU e o TSE os soma em `vnt`;
  - candidatos "anulado sub judice" têm `dvt` próprio;
  - seções "Não instalada" entram em `s.st` sem BU;
  - há BUs reenviados dias depois (situação "Recebido");
  - o arquivo oficial de uma abrangência menor repete a situação da circunscrição do cargo (para Prefeito, a circunscrição é o município).

## Plano sugerido
1. **Descobrir, sem deduzir** (poucas requisições, 1 req/s):
   - `uv-coletor eleicoes` (`ele-c.json`): pleitos de 2024 (1º e 2º turnos), códigos das eleições municipais ordinárias e das **consultas populares**, e o ciclo (provavelmente `ele2024`);
   - um `cs.json`, um `aux.json` e um BU de 2024: nomes de arquivo (o acervo sugere `o00452ac0101500020088-bu.dat`, a confirmar), códigos de cargo (Prefeito = 11, Vereador = 13, consultas = ?) e diferenças de formato;
   - a especificação `bu.asn1` de 2024 (pacote "formato dos arquivos de BU, RDV e assinatura digital" do TSE, ou `documentacao/2024` do acervo);
   - o **formato e o endereço dos arquivos oficiais** por município, para Prefeito e consultas (em 2026 foi `<eleição>/dados/<abr>/<abr>-c<cargo>-e<eleição>-u.json`; em 2024 pode ser outro, como `dados-simplificados/.../-r.json`). Registre tudo em `docs/arquitetura/AQUISICAO.md`.
2. **Adaptar o leitor e o coletor ao formato de 2024** (`bu.rs`, `tse.rs`, `zip.rs`), sem quebrar 2026. Confirme com `asn1tools` + `bu.asn1` de 2024 (`conferir_referencia.py`) e rode de novo a conferência de 2026 (`fechar_nacional.sh` deve continuar com 0 divergências).
3. **Coletar o 1º turno de 2024 inteiro** (todas as UFs; não há exterior em eleição municipal), pulando o que já está no acervo (AC). **Decisão a confirmar com o Luciano antes de começar:** pela API, como em 2026 (~2 h 40 a 95 req/s, e grava o horário de recebimento para o benchmark), ou pelos zips de dados abertos (mais rápido com banda, uma requisição por UF, seguindo as regras do acervo). A preferência inicial é a API, pela comparabilidade com 2026.
4. **Baixar os arquivos oficiais** de Prefeito (~5,5 mil municípios) e das consultas, por município. Isso são milhares de requisições: use um ritmo moderado (ex.: 10–20 req/s), nunca simultâneo a uma coleta que já esteja perto de 100 req/s, e pule o que já foi baixado.
5. **Totalizar e conferir:** generalizar `comparar_oficial.py` para a abrangência municipal, para o Prefeito (situação: maioria absoluta com 2º turno só em municípios com mais de 200 mil eleitores, usando o eleitorado do próprio arquivo oficial; maioria simples nos demais; desempate pelo mais idoso) e para as consultas (votos por opção). Meta: **100 % dos municípios idênticos**, inclusive a situação de cada candidato.
6. **Medir o desempenho** como em `SENADOR.md` (`totalizar --cargos 11`, 1 e 28 threads; replay da noite, se o horário de recebimento estiver disponível) e escrever `aquisicao/results/PREFEITO_2024.md`.
7. **Atualizar** ROADMAP (tabela de cobertura: Prefeito e consultas), CHANGELOG, docs e o doc do projeto. Commit e push a cada passo verificado.
8. Depois: o 2º turno de 2024 (27/10) pelo mesmo caminho.

## Critério de pronto desta etapa
- Prefeito e consultas populares do 1º turno de 2024 totalizados a partir dos BUs, idênticos à divulgação oficial em todos os municípios (votos, brancos, nulos, nulos técnicos, comparecimento, seções e situação de cada candidato).
- 2026 continua idêntico (sem regressão).
- Desempenho medido e documentado; ROADMAP e CHANGELOG atualizados; tudo versionado e com tag.
