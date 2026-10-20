# Regras permanentes do projeto Urna Velox (para o Claude e para quem colaborar)

Leia isto antes de qualquer trabalho no repositório.

## Versionamento — obrigatório em todo commit
* Siga [docs/VERSIONAMENTO.md](docs/VERSIONAMENTO.md). **Cada commit sobe a versão** (`scripts/versao.sh bump patch`, sem resumo na linha de comando: o PowerShell corta argumentos com espaço; o resumo vai direto no CHANGELOG), atualiza o [CHANGELOG.md](CHANGELOG.md). **Só as versões "zeradas" (`X.Y.0`: etapa concluída ou MAJOR) ganham tag** anotada `vX.Y.0` (o hook cria); os PATCH não têm tag. Publique com `git push --follow-tags`.
* MINOR sobe só quando uma etapa do [ROADMAP.md](ROADMAP.md) é concluída: motor = `0.1.0`; catálogo pré-eleição = `0.2.0`; operação automatizada = `0.3.0`; WebSocket = `0.4.0`; site de exemplo = `0.5.0`; escalabilidade segura (federação de nós; [docs/ESCALABILIDADE_SEGURA.md](docs/ESCALABILIDADE_SEGURA.md)) = `0.6.0`; comparação Urna Velox × TSE ([docs/HANDOFF_0.7.0.md](docs/HANDOFF_0.7.0.md)) = `0.7.0`; previsão do matematicamente e do fim da apuração ([docs/HANDOFF_0.8.0.md](docs/HANDOFF_0.8.0.md)) = `0.8.0`; ensaio geral ([docs/HANDOFF_0.9.0.md](docs/HANDOFF_0.9.0.md)) = `0.9.0`. **`1.0.0` = sistema completo para os cargos majoritários** (automatizado, federação, WebSocket, site de exemplo com comparação e previsão), meta: pronto para o 2º turno de 2026 (25/10/2026). Publicação na borda, quórum e Wasm ficam para a `1.x`. **`2.0.0` = cargos proporcionais** com distribuição de vagas. Renumeração de 15/10/2026: o que documentos anteriores chamam de "2.0" / `2.0.0-alpha.N` é a etapa 6, a antiga etapa 6 (ensaio) é a 7 e a antiga `3.0.0` é a `2.0.0`. Renumeração de 16/10/2026: o ensaio geral passou de etapa 7 (`0.7.0`) para etapa 9 (`0.9.0`), com a comparação (7) e a previsão (8) antes dele (mapas no [ROADMAP.md](ROADMAP.md)). Protótipo = série `0.0.x`. Usar o sistema numa eleição real não muda a versão.
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
* Para corrigir um bug, nesta ordem:
  1. **Criar o caso de teste que comprova o bug** (teste automatizado: `cargo test`, `--ignored` com dados reais, teste
     da página em `site/testes/`, teste de script). Rodar e **ver o teste falhar** pelo motivo do bug; registrar no
     `docs/BUGS.md` o nome do teste e a saída da falha.
  2. **Só então implementar a correção.**
  3. Rodar o teste de novo (agora passa) e a suíte inteira (nada que passava pode quebrar); se mexe em totalização ou
     resultado, vale também a verificação contra os oficiais (seção acima).
  4. No commit (com versão nova): o teste fica no repositório para sempre; CHANGELOG e `docs/BUGS.md` citam o bug, o
     teste e o resultado antes/depois.
* Bug que não dá para reproduzir em teste automatizado: registrar por que e combinar com o Luciano antes de corrigir.

## Rede e servidores do TSE
* **No máximo 100 requisições por segundo ao TSE por órgão**, somando tudo o que sai do órgão (cada órgão acessa pelo seu IP; acima de 100 req/s a borda do TSE responde HTTP 429). Coletas longas usam 95 req/s. Nunca usar vários IPs, proxies ou processos **do mesmo órgão** para contornar o limite.
  * **O limite é da borda pública** (`resultados.tse.jus.br`, `cdn.tse.jus.br`; regra do Luciano, 19/10/2026). Contra um **servidor espelho** dos arquivos públicos (fora da borda pública; modo `interno`) o operador calibra o teto ou o desliga: `ACESSO=interno` e `TETO_RPS_INTERNO` no `no.env`, pelo painel dos nós ([docs/PAINEL.md](docs/PAINEL.md)). O código recusa o modo espelho para os endereços da borda pública (e quando os BUs vêm de lá): neles o limite de 100 vale sempre. A pausa de 10 min depois de um 429 vale nos dois modos.
