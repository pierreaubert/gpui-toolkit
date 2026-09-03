use super::misc::default_showcase_path;
use super::misc::repo_root;
use gpui_python_runtime::session::{
    DEFAULT_HOST_CAPABILITIES, DEFAULT_MAX_SESSION_MESSAGE_BYTES, HostMessage, PythonMessage,
    UiEvent, parse_python_message,
};
use gpui_python_runtime::ui_ir::PythonAppIr;
use std::collections::VecDeque;
use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::future::Future;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use std::time::{Duration, Instant};

type SharedResult<T> = Arc<Mutex<Option<Result<T, Box<dyn Error + Send + Sync>>>>>;

#[derive(Clone)]
struct MmapSessionConfig {
    directory: PathBuf,
    token: String,
}

/// Supervised persistent Python child. Stdout and stderr are drained on helper
/// threads, so neither a chatty application nor a stalled GPUI frame can block
/// the child process on a full pipe.
pub(super) struct PythonSession {
    child: Arc<Mutex<std::process::Child>>,
    stdin: Arc<Mutex<std::process::ChildStdin>>,
    messages: Receiver<Result<PythonMessage, String>>,
    /// Messages emitted by `on_session_ready` can legally precede the initial
    /// snapshot (for example restored job updates). Keep them until the host
    /// entity has been constructed instead of treating ordering as fatal.
    pending: Arc<Mutex<VecDeque<PythonMessage>>>,
    pub stderr: Arc<Mutex<Vec<String>>>,
    event_sequence: Arc<AtomicU64>,
    wake: PythonSessionWake,
    /// Owns and crash-cleans all mmap publication files for this session.
    _resource_directory: tempfile::TempDir,
}

/// A zero-allocation bridge from the blocking stdout reader to GPUI's async
/// executor. A notification coalesces messages; the entity drains the actual
/// bounded channel on the foreground task.
#[derive(Clone)]
pub(super) struct PythonSessionWake {
    notified: Arc<std::sync::atomic::AtomicBool>,
    waker: Arc<Mutex<Option<Waker>>>,
}

impl PythonSessionWake {
    fn new() -> Self {
        Self {
            notified: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            waker: Arc::new(Mutex::new(None)),
        }
    }

    fn notify(&self) {
        self.notified.store(true, Ordering::Release);
        if let Some(waker) = self.waker.lock().ok().and_then(|mut waker| waker.take()) {
            waker.wake();
        }
    }
}

