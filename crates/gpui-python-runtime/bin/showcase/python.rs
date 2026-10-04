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
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::sync::{Arc, Condvar, Mutex, TryLockError};
use std::task::{Context, Poll, Waker};
use std::time::{Duration, Instant};

type SharedResult<T> = Arc<Mutex<Option<Result<T, Box<dyn Error + Send + Sync>>>>>;

// The writer queue is small because host input messages are tiny. Its byte
// budget also bounds memory when a child stops reading its stdin pipe.
const MAX_OUTBOUND_MESSAGES: usize = 128;
const MAX_OUTBOUND_BYTES: usize = 2 * 1024 * 1024;

#[derive(Clone)]
struct PythonOutboundWriter {
    shared: Arc<OutboundWriterShared>,
}

struct OutboundWriterShared {
    state: Mutex<OutboundWriterState>,
    ready: Condvar,
    event_sequence: AtomicU64,
    overflow_requested: AtomicBool,
    close_requested: AtomicBool,
}

struct OutboundWriterState {
    queue: VecDeque<Vec<u8>>,
    outstanding_messages: usize,
    outstanding_bytes: usize,
    recovering: bool,
    recovery_pending: bool,
    close_requested: bool,
    closed: bool,
    last_error: Option<String>,
}

impl PythonOutboundWriter {
    fn new(stdin: std::process::ChildStdin) -> Self {
        Self::with_writer(stdin)
    }

    fn with_writer(writer: impl Write + Send + 'static) -> Self {
        let shared = Arc::new(OutboundWriterShared {
            state: Mutex::new(OutboundWriterState {
                queue: VecDeque::new(),
                outstanding_messages: 0,
                outstanding_bytes: 0,
                recovering: false,
                recovery_pending: false,
                close_requested: false,
                closed: false,
                last_error: None,
            }),
            ready: Condvar::new(),
            event_sequence: AtomicU64::new(0),
            overflow_requested: AtomicBool::new(false),
            close_requested: AtomicBool::new(false),
        });
        let worker_state = shared.clone();
        std::thread::Builder::new()
            .name("gpui-python-writer".into())
            .spawn(move || run_outbound_writer(writer, worker_state))
            .expect("failed to start Python session writer");
        Self { shared }
    }

