# Handoff — Urna Velox: Etapa 8, previsão do matematicamente e do fim da apuração (`0.8.0`, alvo 21/10/2026)

> **Etapa nova (decisão do Luciano, 16/10/2026).** Vem depois da comparação Urna Velox × TSE (etapa 7,
> [HANDOFF_0.7.0.md](/mnt/d/git/lukeboh/urna-velox/docs/HANDOFF_0.7.0.md)) e antes do ensaio geral (etapa 9,
> [HANDOFF_0.9.0.md](/mnt/d/git/lukeboh/urna-velox/docs/HANDOFF_0.9.0.md)). Mapa no
> [ROADMAP](/mnt/d/git/lukeboh/urna-velox/ROADMAP.md).

*Preparado em 16/10/2026. Pode começar antes de a etapa 7 fechar (o motor e o backtest não dependem dela); o
`bump minor` → `0.8.0` só depois da `0.7.0`. Quem executa trabalha em PATCH e fecha com validação independente ou por
decisão explícita do Luciano.*

**Caminhos:** como no [HANDOFF_0.7.0.md](/mnt/d/git/lukeboh/urna-velox/docs/HANDOFF_0.7.0.md); ensaios em
`/home/dev/uvreal/ensaios/etapa8-*`.

## Objetivo

Para cada abrangência e cargo, estimar **a que horas sai o matematicamente** (eleito ou 2º turno) e **a que horas
termina a apuração** (99 % e 100 % das seções), a partir da velocidade com que os BUs estão chegando e da tendência dos
votos, **sempre com um intervalo** ("entre 20h10 e 20h35") e sempre rotulado como **estimativa**. Recalculada ao longo
da noite; depois da definição, a tela mostra o horário real ao lado do que foi previsto.

## Método

1. **Avanço da apuração.** Medir em **eleitores aptos** das seções apuradas (do catálogo), não em seções (seção pequena
   distorce). Série (instante, aptos apurados) por circunscrição, no relógio do serviço (no replay, o relógio simulado).
2. **Velocidade.** Média móvel exponencial de aptos/min com meia-vida de 10 min (`--previsao-meia-vida`), mais a
   dispersão das taxas por minuto numa janela de 30 min (P10 e P90), que alimenta o intervalo.
3. **Projeção dos votos que faltam, estratificada.** A ordem de chegada dos BUs tem viés regional (fuso, capital ×
   interior, exterior), então proporções globais constantes erram. Estrato = a subdivisão logo abaixo da circunscrição
   (UF para Brasil, município para UF, zona para município). Em cada estrato, o restante = aptos não apurados ×
   comparecimento observado × fração de válidos, repartido pelas proporções do próprio estrato **com encolhimento**
   para as da circunscrição quando o estrato tem poucos votos (`(v_e + k·p_circ)/(n_e + k)`, `k` parâmetro).
4. **Momento da definição.** Projetar os votos em instantes futuros (passo de 1 min), com o restante chegando na
   proporção dos aptos que faltam em cada estrato, e aplicar a mesma regra do motor (`matematica::avaliar`, com o
   eleitorado ainda não apurado). O primeiro instante com matematicamente é a estimativa central.
5. **Intervalo.** Monte Carlo com semente fixa (≈ 200 sorteios por rodada): velocidade sorteada da dispersão da janela,
   proporções por estrato sorteadas em torno da estimativa (erro de amostragem + um termo de deriva calibrado nas
   noites anteriores) e ordem de chegada perturbada entre estratos. Publicar P10, P50, P90 e a **probabilidade de
   definir antes do fim** da apuração (perto de 0 = "deve ir até o fim").
6. **Fim da apuração.** A cauda é lenta (exterior, seções remotas), e ritmo constante subestima. Curvas normalizadas de
   fração apurada × tempo das noites que temos (2024 1º e 2º turnos, 2026 1º turno) viram gabaritos; a curva corrente
   é ajustada a eles (deslocamento e escala, nos últimos 30 min) e projetada até 99 % e 100 %; o intervalo vem da
   diferença entre os gabaritos e do erro do ajuste.
7. **Salvaguardas.** Só mostrar com ≥ 5 % dos aptos apurados e ≥ 10 min de série (parâmetros); intervalo maior que 3 h
   aparece como "ainda incerto"; depois da definição, congelar e mostrar "saiu às hh:mm (previsto entre hh:mm e hh:mm
   desde hh:mm)". Nunca usar a palavra "resultado" para a previsão.

## Onde fica cada coisa

* **Motor:** módulo novo `motor/src/previsao.rs`, puro (sem rede, sem relógio próprio: o instante entra como
  parâmetro) e determinístico com a semente; entradas: catálogo, estado, série de avanço. Reaproveita `matematica`
  (`avaliar`, `faltam_para_mat`) e `situacao`.
* **Serviço:** o `uv-servico` roda a previsão a cada 30 s (`--previsao-intervalo`) **fora da thread do motor**, só para
  os tópicos que mudaram, e publica.
* **Protocolo:** mensagem nova `previsao` por tópico (por acréscimo na
  [PROTOCOLO_WS.md](/mnt/d/git/lukeboh/urna-velox/docs/PROTOCOLO_WS.md)): `mat` (tipo, p10, p50, p90,
  probabilidade de definir antes do fim), `fim99` e `fim100` (p10, p50, p90) e a base (aptos/min, % apurado, instante).
* **Página:** cartão "Previsão (estimativa)" com a faixa de horário, o hodômetro e uma faixa P10–P90 da evolução da
  previsão na noite (mostra a previsão convergindo); marco "previsão confirmada: saiu às hh:mm, dentro da faixa
  prevista desde hh:mm" (ou "fora da faixa", com honestidade). CLS 0, como o resto.

## Critério (backtest nas noites reais, só com o que se sabia em cada instante)

Reproduzir as três noites (2026 1º turno, 2024 1º e 2º turnos), emitindo a previsão a cada 30 s do relógio da noite
sem olhar adiante, e comparar com o horário real do matematicamente (o `eventos.csv` do motor) e do fim:

1. **Calibração:** nas previsões emitidas com ≥ 20 % dos aptos apurados, o horário real cai dentro do P10–P90 em
   70 % a 90 % dos casos (meta 80 %), por cargo.
2. **Erro:** erro mediano do P50 relatado por faixa de apuração (20 %, 50 %, 80 %); meta ≤ 10 min a partir de 50 %.
   Se a meta não for atingida, o relatório diz por quê e o Luciano decide.
3. **Custo:** uma rodada para todos os tópicos do 3221 em < 50 ms e para os 5.569 municípios de 2024 em < 1 s, sem
   regressão do BU → publicação.
4. **Site:** replay do 1º turno de 2026 a 60× com o cartão visível: CLS 0, marcos de previsão batendo com o backtest.

Relatório em `docs/RELATORIO_0.8.0.md` (tabelas de calibração e erro, gráficos da faixa ao longo da noite para
Presidente 2026 e alguns prefeitos de 2024); marcar os itens no [ROADMAP](/mnt/d/git/lukeboh/urna-velox/ROADMAP.md).
