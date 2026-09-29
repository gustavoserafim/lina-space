//! `shell` — a **geometria do shell em colunas** (ADR 0053, Fase 1 da modernização da UI).
//!
//! Antes, o rail, o topo e o rodapé eram camadas `.absolute()` empilhadas sobre o canvas (o rail
//! pintado por último cobria os primeiros 52px do topo e do rodapé, e os cards passavam por baixo
//! deles). Agora o shell é estrutura de verdade:
//!
//! ```text
//! ┌──────┬──────────────────────────────┬────────┐
//! │      │ topo (altura fixa)           │        │
//! │ rail ├──────────────────────────────┤ coluna │
//! │      │ faixa de avisos (só se houver)│  do   │
//! │      ├──────────────────────────────┤ time   │
//! │      │ ÁREA DOS AGENTES (recorta)   │ (fase 2)│
//! │      ├──────────────────────────────┤        │
//! │      │ caixa de pedido (fixa)       │        │
//! └──────┴──────────────────────────────┴────────┘
//! ```
//!
//! Este módulo é gpui-free: as constantes e o `layout` são a FONTE ÚNICA que o render e o cálculo da
//! câmera (zoom, enquadrar, revelar) consomem — o retângulo da área dos agentes nunca é "adivinhado"
//! em dois lugares. Testado: as regiões não se sobrepõem e ladrilham a janela.

use crate::bridge::Camera;

/// Altura fixa do topo. Um topo que QUEBRA em várias linhas (como o antigo) tem altura imprevisível
/// e empurra a matemática da câmera; fixo, ele nunca quebra — o que não cabe vira ícone.
pub const TOPBAR_H: f32 = 44.0;
/// Altura da faixa de avisos (custódia, teto de custo, recuperação…) — só existe quando há aviso.
pub const NOTICE_H: f32 = 32.0;
/// Altura fixa da caixa de pedido: uma linha de campo + uma linha de dica/aviso.
pub const COMPOSER_H: f32 = 80.0;

/// Um retângulo em coordenadas da JANELA.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    #[must_use]
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    #[cfg(test)]
    #[must_use]
    pub fn right(&self) -> f32 {
        self.x + self.w
    }

    #[cfg(test)]
    #[must_use]
    pub fn bottom(&self) -> f32 {
        self.y + self.h
    }

    /// Tamanho `(largura, altura)`.
    #[must_use]
    pub fn size(&self) -> (f32, f32) {
        (self.w, self.h)
    }

    /// Centro do retângulo (âncora do zoom por teclado).
    #[must_use]
    pub fn center(&self) -> (f32, f32) {
        (self.x + self.w / 2.0, self.y + self.h / 2.0)
    }

    /// Os retângulos têm área em comum? Bordas que só encostam NÃO contam.
    #[cfg(test)]
    #[must_use]
    pub fn overlaps(&self, other: &Rect) -> bool {
        self.x < other.right()
            && other.x < self.right()
            && self.y < other.bottom()
            && other.y < self.bottom()
    }
}

/// As regiões do shell, em coordenadas da janela.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShellRects {
    pub rail: Rect,
    pub topbar: Rect,
    /// Faixa de avisos (altura 0 quando não há aviso).
    pub notice: Rect,
    /// A ÁREA DOS AGENTES: onde o canvas vive e recorta o que passa da borda.
    pub viewport: Rect,
    pub composer: Rect,
    /// Coluna do Time (largura 0 quando ausente).
    pub team: Rect,
}

/// Calcula as regiões do shell para uma janela `(largura, altura)`, um rail de `rail_w`, uma coluna
/// do time de `team_w` (0 = ausente) e uma faixa de avisos de `notice_h` (0 = sem aviso). PURA.
///
/// Garantias (testadas): as regiões não se sobrepõem, ladrilham a janela e a área dos agentes nunca
/// tem tamanho negativo — numa janela minúscula ela encolhe até 0 em vez de inverter.
#[must_use]
pub fn layout(window: (f32, f32), rail_w: f32, team_w: f32, notice_h: f32) -> ShellRects {
    let (ww, wh) = (window.0.max(0.0), window.1.max(0.0));
    let rail_w = rail_w.clamp(0.0, ww);
    let team_w = team_w.clamp(0.0, (ww - rail_w).max(0.0));
    let main_w = (ww - rail_w - team_w).max(0.0);
    let topbar_h = TOPBAR_H.min(wh);
    let composer_h = COMPOSER_H.min((wh - topbar_h).max(0.0));
    let notice_h = notice_h.clamp(0.0, (wh - topbar_h - composer_h).max(0.0));
    let viewport_h = (wh - topbar_h - notice_h - composer_h).max(0.0);
    let main_x = rail_w;
    ShellRects {
        rail: Rect::new(0.0, 0.0, rail_w, wh),
        topbar: Rect::new(main_x, 0.0, main_w, topbar_h),
        notice: Rect::new(main_x, topbar_h, main_w, notice_h),
        viewport: Rect::new(main_x, topbar_h + notice_h, main_w, viewport_h),
        composer: Rect::new(main_x, topbar_h + notice_h + viewport_h, main_w, composer_h),
        team: Rect::new(main_x + main_w, 0.0, team_w, wh),
    }
}

