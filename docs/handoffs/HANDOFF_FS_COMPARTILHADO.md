# Handoff — N instâncias de coleta com sistema de arquivos compartilhado (para revisão crítica)

**Para quem recebe:** este documento é autossuficiente. Você não tem acesso ao repositório nem aos nós; tudo o que é
preciso para criticar a proposta está aqui. Queremos uma **crítica dura**: o que está errado, o que falta, o que é
mais simples, o que vai falhar na noite da eleição. Responda às perguntas da seção 8 e acrescente o que achar.

Autor da proposta: Claude (assistente), a pedido de Luciano Bohnert, 26/10/2026. Versão do sistema: Urna Velox 1.0.13.

## 1. O sistema hoje, em uma página

* **O que faz.** O Urna Velox totaliza a eleição brasileira em tempo real a partir dos boletins de urna (BUs)
  publicados pelo TSE em `https://resultados.tse.jus.br`. Para cada seção eleitoral (~517 mil no 2º turno de 2026, ~499
  mil com BU), o coletor descobre a seção pelo `cs.json` da UF, pede o `aux.json` da seção (hash e situação) e depois
  o BU (`-bu.dat`, ~1 KB). São **2 requisições por BU**. A totalização em si é desprezível (~1 ms de CPU por BU).
* **Linguagem/estrutura.** Rust, um binário de serviço (`uv-servico`) por nó. Cada nó totaliza **todos** os BUs (os que
  ele baixa e os que recebe do par), publica o resultado e serve um site/WebSocket.
* **Limite do TSE (regra dura do projeto).** A borda pública do TSE responde **HTTP 429 acima de 100 req/s por IP de
  saída**; depois de um 429, o IP fica numa "caixa de penalidade" e o projeto **para tudo por 10 min**. A regra exata
  da borda (janela fixa ou deslizante, se 404 pesa mais, se conta por IP ou por faixa/ASN) **não é conhecida**. Cada
  nó roda a **95 req/s** (`Limitador` que espaça as saídas a 1/95 s; HTTP/2, 64 requisições em andamento).
* **Regra de governança.** Um órgão (instituição) não pode usar vários IPs para contornar o limite. A federação existe
  para órgãos *diferentes* colaborarem, cada um com o seu IP e o seu limite. O teste de várias instâncias descrito
  aqui é uma medição autorizada pelo dono do projeto; a operação real exigiria um órgão por IP.
* **Federação atual (2 nós, A e B, OCI São Paulo, Ampere 2 OCPU/12 GB cada, Always Free).** Partição das seções entre
  os nós ativos por peso (cota de req/s), batimento a cada 10 s, troca de **lotes** de BUs entre os pares por HTTP
  (`/fed/v1/...`, porta privada 7700) a cada ~2 s; cada nó aplica tudo. Um nó em pausa de 429 **continua na
  partição** (a metade dele fica parada 10 min) — defeito conhecido.
* **Outras requisições no mesmo teto de 95 req/s:** `cs.json` das 28 UFs a cada 10 s (2,8 req/s, quase sempre 304);
  "sondagem" de seções sem `aux.json` (sobretudo do exterior), 2 req/s, com reforço de 12 req/s quando a fila está
  quase vazia (respostas 404 na maioria); a comparação com o TSE (`-u.json` das abrangências que algum cliente está
  vendo, 1 a cada 30–60 s por abrangência).

## 2. O que aconteceu em 25/10/2026 (2º turno)

Dados públicos (horário de recebimento de cada BU no TSE, do próprio `cs.json`/`aux.json`) e registros dos nós:

* A votação foi prorrogada até as 19h. Nenhum BU foi recebido entre 17:09 e 19:08.
* Das 19:08 às 20:02 o TSE recebeu/publicou ~47 BUs/s; **das 20:02 às 21:14, ~92 BUs/s**, constantes, até quase todos
  os ~499 mil (99 % às 21:14). Houve uma parada do lado do TSE das 19:48:30 às 19:50:45 (nenhum BU recebido).
* Capacidade útil da federação: 2 × (95 − cs.json − sondagem) ÷ 2 ≈ **90 BUs/s** — abaixo dos 92 BUs/s de chegada
  por 72 min.
* **Três 429** apesar de cada nó estar a ≤ 95 req/s: nó B às 19:47:57, nó A às 19:53:50 e às 21:00:11. Cada um tirou
  aquele nó do ar por 10 min.
