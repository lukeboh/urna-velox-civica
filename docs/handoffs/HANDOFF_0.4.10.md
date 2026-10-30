# Handoff — sessão de 14/10/2026 (0.4.10 a 0.4.12), etapa 5 (site de exemplo)

Para continuar e **finalizar a etapa 5** em outra sessão. Plano original:
`/mnt/d/git/lukeboh/urna-velox/docs/handoffs/HANDOFF_0.5.0.md`; relatório da etapa:
`/mnt/d/git/lukeboh/urna-velox/docs/relatorios/RELATORIO_0.5.0.md`. Último commit desta sessão: `0.4.12` (este handoff); o código
está na `0.4.11` (`23685a3`). Trabalho sempre em PATCH 0.4.x — **não subir para 0.5.0 sem o Luciano pedir.**

## 1. Regras que continuam valendo

* Rede TSE: ≤ 100 req/s por órgão e por IP de saída; depois de qualquer 429, parar ≥ 10 min; nada de vários IPs,
  proxies ou processos do mesmo órgão. Site real só `preparar --pleito 3221` a ≤ 5 req/s, 1×/dia até 24/10.
  **Nada contra o site real em 25/10 e 26/10.**
* Motor sem rede e sem tokio; executáveis começam com `uv-`.
* Commit: `scripts/versao.sh bump patch` (sem resumo na linha de comando); CHANGELOG sem `PREENCHER`; `git add`
  (desde a `0.4.13` a `aquisicao/acervo/` é versionada, a pedido do Luciano — a regra antiga de deixá-la fora caiu);
  `git commit -F D:\git\lukeboh\urna-velox\.git\UV_COMMIT_MSG` (arquivo escrito com a ferramenta
  de escrita e relido); `git push --follow-tags`; identidade Luciano Bohnert <luciano.bohnert@gmail.com>.
* Antes de commit que mexe no serviço: `compilar_dev` sem avisos, motor `--ignored` 13/13, serviço `--ignored` 9/9.
  Números medidos não podem piorar sem explicação. Documentos novos com caminhos absolutos.
* Outra sessão também commita neste repositório: `git fetch` + `git log` antes de numerar/commitar.

## 2. O que esta sessão entregou (tudo commitado e enviado)

| versão | o quê |
|---|---|
| `0.4.9` | replay como um vídeo (linha do tempo com retotalização local, até 300×), seções agregadas fora do total |
| `0.4.10` | curva da apuração com o histórico da noite (`GET /api/v1/curva/<topico>`, PROTOCOLO_WS §6) e escala própria dos candidatos (as linhas pareciam constantes); painel **"O que já passou"** (`site/src/historico.js`): histórico do letreiro com filtro por categoria/texto e slider próprio; pendência do atraso no ROADMAP (etapa 6) |
| `0.4.11` | curva completa depois de voltar no tempo (a publicação a cada 2 min da retotalização nunca saía por estouro de `i64` em `reconstruir`; agora `saturating_sub`; gravador manda todas as versões à difusão e `amostrar_curva_superada` fecha os pontos); legenda da curva com reticências no celular. Verificado: `compilar_dev` 0 avisos, motor `--ignored` 13/13, serviço `--ignored` 9/9 (476 s) |

Fora do repositório:
* `candidatos.json` regerados **offline** (sem baixar nada) com o campo `agregada`, backup
  `candidatos.antes-0.4.9.json` ao lado de cada um: `/home/dev/uvreal/preparo_2026_1t`,
  `/home/dev/uvreal/2026-2t/preparo` (**incompleto: falta o `cs.json` de TO**; gerado com
  `--ignorar-destinacao --cargos 1,3,11,25,26,27`), `/home/dev/uvreal/2024-1t/preparo`, `/home/dev/uvreal/2024-2t/preparo`.
