# Handoff — Urna Velox 2.0: módulo de escalabilidade segura (`2.0.0-alpha.N`)

> **Renumeração de 15/10/2026.** Este documento era o `docs/HANDOFF_2.0.0.md`. A escalabilidade segura virou a
> **etapa 6 (`0.6.0`)**, trazida da branch para a `main` e fechada na `v0.6.0` com a etapa A (federação) entregue nos
> ensaios reduzidos e o critério A completo pendente, que passou ao ensaio geral (etapa 9 desde a
> renumeração de 16/10, [HANDOFF_0.9.0.md](/mnt/d/git/lukeboh/urna-velox/docs/handoffs/HANDOFF_0.9.0.md)). As etapas B a E (borda, assinatura e
> quórum, Wasm/SPA, ensaio da antiga 2.0) ficaram para a `1.x`. Não há mais versões `2.0.0-alpha.N` nem branch em
> paralelo: o trabalho segue na `main`, em PATCH. "3.0" abaixo = cargos proporcionais, hoje `2.0.0`. Mapa no
> [ROADMAP](/mnt/d/git/lukeboh/urna-velox/ROADMAP.md).

*Preparado em 14/10/2026, a pedido do Luciano, para adiantar a 2.0 em paralelo à reta final da 1.0. **Ponto de
partida: `v0.4.3`** (`main`, commit `83fe5e7`). Para iniciar uma sessão, cole este documento ou peça para lê-lo.
**Quem executa trabalha só na branch `feature/2.0-escalabilidade`, sobe `2.0.0-alpha.N` a cada commit e entrega um
relatório por etapa; a `2.0.0` (com tag) só quando a branch voltar para a `main`, depois da validação independente
descrita no fim.***

**Caminhos.** Branch num diretório próprio (*git worktree*), separado do checkout da `main`:

| | Windows (Git, PowerShell) | WSL `Ubuntu-24.04` (scripts, cargo) |
|---|---|---|
| 2.0 (esta branch) | `D:\git\lukeboh\urna-velox-v2` | `/mnt/d/git/lukeboh/urna-velox-v2` |
| 1.0 (`main`) | `D:\git\lukeboh\urna-velox` | `/mnt/d/git/lukeboh/urna-velox` |
| dados | — | `/home/dev/uvreal` (ensaios da 2.0 em `/home/dev/uvreal/v2`) |

* **Faça `git commit`/`push` pelo Git do Windows.** O worktree usa caminho relativo para o `.git`; o git do WSL (2.43)
  lê, mas lista o worktree como *prunable*: **nunca rode `git worktree prune` no WSL**. Não ligue
  `--relative-paths`/`extensions.relativeWorktrees` (o git 2.43 do WSL deixa de abrir o repositório).
* **Compile com `target` próprio** (`CARGO_TARGET_DIR=/home/dev/uvreal/target-v2`), para não disputar o `target` da
  `main` nem o binário de uma coleta em andamento.
* **A `main` tem prioridade até 25/10.** Nada desta branch roda contra o site do TSE antes do 2º turno, e nada
  aqui pode atrasar a 1.0. Traga a `main` para cá (`git merge main`) depois de cada `X.Y.0` dela; o conflito em
  `VERSION`, manifestos e CHANGELOG fica com a alpha ([VERSIONAMENTO.md](../processo/VERSIONAMENTO.md)).

## Por que a 2.0 agora

O Luciano quer a 2.0 adiantada porque **é ela que permite superar a disponibilização oficial**. O benchmark já mostrou
onde está o gargalo:

* **O motor não é o gargalo:** do BU recebido ao resultado publicado, ~5 ms.
* **O gargalo é a aquisição.** São 2 requisições por seção (`aux.json` + BU). A 100 req/s cabem ~2.850 BUs/min, contra
  um pico de chegada de ~6.500 BUs/min (horários de recebimento dos `aux.json`): a fila cresce e o Urna Velox fica
  dezenas de minutos atrás no pico; acompanhar o pico sem fila pede mais que o dobro da vazão de um órgão.
