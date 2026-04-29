//! Round-trip latency measurements against the real `devlogger-mcp`
//! binary over stdio — the exact code path a host like Claude Code
//! takes.  Claude Code 2.1.x has a hardcoded 60 s request timeout
//! (tracked in anthropics/claude-code #40207 / #43299), so these tests
//! exist to prove empirically that our responses arrive in
//! milliseconds, not seconds, and to flag any regression that drags
//! them into the danger zone.
//!
//! Each measurement is asserted against a generous bound AND printed,
//! so `cargo test -- --nocapture` gives a quick latency profile.

use std::process::Stdio;
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{ChildStdin, ChildStdout, Command};
use tokio::time::timeout;

use devlogger::mcp::claude_code_race_workaround::{INITIALIZE_DELAY, REVIEW_BY};

// Bounds are intentionally loose — they are "is this a catastrophe?"
// alarms, not micro-benchmark thresholds.  Typical numbers on a
// release build are well under these.  The `INITIALIZE_BUDGET` has
// to accommodate the claude-code-race workaround sleep (see
// `claude_code_race_workaround.rs`) plus process-spawn overhead.
const INITIALIZE_BUDGET: Duration = Duration::from_secs(2);
const TOOLS_LIST_BUDGET: Duration = Duration::from_secs(2);
const TOOL_CALL_BUDGET: Duration = Duration::from_secs(2);

async fn read_frame(reader: &mut BufReader<ChildStdout>) -> Value {
    let mut buf = String::new();
    timeout(Duration::from_secs(10), reader.read_line(&mut buf))
        .await
        .expect("stdout frame within 10s")
        .expect("read_line");
    serde_json::from_str(buf.trim_end()).expect("valid JSON-RPC frame")
}

async fn send(stdin: &mut ChildStdin, v: &Value) {
    stdin
        .write_all(format!("{v}\n").as_bytes())
        .await
        .expect("write stdin");
}

/// Round-trip: send a request, read until the matching `id` arrives.
/// Returns `(response, elapsed)`.  Ignores server-initiated
/// notifications (no `id`), which rmcp may interleave.
async fn request(
    stdin: &mut ChildStdin,
    reader: &mut BufReader<ChildStdout>,
    req: Value,
) -> (Value, Duration) {
    let id = req["id"].clone();
    let start = Instant::now();
    send(stdin, &req).await;
    loop {
        let frame = read_frame(reader).await;
        if frame.get("id") == Some(&id) {
            return (frame, start.elapsed());
        }
    }
}

#[tokio::test]
async fn initialize_and_tools_list_complete_in_milliseconds() {
    let dir = tempfile::tempdir().unwrap();

    let spawn_start = Instant::now();
    let mut child = Command::new(env!("CARGO_BIN_EXE_devlogger-mcp"))
        .arg("--dir")
        .arg(dir.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");

    let mut stdin = child.stdin.take().unwrap();
    let mut reader = BufReader::new(child.stdout.take().unwrap());

    let (init_resp, init_elapsed) = request(
        &mut stdin,
        &mut reader,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": { "name": "timing-test", "version": "0" }
            }
        }),
    )
    .await;
    let spawn_to_init = spawn_start.elapsed();
    assert_eq!(init_resp["id"], 1);

    send(
        &mut stdin,
        &json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }),
    )
    .await;

    let (list_resp, list_elapsed) = request(
        &mut stdin,
        &mut reader,
        json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" }),
    )
    .await;
    assert_eq!(list_resp["id"], 2);
    let tools = list_resp["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 6);

    // Size of the tools/list payload matters because Claude Code
    // serializes, writes, and re-parses it; a pathologically large
    // payload could in theory back-pressure the stdio pipe.  Print it
    // so any balloon in descriptions is visible.
    let payload_bytes = serde_json::to_vec(&list_resp).unwrap().len();

    eprintln!("--- devlogger-mcp wire-latency profile ---");
    eprintln!("spawn -> initialize response : {spawn_to_init:?}");
    eprintln!("initialize round-trip        : {init_elapsed:?}");
    eprintln!("tools/list round-trip        : {list_elapsed:?}");
    eprintln!("tools/list payload size      : {payload_bytes} bytes");

    assert!(
        init_elapsed < INITIALIZE_BUDGET,
        "initialize took {init_elapsed:?} (budget {INITIALIZE_BUDGET:?}); \
         well below Claude Code's 60 s timeout but still a red flag"
    );
    // Lower bound: the claude-code-race workaround MUST still be
    // applying its ~1.2 s sleep.  If this fails it almost certainly
    // means somebody deleted the override in `src/mcp/server.rs` or
    // the module in `src/mcp/claude_code_race_workaround.rs` — and
    // the reconnect-every-minute bug is now back.  DO NOT just lower
    // the bound; go read the module header first.
    let floor = INITIALIZE_DELAY - Duration::from_millis(100);
    assert!(
        init_elapsed >= floor,
        "initialize returned in {init_elapsed:?}, shorter than the expected \
         claude-code-race workaround floor of {floor:?}.  Did somebody drop \
         the initialize() override or delete src/mcp/claude_code_race_workaround.rs?"
    );
    assert!(
        list_elapsed < TOOLS_LIST_BUDGET,
        "tools/list took {list_elapsed:?} (budget {TOOLS_LIST_BUDGET:?}); \
         this is what Claude Code's hardcoded 60 s timer races against"
    );

    drop(stdin);
    let _ = timeout(Duration::from_secs(5), child.wait()).await;
    let _ = child.kill().await;
}

