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
/// Dica de teclas enquanto o campo está em foco e não há aviso a mostrar — ensina como sair e onde
/// achar o resto dos atalhos, sem ocupar espaço fora do foco.
pub const COPY_HINT: &str = "⏎ envia · ↑ traz o pedido anterior · esc volta ao agente · ⌘/ atalhos";

/// Quantos pedidos anteriores a caixa lembra (curto: é um "de novo", não um arquivo).
const HISTORY_CAP: usize = 20;

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
    /// Pedidos já enviados, do mais antigo ao mais novo (↑/↓ os recuperam).
    history: Vec<String>,
    /// Posição do pedido recuperado em `history` (`None` = digitando um texto novo).
    recall: Option<usize>,
    /// O que estava digitado antes de começar a navegar pelo histórico (volta com ↓ no fim).
    draft: String,
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

    /// O botão "Enviar" só vale com texto — sem ele fica esmaecido (a linha já mostra a dica).
    #[must_use]
    pub fn can_send(&self) -> bool {
        !self.text.trim().is_empty()
    }

    /// A dica de teclas: só com o campo em foco e sem outro aviso a mostrar.
    #[must_use]
    pub fn hint(&self) -> Option<&'static str> {
        (self.focused && self.notice.is_none()).then_some(COPY_HINT)
    }

    /// ↑ — recupera o pedido anterior (o mais novo primeiro). Guarda o rascunho na primeira subida.
    pub fn recall_previous(&mut self) {
        if self.history.is_empty() {
            return;
        }
        let next = match self.recall {
            None => {
                self.draft = std::mem::take(&mut self.text);
                self.history.len() - 1
            }
            Some(0) => 0,
            Some(i) => i - 1,
        };
        self.recall = Some(next);
        self.text = self.history[next].clone();
        self.notice = None;
    }

    /// ↓ — volta ao pedido seguinte; passando do mais novo, devolve o rascunho.
    pub fn recall_next(&mut self) {
        let Some(i) = self.recall else {
            return;
        };
        if i + 1 < self.history.len() {
            self.recall = Some(i + 1);
            self.text = self.history[i + 1].clone();
        } else {
            self.recall = None;
            self.text = std::mem::take(&mut self.draft);
        }
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
        self.recall = None; // digitar sobre um pedido recuperado o torna um texto novo
    }
    pub fn backspace(&mut self) {
        self.text.pop();
        self.recall = None;
    }

    /// Retira o pedido para envio (sem espaços nas pontas). Vazio ⇒ `None` e nada muda.
    pub fn take_submission(&mut self) -> Option<String> {
        let text = self.text.trim().to_string();
        if text.is_empty() {
            return None;
        }
        self.text.clear();
        self.recall = None;
        self.draft.clear();
        if self.history.last() != Some(&text) {
            self.history.push(text.clone());
            if self.history.len() > HISTORY_CAP {
                self.history.remove(0);
            }
        }
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
    fn send_needs_text_and_hint_shows_only_when_focused_without_notice() {
        let mut b = RequestBox::default();
        assert!(!b.can_send());
        assert_eq!(b.hint(), None, "fora do foco a dica não ocupa espaço");
        b.focus();
        assert_eq!(b.hint(), Some(COPY_HINT));
        b.type_str("   ");
        assert!(!b.can_send(), "só espaços não enviam");
        b.type_str("oi");
        assert!(b.can_send());
        let text = b.take_submission().expect("pedido");
        b.delivered_to("Maestro");
        assert_eq!(
            b.hint(),
            None,
            "o aviso de envio tem prioridade sobre a dica"
        );
        drop(text);
    }

    /// ↑ traz o pedido anterior (mais novo primeiro), ↓ volta, e passar do mais novo devolve o que
    /// estava sendo digitado; digitar sobre um recuperado o vira texto novo; repetir não duplica.
    #[test]
    fn up_and_down_recall_previous_requests_and_restore_the_draft() {
        let mut b = RequestBox::default();
        b.recall_previous();
        assert_eq!(b.text(), "", "sem histórico ↑ não faz nada");
        for req in [
            "montar a landing",
            "pesquisar concorrentes",
            "pesquisar concorrentes",
        ] {
            b.type_str(req);
            b.take_submission().expect("pedido");
        }
        assert_eq!(b.history.len(), 2, "pedido repetido em seguida não duplica");

        b.type_str("rascunho");
        b.recall_previous();
        assert_eq!(b.text(), "pesquisar concorrentes", "o mais novo primeiro");
        b.recall_previous();
        assert_eq!(b.text(), "montar a landing");
        b.recall_previous();
        assert_eq!(b.text(), "montar a landing", "trava no mais antigo");
        b.recall_next();
        assert_eq!(b.text(), "pesquisar concorrentes");
        b.recall_next();
        assert_eq!(
            b.text(),
            "rascunho",
            "passou do mais novo: o rascunho volta"
        );
        b.recall_next();
        assert_eq!(b.text(), "rascunho", "↓ sem navegar não muda nada");

        b.recall_previous();
        b.type_str(" agora");
        assert_eq!(b.text(), "pesquisar concorrentes agora");
        b.recall_next();
        assert_eq!(
            b.text(),
            "pesquisar concorrentes agora",
            "editado, deixou de ser 'recuperado'"
        );
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