    fn send(&self, message: &HostMessage) -> Result<(), Box<dyn Error + Send + Sync>> {
        if matches!(message, HostMessage::Event(_)) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "sequenced host events must use the ordered event dispatcher",
            )
            .into());
        }
        let mut bytes = serde_json::to_vec(message)?;
        if bytes.len() > DEFAULT_MAX_SESSION_MESSAGE_BYTES {
            return Err("host session message exceeds maximum size".into());
        }
        bytes.push(b'\n');
        let mut state = match self.shared.state.try_lock() {
            Ok(state) => state,
            Err(TryLockError::WouldBlock) => {
                return Err(io::Error::new(
                    io::ErrorKind::WouldBlock,
                    "Python host-to-child queue is busy",
                )
                .into());
            }
            Err(TryLockError::Poisoned(_)) => {
                return Err("python writer queue lock poisoned".into());
            }
        };
        apply_requested_close(&self.shared, &mut state);
        apply_requested_overflow(&self.shared, &mut state);
        if state.closed {
            return Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                state
                    .last_error
                    .as_deref()
                    .unwrap_or("Python writer is closed"),
            )
            .into());
        }
        if state.close_requested {
            return Err(
                io::Error::new(io::ErrorKind::BrokenPipe, "Python writer is closing").into(),
            );
        }
        if state.recovery_pending || state.recovering {
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "Python event stream is recovering after overflow",
            )
            .into());
        }
        let over_limit = state.outstanding_messages >= MAX_OUTBOUND_MESSAGES
            || state.outstanding_bytes.saturating_add(bytes.len()) > MAX_OUTBOUND_BYTES;
        if over_limit {
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "Python host-to-child queue is full",
            )
            .into());
        }
        state.outstanding_messages += 1;
        state.outstanding_bytes += bytes.len();
        state.queue.push_back(bytes);
        self.shared.ready.notify_one();
        Ok(())
    }

    fn dispatch(
        &self,
        node_id: String,
        event: String,
        action: Option<String>,
        payload: serde_json::Value,
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        let message = HostMessage::Event(UiEvent {
            id: "event-00000000000000000000".into(),
            node_id,
            event,
            action,
            payload,
            sequence: u64::MAX,
        });
        let mut bytes = serde_json::to_vec(&message)?;
        bytes.push(b'\n');

        let mut state = match self.shared.state.try_lock() {
            Ok(state) => state,
            Err(TryLockError::WouldBlock) => {
                request_overflow_recovery(&self.shared);
                return Err(io::Error::new(
                    io::ErrorKind::WouldBlock,
                    "Python event queue is busy; cancellation recovery scheduled",
                )
                .into());
            }
            Err(TryLockError::Poisoned(_)) => {
                return Err("python writer queue lock poisoned".into());
            }
        };
        apply_requested_close(&self.shared, &mut state);
        apply_requested_overflow(&self.shared, &mut state);
        if state.closed {
            return Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                state
                    .last_error
                    .as_deref()
                    .unwrap_or("Python writer is closed"),
            )
            .into());
        }
        if state.close_requested {
            return Err(
                io::Error::new(io::ErrorKind::BrokenPipe, "Python writer is closing").into(),
            );
        }
        if state.recovery_pending || state.recovering {
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "Python event stream is recovering after overflow",
            )
            .into());
        }
        let sequence = self
            .shared
            .event_sequence
            .load(Ordering::Relaxed)
            .checked_add(1)
            .ok_or("Python event sequence exhausted")?;
        let sequence_text = sequence.to_string();
        let sequence_digits = sequence_text.as_bytes();
        let actual_size = bytes
            .len()
            .saturating_sub(u64::MAX.to_string().len())
            .saturating_add(sequence_digits.len());
        if actual_size > DEFAULT_MAX_SESSION_MESSAGE_BYTES {
            return Err("host session message exceeds maximum size".into());
        }
        let over_limit = state.outstanding_messages >= MAX_OUTBOUND_MESSAGES
            || state.outstanding_bytes.saturating_add(actual_size) > MAX_OUTBOUND_BYTES;
        if over_limit {
            state.recovery_pending = true;
            self.shared.ready.notify_one();
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "Python event queue overflow; cancellation recovery scheduled",
            )
            .into());
        }
        patch_event_identity(&mut bytes, sequence, sequence_digits)?;
        self.shared
            .event_sequence
            .store(sequence, Ordering::Relaxed);
        state.outstanding_messages += 1;
        state.outstanding_bytes += bytes.len();
        state.queue.push_back(bytes);
        self.shared.ready.notify_one();
        Ok(format!("event-{sequence:020}"))
    }

    fn close(&self) {
        self.shared.close_requested.store(true, Ordering::Release);
        self.shared.ready.notify_one();
    }
}

fn request_overflow_recovery(shared: &OutboundWriterShared) {
    if shared.close_requested.load(Ordering::Acquire) {
        return;
    }
    shared.overflow_requested.store(true, Ordering::Release);
    shared.ready.notify_one();
}

fn apply_requested_close(shared: &OutboundWriterShared, state: &mut OutboundWriterState) {
    if shared.close_requested.swap(false, Ordering::AcqRel) {
        state.close_requested = true;
        shared.ready.notify_one();
    }
}

fn apply_requested_overflow(shared: &OutboundWriterShared, state: &mut OutboundWriterState) {
    if shared.overflow_requested.swap(false, Ordering::AcqRel)
        && !state.closed
        && !state.close_requested
    {
        state.recovery_pending = true;
        shared.ready.notify_one();
    }
}

