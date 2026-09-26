//! **ADR 0063 — Memória do Espaço (`lina memo`).** Fatos, decisões e preferências que os agentes
//! de um Espaço anotam para os colegas lembrarem. A verdade é o log (`MemoNoted`); isto é só a
//! projeção por replay, mais a busca por termos e o índice curto que vai para o `whoami`.
//!
//! Nota é DADO escrito por agente: nunca instrução, identidade ou autorização (ADR 0007). O `id` e
//! o `by` são carimbados por quem apenda (o router, a partir do remetente autenticado).

use crate::events::{DomainEvent, EventRecord, EventStore, StoreError};

/// Teto de uma nota — memória é fato curto; o resto vive no vault ou no plano.
pub const MEMO_MAX_CHARS: usize = 1_000;

/// Uma nota da Memória do Espaço.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Memo {
    pub id: String,
    pub text: String,
    pub by: String,
}

/// Por que uma nota não foi aceita.
#[derive(Debug, thiserror::Error)]
pub enum MemoError {
    #[error("a nota está vazia")]
    Empty,
    #[error(transparent)]
    Store(#[from] StoreError),
}

/// A projeção: todas as notas, na ordem do log (a mais antiga primeiro).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpaceMemory {
    memos: Vec<Memo>,
}

impl SpaceMemory {
    /// Reconstrói a projeção do `EventStore` por replay.
    ///
    /// # Errors
    /// Falha ao ler o event log.
    pub fn replay(store: &EventStore) -> Result<Self, StoreError> {
        Ok(Self::from_records(&store.events()?))
    }

    /// Núcleo determinístico (PURO): varre os registros em ordem de log.
    #[must_use]
    pub fn from_records(records: &[EventRecord]) -> Self {
        let memos = records
            .iter()
            .filter(|record| record.kind == "MemoNoted")
            .filter_map(|record| {
                // Registro indecodificável (versão futura) é pulado: a projeção é derivada, não
                // validadora do log (mesma postura de `ClueSet`/`Mentality`).
                match DomainEvent::from_record(&record.kind, record.version, record.payload.clone())
                {
                    Ok(DomainEvent::MemoNoted { id, text, by }) => Some(Memo { id, text, by }),
                    _ => None,
                }
            })
            .collect();
        Self { memos }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.memos.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.memos.is_empty()
    }

    /// Todas as notas, da mais antiga à mais recente.
    #[must_use]
    pub fn all(&self) -> &[Memo] {
        &self.memos
    }

    /// As `n` notas mais recentes, da mais nova para a mais antiga.
    #[must_use]
    pub fn recent(&self, n: usize) -> Vec<&Memo> {
        self.memos.iter().rev().take(n).collect()
    }

    /// Busca por termos: pontua cada nota pelos termos distintos da consulta que aparecem nela
    /// (sem caixa e sem acento); empate → a mais recente primeiro. Consulta sem termos ⇒ vazio.
    #[must_use]
    pub fn search(&self, query: &str, limit: usize) -> Vec<&Memo> {
        let terms: Vec<String> = {
            let mut t: Vec<String> = fold(query)
                .split(|c: char| !c.is_alphanumeric())
                .filter(|w| w.chars().count() >= 2)
                .map(str::to_owned)
                .collect();
            t.sort();
            t.dedup();
            t
        };
        if terms.is_empty() {
            return Vec::new();
        }
        let mut scored: Vec<(usize, usize, &Memo)> = self
            .memos
            .iter()
            .enumerate()
            .filter_map(|(pos, memo)| {
                let haystack = fold(&memo.text);
                let score = terms
                    .iter()
                    .filter(|t| haystack.contains(t.as_str()))
                    .count();
                (score > 0).then_some((score, pos, memo))
            })
            .collect();
        scored.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.cmp(&a.1)));
        scored.into_iter().take(limit).map(|(_, _, m)| m).collect()
    }

    /// Linha curta para o bloco `whoami`: contagem + as `n` mais recentes (cada uma cortada em
    /// `width` caracteres). `None` quando não há nota — o bloco não ganha linha vazia.
    #[must_use]
    pub fn index_line(&self, n: usize, width: usize) -> Option<String> {
        if self.memos.is_empty() {
            return None;
        }
        let recent: Vec<String> = self
            .recent(n)
            .into_iter()
            .map(|m| format!("[{}] {}", m.id, clip(&m.text, width)))
            .collect();
        Some(format!(
            "{} nota(s) (use `lina memo search \"termo\"` ANTES de perguntar ao time); ultimas: {}",
            self.memos.len(),
            recent.join(" | ")
        ))
    }
}

