# Checklist — subdomínio `dados` na Cloudflare (passo 3 do plano de borda)

*Para o Luciano, 20/10/2026. Plano: [HANDOFF_MODELO_BORDA_E_CONSISTENCIA.md](HANDOFF_MODELO_BORDA_E_CONSISTENCIA.md)
§3, passo 3. Andamento: [CHECKPOINT_BORDA.md](CHECKPOINT_BORDA.md).*

**O que muda:** só o nome novo `dados.urnavelox.com.br` passa pela Cloudflare (nuvem **laranja**), com cache de 30 s
do resultado e do estado: é dele que as páginas no modo HTTP (WebSocket lotado) leem. O domínio principal
(`urnavelox.com.br`: página, WebSocket, playlist, painel) continua **direto nos nós**, com a nuvem **cinza**, como hoje.

Legenda: **[você]** só você pode fazer · **[Claude]** eu faço, com o seu ok na hora · ✔ conferência.

## 1. Conta e zona na Cloudflare
- [ ] **[você]** Criar a conta em cloudflare.com (se ainda não tem).
- [ ] **[você]** *Add a domain* → `urnavelox.com.br` → plano **Free**. A Cloudflare lê os registros que existem hoje.
- [ ] **[você]** Mandar para mim **duas listas**: a dos registros que a Cloudflare importou e a do DNS atual no
      Registro.br (print serve). Vou conferir que nada se perde. Esperado no mínimo: `urnavelox.com.br` tipo A para os
      IPs públicos dos **dois nós** (`NO_A_IP_PUBLICO` e `NO_B_IP_PUBLICO` do `deploy/oci/nos.env`, fora do git; os
      mesmos de hoje); `www` (se existir hoje); e qualquer MX/TXT (e-mail, verificação do Google etc.).
- [ ] **[você]** Em **todos** os registros importados: nuvem **cinza** ("DNS only"). Se algum vier laranja, trocar
      para cinza. (Laranja no domínio principal mexeria no WebSocket, na playlist, no painel e no IP do visitante.)
- [ ] **[você]** *SSL/TLS → Overview*: modo **Full (strict)**. Deixar *Always Use HTTPS* **desligado** (o padrão).

## 2. Servidores DNS no Registro.br
- [ ] **[você]** No Registro.br, se o **DNSSEC** estiver ativo no domínio, **desativar antes** da troca (com DNSSEC
      ligado e servidores novos, o domínio pode parar de resolver).
- [ ] **[você]** *Alterar servidores DNS* → os **dois** nomes que a Cloudflare indicar (`xxx.ns.cloudflare.com`).
- [ ] **[você]** Esperar o e-mail da Cloudflare dizendo que a zona está **ativa** (minutos a algumas horas) e me avisar.
- [ ] ✔ **[Claude]** Conferir que o site principal ficou igual: página, `/ws`, playlist e painel pelos dois nós, e que
      o DNS responde pela Cloudflare com os mesmos IPs.

## 3. O nome `dados` (ainda cinza)
- [ ] **[você]** *DNS → Add record*: tipo **A**, nome `dados`, IPv4 = **IP público do nó A** (o mesmo do registro
      `urnavelox.com.br`), nuvem **cinza**.
- [ ] **[você]** Outro: tipo **A**, nome `dados`, IPv4 = **IP público do nó B**, nuvem **cinza**. Me avisar.
- [ ] **[Claude]** Implantar a versão nova (página com a base `dados` e a CSP que a libera; binários nos dois nós, B
      e depois A) e o Caddy com o site `dados` (`DOMINIO_DADOS=dados.urnavelox.com.br` no `nos.env` e
      `02_configurar_borda.sh`: ~1 s de reinício do Caddy por nó). O Caddy obtém o certificado do `dados`.
- [ ] ✔ **[Claude]** `https://dados.urnavelox.com.br/api/v1/estado` responde pelos dois nós com
      `Cache-Control: public, max-age=30, s-maxage=30`; outras rotas dão 404.

## 4. Nuvem laranja e cache, só no `dados`
- [ ] **[você]** Nos **dois** registros `dados`: nuvem **laranja** (os do domínio principal ficam cinza).
- [ ] **[você]** *Caching → Cache Rules → Create rule*:
  - nome: `dados-api`;
  - quando: *Hostname* **equals** `dados.urnavelox.com.br`;
  - então: **Eligible for cache**; *Edge TTL*: **Use cache-control header if present** (respeitar a origem);
    *Browser TTL*: **Respect origin**;
  - *Deploy*. (Sem essa regra a Cloudflare não guarda JSON: tudo iria aos nós.)
- [ ] **[você]** Me avisar.
- [ ] ✔ **[Claude]** Duas consultas seguidas ao mesmo tópico pela borda: `cf-cache-status: HIT` e o mesmo `ETag`;
      o site principal idêntico ao de antes.
- [ ] ✔ **[você]** No celular: `https://urnavelox.com.br/?modo=http` — a apuração aparece e é atualizada a cada
      minuto (agora lida pela Cloudflare).

## 5. Depois
- [ ] **[Claude]** Teste de carga do WebSocket com os 2 OCPUs atuais (com o seu ok, fora de qualquer coleta real): o
      número decide o passo 2 (CPU paga). Decidir até **quarta à noite**.

**Desfazer, se algo der errado:** nuvem cinza nos registros `dados` (a página continua funcionando: cai sozinha no
próprio domínio). O domínio principal não depende de nada disto.

**Alternativa ao painel:** em vez de você clicar nos itens 3 e 4, um *API token* da Cloudflare restrito a esta zona
(*Zone → DNS → Edit* e *Zone → Cache Rules → Edit*) me deixa fazer os dois; o 1 e o 2 (conta, DNSSEC e servidores
DNS no Registro.br) continuam com você.
