# ADR 0067 — Coluna do Time, fase 2: quem está no Espaço, o que faz e o que espera por você

- **Status:** ✅ Aceito (2026-09-29, fundador — aprovou a Fase 1 e mandou seguir; decisão herdada do
  mockup: a coluna do Time é **sempre visível**)
- **Depende de / toca:** ADR 0066 (geometria do shell), ADR 0021 §6 e ADR 0004 (decisão humana),
  ADR 0028 (live-region), ADR 0055 (chrome reto, hairline)

## Contexto

Depois da Fase 1 o canvas tem estrutura de colunas, mas a pergunta central do leigo — "quem está
trabalhando, quem precisa de mim?" — ainda só se responde olhando card por card, ou abrindo o painel da
fila (sino ⌘J). A informação existe (estado de cada nó, fila de atenção, freio), só estava espalhada e
escondida atrás de cliques.

## Decisão

1. **Terceira coluna, em fluxo:** `rail | coluna principal | coluna do Time`. Ela ocupa a altura
   inteira; o canvas ENCOLHE por causa dela (nada de painel flutuante). A largura vem de
   `shell::team_width` (280 → 240 → 208 px conforme a janela) e ela só **some** quando a coluna
   principal ficaria abaixo de ~500 px; nesse caso o topo volta a mostrar o resumo do time e o
   "Pausar time" (o freio nunca desaparece). Testado em toda a faixa de larguras.
2. **Conteúdo, de cima para baixo:** cabeçalho na altura do topo ("Time" + resumo: *N precisam de
   você › N trabalhando › tudo tranquilo*) · bloco fixo **"Precisa de você"** · lista dos agentes ·
   **Pausar/Retomar time** no rodapé (o botão saiu do topo enquanto a coluna existe).
3. **Lista dos agentes:** só terminais, **na ordem de chegada** ao Espaço. Estado NÃO reordena — a
   lista não pula sob o mouse; o que exige você sobe para o bloco fixado. Cada linha: indicador na cor
   semântica do card (mesma leitura), nome, linha fina *modelo · esforço · conversa N%* e a pílula do
   estado **com a mesma palavra do card e do leitor de tela** (`canvas::aggregate_badge`). Clicar
   foca o agente e leva a câmera até ele.
4. **"Precisa de você" fixado (até 3, «+N na fila» abre o painel):** cada pedido oferece
   **exatamente** o que o painel da fila já oferece, classificado por `team::actions_for`:
   - permissão sim/não e nascimento de agente: **Aprovar / Recusar** (mesmos `attention_approve` /
     `attention_deny`, que só registram a decisão — nenhum byte vai ao PTY);
   - **custódia**: Aprovar/Recusar **só quando o pedido está na FRENTE do gate** (o mesmo efeito do
     ⌘⏎/⌘⇧⏎: `confirm_requested`/`reject_requested`); fora da frente, só a dica. O clique decide o
     pedido que estava **na tela**: se a frente mudou entre o desenho e o clique, nada é decidido;
   - perguntas (escolha/confiança), ask do guard, conflito de código, entrega parada, mensagem
     guardada: **"→ Ir até o terminal"** — nunca um Aprovar remoto (R2b).
5. **Agente "precisa de você"** = pedido *bloqueante* na fila (custódia, nascimento, permissão, ask do
   guard) ou gate dele na mesa. Avisos (entrega parada, conflito, mensagem guardada) aparecem na fila
   mas não pintam o agente de vermelho.
6. **O resto do chrome se afasta da coluna:** toast e painel da fila, painel de custos, selos de
   exposição/WhatsApp, painéis de metas e de mentalidade ancoram à direita da **coluna principal**;
   toast de arquivamento e selos de baixo ficam **acima da caixa de pedido**. (Corrige um resíduo da
   Fase 1: o toast ficava 56 px acima do fundo, sobre o topo da caixa de pedido de 80 px, e o selo de
   exposição caía sobre o botão "Novo agente".)

## Segurança e portas

Nenhuma decisão nova de identidade, ordem ou autorização. A coluna é **apresentação** sobre a fila e o
gate que já existem; nenhum campo escrito por agente decide o que aparece (a lista vem do modelo de
nós; o texto do pedido é o mesmo `toast_copy` do painel, cortado em 110 caracteres só para caber — a
frase inteira segue no aria e no painel). `UiHost`/`PortalEngine` não são tocados.

## Ressalvas conhecidas

- O card ainda pinta "precisa de você" só pelo gate de custódia; a linha do Time também considera
  permissão e ask do guard. Os dois convergem na Fase 4 (cabeçalho do card).
- A palavra do estado "rodando" cobre também *iniciando* (voz herdada do card); o esqueleto
  "Iniciando…" entra na Fase 4 nos dois lugares ao mesmo tempo.

## Verificação

- `shell::team_width`: nunca passa de 280 e, visível, sempre deixa ≥ 500 px à coluna principal
  (varredura 400–2600 px, rail 52 e 280); o layout continua ladrilhando a janela.
- `team::*` (puros): tom = indicador do card; só agentes, ordem estável; rótulo = voz do card; cabeçalho
  prioriza atenção › trabalho › calma; só pedido bloqueante marca o agente; custódia só decide na
  frente; tipos "só terminal" nunca oferecem Aprovar; teto de 3 + contagem; corte por caractere.
- Conformidade a11y (ADR 0028) e catraca de tokens verdes; arquivos novos com zero literais.
- **Não verificado na tela** (sem acesso à captura): proporções, rolagem da lista e o bloco fixado
  dependem do olho do fundador — pedir screenshot.
