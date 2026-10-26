# Checkpoint — plano de borda do 2º turno (para retomar de outra sessão)

*Atualizado pelo Claude a cada etapa. Plano: [HANDOFF_MODELO_BORDA_E_CONSISTENCIA.md](HANDOFF_MODELO_BORDA_E_CONSISTENCIA.md) §3.
Regras: [CLAUDE.md](../CLAUDE.md) (bugs: teste vermelho antes da correção; versão a cada commit). Um passo por vez;
nada em produção sem o ok do Luciano; a partir de 23/10 nenhuma mudança.*

## Onde estamos

**Passo 1 (B-026, modo HTTP na página)** — em andamento (20/10, noite).

- [x] Lidos CLAUDE.md, handoff de borda, §7 do handoff de carga, B-026.
- [x] Rotas REST conferidas (`difusao.rs:573–585`, `PROTOCOLO_WS.md` §6): `/api/v1/estado` (no-cache) e
      `/api/v1/resultado/{topico}` (ETag `"<seq>"`, 304, corpo = arquivo publicado, sem envelope) existem;
      `/api/v1/eventos` e `/api/v1/marcos` existem mas ficam **fora** do modo HTTP (no-cache, iriam à origem; o site
      `dados` do passo 3 só terá resultado e estado). Nenhuma rota nova.
- [x] a) Teste `site/testes/ws_recusado.mjs` (servidor node:http falso; `/ws` → 503; intervalos curtos por
      `window.__uvConexao = { consultaMs, novoWsMs }`). **Vermelho** antes da correção (saída em `docs/BUGS.md`).
- [x] Registrado no `docs/BUGS.md` (linha "teste vermelho" + seção "B-026" no fim, com a saída: 22 falhas).
- [x] b) Correção em `site/src/conexao.js` (+ `app.js`, `estilo.css`, `index.html` com `<meta name="uv-dados" content="">`).
- [x] `npm run build` (o `dist/` é versionado e embutido no `uv-servico`).
- [x] `ws_recusado: tudo certo` (0 falhas).
- [x] c) Teste verde; suíte do site (sem servidor): `modo_pagina`, `relogio`, `hodometro`, `termos`, `painel_celular`,
      `ws_recusado` ok; `cargo test -p uv-aquisicao --lib` 98 ok.
- [x] `scripts/versao.sh bump patch` (0.8.44), CHANGELOG, BUGS.md (antes/depois), commit, `git push --follow-tags`.
- [x] Ok do Luciano para implantar (20/10, 19h47).
- [x] 0.8.44: binários nos dois nós; B reiniciado (19h52, ok).
- [x] Pedido do Luciano (19h52): `?modo=http` na URL força o modo HTTP (conferir do celular) → 0.8.45 (teste antes 4
      falhas, depois ok).
- [x] 0.8.45 implantada (20h05): binários nos dois, B e A reiniciados (retomaram, ~477,5 mil BUs do replay 452);
      `/api/v1/estado` com `versao` 0.8.45 pelos dois IPs.
- [x] Conferido em produção (`site/testes/modo_http_producao.mjs`): modo HTTP em 4 s, resultado na tela, consultas a
      55–59 s com 200/304, sem eventos/marcos, volta ao tempo real ao liberar o WebSocket.
- Observação: no celular **durante um replay**, o aviso "atualizando a cada minuto" fica cortado pela contagem da
  próxima eleição (a faixa não rola). Na noite real a contagem some e o aviso cabe inteiro (testado a 390 px).
  Encurtar o texto no celular só se o Luciano quiser.
- [x] Passo 1 concluído.

**Ordem trocada pelo Luciano (20/10, 22h23/23h04): passo 3 antes do 2** — Cloudflare primeiro; depois um teste de
carga do WebSocket com os 2 OCPUs atuais decide se o passo 2 (pago) é necessário (decidir até quarta à noite).

**Passo 3 (subdomínio `dados` na Cloudflare) — parte do código, em andamento (20/10, 23h):**
- [x] Teste vermelho em `deploy/oci/testes/test_caddyfile.py` (`TestDados`, 4 falhas; `UV_CADDY=~/uvreal/ferramentas/caddy`).
- [x] Correção: `Caddyfile.template` (bloco `DADOS`), `gerar_caddyfile.sh` (6º arg.), `instalar_caddy.sh` (5º arg.),
      `02_configurar_borda.sh`, `63_ciclo.sh --playlist`, `nos.env.exemplo` (`DOMINIO_DADOS`, vazio = sem o site).
      `deploy/oci/testes` 53 ok.
- [x] `pagina.rs`: CSP `connect-src 'self' https://dados.urnavelox.com.br` (teste `csp_libera_a_base_das_consultas`
      vermelho antes, verde depois); `index.html` com o meta preenchido.
- [x] `site/testes/ws_recusado.mjs`: cenário "base no ar" ok (0 preflight).
- [x] Suítes verdes; 0.8.46; CHANGELOG; commit e push. **Nada implantado.**
- [x] Checklist do Luciano: `docs/CHECKLIST_CLOUDFLARE.md`.
- [x] 21/10, 12h47–13h06 (Luciano): conta e zona na Cloudflare (Free); registros importados conferidos com o DNS
      atual (2 A dos nós, `www` CNAME, TXT google-site-verification; sem MX/AAAA), todos **DNS only**. O domínio tinha
      **DNSSEC** (DS KeyTag 60565, servidores `d/e.sec.dns.br` do Registro.br): no Registro.br não há como desligar
      separado; ao trocar para `paris.ns.cloudflare.com` / `zod.ns.cloudflare.com` sem "+DNSSEC", o Registro.br tira
      o DS e só delega depois (~2 h de "transição", até ~15h05 de 21/10). Conferência agendada ~15h10.
