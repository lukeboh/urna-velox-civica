# Handoff — Urna Velox: Etapa 7, comparação Urna Velox × TSE (`0.7.0`, alvo 19/10/2026)

> **Etapa nova (decisão do Luciano, 16/10/2026).** Duas funcionalidades de usabilidade entraram antes da `1.0.0`: esta
> (etapa 7) e a previsão do matematicamente e do fim da apuração (etapa 8,
> [HANDOFF_0.8.0.md](/mnt/d/git/lukeboh/urna-velox/docs/handoffs/HANDOFF_0.8.0.md)). O ensaio geral passou a ser a **etapa 9
> (`0.9.0`)**, [HANDOFF_0.9.0.md](/mnt/d/git/lukeboh/urna-velox/docs/handoffs/HANDOFF_0.9.0.md) (até 16/10 era este nome de
> arquivo, `HANDOFF_0.7.0.md`). Mapa no [ROADMAP](/mnt/d/git/lukeboh/urna-velox/ROADMAP.md).

*Preparado em 16/10/2026. Ponto de partida: a `0.6.x` mais recente (`git fetch` + `git log`). Quem executa trabalha em
PATCH sobre a série corrente e fecha com `scripts/versao.sh bump minor` → `0.7.0` quando o critério abaixo for atingido
(com validação independente ou por decisão explícita do Luciano). Os itens do ensaio geral podem andar em paralelo, em
outra sessão: combine antes de mexer nos mesmos arquivos (`aquisicao/src/servico*`, `site/src/app.js`).*

**Caminhos:** todos absolutos. Repositório = `D:\git\lukeboh\urna-velox` = `/mnt/d/git/lukeboh/urna-velox` (WSL
`Ubuntu-24.04`); dados = `/home/dev/uvreal`; ensaios desta etapa em `/home/dev/uvreal/ensaios/etapa7-*` (arquivar
depois com `scripts/disco.sh arquivar`, regra **Disco** do [CLAUDE.md](/mnt/d/git/lukeboh/urna-velox/CLAUDE.md)).

## Objetivo

Na página de resultados, poder ver **lado a lado** a totalização do Urna Velox e a do site de resultados do TSE, para a
mesma abrangência e cargo: em que **posição** e com que **contagem** está cada candidato em cada uma, quantas seções
cada uma já totalizou e de quando é o último arquivo do TSE. Um controle de três posições escolhe o que aparece:
**só Urna Velox**, **ambos** ou **só TSE**.

## Regra de rede (combinada com o Luciano em 16/10/2026)

O [CLAUDE.md](/mnt/d/git/lukeboh/urna-velox/CLAUDE.md) proíbe coletar durante a apuração sem combinar. Para a
comparação, ficou combinado, **inclusive na noite da apuração**:

* só o arquivo de resultado (`<abr>-c<cargo>-e<eleição>-u.json`) da **abrangência e cargo que estão na tela** de pelo
  menos um cliente com a comparação ligada (modo "ambos" ou "só TSE");
* no máximo **uma busca a cada 30–60 s** por (abrangência, cargo), não importa quantos clientes a estejam vendo
  (cache compartilhado no servidor; padrão 30 s, abaixo de 30 s recusado);
* **nada em segundo plano**: ninguém olhando, nenhuma requisição;
* dentro dos 100 req/s do órgão (mesmo limitador da coleta, com prioridade abaixo dela) e com a regra do HTTP 429
  (parar tudo ≥ 10 min). Isso fica na casa de poucas requisições por segundo no pior caso, muito abaixo do teto.

## Desenho

### Quem busca o arquivo do TSE: o `uv-servico`, não o navegador

* O navegador não pode buscar direto no site do TSE (CORS) e, se pudesse, cada visitante faria a sua requisição. O
  servidor busca uma vez por intervalo e distribui para todos os clientes, e a requisição sai pelo IP do órgão, dentro
  do limite dele.
* Módulo novo em `aquisicao/src/` (nome sugerido: `tse_resultados.rs`), dono de um mapa (abrangência, cargo) →
  último arquivo, com `If-None-Match`/`If-Modified-Since` (304 não traz corpo) e `--tse-intervalo` (padrão 30 s,
  mínimo 30, máximo 60) e `--tse-max-topicos` (padrão 200; acima disso, o tópico novo espera vaga e o cliente vê
  "comparação indisponível no momento"). Pior caso no padrão: 200 / 30 s ≈ 6,7 req/s.
