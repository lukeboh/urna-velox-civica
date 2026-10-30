# Regras permanentes do projeto Urna Velox (para o Claude e para quem colaborar)

Leia isto antes de qualquer trabalho no repositório.

## Versionamento — obrigatório em todo commit
* Siga [docs/processo/VERSIONAMENTO.md](docs/processo/VERSIONAMENTO.md). **Cada commit sobe a versão** (`scripts/versao.sh bump patch`, sem resumo na linha de comando: o PowerShell corta argumentos com espaço; o resumo vai direto no CHANGELOG), atualiza o [CHANGELOG.md](CHANGELOG.md). **Só as versões "zeradas" (`X.Y.0`: etapa concluída ou MAJOR) ganham tag** anotada `vX.Y.0` (o hook cria); os PATCH não têm tag. Publique com `git push --follow-tags`.
* MINOR sobe só quando uma etapa do [ROADMAP.md](ROADMAP.md) é concluída: motor = `0.1.0`; catálogo pré-eleição = `0.2.0`; operação automatizada = `0.3.0`; WebSocket = `0.4.0`; site de exemplo = `0.5.0`; escalabilidade segura (federação de nós; [docs/arquitetura/ESCALABILIDADE_SEGURA.md](docs/arquitetura/ESCALABILIDADE_SEGURA.md)) = `0.6.0`; comparação Urna Velox × TSE ([docs/handoffs/HANDOFF_0.7.0.md](docs/handoffs/HANDOFF_0.7.0.md)) = `0.7.0`; previsão do matematicamente e do fim da apuração ([docs/handoffs/HANDOFF_0.8.0.md](docs/handoffs/HANDOFF_0.8.0.md)) = `0.8.0`; ensaio geral ([docs/handoffs/HANDOFF_0.9.0.md](docs/handoffs/HANDOFF_0.9.0.md)) = `0.9.0`. **`1.0.0` = sistema completo para os cargos majoritários** (automatizado, federação, WebSocket, site de exemplo com comparação e previsão), meta: pronto para o 2º turno de 2026 (25/10/2026). **`1.1.0` = multipleito** (concluída em 30/10/2026: todo pleito do `ele-c.json` reconhecido sozinho, playlist dinâmica, consulta de qualquer pleito com o slider, seletor de pleito no cabeçalho). Publicação na borda, quórum e Wasm ficam para a `1.x`. **`2.0.0` = cargos proporcionais** com distribuição de vagas. Renumeração de 15/10/2026: o que documentos anteriores chamam de "2.0" / `2.0.0-alpha.N` é a etapa 6, a antiga etapa 6 (ensaio) é a 7 e a antiga `3.0.0` é a `2.0.0`. Renumeração de 16/10/2026: o ensaio geral passou de etapa 7 (`0.7.0`) para etapa 9 (`0.9.0`), com a comparação (7) e a previsão (8) antes dele (mapas no [ROADMAP.md](ROADMAP.md)). Protótipo = série `0.0.x`. Usar o sistema numa eleição real não muda a versão.
* Num clone novo, rode `scripts/instalar_hooks.sh` (hooks em `.githooks/` conferem o bump e criam a tag).
* Ao concluir um item ou critério do roadmap, marque-o no `ROADMAP.md` com a versão em que foi atingido.
* Identidade git deste repositório: `Luciano Bohnert <luciano.bohnert@gmail.com>` (configurada em `git config` local).

## Verificação — obrigatória antes de todo commit que mexe em totalização ou resultado
* **Toda implementação que mexe em totalização ou resultado deve ser verificada contra os arquivos oficiais de divulgação do TSE** antes do commit, e o resultado da conferência (abrangências idênticas / divergências) vai no CHANGELOG.
* Desde a `v0.1.0` as conferências da etapa 0 são **testes automatizados** do motor: `cargo test --release -p urna-velox` (unidade + fixtures) e `UV_DADOS=/home/dev/uvreal cargo test --release -p urna-velox -- --ignored --test-threads 1` (2026 83/83, 2024 5.574/5.574 e 51/51, com a situação de cada candidato e o "matematicamente"). Os scripts Python (`comparar_oficial.py`, `comparar_municipal.py`, `aquisicao/scripts/conferir_tres.sh`) ficam como conferência secundária do `uv-coletor`.
* Cobertura: `scripts/cobertura.sh` (motor: 84,3 % das linhas na `v0.1.0`).

## Bugs — primeiro o teste que comprova, depois a correção (regra de 18/10/2026)
* Todo bug vai para a lista [docs/BUGS.md](docs/BUGS.md) assim que é visto, só com o sintoma (sem análise). **A ordem de
  ataque é decidida pelo Luciano**: nenhum bug da lista é analisado ou corrigido antes disso.
