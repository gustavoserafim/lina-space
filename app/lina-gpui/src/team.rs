//! `team` — a **coluna do Time** (Fase 2 da modernização da UI, ADR 0067): quem está no Espaço, o
//! que cada um está fazendo, o que espera por você — sempre à vista, ao lado do canvas.
//!
//! Estrutura da coluna (de cima para baixo): cabeçalho (mesma altura do topo) · bloco fixo
//! "Precisa de você" (só se houver) · lista dos agentes (rola por dentro) · "Pausar time".
//!
//! A metade de cima deste arquivo é **PURA** (sem gpui): quais linhas existem, o tom de cada estado,
//! o resumo do cabeçalho, quais ações cada pedido oferece. A metade de baixo é a casca gpui fina.
//!
//! **Segurança (ADR 0021 §6, ADR 0004):** a coluna NÃO cria caminho novo de decisão. Cada pedido
//! oferece exatamente o que o painel da fila já oferece — permissão sim/não registra a decisão pelos
//! mesmos `attention_approve`/`attention_deny`; a custódia usa o MESMO gate do ⌘⏎/⌘⇧⏎ (só quando o
//! pedido está na frente da fila, e um clique nunca decide um pedido diferente do que está na tela);
//! perguntas, conflitos e avisos só levam ao terminal. Nenhum campo escrito por agente decide nada.

use lina_core::{AttentionItem, AttentionKind};
use lina_host::{NodeId, NodeKind, NodeStatus};

use crate::attention_ui;

/// Quantos pedidos ficam fixados na coluna; o resto vira «+N na fila» (abre o painel da fila).
pub const MAX_PINNED: usize = 3;
/// Teto de caracteres da frase de um pedido fixado (a frase inteira segue no painel e no aria).
const PINNED_COPY_MAX: usize = 110;

// ═══════════════════════════ linhas do time (PURO) ═══════════════════════════

/// O tom semântico de um agente — a MESMA leitura de cor do indicador do card (vermelho = precisa de
/// você · âmbar = trabalhando · verde = pronto · acento = pausado/iniciando · vermelho = encerrado).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    NeedsYou,
    Working,
    Ready,
    Paused,
    Stopped,
    Starting,
}

/// Tom do agente. "Precisa de você" vence o estado (igual ao card).
#[must_use]
pub fn tone_of(status: NodeStatus, needs_you: bool) -> Tone {
    if needs_you {
        return Tone::NeedsYou;
    }
    match status {
        NodeStatus::Idle => Tone::Ready,
        NodeStatus::Busy | NodeStatus::Running => Tone::Working,
        NodeStatus::Blocked => Tone::Paused,
        NodeStatus::Crashed | NodeStatus::Dead => Tone::Stopped,
        // Starting e qualquer estado futuro do core (`#[non_exhaustive]`): vivo, neutro.
        _ => Tone::Starting,
    }
}

/// Este tipo de pedido é um "precisa de você" de verdade? Custódia, nascimento de agente, permissão
/// e ask do guard param o trabalho até você decidir; entrega sem progresso, conflito e mensagem
/// guardada são AVISOS (aparecem na fila, mas não pintam o agente de vermelho).
#[must_use]
pub fn is_blocking(kind: AttentionKind) -> bool {
    matches!(
        kind,
        AttentionKind::Custody
            | AttentionKind::Spawn
            | AttentionKind::Permission
            | AttentionKind::GuardAsk
    )
}

/// O agente `name` tem um pedido bloqueante na fila de atenção ou um gate de custódia dele na mesa?
/// (`desk_requesters` = nomes dos solicitantes da fila do gate — mesma fonte do card.)
#[must_use]
pub fn agent_needs_you(name: &str, desk_requesters: &[String], items: &[AttentionItem]) -> bool {
    desk_requesters.iter().any(|r| r == name)
        || items
            .iter()
            .any(|i| i.node_id == name && is_blocking(i.kind))
}

