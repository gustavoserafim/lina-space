# ADR 0066 — Shell em colunas, fase 1: geometria única, topo fixo e caixa de pedido no rodapé

- **Status:** ✅ Aceito (2026-09-29, fundador — direção aprovada sobre o mockup; decisões: visão padrão
  Grade, coluna do Time sempre visível, paleta terrosa mantida)
- **Depende de / toca:** ADR 0053 (shell de colunas — este ADR entrega o D2-1), ADR 0055 (chrome reto,
  hairline), ADR 0029 (câmera fora do log), ADR 0028 (live-region), ADR 0061 (caixa de pedido)

## Contexto

O screenshot do fundador mostrou o defeito estrutural que o ADR 0053 previa: rail, topo e rodapé eram
camadas `.absolute()` sobre o canvas, pintadas em ordem própria. O rail (pintado por último) cobria os
primeiros 52 px do topo e do rodapé ("Lina Space" virava "Space"); os cards passavam por baixo do topo
e do rodapé; o topo QUEBRAVA em várias linhas (altura imprevisível); e os gestos de canvas (pan, zoom)
viviam na raiz, então clicar no topo ou rolar sobre um painel mexia no canvas por baixo. A lição do
épico vale: "coluna" é estrutura de layout onde o canvas ENCOLHE, nunca overlay flutuante.

## Decisão

1. **A geometria é uma função pura** (`shell::layout`): dada a janela, a largura do rail, a da coluna
   do Time (0 até a Fase 2) e a altura da faixa de avisos, devolve rail · topo · avisos · **área dos
   agentes** · caixa de pedido · coluna do Time. Elas ladrilham a janela sem sobreposição e a área dos
   agentes nunca fica negativa (testado). O render e a câmera leem esta mesma fonte.
2. **Estrutura real:** o `render` monta uma linha flex `rail | coluna principal`; a coluna principal é
   `topo (44 px, uma linha) · faixa de avisos (32 px, só se houver) · área dos agentes (flex, RECORTA)
   · caixa de pedido (80 px)`. O rail EMPURRA o conteúdo (280 px expandido, 52 px colapsado) em vez de
   cobri-lo.
3. **A área dos agentes recorta** (`overflow_hidden`) uma "camada do mundo" deslocada por −origem. A
   câmera continua em coordenadas de JANELA (hit-test, seleção, arrasto e snap intactos); o que muda é
   o recorte e o **home**: o mundo (0,0) nasce no canto da área dos agentes. Revelar, enquadrar,
   recentrar e o zoom por teclado passam a usar o retângulo da área (`shell::to_local/from_local`).
   Os gestos de canvas (pan, mover card, roda) agora vivem na área dos agentes; mover/soltar seguem na
   raiz para o arrasto continuar quando o cursor sai dela.
4. **Topo que não quebra:** marca · resumo do time · "salvo" · sino · Pausar time · Poderes · Visual ·
   Centralizar · Ajustes · **Novo agente** (único primário, terracota). Modos por largura: cheio, só
   ícones e mínimo (o resto vive na paleta ⌘K). Saíram a frase de instrução, "203 registros" e o
   "Time conectado" fixo.
5. **Faixa de avisos:** custódia, teto de custo, recuperação, erro, nota em criação e pausa viram UMA
   linha (o de maior prioridade, com "+N"), sobre o live-region do catálogo (ADR 0028; bloqueios que
   exigem ação interrompem, o resto é cortês). Sem aviso a faixa some e a área dos agentes ganha a altura.
6. **Rodapé → caixa de pedido.** "Pausar cooperação" vira "Pausar time" no topo (âmbar quando ativo);
   "Animações" vira comando da paleta (⌘K); a legenda longa some (a faixa de avisos explica a pausa).
7. **Sem borda pontilhada** ao redor do canvas (era o "aura" do time conectado; lia-se como resíduo de
   depuração). O rail ganha hairline à direita e o topo/caixa de pedido, hairline (ADR 0055).

## Segurança e portas

Nenhuma decisão de identidade, ordem ou autorização muda. Os gates humanos (⌘⏎, fila de atenção,
custódia) seguem intactos — o aviso de custódia continua visível e agora interrompe o leitor de tela.
A coluna do Time (Fase 2) já tem lugar no layout (`team_w`); `PortalEngine`/`UiHost` não são tocados.

## Verificação

- `shell::layout`: regiões ladrilham e não se sobrepõem em 4 tamanhos de janela; janela minúscula não
  produz retângulo negativo; `to_local`/`from_local` fazem ida e volta; `home` põe o mundo no canto da
  área; o aviso escolhe o de maior prioridade e conta o resto; modos do topo por largura.
- Conformidade a11y (ADR 0028) e catraca de tokens verdes; a catraca REGISTROU queda (o `main.rs`
  perdeu literais ao mover o chrome) e o snapshot foi apertado.
- Boot real do app: sem pânico, Maestro criado e `camera.json` gravado com o home da área dos agentes.
- **Não verificado na tela** (sem acesso à captura): proporções, o recorte na borda e o comportamento
  do rail expandido dependem do olho do fundador — pedir screenshot.
