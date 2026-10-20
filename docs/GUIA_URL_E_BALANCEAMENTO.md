# Guia de Configuração: URL Pública e Balanceamento para o Urna Velox

Este guia orienta o passo a passo para colocar uma **URL única** (ex.: `urnavelox.com.br` ou subdomínio dinâmico) atendida pelos **2 nós do Urna Velox na OCI**, com **terminação TLS/HTTPS automática e balanceamento**.

---

## 1. Passo 1: Obter a URL Gratuita (DuckDNS ou No-IP)

### Opção A: DuckDNS (Mais simples e rápido — 1 minuto)
1. Acesse **[duckdns.org](https://www.duckdns.org/)** e faça login com sua conta do GitHub ou Google.
2. Na caixa **domains**, escolha um subdomínio (ex.: `urna-velox`).
3. O DuckDNS aceita um IP no painel inicial. Coloque o IP público do **Nó A** (veja no `nos.env` em `NO_A_IP_PUBLICO`).
4. Seu endereço será: `urna-velox.duckdns.org`.

> [!TIP]
> O DuckDNS permite atualizar o IP via URL rápida. Você pode inclusive alternar entre os nós ou apontar para o Load Balancer da OCI.

### Opção B: No-IP
1. Crie uma conta gratuita em **[noip.com](https://www.noip.com/)**.
2. Crie um *Hostname* gratuito (ex.: `urna-velox.ddns.net`).
3. Aponte para o IP público do **Nó A** ou para o IP do Load Balancer da OCI.

---

## 2. Passo 2: Configurar a Cloudflare (Opcional, para escala e proteção)

Se você tiver um domínio registrado ou quiser colocar a Cloudflare na frente para balancear os dois nós com Anycast global:

1. **Adicionar o Domínio:**
   * No painel da Cloudflare, adicione seu domínio e aponte os NameServers indicados.
2. **Configurar o DNS (Round-Robin entre os Nós):**
   * Crie dois registros do tipo `A` com o mesmo nome (ex.: `resultados`):
     * `Tipo A` | Nome: `resultados` | Conteúdo: `IP_PUBLICO_NO_A` | Proxy: **Ativado (Nuvem Laranja)**
     * `Tipo A` | Nome: `resultados` | Conteúdo: `IP_PUBLICO_NO_B` | Proxy: **Ativado (Nuvem Laranja)**
3. **Configuração de SSL/TLS:**
   * No menu **SSL/TLS**, selecione o modo **Full** (ou *Full Strict* se o Caddy já tiver emitido o certificado Let's Encrypt).
4. **Verificar WebSockets:**
   * No menu **Network**, verifique se a opção **WebSockets** está ligada (*ON* por padrão em todas as contas).
5. **Regra de Cache na Borda (Edge Cache):**
   * Em **Caching > Cache Rules**, crie uma regra para otimizar os assets estáticos:
     * Se `URI Path` começa com `/static/` ou termina com `.js`, `.css`, `.woff2`: **Cache Everything** (Edge TTL: 1 dia).
     * O WebSocket (`/ws`) passa direto sem cache automaticamente.

---

## 3. Passo 3: Executar a Automação no Servidor

Na pasta `deploy/oci/` do repositório (via WSL ou Linux):

### Cenário 1: Borda Direta com Caddy (Recomendado para DuckDNS / No-IP / Cloudflare)
Rode o script fornecendo o seu subdomínio criado:

```bash
cd deploy/oci
./02_configurar_borda.sh urnavelox.com.br
```

O que este script faz automaticamente:
1. Abre as portas **80 (HTTP)** e **443 (HTTPS)** na Security List `uv-seguranca` da OCI.
2. Instala o servidor web **Caddy** em ambos os nós (`uv-no-a` e `uv-no-b`).
3. Configura o Caddy para:
   * Solicitar o certificado SSL gratuito da Let's Encrypt automaticamente.
   * Repassar o WebSocket `/ws` para o Urna Velox local (`127.0.0.1:8080`) com streaming contínuo.
   * Repassar a API REST `/api/v1/*` e a página SPA `/`.
4. Abre as portas 80 e 443 no `iptables` interno do Ubuntu.

---

### Cenário 2: Usar o Load Balancer Oficial da OCI (Always Free)
Se preferir que a própria Oracle Cloud forneça um IP único balanceador antes dos nós:

```bash
cd deploy/oci
./03_criar_load_balancer_oci.sh
```

Esse script cria o **uv-load-balancer** (10 Mbps gratuito), cadastra o Nó A e Nó B no backend com health check em `/api/v1/estado` e imprime o IP público do Load Balancer. Basta apontar o DuckDNS ou Cloudflare para esse IP!

---

## 4. Teste e Validação

Após rodar o script e o DNS propagar:

1. **Teste no Navegador:**
   Acesse no Chrome/Firefox:
   ```
   https://urnavelox.com.br
   ```
   A página do Urna Velox deve carregar com o cadeado verde (HTTPS) e conectar ao WebSocket (`/ws`).

2. **Teste via cURL:**
   ```bash
   curl -I https://urnavelox.com.br/api/v1/estado
   ```
   Deve responder `HTTP/2 200` com os metadados da apuração.

3. **Verificação dos Logs no Servidor:**
   ```bash
   sudo tail -f /var/log/caddy/urna-velox-acesso.log
   ```
