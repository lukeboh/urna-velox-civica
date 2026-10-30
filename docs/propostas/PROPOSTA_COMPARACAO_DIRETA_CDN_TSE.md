# Proposta: Comparação ao Vivo Direto da CDN do TSE

> **Origem:** Proposta do Luciano em 21/10/2026.  
> **Classificação:** Melhoria arquitetural para Usabilidade / Borda / Comparação (`1.x`).  
> **Referência no Roadmap:** [ROADMAP.md](../../ROADMAP.md) § `🔧 1.x — depois da 1.0`.

---

## 1. Motivação e Contexto

Na Etapa 7 (`0.7.0`, [HANDOFF_0.7.0.md](../handoffs/HANDOFF_0.7.0.md)), foi introduzida a funcionalidade de comparação lado a lado entre o Urna Velox e a totalização oficial do TSE ("Urna Velox · Ambos · TSE").

No modelo atual implementado:
1. Quando o usuário ativa "Ambos" ou "TSE" no navegador, a página se inscreve no tópico WebSocket `tse:<tópico>` no servidor `uv-servico`.
2. O servidor `uv-servico` (no módulo `tse_resultados.rs`) faz o polling do arquivo de resultados (`<abr>-c<cargo>-e<eleição>-u.json`) diretamente no TSE a cada 30–60 segundos, consome sua cota de requisições de saída (teto de 100 req/s da rede do órgão), faz o parse do JSON e distribui mensagens `tipo: "tse"` para os clientes conectados.

### O Problema
Em eleições reais ao vivo com muitos usuários acompanhando diferentes circunscrições:
- O servidor do Urna Velox consome CPU, conexões de saída e limite de requisições (rate limit de rede/órgão) atuando como proxy de um dado público que **já é estático e distribuído pelo próprio TSE**.
- O TSE já possui uma CDN massiva (Akamai/Cloudflare) desenhada exatamente para absorver milhões de acessos simultâneos a esses arquivos `.json` de resultado.

---

## 2. A Solução Proposta

Em **eleições ao vivo**, descentralizar a busca do resultado do TSE, transferindo-a diretamente para o cliente web (navegador do usuário):

1. **Busca Direta no Carregamento:** Assim que a página for carregada ou o usuário trocar de tópico (abrangência/cargo) com o modo de comparação ativo ("Ambos" ou "TSE"), o navegador faz `fetch()` direto na URL oficial da CDN do TSE:
   ```text
   https://resultados.tse.jus.br/oficial/ele2026/<eleição>/dados/<abr>/<abr>-c<cargo>-e<eleição>-u.json
   ```
2. **Cadência Periódica de 30 em 30 Segundos:** O navegador agenda um polling a cada 30 segundos usando cabeçalhos condicionais (`If-Modified-Since` e `If-None-Match`/`ETag`), recebendo `304 Not Modified` quando não houver atualização no TSE.
3. **Parse no Navegador:** O cliente web interpreta os campos essenciais do JSON (`vap`, `pvap`, `pst`, `s`, `dg`, `hg`) e atualiza as barras e comparativos de voto na tela.
4. **Desoneração Total do Backend:** O `uv-servico` **não** recebe inscrição `tse:<tópico>`, **não** faz requisições externas para o TSE para fins de comparação na tela e concentra 100% dos seus recursos na ingestão de BUs, totalização instantânea e difusão dos dados do Urna Velox.

---

## 3. Convivência: Eleição ao Vivo vs. Simulação / Replay

| Cenário | Origem do dado do TSE | Mecanismo |
|---|---|---|
| **Eleição ao vivo (produção)** | **CDN do TSE** (`resultados.tse.jus.br`) | Navegador busca direto via HTTP/Fetch (inicial + a cada 30 s). Servidor `uv-servico` sem tópicos `tse:`. |
| **Simulação / Replay local** | **Servidor Urna Velox** (`uv-servico`) | Mantém a inscrição WebSocket `tse:<tópico>`. Como a CDN do TSE só possui o arquivo final consolidado, o simulador sintetiza os arquivos progressivos via módulo `tse_simulado.rs`. |

O frontend detecta o ambiente pelo `estado` do serviço (`ambiente == "oficial"` ou ausência de âncora/simulação) e seleciona o canal correspondente de forma transparente.

---

## 4. Requisitos e Cuidados Técnicos

1. **CORS (Cross-Origin Resource Sharing):**
   - Confirmar que a CDN de divulgação do TSE envia `Access-Control-Allow-Origin: *` nos arquivos de resultados em produção.
   - Caso haja restrição em algum ambiente de homologação, prever fallback no servidor ou proxy de borda leve.
2. **CSP (`connect-src`):**
   - Adicionar `https://resultados.tse.jus.br` na política de Content Security Policy servida pelo `pagina.rs` e nas regras do Caddy/Cloudflare.
3. **Resiliência e Tratamento de Erros:**
   - Se o arquivo do TSE retornar 404 (apuração ainda não iniciada) ou erro de CDN (503/429), o frontend deve apresentar o status gracioso ("Aguardando início pelo TSE" / "TSE indisponível temporariamente"), sem travar ou interromper a exibição do Urna Velox.
4. **Alinhamento com o Modo HTTP:**
   - No modo HTTP de contingência da página (`?modo=http` / B-026), essa abordagem integra-se naturalmente, pois a busca do TSE já opera via HTTP estático no cliente.

---

## 5. Próximos Passos (Critérios de Entrega na `1.x`)

- [ ] Teste de cabeçalhos de CORS contra a infraestrutura oficial do pleito 3221.
- [ ] Implementação da função `buscarTseDireto()` em `site/src/comparacao.js`.
- [ ] Atualização da CSP em `aquisicao/src/servico/pagina.rs`.
- [ ] Teste automatizado de front-end simulando respostas 200, 304 e falhas de rede da CDN externa.