/// Câmera em coordenadas LOCAIS da área dos agentes (origem no canto dela). A matemática da câmera
/// (revelar, enquadrar, visibilidade) foi escrita para uma área que começa em `(0,0)`; a área agora
/// começa depois do rail e do topo — converter, aplicar e voltar mantém essa matemática intacta.
#[must_use]
pub fn to_local(cam: Camera, viewport: Rect) -> Camera {
    Camera {
        pan: (cam.pan.0 - viewport.x, cam.pan.1 - viewport.y),
        zoom: cam.zoom,
    }
}

/// Inverso de [`to_local`].
#[must_use]
pub fn from_local(cam: Camera, viewport: Rect) -> Camera {
    Camera {
        pan: (cam.pan.0 + viewport.x, cam.pan.1 + viewport.y),
        zoom: cam.zoom,
    }
}

/// O "home" da câmera: o mundo (0,0) no canto da ÁREA DOS AGENTES, zoom 1 (⌘0, 🏠 e recentrar).
#[must_use]
pub fn home(viewport: Rect) -> Camera {
    from_local(Camera::default(), viewport)
}

/// Tom de um aviso da faixa (decide as cores — sempre pares já validados no gate de contraste).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoticeTone {
    Warning,
    Danger,
    Confirm,
    Neutral,
}

/// Um aviso do shell (custódia, teto de custo, recuperação, pausa…).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notice {
    pub tone: NoticeTone,
    pub text: String,
}

/// Escolhe o aviso da faixa: o de MAIOR prioridade (a lista vem em ordem de prioridade) e, se houver
/// outros, diz quantos esperam (`+N`) — nada some em silêncio e a faixa nunca cresce além de 1 linha.
#[must_use]
pub fn pick_notice(candidates: Vec<Notice>) -> Option<Notice> {
    let more = candidates.len().saturating_sub(1);
    let mut first = candidates.into_iter().next()?;
    if more > 0 {
        first.text = format!("{}  (+{more})", first.text);
    }
    Some(first)
}

/// Como o topo se apresenta conforme a largura da janela (o topo NÃO quebra em linhas).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopbarMode {
    /// Rótulos completos.
    Full,
    /// Só ícones nos botões secundários (o primário mantém o rótulo).
    Compact,
    /// Só o essencial: primário e Ajustes; o resto mora na paleta ⌘K.
    Minimal,
}

/// Modo do topo pela largura DISPONÍVEL (a da coluna principal, sem rail nem coluna do time).
#[must_use]
pub fn topbar_mode(main_w: f32) -> TopbarMode {
    if main_w >= 940.0 {
        TopbarMode::Full
    } else if main_w >= 620.0 {
        TopbarMode::Compact
    } else {
        TopbarMode::Minimal
    }
}

#[cfg(test)]
mod tests {
    /// O "home" põe o mundo (0,0) no canto da ÁREA DOS AGENTES (ex.: o card em (30,96) aparece a
    /// (30,96) DENTRO da área, não por baixo do rail e do topo) — substitui o antigo reset a (0,0)
    /// da janela.
    #[test]
    fn home_puts_world_origin_at_the_viewport_corner() {
        let vp = Rect::new(52.0, 44.0, 800.0, 600.0);
        let cam = home(vp);
        assert_eq!(cam.pan, (52.0, 44.0));
        assert!((cam.zoom - 1.0).abs() < f32::EPSILON);
        assert_eq!(cam.world_to_screen((30.0, 96.0)), (82.0, 140.0));
    }