* **A entrega também tem limite:** um `uv-servico` atende milhares de conexões WebSocket (10.000 testadas, p99
  35,6 ms), não centenas de milhares de navegadores.

A 2.0 ataca os dois lados: **coleta dividida entre nós** (ADR-022) e **entrega por arquivos estáticos na borda de uma
CDN** (ADR-021), com **assinatura e quórum** (ADR-023) para que vários operadores possam colaborar sem que um deles
consiga adulterar o resultado, e o **motor em Wasm** no navegador (ADR-024/025).

## ⚠️ Regra de rede da federação: 100 req/s por órgão, contados por IP de saída

Regra do projeto (14/10, no [CLAUDE.md](../../CLAUDE.md), "Rede e servidores do TSE"): cada **órgão** acessa pelo seu
IP e não passa de **100 req/s** (acima disso, HTTP 429); órgãos diferentes podem fazer consórcio e colaborar.
A CDN do TSE conta pelo **IP de saída**: dois nós que saem pelo mesmo IP são, para ela, um só. Daí, nesta branch:

1. **Um nó por órgão.** O `federacao.json` assinado lista cada nó com o **órgão**, a chave Ed25519 e o **IP de saída
   declarado**; o código recusa dois nós do mesmo órgão. Vazão da federação = soma dos órgãos (K × 95 req/s nas
   coletas longas): com 2 ou 3 órgãos a coleta acompanha o pico.
2. **Cada nó mede e publica o seu IP de saída** — o que a CDN vê, não o da interface local — no *heartbeat*, e todo nó
   conhece o dos outros. Medida: um endpoint de eco da própria federação (ex.: um Worker da Cloudflare que devolve o
   `CF-Connecting-IP`), repetida a cada poucos minutos (o IP pode mudar durante a noite). IP medido diferente do
   declarado → o nó **não coleta** (só recebe dos pares) e publica um alerta.
3. **Nós com o mesmo IP de saída formam um grupo e dividem 100 req/s** (95 nas coletas longas), recalculado a cada
   mudança de *heartbeat*. Para a partição continuar proporcional à capacidade, o HRW é **ponderado** pelo req/s de
   cada nó (*weighted rendezvous hashing*): um nó com metade do teto recebe metade das seções.
4. **429 pausa o IP, não a federação.** Um 429 põe o IP numa "caixa de penalidade": todos os nós desse IP param por
   ≥ 10 min (como hoje), marcam `em_pausa` no *heartbeat* e **saem da partição**, e as seções deles passam aos outros
   IPs. Os outros continuam, cada um no seu teto. Spot-checking (REQ-ESC-12) e sondagens contam no teto do IP de quem
   as faz.
5. **Tetos no código, não só na configuração**, como hoje em `coletor::limites`: contra o site real, nenhum nó passa de
   100 req/s, qualquer que seja o `federacao.json`; acima disso, só com um teto acordado com o TSE para aquele órgão,
   registrado na configuração e no relatório.
6. **Contra o site real, nada da 2.0 roda antes de 26/10** (noite e dia seguinte do 2º turno), e qualquer teste real é
   combinado antes com o Luciano. Todo o desenvolvimento usa o `uv-simulador`.

**No simulador, o IP de saída precisa existir de verdade.** Cada nó local sai por um endereço de *loopback* próprio
(`127.0.0.2`, `127.0.0.3`…, o Linux aceita toda a faixa `127/8`). O `uv-simulador` passa a registrar o IP de origem de
cada requisição e a **impor o limite por IP** (429 acima de 100 × aceleração por IP, com a caixa de penalidade), para
que os ensaios provem a regra em vez de só confiar nela. Ensaio obrigatório: dois nós no mesmo IP dividindo o teto.