* Vídeos de 10 s (desktop 1440×900 e celular 390×844), simulação a 300×: `D:\git\lukeboh\urna-velox\.git\uv_video\`
  (`desktop.mp4`, `celular.mp4`).

## 3. O que falta para finalizar a etapa 5

1. **Medir de novo BU → publicação com 0 clientes** (RELATORIO §5.1): a `0.4.11` mudou o gravador (clona nome e
   `Bytes` de cada versão para a difusão quando há `--http`) e a difusão (lê o `ultimo_bu` das versões superadas).
   Repetir o B com `uv_tmp_e5_base.sh` (rodadas alternadas `v0.4.0` × atual, núcleos separados) e registrar no §5.1.
   Se o p99 piorar além do ruído (8–18 ms já visto entre rodadas do mesmo binário), mandar as versões só na
   retotalização ou amostrar a curva no próprio `reconstruir`.
2. **Retotalização da noite inteira** depois da 0.4.11: medido só até 18:30 (1,8 s, igual ao de antes); medir também
   a noite toda (antes 5,8 s) e anotar no RELATORIO §2.1.
3. **Validação independente** (RELATORIO §8): diff desde `v0.4.0`, `compilar_dev`, motor e serviço `--ignored`,
   ensaios E1/E2 (`uv_tmp_e5_ensaio.sh`) e conferências, página no Chrome do Windows durante um replay (CLS, fps,
   aba Network: agora também `/api/v1/curva/<topico>` e `/api/v1/replay` além de `/`, `/static/*`,
   `/api/v1/nomes`, `/api/v1/topicos` e `/ws` — atualizar essa lista no §8), `kill -9` no serviço e a página
   voltar sozinha. Só depois, **se o Luciano pedir**, `bump minor` → `0.5.0`.
4. RELATORIO §6 (desempenho da página): acrescentar o painel do histórico e a curva semeada (sem medida de fps
   ainda com o painel aberto; o CLS medido foi 0 em desktop claro/escuro e celular).
5. Pendências que ficam para depois (já registradas): ROADMAP etapa 6 "Atraso disponibilização → BU aplicado";
   `cs.json` de TO do 2º turno de 2026 (só com rede, respeitando as regras do §1).

## 4. Como testar (ferramentas desta sessão, em `D:\git\lukeboh\urna-velox\.git\`)

* Build (página + `compilar_dev`, ~1–2 min): `wsl -e bash /mnt/d/git/lukeboh/urna-velox/.git/uv_tmp_e5_build.sh`;
  esperar/ver resultado: `uv_tmp_esperar_build.sh` (log `/home/dev/uvreal/etapa5/logs/build.log`).
* Simulação na porta 18080: `wsl -e bash /mnt/d/git/lukeboh/urna-velox/.git/uv_tmp_sim.sh [velocidade]` (para a
  anterior e espera o roteiro antigo limpar a pasta). Página: `http://localhost:18080/#/br/c0001`.
* Voltar no tempo e listar a curva: `uv_tmp_cv_volta.sh [t]` (padrão 18:30; `t` em segundos civis, ex. 21:00 =
  1791145800) e `uv_tmp_cv_volta2.sh` (curva a cada 2 s logo depois do reinício).
* Fotos e CLS no Chrome com GPU: `node D:\git\lukeboh\urna-velox\.git\uv_tmp_cv_cdp.mjs [curva|hist|uma]`
  (fotos em `uv_cv\`); vídeos: `node ...\uv_tmp_video.mjs` (+ `uv_tmp_video_ver.sh` para duração e folha de contato).
* Verificação completa: `wsl -e bash /mnt/d/git/lukeboh/urna-velox/.git/uv_tmp_verif0410.sh` (para a simulação,
  motor `--ignored` → `logs/motor.log`, serviço `--ignored` → `logs/servico_ignored_0410.log`; ~10 min).
* Chrome para os testes: **porta 9333** (instância separada, sem conta):
  `chrome.exe --remote-debugging-port=9333 --user-data-dir=C:\Temp\uv-chrome-9333 --disable-backgrounding-occluded-windows
  --disable-renderer-backgrounding --disable-background-timer-throttling
  --disable-features=CalculateNativeWinOcclusion --force_high_performance_gpu`. A **porta 9222** é o Chrome do
  Luciano.
* Cuidados: `pkill -f`/`pgrep -f` dentro de `wsl -e bash -c "..."` acham o próprio comando (usar scripts); chamadas
  ao computador demoram > 60 s às vezes (o comando continua rodando: conferir antes de repetir); o screencast do
  Chrome entrega os quadros do celular em 1× (390 px), não em 2×.
