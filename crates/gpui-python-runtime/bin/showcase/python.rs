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
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use std::time::{Duration, Instant};

type SharedResult<T> = Arc<Mutex<Option<Result<T, Box<dyn Error + Send + Sync>>>>>;

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
    let mut child = Command::new(python_executable())
        .arg(&script)
        .env("GPUI_TOOLKIT_SESSION", "1")
        .env("PYTHONPATH", python_path(&script))
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
        let mut reader = BufReader::new(stdout);
        loop {
            let mut line = Vec::new();
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
                return;
            }
            let mut parsed = match parse_python_message(&line, DEFAULT_MAX_SESSION_MESSAGE_BYTES) {
                Ok(message) => message,
                Err(error) => {
                    let _ = tx.send(Err(error.to_string()));
                    reader_wake.notify();
                    return;
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
            }
            if tx.send(Ok(parsed)).is_err() {
                break;
            }
            reader_wake.notify();
        }
        let _ = tx.send(Err("Python session stdout closed unexpectedly".into()));
        reader_wake.notify();
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
    })
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
