# Painel dos nós (`uv-painel`)

Página de monitoramento e gerenciamento dos nós da federação, em `https://<domínio>/painel/`, com login do Google e
acesso só para os e-mails autorizados.

## O que mostra e o que deixa fazer

* **Visão geral** (atualiza a cada 5 s), um cartão por nó:
  * **Situação:** coletando ou ocioso; a `saude.json` parada ou atrasada; pausa por HTTP 429; painel do nó fora do
    alcance.
  * **Coleta e totalização:** BUs aplicados/s, seções apuradas (% do catálogo), BUs baixados/s, fila e pendentes,
    requisições/s contra o teto, HTTP 429 e hora do último BU.
  * **Recursos:** CPU e memória do `uv-servico`, CPU, memória e carga da máquina, disco livre e clientes WebSocket
    (com mensagens/s e KB/s).
  * **Federação:** BUs recebidos dos pares/s, BUs publicados aos pares/s, lotes copiados/s, nós ativos e fora da
    partição (com o motivo) e a cota de req/s do nó.
  * **Alertas recentes** do serviço.
* **Gráficos** dos dois nós sobrepostos, com janela de 15 min, 1 h ou 2 h. São oito: BUs aplicados/s, BUs baixados/s,
  req/s com o teto, BUs recebidos dos pares, BUs publicados aos pares, memória do serviço, CPU da máquina e clientes
  WebSocket. O histórico fica na memória do painel (2 h) e recomeça quando ele reinicia.
* **Configuração** de cada nó:
  * **`no.env`:** pleito, ciclo, ambiente, servidor, req/s, UFs, aceleração, `candidatos.json` e ordem. O que define
    a identidade do nó fica só leitura: `NO_ID`, porta da federação e `EXTRA`.
  * **Serviço:** iniciar, parar ou reiniciar o `urna-velox`.
  * **Atualizador:** travar ou destravar (`controle-manual`).
  * **Leitura:** estado das unidades systemd, o `federacao.json` (só leitura) e a auditoria do nó.
* **Detalhe:** a `saude.json` inteira do nó.
* **Endereços do site de resultados** (na aba Configuração, por nó): veja o "Endereços do site de resultados" abaixo.

Confirmações por frase:

* Mudar pleito, ciclo, ambiente, servidor ou req/s pede `CONFIRMO`.
* Se o servidor (`BASE`) for o TSE, pede `CONFIRMO TSE`. Vale também para iniciar ou reiniciar um nó que coleta do
  TSE. É o lembrete das regras do `CLAUDE.md`: no máximo 100 req/s por órgão e nenhuma coleta durante a apuração sem
  combinar.
* O painel recusa req/s acima de 100.

## Endereços do site de resultados

O TSE pode mudar o endereço do site ou a organização dos arquivos. O painel deixa trocar isso sem mexer no código.

* **De onde o nó coleta:** a `BASE` do `no.env` (o servidor) e os **modelos de caminho** do
  `/etc/urna-velox/rotas_tse.json`. Sem esse arquivo valem os caminhos de sempre, idênticos aos de antes (há teste).
  São seis modelos, com variáveis entre chaves:

  | Modelo | O que é | Padrão |
  |---|---|---|
  | `ele_c` | lista de pleitos | `{base}/{amb}/comum/config/ele-c.json` |
  | `cs` | seções da UF | `{base}/{amb}/{ciclo}/arquivo-urna/{pleito}/config/{uf}/{uf}-p{pleito06}-cs.json` |
  | `aux` | arquivos da seção | `…/dados/{uf}/{mun05}/{zona04}/{secao04}/p{pleito06}-{uf}-m{mun05}-z{zona04}-s{secao04}-aux.json` |
  | `arquivo` | o BU | `{base_arquivos}/{amb}/{ciclo}/arquivo-urna/{pleito}/dados/{uf}/{mun05}/{zona04}/{secao04}/{hash}/{nome}` |
  | `mun_cm`, `u_json` | municípios e resultado oficial (comparação) | como antes |

* **Buscar os BUs de outro endereço:** `base_arquivos` troca só o servidor dos BUs (um espelho, uma CDN). As listas
  de seções e os `aux.json` continuam vindo da base.
