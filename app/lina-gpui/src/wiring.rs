//! **W4-3 — chrome de conexão "sem fios" (gpui-free).** O diferencial #1 visível: estar no Espaço =
//! estar conectado, SEM ligar cabos. Modela (puro e testável):
//! - o **FREIO** (pausa da auto-orquestração) — estado compartilhado entre a UI (que pede) e a
//!   `MailboxPump` (que aplica no `Router` e espelha), no mesmo padrão da mesa de custódia (round 5);
//! - o **SELO "Time conectado"** — full-mesh LÓGICO derivado da MEMBERSHIP (decisão arq §2.2: sem
//!   arestas persistentes; a conexão vem da presença, não de cabos);
//! - a gate de **reduce-motion** do pulso (a11y, W4-6 liga ao SO; aqui fica o hook).
//!
//! O render gpui e a `MailboxPump` CONSOMEM este modelo. O pulso efêmero A→B em si é decorativo
//! (a entrega acontece independente dele).

use std::sync::{Arc, Mutex};

/// **Estado do FREIO**, compartilhado entre a UI e a [`crate::bridge::MailboxPump`]. A UI NUNCA toca o
/// `Router`: só sinaliza `toggle_requested`; a pump aplica `Router::pause`/`resume` e espelha `paused`
/// (que a UI lê para o rótulo). Pausado = delegações novas ENFILEIRAM (não injetam) até retomar.
#[derive(Debug, Default)]
pub struct BrakeState {
    /// Espelho read-only (p/ a UI) de `Router::is_paused` — atualizado pela pump.
    pub paused: bool,
    /// A UI pediu para ALTERNAR o freio (pausa↔retoma). A pump consome (e zera) no próximo tick.
    pub toggle_requested: bool,
}

/// Handle compartilhado do freio.
pub type Brake = Arc<Mutex<BrakeState>>;

/// Cria um freio novo (despausado). A pump pode sobrescrever `paused` ao restaurar o estado do log.
#[must_use]
pub fn new_brake() -> Brake {
    Arc::new(Mutex::new(BrakeState::default()))
}

/// **Selo "Time conectado" (full-mesh LÓGICO por membership).** 2+ nós vivos no Espaço ⇒ o time se
/// fala (a conexão é derivada da presença — sem tabela de arestas). 0-1 nó ⇒ ainda não há "time".
#[must_use]
pub fn team_connected(live_node_count: usize) -> bool {
    live_node_count >= 2
}

/// Contagens do time para o resumo do topo. `agents` só conta agentes VIVOS (Dead sai); `working`
/// = produzindo saída agora; `needs_you` = pendências reais na fila de atenção (o mesmo número do 🔔).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TeamPulse {
    pub agents: usize,
    pub working: usize,
    pub needs_you: usize,
}

impl TeamPulse {
    /// Conta a partir do status de cada TERMINAL do Espaço + o nº de pendências de atenção.
    #[must_use]
    pub fn from_statuses(
        terminals: impl IntoIterator<Item = lina_host::NodeStatus>,
        needs_you: usize,
    ) -> Self {
        let mut pulse = Self {
            needs_you,
            ..Self::default()
        };
        for status in terminals {
            match status {
                lina_host::NodeStatus::Dead => {}
                lina_host::NodeStatus::Busy => {
                    pulse.agents += 1;
                    pulse.working += 1;
                }
                _ => pulse.agents += 1,
            }
        }
        pulse
    }
}