fn patch_event_identity(
    bytes: &mut Vec<u8>,
    sequence: u64,
    sequence_digits: &[u8],
) -> io::Result<()> {
    const ID_PLACEHOLDER: &[u8] = b"event-00000000000000000000";
    const SEQUENCE_PLACEHOLDER: &[u8] = b"18446744073709551615";
    const SEQUENCE_SUFFIX_LEN: usize = SEQUENCE_PLACEHOLDER.len() + 2;

    let id_start = bytes
        .windows(ID_PLACEHOLDER.len())
        .position(|window| window == ID_PLACEHOLDER)
        .ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "event id placeholder missing")
        })?;
    let id_digits = format!("{sequence:020}");
    bytes[id_start + b"event-".len()..id_start + ID_PLACEHOLDER.len()]
        .copy_from_slice(id_digits.as_bytes());

    let sequence_start = bytes
        .len()
        .checked_sub(SEQUENCE_SUFFIX_LEN)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "event sequence placeholder missing",
            )
        })?;
    if &bytes[sequence_start..sequence_start + SEQUENCE_PLACEHOLDER.len()] != SEQUENCE_PLACEHOLDER
        || &bytes[sequence_start + SEQUENCE_PLACEHOLDER.len()..] != b"}\n"
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "event sequence must be the final serialized field",
        ));
    }
    bytes.splice(
        sequence_start..sequence_start + SEQUENCE_PLACEHOLDER.len(),
        sequence_digits.iter().copied(),
    );
    Ok(())
}

fn run_outbound_writer(mut writer: impl Write, shared: Arc<OutboundWriterShared>) {
    loop {
        let next = {
            let mut state = match shared.state.lock() {
                Ok(state) => state,
                Err(_) => return,
            };
            loop {
                apply_requested_close(&shared, &mut state);
                apply_requested_overflow(&shared, &mut state);
                if let Some(bytes) = state.queue.pop_front() {
                    break Some(bytes);
                }
                if state.recovery_pending && !state.recovering {
                    state.recovery_pending = false;
                    state.recovering = true;
                    break None;
                }
                if state.close_requested {
                    state.closed = true;
                    shared.ready.notify_all();
                    return;
                }
                state = match shared.ready.wait_timeout(state, Duration::from_millis(10)) {
                    Ok((state, _timeout)) => state,
                    Err(_) => return,
                };
            }
        };

        let (bytes, is_recovery) = match next {
            Some(bytes) => (bytes, false),
            None => {
                let sequence = shared.event_sequence.fetch_add(1, Ordering::Relaxed) + 1;
                let recovery = HostMessage::Event(UiEvent {
                    id: format!("event-{sequence:020}"),
                    sequence,
                    node_id: "__host__".into(),
                    event: "scene2d.cancel_all".into(),
                    action: Some("scene2d_cancel_all".into()),
                    payload: serde_json::json!({
                        "type": "scene2d.cancel_all",
                        "reason": "outbound_overflow",
                    }),
                });
                let mut bytes = match serde_json::to_vec(&recovery) {
                    Ok(bytes) => bytes,
                    Err(error) => {
                        fail_outbound_writer(&shared, error.to_string());
                        return;
                    }
                };
                bytes.push(b'\n');
                (bytes, true)
            }
        };

        let result = writer.write_all(&bytes).and_then(|()| writer.flush());
        let mut state = match shared.state.lock() {
            Ok(state) => state,
            Err(_) => return,
        };
        match result {
            Ok(()) if is_recovery => {
                state.recovering = false;
            }
            Ok(()) => {
                state.outstanding_messages = state.outstanding_messages.saturating_sub(1);
                state.outstanding_bytes = state.outstanding_bytes.saturating_sub(bytes.len());
            }
            Err(error) => {
                state.last_error = Some(error.to_string());
                state.closed = true;
                state.queue.clear();
                state.outstanding_messages = 0;
                state.outstanding_bytes = 0;
                shared.ready.notify_all();
                return;
            }
        }
        shared.ready.notify_all();
    }
}