impl Future for PythonSessionWake {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.notified.swap(false, Ordering::AcqRel) {
            return Poll::Ready(());
        }
        if let Ok(mut waker) = self.waker.lock() {
            *waker = Some(cx.waker().clone());
        }
        if self.notified.swap(false, Ordering::AcqRel) {
            if let Ok(mut waker) = self.waker.lock() {
                *waker = None;
            }
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

/// Cloneable, write-only half of a session for native UI callbacks. The
/// receiver remains owned by `PythonSession`, while controls can emit events
/// from their `'static` callback closures.
#[derive(Clone)]
pub(super) struct PythonEventSink {
    stdin: Arc<Mutex<std::process::ChildStdin>>,
    event_sequence: Arc<AtomicU64>,
}

impl PythonEventSink {
    pub fn send(&self, message: &HostMessage) -> Result<(), Box<dyn Error + Send + Sync>> {
        let encoded = serde_json::to_vec(message)?;
        if encoded.len() > DEFAULT_MAX_SESSION_MESSAGE_BYTES {
            return Err("host session message exceeds maximum size".into());
        }
        let mut stdin = self
            .stdin
            .lock()
            .map_err(|_| "python stdin lock poisoned")?;
        stdin.write_all(&encoded)?;
        stdin.write_all(b"\n")?;
        stdin.flush()?;
        Ok(())
    }

    pub fn dispatch(
        &self,
        node_id: impl Into<String>,
        event: impl Into<String>,
        action: Option<String>,
        payload: serde_json::Value,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let sequence = self.event_sequence.fetch_add(1, Ordering::Relaxed) + 1;
        let message = HostMessage::Event(UiEvent {
            id: format!("event-{sequence}"),
            sequence,
            node_id: node_id.into(),
            event: event.into(),
            action,
            payload,
        });
        self.send(&message)
    }
}

impl PythonSession {
    pub fn wake_handle(&self) -> PythonSessionWake {
        self.wake.clone()
    }

    pub fn event_sink(&self) -> PythonEventSink {
        PythonEventSink {
            stdin: self.stdin.clone(),
            event_sequence: self.event_sequence.clone(),
        }
    }

    pub fn send(&self, message: &HostMessage) -> Result<(), Box<dyn Error + Send + Sync>> {
        let encoded = serde_json::to_vec(message)?;
        if encoded.len() > DEFAULT_MAX_SESSION_MESSAGE_BYTES {
            return Err("host session message exceeds maximum size".into());
        }
        let mut stdin = self
            .stdin
            .lock()
            .map_err(|_| "python stdin lock poisoned")?;
        stdin.write_all(&encoded)?;
        stdin.write_all(b"\n")?;
        stdin.flush()?;
        Ok(())
    }

    pub fn try_recv(&self) -> Option<Result<PythonMessage, String>> {
        if let Ok(mut pending) = self.pending.lock()
            && let Some(message) = pending.pop_front()
        {
            return Some(Ok(message));
        }
        self.messages.try_recv().ok()
    }

    pub fn stderr_diagnostics(&self) -> String {
        self.stderr
            .lock()
            .map(|lines| lines.iter().rev().take(30).cloned().collect::<Vec<_>>())
            .unwrap_or_default()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn recv(&self) -> Result<PythonMessage, Box<dyn Error + Send + Sync>> {
        if let Ok(mut pending) = self.pending.lock()
            && let Some(message) = pending.pop_front()
        {
            return Ok(message);
        }
        self.messages
            .recv()
            .map_err(|error| -> Box<dyn Error + Send + Sync> { error.to_string().into() })?
            .map_err(Into::into)
    }

    fn prepend_messages(&self, messages: Vec<PythonMessage>) {
        if let Ok(mut pending) = self.pending.lock() {
            for message in messages.into_iter().rev() {
                pending.push_front(message);
            }
        }
    }

    pub fn shutdown(&self) {
        self.wake.notify();
        let _ = self.send(&HostMessage::Shutdown(
            gpui_python_runtime::session::Shutdown {
                reason: "host_shutdown".into(),
            },
        ));
        if let Ok(mut child) = self.child.lock() {
            let timeout = env::var("GPUI_TOOLKIT_SHUTDOWN_TIMEOUT_MS")
                .ok()
                .and_then(|value| value.parse::<u64>().ok())
                .map(Duration::from_millis)
                .unwrap_or(Duration::from_secs(2));
            let deadline = Instant::now() + timeout;
            while Instant::now() < deadline {
                match child.try_wait() {
                    Ok(Some(_)) | Err(_) => return,
                    Ok(None) => std::thread::sleep(Duration::from_millis(10)),
                }
            }
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Drop for PythonSession {
    fn drop(&mut self) {
        self.shutdown();
    }
}

pub(super) fn spawn_python_session() -> Result<PythonSession, Box<dyn Error + Send + Sync>> {
    let script = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(default_showcase_path);
    spawn_python_session_for_script(script)
}

fn spawn_python_session_for_script(
    script: PathBuf,
) -> Result<PythonSession, Box<dyn Error + Send + Sync>> {
    let resource_directory = tempfile::Builder::new()
        .prefix("gpui-toolkit-resource-")
        .rand_bytes(32)
        .tempdir()?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(
            resource_directory.path(),
            std::fs::Permissions::from_mode(0o700),
        )?;
    }
    let resource_token = resource_directory
        .path()
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("resource directory name is not UTF-8")?
        .to_owned();
    let mmap_config = MmapSessionConfig {
        directory: resource_directory.path().to_path_buf(),
        token: resource_token.clone(),
    };
    let mut child = Command::new(python_executable())
        .arg(&script)
        .env("GPUI_TOOLKIT_SESSION", "1")
        .env("PYTHONPATH", python_path(&script))
        .env("GPUI_TOOLKIT_RESOURCE_DIR", resource_directory.path())
        .env("GPUI_TOOLKIT_RESOURCE_TOKEN", resource_token)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let stdin = child.stdin.take().ok_or("failed to capture Python stdin")?;
    let stdout = child
        .stdout
        .take()
        .ok_or("failed to capture Python stdout")?;
    let stderr = child
        .stderr
        .take()
        .ok_or("failed to capture Python stderr")?;
    // Keep the render thread decoupled from a chatty child while bounding host
    // memory. Backpressure is applied on this reader thread, never in GPUI.
    let (tx, rx) = mpsc::sync_channel(256);
    let wake = PythonSessionWake::new();
    let reader_wake = wake.clone();
    std::thread::spawn(move || {
        read_python_messages_with_mmap(BufReader::new(stdout), tx, reader_wake, Some(mmap_config));
    });
    let stderr_lines = Arc::new(Mutex::new(Vec::new()));
    let stderr_sink = stderr_lines.clone();
    std::thread::spawn(move || {
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            let mut lines = stderr_sink.lock().expect("stderr lock");
            lines.push(line);
            if lines.len() > 1_000 {
                lines.remove(0);
            }
        }
    });
    Ok(PythonSession {
        child: Arc::new(Mutex::new(child)),
        stdin: Arc::new(Mutex::new(stdin)),
        messages: rx,
        pending: Arc::new(Mutex::new(VecDeque::new())),
        stderr: stderr_lines,
        event_sequence: Arc::new(AtomicU64::new(0)),
        wake,
        _resource_directory: resource_directory,
    })
}

fn read_python_messages<R: BufRead>(
    reader: R,
    tx: SyncSender<Result<PythonMessage, String>>,
    reader_wake: PythonSessionWake,
) {
    read_python_messages_with_mmap(reader, tx, reader_wake, None);
}

fn prepare_mapped_frame(
    frame: &mut gpui_python_runtime::dataset_frames::MappedDatasetFrame,
    config: Option<&MmapSessionConfig>,
) -> Result<(), String> {
    let config = config.ok_or_else(|| {
        "Python requested mmap transport outside a negotiated session".to_string()
    })?;
    if frame.session_token != config.token {
        return Err("Python mmap session token does not match".into());
    }
    frame
        .validate_header()
        .map_err(|error| format!("invalid Python mmap frame header: {error}"))?;
    let relative = Path::new(&frame.filename);
    let mut components = relative.components();
    if !matches!(components.next(), Some(std::path::Component::Normal(_)))
        || components.next().is_some()
    {
        return Err("Python mmap frame filename is not local".into());
    }
    let path = config.directory.join(relative);
    let payload = gpui_python_runtime::dataset_frames::MappedDatasetPayload::map_file(
        &path,
        frame.byte_length,
    )
    .map_err(|error| {
        let _ = std::fs::remove_file(&path);
        format!("invalid Python mmap frame: {error}")
    })?;
    frame.payload = Some(Arc::new(payload));
    frame
        .validate()
        .map_err(|error| format!("invalid Python mmap frame: {error}"))
}

fn read_python_messages_with_mmap<R: BufRead>(
    mut reader: R,
    tx: SyncSender<Result<PythonMessage, String>>,
    reader_wake: PythonSessionWake,
    mmap_config: Option<MmapSessionConfig>,
) {
    let mut line = Vec::new();
    loop {
        line.clear();
        match reader.read_until(b'\n', &mut line) {
            Ok(0) => break,
            Ok(_) => {}
            Err(error) => {
                let _ = tx.send(Err(error.to_string()));
                reader_wake.notify();
                return;
            }
        }
        if line.last() == Some(&b'\n') {
            line.pop();
        }
        if line.last() == Some(&b'\r') {
            line.pop();
        }
        if line.is_empty() {
            continue;
        }
        if line.len() > DEFAULT_MAX_SESSION_MESSAGE_BYTES {
            let _ = tx.send(Err("Python session message exceeds maximum size".into()));
            reader_wake.notify();
            // `read_until` consumed the complete control line, so the
            // newline-delimited stream is still synchronized. Keep the
            // child alive and allow a later valid patch or heartbeat to
            // recover the session.
            continue;
        }
        let mut parsed = match parse_python_message(&line, DEFAULT_MAX_SESSION_MESSAGE_BYTES) {
            Ok(message) => message,
            Err(error) => {
                let _ = tx.send(Err(error.to_string()));
                reader_wake.notify();
                // JSON/control-message failures are line-local. The
                // reader has consumed the delimiter, unlike a malformed
                // binary frame whose payload length can desynchronize the
                // stream, so continue reading subsequent messages.
                continue;
            }
        };
        if let PythonMessage::ResourceFrame(frame) = &mut parsed {
            if frame.byte_length > gpui_python_runtime::audio_stream::MAX_AUDIO_FRAME_BYTES {
                let _ = tx.send(Err("Python audio frame exceeds maximum size".into()));
                reader_wake.notify();
                return;
            }
            frame.payload.resize(frame.byte_length, 0);
            if let Err(error) = reader.read_exact(&mut frame.payload) {
                let _ = tx.send(Err(format!("truncated Python audio frame: {error}")));
                reader_wake.notify();
                return;
            }
            let mut delimiter = [0_u8; 1];
            if reader.read_exact(&mut delimiter).is_err() || delimiter[0] != b'\n' {
                let _ = tx.send(Err("Python audio frame is missing its delimiter".into()));
                reader_wake.notify();
                return;
            }
        } else if let PythonMessage::DatasetFrame(frame) = &mut parsed {
            if frame.byte_length > gpui_python_runtime::dataset_frames::MAX_DATASET_FRAME_BYTES {
                let _ = tx.send(Err("Python dataset frame exceeds maximum size".into()));
                reader_wake.notify();
                return;
            }
            frame.payload.resize(frame.byte_length, 0);
            if let Err(error) = reader.read_exact(&mut frame.payload) {
                let _ = tx.send(Err(format!("truncated Python dataset frame: {error}")));
                reader_wake.notify();
                return;
            }
            let mut delimiter = [0_u8; 1];
            if reader.read_exact(&mut delimiter).is_err() || delimiter[0] != b'\n' {
                let _ = tx.send(Err("Python dataset frame missing its delimiter".into()));
                reader_wake.notify();
                return;
            }
            if let Err(error) = frame.validate() {
                let _ = tx.send(Err(format!("invalid Python dataset frame: {error}")));
                reader_wake.notify();
            }
        } else if let PythonMessage::MappedDatasetFrame(frame) = &mut parsed {
            if let Err(error) = prepare_mapped_frame(frame, mmap_config.as_ref()) {
                let _ = tx.send(Err(error));
                reader_wake.notify();
            }
        } else if let PythonMessage::MeshFrame(frame) = &mut parsed {
            let byte_length = frame.payload.len();
            if byte_length > gpui_python_runtime::mesh_frames::MAX_MESH_FRAME_BYTES {
                let _ = tx.send(Err("Python mesh frame exceeds maximum size".into()));
                reader_wake.notify();
                return;
            }
            frame.payload.resize(byte_length, 0);
            if let Err(error) = reader.read_exact(&mut frame.payload) {
                let _ = tx.send(Err(format!("truncated Python mesh frame: {error}")));
                reader_wake.notify();
                return;
            }
            let mut delimiter = [0_u8; 1];
            if reader.read_exact(&mut delimiter).is_err() || delimiter[0] != b'\n' {
                let _ = tx.send(Err("Python mesh frame is missing its delimiter".into()));
                reader_wake.notify();
                return;
            }
            if let Err(error) = frame.validate() {
                let _ = tx.send(Err(format!("invalid Python mesh frame: {error}")));
                reader_wake.notify();
                // The payload and delimiter have both been consumed, so the
                // newline-delimited stream is still synchronized. Keep the
                // session alive and allow a later generation or heartbeat to
                // recover after a frame-local validation error.
            }
        }
        if tx.send(Ok(parsed)).is_err() {
            break;
        }
        reader_wake.notify();
    }
    let _ = tx.send(Err("Python session stdout closed unexpectedly".into()));
    reader_wake.notify();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn python_session_shutdown_handshake_reaps_child_process() {
        let script = env::temp_dir().join(format!(
            "gpui-toolkit-python-shutdown-{}.py",
            std::process::id()
        ));
        let marker = script.with_extension("marker");
        let _ = std::fs::remove_file(&script);
        let _ = std::fs::remove_file(&marker);
        std::fs::write(
            &script,
            r#"import json
import pathlib
import sys

for line in sys.stdin:
    message = json.loads(line)
    if message.get("type") == "shutdown":
        pathlib.Path(__file__).with_suffix(".marker").write_text("shutdown", encoding="utf-8")
        break
"#,
        )
        .expect("write shutdown probe script");

        let session =
            spawn_python_session_for_script(script.clone()).expect("spawn Python shutdown probe");
        session.shutdown();
        drop(session);

        assert_eq!(
            std::fs::read_to_string(&marker).expect("read shutdown probe marker"),
            "shutdown",
            "host shutdown must reach the persistent Python child before reaping it"
        );
        let _ = std::fs::remove_file(script);
        let _ = std::fs::remove_file(marker);
    }

    #[test]
    fn spawned_python_session_publishes_through_private_mmap_directory() {
        let directory = tempfile::tempdir().unwrap();
        let script = directory.path().join("mmap_probe.py");
        std::fs::write(
            &script,
            r#"import json
import os
import sys

for line in sys.stdin:
    message = json.loads(line)
    if message.get("type") == "initialize":
        payload = b"cross-process-mmap"
        filename = "resource.bin"
        path = os.path.join(os.environ["GPUI_TOOLKIT_RESOURCE_DIR"], filename)
        descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
        with os.fdopen(descriptor, "wb") as stream:
            stream.write(payload)
        checksum = 0xCBF29CE484222325
        for byte in payload:
            checksum = ((checksum ^ byte) * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
        print(json.dumps({
            "type": "mapped_dataset_frame",
            "resource_id": "probe",
            "generation": 1,
            "sequence": 0,
            "chunk_count": 1,
            "byte_length": len(payload),
            "schema_fingerprint": "probe-schema",
            "checksum": checksum,
            "filename": filename,
            "session_token": os.environ["GPUI_TOOLKIT_RESOURCE_TOKEN"],
        }), flush=True)
    elif message.get("type") == "shutdown":
        break
"#,
        )
        .unwrap();
        let session = spawn_python_session_for_script(script).unwrap();
        let resource_directory = session._resource_directory.path().to_path_buf();
        session
            .send(&HostMessage::Initialize(
                gpui_python_runtime::session::Initialize {
                    session_version: gpui_python_runtime::session::PYTHON_APP_SESSION_VERSION,
                    capabilities: DEFAULT_HOST_CAPABILITIES
                        .iter()
                        .map(|value| (*value).into())
                        .collect(),
                    platform: std::env::consts::OS.into(),
                    theme: "system".into(),
                    window: gpui_python_runtime::session::WindowMetadata {
                        width: 320.0,
                        height: 200.0,
                        scale_factor: 1.0,
                    },
                },
            ))
            .unwrap();
        let PythonMessage::MappedDatasetFrame(frame) = session.recv().unwrap() else {
            panic!("expected mmap frame from spawned Python process");
        };
        assert_eq!(
            frame.payload.expect("mapped payload").as_slice(),
            b"cross-process-mmap"
        );
        assert_eq!(std::fs::read_dir(&resource_directory).unwrap().count(), 0);
        session.shutdown();
        drop(session);
        assert!(!resource_directory.exists());
    }

    #[test]
    fn reader_recovers_after_malformed_patch() {
        let (tx, rx) = mpsc::sync_channel(8);
        let wake = PythonSessionWake::new();
        read_python_messages(
            Cursor::new(
                b"{\"type\":\"patch\",\"revision\":\"not-a-number\",\"ops\":[]}\n{\"type\":\"heartbeat\",\"id\":\"after-malformed\"}\n".to_vec(),
            ),
            tx,
            wake,
        );

        let malformed = rx.recv().expect("malformed message diagnostic");
        assert!(matches!(
            malformed,
            Err(message) if message.contains("malformed session message")
        ));
        assert_eq!(
            rx.recv().expect("message after malformed line"),
            Ok(PythonMessage::Heartbeat {
                id: "after-malformed".into()
            })
        );
        assert!(matches!(
            rx.recv().expect("stream-close diagnostic"),
            Err(message) if message.contains("stdout closed")
        ));
    }

    #[test]
    fn stale_patch_does_not_block_later_session_messages() {
        let (tx, rx) = mpsc::sync_channel(8);
        let wake = PythonSessionWake::new();
        read_python_messages(
            Cursor::new(
                b"{\"type\":\"patch\",\"revision\":1,\"ops\":[]}\n{\"type\":\"patch\",\"revision\":1,\"ops\":[]}\n{\"type\":\"heartbeat\",\"id\":\"after-stale\"}\n"
                    .to_vec(),
            ),
            tx,
            wake,
        );

        let mut state = gpui_python_runtime::session::SessionState::new(vec!["patches".into()]);
        let first = rx.recv().expect("first patch").expect("valid first patch");
        let second = rx.recv().expect("stale patch").expect("parsed stale patch");
        let PythonMessage::Patch(first) = first else {
            panic!("expected first patch");
        };
        let PythonMessage::Patch(second) = second else {
            panic!("expected stale patch");
        };
        state
            .apply_patch_revision(&first)
            .expect("first patch accepted");
        assert!(matches!(
            state.apply_patch_revision(&second),
            Err(gpui_python_runtime::session::SessionError::StaleRevision { .. })
        ));
        assert_eq!(
            rx.recv().expect("message after stale patch"),
            Ok(PythonMessage::Heartbeat {
                id: "after-stale".into()
            })
        );
    }

    #[test]
    fn reader_recovers_after_a_consumed_invalid_mesh_frame() {
        let (tx, rx) = mpsc::sync_channel(8);
        let wake = PythonSessionWake::new();
        let header = serde_json::json!({
            "type": "mesh_frame",
            "resource_id": "field",
            "generation": 1,
            "sequence": 0,
            "chunk_count": 1,
            "kind": "field",
            "dtype": "u64le",
            "shape": [2],
            "byte_length": 8,
            "checksum": 0,
        });
        let mut stream = serde_json::to_vec(&header).unwrap();
        stream.extend_from_slice(b"\n12345678\n");
        stream.extend_from_slice(b"{\"type\":\"heartbeat\",\"id\":\"after-frame\"}\n");

        read_python_messages(Cursor::new(stream), tx, wake);

        let malformed = rx.recv().expect("invalid frame diagnostic");
        assert!(matches!(
            malformed,
            Err(message) if message.contains("invalid Python mesh frame")
        ));
        let forwarded = rx.recv().expect("rejected frame forwarding");
        assert!(matches!(
            forwarded,
            Ok(PythonMessage::MeshFrame(frame)) if frame.validate().is_err()
        ));
        assert_eq!(
            rx.recv().expect("message after invalid frame"),
            Ok(PythonMessage::Heartbeat {
                id: "after-frame".into()
            })
        );
    }

    #[test]
    fn reader_preserves_audio_frame_payload_and_following_drop_resource() {
        let (tx, rx) = mpsc::sync_channel(8);
        let wake = PythonSessionWake::new();
        let header = serde_json::json!({
            "type": "resource_frame",
            "resource_id": "meter",
            "generation": 3,
            "sequence": 1,
            "frame_kind": "meter",
            "byte_length": 4,
            "shape": [1, 1],
            "dtype": "f32",
            "byte_order": "little",
            "finite_policy": "drop_frame",
            "coalesce": "latest",
            "sample_rate": 48_000.0,
            "attack_ms": 10.0,
            "release_ms": 120.0,
        });
        let mut stream = serde_json::to_vec(&header).unwrap();
        stream.extend_from_slice(b"\n");
        stream.extend_from_slice(&1.25_f32.to_le_bytes());
        stream.extend_from_slice(b"\n");
        stream
            .extend_from_slice(br#"{"type":"drop_resource","resource_id":"meter","generation":3}"#);
        stream.extend_from_slice(b"\n");

        read_python_messages(Cursor::new(stream), tx, wake);

        let Ok(PythonMessage::ResourceFrame(frame)) = rx.recv().expect("audio frame") else {
            panic!("expected audio resource frame");
        };
        assert_eq!(frame.resource_id, "meter");
        assert_eq!(frame.generation, 3);
        assert_eq!(frame.payload, 1.25_f32.to_le_bytes());
        assert_eq!(
            rx.recv().expect("drop resource after audio frame"),
            Ok(PythonMessage::DropResource {
                resource_id: "meter".into(),
                generation: 3,
            })
        );
    }

    #[test]
    fn reader_maps_session_resource_and_unlinks_publication_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("resource.bin");
        let payload = b"mapped-values";
        std::fs::write(&path, payload).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        }
        let config = MmapSessionConfig {
            directory: directory.path().to_path_buf(),
            token: "session-token".into(),
        };
        let header = serde_json::json!({
            "type": "mapped_dataset_frame",
            "resource_id": "events",
            "generation": 4,
            "sequence": 0,
            "chunk_count": 1,
            "byte_length": payload.len(),
            "schema_fingerprint": "events-v4",
            "checksum": gpui_python_runtime::dataset_frames::DatasetFrame::checksum(payload),
            "filename": "resource.bin",
            "session_token": "session-token",
        });
        let mut stream = serde_json::to_vec(&header).unwrap();
        stream.push(b'\n');
        let (tx, rx) = mpsc::sync_channel(4);
        read_python_messages_with_mmap(
            Cursor::new(stream),
            tx,
            PythonSessionWake::new(),
            Some(config),
        );

        let Ok(PythonMessage::MappedDatasetFrame(frame)) = rx.recv().expect("mapped dataset frame")
        else {
            panic!("expected mapped dataset frame");
        };
        assert_eq!(
            frame.payload.expect("mapped payload").as_slice(),
            payload.as_slice()
        );
        assert!(!path.exists());
    }

    #[test]
    fn reader_rejects_mmap_frame_from_another_session() {
        let directory = tempfile::tempdir().unwrap();
        let header = serde_json::json!({
            "type": "mapped_dataset_frame",
            "resource_id": "events",
            "generation": 1,
            "sequence": 0,
            "chunk_count": 1,
            "byte_length": 8,
            "schema_fingerprint": "events-v1",
            "checksum": 0,
            "filename": "resource.bin",
            "session_token": "foreign-token",
        });
        let mut stream = serde_json::to_vec(&header).unwrap();
        stream.push(b'\n');
        let (tx, rx) = mpsc::sync_channel(4);
        read_python_messages_with_mmap(
            Cursor::new(stream),
            tx,
            PythonSessionWake::new(),
            Some(MmapSessionConfig {
                directory: directory.path().to_path_buf(),
                token: "local-token".into(),
            }),
        );
        assert!(matches!(
            rx.recv().expect("session token diagnostic"),
            Err(message) if message.contains("session token does not match")
        ));
        assert!(matches!(
            rx.recv().expect("rejected frame forwarded for acknowledgement"),
            Ok(PythonMessage::MappedDatasetFrame(frame)) if frame.payload.is_none()
        ));
    }
}

pub(super) fn load_python_session_blocking()
-> Result<(PythonAppIr, PythonSession), Box<dyn Error + Send + Sync>> {
    let session = spawn_python_session()?;
    session.send(&HostMessage::Initialize(
        gpui_python_runtime::session::Initialize {
            session_version: gpui_python_runtime::session::PYTHON_APP_SESSION_VERSION,
            capabilities: DEFAULT_HOST_CAPABILITIES
                .iter()
                .map(|value| (*value).into())
                .collect(),
            platform: std::env::consts::OS.into(),
            theme: "system".into(),
            window: gpui_python_runtime::session::WindowMetadata {
                width: 1240.0,
                height: 820.0,
                scale_factor: 1.0,
            },
        },
    ))?;
    match session.recv()? {
        PythonMessage::Ready(ready) => {
            gpui_python_runtime::session::SessionState::new(
                DEFAULT_HOST_CAPABILITIES
                    .iter()
                    .map(|capability| (*capability).into())
                    .collect(),
            )
            .validate_ready(&ready)?;
        }
        other => return Err(format!("expected Python session ready, received {other:?}").into()),
    }
    let mut before_snapshot = Vec::new();
    let app_ir = loop {
        match session.recv()? {
            PythonMessage::Snapshot { app_ir } => {
                app_ir.validate()?;
                break app_ir;
            }
            message => before_snapshot.push(message),
        }
    };
    // Re-play startup effects, commands, jobs, and diagnostics through the
    // normal host message loop after the initial UI tree is available.
    session.prepend_messages(before_snapshot);
    Ok((app_ir, session))
}

/// Validate the initial snapshot through the same persistent-session
/// handshake used by the interactive host.
pub(super) fn load_python_app_blocking() -> Result<PythonAppIr, Box<dyn Error + Send + Sync>> {
    let (app, _session) = load_python_session_blocking()?;
    Ok(app)
}

struct BackgroundFuture<T> {
    result: SharedResult<T>,
    waker: Arc<Mutex<Option<Waker>>>,
}

impl<T> Future for BackgroundFuture<T> {
    type Output = Result<T, Box<dyn Error + Send + Sync>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Some(result) = self.result.lock().unwrap().take() {
            return Poll::Ready(result);
        }
        *self.waker.lock().unwrap() = Some(cx.waker().clone());
        Poll::Pending
    }
}

pub(super) fn load_python_session_async()
-> impl Future<Output = Result<(PythonAppIr, PythonSession), Box<dyn Error + Send + Sync>>> {
    let result = Arc::new(Mutex::new(None));
    let waker = Arc::new(Mutex::new(None::<Waker>));
    let result2 = result.clone();
    let waker2 = waker.clone();
    std::thread::spawn(move || {
        let output = load_python_session_blocking();
        *result2.lock().unwrap() = Some(output);
        if let Some(w) = waker2.lock().unwrap().take() {
            w.wake();
        }
    });
    BackgroundFuture { result, waker }
}

pub(super) fn python_executable() -> OsString {
    if let Some(value) = env::var_os("GPUI_PYTHON") {
        return value;
    }
    let repo_venv = repo_root().join("venv/bin/python");
    if repo_venv.exists() {
        return repo_venv.into_os_string();
    }
    OsString::from("python3")
}

pub(super) fn python_path(script: &Path) -> OsString {
    let mut paths = vec![PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("python")];
    if let Some(parent) = script.parent() {
        paths.push(parent.to_path_buf());
    }
    if let Some(existing) = env::var_os("PYTHONPATH") {
        paths.extend(env::split_paths(&existing));
    }
    env::join_paths(paths).unwrap_or_else(|_| OsString::new())
}
