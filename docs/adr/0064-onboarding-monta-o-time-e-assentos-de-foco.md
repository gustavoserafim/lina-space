# ADR 0064 — O onboarding monta o time; assentos de Foco nascem como agentes

- **Status:** ✅ Aceito (2026-09-26, fundador — Fatia C da análise de usabilidade)
- **Depende de / toca:** W4-1 (onboarding), W4-5 (galeria de Focos), F1-4-3 (restore), ADR 0022
  (admissão canônica), ADR 0031 addendum (modelo por papel), ADR 0061 (Maestro default)

## Contexto

1. O passo final do onboarding ("Criar meu Espaço") gravava um `WorkspaceCreated` chamado "Meu
   primeiro Espaço" num log DESCARTÁVEL — nenhum Espaço real nascia dali, e o usuário ia para o
   canvas vazio de "Meu Espaço", criado no boot. Duas identidades para a mesma coisa, nenhuma com time.
2. Os Focos (App, Pesquisa & Conteúdo) gravam cada membro do time só como `NodeAdded` + papel — sem
   nome e sem CLI. No boot, o restore os re-erguia como shells "Terminal A/B/C…": o time prometido
   na galeria nunca chegava como agentes.

## Decisão

1. **O passo final do onboarding vira "Monte seu time".** O usuário escolhe um Foco (cartões com o
   time de cada um) e o canvas monta os agentes NO Espaço já aberto, pelo funil canônico
   (`create_agent_with_autonomy` → `admit_node`): o Maestro (se ainda não há um vivo) e os membros do
   preset, com o motor default e o modelo/esforço sugeridos pelo papel. Idempotente por nome. O
   onboarding não conhece o runtime: recebe um `TeamBuilder` do canvas. "Seguir sem time" conclui sem
   gravar nada — fim do Espaço fantasma.
2. **Assento de Foco = nó que nunca foi lançado** (sem nome, sem CLI, sem status na projeção). No
   restore ele nasce como agente: nome = rótulo leigo do papel (sem colidir), motor = o default
   descoberto no boot, modelo/esforço = sugestão do papel. Sem motor instalado, degrada ao shell de
   antes. O formato gravado na criação do Espaço NÃO muda — criações em andamento de versões
   anteriores continuam válidas no validador.

## Portas

Nenhuma fechada. Um "assento" continua sendo só `NodeAdded` + papel no log; quem decide como ele
nasce é o restore, que pode passar a oferecer escolha de motor por assento no futuro.

## Verificação

- Onboarding: seguir sem time não grava `WorkspaceCreated`; a montagem trava contra duplo clique,
  falha fica no passo com o motivo e sucesso conclui com a lista de quem chegou.
- `team_to_create`: Maestro uma vez, membros sem `@`, nomes existentes pulados; Em Branco = só Maestro.
- Restore de um Foco App: todos os assentos com nome de papel e motor Claude; Arquiteto em `opus`/
  `high`, QA em `sonnet`; sem motor default, seguem como shell.
