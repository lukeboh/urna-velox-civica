# Guia Rápido de Uso da Marca — Urna Velox

*Aprovado em 15/10/2026. Este documento estabelece as regras oficiais de aplicação visual do símbolo e logotipo.*

---

## 1. O Símbolo Oficial

O símbolo do **Urna Velox** é a letra "U" construída com traço monolítico encorpado, caracterizada por:
1. **Hastes com alturas assimétricas escalonadas:** a haste esquerda se eleva como vetor de estabilidade institucional e a haste direita é rebaixada em 12 px (ou 96 px na escala mestre 1024), conferindo dinamismo e velocidade de processamento.
2. **Serifas com ângulo agudo voltadas para a esquerda:** ambas as hastes são coroadas por pontas afiadas agudas projetadas para a esquerda, remetendo a aletas aerodinâmicas que cortam o vento e aceleram o fluxo de dados em nanossegundos (Rust).

---

## 2. Paleta de Cores e Gradientes

### Gradiente Oficial do Símbolo (Tema Escuro / Padrão)
* **0% (Início):** `#4fc3ff` (Ciano Velox)
* **50% (Meio):** `#8c6bff` (Violeta Velox)
* **100% (Fim):** `#ff5fd2` (Magenta Velox)
* **Ângulo:** Linear diagonal (de cima à esquerda para baixo à direita).

### Fundo Escuro Oficial
* **Fundo do Ícone:** `#0d1431` (Azul-noite profundo)
* **Borda do Ícone:** `rgba(255, 255, 255, 0.15)`
* **Filete de Luz Superior:** `rgba(255, 255, 255, 0.32)` (efeito vidro tátil)

### Gradiente Oficial para Fundo Claro (Contraste Aumentado $\ge 4.5:1$)
* **0% (Início):** `#0094d4`
* **50% (Meio):** `#6940d6`
* **100% (Fim):** `#d6229e`
* **Fundo do Ícone Claro:** `#f6f7fc`
* **Borda do Ícone Claro:** `rgba(30, 40, 90, 0.15)`

### Halo Arco-Íris Giratório (Destaques e Animação Opcional)
* `conic-gradient(#ff5f7e, #ffb84d, #f9f871, #4fe0b0, #4fc3ff, #8c6bff, #ff5fd2, #ff5f7e)`
* Ciclo de rotação: 5 segundos por volta. Sempre respeitar `prefers-reduced-motion` com estado estático.

---

## 3. Tipografia Institucional do Logotipo

* **Família:** **Archivo** (variável, licença SIL Open Font License).
* **"Urna":** Archivo Bold/Black (peso `800`, largura/stretch `125%`), cor `#eef1ff` (escuro) ou `#121836` (claro).
* **"Velox":** Archivo Light/Thin (peso `300`, largura/stretch `100%`), cor `#a8b2d8` (escuro) ou `#4a5378` (claro).
* **Espaçamento entre letras (letter-spacing):** `-0.02em`.

---

## 4. Área de Proteção e Tamanhos Mínimos

* **Área de Proteção ($X$):** O espaçamento mínimo em volta do símbolo ou logotipo equivale à largura da haste ($X = 22\text{ px}$ na escala 128px, ou $175\text{ px}$ na escala 1024px). Nenhum texto, botão ou elemento gráfico pode invadir essa área.
* **Zona de Segurança Maskable (PWA / Android):** O símbolo fica contido dentro de um diâmetro central de 80% do quadro total, com o fundo `#0d1431` sangrando uniformemente até as bordas.
* **Tamanho Mínimo do Símbolo em Tela:** $16 \times 16\text{ px}$ (favicon oficial `site/src/icone.svg`).
* **Tamanho Mínimo do Logotipo Horizontal:** $140\text{ px}$ de largura para garantir legibilidade do subtítulo "Velox".

---

## 5. Arquivos e Diretórios de Entrega

Todos os arquivos estão organizados no repositório em `docs/marca/`:

| Arquivo | Formato / Resolução | Descrição e Finalidade |
|---|---|---|
| `simbolo_mestre_1024.svg` | SVG (1024×1024) | Arquivo mestre vetorial em altíssima definição |
| `simbolo_mestre_grade.svg` | SVG (1024×1024) | Mestre com grade construtiva e cotas de proporção |
| `icone_1024.png` | PNG (1024×1024) | Ícone de aplicativo em alta resolução (stores) |
| `icone_512.png` | PNG (512×512) | Ícone PWA padrão |
| `icone_512_maskable.png` | PNG (512×512) | Ícone PWA maskable (zona segura de 80%) |
| `icone_192.png` | PNG (192×192) | Ícone PWA para telas móveis |
| `icone_180.png` | PNG (180×180) | Apple Touch Icon (iOS Safari) |
| `icone_48.png` | PNG (48×48) | Atalho de desktop e bookmark |
| `icone_32.png` | PNG (32×32) | Favicon de alta densidade |
| `icone_16.png` | PNG (16×16) | Favicon padrão do navegador |
| `icone_monocromatico_branco.svg` | SVG (128×128) | Ícone mono branco para notificações e barras de status |
| `icone_monocromatico_preto.svg` | SVG (128×128) | Ícone mono preto para impressão P&B e documentos |
| `logotipo_horizontal_escuro.svg` | SVG (240×48) | Logotipo oficial horizontal para fundo escuro |
| `logotipo_horizontal_claro.svg` | SVG (240×48) | Logotipo oficial horizontal para fundo claro |
| `logotipo_empilhado_escuro.svg` | SVG (200×200) | Logotipo vertical/empilhado para fundo escuro |
| `logotipo_empilhado_claro.svg` | SVG (200×200) | Logotipo vertical/empilhado para fundo claro |
| `logotipo_horizontal_monocromatico.svg` | SVG (240×48) | Logotipo em `currentColor` adaptativo |

O favicon de produção da aplicação web foi atualizado em:
`site/src/icone.svg` (tamanho: 384 bytes, compatível com a política CSP `default-src 'self'`).

---

## 6. Usos Corretos e Incorretos

### ✅ Permitido e Recomendado
* Usar o símbolo sobre o fundo quadrado arredondado `#0d1431` com o filete de luz superior.
* Usar a versão monocromática quando a interface do sistema operacional ou o tema exigir alto contraste.
* Animar o anel arco-íris girando apenas em momentos de apuração ativa ou marcos atingidos.

### ❌ Estritamente Proibido
* **Não distorcer as proporções:** jamais esticar horizontalmente ou achatar verticalmente.
* **Não alterar a orientação das serifas:** ambas as pontas devem sempre apontar para a esquerda.
* **Não nivelar a haste direita:** o desnível de 12 px (haste direita mais baixa) é a assinatura identitária do Urna Velox.
* **Não usar cores partidárias únicas:** jamais preencher o símbolo com verde-amarelo, vermelho puro ou azul puro isolados. A identidade é 100% neutra e independente.