A [ESCALABILIDADE_SEGURA.md](../arquitetura/ESCALABILIDADE_SEGURA.md) (REQ-ESC-08) foi ajustada nesta alpha (um nó por órgão, IP de
saída conhecido, grupo por IP). Os caminhos que **reduzem** requisições por BU continuam valendo como investigação:
1 requisição por seção se o `hash` estiver disponível sem o `aux.json`; BUs em lote/fluxo. Cada um
dobra (ou mais) a capacidade de cada órgão.

## Leia antes de começar (nesta ordem)
1. `CLAUDE.md` — regras permanentes (versionamento, rede, verificação). Valem aqui sem exceção.
2. `docs/arquitetura/ESCALABILIDADE_SEGURA.md` — a especificação (REQ-ESC-01 a 17) — e as ADR-021 a 025 em
   `docs/arquitetura/ARCHITECTURE_DECISIONS.md`; depois a ADR-026 (WebSocket com último valor por tópico, REST já com
   `ETag`/304 pensado para ficar atrás de uma CDN) e a ADR-020 (simulador).
3. `docs/arquitetura/AQUISICAO.md` (URLs, `cs.json` → `aux.json` → BU, o `<hash>` do diretório = SHA-256 do `.vsc`), `docs/relatorios/RELATORIO_0.3.0.md`
   e `docs/relatorios/RELATORIO_0.4.0.md` (como o serviço e a difusão foram medidos).
4. Código: `aquisicao/src/servico.rs` (fila de seções, `aplicar_bu`, diário `secoes.log`, publicação),
   `aquisicao/src/coletor.rs` (limitador, AIMD, disjuntor), `aquisicao/src/tse.rs`, `aquisicao/src/simulador.rs`,
   `aquisicao/src/difusao.rs`, `motor/src/pacote.rs` (`.uvpk`), `motor/src/estado.rs`, `motor/src/resultado.rs`, `site/`.

## Vários órgãos na infraestrutura do operador (ADR-029, 15/10)

Vários órgãos podem ser hospedados na mesma infraestrutura. **Eles formam uma
federação** (colaboram na coleta: menos pressão na CDN do TSE e todos mais rápidos) e
**cada órgão tem máquina e IP público reservado exclusivos** — garantia **operacional**, para que nenhum órgão perca
vazão por dividir IP com outro. A divisão por IP no código fica só como rede de segurança e passa a gerar alerta grave
(a implementar na etapa A/C).

## Lacunas da especificação que esta branch precisa decidir (registrar em ADR nova, 028+)

1. **Cada nó só coleta 1/K das seções — de onde vêm as outras?** A especificação fala em "feeds com totais
   consolidados" por nó, mas um nó sozinho não tem o Brasil. **Decidido (ADR-028, revisto na `alpha.5`): troca de BUs
   direto entre os nós, sem storage compartilhado.** Quem coleta uma seção grava os bytes do BU (e o `<hash>`, o SHA-256
   do `.vsc`) em lotes só de acréscimo no seu diretório; cada nó serve o seu diretório por HTTP só de leitura e puxa dos
   pares o que falta (da origem ou, se ela não responde, de outro par: repasse). Os BUs vêm dos pares, não do TSE, sem
   custo no teto, e **todos totalizam o país inteiro** com o mesmo motor; o quórum compara totais independentes.
2. **"Zero-trust" depende de verificar a assinatura da urna.** Sem conferir o `.vsc` (ECDSA, cadeia ICP-Brasil), um nó
   malicioso pode servir um BU inventado e os outros o aceitariam. Isso é a "auditoria cívica" (ADR-011)
   que o ROADMAP pôs na `1.x`: **na 2.0 ela vira pré-requisito da troca de BUs** (pelo menos a verificação das
   assinaturas do BU; o `.vsc` tem ~62 KB e é mais uma requisição por seção — entra no teto, ou só para a amostra do
   spot-check; decidir e medir).
3. **Diretório local de cada nó** (`Armazem`: gravar, ler com `ETag`, listar por prefixo): os dados do próprio nó e as
   cópias dos pares; nada compartilhado. Isso vive no `uv-aquisicao`; o motor continua sem rede e sem `tokio`.
