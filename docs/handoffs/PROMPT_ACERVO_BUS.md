# Prompt: acervo local de BUs (dados abertos do TSE)

Prompt para pedir a outra IA (ou agente) que monte, no computador de testes, um **acervo local, estruturado e verificável** com os arquivos de urna de todas as eleições publicadas no portal de dados abertos do TSE. Com ele, os testes de totalização do Urna Velox deixam de depender de download. O `uv-coletor` já lê esses zips sem rede: `uv-coletor zip --uf df --local <arquivo.zip> --dados <pasta>`.

Copie a partir da linha abaixo.

---

## Tarefa

Monte um acervo local com **todos os arquivos de urna (Boletins de Urna brutos e arquivos associados)** publicados no portal de dados abertos do TSE, baixando as eleições em **ordem cronológica inversa** (da mais recente para a mais antiga). O acervo vai alimentar testes de um totalizador eleitoral de código aberto (Urna Velox, repositório `D:\git\lukeboh\urna-velox`), que não pode depender de rede durante os testes.

### Ambiente

* Windows com WSL2 (Ubuntu-24.04). Use Python 3 (biblioteca padrão; `requests` é opcional).
* Pasta raiz do acervo: `D:\dados\tse-arquivos-urna` (no WSL: `/mnt/d/dados/tse-arquivos-urna`). Deixe-a configurável. **Nunca** grave dados dentro do repositório git.
* Coloque o(s) script(s) em `D:\git\lukeboh\urna-velox\aquisicao\acervo\`, com um `README.md` explicando como rodar e retomar.
* Há cerca de 850 GB livres em `D:`. Confira o espaço livre antes de cada eleição.

### Fonte (descubra, não deduza)

1. Liste os conjuntos pela API CKAN do portal: `https://dadosabertos.tse.jus.br/api/3/action/package_search` e `package_show`. Os conjuntos de interesse se chamam `resultados-<ano>-arquivos-transmitidos-para-totalizacao`. Em 11/10/2026 existiam:
   * `resultados-2026-...`: 28 recursos (1º turno: 27 UFs + ZZ = exterior; o 2º turno é em 25/10/2026 e deve aparecer depois);
   * `resultados-2024-...`: 52 recursos (1º e 2º turnos);
   * `resultados-2022-...`: 56 recursos (1º e 2º turnos).
2. Cada recurso é um zip por UF e turno em `https://cdn.tse.jus.br/estatistica/sead/eleicoes/eleicoes<ano>/arqurnatot/bu_imgbu_logjez_rdv_vscmr_<ano>_<turno>t_<UF>.zip`, com BU, imagem do BU, log, RDV e assinaturas de todas as seções (o do DF em 2026 tem 1,27 GB; os BUs são só ~8 % disso).
3. Procure também conjuntos de anos anteriores ou de eleições suplementares que tragam **arquivos de urna brutos** (não confundir com o "boletim de urna" em CSV, `bweb_*`, que é outra coisa). Se não existirem, registre no relatório o que procurou e não encontrou.
4. Para cada eleição, baixe também a documentação do formato publicada pelo TSE (ex.: para 2026, `https://www.tse.jus.br/eleicoes/eleicoes-2026` → "formato-arquivos-de-bu-rdv-e-assinatura-digital", que traz `bu.asn1`, `rdv.asn1`, `assinatura.asn1` e scripts Python) e o `leiame.pdf` de dentro dos zips.

### Ordem

Cronológica inversa por eleição e turno: 2026 2º turno (quando publicado) → 2026 1º turno → 2024 2º → 2024 1º → 2022 2º → 2022 1º → anos anteriores, se houver. Dentro de cada turno, UFs em ordem alfabética, ZZ por último. Ao rodar de novo, o script deve pegar o que for publicado depois (ex.: o 2º turno de 2026) sem baixar de novo o que já está completo.

### Regras de uso da rede (obrigatórias)