* **Bugs, melhorias e roadmap andam juntos (regra de 27/10/2026).** Todo item novo (B ou M) entra no `docs/BUGS.md` **já
  com destino** na coluna `roadmap`, no mesmo commit: a etapa do [ROADMAP.md](ROADMAP.md) a que ele pertence (`1.0.x` =
  manutenção do que já foi entregue, só PATCH; `1.1`, `1.2`, `1.x`, `2.0`… = etapa futura, e o item vira parte dela) ou
  `fora` (descartado, com o motivo). Na dúvida, `1.0.x` e o Luciano reclassifica. Item que muda o escopo de uma etapa
  (ou pede uma etapa nova) entra também no texto da etapa no ROADMAP. Ao fechar um item: estado `corrigido`/`feito` e a
  versão. Depois de mexer no `BUGS.md`: `python3 scripts/itens.py gerar` (reescreve as listas de itens de cada etapa no
  ROADMAP) — o pre-commit roda `scripts/itens.py verificar` e recusa item sem destino, item fechado sem versão, etapa ✅
  com item aberto e lista desatualizada. A tabela do `BUGS.md` fica curta (uma frase por item); análise, saídas de teste
  e o antes/depois vão no arquivo do item em `docs/itens/<id>.md`, ligado na coluna `detalhe`.
* Para corrigir um bug, nesta ordem:
  1. **Criar o caso de teste que comprova o bug** (teste automatizado: `cargo test`, `--ignored` com dados reais, teste
     da página em `site/testes/`, teste de script). Rodar e **ver o teste falhar** pelo motivo do bug; registrar no
     `docs/BUGS.md` o nome do teste e a saída da falha.
  2. **Só então implementar a correção.**
  3. Rodar o teste de novo (agora passa) e a suíte inteira (nada que passava pode quebrar); se mexe em totalização ou
     resultado, vale também a verificação contra os oficiais (seção acima).
  4. No commit (com versão nova): o teste fica no repositório para sempre; CHANGELOG e `docs/BUGS.md` citam o bug, o
     teste e o resultado antes/depois (o detalhe em `docs/itens/<id>.md`).
* Bug que não dá para reproduzir em teste automatizado: registrar por que e combinar com o Luciano antes de corrigir.

## Rede e servidores do TSE
* **No máximo 100 requisições por segundo ao TSE por IP de saída** (o IP que a CDN do TSE vê), somando tudo o que sai por esse IP; acima disso a borda responde HTTP 429. Coletas longas usam 95 req/s. Nunca usar proxies ou outros truques para fazer um IP passar do limite.
  * **O limite é da borda pública** (`resultados.tse.jus.br`, `cdn.tse.jus.br`; regra do Luciano, 19/10/2026). Contra um **servidor espelho** dos arquivos públicos (fora da borda pública; modo `interno`) o operador calibra o teto ou o desliga: `ACESSO=interno` e `TETO_RPS_INTERNO` no `no.env`, pelo painel dos nós ([docs/operacao/PAINEL.md](docs/operacao/PAINEL.md)). O código recusa o modo espelho para os endereços da borda pública (e quando os BUs vêm de lá): neles o limite de 100 vale sempre. A pausa de 10 min depois de um 429 vale nos dois modos.