/// Entrada de uma linha: o que o render já sabe de cada nó (sem tipos de gpui).
#[derive(Debug, Clone)]
pub struct RowInput {
    pub node: NodeId,
    pub name: String,
    pub kind: NodeKind,
    pub status: NodeStatus,
    pub needs_you: bool,
    /// «equilibrado · modelo» e/ou «conversa N%» — a linha fina sob o nome.
    pub meta: Option<String>,
    pub focused: bool,
}

/// Uma linha pronta para desenhar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamRow {
    pub node: NodeId,
    pub name: String,
    pub tone: Tone,
    /// O estado na voz do leigo — a MESMA palavra do card e do leitor de tela.
    pub label: String,
    pub meta: Option<String>,
    pub focused: bool,
}

/// Linhas da coluna: SÓ agentes (terminais), na ordem de chegada ao Espaço — estável: uma linha não
/// pula de lugar quando o estado dela muda (o que precisa de você sobe para o bloco fixado, não para
/// o topo da lista).
#[must_use]
pub fn build_rows(inputs: &[RowInput]) -> Vec<TeamRow> {
    inputs
        .iter()
        .filter(|i| matches!(i.kind, NodeKind::Terminal))
        .map(|i| TeamRow {
            node: i.node,
            name: i.name.clone(),
            tone: tone_of(i.status, i.needs_you),
            label: crate::canvas::aggregate_badge(i.status, i.needs_you, 0).label(),
            meta: i.meta.clone(),
            focused: i.focused,
        })
        .collect()
}

/// Linha fina sob o nome: modelo/esforço e quão cheia está a conversa. `None` se nada a dizer.
#[must_use]
pub fn meta_line(effort: Option<&str>, context_percent: Option<u8>) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    if let Some(e) = effort.filter(|e| !e.is_empty()) {
        parts.push(e.to_owned());
    }
    if let Some(p) = context_percent {
        parts.push(format!("conversa {p}%"));
    }
    (!parts.is_empty()).then(|| parts.join(" · "))
}

/// O resumo do cabeçalho: prioridade de atenção › trabalho › calma.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderStatus {
    pub text: String,
    pub tone: Tone,
}

#[must_use]
pub fn header_status(rows: &[TeamRow]) -> HeaderStatus {
    let needs = rows.iter().filter(|r| r.tone == Tone::NeedsYou).count();
    let working = rows.iter().filter(|r| r.tone == Tone::Working).count();
    if needs > 0 {
        HeaderStatus {
            text: if needs == 1 {
                "1 precisa de você".to_owned()
            } else {
                format!("{needs} precisam de você")
            },
            tone: Tone::NeedsYou,
        }
    } else if working > 0 {
        HeaderStatus {
            text: if working == 1 {
                "1 trabalhando".to_owned()
            } else {
                format!("{working} trabalhando")
            },
            tone: Tone::Working,
        }
    } else {
        HeaderStatus {
            text: "tudo tranquilo".to_owned(),
            tone: Tone::Ready,
        }
    }
}

// ═══════════════════════════ pedidos fixados (PURO) ═══════════════════════════

/// O que a coluna oferece para um pedido — um subconjunto do que o painel da fila já oferece.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PinnedAction {
    /// Custódia na FRENTE do gate: confirma (o mesmo efeito do ⌘⏎).
    ApproveGate,
    /// Custódia na FRENTE do gate: recusa (o mesmo efeito do ⌘⇧⏎).
    RejectGate,
    /// Permissão sim/não (ou nascimento de agente): registra a decisão.
    Approve,
    Deny,
    /// Pergunta, conflito ou aviso: a decisão é no terminal — a coluna só leva até lá.
    Goto,
}

/// Ações de um pedido. Espelha a classificação do painel da fila (`attention_ui::render_panel`):
/// custódia só decide na frente do gate; "goto-only" só navega; o resto aprova/recusa.
#[must_use]
pub fn actions_for(item: &AttentionItem, desk_front: Option<&str>) -> Vec<PinnedAction> {
    if item.kind == AttentionKind::Custody {
        return if desk_front == Some(item.stable_id.as_str()) {
            vec![PinnedAction::ApproveGate, PinnedAction::RejectGate]
        } else {
            Vec::new()
        };
    }
    if attention_ui::is_goto_only(item) {
        vec![PinnedAction::Goto]
    } else {
        vec![PinnedAction::Approve, PinnedAction::Deny]
    }
}