* **No máximo 100 requisições por segundo no total**, somando CKAN, HEAD, GET e faixas (Range). Imponha isso em código.
* **No máximo 2 downloads simultâneos** (são arquivos grandes; banda não é o objetivo).
* Retomada com `Range` a partir de um arquivo `.part`; confira `Content-Length`, `ETag` e `Last-Modified`.
* HTTP 429 ou 5xx: pare **todas** as requisições e espere com espera exponencial começando em 30 s. A borda do CDN (Akamai) põe o IP numa "caixa de penalidade" de ~8 minutos; depois de um 429, espere pelo menos 10 minutos antes de continuar. O IP de saída é compartilhado com outras pessoas.
* `User-Agent: urna-velox-acervo/1.0 (+https://github.com/lukeboh/urna-velox-civica)`.
* Não use vários IPs, proxies ou outros truques para contornar limites.
* Não rode durante a apuração de uma eleição (noite do pleito e dia seguinte).

### Estrutura do acervo

```text
<raiz>/
  README.md                 gerado: o que é, origem, data, como verificar e como usar
  catalogo.json             eleições/recursos descobertos na API CKAN e situação de cada um
  manifesto.csv             uma linha por arquivo: ano,turno,uf,url,bytes,etag,last_modified,sha256,baixado_em,situacao
  originais/<ano>/<turno>t/bu_imgbu_logjez_rdv_vscmr_<ano>_<turno>t_<UF>.zip   (zip original, intocado)
  bus/<ano>/<turno>t/<UF>.zip      só os BUs + arquivos de assinatura da seção, nomes originais,
                                    sem recomprimir (método "stored"); é o que o Urna Velox lê nos testes
  bus/<ano>/<turno>t/<UF>.csv      índice: arquivo,municipio,zona,secao,bytes,sha256
  documentacao/<ano>/              documentação de formato (.asn1, scripts, PDFs) e leiame.pdf
  logs/                            log de cada execução
```

* Os nomes internos mudam entre eleições (em 2026: `o03220df9701200150300-bu.dat`, `…-vota.vsc`, `…-rdv.dat`, `…-log.jez`; em outros anos podem ser `.bu`, `.vscmr` etc.). Descubra pelo diretório central de cada zip, descreva no `README.md` gerado a convenção de cada ano, e extraia município, zona e seção para o índice CSV.
* Se o espaço livre não permitir guardar os originais de uma eleição inteira, guarde só `bus/` dessa eleição, anote isso no manifesto e avise no relatório final. Nunca apague um original já baixado sem perguntar.

### Integridade e idempotência

* Grave em `.part` e renomeie (atomicamente) só depois de: tamanho igual ao `Content-Length`; zip íntegro (`zipfile.ZipFile.testzip()` sem erro); SHA-256 calculado e registrado no manifesto.
* Rodar de novo deve: pular o que já está completo e íntegro; retomar o incompleto; baixar de novo o que mudou no servidor (ETag ou tamanho diferentes), guardando a versão anterior com sufixo de data.
* Para cada UF, confira que o número de BUs no zip derivado é igual ao do original.
* Decodifique uma amostra de BUs de cada eleição (por exemplo, 20 por UF) com `asn1tools` e a especificação ASN.1 daquele ano, para confirmar que o formato é o esperado. Em 2026, `EntidadeEnvelopeGenerico` tem o campo `conteudo` com um `EntidadeBoletimUrna`. Registre as falhas.

### Entregáveis

1. Script(s) e `README.md` em `aquisicao/acervo/`, com opções para raiz, anos, turnos, UFs e "somente BUs".
2. O acervo na estrutura acima.
3. Um relatório final (`<raiz>/RELATORIO.md`) com uma tabela por eleição e turno: UFs, nº de BUs, tamanho dos originais, tamanho de `bus/`, tempo de download, falhas e pendências. Inclua o que não foi encontrado no portal.
4. Não faça commit dos dados. Faça commit apenas dos scripts e da documentação, se o usuário pedir.