fn fail_outbound_writer(shared: &OutboundWriterShared, error: String) {
    if let Ok(mut state) = shared.state.lock() {
        state.last_error = Some(error);
        state.closed = true;
        state.queue.clear();
        state.outstanding_messages = 0;
        state.outstanding_bytes = 0;
        shared.ready.notify_all();
    }
}

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
    outbound: PythonOutboundWriter,
    messages: Receiver<Result<PythonMessage, String>>,
    /// Messages emitted by `on_session_ready` can legally precede the initial
    /// snapshot (for example restored job updates). Keep them until the host
    /// entity has been constructed instead of treating ordering as fatal.
    pending: Arc<Mutex<VecDeque<PythonMessage>>>,
    pub stderr: Arc<Mutex<Vec<String>>>,
    wake: PythonSessionWake,
    /// Owns and crash-cleans all mmap publication files for this session.
    resource_directory: Arc<Mutex<Option<tempfile::TempDir>>>,
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
    outbound: PythonOutboundWriter,
}

impl PythonEventSink {
    pub fn send(&self, message: &HostMessage) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.outbound.send(message)
    }

    pub fn dispatch(
        &self,
        node_id: impl Into<String>,
        event: impl Into<String>,
        action: Option<String>,
        payload: serde_json::Value,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.outbound
            .dispatch(node_id.into(), event.into(), action, payload)
            .map(|_| ())
    }

    pub fn dispatch_with_id(
        &self,
        node_id: impl Into<String>,
        event: impl Into<String>,
        action: Option<String>,
        payload: serde_json::Value,
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        self.outbound
            .dispatch(node_id.into(), event.into(), action, payload)
    }

    #[cfg(test)]
    pub fn close(&self) {
        self.outbound.close();
    }
}

#[cfg(test)]
pub(super) fn test_event_sink(writer: impl Write + Send + 'static) -> PythonEventSink {
    PythonEventSink {
        outbound: PythonOutboundWriter::with_writer(writer),
    }
}

impl PythonSession {
    pub fn wake_handle(&self) -> PythonSessionWake {
        self.wake.clone()
    }

    /// Re-notifies the UI executor when a bounded frame budget leaves work in
    /// the inbound queue.
    pub fn reschedule_drain(&self) {
        self.wake.notify();
    }

    pub fn event_sink(&self) -> PythonEventSink {
        PythonEventSink {
            outbound: self.outbound.clone(),
        }
    }

    pub fn send(&self, message: &HostMessage) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.outbound.send(message)
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