    #[test]
    fn notice_strip_shows_the_top_priority_and_counts_the_rest() {
        let n = |tone, text: &str| Notice {
            tone,
            text: text.to_string(),
        };
        assert_eq!(
            pick_notice(vec![]),
            None,
            "sem aviso a faixa some (altura 0)"
        );
        let only = pick_notice(vec![n(NoticeTone::Danger, "teto atingido")]).expect("aviso");
        assert_eq!(only.text, "teto atingido", "1 aviso não ganha sufixo");
        let many = pick_notice(vec![
            n(NoticeTone::Warning, "custódia"),
            n(NoticeTone::Danger, "recuperando"),
            n(NoticeTone::Neutral, "pausado"),
        ])
        .expect("aviso");
        assert_eq!(many.tone, NoticeTone::Warning, "vence o primeiro da lista");
        assert_eq!(many.text, "custódia  (+2)");
    }

    use super::*;

    fn regions(s: &ShellRects) -> Vec<Rect> {
        vec![s.rail, s.topbar, s.notice, s.viewport, s.composer, s.team]
    }

    fn area(r: &Rect) -> f32 {
        r.w * r.h
    }

    /// As regiões NÃO se sobrepõem e ladrilham a janela inteira — nada passa por baixo de nada (a
    /// falha do screenshot: cards sob o topo, rodapé e rail).
    #[test]
    fn regions_tile_the_window_without_overlap() {
        for (window, rail_w, team_w, notice_h) in [
            ((1915.0, 969.0), 52.0, 0.0, 0.0),
            ((1915.0, 969.0), 280.0, 240.0, 32.0),
            ((1280.0, 720.0), 52.0, 240.0, 0.0),
            ((900.0, 600.0), 52.0, 0.0, 32.0),
        ] {
            let s = layout(window, rail_w, team_w, notice_h);
            let all = regions(&s);
            let total: f32 = all.iter().map(area).sum();
            assert!(
                (total - window.0 * window.1).abs() < 0.5,
                "ladrilha a janela {window:?}: soma {total}"
            );
            for (i, a) in all.iter().enumerate() {
                for b in all.iter().skip(i + 1) {
                    assert!(!a.overlaps(b), "{a:?} sobrepõe {b:?}");
                }
            }
            assert!(
                (s.viewport.x - rail_w).abs() < f32::EPSILON,
                "começa depois do rail"
            );
            assert!(
                (s.viewport.y - (TOPBAR_H + notice_h)).abs() < f32::EPSILON,
                "começa depois do topo e dos avisos"
            );
            assert!(
                (s.composer.bottom() - window.1).abs() < 0.5,
                "a caixa de pedido encosta no fundo"
            );
        }
    }

    /// Janela minúscula: a área dos agentes encolhe até 0, nunca fica negativa nem invertida.
    #[test]
    fn viewport_never_goes_negative_in_a_tiny_window() {
        for window in [(300.0, 100.0), (60.0, 30.0), (0.0, 0.0), (500.0, 110.0)] {
            let s = layout(window, 52.0, 240.0, 32.0);
            for r in regions(&s) {
                assert!(r.w >= 0.0 && r.h >= 0.0, "{r:?} negativo em {window:?}");
            }
            assert!(s.viewport.h >= 0.0 && s.viewport.w >= 0.0);
        }
    }

    /// Converter a câmera para o local da área e voltar devolve a mesma câmera; e um ponto do mundo
    /// aparece na tela deslocado exatamente pela origem da área.
    #[test]
    fn camera_local_roundtrip_and_origin_offset() {
        let vp = Rect::new(52.0, 44.0, 800.0, 600.0);
        let cam = Camera {
            pan: (100.0, -30.0),
            zoom: 1.5,
        };
        assert_eq!(from_local(to_local(cam, vp), vp), cam);
        let local = to_local(cam, vp);
        let (wx, wy) = (10.0, 20.0);
        let (sx, sy) = cam.world_to_screen((wx, wy));
        let (lx, ly) = local.world_to_screen((wx, wy));
        assert!((sx - (lx + vp.x)).abs() < 1e-4 && (sy - (ly + vp.y)).abs() < 1e-4);
    }

    #[test]
    fn center_and_topbar_modes_follow_available_width() {
        assert_eq!(Rect::new(10.0, 20.0, 100.0, 60.0).center(), (60.0, 50.0));
        assert_eq!(topbar_mode(1400.0), TopbarMode::Full);
        assert_eq!(topbar_mode(940.0), TopbarMode::Full);
        assert_eq!(topbar_mode(800.0), TopbarMode::Compact);
        assert_eq!(topbar_mode(500.0), TopbarMode::Minimal);
    }
}