* **Órgãos diferentes podem colaborar** (consórcio, acordo entre si): é o que a federação (etapa 6) faz. Cada nó é de um órgão diferente, no IP desse órgão, com o seu próprio limite de 100 req/s; a vazão da federação soma os limites dos órgãos participantes. Um órgão não roda mais de um nó.
* **Órgãos hospedados na mesma infraestrutura** (ADR-029): cada órgão tem máquina e IP público reservado **exclusivos** e saída direta por ele (sem NAT/gateway comum). Isso é garantia da operação: um órgão nunca pode perder vazão por dividir IP com outro. Os órgãos colaboram numa só federação.
* **O limite vale por IP de saída** (o que a CDN do TSE vê). Cada nó da federação publica o seu IP de saída e conhece o dos outros; nós que saem pelo **mesmo IP** (NAT, proxy, rede compartilhada) contam como um só e **dividem** os 100 req/s desse IP. IPs de saída diferentes não têm por que somar no mesmo limite.
* Depois de um HTTP 429: parar tudo e esperar pelo menos 10 minutos (a borda põe o IP numa "caixa de penalidade"; o IP de saída pode ser compartilhado). O `uv-coletor` tem disjuntor (`--max-429`).
* Retomadas: usar `--pular-sem-bu` com as métricas anteriores (milhares de 404 seguidos provocam 429 mesmo abaixo de 100 req/s).
* BUs já salvos no acervo local (`D:\dados\tse-arquivos-urna`, regras em `aquisicao/acervo/README.md`) não devem ser baixados de novo.
* Não rodar coletas durante a apuração de uma eleição (noite do pleito e dia seguinte) sem combinar.
  * **Já combinado (16/10/2026): a comparação Urna Velox × TSE** (etapa 7) pode buscar, durante a apuração, o arquivo de resultado (`-u.json`) **só da abrangência e cargo que estão na tela** de algum cliente com a comparação ligada, **no máximo uma vez a cada 30–60 s** por (abrangência, cargo), com cache compartilhado entre os clientes, **nada em segundo plano**, dentro dos 100 req/s do órgão e com a regra do 429. Detalhes no [docs/HANDOFF_0.7.0.md](docs/HANDOFF_0.7.0.md). Qualquer coisa além disso continua precisando de acordo.

## Onde fica cada coisa
* Workspace Cargo na raiz (`Cargo.toml`, um `Cargo.lock` só): **motor** em `motor/` (crate `urna-velox`, binário `uv-motor`: leitor de BU, pacote `.uvpk`, catálogo, estado, agregador) e **aquisição** em `aquisicao/` (crate `uv-aquisicao`, binário `uv-coletor`, depende do motor); documentação em [docs/AQUISICAO.md](docs/AQUISICAO.md) e [aquisicao/README.md](aquisicao/README.md).
* As três conferências (2026, 2024 1º e 2º turnos) de uma vez: `aquisicao/scripts/conferir_tres.sh <saída>` (`UV_BIN` = binário).
* Dados grandes ficam fora do repositório, em `~/uvreal` no WSL (Ubuntu-24.04): `dados/*.uvpk`, `oficial/`, `resultado/`, `replay-br/`, métricas.
* Disco da distro e dos ensaios: ver a seção **Disco** abaixo (obrigatória antes e depois de cada ensaio).
* Benchmark sintético: `bench/`. Decisões: `docs/ARCHITECTURE_DECISIONS.md`. Glossário: `docs/GLOSSARIO.md`.