* **Descobrir e testar:** você cola a URL (a base ou o endereço da página do site) e o painel, **a partir do nó
  escolhido**, segue estes passos:
  1. acha a base, procurando o `ele-c.json` em prefixos da URL;
  2. confere o pleito (sem ele, mostra a lista dos pleitos do servidor);
  3. lê o `cs.json` de uma UF pequena;
  4. numa seção com BU publicado, lê o `aux.json`, monta o endereço do BU e o baixa (envelope BER);
  5. com outro endereço para os BUs, baixa o mesmo BU também pela base e compara os bytes.

  São no máximo 20 requisições, uma a cada 350 ms (≈ 3 req/s, que somadas às do serviço ficam abaixo de 100 req/s
  por órgão). A descoberta para no primeiro HTTP 429 e não roda com o serviço em pausa por 429. Só acontece quando
  você clica.
* **Antes da apuração ainda não há BU publicado** do pleito novo: a descoberta acha o servidor e as seções e avisa.
  Para testar o caminho dos BUs na hora, use um pleito já apurado (o campo "Pleito" serve só ao teste).
* **Usar estes endereços** grava a `BASE` (e ambiente, ciclo e pleito achados) no `no.env` e o `rotas_tse.json`, num
  nó ou nos dois. Pede `CONFIRMO`, ou `CONFIRMO TSE` quando o servidor é o TSE. Os caminhos novos valem depois de
  reiniciar o `urna-velox` (há uma opção para isso). O painel avisa quando o serviço ainda roda com caminhos
  diferentes dos gravados.
* **Recusado sempre** (no painel, no `uv-painel-acao`, que chama `uv-painel validar-rotas`, e no `uv-servico` ao
  iniciar):
  * modelo que não comece por `{base}/` (o do BU pode começar por `{base_arquivos}/`). O servidor vem sempre da
    base configurada, nunca de um endereço escrito no modelo;
  * variável desconhecida ou faltando (por exemplo, `aux` sem `{secao04}`), `..`, `?`, `#`, espaço;
  * `base_arquivos` local com base remota, ou o contrário (a aceleração do simulador nunca vale contra o TSE).

## Limite de requisições: borda pública × servidor espelho

O limite de 100 req/s por órgão (e a pausa de 10 min depois de um 429) é da **borda pública** do TSE
(`resultados.tse.jus.br`, `cdn.tse.jus.br`). Um **servidor espelho** dos arquivos públicos (fora da borda pública; modo `interno`) não tem
esse limite. Na aba Configuração, campos do `no.env`:

| Campo | Valores | Efeito |
|---|---|---|
| `ACESSO` | `borda` (padrão, vazio) ou `interno` | como o nó chega ao servidor |
| `TETO_RPS_INTERNO` | número; `0` ou vazio = **sem limite** | teto do modo espelho |
| `RPS` | req/s pedidas | na borda, até 100; no interno, até o teto |

Regras de segurança, no painel, no `uv-painel-acao` e no `uv-servico`:

* **Modo espelho recusado** quando a base ou a base dos arquivos de urna é a borda pública. Também recusado com o
  simulador local, que já não tem o teto.
* **Na borda**, RPS acima de 100 é recusado (menos contra o simulador local, como sempre foi).
* **Mesmo sem limite**, a concorrência (`--concorrencia`, 64) continua segurando o ritmo. Um 429 também para tudo
  por 10 min no modo espelho.
* **Na federação**, a cota de cada nó deixa de ser a fatia do IP da borda: vale o teto do próprio nó.
* **Confirmação:** mudar `ACESSO`, `TETO_RPS_INTERNO` ou `RPS` pede a frase de sempre (`CONFIRMO TSE` quando o
  servidor é do TSE).

Passa ao serviço como `uv-servico --acesso interno --teto-interno N` (o `uv-iniciar.sh` monta). A `saude.json`
mostra `rede.acesso`.

## Segurança dos controles do serviço

O que **muda** o serviço fora do painel exige o mesmo login do Google: qualquer `POST`, `PUT`, `PATCH` ou `DELETE`
em `/api/*`. Hoje são os controles do replay (`/api/v1/replay/velocidade` e `/api/v1/replay/posicao`; para trás, o
`posicao` apaga a saída e recomeça a totalização). Antes, eles só conferiam a origem quando o navegador mandava o
cabeçalho `Origin`, e um `curl` de qualquer lugar passava.

Como funciona: o Caddy pergunta ao `uv-painel` do nó (`forward_auth` em `/painel/auth/verificar`). Sem sessão
válida, responde 401 e nada chega ao serviço. Com sessão, repassa o e-mail em `X-Uv-Operador` (um valor mandado pelo
cliente é sobrescrito). O pedido aceito ou recusado vai para a auditoria. Para isso, o cookie de sessão vale no site
todo (`Path=/`, continua `HttpOnly` e `SameSite=Lax`). As leituras (`GET /api/*`, `/ws`, a página) continuam
públicas.

Conferido com o Caddy 2.10 e serviços falsos:

