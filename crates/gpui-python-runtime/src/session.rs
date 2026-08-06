//! Versioned live-session protocol for Python-authored GPUI applications.
//!
//! The UI IR describes a snapshot; this module describes the independent,
//! newline-delimited JSON control plane used after that snapshot is rendered.

use crate::ui_ir::PythonAppIr;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, VecDeque};
use thiserror::Error;

pub const PYTHON_APP_SESSION_VERSION: u32 = 1;
pub const DEFAULT_MAX_SESSION_MESSAGE_BYTES: usize = 4 * 1024 * 1024;
/// Native capabilities advertised during initialization. Keep these symbolic
/// rather than tying applications to a particular host build or renderer.
pub const DEFAULT_HOST_CAPABILITIES: &[&str] = &[
    "events",
    "patches",
    "jobs",
    "effects",
    "forms",
    "tables",
    "charts",
    "scene3d",
    "state_store",
];

#[derive(Debug, Clone, PartialEq, Error)]
pub enum SessionError {
    #[error("unsupported python_app_session version {received}; supported version is {supported}")]
    UnsupportedVersion { received: u32, supported: u32 },
    #[error("session message id is empty")]
    EmptyId,
    #[error("patch revision {received} is stale; current revision is {current}")]
    StaleRevision { received: u64, current: u64 },
    #[error("session message exceeds {limit} bytes")]
    MessageTooLarge { limit: usize },
    #[error("malformed session message: {message}")]
    MalformedMessage { message: String },
    #[error("session peer advertised unsupported capabilities: {capabilities:?}")]
    UnsupportedCapabilities { capabilities: Vec<String> },
    #[error("invalid job transition from {from:?} to {to:?}")]
    InvalidJobTransition { from: JobState, to: JobState },
    #[error("unknown job {id:?}")]
    UnknownJob { id: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Initialize {
    pub session_version: u32,
    #[serde(default)]
    pub capabilities: Vec<String>,
    pub platform: String,
    pub theme: String,
    pub window: WindowMetadata,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowMetadata {
    pub width: f32,
    pub height: f32,
    #[serde(default)]
    pub scale_factor: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiEvent {
    pub id: String,
    pub sequence: u64,
    pub node_id: String,
    pub event: String,
    #[serde(default)]
    pub action: Option<String>,
    #[serde(default)]
    pub payload: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Shutdown {
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HostMessage {
    Initialize(Initialize),
    Event(UiEvent),
    Cancel { request_id: String },
    Shutdown(Shutdown),
    Heartbeat { id: String },
    EffectResult { request_id: String, result: Value },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionReady {
    pub session_version: u32,
    #[serde(default)]
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Patch {
    pub revision: u64,
    /// Correlates a response patch with the host event that produced it. When
    /// that event is superseded, the host keeps revision ordering but drops the
    /// mutation so a late handler cannot overwrite newer UI state.
    #[serde(default)]
    pub request_id: Option<String>,
    #[serde(default)]
    pub ops: Vec<PatchOp>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum PatchOp {
    Set {
        id: String,
        property: String,
        value: Value,
    },
    Insert {
        parent_id: String,
        index: usize,
        node: Value,
    },
    Remove {
        id: String,
    },
    Replace {
        id: String,
        node: Value,
    },
    Reorder {
        parent_id: String,
        child_ids: Vec<String>,
    },
    /// Replace one named Cartesian series without replacing the containing
    /// chart node or its retained interaction state.
    ReplaceChartSeries {
        chart_id: String,
        series: Value,
    },
    /// Append samples to an existing named Cartesian series.
    AppendChartSeries {
        chart_id: String,
        series_id: String,
        x: Vec<f64>,
        y: Vec<f64>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    Queued,
    Running,
    Cancelling,
    Cancelled,
    Succeeded,
    Failed,
}

impl JobState {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Cancelled | Self::Succeeded | Self::Failed)
    }
    fn permits(self, next: Self) -> bool {
        // Progress-bearing updates commonly repeat `running` (and transport
        // retries may repeat a terminal record). State transitions therefore
        // need idempotent same-state acceptance.
        if self == next {
            return true;
        }
        matches!(
            (self, next),
            (
                Self::Queued,
                Self::Running | Self::Cancelling | Self::Cancelled | Self::Failed
            ) | (
                Self::Running,
                Self::Cancelling | Self::Succeeded | Self::Failed
            ) | (Self::Cancelling, Self::Cancelled | Self::Failed)
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogSeverity {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JobLogLine {
    pub severity: LogSeverity,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JobRecord {
    pub id: String,
    pub state: JobState,
    pub completed: Option<u64>,
    pub total: Option<u64>,
    pub message: Option<String>,
    #[serde(skip)]
    logs: VecDeque<JobLogLine>,
}

impl JobRecord {
    pub fn logs(&self) -> impl DoubleEndedIterator<Item = &JobLogLine> + ExactSizeIterator {
        self.logs.iter()
    }
}

#[derive(Debug, Clone)]
pub struct JobRegistry {
    jobs: HashMap<String, JobRecord>,
    max_logs_per_job: usize,
}

impl JobRegistry {
    pub fn new(max_logs_per_job: usize) -> Self {
        Self {
            jobs: HashMap::new(),
            max_logs_per_job,
        }
    }
    pub fn get(&self, id: &str) -> Option<&JobRecord> {
        self.jobs.get(id)
    }
    pub fn iter(&self) -> impl Iterator<Item = &JobRecord> {
        self.jobs.values()
    }

    /// Whether closing the application needs an explicit cancellation/confirmation policy.
    pub fn has_active_jobs(&self) -> bool {
        self.jobs.values().any(|job| !job.state.is_terminal())
    }
    pub fn update(&mut self, update: JobUpdate) -> Result<(), SessionError> {
        if update.id.trim().is_empty() {
            return Err(SessionError::EmptyId);
        }
        match self.jobs.get_mut(&update.id) {
            Some(record) => {
                if record.state != update.state && !record.state.permits(update.state) {
                    return Err(SessionError::InvalidJobTransition {
                        from: record.state,
                        to: update.state,
                    });
                }
                record.state = update.state;
                record.completed = update.completed.or(record.completed);
                record.total = update.total.or(record.total);
                record.message = update.message.or(record.message.take());
            }
            None => {
                self.jobs.insert(
                    update.id.clone(),
                    JobRecord {
                        id: update.id,
                        state: update.state,
                        completed: update.completed,
                        total: update.total,
                        message: update.message,
                        logs: VecDeque::new(),
                    },
                );
            }
        }
        Ok(())
    }
    pub fn append_log(&mut self, id: &str, line: JobLogLine) -> Result<(), SessionError> {
        let record = self
            .jobs
            .get_mut(id)
            .ok_or_else(|| SessionError::UnknownJob { id: id.into() })?;
        record.logs.push_back(line);
        while record.logs.len() > self.max_logs_per_job {
            record.logs.pop_front();
        }
        Ok(())
    }

    /// Clear retained output without changing the job's state or metadata.
    pub fn clear_logs(&mut self, id: &str) -> Result<(), SessionError> {
        let record = self
            .jobs
            .get_mut(id)
            .ok_or_else(|| SessionError::UnknownJob { id: id.into() })?;
        record.logs.clear();
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JobUpdate {
    pub id: String,
    pub state: JobState,
    #[serde(default)]
    pub completed: Option<u64>,
    #[serde(default)]
    pub total: Option<u64>,
    #[serde(default)]
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JobLog {
    pub id: String,
    pub line: JobLogLine,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProtocolError {
    #[serde(default)]
    pub request_id: Option<String>,
    pub code: String,
    pub message: String,
}

/// Explicit terminal handling for a host event. These messages make validation
/// and input coalescing observable without encoding callback behavior in action
/// strings. Patches remain revision-ordered, so a superseded event cannot
/// mutate newer UI state through an older patch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActionSuperseded {
    pub request_id: String,
    pub superseded_by: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PythonMessage {
    Ready(SessionReady),
    Snapshot {
        app_ir: PythonAppIr,
    },
    Patch(Patch),
    Job(JobUpdate),
    JobLog(JobLog),
    Effect {
        request_id: String,
        effect: String,
        #[serde(default)]
        arguments: Value,
    },
    Acknowledged { request_id: String },
    Rejected(ProtocolError),
    Superseded(ActionSuperseded),
    Error(ProtocolError),
    Heartbeat {
        id: String,
    },
}

/// Tracks negotiated version and rejects stale patch revisions before UI state
/// can be mutated by a late child-process result.
#[derive(Debug, Clone)]
pub struct SessionState {
    revision: u64,
    pub capabilities: Vec<String>,
}

impl SessionState {
    pub fn new(capabilities: Vec<String>) -> Self {
        Self {
            revision: 0,
            capabilities,
        }
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn validate_ready(&self, ready: &SessionReady) -> Result<(), SessionError> {
        if ready.session_version != PYTHON_APP_SESSION_VERSION {
            return Err(SessionError::UnsupportedVersion {
                received: ready.session_version,
                supported: PYTHON_APP_SESSION_VERSION,
            });
        }
        let unsupported = ready
            .capabilities
            .iter()
            .filter(|capability| !self.capabilities.contains(capability))
            .cloned()
            .collect::<Vec<_>>();
        if !unsupported.is_empty() {
            return Err(SessionError::UnsupportedCapabilities {
                capabilities: unsupported,
            });
        }
        Ok(())
    }
    pub fn apply_patch_revision(&mut self, patch: &Patch) -> Result<(), SessionError> {
        if patch.revision <= self.revision {
            return Err(SessionError::StaleRevision {
                received: patch.revision,
                current: self.revision,
            });
        }
        self.revision = patch.revision;
        Ok(())
    }
}

pub fn parse_python_message(line: &[u8], max_bytes: usize) -> Result<PythonMessage, SessionError> {
    if line.len() > max_bytes {
        return Err(SessionError::MessageTooLarge { limit: max_bytes });
    }
    serde_json::from_slice(line).map_err(|error| SessionError::MalformedMessage {
        message: error.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn session_versions_and_stale_patches_are_rejected() {
        let mut state = SessionState::new(vec!["patches".into()]);
        assert!(matches!(
            state.validate_ready(&SessionReady {
                session_version: 2,
                capabilities: vec![]
            }),
            Err(SessionError::UnsupportedVersion { .. })
        ));
        assert!(matches!(
            state.validate_ready(&SessionReady {
                session_version: PYTHON_APP_SESSION_VERSION,
                capabilities: vec!["unavailable".into()],
            }),
            Err(SessionError::UnsupportedCapabilities { .. })
        ));
        state
            .apply_patch_revision(&Patch {
                revision: 2,
                request_id: None,
                ops: vec![],
            })
            .unwrap();
        assert!(matches!(
            state.apply_patch_revision(&Patch {
                revision: 2,
                request_id: None,
                ops: vec![]
            }),
            Err(SessionError::StaleRevision { .. })
        ));
    }
    #[test]
    fn session_messages_round_trip() {
        let message = HostMessage::Event(UiEvent {
            id: "evt-1".into(),
            sequence: 4,
            node_id: "run".into(),
            event: "click".into(),
            action: Some("run".into()),
            payload: serde_json::json!({"safe": true}),
        });
        let text = serde_json::to_vec(&message).unwrap();
        assert_eq!(
            serde_json::from_slice::<HostMessage>(&text).unwrap(),
            message
        );

        let effect_result = HostMessage::EffectResult {
            request_id: "pick-file".into(),
            result: serde_json::json!({"ok": true, "cancelled": true}),
        };
        let text = serde_json::to_vec(&effect_result).unwrap();
        assert_eq!(
            serde_json::from_slice::<HostMessage>(&text).unwrap(),
            effect_result
        );
    }

    #[test]
    fn malformed_messages_are_reported_without_reusing_an_id_error() {
        assert!(matches!(
            parse_python_message(b"{not-json}", DEFAULT_MAX_SESSION_MESSAGE_BYTES),
            Err(SessionError::MalformedMessage { .. })
        ));
    }

    #[test]
    fn jobs_keep_terminal_outcomes_distinct_and_bound_logs() {
        let mut jobs = JobRegistry::new(2);
        jobs.update(JobUpdate {
            id: "solve".into(),
            state: JobState::Queued,
            completed: None,
            total: None,
            message: None,
        })
        .unwrap();
        jobs.update(JobUpdate {
            id: "solve".into(),
            state: JobState::Running,
            completed: Some(1),
            total: Some(3),
            message: None,
        })
        .unwrap();
        jobs.update(JobUpdate {
            id: "solve".into(),
            state: JobState::Running,
            completed: Some(2),
            total: Some(3),
            message: Some("progress update".into()),
        })
        .unwrap();
        assert_eq!(jobs.get("solve").unwrap().completed, Some(2));
        jobs.update(JobUpdate {
            id: "solve".into(),
            state: JobState::Cancelling,
            completed: None,
            total: None,
            message: None,
        })
        .unwrap();
        jobs.update(JobUpdate {
            id: "solve".into(),
            state: JobState::Cancelled,
            completed: None,
            total: None,
            message: None,
        })
        .unwrap();
        assert_eq!(jobs.get("solve").unwrap().state, JobState::Cancelled);
        assert!(matches!(
            jobs.update(JobUpdate {
                id: "solve".into(),
                state: JobState::Succeeded,
                completed: None,
                total: None,
                message: None
            }),
            Err(SessionError::InvalidJobTransition { .. })
        ));
        for message in ["one", "two", "three"] {
            jobs.append_log(
                "solve",
                JobLogLine {
                    severity: LogSeverity::Info,
                    message: message.into(),
                },
            )
            .unwrap();
        }
        assert_eq!(jobs.get("solve").unwrap().logs().len(), 2);
    }

    #[test]
    fn clearing_logs_preserves_job_state() {
        let mut jobs = JobRegistry::new(2);
        jobs.update(JobUpdate {
            id: "solve".into(),
            state: JobState::Running,
            completed: None,
            total: None,
            message: None,
        })
        .unwrap();
        jobs.append_log(
            "solve",
            JobLogLine {
                severity: LogSeverity::Info,
                message: "started".into(),
            },
        )
        .unwrap();
        jobs.clear_logs("solve").unwrap();
        assert_eq!(jobs.get("solve").unwrap().state, JobState::Running);
        assert_eq!(jobs.get("solve").unwrap().logs().len(), 0);
    }

    #[test]
    fn job_logs_round_trip_through_the_session_protocol() {
        let message = PythonMessage::JobLog(JobLog {
            id: "solve".into(),
            line: JobLogLine {
                severity: LogSeverity::Warn,
                message: "remote worker is slow".into(),
            },
        });
        let encoded = serde_json::to_vec(&message).unwrap();
        assert_eq!(
            serde_json::from_slice::<PythonMessage>(&encoded).unwrap(),
            message
        );
    }

    #[test]
    fn action_outcomes_round_trip_through_the_session_protocol() {
        let superseded = PythonMessage::Superseded(ActionSuperseded {
            request_id: "evt-1".into(),
            superseded_by: "evt-2".into(),
        });
        let encoded = serde_json::to_vec(&superseded).unwrap();
        assert_eq!(
            serde_json::from_slice::<PythonMessage>(&encoded).unwrap(),
            superseded
        );
        let rejected = PythonMessage::Rejected(ProtocolError {
            request_id: Some("evt-2".into()),
            code: "invalid_frequency".into(),
            message: "End frequency must exceed start frequency".into(),
        });
        let encoded = serde_json::to_vec(&rejected).unwrap();
        assert_eq!(
            serde_json::from_slice::<PythonMessage>(&encoded).unwrap(),
            rejected
        );
    }

    #[test]
    fn active_job_query_excludes_terminal_outcomes() {
        let mut jobs = JobRegistry::new(2);
        jobs.update(JobUpdate {
            id: "solve".into(),
            state: JobState::Running,
            completed: None,
            total: None,
            message: None,
        })
        .unwrap();
        assert!(jobs.has_active_jobs());
        jobs.update(JobUpdate {
            id: "solve".into(),
            state: JobState::Cancelling,
            completed: None,
            total: None,
            message: None,
        })
        .unwrap();
        jobs.update(JobUpdate {
            id: "solve".into(),
            state: JobState::Cancelled,
            completed: None,
            total: None,
            message: None,
        })
        .unwrap();
        assert!(!jobs.has_active_jobs());
    }
}