    #[cfg(test)]
    pub fn shutdown(&self) {
        self.wake.notify();
        send_shutdown_when_available(&self.outbound);
        let child_exited = {
            let mut child = self
                .child
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let deadline = Instant::now() + shutdown_timeout();
            loop {
                match child.try_wait() {
                    Ok(Some(_)) => break true,
                    Ok(None) if Instant::now() < deadline => {
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    Ok(None) | Err(_) => {
                        let _ = child.kill();
                        break child.wait().is_ok() || child.try_wait().ok().flatten().is_some();
                    }
                }
            }
        };
        self.outbound.close();
        if child_exited {
            cleanup_resource_directory(&self.resource_directory);
        }
    }
}

impl Drop for PythonSession {
    fn drop(&mut self) {
        self.wake.notify();
        let child = self.child.clone();
        let outbound = self.outbound.clone();
        let resource_directory = self.resource_directory.clone();
        let timeout = shutdown_timeout();
        let _ = std::thread::Builder::new()
            .name("gpui-python-reaper".into())
            .spawn(move || {
                send_shutdown_when_available(&outbound);
                outbound.close();
                if let Ok(mut child) = child.lock() {
                    let deadline = Instant::now() + timeout;
                    while Instant::now() < deadline {
                        match child.try_wait() {
                            Ok(Some(_)) | Err(_) => break,
                            Ok(None) => std::thread::sleep(Duration::from_millis(10)),
                        }
                    }
                    if child.try_wait().ok().flatten().is_none() {
                        let _ = child.kill();
                        let _ = child.wait();
                    }
                }
                outbound.close();
                cleanup_resource_directory(&resource_directory);
            });
    }
}

fn cleanup_resource_directory(resource_directory: &Mutex<Option<tempfile::TempDir>>) {
    let mut resource_directory = resource_directory
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    drop(resource_directory.take());
}

fn shutdown_timeout() -> Duration {
    env::var("GPUI_TOOLKIT_SHUTDOWN_TIMEOUT_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .map(Duration::from_millis)
        .unwrap_or(Duration::from_secs(2))
}

fn send_shutdown_when_available(outbound: &PythonOutboundWriter) {
    let message = HostMessage::Shutdown(gpui_python_runtime::session::Shutdown {
        reason: "host_shutdown".into(),
    });
    let deadline = Instant::now() + Duration::from_millis(250);
    loop {
        match outbound.send(&message) {
            Ok(()) => break,
            Err(error)
                if error
                    .downcast_ref::<io::Error>()
                    .is_some_and(|error| error.kind() == io::ErrorKind::WouldBlock)
                    && Instant::now() < deadline =>
            {
                std::thread::sleep(Duration::from_millis(2));
            }
            Err(_) => break,
        }
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
    let outbound = PythonOutboundWriter::new(stdin);
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
        outbound,
        messages: rx,
        pending: Arc::new(Mutex::new(VecDeque::new())),
        stderr: stderr_lines,
        wake,
        resource_directory: Arc::new(Mutex::new(Some(resource_directory))),
    })
}

#[cfg(test)]
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
            Ok(0) => {
                let _ = tx.send(Err("Python process closed its session stream".into()));
                reader_wake.notify();
                return;
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::io::Cursor;

    struct BlockFirstWrite {
        output: Arc<Mutex<Vec<u8>>>,
        entered: SyncSender<()>,
        release: Receiver<()>,
        blocked: bool,
    }

    impl Write for BlockFirstWrite {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if !self.blocked {
                self.blocked = true;
                let _ = self.entered.send(());
                let _ = self.release.recv();
            }
            self.output
                .lock()
                .map_err(|_| io::Error::other("test output lock poisoned"))?
                .extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    struct CapturedWriter(Arc<Mutex<Vec<u8>>>);

    impl Write for CapturedWriter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.0
                .lock()
                .map_err(|_| io::Error::other("captured writer lock poisoned"))?
                .extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn outbound_writer_bounds_events_and_emits_ordered_cancel_recovery() {
        let output = Arc::new(Mutex::new(Vec::new()));
        let (entered_tx, entered_rx) = mpsc::sync_channel(1);
        let (release_tx, release_rx) = mpsc::sync_channel(1);
        let writer = PythonOutboundWriter::with_writer(BlockFirstWrite {
            output: output.clone(),
            entered: entered_tx,
            release: release_rx,
            blocked: false,
        });
        let sink = PythonEventSink {
            outbound: writer.clone(),
        };
        let dispatch = |index| {
            sink.dispatch(
                "surface",
                "scene2d.event",
                Some("scene2d_event".into()),
                serde_json::json!({"index": index}),
            )
        };

        dispatch(0).expect("first event enters bounded queue");
        entered_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("writer reached blocked pipe write");
        for index in 1..MAX_OUTBOUND_MESSAGES {
            dispatch(index).expect("event fits bounded queue");
        }
        assert!(dispatch(MAX_OUTBOUND_MESSAGES).is_err());
        release_tx.send(()).expect("release fake child pipe");

        let deadline = Instant::now() + Duration::from_secs(2);
        let lines = loop {
            let output = output.lock().expect("test output lock").clone();
            let line_count = output.iter().filter(|byte| **byte == b'\n').count();
            if line_count == MAX_OUTBOUND_MESSAGES + 1 {
                break output
                    .split(|byte| *byte == b'\n')
                    .filter(|line| !line.is_empty())
                    .map(|line| serde_json::from_slice::<Value>(line).expect("JSON line"))
                    .collect::<Vec<_>>();
            }
            assert!(
                Instant::now() < deadline,
                "writer did not finish queued events"
            );
            std::thread::yield_now();
        };
        assert_eq!(lines.len(), MAX_OUTBOUND_MESSAGES + 1);
        for (expected, line) in lines.iter().take(MAX_OUTBOUND_MESSAGES).enumerate() {
            assert_eq!(line["sequence"], expected as u64 + 1);
            assert_eq!(line["id"], format!("event-{:020}", expected + 1));
        }
        assert_eq!(
            lines.last().expect("overflow recovery")["event"],
            "scene2d.cancel_all"
        );
        assert_eq!(
            lines.last().expect("overflow recovery")["id"],
            format!(
                "event-{:020}",
                lines.last().expect("overflow recovery")["sequence"]
                    .as_u64()
                    .expect("recovery sequence")
            )
        );
        assert_eq!(
            lines.last().expect("overflow recovery")["payload"]["reason"],
            "outbound_overflow"
        );
        writer.close();
    }

    #[test]
    fn outbound_event_callback_never_waits_for_queue_lock_and_requests_recovery() {
        let output = Arc::new(Mutex::new(Vec::new()));
        let writer = PythonOutboundWriter::with_writer(CapturedWriter(output.clone()));
        let sink = PythonEventSink {
            outbound: writer.clone(),
        };
        let guard = writer
            .shared
            .state
            .lock()
            .expect("hold queue lock to simulate another producer");
        let (result_tx, result_rx) = mpsc::channel();
        std::thread::spawn(move || {
            let result = sink.dispatch(
                "board",
                "scene2d.event",
                Some("scene2d_event".into()),
                serde_json::json!({"type": "scene2d.event", "event": {"type": "pointer"}}),
            );
            let _ = result_tx.send(result.map_err(|error| error.to_string()));
        });
        let result = result_rx
            .recv_timeout(Duration::from_millis(100))
            .expect("event callback must return instead of waiting for the queue lock");
        assert!(
            result.is_err(),
            "contended input must be rejected explicitly"
        );
        drop(guard);

        let deadline = Instant::now() + Duration::from_secs(2);
        let recovery = loop {
            let bytes = output.lock().expect("captured output lock").clone();
            if let Some(end) = bytes.iter().position(|byte| *byte == b'\n') {
                break serde_json::from_slice::<Value>(&bytes[..end])
                    .expect("overflow cancellation event JSON");
            }
            assert!(
                Instant::now() < deadline,
                "contended input should wake the writer and emit cancellation recovery"
            );
            std::thread::yield_now();
        };
        assert_eq!(recovery["sequence"], 1);
        assert_eq!(recovery["event"], "scene2d.cancel_all");
        assert_eq!(recovery["payload"]["reason"], "outbound_overflow");
        writer.close();
    }

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
    fn dropping_python_session_hands_shutdown_to_background_reaper() {
        let script = env::temp_dir().join(format!(
            "gpui-toolkit-python-drop-{}.py",
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
import time

for line in sys.stdin:
    message = json.loads(line)
    if message.get("type") == "shutdown":
        time.sleep(0.3)
        pathlib.Path(__file__).with_suffix(".marker").write_text("shutdown", encoding="utf-8")
        break
"#,
        )
        .expect("write asynchronous shutdown probe script");

        let session = spawn_python_session_for_script(script.clone())
            .expect("spawn Python asynchronous shutdown probe");
        let resource_directory = session
            .resource_directory
            .lock()
            .expect("session resource directory lock")
            .as_ref()
            .expect("session owns the private resource directory")
            .path()
            .to_path_buf();
        let started = Instant::now();
        drop(session);
        assert!(
            started.elapsed() < Duration::from_millis(100),
            "dropping a Python session must not wait for the child process"
        );

        let deadline = Instant::now() + Duration::from_secs(3);
        while !marker.exists() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(
            std::fs::read_to_string(&marker).expect("read asynchronous shutdown marker"),
            "shutdown",
            "the background reaper should deliver shutdown and let the child exit"
        );
        while resource_directory.exists() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(
            !resource_directory.exists(),
            "the background reaper should remove private mmap files after child exit"
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
        let resource_directory = {
            let directory = session.resource_directory.lock().unwrap();
            directory.as_ref().unwrap().path().to_path_buf()
        };
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
        assert!(
            !resource_directory.exists(),
            "explicit shutdown removes private mmap files after the child exits"
        );
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
            Err(message) if message.contains("Python process closed its session stream")
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
