//! `shortcuts` — a **ajuda de atalhos do teclado** (⌘/ ou "⌨ Atalhos do teclado" na paleta ⌘K).
//!
//! Os atalhos do canvas eram invisíveis: só quem lia o código sabia que existia ⌘L, ⌘R, ⌘J… Aqui
//! mora a lista em linguagem de leigo (gpui-free, testável) e o painel que a mostra sobre o `Modal`
//! do catálogo (role=Dialog, corpo com rolagem, clique fora fecha).
//!
//! **Anti-mentira:** cada linha carrega um `needle` — o trecho de código que a implementa — e um
//! teste confere que ele ainda existe na fonte. Atalho removido/renomeado sem atualizar esta lista
//! quebra o teste (a ajuda nunca promete um atalho que não funciona).

use gpui::{div, prelude::*, px, rgb, AnyElement, ClickEvent, Context, Pixels, Size};

use crate::theme;
use crate::ui::{clamp_frame, Modal, RadiusExt};
use crate::WorkspaceView;

pub const COPY_TITLE: &str = "Atalhos do teclado";
pub const COPY_FOOTER: &str = "Esc fecha esta ajuda.";

/// Um atalho: as teclas, o que ele faz (pt-br leigo) e o trecho de código que o implementa.
#[derive(Debug, Clone, Copy)]
pub struct Shortcut {
    pub keys: &'static str,
    pub what: &'static str,
    /// Trecho que TEM de existir em `main.rs`/`sidebar.rs` — âncora do teste anti-mentira (só o
    /// teste o lê; o painel não).
    #[cfg_attr(not(test), allow(dead_code))]
    pub needle: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub struct Group {
    pub title: &'static str,
    pub items: &'static [Shortcut],
}

pub const GROUPS: &[Group] = &[
    Group {
        title: "Falar com o time",
        items: &[
            Shortcut {
                keys: "⌘ L",
                what: "Escrever um pedido para o time",
                needle: "ks.key == \"l\"",
            },
            Shortcut {
                keys: "⌘ N",
                what: "Criar um agente",
                needle: "ks.key == \"n\"",
            },
            Shortcut {
                keys: "⌘ R",
                what: "Editar o agente selecionado",
                needle: "ks.key == \"r\"",
            },
            Shortcut {
                keys: "⌘ W",
                what: "Fechar o agente selecionado",
                needle: "ks.key == \"w\"",
            },
        ],
    },
    Group {
        title: "Encontrar e navegar",
        items: &[
            Shortcut {
                keys: "⌘ K",
                what: "Buscar qualquer comando",
                needle: "ks.key == \"k\"",
            },
            Shortcut {
                keys: "⌘ O",
                what: "Abrir ou fechar a lista de Espaços",
                needle: "platform && key == \"o\"",
            },
            Shortcut {
                keys: "⌘ 1 … ⌘ 9",
                what: "Ir direto para o Espaço 1 a 9",
                needle: "(1..=9).contains(&n)",
            },
            Shortcut {
                keys: "⌘ 0",
                what: "Voltar à vista inicial do canvas",
                needle: "ks.key == \"0\"",
            },
            Shortcut {
                keys: "⌘ +  /  ⌘ −",
                what: "Aproximar / afastar o canvas",
                needle: "ks.key == \"=\"",
            },
        ],
    },
    Group {
        title: "Quando o time precisa de você",
        items: &[
            Shortcut {
                keys: "⌘ J",
                what: "Ver os avisos que esperam por você",
                needle: "ks.key == \"j\"",
            },
            Shortcut {
                keys: "⌘ ⏎",
                what: "Aprovar o que está pedindo permissão",
                needle: "(ks.key == \"enter\" || ks.key == \"return\")",
            },
            Shortcut {
                keys: "⌘ ⇧ ⏎",
                what: "Recusar",
                needle: "ks.modifiers.shift",
            },
        ],
    },
    Group {
        title: "Ajustes",
        items: &[
            Shortcut {
                keys: "⌘ ,",
                what: "Abrir os Ajustes (tema, cores…)",
                needle: "ks.key == \",\"",
            },
            Shortcut {
                keys: "⌘ /",
                what: "Mostrar esta ajuda",
                needle: "ks.key == \"/\"",
            },
        ],
    },
];

/// Todas as linhas, na ordem de exibição (achatadas) — base dos testes.
#[cfg(test)]
pub fn all() -> impl Iterator<Item = &'static Shortcut> {
    GROUPS.iter().flat_map(|g| g.items.iter())
}

