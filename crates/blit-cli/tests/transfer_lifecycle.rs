//! etl-3: the real CLI process carries one lifecycle trace from async-main
//! entry through both remote initiator roles, result rendering, terminal, and
//! flush. The fixture is deliberately tiny; throughput validation is etl-5.

use std::fs;
use std::process::{Command, Output};
use std::time::Duration;

mod common;
use common::{run_with_timeout, TestContext};

const PREFIX: &str = "[transfer-lifecycle] ";

fn lifecycle_events(output: &Output) -> Vec<serde_json::Value> {
    String::from_utf8_lossy(&output.stderr)
        .lines()
        .filter_map(|line| line.strip_prefix(PREFIX))
        .map(|json| serde_json::from_str(json).expect("valid lifecycle JSON"))
        .collect()
}

fn run_traced(mut command: Command, run_id: &str) -> Output {
    command
        .env("BLIT_TRACE_SESSION_PHASES", "1")
        .env("BLIT_TRACE_RUN_ID", run_id);
    run_with_timeout(command, Duration::from_secs(60))
}

fn assert_complete_timeline(output: &Output, run_id: &str, role: &str) {
    assert!(
        output.status.success(),
        "traced command failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let events = lifecycle_events(output);
    let names = events
        .iter()
        .map(|event| event["event"].as_str().expect("event name"))
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        [
            "async_main_enter",
            "argument_parse_end",
            "context_load_begin",
            "context_load_end",
            "transfer_dispatch_begin",
            "transfer_route_select_begin",
            "transfer_route_select_end",
            "control_connect_begin",
            "control_connect_end",
            "transfer_rpc_open_begin",
            "transfer_rpc_open_end",
            "session_establish_begin",
            "session_establish_end",
            "session_body_return",
            "result_render_begin",
            "result_render_end",
            "transfer_dispatch_end",
            "command_terminal",
        ],
        "unexpected lifecycle timeline\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    for (sequence, event) in events.iter().enumerate() {
        assert_eq!(event["schema"], 1);
        assert_eq!(event["run_id"], run_id);
        assert_eq!(event["producer_seq"], sequence as u64);
    }
    assert_eq!(events[6]["initiator_role"], role);
    assert!(
        events[..6]
            .iter()
            .all(|event| event.get("initiator_role").is_none()),
        "role must appear only after route selection"
    );

    let session_id = events[12]["session_id"]
        .as_str()
        .expect("session id attached after establishment");
    assert!(!session_id.is_empty());
    assert_eq!(events[13]["session_id"], session_id);
    assert_eq!(events.last().unwrap()["outcome"], "SUCCESS");
    assert_eq!(
        events
            .iter()
            .filter(|event| event["event"] == "command_terminal")
            .count(),
        1,
        "exactly one command terminal"
    );
}

#[test]
fn cli_lifecycle_covers_push_pull_and_trace_off_silence() {
    let ctx = TestContext::new();
    let source = ctx.workspace.join("lifecycle-source");
    fs::create_dir_all(&source).expect("source dir");
    fs::write(source.join("tiny.txt"), b"tiny lifecycle fixture").expect("source file");

    let remote = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut push = Command::new(&ctx.cli_bin);
    push.arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("copy")
        .arg(format!("{}/", source.display()))
        .arg(&remote);
    let push_output = run_traced(push, "etl3-push");
    assert_complete_timeline(&push_output, "etl3-push", "SOURCE");
    assert_eq!(
        fs::read(ctx.module_dir.join("tiny.txt")).expect("pushed file"),
        b"tiny lifecycle fixture"
    );

    let pull_destination = ctx.workspace.join("lifecycle-pull");
    let mut pull = Command::new(&ctx.cli_bin);
    pull.arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("copy")
        .arg(&remote)
        .arg(&pull_destination);
    let pull_output = run_traced(pull, "etl3-pull");
    assert_complete_timeline(&pull_output, "etl3-pull", "DESTINATION");
    assert_eq!(
        fs::read(pull_destination.join("tiny.txt")).expect("pulled file"),
        b"tiny lifecycle fixture"
    );

    let local_destination = ctx.workspace.join("trace-off-destination");
    let mut trace_off = Command::new(&ctx.cli_bin);
    trace_off
        .env_remove("BLIT_TRACE_SESSION_PHASES")
        .env_remove("BLIT_TRACE_RUN_ID")
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("copy")
        .arg(format!("{}/", source.display()))
        .arg(&local_destination);
    let trace_off_output = run_with_timeout(trace_off, Duration::from_secs(30));
    assert!(trace_off_output.status.success());
    assert!(
        lifecycle_events(&trace_off_output).is_empty(),
        "trace-off command emitted lifecycle records"
    );
}
