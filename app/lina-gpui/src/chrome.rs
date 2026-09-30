//! `chrome` — o **chrome do shell em colunas** (Fase 1 da modernização da UI): o topo, a faixa de
//! avisos e a caixa de pedido. Antes viviam inline no `render` de 1.600 linhas do `main.rs` como
//! camadas `.absolute()` (topo que quebrava em várias linhas, rodapé com legenda fixa); agora são
//! regiões de altura FIXA (`shell::TOPBAR_H`, `NOTICE_H`, `COMPOSER_H`) — a geometria que a câmera
//! também usa.
//!
//! Regras de design (ADRs 0053–0055): uma linha, nunca quebra; um só botão primário (terracota);
//! secundários neutros; o que não cabe vira ícone e, no limite, vai para a paleta ⌘K.

use gpui::{
    div, prelude::*, px, rgb, text, AnyElement, ClickEvent, Context, Div, FontWeight, Role,
    Stateful,
};

use crate::a11y_live::{live_region, Politeness};
use crate::bridge::lock;
use crate::shell::{self, Notice, NoticeTone, TopbarMode};
use crate::theme::Theme;
use crate::ui::RadiusExt;
use crate::{a11y, attention_ui, creators, WorkspaceView};

impl WorkspaceView {
    /// O aviso da faixa (uma linha sob o topo): o de MAIOR prioridade, com `+N` se houver outros.
    /// Ordem: custódia (o humano decide) › teto de custo › recuperação › erro › nota/pasta em
    /// criação › pausa. Sem aviso a faixa some (altura 0) e a área dos agentes ganha o espaço.
    pub(crate) fn shell_notice(&self, recovering: bool, cost_paused: bool) -> Option<Notice> {
        let mut list: Vec<Notice> = Vec::new();
        let (banner, pending) = {
            let d = lock(&self.desk);
            (d.banner(), d.front().is_some())
        };
        if let Some(text) = banner {
            list.push(Notice {
                // Pedido na frente da fila aguarda ⌘⏎ (âmbar); senão é o resultado da execução (verde).
                tone: if pending {
                    NoticeTone::Warning
                } else {
                    NoticeTone::Confirm
                },
                text,
            });
        }
        if cost_paused {
            list.push(Notice {
                tone: NoticeTone::Danger,
                text: "Teto de custo atingido — o Espaço está pausado. Rode `lina resume` e confirme com ⌘⏎."
                    .to_owned(),
            });
        }
        if recovering {
            list.push(Notice {
                tone: NoticeTone::Danger,
                text: "Recuperando o Espaço…".to_owned(),
            });
        }
        if let Some(error) = &self.human_intent_error {
            list.push(Notice {
                tone: NoticeTone::Danger,
                text: error.clone(),
            });
        }
        if let Some((kind, buf)) = &self.creating {
            let what = match kind {
                creators::CreatorKind::Note => "Nova nota",
                creators::CreatorKind::Folder => "Nova pasta",
            };
            list.push(Notice {
                tone: NoticeTone::Neutral,
                text: format!("{what}: {buf}▌   Enter cria · Esc cancela"),
            });
        }
        if lock(&self.brake).paused {
            list.push(Notice {
                tone: NoticeTone::Warning,
                text: "Time em pausa — novas delegações ficam na fila (nada se perde). Retome quando quiser."
                    .to_owned(),
            });
        }
        shell::pick_notice(list)
    }

