//! ADR 0063 — `lina memo` pelo BINÁRIO real: `add` enfileira `memo.add` no outbox por-nó (quem
//! carimba `id`/`by` é o supervisor), `list`/`search` leem o espelho `log.jsonl` (SÓ-LEITURA) e o
//! `whoami --bootstrap` (SessionStart) ganha a linha `MEMORIA DO ESPACO` quando há notas.

use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

struct TempWs {
    home: PathBuf,
    cwd: PathBuf,
}

impl TempWs {
    fn new(tag: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let root =
            std::env::temp_dir().join(format!("lina-memo-{tag}-{}-{nanos}", std::process::id()));
        let home = root.join("shared");
        let cwd = root.join("terminal-a");
        std::fs::create_dir_all(home.join("events")).expect("criar LINA_HOME/events");
        std::fs::create_dir_all(cwd.join(".lina")).expect("criar cwd/.lina");
        let bootstrap = r#"{"terminal_name":"Terminal A","roster":["Terminal A","QA"],"vault_path":"/tmp/none","autonomy":"assisted"}"#;
        std::fs::write(cwd.join(".lina/bootstrap.json"), bootstrap).expect("bootstrap.json");
        Self { home, cwd }
    }

    fn seed_log(&self, lines: &[&str]) {
        std::fs::write(
            self.home.join("events/log.jsonl"),
            format!("{}\n", lines.join("\n")),
        )
        .expect("log.jsonl");
    }
}

impl Drop for TempWs {
    fn drop(&mut self) {
        if let Some(root) = self.home.parent() {
            let _ = std::fs::remove_dir_all(root);
        }
    }
}

const MEMO_1: &str = r#"{"seq":1,"ts":1,"kind":"MemoNoted","version":1,"payload":{"event":"MemoNoted","id":"M1","text":"Decisão: banco é Postgres","by":"Maestro"}}"#;
const MEMO_2: &str = r#"{"seq":2,"ts":2,"kind":"MemoNoted","version":1,"payload":{"event":"MemoNoted","id":"M2","text":"cliente prefere tom informal","by":"Redator"}}"#;

fn run(ws: &TempWs, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_lina"))
        .args(args)
        .current_dir(&ws.cwd)
        .env("LINA_HOME", &ws.home)
        .env_remove("LINA_NODE_NAME")
        .output()
        .expect("executar o binário lina")
}

fn stdout(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn memo_add_enqueues_text_only_contract() {
    let ws = TempWs::new("add");
    let out = run(&ws, &["memo", "add", "usar", "pnpm"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let dir = ws.home.join("outbox").join("Terminal A");
    let envelope = std::fs::read_dir(&dir)
        .expect("outbox por-nó")
        .filter_map(Result::ok)
        .find(|e| e.path().extension().is_some_and(|x| x == "json"))
        .map(|e| std::fs::read_to_string(e.path()).expect("ler envelope"))
        .expect("um envelope");
    let msg: serde_json::Value = serde_json::from_str(&envelope).expect("json");
    assert_eq!(msg["intent"], "memo.add");
    let payload: serde_json::Value =
        serde_json::from_str(msg["payload"].as_str().expect("payload")).expect("payload json");
    assert_eq!(
        payload,
        serde_json::json!({ "text": "usar pnpm" }),
        "só o texto viaja"
    );

    assert!(
        !run(&ws, &["memo", "add"]).status.success(),
        "sem texto recusa"
    );
}

#[test]
fn memo_list_and_search_read_the_log() {
    let ws = TempWs::new("ler");
    ws.seed_log(&[MEMO_1, MEMO_2]);
    let list = stdout(&run(&ws, &["memo", "list"]));
    assert!(
        list.find("[M2]") < list.find("[M1]"),
        "mais recente primeiro: {list}"
    );
    let hits = stdout(&run(&ws, &["memo", "search", "decisao"]));
    assert!(
        hits.contains("[M1] Decisão: banco é Postgres — por Maestro"),
        "{hits}"
    );
    assert!(!hits.contains("[M2]"));
}

#[test]
fn whoami_bootstrap_carries_memory_line_only_when_there_are_notes() {
    let ws = TempWs::new("whoami");
    let empty = stdout(&run(&ws, &["whoami", "--bootstrap"]));
    assert!(!empty.contains("MEMORIA DO ESPACO"), "{empty}");

    ws.seed_log(&[MEMO_1, MEMO_2]);
    let hook = stdout(&run(&ws, &["whoami", "--bootstrap"]));
    let json: serde_json::Value = serde_json::from_str(hook.trim()).expect("hook é JSON");
    let context = json["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .expect("contexto");
    assert!(context.contains("MEMORIA DO ESPACO"), "{context}");
    assert!(context.contains("2 nota(s)") && context.contains("[M2] cliente prefere"));
}