- [x] 21/10, 13h12 (Luciano): `dados` A para os dois IPs dos nós, **DNS only**; SSL/TLS **Full (strict)**.
- [x] Depois da ativação (~15h05): NS `paris`/`zod` da Cloudflare, sem DS, `dados` resolvendo para os dois nós.
- [x] 21/10, ~15h18–15h20 (sessão anterior): `DOMINIO_DADOS` no `nos.env` e Caddy aplicado nos dois nós. **O Caddy
      não subiu (B-027): site principal fora do ar até 16h37/16h38**, restabelecido à mão pelo Run Command.
- [x] 21/10, 16h38–16h45: certificado do `dados` nos dois nós (B-028, um registro renomeado de cada vez na
      Cloudflare; DNS de volta ao original às 16h45).
- [x] Item 3 conferido nos dois nós: `/api/v1/estado` e `/api/v1/resultado/<tópico>` 200 com
      `Cache-Control: public, max-age=30, s-maxage=30`; `/painel` e `/ws` 404; versão 0.8.46.
- [x] 21/10, 16h47 (token da Cloudflare, skill `cloudflare`): Cache Rule `dados-api` (respeita a origem) e nuvem
      **laranja** nos dois `dados`; principal e `www` cinza. Pela borda: `Server: cloudflare`, resultado MISS → HIT
      com o mesmo `ETag`.
- Observação: direto nos nós, o `If-None-Match` com o ETag de um nó volta 200 quando o Caddy manda ao outro (o `seq`
  do mesmo tópico difere entre A e B: 209086 × 209256 em `ac01007-c0011`). Fica para o passo 4 (nó canônico).
- [ ] **[Luciano]** No celular: `https://urnavelox.com.br/?modo=http` (apuração lida pela Cloudflare).
- [ ] Passo 5 do checklist: teste de carga do WebSocket com os 2 OCPUs (com o ok do Luciano) → decide o passo 2.
- [x] (feito, ver as linhas acima) **Aguardando o Luciano** (itens 1–3 do checklist). Quando ele avisar que os registros `dados` (cinza) existem,
      com o ok: pôr `DOMINIO_DADOS=dados.urnavelox.com.br` no `deploy/oci/nos.env` (fora do git), compilar ARM,
      `63_ciclo.sh --binarios`, `--reiniciar B`/`A`, `02_configurar_borda.sh` (Caddy dos dois nós) e conferir o
      `https://dados.urnavelox.com.br/api/v1/estado` e os cabeçalhos; depois nuvem laranja + Cache Rule (item 4) e
      conferir `cf-cache-status: HIT`.
- Ordem das origens A, B (nó canônico) fica para o passo 4, que muda todas as rotas, inclusive as do `dados`.

## Como rodar o teste
```
cd /mnt/d/git/lukeboh/urna-velox/site && node testes/ws_recusado.mjs --saida /tmp/uv-ws-recusado
```

## Decisões de implementação (passo 1)
- Modo HTTP depois de 3 falhas seguidas do WebSocket (recusa ou queda; uma conexão que durou ≥ 30 s zera a conta).
- Consulta a cada 60 s ±20 % do(s) tópico(s) da tela (sem os `tse:`) e do estado; tópico novo na tela → consulta na hora.
- Próprio domínio: `If-None-Match` explícito (`cache: 'no-store'`, 304 chega ao código). Base `uv-dados` (outro
  domínio): **sem** cabeçalho próprio (evita o preflight CORS, que o site `dados` não atende) e `cache: 'no-cache'`,
  o navegador revalida com o ETag sozinho. Falha de rede/5xx na base → próprio domínio e base em pausa por 10 min.
- WebSocket tentado de novo a cada ~2 min ±20 %; ao abrir, sai do modo HTTP.
- Aviso: chip na faixa "atualizando a cada minuto (muitos acessos)" no lugar do BUs/min; selo mantém o modo.
- Fora do modo HTTP (avisado no título do selo): feed de eventos/marcos, "faltam para o matematicamente",
  comparação com o TSE e previsão.

## Passos 2 e 3 (o que vai precisar do Luciano)
Ver §3 do handoff; listar exatamente ao chegar a vez.

## Pendências descobertas para o passo 3 (não esquecer)
- **CSP da página** (`aquisicao/src/pagina.rs`): `connect-src 'self'` bloqueia o `fetch` para
  `https://dados.urnavelox.com.br`. No passo 3, acrescentar essa origem ao `connect-src` (teste primeiro em
  `pagina::testes`), senão a página cai sempre no próprio domínio (funciona, mas sem o cache da borda).
- `pagina::testes::tabela_fixa` proíbe endereço externo no `index.html` além do canônico e da licença: ao pôr
  `<meta name="uv-dados" content="https://dados.urnavelox.com.br">`, incluir a exceção nesse teste.
- O site `dados` no Caddy não precisa de `OPTIONS`/preflight: a página não põe cabeçalho próprio na base externa.