4. **Identidade e chaves dos nós:** `uv-federacao chave nova` gera Ed25519; a lista de nós autorizados (chave, órgão,
   IP de saída declarado, endereço de sincronização e teto de cada um, o quórum M de N e a URL do eco de IP) vai num `federacao.json` assinado pela chave do coordenador,
   embutido na SPA na hora do build (REQ-ESC-10).
5. **Relógio:** heartbeat de 10 s e morte em 30 s (REQ-ESC-05) medidos por cada nó localmente: o *heartbeat* muda a
   cada batida e a cópia local só é regravada quando muda; a idade dela é a idade do par (sem depender do relógio dele).

## Etapas da 2.0 (uma por vez; cada uma fecha com relatório e os critérios medidos)

Todas em `2.0.0-alpha.N`. Marque cada item no ROADMAP (seção `2.0.0`) com a alpha em que foi atingido.

### Etapa A — Federação no simulador (REQ-ESC-05 a 08)
- Diretório local por nó; heartbeat por nó (`/nos/<id>.json`, 10 s), lista de vivos, *Rendezvous Hashing* (HRW) sobre
  `(municipio, zona, secao)` com hash estável e documentado (ex.: SipHash-1-3 com chave fixa, ou SHA-256 truncado).
- O `uv-servico` ganha `--federacao <federacao.json> --no <id>`: só pede ao TSE as seções que lhe cabem; grava os BUs
  que coletou no seu diretório, serve-os aos pares e aplica no motor também os BUs que puxa dos outros nós.
- IP de saída medido e publicado no *heartbeat*, grupos por IP dividindo 100 req/s, HRW ponderado, 429 pausando só o
  IP (ver a regra de rede acima), conferidos no código como hoje (`coletor::limites`); simulador com limite e caixa
  de penalidade por IP de origem.
- Nó que morre: as seções dele passam aos vivos pelo segundo maior peso do HRW, sem reatribuir as outras (medir: fração
  movida ≈ 1/K, REQ-ESC-07).

**Critério A:** 3 nós no mesmo computador, cada um com seu `uv-servico`, contra o `uv-simulador` com a noite de 2026
a 60×: cada nó termina com **83/83 abrangências e 832/832 situações idênticas ao oficial** (conferência automática de
cada um); seções pedidas ao TSE somando os 3 = seções com BU (+ as do spot-check), nenhuma pedida por 2 nós fora da
amostra; `kill -9` de um nó no pico: seções dele reassumidas em ≤ 40 s, nenhuma perdida; **por IP de origem**, req/s ≤ teto
em todo segundo e 0 HTTP 429 (log do simulador, que agora impõe o limite por IP); com 2 nós no mesmo IP, os dois somados
≤ teto desse IP e a partição proporcional; 429 provocado num IP: só ele pausa, as seções dele passam aos outros e
voltam depois da pausa. Repetir com 2024 2º turno a 10× (51/51, 102/102).

**Estado da etapa A (`2.0.0-alpha.5`, 14/10).** Feito: `aquisicao/src/federacao/` (`config`, `particao`, `estado`,
`armazem`, `no`, `sincronia`), `uv-servico --federacao <json> --no <id> --armazem <dir local> [--federacao-escuta
<end:porta>] [--ip-local <ip>] [--eco-ip <url>]` — **sem storage compartilhado**: cada nó serve o seu diretório
(`GET /fed/v1/...`) e puxa dos pares, com repasse quando não alcança a origem;
`uv-simulador --limite-ip-rps` (limite e caixa de penalidade por IP de origem, `GET /eco-ip`, IP na 7ª coluna do
`simulador.log`, `por_ip` no resumo) e os ensaios reduzidos em `aquisicao/tests/federacao.rs` (2º turno de 2024, AM e
RO, 120×; vazão reduzida a 5 req/s da noite por IP para exercitar o limite): três órgãos; dois órgãos no mesmo IP;
IP divergente; `kill -9` de um nó; `kill -9` e reinício; repasse quando um nó não alcança outro — **6/6 passando**,
cada nó com 2/2 idênticos ao oficial e 0 HTTP 429.
Falta para fechar o critério A: a noite de 2026 a 60× com 3 nós e o `kill -9` no pico, e o 2º turno de 2024 inteiro
a 10×; o relatório `docs/RELATORIO_2.0.0-A.md`.