/// Custódia que ainda não está na frente do gate: só a dica (decide ao chegar na frente).
pub const GATE_WAIT_HINT: &str = "na fila do gate — decide ao chegar na frente";

/// Corta a frase do pedido para caber no bloco fixado, com «…» (a frase inteira segue no aria e no
/// painel da fila). Conta caracteres, nunca bytes (acentos).
#[must_use]
pub fn short_copy(text: &str) -> String {
    let text = text.trim();
    if text.chars().count() <= PINNED_COPY_MAX {
        return text.to_owned();
    }
    let cut: String = text.chars().take(PINNED_COPY_MAX).collect();
    format!("{}…", cut.trim_end())
}

/// Quantos pedidos ficam fixados e quantos esperam na fila.
#[must_use]
pub fn pinned_split(total: usize) -> (usize, usize) {
    let shown = total.min(MAX_PINNED);
    (shown, total - shown)
}

/// Rótulo do botão «+N na fila».
#[must_use]
pub fn more_label(hidden: usize) -> String {
    format!("+{hidden} na fila")
}

// ═══════════════════════════ render gpui (casca FINA sobre as puras) ═══════════════════════════

use gpui::{div, prelude::*, px, rgb, text, AnyElement, ClickEvent, Context, FontWeight, Role};

use crate::bridge::lock;
use crate::shell;
use crate::theme::Theme;
use crate::ui::RadiusExt;
use crate::WorkspaceView;

/// Cor do indicador de um tom (a mesma leitura do card).
fn tone_color(tone: Tone, th: &Theme) -> u32 {
    match tone {
        Tone::NeedsYou | Tone::Stopped => th.state.danger,
        Tone::Working => th.state.warning,
        Tone::Ready => th.state.success,
        Tone::Paused | Tone::Starting => th.accent.primary,
    }
}

/// Par (fundo, texto) da pílula de estado — só pares já validados no gate de contraste.
fn pill_colors(tone: Tone, th: &Theme) -> (u32, u32) {
    match tone {
        Tone::NeedsYou | Tone::Stopped => (th.state.danger, th.text.on_emphasis),
        Tone::Working => (th.state.warning, th.text.on_emphasis),
        Tone::Ready => (th.state.success, th.text.on_emphasis),
        Tone::Paused | Tone::Starting => (th.accent.primary, th.text.on_accent),
    }
}

/// Botão pequeno de ação dos pedidos fixados (id único + role + aria — teclado/leitor de tela).
fn action_btn<F>(
    cx: &mut Context<WorkspaceView>,
    id: impl Into<gpui::ElementId>,
    label: &str,
    aria: String,
    (bg, fg): (u32, u32),
    on_click: F,
) -> AnyElement
where
    F: Fn(&mut WorkspaceView, &mut gpui::Window, &mut Context<WorkspaceView>) + 'static,
{
    div()
        .id(id.into())
        .flex_none()
        .whitespace_nowrap()
        .px_2()
        .py_1()
        .rounded_content()
        .bg(rgb(bg))
        .text_color(rgb(fg))
        .cursor_pointer()
        .role(Role::Button)
        .aria_label(aria)
        .on_click(cx.listener(move |v, _ev: &ClickEvent, w, cx| on_click(v, w, cx)))
        .child(text!(label.to_owned()))
        .into_any_element()
}