* Resultado: fila de até 63 mil BUs (21:12); atraso do BU (recebido no TSE → aplicado) com mediana de 389 s e p90 de
  717 s. Um "espelho" sem teto (lendo de um servidor espelho fora da borda, mesmo código) teve mediana de 42 s.

## 3. Quanta capacidade é preciso (modelo de fila, chegada real)

Modelo: cada BU entra na fila no instante em que chegou ao TSE e sai à capacidade `N × (rps − 28/30) ÷ req_por_BU`, em
passos de 1 s; "1 fora" = uma instância parada 10 min começando 5 min antes do pico de chegada (o efeito de um 429).
Não modela sondagem nem comparação.

| instâncias × req/s por IP (2 req/BU) | cap. BU/s | 3221 (2º t. 2026) atraso máx. | idem, 1 fora 10 min | 452 (1º t. 2024, pico 115 BU/s) 1 fora |
|---|---|---|---|---|
| 2 × 95 (hoje) | 94 | 0,1 min | 4,7 min | 5,6 min |
| 3 × 80 | 119 | 0,0 min | 0,9 min | 2,4 min |
| 4 × 60 | 118 | 0,0 min | 0,2 min | 1,6 min |
| 5 × 60 | 148 | 0,0 min | 0,1 min | 0,1 min |
| 2 × 80 com 1 req/BU | 158 | 0,0 min | 0,9 min | 2,4 min |

Leitura: não dá para baixar um BU antes de o TSE publicá-lo; passou da taxa de chegada, mais instâncias só servem
para (a) folga contra a perda de uma instância e (b) rodar cada IP **mais longe** do limite (menos risco de 429).
Proposta de dimensionamento: **4 instâncias a 60 req/s** (40 % de folga por IP; < 2 min de atraso com uma fora).
Acima de 5 não ganha nada.

## 4. A proposta: N instâncias + sistema de arquivos compartilhado

### 4.1 Infraestrutura
* N instâncias na mesma sub-rede, **cada uma com IP público próprio e saída direta** (sem NAT comum: atrás do mesmo
  NAT dividiriam os 100 req/s).
* **OCI File Storage (NFSv4.1)** montado em todas — gerenciado, não depende de nenhuma das instâncias. (Alternativa
  descartada: Object Storage/bucket — cada reserva/conclusão seria uma chamada de API; ~500 mil BUs × várias
  operações estouram a cota gratuita e somam latência de 50–100 ms por operação.)
* NFS não tem notificação entre clientes (inotify não atravessa) e travas de arquivo por NFS são frágeis; o desenho
  evita disputar arquivo.

### 4.2 Quem descobre o que é novo
* O `cs.json` de cada UF fica com **uma** instância (rodízio fixo por UF, com uma reserva defasada). Ela grava no
  compartilhado `novos/<uf>/<seq>` com as seções que ganharam disponibilização desde a leitura anterior. Cada IP
  gasta menos com consulta (hoje cada nó lê as 28 UFs).

### 4.3 Quem baixa cada seção (sem trava no caso normal)
* Cada instância grava `vivos/<inst>` a cada 1 s (mtime).
* Dono da seção = **rendezvous hashing** (HRW) da chave da seção entre as instâncias vivas e não pausadas. Ninguém
  baixa o que é de outra; quando uma instância sai, só as seções dela mudam de dono.
* **Instância em pausa de 429** grava `pausa/<inst>` com a hora em que volta; enquanto existir, ela sai do HRW (corrige
  o defeito de hoje).
* **Na troca de dono** (instância entrou/saiu), para as seções reatribuídas: reserva por criação exclusiva
  `pegos/<seção>` (`O_CREAT|O_EXCL`, atômico no NFSv4), com o id e a hora; reserva com mais de X s sem conclusão pode
  ser tomada.

### 4.4 Como todos ficam sabendo do BU baixado
* A instância grava o BU como `bu/<uf>/<seção>.tmp`, faz `rename` para o nome final (atômico) e acrescenta uma linha no
  **diário dela**, `diario/<inst>.log` (append-only, um escritor por arquivo).
* Cada instância lê o final dos diários das outras a cada ~200 ms (guarda o deslocamento) e aplica os BUs novos.