> **⚠️ Pendência de teste (15/10, `2.0.0-alpha.12`; decisão do Luciano: registrar e parar por ora).** O critério A completo
> **não tem número válido** com o código atual (`alpha.11`). O que se sabe:
> * **Rodadas de 15/10 que acharam defeitos** (2026 a 60×, 3 nós, `kill -9` do b com ~104 mil BUs e reinício 30 s depois):
>   código da `alpha.9` — cada nó 83/83 idêntico ao oficial, 0 duplicados no motor, 0 HTTP 429, pico ≤ 5.767 req/s por IP
>   (limite 6.000), queda vista em 4 s, mas **120.977 seções pedidas ao TSE por mais de um nó** (0 antes do `kill -9`; o b
>   repedia o que os outros coletaram); com a 1ª correção, **145.940**, por falsas mortes no pico. O 2º turno de 2024 a
>   10× passou nas duas rodadas: 51/51, 0 duplicatas, 0 HTTP 429. Defeitos e correções no CHANGELOG da `alpha.11`.
> * **Com a `alpha.11`** passaram unidade 40/40, federação 6/6 e nó único 9/9 (na RAM); o critério A completo não rodou
>   até o fim em nenhum disco desta máquina (abaixo).
> * **Onde rodar** (~45 GB no pico: por nó ~5,9 GB de pacotes + ~8,6 GB de lotes): dentro da distro enche o `ext4.vhdx`
>   do C: (incidente de 15/10), e a saída não deve ficar lá (regra do `CLAUDE.md`); o D: pelo `/mnt/d` tem latência por
>   arquivo que provoca falsas mortes (o mesmo ensaio passa em `/tmp` e falha lá); uma imagem ext4 no D: montada por
>   loop ficou 2× mais lenta e **travou** (o b ficou 40 min em E/S no estado D); e o `wsl --mount --vhd` exige
>   administrador da própria conta (o PowerShell de administrador desta máquina roda com outra conta, sem WSL2).
> * **Caminhos para retomar:** (1) a distro em modo esparso (`wsl --shutdown`; `wsl --manage Ubuntu-24.04 --set-sparse
>   true`), ensaio em `~/uvreal`, limpeza no fim e `fstrim` devolvendo o espaço ao C:; (2) os servidores Arm da OCI,
>   junto com a primeira implantação (depois da RC); (3) `wsl --mount --vhd` com administrador da própria conta.
>   Roteiros prontos em `D:\dados\uv2-scripts\` (`criterio_a.sh`, `duplicadas.py`, `resumo_a.py`); trocar o `W`/`HOST`
>   do `criterio_a.sh` para o disco escolhido.

Como rodar um nó (no simulador; contra o TSE, nada antes de 26/10):

```bash
uv-simulador ... --limite-ip-rps 100                 # limite por IP, como a borda do TSE
uv-servico ... --base http://127.0.0.1:<porta> --aceleracao-local 60 \
  --federacao federacao.json --no orgao-a --armazem /dados/federacao --ip-local 127.0.0.2