impl WorkspaceView {
    /// Custódia: confirma/recusa o pedido `stable_id` **só se ele ainda é a frente do gate**. O
    /// clique decide o que estava NA TELA; se a frente mudou entre o desenho e o clique (outro pedido
    /// chegou primeiro), nada é decidido. Mesmo canal do ⌘⏎/⌘⇧⏎ (`confirm_requested`/`reject_requested`).
    pub(crate) fn decide_gate(&mut self, stable_id: &str, approve: bool, cx: &mut Context<Self>) {
        let mut d = lock(&self.desk);
        if d.front().map(|p| p.id().to_string()).as_deref() != Some(stable_id) {
            eprintln!("lina-gpui: coluna do Time — frente do gate mudou; clique ignorado");
            return;
        }
        if approve {
            d.confirm_requested = Some(stable_id.to_owned());
        } else {
            d.reject_requested = Some(stable_id.to_owned());
        }
        drop(d);
        eprintln!(
            "lina-gpui: GATE HUMANO — pedido da frente {} na coluna do Time",
            if approve { "confirmado" } else { "RECUSADO" }
        );
        cx.notify();
    }

    /// O bloco fixo "Precisa de você": até `MAX_PINNED` pedidos, cada um com as ações que a fila
    /// já oferece, e «+N na fila» abrindo o painel. `None` sem pedidos.
    fn render_pinned(&self, th: &Theme, cx: &mut Context<Self>) -> Option<AnyElement> {
        if self.attention_items.is_empty() {
            return None;
        }
        let now = lina_core::now_ms();
        let desk_front = lock(&self.desk).front().map(|p| p.id().to_string());
        let (shown, hidden) = pinned_split(self.attention_items.len());
        let mut block = div()
            .id("team-pinned")
            .flex()
            .flex_col()
            .flex_none()
            .gap_2()
            .p_3()
            .border_b_1()
            .border_color(rgb(th.surface.border))
            .child(
                div()
                    .whitespace_nowrap()
                    .text_size(px(f32::from(th.typography.size.small)))
                    .text_color(rgb(th.state.danger))
                    .font_weight(FontWeight(f32::from(th.typography.weight.semibold)))
                    .child(text!("Precisa de você")),
            );
        for (i, item) in self.attention_items.iter().take(shown).enumerate() {
            let idx = i as u64;
            let copy = attention_ui::toast_copy(item);
            let mut card = div()
                .id(("team-pin", idx))
                .flex()
                .flex_col()
                .gap_1()
                .p_2()
                .rounded_content()
                .bg(rgb(th.surface.card))
                .border_1()
                .border_color(rgb(th.state.danger))
                .aria_label(format!("{}: {copy}", item.node_id))
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .flex_1()
                                .min_w(px(0.0))
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .text_ellipsis()
                                .font_weight(FontWeight(f32::from(th.typography.weight.semibold)))
                                .text_color(rgb(th.text.bright))
                                .child(text!(item.node_id.clone())),
                        )
                        .child(
                            div()
                                .flex_none()
                                .whitespace_nowrap()
                                .text_size(px(f32::from(th.typography.size.caption)))
                                .text_color(rgb(th.text.secondary))
                                .child(text!(attention_ui::age_label(item.created_ts, now))),
                        ),
                )
                .child(
                    div()
                        .text_size(px(f32::from(th.typography.size.small)))
                        .text_color(rgb(th.text.secondary))
                        .child(text!(short_copy(&copy))),
                );
            let actions = actions_for(item, desk_front.as_deref());
            if item.kind == AttentionKind::Custody && actions.is_empty() {
                card = card.child(
                    div()
                        .text_size(px(f32::from(th.typography.size.small)))
                        .text_color(rgb(th.text.secondary))
                        .child(text!(GATE_WAIT_HINT)),
                );
            }
            if !actions.is_empty() {
                let mut row = div().flex().flex_row().flex_wrap().gap_2();
                for a in actions {
                    let sid = item.stable_id.clone();
                    let node = item.node_id.clone();
                    row = row.child(match a {
                        PinnedAction::ApproveGate => action_btn(
                            cx,
                            ("team-gate-ok", idx),
                            "✓ Aprovar",
                            format!("Aprovar o pedido de {node}"),
                            (th.accent.confirm, th.text.on_accent),
                            move |v, _w, cx| v.decide_gate(&sid, true, cx),
                        ),
                        PinnedAction::RejectGate => action_btn(
                            cx,
                            ("team-gate-no", idx),
                            "✗ Recusar",
                            format!("Recusar o pedido de {node}"),
                            (th.state.danger, th.text.on_emphasis),
                            move |v, _w, cx| v.decide_gate(&sid, false, cx),
                        ),
                        PinnedAction::Approve => action_btn(
                            cx,
                            ("team-approve", idx),
                            "✓ Aprovar",
                            format!("Aprovar o pedido de {node}"),
                            (th.accent.confirm, th.text.on_accent),
                            move |v, _w, cx| v.attention_approve(&sid, cx),
                        ),
                        PinnedAction::Deny => action_btn(
                            cx,
                            ("team-deny", idx),
                            "✗ Recusar",
                            format!("Recusar o pedido de {node}"),
                            (th.state.danger, th.text.on_emphasis),
                            move |v, _w, cx| v.attention_deny(&sid, cx),
                        ),
                        PinnedAction::Goto => action_btn(
                            cx,
                            ("team-goto", idx),
                            "→ Ir até o terminal",
                            format!("Ir até o terminal de {node}"),
                            (th.accent.action, th.text.on_accent),
                            move |v, w, cx| v.attention_goto_node(&node, w, cx),
                        ),
                    });
                }
                card = card.child(row);
            }
            block = block.child(card);
        }
        if hidden > 0 {
            block = block.child(action_btn(
                cx,
                "team-pin-more",
                &more_label(hidden),
                "Abrir a fila de atenção com todos os pedidos".to_owned(),
                (th.surface.raised, th.text.primary),
                |v, _w, cx| v.attention_open_panel(cx),
            ));
        }
        Some(block.into_any_element())
    }

    /// Uma linha de agente: indicador de estado, nome, linha fina (modelo · conversa) e a pílula do
    /// estado. Clicar foca e leva a câmera até o agente.
    fn render_team_row(
        &self,
        i: usize,
        row: &TeamRow,
        th: &Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let node = row.node;
        let (pill_bg, pill_fg) = pill_colors(row.tone, th);
        let mut label_col = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w(px(0.0))
            .overflow_hidden()
            .child(
                div()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .text_color(rgb(if row.focused {
                        th.text.bright
                    } else {
                        th.text.primary
                    }))
                    .font_weight(FontWeight(f32::from(if row.focused {
                        th.typography.weight.semibold
                    } else {
                        th.typography.weight.regular
                    })))
                    .child(text!(row.name.clone())),
            );
        if let Some(meta) = &row.meta {
            label_col = label_col.child(
                div()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .text_size(px(f32::from(th.typography.size.caption)))
                    .text_color(rgb(th.text.secondary))
                    .child(text!(meta.clone())),
            );
        }
        let aria = format!("{}: {}. Ir até este agente", row.name, row.label);
        let hover_bg = th.surface.raised;
        div()
            .id(("team-row", i as u64))
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .px_2()
            .py_2()
            .rounded_content()
            .bg(rgb(if row.focused {
                th.surface.raised
            } else {
                th.surface.chrome
            }))
            .hover(move |s| s.bg(rgb(hover_bg)))
            .cursor_pointer()
            .role(Role::Button)
            .aria_label(aria)
            .on_click(cx.listener(move |view, _ev: &ClickEvent, window, cx| {
                view.focus(node);
                view.reveal(node, window);
                cx.notify();
            }))
            .child(
                div()
                    .flex_none()
                    .size(px(f32::from(th.spacing.sm)))
                    .rounded_full()
                    .bg(rgb(tone_color(row.tone, th))),
            )
            .child(label_col)
            .child(
                div()
                    .flex_none()
                    .whitespace_nowrap()
                    .px_2()
                    .rounded_content()
                    .bg(rgb(pill_bg))
                    .text_color(rgb(pill_fg))
                    .text_size(px(f32::from(th.typography.size.caption)))
                    .child(text!(row.label.clone())),
            )
            .into_any_element()
    }

    /// A coluna do Time inteira. `w` vem de `shell::team_width` (a mesma fonte da geometria).
    pub(crate) fn render_team_column(
        &self,
        w: f32,
        rows: &[TeamRow],
        th: &Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let status = header_status(rows);
        let header = div()
            .flex()
            .flex_none()
            .flex_row()
            .items_center()
            .gap_2()
            .h(px(shell::TOPBAR_H))
            .px_3()
            .border_b_1()
            .border_color(rgb(th.surface.border))
            .child(
                div()
                    .flex_none()
                    .whitespace_nowrap()
                    .font_weight(FontWeight(f32::from(th.typography.weight.semibold)))
                    .child(text!("Time")),
            )
            .child(div().flex_1())
            .child(
                div()
                    .id("team-header-status")
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .aria_label(format!("Resumo do time: {}", status.text))
                    .child(
                        div()
                            .flex_none()
                            .size(px(f32::from(th.spacing.sm)))
                            .rounded_full()
                            .bg(rgb(tone_color(status.tone, th))),
                    )
                    .child(
                        div()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .text_size(px(f32::from(th.typography.size.small)))
                            .text_color(rgb(th.text.secondary))
                            .child(text!(status.text.clone())),
                    ),
            );

        let mut list = div()
            .id("team-list")
            .flex()
            .flex_col()
            .flex_1()
            .min_h(px(0.0))
            .gap_1()
            .p_2()
            .overflow_y_scroll();
        if rows.is_empty() {
            list = list.child(
                div()
                    .p_2()
                    .text_size(px(f32::from(th.typography.size.small)))
                    .text_color(rgb(th.text.secondary))
                    .child(text!(
                        "Seu time aparece aqui. Escreva um pedido na caixa abaixo — o Maestro monta o time."
                    )),
            );
        }
        for (i, row) in rows.iter().enumerate() {
            list = list.child(self.render_team_row(i, row, th, cx));
        }

        let mut col = div()
            .id("team-column")
            .flex()
            .flex_col()
            .flex_none()
            .w(px(w))
            .h_full()
            .overflow_hidden()
            .bg(rgb(th.surface.chrome))
            .border_l_1()
            .border_color(rgb(th.surface.border))
            .text_color(rgb(th.text.primary))
            .child(header);
        if let Some(pinned) = self.render_pinned(th, cx) {
            col = col.child(pinned);
        }
        col.child(list)
            .child(
                div()
                    .flex()
                    .flex_none()
                    .p_3()
                    .border_t_1()
                    .border_color(rgb(th.surface.border))
                    .child(self.brake_button(true, th, cx).w_full().justify_center()),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lina_core::{AttentionEvidence, AttentionState};

    fn id(n: u128) -> NodeId {
        NodeId::from_u128(n)
    }

    fn input(n: u128, name: &str, status: NodeStatus, needs_you: bool) -> RowInput {
        RowInput {
            node: id(n),
            name: name.to_owned(),
            kind: NodeKind::Terminal,
            status,
            needs_you,
            meta: None,
            focused: false,
        }
    }

    fn item(sid: &str, node: &str, kind: AttentionKind) -> AttentionItem {
        AttentionItem {
            stable_id: sid.into(),
            node_id: node.into(),
            kind,
            detail: Some(format!("detalhe {sid}")),
            evidence: AttentionEvidence::Hook,
            created_ts: 0,
            state: AttentionState::Pending,
            prompt_kind: lina_core::attention::PromptKind::Yn,
            vt_snapshot_hash: None,
        }
    }

    /// O tom espelha o indicador do card: "precisa de você" vence; o resto segue o estado.
    #[test]
    fn tone_mirrors_the_card_indicator() {
        assert_eq!(tone_of(NodeStatus::Idle, true), Tone::NeedsYou);
        assert_eq!(tone_of(NodeStatus::Dead, true), Tone::NeedsYou);
        assert_eq!(tone_of(NodeStatus::Idle, false), Tone::Ready);
        assert_eq!(tone_of(NodeStatus::Busy, false), Tone::Working);
        assert_eq!(tone_of(NodeStatus::Running, false), Tone::Working);
        assert_eq!(tone_of(NodeStatus::Blocked, false), Tone::Paused);
        assert_eq!(tone_of(NodeStatus::Crashed, false), Tone::Stopped);
        assert_eq!(tone_of(NodeStatus::Dead, false), Tone::Stopped);
        assert_eq!(tone_of(NodeStatus::Starting, false), Tone::Starting);
    }

    /// Só agentes entram na lista (nota/pasta não), na ordem de chegada — e trocar o estado de um
    /// NÃO muda a ordem (a lista não "pula" sob o mouse).
    #[test]
    fn rows_are_agents_only_in_stable_arrival_order() {
        let mut note = input(9, "Nota", NodeStatus::Idle, false);
        note.kind = NodeKind::Note;
        let a = vec![
            input(1, "Maestro", NodeStatus::Busy, false),
            note.clone(),
            input(2, "QA", NodeStatus::Idle, false),
        ];
        let rows = build_rows(&a);
        assert_eq!(
            rows.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(),
            ["Maestro", "QA"]
        );
        let b = vec![
            input(1, "Maestro", NodeStatus::Idle, false),
            note,
            input(2, "QA", NodeStatus::Busy, true),
        ];
        let rows_b = build_rows(&b);
        assert_eq!(
            rows_b.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(),
            ["Maestro", "QA"],
            "mudar estado não reordena"
        );
        assert_eq!(rows_b[1].tone, Tone::NeedsYou);
    }

    /// O rótulo do estado é a MESMA voz do card/leitor de tela (`aggregate_badge`).
    #[test]
    fn label_uses_the_shared_status_voice() {
        let rows = build_rows(&[
            input(1, "A", NodeStatus::Idle, false),
            input(2, "B", NodeStatus::Dead, false),
            input(3, "C", NodeStatus::Idle, true),
        ]);
        assert_eq!(
            rows[0].label,
            crate::canvas::aggregate_badge(NodeStatus::Idle, false, 0).label()
        );
        assert_eq!(rows[1].label, "encerrado");
        assert_eq!(
            rows[2].label,
            crate::canvas::aggregate_badge(NodeStatus::Idle, true, 0).label(),
            "precisa de você vence o estado"
        );
    }

    #[test]
    fn meta_line_joins_only_what_is_known() {
        assert_eq!(meta_line(None, None), None);
        assert_eq!(meta_line(Some(""), None), None);
        assert_eq!(
            meta_line(Some("equilibrado · opus"), None).as_deref(),
            Some("equilibrado · opus")
        );
        assert_eq!(meta_line(None, Some(42)).as_deref(), Some("conversa 42%"));
        assert_eq!(
            meta_line(Some("rápido"), Some(7)).as_deref(),
            Some("rápido · conversa 7%")
        );
    }

    /// O cabeçalho prioriza: quem espera por você › quem trabalha › calma. Singular e plural.
    #[test]
    fn header_status_prioritises_attention_then_work_then_calm() {
        let calm = build_rows(&[input(1, "A", NodeStatus::Idle, false)]);
        assert_eq!(header_status(&calm).text, "tudo tranquilo");
        assert_eq!(header_status(&calm).tone, Tone::Ready);
        let one = build_rows(&[
            input(1, "A", NodeStatus::Busy, false),
            input(2, "B", NodeStatus::Idle, false),
        ]);
        assert_eq!(header_status(&one).text, "1 trabalhando");
        let many = build_rows(&[
            input(1, "A", NodeStatus::Busy, false),
            input(2, "B", NodeStatus::Busy, false),
        ]);
        assert_eq!(header_status(&many).text, "2 trabalhando");
        let needs = build_rows(&[
            input(1, "A", NodeStatus::Busy, true),
            input(2, "B", NodeStatus::Idle, true),
            input(3, "C", NodeStatus::Busy, false),
        ]);
        assert_eq!(header_status(&needs).text, "2 precisam de você");
        assert_eq!(header_status(&needs).tone, Tone::NeedsYou);
        assert_eq!(header_status(&[]).text, "tudo tranquilo");
    }

    /// Só pedido que PARA o trabalho pinta o agente de "precisa de você"; aviso não.
    #[test]
    fn only_blocking_requests_mark_an_agent_as_needing_you() {
        let items = vec![
            item("p1", "QA", AttentionKind::Permission),
            item("d1", "Dev", AttentionKind::DeliveredNoProgress),
            item("c1", "Arquiteto", AttentionKind::CodeConflict),
        ];
        assert!(agent_needs_you("QA", &[], &items));
        assert!(!agent_needs_you("Dev", &[], &items), "aviso não bloqueia");
        assert!(!agent_needs_you("Arquiteto", &[], &items));
        assert!(
            agent_needs_you("Dev", &["Dev".to_owned()], &[]),
            "gate na mesa"
        );
        assert!(!agent_needs_you("Ninguém", &[], &items));
    }

    /// Custódia: botões só na FRENTE do gate; fora dela, nenhuma ação (só a dica).
    #[test]
    fn custody_offers_gate_buttons_only_at_the_front() {
        let it = item("gate-1", "Dev", AttentionKind::Custody);
        assert_eq!(
            actions_for(&it, Some("gate-1")),
            [PinnedAction::ApproveGate, PinnedAction::RejectGate]
        );
        assert!(actions_for(&it, Some("gate-0")).is_empty());
        assert!(actions_for(&it, None).is_empty());
    }

    /// Permissão sim/não aprova/recusa; pergunta, ask do guard, conflito e entrega parada só levam ao
    /// terminal — NUNCA um Aprovar remoto (mesma regra do painel da fila).
    #[test]
    fn goto_only_kinds_never_offer_approve() {
        let yn = item("p", "QA", AttentionKind::Permission);
        assert_eq!(
            actions_for(&yn, None),
            [PinnedAction::Approve, PinnedAction::Deny]
        );
        let mut choice = item("q", "QA", AttentionKind::Permission);
        choice.prompt_kind = lina_core::attention::PromptKind::Choice;
        assert_eq!(actions_for(&choice, None), [PinnedAction::Goto]);
        for kind in [
            AttentionKind::GuardAsk,
            AttentionKind::DeliveredNoProgress,
            AttentionKind::CodeConflict,
            AttentionKind::DeadLetter,
        ] {
            assert_eq!(
                actions_for(&item("x", "N", kind), Some("x")),
                [PinnedAction::Goto],
                "{kind:?}"
            );
        }
        // Nascimento de agente (Spawn): aprovar deixa nascer, recusar não nasce nada (funil da fila).
        assert_eq!(
            actions_for(&item("s", "Maestro", AttentionKind::Spawn), None),
            [PinnedAction::Approve, PinnedAction::Deny]
        );
    }

    #[test]
    fn pinned_block_caps_at_three_and_counts_the_rest() {
        assert_eq!(pinned_split(0), (0, 0));
        assert_eq!(pinned_split(2), (2, 0));
        assert_eq!(pinned_split(3), (3, 0));
        assert_eq!(pinned_split(7), (3, 4));
        assert_eq!(more_label(4), "+4 na fila");
    }

    /// A frase longa é cortada por CARACTERE (acento não quebra) com «…»; a curta fica intacta.
    #[test]
    fn short_copy_cuts_on_characters_not_bytes() {
        assert_eq!(short_copy("  ok  "), "ok");
        let long = "ç".repeat(PINNED_COPY_MAX + 40);
        let cut = short_copy(&long);
        assert!(cut.ends_with('…'));
        assert_eq!(cut.chars().count(), PINNED_COPY_MAX + 1);
        let exact = "a".repeat(PINNED_COPY_MAX);
        assert_eq!(short_copy(&exact), exact, "no limite não corta");
    }
}