/// Time-bomb: fails after the `REVIEW_BY` date declared in
/// `src/mcp/claude_code_race_workaround.rs`, forcing a human to go
/// re-check upstream status.  If the race is fixed — delete the
/// workaround.  If it isn't — bump `REVIEW_BY` and log a devlog entry
/// explaining what you checked.
#[test]
fn workaround_has_not_outlived_its_welcome() {
    let review_by = chrono::NaiveDate::parse_from_str(REVIEW_BY, "%Y-%m-%d")
        .expect("REVIEW_BY must be YYYY-MM-DD");
    let today = chrono::Utc::now().date_naive();
    assert!(
        today <= review_by,
        "claude-code initialize-race workaround has hit its REVIEW_BY \
         date ({review_by}); today is {today}. Check upstream \
         (anthropics/claude-code#50095, #43299, #40207) — if the race \
         is fixed, delete src/mcp/claude_code_race_workaround.rs; if \
         not, bump REVIEW_BY and log a devlog entry explaining why."
    );
}

#[tokio::test]
async fn every_tool_call_completes_in_milliseconds() {
    let dir = tempfile::tempdir().unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_devlogger-mcp"))
        .arg("--dir")
        .arg(dir.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    let mut stdin = child.stdin.take().unwrap();
    let mut reader = BufReader::new(child.stdout.take().unwrap());

    // Handshake.
    let (_, _) = request(
        &mut stdin,
        &mut reader,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": { "name": "timing-test", "version": "0" }
            }
        }),
    )
    .await;
    send(
        &mut stdin,
        &json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }),
    )
    .await;

    let mut id = 10i64;
    let mut call = |name: &'static str, args: Value| {
        id += 1;
        let req = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tools/call",
            "params": { "name": name, "arguments": args }
        });
        (name, req)
    };

    let steps = vec![
        call(
            "devlog_new",
            json!({ "section": "timing", "text": "first" }),
        ),
        call(
            "devlog_new",
            json!({ "section": "timing", "text": "second" }),
        ),
        call("devlog_sections", json!({})),
        call("devlog_list", json!({ "section": "timing" })),
        call("devlog_list", json!({})),
        call("devlog_read", json!({ "section": "timing" })),
        call(
            "devlog_update",
            json!({ "section": "timing", "id": "1", "text": "first, rewritten" }),
        ),
        call(
            "devlog_move",
            json!({ "from_section": "timing", "id": "2", "to_section": "elsewhere" }),
        ),
    ];

    eprintln!("--- per-tool round-trip latencies ---");
    for (name, req) in steps {
        let (resp, elapsed) = request(&mut stdin, &mut reader, req).await;
        assert!(
            resp.get("error").is_none(),
            "{name} returned a protocol error: {resp}"
        );
        assert!(
            resp["result"]["isError"].as_bool() != Some(true),
            "{name} returned a tool-level error: {resp}"
        );
        eprintln!("{name:<18} : {elapsed:?}");
        assert!(
            elapsed < TOOL_CALL_BUDGET,
            "{name} took {elapsed:?} (budget {TOOL_CALL_BUDGET:?})"
        );
    }

    drop(stdin);
    let _ = timeout(Duration::from_secs(5), child.wait()).await;
    let _ = child.kill().await;
}
