# ADR 0061 — Maestro nasce com o Espaço + caixa de pedido como entrada única

- **Status:** ✅ Aceito (2026-09-26, fundador — Fatia A da análise de usabilidade)
- **Depende de / toca:** ADR 0022 (admissão canônica de nó), ADR 0031 (modelo/esforço no spawn, addendum), ADR 0036 (identidade do gesto humano na UI), F3-2-1 (`entry_origin`: Tradutor → Maestro)

## Contexto

A doutrina gerada em todo terminal (`assets/lina-doctrine/*.md`) manda os agentes reportarem e
pedirem ajuda a `@Maestro` (`lina ask "@Maestro" …`). Mas nenhum caminho de criação de Espaço
produz um Maestro: o primeiro boot nasce `Blank` e sem seed, e os Focos (App, Pesquisa) não o
incluem. Sem Maestro no roster, essas mensagens falham com `NoTarget`, são re-tentadas e
descartadas — o time trabalha isolado sem que o leigo perceba.

Do lado do humano, não existe lugar óbvio para "fazer um pedido": o empty state ensina `⌘N`, o
card de Goal só confirma/corrige, e a paleta não tem ação de pedido. O leigo digita em qualquer
terminal e o pedido não chega a quem coordena.

## Decisão

1. **Maestro no primeiro boot de um Espaço novo.** Ao subir o runtime de um Espaço cujo log
   **nunca teve um `TerminalSpawned`** e **nunca atribuiu o papel `MAESTRO`**, o boot admite um
   agente "Maestro" pelo funil canônico (`admit_node` via `create_agent_with_autonomy`), com o
   motor default descoberto (Claude primeiro) e a sugestão de lançamento do papel (faixa `top`,
   esforço `high` — ADR 0031 addendum). Sem CLI instalado: não cria (degrada, nunca panica); a
   caixa de pedido oferece criar depois. `LINA_DEMO` não é afetado.
   - A regra é derivada **só do log** (inv. #4): sem evento novo, sem flag em disco. Um Espaço
     que já rodou terminais nunca ganha um Maestro-surpresa; um Maestro removido pelo humano não
     renasce sozinho (o papel já foi atribuído no log).
2. **Caixa de pedido no rodapé do canvas.** Um campo único "O que você quer que o time faça?"
   entrega o texto ao terminal de entrada do roster vivo — `@Tradutor` se existir, senão
   `@Maestro` (`lina_role_discovery::entry_origin`, regra já existente). A entrega é **input
   humano** (mesma classe de confiança do ⌘V + Enter: `WriteOp::HumanKeys` pela fila serial do
   Supervisor, colagem com bracketed-paste e Enter separado após o `submit_delay` do profile).
   Não passa por `deliver_a2a`/`Router` e não cria intent humano novo.
   - Sem terminal de entrada vivo, a caixa mostra "Criar o Maestro" em vez de enviar.
   - Digitar direto em qualquer terminal continua funcionando — a caixa é o caminho recomendado,
     não o único.

## Segurança

- A caixa não confere autoridade nova: é teclado humano endereçado a um nó escolhido pela UI a
  partir do papel **projetado** (não de conteúdo escrito por agente). Nenhum campo de agente decide
  o destino.
- O papel `Tradutor`/`Maestro` continua rótulo de proveniência, jamais credencial (F3-2-1).
- O Maestro automático nasce com a autonomia default do produto (`Assisted`) — gates humanos
  intactos.

## Portas

Não fecha nenhuma: Maestro é só um papel por nome (troca de CLI/modelo segue pelo profile), e a
caixa reusa o caminho de input humano. Uma entrada estruturada futura (Goal com critérios,
`goal.define` humano pelo Router) pode substituir a entrega por teclado sem mudar a superfície.

## Verificação

- Espaço novo (log sem `TerminalSpawned`/`MAESTRO`) → `needs_first_maestro` verdadeiro; qualquer
  um dos dois presentes → falso (teste puro sobre registros).
- Resolução do destino da caixa: Tradutor vivo vence Maestro; sem nenhum → `None` (teste puro).
- Na tela: Espaço novo abre com o card "Maestro" rodando o Claude com `--model opus --effort high`;
  texto na caixa + Enter aparece no terminal do Maestro e é submetido.