* `GET /api/v1/estado` passa;
* `POST` de replay sem login ou com cookie forjado dá 401;
* com login passa, com o e-mail do painel;
* `DELETE` sem login dá 401;
* `/painel/` vai ao painel.

Fora do alcance do Caddy:

* **Porta 8080 do serviço:** aberta só para o par, na rede privada. Se o balanceador da OCI do
  `03_criar_load_balancer_oci.sh` for usado, ele fala direto com a 8080 e **pula o login**. Aponte-o para o Caddy
  (443) ou não o use.
* **Federação (7700):** só a rede privada e o IP do par. A assinatura Ed25519 dos lotes está prevista na etapa C.
* **Simulador (`/controle`):** escuta só em 127.0.0.1.

## Como funciona

```
navegador ──443──▶ Caddy (nó X) ──/painel*──▶ uv-painel (nó X, 127.0.0.1:8095)
                                                 │  lê: saude.json, /proc, no.env, federacao.json
                                                 │  muda: sudo uv-painel-acao <ação nomeada>
                                                 └──rede privada:8095 (HMAC)──▶ uv-painel (nó Y)
```

* **Um `uv-painel` por nó, separado do `uv-servico`.** Ele só lê arquivos (a `saude.json` que o serviço já grava e o
  `/proc`): não abre conexão com o serviço nem disputa a thread do motor. Uma queda do painel não atinge a coleta.
* **O Caddy manda `/painel*` sempre para o painel do próprio nó** (não balanceia). O painel mostra os dois nós:
  busca a visão do outro pela rede privada, na porta 8095 (a 8090 é da playlist de simulados). A porta só abre para o IP privado do par, no `iptables` e
  na Security List.
* **Pedidos entre os painéis são assinados** com HMAC-SHA256 (método, caminho, instante e SHA-256 do corpo; janela
  de 60 s). O segredo é comum aos dois nós (`UV_PAINEL_SEGREDO`). Uma mudança feita pelo painel do A no nó B leva o
  e-mail de quem pediu para a auditoria do B.
* **Login com o Google** (OpenID Connect, fluxo de código):
  * O `state` e o `nonce` vão num cookie assinado de 10 min.
  * O `code` é trocado no endpoint de token do Google, por TLS. O `id_token` que volta é conferido: emissor,
    audiência (o client_id), validade, `nonce` e `email_verified`.
  * O e-mail tem de estar em `UV_PAINEL_EMAILS`.
  * A sessão é um cookie assinado de 12 h (`HttpOnly`, `Secure`, `SameSite=Lax`, `Path=/painel`). Nada fica gravado
    no servidor.
  * Tirar um e-mail da lista derruba a sessão dele na hora.
  * O mesmo segredo nos dois nós faz o login valer em qualquer um.
* **Tudo o que muda algo** exige três coisas: a sessão, o cabeçalho anti-CSRF `X-UV-CSRF` (derivado da sessão) e a
  origem igual à do painel. A página roda com uma CSP sem nada inline.
* **Mudanças como root, só por ações nomeadas.** O painel roda como o usuário `uvpainel`. O sudo dele vale para um
  único programa, o `/opt/urna-velox/bin/uv-painel-acao`, que aceita só estas ações:
  * `aplicar-no-env <arquivo>`;
  * `servico iniciar|parar|reiniciar`;
  * `travar` e `destravar`.

  O `no.env` é lido pelo bash (`source`), então o arquivo passa por duas conferências:
  * **No painel** (`config_no.rs`): só chaves editáveis, regra por campo e uma lista fechada de caracteres (sem
    espaço, aspas, `$`, crase ou `;|&<>()`).
  * **No `uv-painel-acao`**, de novo, antes de instalar:
    * cada linha nova ou mudada tem de seguir o formato;
    * nenhuma chave não editável pode entrar ou sair;
    * o arquivo tem de passar no `bash -n`;
    * a cópia do anterior vai para `/etc/urna-velox/historico/`.
* **Auditoria:** `/srv/urna-velox/painel/auditoria.jsonl` registra logins, recusas, mudanças com o antes e o depois,
  e ações. A auditoria também aparece no `journalctl -u uv-painel`.

## Implantação

1. **Cliente OAuth no Google Cloud.** Abra o console → APIs e serviços → Credenciais → Criar credenciais → ID do
   cliente OAuth → **Aplicativo da Web**.
   * URI de redirecionamento autorizado: `https://<URL_DOMINIO>/painel/oauth/retorno`. Para testar na estação,
     acrescente também `http://localhost:8090/painel/oauth/retorno`.
   * A tela de consentimento pode ficar em "Teste", com o seu e-mail como usuário de teste. O painel só pede
     `openid email`.