    /// A faixa de avisos: uma linha, texto com «…» se não couber, cores dos pares validados.
    pub(crate) fn render_notice_strip(notice: &Notice, th: &Theme) -> AnyElement {
        let (bg, fg) = match notice.tone {
            NoticeTone::Warning => (th.state.warning, th.text.on_emphasis),
            NoticeTone::Danger => (th.state.danger, th.text.on_emphasis),
            NoticeTone::Confirm => (th.accent.confirm, th.text.on_accent),
            NoticeTone::Neutral => (th.surface.raised_alt, th.text.bright),
        };
        // O aviso nasce SOBRE o live-region do catálogo (ADR 0028 — retrofit proibido): o leitor de
        // tela o anuncia quando aparece ou muda. Bloqueio que exige ação (custódia, teto de custo)
        // interrompe (`Assertive`); o resto é cortês.
        let politeness = match notice.tone {
            NoticeTone::Warning | NoticeTone::Danger => Politeness::Assertive,
            NoticeTone::Confirm | NoticeTone::Neutral => Politeness::Polite,
        };
        let visual = div()
            .flex()
            .flex_none()
            .items_center()
            .w_full()
            .h(px(shell::NOTICE_H))
            .px_4()
            .bg(rgb(bg))
            .text_color(rgb(fg))
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .child(text!(notice.text.clone())),
            );
        live_region("shell-notice", notice.text.clone(), politeness, visual).into_any_element()
    }

    /// O topo: uma linha de altura fixa. Esquerda: marca, resumo do time, "salvo". Direita: sino,
    /// pausa, atalhos secundários e o ÚNICO botão primário. `mode` decide rótulo × ícone × essencial.
    pub(crate) fn render_topbar(
        &self,
        mode: TopbarMode,
        summary: Option<String>,
        needs_you: bool,
        team_visible: bool,
        th: &Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let full = mode == TopbarMode::Full;
        let minimal = mode == TopbarMode::Minimal;
        // Botão secundário neutro: ícone + rótulo (largo) ou só o ícone (compacto). O rótulo SEMPRE
        // viaja no aria — nunca só ícone para o leitor de tela.
        let sec_btn = |id: &'static str, icon: &'static str, label: &'static str| {
            let shown = if full {
                format!("{icon} {label}")
            } else {
                icon.to_owned()
            };
            div()
                .id(id)
                .flex_none()
                .whitespace_nowrap()
                .px_3()
                .py_1()
                .rounded_content()
                .bg(rgb(th.surface.raised))
                .text_color(rgb(th.text.primary))
                .cursor_pointer()
                .role(Role::Button)
                .aria_label(label)
                .child(text!(shown))
        };

        let mut bar = div()
            .flex()
            .flex_none()
            .flex_row()
            .items_center()
            .gap_2()
            .w_full()
            .h(px(shell::TOPBAR_H))
            .px_4()
            .overflow_hidden()
            .bg(rgb(th.surface.chrome))
            .border_b_1()
            .border_color(rgb(th.surface.border))
            .text_color(rgb(th.text.primary))
            .child(
                div()
                    .flex_none()
                    .whitespace_nowrap()
                    .font_weight(FontWeight(f32::from(th.typography.weight.semibold)))
                    .child(text!("Lina Space")),
            );

        // Resumo do time (quem trabalha / quem espera por você): âmbar se algo espera, verde se bem.
        if let (Some(summary), false) = (summary.filter(|_| !team_visible), minimal) {
            let tone = if needs_you {
                th.state.warning
            } else {
                th.state.success
            };
            bar = bar.child(
                div()
                    .id("team-summary")
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .aria_label(format!("Resumo do time: {summary}"))
                    .child(
                        div()
                            .flex_none()
                            .size(px(f32::from(th.spacing.sm)))
                            .rounded_full()
                            .bg(rgb(tone)),
                    )
                    .child(
                        div()
                            .min_w(px(0.0))
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .text_color(rgb(tone))
                            .child(text!(summary)),
                    ),
            );
        }
        if full {
            bar = bar.child(
                div()
                    .id("saved-indicator")
                    .flex_none()
                    .whitespace_nowrap()
                    .text_color(rgb(th.text.muted))
                    .aria_label("Tudo salvo — o histórico do Espaço está gravado no disco")
                    .child(text!("✓ tudo salvo")),
            );
        }
        bar = bar.child(div().flex_1());

        // O ⚡ A2A (A→B) é demo-only e só aparece com AMBOS os alvos vivos.
        if full && self.a2a.as_ref().is_some_and(|a| a.ready()) {
            bar = bar.child(
                div()
                    .id("a2a-btn")
                    .flex_none()
                    .px_3()
                    .py_1()
                    .rounded_content()
                    .bg(rgb(th.accent.action))
                    .text_color(rgb(th.text.on_accent))
                    .cursor_pointer()
                    .on_click(cx.listener(|view, _ev: &ClickEvent, _w, _cx| {
                        if let Some(a) = &view.a2a {
                            a.fire(
                                "echo '📨 A2A recebido de Terminal A · cooperacao sem fios'"
                                    .to_owned(),
                            );
                        }
                    }))
                    .child(text!("⚡ Enviar A2A (A→B)")),
            );
        }

        // 🔔 Fila de Atenção: pendências reais; pulsa com item escalado (suprimido em reduce-motion).
        bar = bar.child(attention_ui::render_badge(
            &self.attention_items,
            self.attention_panel_open,
            a11y::reduce_motion_effective(self.reduce_motion),
            lina_core::now_ms(),
            th,
            cx,
        ));

        // ⏸ Pausar / ▶ Retomar o time — com a coluna do Time visível o botão mora no rodapé DELA
        // (Fase 2); sem coluna (janela estreita) ele fica aqui, para o freio nunca sumir.
        if !team_visible {
            bar = bar.child(self.brake_button(full, th, cx));
        }

        if !minimal {
            // F2-4-3+4: Área de Poderes. Ao ABRIR, faz o scan-ao-abrir e preenche o painel.
            bar = bar.child(sec_btn("powers-btn", "⚡", "Poderes").on_click(cx.listener(
                |view, _ev: &ClickEvent, _w, cx| {
                    view.powers_panel_open = !view.powers_panel_open;
                    if view.powers_panel_open {
                        view.refresh_powers_inventory();
                    }
                    cx.notify();
                },
            )));
            // F2-4-5: galeria de Direções Visuais.
            bar = bar.child(sec_btn("visual-btn", "🎨", "Visual").on_click(cx.listener(
                |view, _ev: &ClickEvent, _w, cx| {
                    view.design_gallery_open = !view.design_gallery_open;
                    cx.notify();
                },
            )));
            // Centralizar: resgata a vista (mesmo efeito do ⌘0) — o leigo nunca fica perdido.
            bar = bar.child(
                sec_btn("home-btn", "🏠", "Centralizar").on_click(cx.listener(
                    |view, _ev: &ClickEvent, _w, cx| {
                        view.camera = shell::home(view.canvas_rect);
                        cx.notify();
                    },
                )),
            );
        }
        // Ajustes (⌘,): descobrível, nunca só atrás de atalho.
        bar = bar.child(
            sec_btn("settings-btn", "⚙", "Ajustes").on_click(cx.listener(
                |view, _ev: &ClickEvent, _w, cx| {
                    view.open_settings_window(cx);
                },
            )),
        );

        // O ÚNICO primário do topo (terracota). No modo mínimo só o "+", com o rótulo no aria.
        bar = bar.child(
            div()
                .id("new-agent-btn")
                .flex_none()
                .whitespace_nowrap()
                .px_3()
                .py_1()
                .rounded_content()
                .bg(rgb(th.accent.create))
                .text_color(rgb(th.text.on_accent))
                .cursor_pointer()
                .role(Role::Button)
                .aria_label("Novo agente")
                .on_click(cx.listener(|view, _ev: &ClickEvent, _w, cx| {
                    view.open_agent_modal_create(cx);
                }))
                .child(text!(if minimal { "✦" } else { "✦ Novo agente" })),
        );
        bar.into_any_element()
    }

    /// ⏸ Pausar / ▶ Retomar o time — o "freio" da cooperação. Só SINALIZA: a MailboxPump aplica
    /// Router::pause/resume no próximo tick. Pausado, o botão fica âmbar (estado ativo) e a faixa de
    /// avisos explica o que acontece. `with_label` = ícone + rótulo (senão só o ícone; o rótulo
    /// sempre viaja no aria). O mesmo id serve ao topo e à coluna do Time (só um existe por vez).
    pub(crate) fn brake_button(
        &self,
        with_label: bool,
        th: &Theme,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let paused = lock(&self.brake).paused;
        let (icon, label) = if paused {
            ("▶", "Retomar time")
        } else {
            ("⏸", "Pausar time")
        };
        let shown = if with_label {
            format!("{icon} {label}")
        } else {
            icon.to_owned()
        };
        let (bg, fg) = if paused {
            (th.state.warning, th.text.on_emphasis)
        } else {
            (th.surface.raised, th.text.primary)
        };
        div()
            .id("freio-btn")
            .flex()
            .flex_none()
            .items_center()
            .whitespace_nowrap()
            .px_3()
            .py_1()
            .rounded_content()
            .bg(rgb(bg))
            .text_color(rgb(fg))
            .cursor_pointer()
            .role(Role::Button)
            .aria_label(if paused {
                "Retomar a cooperação dos agentes"
            } else {
                "Pausar a cooperação: os agentes param de delegar tarefas entre si (nada se perde)"
            })
            .on_click(cx.listener(|view, _ev: &ClickEvent, _w, _cx| {
                lock(&view.brake).toggle_requested = true;
            }))
            .child(text!(shown))
    }

    /// A caixa de pedido no rodapé da coluna principal: região de altura fixa, o campo já cuida do
    /// texto longo (termina em «…») e da linha de dica/aviso.
    pub(crate) fn render_composer(&self, th: &Theme, cx: &mut Context<Self>) -> AnyElement {
        div()
            .id("composer")
            .flex_none()
            .w_full()
            .h(px(shell::COMPOSER_H))
            .px_4()
            .py_2()
            .overflow_hidden()
            .bg(rgb(th.surface.chrome))
            .border_t_1()
            .border_color(rgb(th.surface.border))
            .child(self.render_request_box(cx))
            .into_any_element()
    }
}
