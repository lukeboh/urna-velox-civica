# Versionamento

O Urna Velox usa [versionamento semântico](https://semver.org/lang/pt-BR/) `MAJOR.MINOR.PATCH`, amarrado às etapas do [ROADMAP](../../ROADMAP.md).

## Regras

| parte | quando sobe | exemplo |
|---|---|---|
| **PATCH** | **a cada commit**, sem exceção (código, documentação, dados, scripts) | `0.0.10` → `0.0.11` |
| **MINOR** | quando uma etapa do roadmap é **concluída** (critério atingido e itens obrigatórios entregues); zera o PATCH. Depois da `1.0`, também quando entra funcionalidade nova compatível | etapa 1 (motor) → `0.1.0`; etapa 2 (catálogo pré-eleição) → `0.2.0`; etapa 3 (operação automatizada) → `0.3.0`; etapa 4 (WebSocket) → `0.4.0`; etapa 5 (site de exemplo) → `0.5.0`; etapa 6 (escalabilidade segura, federação) → `0.6.0`; etapa 7 (comparação Urna Velox × TSE) → `0.7.0`; etapa 8 (previsão do matematicamente e do fim da apuração) → `0.8.0`; etapa 9 (ensaio geral) → `0.9.0`; multipleito → `1.1.0` (30/10/2026); coleta robusta na borda → `1.2.0`; auditoria, exportações, borda, quórum, Wasm etc. → `1.x` |
| **MAJOR** | `1.0.0` = **sistema completo para os cargos majoritários** (automatizado, federação, WebSocket, site de exemplo); `2.0.0` = **cargos proporcionais** com distribuição de vagas; depois, **cada grande evolução** (regra do Luciano, 27/10/2026) e toda mudança incompatível de formato, CLI, API ou protocolo entre nós | `1.0.0`, `2.0.0`, `3.0.0`… |

* A **etapa 0** (protótipo e validação) correu na série `0.0.x`. Enquanto a etapa N+1 não estiver concluída, o trabalho continua na série `0.N.x`.
* **Versões alpha (trabalho preliminar de uma MAJOR em paralelo).** Uma MAJOR desenvolvida numa branch enquanto a `main` segue em outra série usa pré-release do semver: `X.0.0-alpha.N` (ex.: a 2.0, na branch `feature/2.0-escalabilidade`: `2.0.0-alpha.1`, `2.0.0-alpha.2`…). Cada commit da branch sobe o `N` (`scripts/versao.sh bump alpha`); a primeira é gravada com `scripts/versao.sh definir 2.0.0-alpha.1`. Alphas não ganham tag. Ao trazer a `main` para a branch, o conflito em `VERSION`, manifestos e CHANGELOG fica com a alpha (e as entradas da `main` entram no CHANGELOG abaixo dela). Quando a MAJOR for concluída e a branch voltar para a `main`, a versão vira `X.0.0` (`definir`), com tag. Se o trabalho da branch for trazido para a `main` como uma etapa da série corrente (como a escalabilidade, em 15/10/2026), a versão vira a dessa etapa (`definir 0.N.0`), com tag, e as alphas ficam no CHANGELOG como histórico dela.
* **Renumeração de 15/10/2026.** A escalabilidade segura (antes `2.0.0`) virou a etapa 6 (`0.6.0`), trazida da branch `feature/2.0-escalabilidade` (`2.0.0-alpha.1` a `2.0.0-alpha.14`); o ensaio geral passou da `0.6.0` para a `0.7.0`; os proporcionais, da `3.0.0` para a `2.0.0`; publicação na borda, quórum e Wasm, para a `1.x`. Mapa no [ROADMAP](../../ROADMAP.md).
* **Renumeração de 16/10/2026.** Entraram antes da `1.0.0` a comparação Urna Velox × TSE (etapa 7, `0.7.0`) e a previsão do matematicamente e do fim da apuração (etapa 8, `0.8.0`); o ensaio geral passou da `0.7.0` para a `0.9.0` (etapa 9). Itens de uma etapa posterior podem ser feitos em PATCH antes de ela começar (o ensaio anda em paralelo), mas os MINOR saem na ordem das etapas. Mapa no [ROADMAP](../../ROADMAP.md).
* **Uma evolução, uma versão (27/10/2026).** Cada evolução nova do roadmap ganha a sua própria versão (MINOR para as pequenas e compatíveis, MAJOR para as grandes) e não é juntada a uma versão já prevista sem decisão do Luciano. A ordem de execução entre as versões também é dele; se a ordem mudar, a numeração das que ainda não começaram pode ser refeita.
* **Usar o sistema numa eleição real não muda a versão** (ex.: o 2º turno de 2026 roda na `1.0.x`); correções feitas por causa dela são PATCH.
* Até a `0.0.19` o roadmap tinha 6 fases (Fase 1 = `0.1.0` … Fase 5 = `1.0.0`, proporcionais na `0.3.0`); a `0.0.20` reorganizou as versões como acima (mapa no [ROADMAP](../../ROADMAP.md)).
* **Fonte única:** o arquivo [`VERSION`](../../VERSION). Os manifestos acompanham: `aquisicao/Cargo.toml` (o `uv-coletor --version` e o `User-Agent` do coletor mostram essa versão), `bench/rust/Cargo.toml`, `bench/js/package.json` e os locks correspondentes.
* **Tag anotada só nas versões "zeradas"** (`vX.Y.0`: etapa concluída ou MAJOR), com o assunto do commit; os PATCH (`X.Y.Z`, Z > 0) não têm tag — a versão de cada commit está no `VERSION` e no CHANGELOG. Desde 13/10/2026; até a `v0.3.0` todo commit ganhava tag, e as tags dos PATCH foram apagadas (local e no GitHub), ficando `v0.0.0`, `v0.1.0`, `v0.2.0` e `v0.3.0`.
* **[CHANGELOG.md](../../CHANGELOG.md):** uma entrada `## [X.Y.Z] — data` por versão, com o que mudou e, quando for o caso, qual item ou critério do roadmap foi atingido.
* **Mensagem de commit:** `área: resumo` (como no histórico), opcionalmente com a versão no corpo.

## Como fazer um commit

```bash
scripts/versao.sh bump patch "resumo do que mudou"     # ou: minor, ao concluir uma fase
# completar a entrada do CHANGELOG.md (o bump abre a entrada com o resumo)
git add -A
git commit -m "área: resumo"                            # o hook confere a versão (e cria a tag, se for X.Y.0)
git push --follow-tags                                  # envia o commit (e a tag, se houver)
```

Se uma fase for concluída: `scripts/versao.sh bump minor "Fase N concluída: …"`, marque a fase no ROADMAP e cite a versão.

## Proteções (hooks versionados)

`scripts/instalar_hooks.sh` (uma vez por clone) aponta `core.hooksPath` para [`.githooks/`](../../.githooks) e liga `push.followTags`:

* **pre-commit:** recusa o commit se `VERSION` não mudou em relação ao `HEAD`, se a tag da nova versão já existe, se algum arquivo de versão ficou fora do índice ou se `scripts/versao.sh verificar` acusar incoerência (manifestos, locks, entrada do CHANGELOG ainda com `PREENCHER`).
* **post-commit:** cria a tag anotada `vX.Y.0` no commit, só quando o PATCH é 0 (`scripts/versao.sh tag`).
* Exceção consciente (ex.: merge): `UV_SEM_BUMP=1 git commit …`.