/// **Resumo do time no topo** ("3 agentes · 1 trabalhando · 1 precisa de você"): o que o leigo quer
/// saber num relance — quem está ocupado e se algo espera por ele — no lugar do selo fixo "Time
/// conectado" e do contador de registros. `None` sem agentes (o estado vazio já guia). Sem jargão.
#[must_use]
pub fn team_summary(p: TeamPulse) -> Option<String> {
    if p.agents == 0 {
        return None;
    }
    let mut parts = vec![if p.agents == 1 {
        "1 agente".to_owned()
    } else {
        format!("{} agentes", p.agents)
    }];
    if p.working > 0 {
        parts.push(if p.working == 1 {
            "1 trabalhando".to_owned()
        } else {
            format!("{} trabalhando", p.working)
        });
    }
    if p.needs_you > 0 {
        parts.push(if p.needs_you == 1 {
            "1 precisa de você".to_owned()
        } else {
            format!("{} precisam de você", p.needs_you)
        });
    }
    if p.working == 0 && p.needs_you == 0 {
        parts.push("tudo tranquilo".to_owned());
    }
    Some(parts.join(" · "))
}

/// `true` se o pulso A→B deve ANIMAR. Com **reduce-motion** ligado (a11y), a animação é suprimida —
/// a entrega ainda ocorre (o pulso é decorativo), apenas não há movimento na tela.
#[must_use]
pub fn animate_pulse(reduce_motion: bool) -> bool {
    !reduce_motion
}

#[cfg(test)]
mod tests {
    // ── UX: resumo do time no topo ──

    #[test]
    fn team_pulse_counts_only_live_agents_and_working_ones() {
        use lina_host::NodeStatus as S;
        let pulse = TeamPulse::from_statuses(
            [S::Idle, S::Busy, S::Busy, S::Dead, S::Blocked, S::Starting],
            2,
        );
        assert_eq!(
            pulse,
            TeamPulse {
                agents: 5,
                working: 2,
                needs_you: 2
            },
            "Dead não conta; Busy é 'trabalhando'"
        );
    }

    #[test]
    fn team_summary_reads_in_plain_portuguese_with_correct_plurals() {
        let s = |agents, working, needs_you| {
            team_summary(TeamPulse {
                agents,
                working,
                needs_you,
            })
        };
        assert_eq!(s(0, 0, 0), None, "sem agentes o estado vazio é quem guia");
        assert_eq!(s(1, 0, 0).as_deref(), Some("1 agente · tudo tranquilo"));
        assert_eq!(
            s(3, 1, 0).as_deref(),
            Some("3 agentes · 1 trabalhando"),
            "sem pendência não diz 'tranquilo' enquanto alguém trabalha"
        );
        assert_eq!(
            s(5, 2, 1).as_deref(),
            Some("5 agentes · 2 trabalhando · 1 precisa de você")
        );
        assert_eq!(
            s(2, 0, 2).as_deref(),
            Some("2 agentes · 2 precisam de você"),
            "plural do verbo acompanha o número"
        );
    }

    use super::*;

    /// O selo é full-mesh por membership: aparece com 2+ nós, some com 0-1.
    #[test]
    fn team_connected_is_membership_derived() {
        assert!(!team_connected(0), "Espaço vazio: sem time");
        assert!(!team_connected(1), "1 nó: ainda não há time");
        assert!(team_connected(2), "2 nós: time conectado");
        assert!(team_connected(7), "muitos nós: conectado");
    }

    /// reduce-motion suprime a animação do pulso (mas não a entrega).
    #[test]
    fn reduce_motion_suppresses_pulse_animation() {
        assert!(animate_pulse(false), "sem reduce-motion: pulso anima");
        assert!(!animate_pulse(true), "reduce-motion: pulso NÃO anima");
    }

    /// O freio nasce despausado; alternar é um pedido que a pump consome.
    #[test]
    fn brake_starts_unpaused_and_toggle_is_a_request() {
        let brake = new_brake();
        {
            let b = brake.lock().unwrap();
            assert!(!b.paused);
            assert!(!b.toggle_requested);
        }
        // A UI pede a alternância.
        brake.lock().unwrap().toggle_requested = true;
        // A pump consome (zera) e espelha o novo estado.
        let mut b = brake.lock().unwrap();
        assert!(
            std::mem::take(&mut b.toggle_requested),
            "o pedido é consumível"
        );
        assert!(!b.toggle_requested, "consumido");
    }
}