## Disco — antes e depois de cada ensaio
O `ext4.vhdx` da distro `Ubuntu-24.04` fica em `C:\_PRJ\WSL\Ubuntu-24.04`, no SSD (C:, 477 GB), cresce com tudo o que se grava na distro e **não devolve espaço sozinho**: apagar arquivos dentro dela não libera o C: até a compactação. Em 15/10/2026 os ensaios acumulados (173 GB de `dados/`) encheram o C: e derrubaram o WSL; à noite ainda sobravam 66 GB regeneráveis e o `.vhdx` estava com 169,8 GB para 121 GB usados. O D: é um HD mecânico (1 TB): bom para guardar, ruim para trabalhar.

* **Onde fica cada coisa.** Dentro da distro: só os dados de entrada (`~/uvreal/dados`, `2024-*`, `2026-2t`, `acervo-*`, `preparo*`, `oficial`, `config`, métricas), o cache de compilação (`~/uvreal/target-dev`, `target`, `target-cov`) e, **durante** um ensaio, o que ele grava. No D: (`/mnt/d/dados/urna-velox/`): evidências de ensaios arquivadas (`ensaios/`), insumos de simulados (`simulado-3220/`), temporários de roteiros (`D:\dados\uv-tmp`). O acervo de dados abertos já está em `D:\dados\tse-arquivos-urna`. Não trabalhar sobre o `/mnt/d` em ensaios acelerados: a latência por arquivo atrasa os *heartbeats* e provoca falsas mortes; uma imagem ext4 no D: montada por loop travou sob carga e o `wsl --mount --vhd` exige administrador.
* **Antes de um ensaio:** `bash /mnt/d/git/lukeboh/urna-velox/scripts/disco.sh` (relatório; o `compilar_dev.sh` mostra a linha curta e avisa se o regenerável passou de 5 GB) e, se houver sobra, `scripts/disco.sh --limpar`. Para um ensaio grande (critério A completo, ~45 GB no pico), conferir o espaço livre do C: no Windows, não só o da distro.
* **Durante:** os testes `--ignored` usam a RAM (`TMPDIR=/dev/shm/uv2-tmp`); ensaios manuais gravam em `~/uvreal/ensaios/<nome>` (a distro, não o `/mnt/d`).
* **Depois** (passando ou não, assim que as conferências e o relatório estiverem feitos): `scripts/disco.sh arquivar ~/uvreal/ensaios/<nome>` — guarda as evidências (`saida/`, logs, resumos; sem `dados/` nem `federacao/`) em `D:\dados\urna-velox\ensaios\<data>-<nome>` e apaga a pasta da distro. Sem arquivar, ao menos `scripts/disco.sh --limpar`. O regenerável (`dados/`, `federacao/` e `armazem/` dos ensaios — cópias dos `.uvpk` e do diário gravadas pelo `uv-servico`, ~6 GB por noite de 2026, e lotes trocados entre os nós —, builds antigos `target-*` e temporários `uv-*` órfãos) nunca fica para o ensaio seguinte.
* **Automático:** os ensaios de `aquisicao/tests/` apagam a própria pasta quando passam (ficam só os que falharam, para o diagnóstico; `UV_MANTER_ENSAIO=1` mantém) e, antes de começar, as sobras de execuções anteriores; o `simular_site.sh` apaga a sua cópia dos pacotes ao sair (`UV_MANTER_DADOS=1` mantém).
* **Devolver o espaço ao C:** `scripts/disco.sh --limpar` roda o `fstrim`; para o `.vhdx` encolher de fato, compactar com a distro parada (`wsl --shutdown` e então `Optimize-VHD -Mode Full` ou `diskpart` → `compact vdisk`, que pedem administrador) — combinar com o Luciano, porque derruba qualquer sessão que esteja usando o WSL.

## Convenções
* Textos e mensagens em português do Brasil. Mensagens de commit `área: resumo`. Todos os executáveis começam com `uv-` (`uv-motor`, `uv-coletor`).
* Compilar e testar o workspace sem tocar no binário de uma coleta em andamento: `aquisicao/scripts/compilar_dev.sh` (usa `~/uvreal/target-dev`).
* Conferência de resultados: `aquisicao/scripts/fechar_nacional.sh` (totais × divulgação oficial, Presidente e Governador).