// Dimensões do painel (consts nomeadas — fora da catraca de tokens, como nos demais modais).
const MODAL_W: f32 = 520.0;
const MODAL_W_MIN: f32 = 320.0;
const MODAL_H_FLOOR: f32 = 240.0;
/// Largura da coluna das teclas (alinha as descrições; cabe "⌘ 1 … ⌘ 9").
const KEYS_COL_W: f32 = 132.0;

/// Pinta a ajuda sobre o [`Modal`] do catálogo. O corpo rola por dentro (lista longa em janela
/// baixa nunca corta linhas) e o título/✕ ficam fixos.
pub fn render(viewport: Size<Pixels>, cx: &mut Context<WorkspaceView>) -> AnyElement {
    let t = theme::active();
    let frame = clamp_frame(
        f32::from(viewport.width),
        f32::from(viewport.height),
        MODAL_W,
        MODAL_W_MIN,
        f32::from(t.spacing.lg),
        MODAL_H_FLOOR,
    );
    let mut body = div().flex().flex_col().gap_4();
    for group in GROUPS {
        let mut section = div().flex().flex_col().gap_1().child(
            div()
                .text_size(px(f32::from(t.typography.size.small)))
                .text_color(rgb(t.text.muted))
                .child(gpui::SharedString::from(group.title)),
        );
        for item in group.items {
            section = section.child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .flex_none()
                            .w(px(KEYS_COL_W))
                            .px_2()
                            .py_1()
                            .rounded_content()
                            .bg(rgb(t.surface.raised))
                            .font_family(t.typography.family.mono)
                            .text_size(px(f32::from(t.typography.size.small)))
                            .text_color(rgb(t.text.primary))
                            .child(gpui::SharedString::from(item.keys)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .text_size(px(f32::from(t.typography.size.body)))
                            .text_color(rgb(t.text.primary))
                            .child(gpui::SharedString::from(item.what)),
                    ),
            );
        }
        body = body.child(section);
    }
    body = body.child(
        div()
            .text_size(px(f32::from(t.typography.size.small)))
            .text_color(rgb(t.text.muted))
            .child(COPY_FOOTER),
    );
    Modal::new("shortcuts-modal", frame)
        .title(COPY_TITLE)
        .aria(COPY_TITLE)
        .dim(true)
        .close(cx.listener(|v, _ev: &ClickEvent, _w, cx| v.close_shortcuts(cx)))
        .dismiss_on_backdrop(cx.listener(|v, _ev: &ClickEvent, _w, cx| v.close_shortcuts(cx)))
        .body(body)
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A ajuda nunca promete atalho que não existe: o trecho que implementa cada linha ainda está
    /// no código do canvas (ou do rail de Espaços).
    #[test]
    fn every_documented_shortcut_still_exists_in_the_code() {
        let source = format!(
            "{}\n{}",
            include_str!("main.rs"),
            include_str!("sidebar.rs")
        );
        for s in all() {
            assert!(
                source.contains(s.needle),
                "o atalho «{}» ({}) não tem mais o código {:?} — atualize a ajuda",
                s.keys,
                s.what,
                s.needle
            );
        }
    }

    #[test]
    fn keys_are_unique_and_text_has_no_jargon() {
        let keys: Vec<&str> = all().map(|s| s.keys).collect();
        for (i, k) in keys.iter().enumerate() {
            assert!(!k.trim().is_empty());
            assert!(!keys[i + 1..].contains(k), "tecla repetida: {k}");
        }
        let jargon = [
            "terminal", "cli", "pty", "nó", "evento", "profile", "toml", "yaml", "log", "runtime",
        ];
        for s in all().map(|s| s.what).chain(GROUPS.iter().map(|g| g.title)) {
            let lower = s.to_lowercase();
            for word in jargon {
                assert!(
                    !lower
                        .split(|c: char| !c.is_alphanumeric())
                        .any(|w| w == word),
                    "jargão «{word}» na ajuda: {s}"
                );
            }
        }
    }

    /// O modal precisa caber numa janela baixa: a altura é clampada à janela (o corpo rola).
    #[test]
    fn frame_fits_a_short_window() {
        let f = clamp_frame(360.0, 300.0, MODAL_W, MODAL_W_MIN, 16.0, MODAL_H_FLOOR);
        assert!(f.w <= 360.0 - 2.0 * 16.0 + f32::EPSILON || f.w == MODAL_W_MIN);
        assert!(f.max_h <= 300.0 - 2.0 * 16.0 + f32::EPSILON || f.max_h == MODAL_H_FLOOR);
    }
}
