#![cfg(all(feature = "embedded", unix))]

//! End-to-end embedded session: initialize, snapshot, event, patch, shutdown.
//!
//! Runs a real `CPython` interpreter in-process; no child process is spawned.

// Rust guideline compliant 2026-10-09

use std::io::BufRead;
use std::path::PathBuf;

use gpui_python_runtime::embedded::spawn_embedded_session;
use gpui_python_runtime::session::{
    DEFAULT_MAX_SESSION_MESSAGE_BYTES, HostMessage, Initialize, PYTHON_APP_SESSION_VERSION,
    PythonMessage, SessionState, UiEvent, WindowMetadata, parse_python_message,
};

fn read_message(reader: &mut impl BufRead) -> PythonMessage {
    let mut line = Vec::new();
    let read = reader
        .read_until(b'\n', &mut line)
        .expect("read session line");
    assert!(read > 0, "embedded Python closed the session stream early");
    parse_python_message(&line, DEFAULT_MAX_SESSION_MESSAGE_BYTES).expect("parse session message")
}

#[test]
fn embedded_session_runs_python_app_in_process() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let script = manifest.join("tests/fixtures/embedded_app.py");
    let python_root = manifest.join("python");

    let mut session =
        spawn_embedded_session(&script, &[python_root]).expect("spawn embedded session");

    let capabilities = vec!["events".to_string(), "patches".to_string()];
    session
        .send(&HostMessage::Initialize(Initialize {
            session_version: PYTHON_APP_SESSION_VERSION,
            capabilities: capabilities.clone(),
            platform: std::env::consts::OS.into(),
            theme: "system".into(),
            window: WindowMetadata {
                width: 800.0,
                height: 600.0,
                scale_factor: 1.0,
            },
        }))
        .expect("send initialize");

    let PythonMessage::Ready(ready) = read_message(&mut session.reader) else {
        panic!("expected ready as the first embedded session message");
    };
    SessionState::new(capabilities)
        .validate_ready(&ready)
        .expect("ready validates against session v1");

    let PythonMessage::Snapshot { app_ir } = read_message(&mut session.reader) else {
        panic!("expected snapshot after ready");
    };
    assert_eq!(app_ir.title, "Embedded Fixture");

    session
        .send(&HostMessage::Event(UiEvent {
            id: "event-1".into(),
            node_id: "button".into(),
            event: "click".into(),
            action: Some("tap".into()),
            payload: serde_json::Value::Object(Default::default()),
            sequence: 1,
        }))
        .expect("send event");

    let PythonMessage::Patch(patch) = read_message(&mut session.reader) else {
        panic!("expected patch in response to the event");
    };
    assert_eq!(patch.request_id.as_deref(), Some("event-1"));
    assert_eq!(patch.ops.len(), 1);

    session.shutdown("test complete").expect("clean shutdown");
}