2. **No `deploy/oci/nos.env`** (fora do git):
   ```bash
   URL_DOMINIO=urnavelox.com.br
   PAINEL_GOOGLE_ID=<id>.apps.googleusercontent.com
   PAINEL_GOOGLE_SEGREDO=<segredo do cliente>
   PAINEL_EMAILS=<seu e-mail>[,<outro>]
   ```
3. **Compilar e instalar:**
   ```bash
   ./compilar_local_arm64.sh
   ./80_painel.sh
   ```
   O `80_painel.sh` faz quatro coisas:
   * gera o segredo comum em `gerado/painel.segredo`;
   * instala o painel nos dois nós (usuário, sudo, porta 8095 para o par, `uv-painel.service`);
   * chama o `02_configurar_borda.sh`, que confere a Security List da sub-rede dos nós (`uv_oci.py liberar-portas`: só
     acrescenta as portas que faltam; para com erro se não conseguir) e atualiza o Caddy;
   * confere, de cada nó, o painel do par na 8095 (sem isso a visão geral mostra o outro nó "fora do alcance", B-019);
   * reinicia o Caddy (a página pública pisca por um instante).

   O `urna-velox.service` não é tocado.

Para incluir ou tirar alguém, edite `PAINEL_EMAILS` e rode `./80_painel.sh --sem-borda`.

## Teste na estação (modo local)

Sem `--acao`, o painel trabalha em modo local: grava o `no.env` direto e não mexe em serviço nenhum.

```bash
export UV_PAINEL_GOOGLE_ID=... UV_PAINEL_GOOGLE_SEGREDO=... UV_PAINEL_EMAILS=<seu e-mail> \
       UV_PAINEL_SEGREDO=$(openssl rand -base64 48)
uv-painel --no no-local --escuta 127.0.0.1:8090 --url-publica http://localhost:8090 --acao "" \
  --saida <pasta com a saude.json de um ensaio> --no-env /tmp/no.env --federacao /tmp/federacao.json \
  --estado /tmp/uv-painel
```

## Testes

* `cargo test --release -p uv-aquisicao painel`:
  * HMAC (vetores da RFC 4231);
  * sessão, login e CSRF;
  * `id_token`;
  * `no.env` (inclusive tentativas de injeção);
  * amostras e taxas.
  * Um teste de ponta a ponta sobe um "Google" falso e dois painéis que se enxergam. Ele cobre:
    * login aceito e recusado, e `state` errado;
    * a sessão do A valendo no B e a visão dos dois nós;
    * mudança no B pelo A, com CSRF, origem, confirmação, teto de 100 req/s e injeção;
    * rotas internas sem assinatura;
    * par fora do ar.
* `bash deploy/oci/testes/test_painel_acao.sh`: o `uv-painel-acao`, sem root. Cobre mudança válida e histórico,
  injeções, chaves proibidas, arquivo fora da pasta, link simbólico e as ações de serviço e trava.

## Rotas

| Rota | O quê |
|---|---|
| `GET /painel/` | página (sem sessão: a de entrada) |
| `GET /painel/entrar`, `GET /painel/oauth/retorno`, `POST /painel/sair` | login e saída |
| `GET /painel/api/eu` | e-mail, validade e token anti-CSRF |
| `GET /painel/api/visao?desde=<ms>` | os nós: resumo e série a partir de `desde` |
| `GET /painel/api/no/<id>/saude` | `saude.json` do nó |
| `GET` e `POST /painel/api/no/<id>/config` | `no.env` e serviços; `{"alteracoes": {...}, "reiniciar": bool, "confirmacao": "..."}` |
| `GET` e `POST /painel/api/no/<id>/rotas` | caminhos do site; `{"base", "rotas": {...}\|null, "AMBIENTE", "CICLO", "PLEITO", "reiniciar", "confirmacao"}` |
| `POST /painel/api/no/<id>/descobrir` | `{"url", "pleito", "ambiente", "base_arquivos", "rotas"}` → passos e proposta |
| `POST /painel/api/no/<id>/servico` | `{"acao": "iniciar\|parar\|reiniciar\|travar\|destravar", "confirmacao": "..."}` |
| `GET` e `POST /painel/interno/<visao\|saude\|config\|servico\|rotas\|descobrir>` | entre os painéis (assinatura HMAC no `x-uv-painel-par`) |
| `GET /painel/vivo` | "ok" (sem sessão) |

Respostas `409` trazem `confirmacao`, a frase que a ação pede.
