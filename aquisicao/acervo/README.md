# Acervo de Boletins de Urna (BUs) — Urna Velox

Scripts para montagem e gerenciamento de um acervo local, estruturado e verificável com arquivos de urna brutos (BUs e arquivos associados) publicados no Portal de Dados Abertos do TSE.

## Funcionalidades
* **Descoberta Dinâmica via API CKAN**: Mapeia automaticamente todas as eleições e pacotes disponíveis em ordem cronológica inversa (2026 2ºT quando publicado -> 2026 1ºT -> 2024 -> 2022).
* **Respeito aos Limites de Rede do CDN**:
  * Limite rigoroso de requisições por segundo (padrão <= 90 req/s, teto absoluto 100 req/s).
  * Retomada automática com HTTP `Range` via `.part`.
  * Tratamento de HTTP 429 / 5xx com recuo defensivo de 10 minutos (proteção contra bloqueio por IP da Akamai).
  * `User-Agent: urna-velox-acervo/1.0 (+https://github.com/lukeboh/urna-velox-civica)`.
* **Derivação de Pacotes "Stored"**:
  * Extrai os BUs brutos (`-bu.dat` / `.bu`) e assinaturas (`-vota.vsc` / `.vscmr`), salvando em formato ZIP `STORED` (sem recompressão).
  * Permite que o totalizador `uv-coletor` leia diretamente os dados via mmap sem gargalo de CPU.
* **Índices Estruturados**:
  * Gera para cada UF um arquivo CSV com as colunas: `arquivo,municipio,zona,secao,bytes,sha256`.
* **Validação de Conformidade ASN.1**:
  * Valida amostras de BUs decodificando o envelope e a estrutura `EntidadeBoletimUrna` com `asn1tools` e a especificação `bu.asn1` oficial.

---

## Como Executar

### Pré-requisitos
* Python 3.10+
* Biblioteca `asn1tools` (instalada no venv do ambiente de testes ou no sistema).

### Execução Completa (todas as eleições descobertas)
```bash
python3 aquisicao/acervo/gerenciar_acervo.py --raiz /mnt/d/dados/tse-arquivos-urna
```

### Execução Filtrada (Exemplos)
```bash
# Apenas a eleição de 2026 (1º turno, DF e AC):
python3 aquisicao/acervo/gerenciar_acervo.py --anos 2026 --turnos 1 --ufs DF,AC

# Amostra de 1 UF por turno para teste rápido:
python3 aquisicao/acervo/gerenciar_acervo.py --limite-ufs 1

# Modo "somente BUs" (apaga o arquivo original de vários GBs após derivar o pacote de BUs):
python3 aquisicao/acervo/gerenciar_acervo.py --somente-bus
```

---

## Estrutura do Acervo Gerado
```text
/mnt/d/dados/tse-arquivos-urna/
  README.md                 Documentação e convenções de nomenclatura
  catalogo.json             Relação completa de recursos e metadados CKAN
  manifesto.csv             Registro com URL, ETag, SHA-256 e data de download
  originais/<ano>/<turno>t/ Arquivos ZIP originais baixados do CDN
  bus/<ano>/<turno>t/<UF>.zip   Pacotes derivados apenas com BUs e assinaturas
  bus/<ano>/<turno>t/<UF>.csv   Índice detalhado de seções e hashes SHA-256
  documentacao/<ano>/       Especificações técnicas, schemas ASN.1 e leiame.pdf
  logs/                     Logs detalhados de cada execução
  RELATORIO.md              Relatório de integridade, contagem e tempos
```
