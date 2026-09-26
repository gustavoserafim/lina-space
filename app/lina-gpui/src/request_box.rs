//! `request_box` — **ADR 0061: a caixa de pedido**, a entrada única do leigo no Espaço.
//!
//! Um campo fixo no rodapé do canvas: "O que você quer que o time faça?". O Enter entrega o texto
//! ao terminal de ENTRADA do roster vivo — o Tradutor quando existe, senão o Maestro (a mesma regra
//! de [`lina_role_discovery::entry_origin`]). A entrega é teclado humano (colagem + Enter separado),
//! a mesma classe de confiança do ⌘V; nada aqui decide identidade nem autorização.
//!
//! Split do shell: [`RequestBox`] é gpui-free e testável (edição, foco, envio); a casca gpui vive no
//! `main.rs`, que só aplica o estado.

use lina_host::NodeId;
use lina_role_discovery::{MAESTRO_ROLE, TRADUTOR_ROLE};

pub const COPY_PLACEHOLDER: &str = "O que você quer que o time faça? (⌘L)";
pub const COPY_SEND: &str = "Enviar ⏎";
pub const COPY_CREATE_MAESTRO: &str = "Criar o Maestro";
pub const COPY_NO_ENTRY: &str =
    "Ainda não há quem coordene este Espaço. Crie o Maestro e o seu pedido vai direto para ele.";
pub const COPY_CREATING: &str = "Criando o Maestro…";

/// Quem recebe o pedido: o terminal de entrada vivo e o seu nome (para a confirmação leiga).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryNode {
    pub node: NodeId,
    pub name: String,
}

/// Escolhe o terminal de entrada entre os nós VIVOS `(id, papel, nome)`: Tradutor vence Maestro;
/// sem nenhum dos dois ⇒ `None`. Empate no mesmo papel ⇒ o mais antigo (NodeId v7 cresce no tempo).
#[must_use]
pub fn pick_entry_node<'a>(
    live: impl IntoIterator<Item = (NodeId, &'a str, &'a str)>,
) -> Option<EntryNode> {
    let mut tradutor: Option<(NodeId, &str)> = None;
    let mut maestro: Option<(NodeId, &str)> = None;
    for (node, role, name) in live {
        let slot = if role.eq_ignore_ascii_case(TRADUTOR_ROLE) {
            &mut tradutor
        } else if role.eq_ignore_ascii_case(MAESTRO_ROLE) {
            &mut maestro
        } else {
            continue;
        };
        if slot.is_none_or(|(current, _)| node < current) {
            *slot = Some((node, name));
        }
    }
    tradutor.or(maestro).map(|(node, name)| EntryNode {
        node,
        name: name.to_string(),
    })
}

/// Estado da caixa (RAM de UI — o pedido entregue fica no scrollback do terminal que o recebeu).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RequestBox {
    text: String,
    focused: bool,
    /// Linha curta abaixo do campo: confirmação do envio ou o motivo de não ter enviado.
    notice: Option<String>,
    /// O último envio não achou terminal de entrada → o botão vira "Criar o Maestro".
    missing_entry: bool,
    /// Criação do Maestro em curso (evita duplo clique criar dois).
    creating_maestro: bool,
}

impl RequestBox {
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }
    #[must_use]
    pub fn is_focused(&self) -> bool {
        self.focused
    }
    #[must_use]
    pub fn notice(&self) -> Option<&str> {
        self.notice.as_deref()
    }
    #[must_use]
    pub fn missing_entry(&self) -> bool {
        self.missing_entry
    }
    #[must_use]
    pub fn creating_maestro(&self) -> bool {
        self.creating_maestro
    }

    pub fn focus(&mut self) {
        self.focused = true;
    }
    pub fn blur(&mut self) {
        self.focused = false;
    }
    pub fn type_str(&mut self, s: &str) {
        self.text.push_str(s);
        self.notice = None;
    }
    pub fn backspace(&mut self) {
        self.text.pop();
    }

    /// Retira o pedido para envio (sem espaços nas pontas). Vazio ⇒ `None` e nada muda.
    pub fn take_submission(&mut self) -> Option<String> {
        let text = self.text.trim().to_string();
        if text.is_empty() {
            return None;
        }
        self.text.clear();
        Some(text)
    }

    /// O pedido chegou ao terminal de entrada.
    pub fn delivered_to(&mut self, name: &str) {
        self.missing_entry = false;
        self.notice = Some(format!("Enviado para {name} ✓"));
    }

    /// Não havia terminal de entrada: devolve o texto ao campo (nada se perde) e oferece criar.
    pub fn no_entry(&mut self, text: String) {
        self.text = text;
        self.missing_entry = true;
        self.notice = Some(COPY_NO_ENTRY.to_string());
    }

    pub fn start_creating_maestro(&mut self) -> bool {
        if self.creating_maestro {
            return false;
        }
        self.creating_maestro = true;
        self.notice = Some(COPY_CREATING.to_string());
        true
    }

    pub fn finish_creating_maestro(&mut self, result: Result<(), String>) {
        self.creating_maestro = false;
        match result {
            Ok(()) => {
                self.missing_entry = false;
                self.notice = Some("O Maestro chegou — pode enviar o seu pedido.".to_string());
            }
            Err(error) => self.notice = Some(format!("Não consegui criar o Maestro: {error}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n: u128) -> NodeId {
        NodeId::from_u128(n)
    }

    #[test]
    fn tradutor_wins_over_maestro_and_others_never_receive() {
        let live = [
            (id(1), "BACKEND", "Dev Backend"),
            (id(2), "MAESTRO", "Maestro"),
            (id(3), "TRADUTOR", "Tradutor"),
        ];
        assert_eq!(
            pick_entry_node(live),
            Some(EntryNode {
                node: id(3),
                name: "Tradutor".into()
            })
        );
        let no_tradutor = [(id(1), "QA", "QA"), (id(5), "maestro", "Maestro")];
        assert_eq!(pick_entry_node(no_tradutor).map(|e| e.node), Some(id(5)));
        assert_eq!(pick_entry_node([(id(1), "QA", "QA")]), None);
    }

    #[test]
    fn oldest_entry_node_wins_a_tie() {
        let live = [
            (id(9), "MAESTRO", "Maestro (2)"),
            (id(4), "MAESTRO", "Maestro"),
        ];
        assert_eq!(
            pick_entry_node(live).map(|e| e.name),
            Some("Maestro".into())
        );
    }

    #[test]
    fn submission_is_trimmed_and_empty_is_ignored() {
        let mut b = RequestBox::default();
        b.type_str("   ");
        assert_eq!(b.take_submission(), None);
        b.type_str(" montar a landing ");
        assert_eq!(b.take_submission().as_deref(), Some("montar a landing"));
        assert_eq!(b.text(), "");
    }

    #[test]
    fn no_entry_keeps_the_text_and_offers_the_maestro() {
        let mut b = RequestBox::default();
        b.type_str("pesquisar concorrentes");
        let text = b.take_submission().expect("pedido");
        b.no_entry(text);
        assert_eq!(b.text(), "pesquisar concorrentes", "nada se perde");
        assert!(b.missing_entry());
        assert!(b.start_creating_maestro());
        assert!(!b.start_creating_maestro(), "duplo clique não cria dois");
        b.finish_creating_maestro(Ok(()));
        assert!(!b.missing_entry() && !b.creating_maestro());
    }
}