# federacao.json: cada nó com "endereco": "host:porta" (onde os pares buscam os dados dele)
```

### Etapa B — Publicação estática na borda (REQ-ESC-01 a 04)
> **Direção confirmada pelo Luciano em 15/10 (ADR-030): sem storage, como na etapa A.** Em vez de enviar os
> arquivos para um storage na Cloudflare (R2), **cada nó serve os próprios arquivos de resultado** (o `uv-servico` já
> serve `GET /api/v1/...` com `ETag`/304) e uma **CDN só faz cache na frente** (ex.: Cloudflare em modo proxy, TTL de
> poucos segundos). Nada para montar além de apontar um domínio para o nó e ligar o cache; a CDN busca no nó só quando a
> cópia expira, então o nó atende poucas requisições mesmo com milhões de leitores; e, como todo nó tem o resultado
> completo, a CDN pode ter vários nós como origem (se um cair, busca em outro). ADR-021 revista pela ADR-030 e
> REQ-ESC-01/02/04 ajustados na `alpha.8`. A origem fica exposta só pela CDN e só com os caminhos públicos de resultado
> (o `/fed/v1/...` continua só entre os pares). Domínio: um por órgão ou um só do
> operador.

- Cada nó serve os arquivos de resultado (os mesmos bytes do `FORMATO_RESULTADO.md`) com
  `Cache-Control: public, max-age=2, stale-while-revalidate=1`, `ETag` e compressão; um `indice.json` por ciclo
  (seq, seções apuradas, hash de cada arquivo), para o cliente saber o que mudou sem baixar tudo.
- Medida de REQ-ESC-02: lote totalizado → arquivo novo visível através do cache < 200 ms + TTL (p99). Sem conta de CDN,
  meça com um cache HTTP local na frente do nó (ex.: um proxy de cache), com vários nós como origem.
- Cliente de carga (`uv-carga --modo pull`): N clientes em *short-polling* com `If-None-Match`; medir 304/200 e a
  latência BU → cliente.

**Critério B:** noite de 2026 a 60× servida pelos nós através de um cache; últimas versões idênticas byte a byte ao
`publicacao/` local; lote → visível pelo cache p99 < 200 ms + TTL; derrubar um nó de origem sem o leitor perceber; 10.000 clientes *pull* sem nenhum efeito na
totalização (BU → publicação igual com 0 clientes).

### Etapa C — Assinatura, quórum e amostragem cruzada (REQ-ESC-09 a 13)
- Verificação das assinaturas da urna (lacuna 2), no motor ou num crate puro (sem rede), com testes sobre as fixtures
  reais; o BU trocado entre nós só entra no estado depois de verificado.
- Feed assinado por nó: Ed25519 sobre SHA-256 do `indice.json` + id do nó + `seq` + horário.
- Spot-check de 1 % das seções dos outros nós (dentro do teto); divergência gera alerta assinado, repassado aos pares.
- Quórum M de N verificado por um cliente de referência (Rust, e depois no Wasm).

**Critério C:** com 3 nós e quórum 2 de 3: um nó adulterado de propósito (`--adulterar` só em teste: troca votos de um
BU, ou publica totais falsos) é **detectado** (BU com assinatura inválida rejeitado pelos outros; totais dele fora do
quórum; alerta do spot-check) e o resultado exibido continua idêntico ao oficial; com chave fora da lista, feed
ignorado.

### Etapa D — Motor em Wasm e SPA/PWA (REQ-ESC-14 a 17)
- `urna-velox` compilando para `wasm32-unknown-unknown` (o que for nativo atrás de *features*: threads, arquivos),
  `wasm-bindgen`, < 2 MB comprimido, num Web Worker; memória < 60 MB na eleição nacional.
- O site da etapa 5 passa a ter um **modo borda**: lê o `indice.json` e os resultados da CDN, confere assinaturas e
  quórum (Web Crypto/Wasm), mostra "Conferido por quórum"; opcionalmente baixa BUs e refaz a conta no Worker. Mesmo
  desenho sem pulo (CLS = 0) da `0.4.x`. `manifest.json` e Service Worker.

**Critério D:** a SPA, servida como arquivo estático, acompanha a noite de 2026 a 60× pela borda e termina com os
números da conferência oficial (83/83), CLS 0, 60 fps no celular de referência; com o Worker refazendo a conta a
partir dos BUs, os mesmos totais.

### Etapa E — Ensaio da 2.0 e a comparação com o TSE
- Noite de 2026 a 1× (trecho) e a 60× (inteira) com 3 nós, borda e SPA, com as falhas da `0.3.0` (429, queda de nó,
  um nó que não alcança outro, reenvio, anulação).
- **A métrica que importa:** atraso **recepção no TSE → número visível no navegador**, por BU, p50/p90/p99, comparado
  com a divulgação oficial. Simular a aquisição com a federação (1, 2, 3 e 4
  órgãos; com 1 requisição por seção; com o atraso da sincronização entre nós e do polling do cliente) e medir no
  simulador com os horários de recebimento dos `aux.json`.

**Critério E (o da 2.0):** dois ensaios completos seguidos sem intervenção e sem divergência, com 3 nós em IPs de
saída diferentes, cada um a ≤ 100 req/s e 0 HTTP 429; **atraso recepção no TSE → navegador abaixo da divulgação
oficial no p50 e no p90** (meta: p90 < 60 s com 3 órgãos), e o relatório mostra quantos órgãos
são precisos para ficar abaixo do TSE.

## O que não pode mudar
* **Os números.** Toda etapa que mexe em totalização ou resultado passa pelas conferências automáticas (`cargo test
  --release -p urna-velox` e `UV_DADOS=/home/dev/uvreal cargo test --release -p urna-velox -- --ignored --test-threads 1`:
  2026 83/83, 2024 5.574/5.574 e 51/51), e o resultado vai no CHANGELOG.
* **O motor continua sem rede e sem `tokio`** (`cargo tree -p urna-velox -e normal` sem `tokio`/`hyper`/`axum`); o que
  for de federação, sincronização e criptografia de nó fica fora dele. A compilação para Wasm não pode mudar o resultado
  nativo (mesmos checksums).
* **O `uv-servico` sem `--federacao` funciona exatamente como na `main`** (os ensaios `--ignored` do serviço passam
  sem mudança): a 2.0 é um modo a mais, não uma troca.
* **Rede:** as regras do `CLAUDE.md` valem para a federação inteira (ver a restrição no começo).

## Versionamento nesta branch
* Primeiro commit (este handoff): `scripts/versao.sh definir 2.0.0-alpha.1`. Depois, cada commit:
  `scripts/versao.sh bump alpha` (no PowerShell, sem resumo na linha de comando; o resumo vai direto no CHANGELOG),
  completar a entrada, `git add` dos arquivos, `git commit` (o hook confere), `git push`.
* Alphas não ganham tag. Etapa concluída = item marcado no ROADMAP com a alpha e relatório
  `docs/RELATORIO_2.0.0-<etapa>.md` (ex.: `RELATORIO_2.0.0-A.md`).
* Ao trazer a `main` (`git merge main`, com `UV_SEM_BUMP=1` se o merge não tiver conflito de versão): `VERSION` e
  manifestos ficam com a alpha atual; as entradas da `main` entram no CHANGELOG abaixo da alpha mais recente.

## Validação independente e volta para a `main`
Cada etapa termina com relatório e números medidos. **Antes da `2.0.0`**, uma sessão que não participou do trabalho
refaz os critérios A a E a partir dos relatórios (mesmos comandos, mesmos dados) e registra o resultado. Só então a
branch volta para a `main`, com `scripts/versao.sh definir 2.0.0` (tag `v2.0.0`), ou por decisão explícita do
Luciano.

## Primeiro passo sugerido
1. Ler os documentos acima e escrever a ADR-028 (troca de BUs entre os nós, verificação da urna como pré-requisito,
   um nó por órgão com IP de saída medido, grupo por IP e HRW ponderado, 429 por IP) e ajustar a ADR-022 com isso.
2. Medir quanto custa verificar a assinatura de um BU (µs por BU, com as fixtures) e o tamanho de um `.uvpk` por nó a
   cada 2–5 s na noite de 2026 — números que decidem a lacuna 2 e o ritmo da troca.
3. Começar a etapa A pelo diretório local e pelo HRW, com testes de unidade (distribuição uniforme, fração movida ao
   entrar/sair um nó).
