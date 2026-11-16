//! In-process `CPython` session transport for embedded hosts.
//!
//! Desktop hosts spawn `CPython` as a child process and exchange session-v1
//! messages over its stdio pipes. Mobile hosts cannot spawn processes, so
//! they embed `CPython` instead: [`spawn_embedded_session`] runs the Python
//! app's `App.serve` loop on a dedicated thread, connected to the host by
//! two OS pipes carrying the exact same wire protocol. The host reads
//! Python messages from [`EmbeddedSession::reader`] and writes host
//! messages with [`EmbeddedSession::send`], just as it would for a child
//! process.
//!
//! Both `build_app()` factories and module-level `app` instances are
//! accepted, mirroring the tutorial app contract. Python failures
//! print their traceback to the process stderr (OS log on mobile) and
//! close the pipes, which the host observes as end-of-stream — the same
//! failure shape as a dead desktop child process.

// Rust guideline compliant 2026-10-09

use std::io::{BufReader, PipeReader, Write};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread::JoinHandle;

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::session::HostMessage;

/// Distinguishes app modules in `sys.modules` across sessions in one process.
static NEXT_MODULE_ID: AtomicU64 = AtomicU64::new(0);

/// Failure modes for an embedded Python session.
#[derive(Debug, thiserror::Error)]
pub enum EmbeddedError {
    /// Pipe or script-path I/O failed on the Rust side.
    #[error("embedded Python session I/O failed: {0}")]
    Io(#[from] std::io::Error),
    /// The Python interpreter raised; the traceback was printed to stderr.
    #[error("embedded Python session failed: {0}")]
    Python(String),
    /// The Python thread panicked or could not be joined.
    #[error("embedded Python thread did not finish cleanly")]
    ThreadPanicked,
}

/// Owns the interpreter thread after the transport pipes change hands.
///
/// Returned by [`EmbeddedSession::into_transport`] so a host with its own
/// outbound writer queue can supervise interpreter exit itself.
pub struct EmbeddedThread {
    handle: Option<JoinHandle<Result<(), EmbeddedError>>>,
}

impl EmbeddedThread {
    /// Whether the interpreter thread has finished running the app.
    pub fn is_finished(&self) -> bool {
        self.handle.as_ref().is_none_or(JoinHandle::is_finished)
    }

    /// Waits for the interpreter thread and reports its session result.
    ///
    /// A detached thread cannot be killed, so hosts must close the input
    /// pipe (or send `shutdown`) before joining a session that may be idle.
    ///
    /// # Errors
    ///
    /// Returns the session's Python failure, or
    /// [`EmbeddedError::ThreadPanicked`] when the thread panicked.
    pub fn join(mut self) -> Result<(), EmbeddedError> {
        let Some(handle) = self.handle.take() else {
            return Ok(());
        };
        handle.join().map_err(|_| EmbeddedError::ThreadPanicked)?
    }

