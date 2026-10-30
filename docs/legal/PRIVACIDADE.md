# Privacidade, acesso e retenção de dados: Urna Velox

Em vigor desde 24/10/2026. O site publica este mesmo texto em
[urnavelox.com.br/privacidade](https://urnavelox.com.br/privacidade) (gerado deste arquivo no build da página,
`site/documentos.mjs`). Os termos de uso e a licença estão no [LICENCIAMENTO.md](LICENCIAMENTO.md), publicado em
[urnavelox.com.br/termos](https://urnavelox.com.br/termos).

## Em poucas palavras

* **Fazemos o mínimo.** Não há cadastro, login de visitante, cookies, anúncios, pixels nem ferramentas de rastreamento
  ou de análise de terceiros.
* **Não vendemos** nem alugamos nenhuma informação, e **não compartilhamos** informação pessoal dos visitantes com
  ninguém.
* Tudo o que é registrado serve **só para viabilizar a sua experiência** (a página funcionar, ficar no ar e lembrar as
  suas preferências) e **melhorar a própria plataforma**.

## 1. Quem somos e a que se aplica

O Urna Velox é uma totalização eleitoral independente: lê os arquivos públicos de divulgação do Tribunal Superior
Eleitoral (TSE) e mostra a apuração em tempo real. Esta política vale para todo o site **urnavelox.com.br**: a página da
apuração, a playlist de replays (`/playlist`), a API pública (`/api/v1/…` e `/ws`) e o subdomínio de dados
(`dados.urnavelox.com.br`). Ela segue a Lei Geral de Proteção de Dados (LGPD, Lei 13.709/2018).

Os resultados exibidos são dados públicos de eleições (votos por candidato, seções apuradas), não dados pessoais de
visitantes.

## 2. Política de acesso

* **Acesso livre e anônimo.** Qualquer pessoa vê a apuração sem se identificar. Nada no site pede nome, e-mail, CPF,
  telefone ou localização.
* **Sem nada de fora.** A página não carrega nada de outros endereços (sem CDN, fonte remota, anúncio ou rastreador); a
  política de segurança de conteúdo do servidor bloqueia qualquer tentativa. A única exceção são as consultas de
  resultado ao nosso próprio subdomínio de dados.
* **Limites de uso.** Para manter o serviço no ar para todos, há limites técnicos de conexões e de pedidos; quem passa
  deles recebe um aviso de serviço ocupado, sem nenhum registro adicional.
* **Área de operação.** O painel dos nós (`/painel`) e os comandos que alteram o serviço são só para os operadores do
  Urna Velox, com login próprio. Visitantes nunca precisam dele.
* **Uso do conteúdo.** Termos de uso e licença no [LICENCIAMENTO.md](LICENCIAMENTO.md) (uso livre sem fins comerciais;
  uso comercial exige licença).

## 3. O que é registrado, para quê e por quanto tempo

| o quê | onde fica | para quê | retenção |
|---|---|---|---|
| Preferências da página: tema claro/escuro, modo da comparação com o TSE, cartão de identificação recolhido, o dia em que o aviso inicial já foi mostrado, a vista e a cor do mapa das seções e a ordem dos cartões (`uv-tema`, `uv-comparar`, `uv-ident`, `uv-aviso-dia`, `uv-mapa`, `uv-cartoes`) | **Só no seu navegador** (armazenamento local); nunca é enviado para nós | Lembrar as suas escolhas na próxima visita | Até você limpar os dados do site no navegador |
| Registro técnico de acesso: endereço IP, data e hora, endereço pedido, resposta e identificação do navegador (*user-agent*) | Nos nossos servidores | Manter o serviço no ar: diagnosticar falhas, medir carga e se defender de abuso e ataques | Apagado automaticamente por rotação: o registro novo sobrescreve o antigo, num volume fixo e pequeno, e não há cópia de longo prazo |
| Voto na playlist de replays: o IP **transformado num código irreversível** (hash com sal secreto; o IP em si não é guardado) e a opção escolhida | Nos nossos servidores | Garantir um voto por pessoa e permitir trocar ou retirar o voto | Apagado quando o replay votado entra no ar ou quando você retira o voto |
| Números agregados de operação: quantas conexões estão abertas, mensagens por minuto, uso de memória e processador | Nos nossos servidores | Melhorar o desempenho e planejar a capacidade da plataforma | Sem dado pessoal: são só contagens, sem IP nem identificação de visitante |

Não usamos esses registros para traçar perfil, fazer publicidade, decidir nada automaticamente sobre ninguém ou cruzar
com outras bases. Não há cookies: nem nossos, nem de terceiros.

## 4. Compartilhamento

**Não vendemos, não alugamos e não compartilhamos informação pessoal dos visitantes** com empresas, partidos, campanhas,
institutos de pesquisa, anunciantes ou qualquer outra pessoa. Para entregar a página, o tráfego passa pela
infraestrutura dos provedores de hospedagem e de rede que contratamos (servidores em nuvem e, no subdomínio de dados,
uma rede de entrega de conteúdo); eles só transportam e entregam o conteúdo, não recebem de nós nenhuma base de dados de
visitantes. Só entregaríamos um registro técnico a uma autoridade diante de ordem judicial, como manda a lei.

## 5. Seus direitos

Pela LGPD você pode pedir confirmação de tratamento, acesso, correção ou eliminação dos seus dados. Como quase nada fica
guardado e o pouco que fica não traz nome nem contato, na prática:

* as preferências você mesmo apaga, limpando os dados do site no seu navegador;
* o voto da playlist você retira na própria playlist;
* os registros técnicos somem sozinhos com a rotação.

Dúvidas ou pedidos: escreva para o titular do projeto, Luciano Soares Bohnert, em `luciano.bohnert@gmail.com` (o mesmo
contato da licença).

## 6. Segurança

Todo o acesso é por HTTPS. Os servidores só respondem por uma lista fixa de endereços, a página não executa nada de
terceiros e o acesso às máquinas e ao painel é restrito aos operadores.

## 7. Mudanças nesta política

Se mudarmos o que é registrado, esta política muda antes, com a data nova no topo. Nunca passaremos a vender ou
compartilhar dados pessoais dos visitantes.