### 4.5 O que muda em relação à federação HTTP de hoje
* A ideia é a mesma (partição + troca de resultado). A diferença: tira a sincronização par a par (N²), torna
  trivial acrescentar a 5ª instância e dá um lugar único para reservas, pausas e diários.
* **Quem acelera é o número de IPs, não o sistema de arquivos.**

## 5. O risco que decide tudo
* **Não sabemos se a borda do TSE conta por IP ou por faixa (/16, /24, ASN da OCI).** Se contar por faixa, mais
  instâncias na OCI não ajudam e podem piorar. Observação: o nó B e as duas máquinas de teste criadas em 26/10 estão
  na mesma faixa /16; o nó A, em outra.
* Também não sabemos por que houve 429 a ≤ 95 req/s (hipóteses na seção 6).

## 6. O que está sendo medido agora (26/10/2026, à noite)
1. **Reprodução local da noite** (sem tocar no TSE): os 2 nós em tempo real contra o simulador, com a chegada real,
   registrando cada requisição (ms, status, IP). Sobre esse registro, testa-se offline cada regra de borda candidata
   (bloco fixo de 1 s/10 s, janela deslizante de 0,5/1/10 s, teto 80–100, 404 com peso 1/2/5, atraso de rede
   aleatório de 0–150 ms na chegada, "N 404 em 60 s") e procura-se a que reproduz os horários reais dos 429.
   Hipóteses: (1) proporção de 404 da sondagem; (2) saída espaçada que chega amontoada na borda (HTTP/2, 64 em
   andamento, jitter de rede) sob uma janela deslizante; (3) outra regra da borda.
2. **Medição real da borda** com 2 máquinas extras na OCI (IPs próprios, mesma faixa do nó B): uma sozinha a 95 req/s;
   as duas ao mesmo tempo a 95 cada; uma a 100; uma com ~20 % de 404 — para ver se a borda soma IPs da mesma faixa e se
   404 pesa.

## 7. Restrições que a crítica deve respeitar
* Nunca passar de 100 req/s por IP; pausa ≥ 10 min depois de um 429.
* Um órgão por IP na operação real (as N instâncias seriam de órgãos diferentes, como a federação).
* Os nós já usam os 4 OCPU/24 GB do Always Free; instância nova custa dinheiro (pequeno).
* A apuração precisa sobreviver à queda de qualquer instância sem perder BU e sem retotalizar do zero.
* A noite dura ~2–4 h; o pico de chegada observado é de 92–115 BUs/s sustentados por até 70 min.

## 8. Perguntas para quem revisa
1. O sistema de arquivos compartilhado é a ferramenta certa aqui? Compare com: (a) estender a federação HTTP atual
   para N nós; (b) um coordenador leve (Redis/etcd/Postgres com `SELECT … FOR UPDATE SKIP LOCKED`); (c) gossip; (d)
   partição estática por UF sem coordenação.
2. NFSv4.1 na OCI File Storage: latência e consistência de `rename`, `O_EXCL`, leitura de arquivo anexado por outro
   cliente (cache de atributos, `actimeo`, close-to-open). O "tail" de diários por polling a 200 ms funciona sem ver
   dados velhos ou linhas pela metade? O que configurar (`noac`, `lookupcache`)?
3. O HRW com batimento por mtime de arquivo: risco de *split-brain* (duas instâncias acharem que a seção é delas)
   quando os relógios ou o cache de atributos divergem? Quantos BUs duplicados por troca de dono são aceitáveis (cada
   duplicado custa 2 requisições)?
4. O ponto único de falha passa a ser o serviço de File Storage. É aceitável na noite da eleição? Qual o plano B?
5. O dimensionamento (4 × 60 req/s) faz sentido? Há argumento para rodar mais perto de 95 com menos instâncias?
6. A descoberta (`cs.json` por UF em uma instância) cria atraso extra ou ponto de falha? Vale manter cada instância
   lendo um subconjunto com sobreposição?
7. Como testar a borda real (seção 6.2) de forma conclusiva sem levar um bloqueio mais longo? Que outros sinais (cabeçalhos
   da resposta 429, `Retry-After`, `Server`, `Via`) observar?
8. O que você faria diferente para a próxima eleição, dado que a chegada passou de 47 para 92 BUs/s de uma hora para a
   outra?