    /// Collects the session result without blocking when already finished.
    ///
    /// Returns `None` while the interpreter is still running; once finished
    /// the result is returned exactly once and later calls return `Ok(())`.
    pub fn try_join(&mut self) -> Option<Result<(), EmbeddedError>> {
        if !self.is_finished() {
            return None;
        }
        let handle = self.handle.take()?;
        Some(match handle.join() {
            Ok(result) => result,
            Err(_) => Err(EmbeddedError::ThreadPanicked),
        })
    }
}

/// Host endpoints of an in-process Python session.
pub struct EmbeddedSession {
    /// Python-to-host messages, one JSON value per line with binary frame
    /// payloads inline, exactly as on the desktop child-process stdout.
    pub reader: BufReader<PipeReader>,
    writer: Option<std::io::PipeWriter>,
    thread: Option<EmbeddedThread>,
}

impl EmbeddedSession {
    /// Sends one host message to the Python session.
    ///
    /// # Errors
    ///
    /// Returns [`EmbeddedError::Io`] when the pipe is closed or the message
    /// cannot be serialized.
    pub fn send(&mut self, message: &HostMessage) -> Result<(), EmbeddedError> {
        let Some(writer) = self.writer.as_mut() else {
            return Err(EmbeddedError::Io(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "embedded Python session writer is closed",
            )));
        };
        serde_json::to_writer(&mut *writer, message).map_err(|error| {
            EmbeddedError::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, error))
        })?;
        writer.write_all(b"\n")?;
        writer.flush()?;
        Ok(())
    }

    /// Sends `shutdown`, closes the pipe, and waits for the Python thread.
    ///
    /// # Errors
    ///
    /// Returns the session's Python or I/O failure when the app did not
    /// exit cleanly, and [`EmbeddedError::ThreadPanicked`] when the
    /// Python thread panicked.
    pub fn shutdown(mut self, reason: &str) -> Result<(), EmbeddedError> {
        let send_result = self.send(&HostMessage::Shutdown(crate::session::Shutdown {
            reason: reason.into(),
        }));
        self.writer = None;
        let Some(thread) = self.thread.take() else {
            return send_result;
        };
        send_result.and(thread.join())
    }

    /// Splits the session into raw transport parts for a host-owned
    /// reader/writer stack.
    ///
    /// The reader and writer are the same OS pipes the typed
    /// [`EmbeddedSession::send`] path uses. Closing the writer delivers
    /// end-of-stream to the Python loop; join the returned thread after
    /// shutdown (or pipe close) to collect the session result.
    pub fn into_transport(self) -> (BufReader<PipeReader>, std::io::PipeWriter, EmbeddedThread) {
        let Self {
            reader,
            writer,
            thread,
        } = self;
        let writer =
            writer.expect("embedded session writer is present until shutdown or into_transport");
        let thread =
            thread.expect("embedded session thread is present until shutdown or into_transport");
        (reader, writer, thread)
    }
}

// Dropping a session closes its write pipe (field drop), giving the Python
// loop end-of-stream like a desktop child losing stdin, and detaches the
// interpreter thread rather than joining it, so a stuck Python handler can
// never block the host's render thread. There is deliberately no `Drop`
// impl: field drop already provides exactly that behavior, and staying
// destructurable keeps `into_transport` free of placeholder pipes.

/// Starts a Python app in-process and returns its session endpoints.
///
/// `script` is the user's application module (defining `build_app()` or a
/// module-level `app`); its directory and every entry of `python_paths`
/// are placed on `sys.path`. The interpreter runs on a dedicated thread
/// and is initialized on first use for the whole process.
///
/// # Errors
///
/// Returns [`EmbeddedError::Io`] when `script` is not a file or the pipes
/// cannot be created. Python failures after spawn surface through
/// end-of-stream on [`EmbeddedSession::reader`] and from
/// [`EmbeddedSession::shutdown`].
pub fn spawn_embedded_session(
    script: &Path,
    python_paths: &[PathBuf],
) -> Result<EmbeddedSession, EmbeddedError> {
    if !script.is_file() {
        return Err(EmbeddedError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Python app script does not exist: {}", script.display()),
        )));
    }
    let (python_reader, host_writer) = std::io::pipe()?;
    let (host_reader, python_writer) = std::io::pipe()?;

    let script = script.to_path_buf();
    let mut sys_paths: Vec<PathBuf> = python_paths.to_vec();
    if let Some(parent) = script.parent() {
        sys_paths.push(parent.to_path_buf());
    }
    let module_name = format!(
        "gpui_user_app_{}",
        NEXT_MODULE_ID.fetch_add(1, Ordering::Relaxed)
    );
    let thread = std::thread::Builder::new()
        .name("gpui-python-embedded".into())
        .spawn(move || {
            run_python_app(script, module_name, sys_paths, python_reader, python_writer)
        })?;

    Ok(EmbeddedSession {
        reader: BufReader::new(host_reader),
        writer: Some(host_writer),
        thread: Some(EmbeddedThread {
            handle: Some(thread),
        }),
    })
}

