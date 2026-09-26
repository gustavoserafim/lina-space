# ADR 0063 — Memória do Espaço (`lina memo`)

- **Status:** ✅ Aceito (2026-09-26, fundador — Fatia B da análise de usabilidade)
- **Depende de / toca:** inv. #4 (log é a fonte), ADR 0007 (contrato é dado, jamais autoridade),
  F3-5-6 (`lina clue`, molde do verbo), W3-2 (bloco `whoami` do SessionStart)

## Contexto

Os agentes trabalham ~95% isolados (notas de campo): cada CLI tem a própria janela e o que um
aprende (decisões, fatos do projeto, preferências do dono) morre na conversa dele. Plano e Goal
existem, mas são tarefas — não fatos. O vault Obsidian é memória do humano, lento para o dia a dia.

## Decisão

1. **Verbo `lina memo`:** `add "<fato>"` registra; `list` e `search "<termos>"` leem. `add` passa
   pela mailbox → router → `MemoNoted { id, text, by }`. `id` e `by` são carimbados SERVER-SIDE (o
   remetente autenticado), nunca do payload (ADR 0007). Texto é DADO, limitado a 1.000 caracteres.
2. **Projeção `SpaceMemory`** (`lina_core::memory`), por replay — sem arquivo próprio de verdade.
   Busca por termos (pontua por termos que aparecem; empate → mais recente). Sem vetor, sem modelo.
3. **Índice empurrado, detalhe puxado:** o bloco `lina whoami --bootstrap` (SessionStart — início de
   sessão e após compactação) ganha a linha `MEMORIA DO ESPACO` com a contagem e as notas mais
   recentes, e a instrução de buscar o resto com `lina memo search`. A doutrina ensina QUANDO anotar
   (decisão tomada, fato do projeto, preferência do dono) e a buscar antes de perguntar.

## Segurança

- Nota de memória é DADO lido por outros agentes: pode conter texto hostil. Ela nunca vira
  instrução do sistema — a doutrina a apresenta como "anotações de colegas", e nada nela decide
  identidade, destino ou autorização.
- `by` identifica quem anotou (auditoria); nenhuma nota pode se passar por outro nó.

## Portas

Não fecha nenhuma: a memória de trajetória (ADR 0057) e o recall por relevância podem consumir o
mesmo `MemoNoted`. Retirar/editar nota é um evento aditivo futuro (`MemoRetired`).

## Verificação

- `memo.add` → `MemoNoted` com `by` = remetente, mesmo com `by` forjado no payload; texto vazio
  rejeitado; texto longo truncado.
- Projeção: ordem, busca por termos, índice das N mais recentes.
- `lina whoami --bootstrap` inclui a linha de memória quando há notas.
