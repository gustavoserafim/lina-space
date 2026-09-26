# ADR 0062 — Medidor de contexto por agente + "Resumir conversa" pelo próprio CLI

- **Status:** ✅ Aceito (2026-09-26, fundador — Fatia B da análise de usabilidade)
- **Depende de / toca:** F1-1-2 (`lina-session-watch`, schema único de sessão), F3-5-2 (sessão
  `--resume` salva por nó), inv. #1 (sem LLM próprio), inv. #3 (neutralidade por CLI Profile)

## Contexto

Cada agente é um CLI com a própria janela de contexto. Quando ela enche, o CLI compacta sozinho
(perdendo detalhe) ou degrada — e o leigo não vê nada disso. O `lina-session-watch` já lê o
`usage` de cada turno, mas só SOMA tokens para custo: não há "quão cheio está este agente agora".

## Decisão

1. **Ocupação = tamanho do contexto do ÚLTIMO turno da conversa principal.** O `Session` ganha
   `context_tokens` = `input_tokens + cache_creation_input_tokens + cache_read_input_tokens` do
   último request não-sidechain (subagentes têm janela própria). É leitura do que o CLI já grava —
   zero modelo, zero rede. Não vai para a projeção SQLite (é estado vivo; re-deriva no próximo poll).
2. **O tamanho da janela vem do CLI Profile** (`context_window_tokens`). Profile sem o campo ⇒ sem
   medidor (nunca um chute). O nó é ligado à sessão pelo `session_id` que o restore já grava por nó
   (F3-5-2) — sem correlação por pasta, que é ambígua com vários agentes na mesma pasta.
3. **O card mostra "contexto N%"** com três faixas: tranquilo (< 70%), atenção (70–85%), cheio
   (≥ 85%). Clicar pede **"Resumir conversa"**: a Lina digita no terminal o comando de compactação
   **do próprio CLI** (`compact_command` do profile — `/compact` no Claude/Codex, `/compress` no
   Gemini), como teclado humano. Quem resume é o CLI; a Lina não tem LLM (inv. #1).
4. Após a compactação, o estado vivo do Espaço volta ao agente pelo hook `SessionStart` já
   instalado (sem matcher ⇒ roda também na origem `compact` do Claude Code). Nada novo aqui.

## Portas

Não fecha nenhuma. Um "passar o bastão" completo (agente grava handoff → terminal renasce limpo com
o handoff injetado) pode vir depois sobre o mesmo medidor; a compactação do CLI é o passo mínimo.

## Verificação

- `context_tokens` = uso do último turno principal; sidechain não altera; request repetido não soma.
- Faixas de 69/70/85% e profile sem janela ⇒ sem medidor (testes puros).
- Na tela: card do Claude mostra a pílula; clique digita `/compact` no terminal certo.
