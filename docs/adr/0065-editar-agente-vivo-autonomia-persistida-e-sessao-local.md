# ADR 0065 — Editar um agente vivo, autonomia persistida e sessão local (notas + câmera)

- **Status:** ✅ Aceito (2026-09-28, fundador — pendências da Fatia A + melhoria de UX)
- **Depende de / toca:** ADR 0022 (admissão canônica), ADR 0029 §3 (câmera fora do log), ADR 0031
  addendum (modelo/esforço), ADR 0037/0039 (restart por pasta/motor), F3-5-2 (`--resume` por nó),
  ADR 0007 (dado ≠ autoridade), doutrina "ação irreversível exige gate humano"

## Contexto

Quatro coisas que a Fatia A deixou pela metade, todas achadas ao usar o app:

1. **Editar autonomia não fazia nada.** O Editar registrava a escolha e imprimia "aguarda a porta de
   autonomia no NodeManager" — o agente vivo seguia no nível antigo. Modelo e raciocínio nem
   apareciam no Editar.
2. **A autonomia escolhida se perdia ao reabrir:** todo restore/reinício religava `assistido`.
3. **Notas e pastas sumiam do canvas ao reabrir** (continuavam no log e no disco): só a criação as
   inseria no model vivo.
4. **A câmera (pan/zoom) voltava ao "home"** em todo boot e em toda troca de Espaço — o store
   (`camera.json`, ADR 0029 §3) existia no core, mas nada o chamava.

## Decisão

1. **`relaunch_node` — um só caminho para "mudar como o agente roda".** O processo de um CLI não troca
   cwd, motor, modelo nem `LINA_AUTONOMY` por fora; então mudar qualquer um = encerrar o vivo e
   re-erguer um sucessor na mesma posição, com o mesmo nome e papel (o `restart_node_in_dir` do
   ADR 0037/0039 vira este caminho, com mais campos). `Relaunch` diz o que muda; o que for `None`
   mantém o valor gravado.
   - **A conversa continua** (`--resume <sessão do nó>`, verbo lido do profile — inv. #3) quando motor
     e pasta não mudam e há sessão salva; senão é um recomeço. A sessão do CLI mora por pasta, por
     isso trocar a pasta recomeça.
   - O Editar mostra o modelo e o raciocínio (antes só na criação) e avisa, com a voz certa, que
     Salvar reinicia o agente: "a conversa continua" (motor que retoma) ou "recomeça do zero" (motor
     que não retoma), e que uma tarefa em andamento é interrompida.
2. **`NodeAutonomySet { node, level }`** grava a autonomia do nó **só quando difere do default
   (`assistido`)**. Log antigo e admissões comuns ficam idênticos (sem upcast, sem mudar a sequência
   canônica de eventos). O restore e o reinício relêem o último por nó — o agente volta com o nível
   que o humano escolheu.
   - **Segurança:** o evento só nasce no app (admissão de um nó / relançamento pelo modal). Nenhum
     verbo `lina` nem intent de agente o emite, então um agente não se auto-concede autonomia
     (ADR 0007). Nível ilegível no log é ignorado (volta a `assistido`).
3. **Notas e pastas voltam no boot:** `restore_artifacts` recria no model, da projeção do log, cada
   `Note`/`Folder` ainda aberto, na posição gravada (o fechado — `NodeRemoved` — não ressuscita;
   idempotente).
4. **Câmera por Espaço:** `camera_sync` carrega o `camera.json` do Espaço ativo (zoom preso aos
   limites da UI; disco insano volta ao home), salva **quando a câmera assenta** (mesma em dois
   ticks de 0,7s — nunca a cada quadro de arrasto) e grava na hora ao trocar de Espaço.

## Melhorias de UX no mesmo passo (sem tela para validar — cobertas por teste)

- **Resumo do time no topo** ("5 agentes · 2 trabalhando · 1 precisa de você") no lugar do selo fixo
  "Time conectado" — âmbar quando algo espera pelo usuário.
- **Caixa de pedido:** "Enviar" esmaece sem texto; dica de teclas quando em foco; ↑/↓ recuperam
  pedidos anteriores (até 20) sem perder o rascunho.
- **Ajuda de atalhos (⌘/ e paleta ⌘K)** com teste que confere cada atalho listado contra o código.
  A paleta ganha "Escrever um pedido para o time" (1ª da lista) e "Atalhos do teclado".

## Portas

Nenhuma fechada. `Relaunch` aceita campos novos sem mudar os chamadores; `NodeAutonomySet` é aditivo;
o store de sessão local segue fora do log e descartável.

## Verificação

- `relaunch_node` com o binário-sonda real: `--resume sess-42 --model opus --effort high` com sessão
  salva; sem sessão ou em outra pasta, sem `--resume`; autonomia/esforço/modelo do sucessor no log.
- Editar: plano só traz o que mudou; só renomear não reinicia; autonomia sozinha reinicia; pasta nova
  sem "reiniciar agora" não reinicia; o aviso acompanha o motor retomar ou não.
- Boot real com nota gravada devolve a nota ao canvas na posição gravada; a fechada não volta.
- Câmera: só grava ao assentar; recarrega; cada Espaço com a sua; zoom/pan insanos domados.