* **Federação** (etapa 6): vários nós dividem as seções, cada um no seu IP de saída e nos seus 100 req/s; a vazão da federação soma os limites dos IPs participantes.
* **Nós na mesma infraestrutura** (ADR-029): cada nó coletor tem IP público reservado próprio e saída direta por ele (sem NAT/gateway comum), para não dividir o limite sem querer.
* **O limite vale por IP de saída** (o que a CDN do TSE vê). Cada nó da federação publica o seu IP de saída e conhece o dos outros; nós que saem pelo **mesmo IP** (NAT, proxy, rede compartilhada) contam como um só e **dividem** os 100 req/s desse IP. IPs de saída diferentes não têm por que somar no mesmo limite.
* Depois de um HTTP 429: parar tudo e esperar pelo menos 10 minutos (a borda põe o IP numa "caixa de penalidade"; o IP de saída pode ser compartilhado). O `uv-coletor` tem disjuntor (`--max-429`).
* Retomadas: usar `--pular-sem-bu` com as métricas anteriores (milhares de 404 seguidos provocam 429 mesmo abaixo de 100 req/s).
* BUs já salvos no acervo local (`D:\dados\tse-arquivos-urna`, regras em `aquisicao/acervo/README.md`) não devem ser baixados de novo.
* Não rodar coletas durante a apuração de uma eleição (noite do pleito e dia seguinte) sem combinar.
  * **Já combinado (29/10/2026, Luciano: "autorizo qualquer pleito em curso totalizar sozinho"): o multipleito** (etapa
    1.1) totaliza ao vivo, sem ninguém disparar, todo pleito **em curso** no `ele-c.json` (do dia da eleição às 8 h do
    dia seguinte), no lugar do replay: modo `aovivo` do `uv-ciclo.sh`, com a configuração de produção do nó (≤ 95 req/s
    por IP de saída, a pausa de 10 min do 429, o B só recebendo quando é do mesmo órgão), um pleito por vez. A montagem
    dos acervos de replay (`uv-pleitos.sh`) continua parada na noite e no dia seguinte de qualquer eleição.
  * **Já combinado (16/10/2026): a comparação Urna Velox × TSE** (etapa 7) pode buscar, durante a apuração, o arquivo de resultado (`-u.json`) **só da abrangência e cargo que estão na tela** de algum cliente com a comparação ligada, **no máximo uma vez a cada 30–60 s** por (abrangência, cargo), com cache compartilhado entre os clientes, **nada em segundo plano**, dentro dos 100 req/s do IP e com a regra do 429. Detalhes no [docs/handoffs/HANDOFF_0.7.0.md](docs/handoffs/HANDOFF_0.7.0.md). Qualquer coisa além disso continua precisando de acordo.

## Onde fica cada coisa
* Workspace Cargo na raiz (`Cargo.toml`, um `Cargo.lock` só): **motor** em `motor/` (crate `urna-velox`, binário `uv-motor`: leitor de BU, pacote `.uvpk`, catálogo, estado, agregador) e **aquisição** em `aquisicao/` (crate `uv-aquisicao`, binário `uv-coletor`, depende do motor); documentação em [docs/arquitetura/AQUISICAO.md](docs/arquitetura/AQUISICAO.md) e [aquisicao/README.md](aquisicao/README.md).
* As três conferências (2026, 2024 1º e 2º turnos) de uma vez: `aquisicao/scripts/conferir_tres.sh <saída>` (`UV_BIN` = binário).
* Dados grandes ficam fora do repositório, em `~/uvreal` no WSL (Ubuntu-24.04): `dados/*.uvpk`, `oficial/`, `resultado/`, `replay-br/`, métricas.
* Disco da distro e dos ensaios: ver a seção **Disco** abaixo (obrigatória antes e depois de cada ensaio).
* Benchmark sintético: `bench/`. Decisões: `docs/arquitetura/ARCHITECTURE_DECISIONS.md`. Glossário: `docs/arquitetura/GLOSSARIO.md`.
* **`ROADMAP.md` é o roadmap público** (vai para a cópia pública). Evolução nova pedida pelo Luciano só entra nele se ele disser que é pública; sem isso, pergunte onde registrar.
* **Documentação em `docs/`, por pastas (regra de 27/10/2026; índice e regras em [docs/README.md](docs/README.md)):**
  `arquitetura/` (como o sistema é feito), `operacao/` (como operar), `relatorios/` (o que foi medido ou aconteceu:
  `RELATORIO_*`, `ANALISE_*_AAAA-MM-DD`, `VALIDACAO_*`), `handoffs/` (`HANDOFF_*`), `propostas/` (`PROPOSTA_*`),
  `processo/`, `legal/`, `marca/`, `img/`; bugs e melhorias no `docs/BUGS.md` com o detalhe em `docs/itens/<id>.md`.
  Nada solto na raiz de `docs/`; todo documento novo entra no índice no mesmo commit; mover com `git mv` e atualizar os
  links. O pre-commit roda `scripts/docs.py verificar` (pastas, nomes, índice, links relativos dos `.md` de `docs/` e
  da raiz). Pasta nova só com o Luciano.

## Disco — antes e depois de cada ensaio
O `ext4.vhdx` da distro `Ubuntu-24.04` fica em `C:\_PRJ\WSL\Ubuntu-24.04`, no SSD (C:, 477 GB), cresce com tudo o que se grava na distro e **não devolve espaço sozinho**: apagar arquivos dentro dela não libera o C: até a compactação. Em 15/10/2026 os ensaios acumulados (173 GB de `dados/`) encheram o C: e derrubaram o WSL; à noite ainda sobravam 66 GB regeneráveis e o `.vhdx` estava com 169,8 GB para 121 GB usados. O D: é um HD mecânico (1 TB): bom para guardar, ruim para trabalhar.