* O parse reaproveita o módulo `oficial` do motor (o mesmo formato das conferências), a partir dos bytes (hoje lê de
  arquivo: acrescentar a variante que lê de memória, sem mudar o resultado das conferências).
* Na federação, cada nó busca para os seus próprios clientes, no seu IP e no seu limite (cada nó é de um órgão).
* `saude.json` ganha um bloco `tse_resultados`: tópicos ativos, req/s, 304, 404, 429, idade do arquivo mais antigo.

### Protocolo

* Tópico paralelo `tse:<tópico>` (mesmo tópico de abrangência × cargo da [PROTOCOLO_WS.md](/mnt/d/git/lukeboh/urna-velox/docs/arquitetura/PROTOCOLO_WS.md)):
  o cliente só se inscreve nele com a comparação ligada, e é essa inscrição que liga a busca no servidor.
* Mensagem nova `tse` (instantâneo na inscrição e atualização a cada arquivo novo): votos e posição por candidato,
  seções totalizadas, brancos, nulos, comparecimento, horário do arquivo no TSE, horário da busca e um estado
  (`ok`, `desatualizado` depois de dois intervalos sem arquivo novo, `indisponivel` em 404/erro, `pausado` depois de 429).
* A posição do TSE é a que o próprio arquivo traz (conferir o campo de ordem no arquivo); se não houver, calcular pelos
  votos com a mesma regra de desempate do motor. Documentar no protocolo por acréscimo (versão 1 continua compatível).

### Página

* Controle segmentado de três posições, **"Urna Velox · Ambos · TSE"**, no topo da tela de resultados; padrão
  "Urna Velox". A escolha fica no navegador (`localStorage` em `try/catch`) e pode vir na URL (`?comparar=ambos`,
  `?comparar=tse`) para compartilhar.
* **Ambos:** cada candidato com votos, % dos válidos e posição nas duas fontes e a diferença (Urna Velox − TSE). Linha
  destacada quando a **ordem diverge**. No cabeçalho, % de seções de cada fonte e "TSE: arquivo das 20:41:10 (há 40 s)":
  sem esse contexto, o Urna Velox à frente (o normal) pareceria divergência.
* **Só TSE:** o mesmo leiaute com os números do TSE e o estado da fonte.
* Mesmas regras da etapa 5: nenhum pulo (CLS 0, só `transform`/`opacity`), números como hodômetro (o do TSE anda em
  degraus, a cada arquivo), tamanhos reservados para as duas colunas também no celular.
* Marco novo no feed: **"TSE confirmou o matematicamente às hh:mm — Urna Velox às hh:mm, N min antes"** (quando o
  arquivo do TSE mostrar a mesma situação), e "ordem diferente entre as fontes" quando durar mais de dois intervalos.
* Arquivos: `site/src/comparacao.js` (novo), `app.js`, `conexao.js`, `estilo.css`, `textos.js`; testes em
  `site/testes/` (o `rodar.mjs` e a `sonda.mjs` medem o CLS e conferem a tela).

### Simulador

* O `uv-simulador` passa a servir também os `-u.json` **progressivos**: um "TSE simulado" que totaliza os mesmos BUs com
  o atraso da divulgação oficial (atraso recebimento → divulgação sorteado de uma log-normal), regenerando os arquivos a cada intervalo do relógio acelerado. No fim
  da noite ele tem de ser igual ao oficial, o que dá a conferência desta etapa.

## Critério

1. **Simulador:** 2º turno de 2024 a 10× e 1º turno de 2026 a 60×, com 10 abas em modos variados (ambos, só TSE,
   alternando): no fim, coluna do TSE = oficial = coluna do Urna Velox (51/51 e 83/83), posições iguais, CLS 0.
2. **Rede:** no log, ≤ 1 busca por (abrangência, cargo) inscrita por intervalo e **0 buscas** sem cliente com a
   comparação ligada; 0 HTTP 429; BU → publicação p99 sem regressão em relação à `0.6.x`.
3. **Site real (antes da eleição):** ≥ 30 min contra o pleito 3220 (arquivos finais) e o 3221 (pré-apuração), com 3
   telas abertas: números idênticos aos arquivos, 0 HTTP 429.
4. O marco "TSE confirmou" bate com o `eventos.csv` do simulado.

Relatório em `docs/relatorios/RELATORIO_0.7.0.md`; marcar os itens no [ROADMAP](/mnt/d/git/lukeboh/urna-velox/ROADMAP.md) com a versão.