/// Registra uma nota no log. O `id` é sequencial por Espaço (`M1`, `M2`, …) e o `by` vem do
/// chamador (o router passa o remetente AUTENTICADO). Texto é aparado e limitado a
/// [`MEMO_MAX_CHARS`] caracteres.
///
/// # Errors
/// [`MemoError::Empty`] para texto vazio; [`MemoError::Store`] se o log falhar.
pub fn note(store: &mut EventStore, text: &str, by: &str) -> Result<Memo, MemoError> {
    let text = clip(text.trim(), MEMO_MAX_CHARS);
    if text.is_empty() {
        return Err(MemoError::Empty);
    }
    let id = format!("M{}", SpaceMemory::replay(store)?.len() + 1);
    let memo = Memo {
        id,
        text,
        by: by.to_owned(),
    };
    store.append(&DomainEvent::MemoNoted {
        id: memo.id.clone(),
        text: memo.text.clone(),
        by: memo.by.clone(),
    })?;
    Ok(memo)
}

/// Corta em `max` caracteres (nunca no meio de um caractere), marcando o corte com `…`.
fn clip(text: &str, max: usize) -> String {
    let flat: String = text
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    if flat.chars().count() <= max {
        return flat;
    }
    let mut out: String = flat.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

/// Minúsculas sem acento — a busca de um leigo não pode falhar por "decisão" × "decisao".
fn fold(text: &str) -> String {
    text.chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            other => other,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TmpStore {
        dir: std::path::PathBuf,
        store: EventStore,
    }
    impl TmpStore {
        fn new(tag: &str) -> Self {
            let dir = std::env::temp_dir().join(format!(
                "lina-memory-{tag}-{}-{:?}",
                std::process::id(),
                std::thread::current().id()
            ));
            let _ = std::fs::remove_dir_all(&dir);
            let store = EventStore::open(&dir).expect("store");
            Self { dir, store }
        }
    }
    impl Drop for TmpStore {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    #[test]
    fn notes_get_sequential_ids_and_replay_in_log_order() {
        let mut t = TmpStore::new("seq");
        let a = note(&mut t.store, "  usar pnpm, não npm  ", "Maestro").expect("nota 1");
        let b = note(&mut t.store, "deploy só na sexta", "Dev Backend").expect("nota 2");
        assert_eq!(
            (a.id.as_str(), a.text.as_str()),
            ("M1", "usar pnpm, não npm")
        );
        assert_eq!(b.id, "M2");
        let mem = SpaceMemory::replay(&t.store).expect("replay");
        assert_eq!(mem.all(), &[a, b.clone()]);
        assert_eq!(mem.recent(1), vec![&b]);
    }

    #[test]
    fn empty_note_is_rejected_and_long_note_is_clipped() {
        let mut t = TmpStore::new("limites");
        assert!(matches!(
            note(&mut t.store, "   ", "X"),
            Err(MemoError::Empty)
        ));
        let long = "a".repeat(MEMO_MAX_CHARS + 50);
        let memo = note(&mut t.store, &long, "X").expect("nota longa");
        assert_eq!(memo.text.chars().count(), MEMO_MAX_CHARS);
        assert!(memo.text.ends_with('…'));
        let multiline = note(&mut t.store, "linha 1\nlinha 2", "X").expect("multilinha");
        assert_eq!(multiline.text, "linha 1 linha 2", "nota vira uma linha só");
    }

    #[test]
    fn search_ranks_by_matched_terms_ignoring_case_and_accents() {
        let mut t = TmpStore::new("busca");
        note(&mut t.store, "Decisão: banco é Postgres no Supabase", "A").expect("1");
        note(&mut t.store, "cliente prefere tom informal", "B").expect("2");
        note(&mut t.store, "Postgres roda na porta 5433", "C").expect("3");
        let mem = SpaceMemory::replay(&t.store).expect("replay");

        let hits = mem.search("decisao postgres", 10);
        assert_eq!(
            hits.first().map(|m| m.id.as_str()),
            Some("M1"),
            "2 termos vencem 1"
        );
        assert_eq!(hits.len(), 2);
        let tie = mem.search("postgres", 10);
        assert_eq!(tie[0].id, "M3", "empate → a mais recente primeiro");
        assert!(mem.search("?", 10).is_empty());
    }

    #[test]
    fn index_line_lists_recent_notes_or_nothing() {
        let mut t = TmpStore::new("indice");
        assert_eq!(SpaceMemory::default().index_line(3, 40), None);
        note(&mut t.store, "primeira", "A").expect("1");
        note(
            &mut t.store,
            "segunda nota bem mais comprida que o corte",
            "A",
        )
        .expect("2");
        let line = SpaceMemory::replay(&t.store)
            .expect("replay")
            .index_line(1, 10)
            .expect("linha");
        assert!(line.starts_with("2 nota(s)"));
        assert!(line.contains("[M2] segunda n…"));
        assert!(!line.contains("primeira"), "só as N mais recentes");
    }
}