* **Onde fica cada coisa.** Dentro da distro: só os dados de entrada (`~/uvreal/dados`, `2024-*`, `2026-2t`, `acervo-*`, `preparo*`, `oficial`, `config`, métricas), o cache de compilação (`~/uvreal/target-dev`, `target`, `target-cov`) e, **durante** um ensaio, o que ele grava. No D: (`/mnt/d/dados/urna-velox/`): **dados de cada pleito em `pleitos/<AAAA-MM-DD-código>/`** (data da eleição e código, ex.: `2026-10-25-3221`; linha do tempo, insumos de simulado, acervo dos nós, saída da noite real; regras em `pleitos/README.md`, e nos roteiros `pasta_pleito` do `deploy/oci/lib.sh`), backups dos nós (`backup-nos/`), evidências de ensaios arquivadas (`ensaios/`), temporários de roteiros (`D:\dados\uv-tmp`). O acervo de dados abertos já está em `D:\dados\tse-arquivos-urna`. Não trabalhar sobre o `/mnt/d` em ensaios acelerados: a latência por arquivo atrasa os *heartbeats* e provoca falsas mortes; uma imagem ext4 no D: montada por loop travou sob carga e o `wsl --mount --vhd` exige administrador.
* **Antes de um ensaio:** `bash /mnt/d/git/lukeboh/urna-velox/scripts/disco.sh` (relatório; o `compilar_dev.sh` mostra a linha curta e avisa se o regenerável passou de 5 GB) e, se houver sobra, `scripts/disco.sh --limpar`. Para um ensaio grande (critério A completo, ~45 GB no pico), conferir o espaço livre do C: no Windows, não só o da distro.
* **Durante:** os testes `--ignored` usam a RAM (`TMPDIR=/dev/shm/uv2-tmp`); ensaios manuais gravam em `~/uvreal/ensaios/<nome>` (a distro, não o `/mnt/d`).
* **Depois** (passando ou não, assim que as conferências e o relatório estiverem feitos): `scripts/disco.sh arquivar ~/uvreal/ensaios/<nome>` — guarda as evidências (`saida/`, logs, resumos; sem `dados/` nem `federacao/`) em `D:\dados\urna-velox\ensaios\<data>-<nome>` e apaga a pasta da distro. Sem arquivar, ao menos `scripts/disco.sh --limpar`. O regenerável (`dados/`, `federacao/` e `armazem/` dos ensaios — cópias dos `.uvpk` e do diário gravadas pelo `uv-servico`, ~6 GB por noite de 2026, e lotes trocados entre os nós —, builds antigos `target-*` e temporários `uv-*` órfãos) nunca fica para o ensaio seguinte.
* **Automático:** os ensaios de `aquisicao/tests/` apagam a própria pasta quando passam (ficam só os que falharam, para o diagnóstico; `UV_MANTER_ENSAIO=1` mantém) e, antes de começar, as sobras de execuções anteriores; o `simular_site.sh` apaga a sua cópia dos pacotes ao sair (`UV_MANTER_DADOS=1` mantém).
* **Devolver o espaço ao C:** `scripts/disco.sh --limpar` roda o `fstrim`; para o `.vhdx` encolher de fato, compactar com a distro parada (`wsl --shutdown` e então `Optimize-VHD -Mode Full` ou `diskpart` → `compact vdisk`, que pedem administrador) — combinar com o Luciano, porque derruba qualquer sessão que esteja usando o WSL.

## Convenções
* **Artigo de lei citado para explicar uma regra é sempre um link para a norma no site do TSE, abrindo em aba nova, com
  âncora no artigo exato** (regra do Luciano, 29/10/2026). Na página: a tabela `site/src/normas.js` (`NORMAS` e `ARTIGOS`,
  com o início literal do artigo) e `comLinks`/`linkArtigo` (`target="_blank"`, `rel="noopener noreferrer"`); a âncora é
  o fragmento de texto `#:~:text=<início do artigo>` (o TSE não publica âncoras por artigo); teste
  `site/testes/normas_links.mjs`. Nos documentos (`.md`), o mesmo link com o fragmento (o Markdown não escolhe a aba).
* Textos e mensagens em português do Brasil. Mensagens de commit `área: resumo`. Todos os executáveis começam com `uv-` (`uv-motor`, `uv-coletor`).
* Compilar e testar o workspace sem tocar no binário de uma coleta em andamento: `aquisicao/scripts/compilar_dev.sh` (usa `~/uvreal/target-dev`).
* Conferência de resultados: `aquisicao/scripts/fechar_nacional.sh` (totais × divulgação oficial, Presidente e Governador).