fn run_python_app(
    script: PathBuf,
    module_name: String,
    sys_paths: Vec<PathBuf>,
    python_reader: std::io::PipeReader,
    python_writer: std::io::PipeWriter,
) -> Result<(), EmbeddedError> {
    let input_fd = python_reader.as_raw_fd();
    let output_fd = python_writer.as_raw_fd();
    let result = Python::attach(|py| {
        run_with_interpreter(py, &script, &module_name, &sys_paths, input_fd, output_fd).map_err(
            |error| {
                error.print(py);
                EmbeddedError::Python(error.to_string())
            },
        )
    });
    // The Python stream wrappers borrow these descriptors with
    // closefd=False; dropping the Rust ends here, after the interpreter
    // closure released every Python object, is what closes them.
    drop(python_reader);
    drop(python_writer);
    result
}

fn run_with_interpreter(
    py: Python<'_>,
    script: &Path,
    module_name: &str,
    sys_paths: &[PathBuf],
    input_fd: i32,
    output_fd: i32,
) -> PyResult<()> {
    let sys = py.import("sys")?;
    let path = sys.getattr("path")?;
    for entry in sys_paths.iter().rev() {
        path.call_method1("insert", (0, entry.to_string_lossy().as_ref()))?;
    }

    let module = load_app_module(py, script, module_name)?;
    let app = find_app(&module)?;

    let io_module = py.import("io")?;
    let input_stream = buffered_file(&io_module, input_fd, "rb", "BufferedReader")?;
    let output_stream = buffered_file(&io_module, output_fd, "wb", "BufferedWriter")?;

    let kwargs = PyDict::new(py);
    kwargs.set_item("input_stream", &input_stream)?;
    kwargs.set_item("output_stream", &output_stream)?;
    let serve_result = app.call_method("serve", (), Some(&kwargs)).map(|_| ());
    // Flush and release the wrappers before the Rust pipe ends are dropped;
    // failures here must not mask the session's own result.
    let _ = output_stream.call_method0("close");
    let _ = input_stream.call_method0("close");
    serve_result
}

fn load_app_module<'py>(
    py: Python<'py>,
    script: &Path,
    module_name: &str,
) -> PyResult<Bound<'py, PyAny>> {
    let util = py.import("importlib.util")?;
    let spec = util.call_method1(
        "spec_from_file_location",
        (module_name, script.to_string_lossy().as_ref()),
    )?;
    if spec.is_none() {
        return Err(PyRuntimeError::new_err(format!(
            "cannot load Python app module from {}",
            script.display()
        )));
    }
    let module = util.call_method1("module_from_spec", (&spec,))?;
    // Dataclasses resolve their defining module through sys.modules, so
    // register before executing, exactly like a real import would.
    py.import("sys")?
        .getattr("modules")?
        .set_item(module_name, &module)?;
    spec.getattr("loader")?
        .call_method1("exec_module", (&module,))?;
    Ok(module)
}

fn find_app<'py>(module: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    if let Ok(factory) = module.getattr("build_app") {
        return factory.call0();
    }
    if let Ok(app) = module.getattr("app") {
        return Ok(app);
    }
    Err(PyRuntimeError::new_err(
        "Python app module defines neither build_app() nor app",
    ))
}

fn buffered_file<'py>(
    io_module: &Bound<'py, PyAny>,
    fd: i32,
    mode: &str,
    wrapper: &str,
) -> PyResult<Bound<'py, PyAny>> {
    let kwargs = PyDict::new(io_module.py());
    kwargs.set_item("closefd", false)?;
    let raw = io_module.call_method("FileIO", (fd, mode), Some(&kwargs))?;
    io_module.call_method1(wrapper, (raw,))
}
