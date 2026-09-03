//! Versioned live-session protocol for Python-authored GPUI applications.
//!
//! The UI IR describes a snapshot; this module describes the independent,
//! newline-delimited JSON control plane used after that snapshot is rendered.

use crate::audio_stream::AudioFrame;
use crate::dataset_frames::{DatasetFrame, MappedDatasetFrame};
use crate::mesh_frames::MeshFrame;
use crate::ui_ir::PythonAppIr;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet, VecDeque};
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
    "commands",
    "profiler_telemetry",
    "forms",
    "tables",
    "charts",
    "scene3d",
    "state_store",
    "audio_binary_frames",
    "meshplot",
    "mesh_binary_frames",
    "mesh_frame_ack",
    "datasets",
    "array_resources",
    "px_interactions",
    "px_static_export",
    "px_chart_results",
    "resource_frame_ack",
    "resource_mmap_frames",
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
    #[error(
        "mesh plot {plot_id:?} generation {received} is stale; current generation is {current} (patch {patch_id:?})"
    )]
    StaleMeshGeneration {
        plot_id: String,
        received: u64,
        current: u64,
        patch_id: Option<String>,
    },
    #[error(
        "resource {resource_id:?} generation {received} is stale; current generation is {current}"
    )]
    StaleResourceGeneration {
        resource_id: String,
        received: u64,
        current: u64,
    },
    #[error("mesh plot {plot_id:?} has invalid generation 0")]
    InvalidMeshGeneration { plot_id: String },
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
    Cancel {
        request_id: String,
    },
    Shutdown(Shutdown),
    Heartbeat {
        id: String,
    },
    EffectResult {
        request_id: String,
        result: Value,
    },
    CommandResult {
        request_id: String,
        result: Value,
    },
    /// Host acceptance or rejection of one binary dataset/ArrayData chunk.
    ResourceFrameResult {
        resource_id: String,
        generation: u64,
        sequence: u32,
        byte_length: usize,
        complete: bool,
        accepted: bool,
        #[serde(default)]
        error: Option<String>,
    },
    /// Host acceptance or rejection of one binary mesh-resource chunk.
    MeshFrameResult {
        resource_id: String,
        generation: u64,
        sequence: u32,
        byte_length: usize,
        complete: bool,
        accepted: bool,
        #[serde(default)]
        error: Option<String>,
    },
    /// A bounded-rate, host-owned allocation sample from a subscription.
    ProfilerSample {
        subscription_id: String,
        sequence: u64,
        sample: Value,
    },
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
    ReplaceMeshGeometry {
        plot_id: String,
        generation: u64,
        geometry: Value,
    },
    ReplaceMeshField {
        plot_id: String,
        generation: u64,
        field: Value,
    },
    SetMeshPlotProp {
        plot_id: String,
        generation: u64,
        property: String,
        value: Value,
    },
    SetMeshPlotSelection {
        plot_id: String,
        generation: u64,
        selection: Value,
    },
    ClearMeshPlotSelection {
        plot_id: String,
        generation: u64,
    },
    SetMeshPlotCamera {
        plot_id: String,
        generation: u64,
        camera: Value,
    },
    ResetMeshPlotCamera {
        plot_id: String,
        generation: u64,
    },
    SetMeshPlotViewport {
        plot_id: String,
        generation: u64,
        viewport: Value,
    },
    ResetMeshPlotViewport {
        plot_id: String,
        generation: u64,
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

/// Metadata-only announcement for a dataset or dense array. Payload bytes are
/// deliberately transported out-of-band; accepting a descriptor must never
/// make the control channel materialize user values.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResourceDescriptor {
    #[serde(default = "default_resource_descriptor_version")]
    pub schema_version: u32,
    pub resource_id: String,
    pub generation: u64,
    pub resource_kind: String,
    #[serde(default)]
    pub schema_fingerprint: Option<String>,
    #[serde(default)]
    pub schema: Vec<String>,
    #[serde(default)]
    pub column_types: HashMap<String, String>,
    #[serde(default)]
    pub shape: Vec<usize>,
    #[serde(default)]
    pub dtype: Option<String>,
    #[serde(default)]
    pub byte_length: Option<u64>,
}

impl ResourceDescriptor {
    pub fn validate(&self) -> Result<(), SessionError> {
        if self.schema_version != 2
            || self.resource_id.trim().is_empty()
            || self.generation == 0
            || self.resource_kind.trim().is_empty()
        {
            return Err(SessionError::EmptyId);
        }
        if self.shape.contains(&0) {
            return Err(SessionError::MalformedMessage {
                message: "resource shape dimensions must be positive".into(),
            });
        }
        if !self.schema.is_empty()
            && (self.schema.len() != self.column_types.len()
                || self
                    .schema
                    .iter()
                    .any(|field| field.trim().is_empty() || !self.column_types.contains_key(field)))
        {
            return Err(SessionError::MalformedMessage {
                message: "resource schema and column types disagree".into(),
            });
        }
        Ok(())
    }
}

fn default_resource_descriptor_version() -> u32 {
    2
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
    /// Header for a raw binary frame. The stdout reader fills `payload` from
    /// the exact byte count immediately following the header line.
    ResourceFrame(AudioFrame),
    DatasetFrame(DatasetFrame),
    /// Complete Dataset/ArrayData generation in a host-created session mmap.
    MappedDatasetFrame(MappedDatasetFrame),
    /// Header for a chunked mesh frame. The stdout reader fills the payload
    /// from the exact byte count immediately following the header line.
    MeshFrame(MeshFrame),
    ResourceDescriptor(ResourceDescriptor),
    DropResource {
        resource_id: String,
        generation: u64,
    },
    Effect {
        request_id: String,
        effect: String,
        #[serde(default)]
        arguments: Value,
    },
    Command {
        request_id: String,
        command: String,
        #[serde(default)]
        arguments: Value,
    },
    Acknowledged {
        request_id: String,
    },
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
    mesh_generations: HashMap<String, u64>,
    resource_generations: HashMap<String, u64>,
    resource_schema_fingerprints: HashMap<String, String>,
}

impl SessionState {
    pub fn new(capabilities: Vec<String>) -> Self {
        Self {
            revision: 0,
            capabilities,
            mesh_generations: HashMap::new(),
            resource_generations: HashMap::new(),
            resource_schema_fingerprints: HashMap::new(),
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
        let mut generations = HashMap::new();
        for op in &patch.ops {
            let (plot_id, generation) = match op {
                PatchOp::ReplaceMeshGeometry {
                    plot_id,
                    generation,
                    ..
                }
                | PatchOp::ReplaceMeshField {
                    plot_id,
                    generation,
                    ..
                }
                | PatchOp::SetMeshPlotProp {
                    plot_id,
                    generation,
                    ..
                }
                | PatchOp::SetMeshPlotSelection {
                    plot_id,
                    generation,
                    ..
                }
                | PatchOp::ClearMeshPlotSelection {
                    plot_id,
                    generation,
                }
                | PatchOp::SetMeshPlotCamera {
                    plot_id,
                    generation,
                    ..
                }
                | PatchOp::ResetMeshPlotCamera {
                    plot_id,
                    generation,
                }
                | PatchOp::SetMeshPlotViewport {
                    plot_id,
                    generation,
                    ..
                }
                | PatchOp::ResetMeshPlotViewport {
                    plot_id,
                    generation,
                } => (plot_id, *generation),
                _ => continue,
            };
            if generation == 0 {
                return Err(SessionError::InvalidMeshGeneration {
                    plot_id: plot_id.clone(),
                });
            }
            if let Some(current) = self.mesh_generations.get(plot_id).copied()
                && generation < current
            {
                return Err(SessionError::StaleMeshGeneration {
                    plot_id: plot_id.clone(),
                    received: generation,
                    current,
                    patch_id: patch.request_id.clone(),
                });
            }
            if let Some(current) = generations.get(plot_id).copied()
                && generation < current
            {
                return Err(SessionError::StaleMeshGeneration {
                    plot_id: plot_id.clone(),
                    received: generation,
                    current,
                    patch_id: patch.request_id.clone(),
                });
            }
            generations
                .entry(plot_id.clone())
                .and_modify(|current| *current = (*current).max(generation))
                .or_insert(generation);
        }
        self.revision = patch.revision;
        self.mesh_generations.extend(generations);
        Ok(())
    }

    /// Return the newest accepted resource generation for a mesh plot.
    #[must_use]
    pub fn mesh_generation(&self, plot_id: &str) -> Option<u64> {
        self.mesh_generations.get(plot_id).copied()
    }

    /// Accept a descriptor only when it is not older than the resource already
    /// known to this session. Equal generations are idempotent announcements.
    pub fn apply_resource_descriptor(
        &mut self,
        descriptor: &ResourceDescriptor,
    ) -> Result<(), SessionError> {
        descriptor.validate()?;
        if let Some(current) = self
            .resource_generations
            .get(&descriptor.resource_id)
            .copied()
            && descriptor.generation < current
        {
            return Err(SessionError::StaleResourceGeneration {
                resource_id: descriptor.resource_id.clone(),
                received: descriptor.generation,
                current,
            });
        }
        if self
            .resource_generations
            .get(&descriptor.resource_id)
            .is_some_and(|current| *current == descriptor.generation)
            && self
                .resource_schema_fingerprints
                .get(&descriptor.resource_id)
                .is_some_and(|current| {
                    descriptor.schema_fingerprint.as_deref() != Some(current.as_str())
                })
        {
            return Err(SessionError::MalformedMessage {
                message: format!(
                    "resource {:?} changed schema fingerprint within generation {}",
                    descriptor.resource_id, descriptor.generation
                ),
            });
        }
        self.resource_generations
            .entry(descriptor.resource_id.clone())
            .and_modify(|current| *current = (*current).max(descriptor.generation))
            .or_insert(descriptor.generation);
        match descriptor.schema_fingerprint.as_ref() {
            Some(fingerprint) => {
                self.resource_schema_fingerprints
                    .insert(descriptor.resource_id.clone(), fingerprint.clone());
            }
            None => {
                self.resource_schema_fingerprints
                    .remove(&descriptor.resource_id);
            }
        }
        Ok(())
    }

    #[must_use]
    pub fn resource_generation(&self, resource_id: &str) -> Option<u64> {
        self.resource_generations.get(resource_id).copied()
    }

    pub fn resource_schema_fingerprint(&self, resource_id: &str) -> Option<&str> {
        self.resource_schema_fingerprints
            .get(resource_id)
            .map(String::as_str)
    }

    /// Release a retained resource only for its currently announced generation.
    /// A delayed close must not tear down a newer publication sharing the ID.
    pub fn drop_resource(
        &mut self,
        resource_id: &str,
        generation: u64,
    ) -> Result<(), SessionError> {
        if let Some(current) = self.resource_generations.get(resource_id).copied()
            && generation < current
        {
            return Err(SessionError::StaleResourceGeneration {
                resource_id: resource_id.into(),
                received: generation,
                current,
            });
        }
        if self
            .resource_generations
            .get(resource_id)
            .is_some_and(|current| *current == generation)
        {
            self.resource_generations.remove(resource_id);
            self.resource_schema_fingerprints.remove(resource_id);
        }
        Ok(())
    }

    /// Reset revision and MeshPlot generation history when a new Python
    /// producer is installed. The negotiated capability set belongs to the
    /// host and remains valid across child-process restarts.
    pub fn reset_for_new_session(&mut self) {
        self.revision = 0;
        self.mesh_generations.clear();
        self.resource_generations.clear();
        self.resource_schema_fingerprints.clear();
    }

    /// Drop generation history for plots that are no longer present in the
    /// committed application snapshot. This is intentionally separate from a
    /// session reset so active plots keep rejecting late generations while a
    /// removed plot can be recreated with a fresh producer generation.
    pub fn retain_mesh_plot_generations(&mut self, live_plot_ids: &HashSet<String>) {
        self.mesh_generations
            .retain(|plot_id, _| live_plot_ids.contains(plot_id));
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

        let command_result = HostMessage::CommandResult {
            request_id: "host-capabilities".into(),
            result: serde_json::json!({"ok": true, "capabilities": ["commands"]}),
        };
        let text = serde_json::to_vec(&command_result).unwrap();
        assert_eq!(
            serde_json::from_slice::<HostMessage>(&text).unwrap(),
            command_result
        );

        let sample = HostMessage::ProfilerSample {
            subscription_id: "render".into(),
            sequence: 1,
            sample: serde_json::json!({"mode": "zero", "bytes": 0, "count": 0}),
        };
        let text = serde_json::to_vec(&sample).unwrap();
        assert_eq!(
            serde_json::from_slice::<HostMessage>(&text).unwrap(),
            sample
        );
    }

    #[test]
    fn mesh_plot_generations_reject_late_field_updates() {
        let mut state = SessionState::new(vec!["meshplot".into(), "patches".into()]);
        state
            .apply_patch_revision(&Patch {
                revision: 1,
                request_id: None,
                ops: vec![PatchOp::ReplaceMeshField {
                    plot_id: "plot".into(),
                    generation: 4,
                    field: serde_json::json!({"values": [1.0]}),
                }],
            })
            .unwrap();
        assert_eq!(state.mesh_generation("plot"), Some(4));
        assert!(matches!(
            state.apply_patch_revision(&Patch {
                revision: 2,
                request_id: None,
                ops: vec![PatchOp::ReplaceMeshField {
                    plot_id: "plot".into(),
                    generation: 3,
                    field: serde_json::json!({"values": [0.0]}),
                }],
            }),
            Err(SessionError::StaleMeshGeneration { .. })
        ));
        assert_eq!(
            state.revision(),
            1,
            "rejected generation must not consume revision"
        );
    }

    #[test]
    fn mesh_plot_generation_can_cover_multiple_same_frame_operations() {
        let mut state = SessionState::new(vec!["meshplot".into(), "patches".into()]);
        state
            .apply_patch_revision(&Patch {
                revision: 1,
                request_id: Some("mesh-update".into()),
                ops: vec![
                    PatchOp::ReplaceMeshField {
                        plot_id: "plot".into(),
                        generation: 2,
                        field: serde_json::json!({"values": [1.0]}),
                    },
                    PatchOp::SetMeshPlotCamera {
                        plot_id: "plot".into(),
                        generation: 2,
                        camera: serde_json::json!({"azimuth": 0.5}),
                    },
                ],
            })
            .unwrap();
        assert_eq!(state.mesh_generation("plot"), Some(2));
        assert!(matches!(
            state.apply_patch_revision(&Patch {
                revision: 2,
                request_id: None,
                ops: vec![PatchOp::SetMeshPlotViewport {
                    plot_id: "plot".into(),
                    generation: 1,
                    viewport: serde_json::json!({"x": [0.0, 1.0]}),
                }],
            }),
            Err(SessionError::StaleMeshGeneration { .. })
        ));
    }

    #[test]
    fn session_restart_clears_revision_and_mesh_generation_history() {
        let mut state = SessionState::new(vec!["meshplot".into(), "patches".into()]);
        state
            .apply_patch_revision(&Patch {
                revision: 7,
                request_id: Some("old-producer".into()),
                ops: vec![PatchOp::ReplaceMeshField {
                    plot_id: "plot".into(),
                    generation: 9,
                    field: serde_json::json!({"values": [1.0]}),
                }],
            })
            .unwrap();

        state.reset_for_new_session();

        assert_eq!(state.revision(), 0);
        assert_eq!(state.mesh_generation("plot"), None);
        state
            .apply_patch_revision(&Patch {
                revision: 1,
                request_id: Some("new-producer".into()),
                ops: vec![PatchOp::ReplaceMeshField {
                    plot_id: "plot".into(),
                    generation: 1,
                    field: serde_json::json!({"values": [2.0]}),
                }],
            })
            .expect("a restarted producer may begin at revision one again");
        assert_eq!(state.revision(), 1);
        assert_eq!(state.mesh_generation("plot"), Some(1));
    }

    #[test]
    fn removed_mesh_plots_release_only_their_generation_history() {
        let mut state = SessionState::new(vec!["meshplot".into(), "patches".into()]);
        for (plot_id, generation) in [("active", 4), ("removed", 8)] {
            state
                .apply_patch_revision(&Patch {
                    revision: generation,
                    request_id: None,
                    ops: vec![PatchOp::ReplaceMeshField {
                        plot_id: plot_id.into(),
                        generation,
                        field: serde_json::json!({"values": [1.0]}),
                    }],
                })
                .unwrap();
        }

        state.retain_mesh_plot_generations(&HashSet::from(["active".into()]));

        assert_eq!(state.mesh_generation("active"), Some(4));
        assert_eq!(state.mesh_generation("removed"), None);
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

    #[test]
    fn resource_descriptors_round_trip_without_values() {
        let message = PythonMessage::ResourceDescriptor(ResourceDescriptor {
            schema_version: 2,
            resource_id: "events".into(),
            generation: 3,
            resource_kind: "dataset".into(),
            schema_fingerprint: Some("schema-v1".into()),
            schema: vec!["frequency".into()],
            column_types: HashMap::from([("frequency".into(), "float64".into())]),
            shape: vec![],
            dtype: None,
            byte_length: None,
        });
        let encoded = serde_json::to_vec(&message).unwrap();
        assert!(!String::from_utf8_lossy(&encoded).contains("20.0"));
        assert_eq!(
            serde_json::from_slice::<PythonMessage>(&encoded).unwrap(),
            message
        );
    }

    #[test]
    fn resource_descriptor_retains_schema_fingerprint_by_generation() {
        let mut state = SessionState::new(vec![]);
        state
            .apply_resource_descriptor(&ResourceDescriptor {
                schema_version: 2,
                resource_id: "events".into(),
                generation: 4,
                resource_kind: "dataset".into(),
                schema_fingerprint: Some("events-v4".into()),
                schema: vec!["id".into()],
                column_types: HashMap::from([("id".into(), "int64".into())]),
                shape: vec![],
                dtype: None,
                byte_length: None,
            })
            .unwrap();
        assert_eq!(
            state.resource_schema_fingerprint("events"),
            Some("events-v4")
        );
        assert!(
            state
                .apply_resource_descriptor(&ResourceDescriptor {
                    schema_version: 2,
                    resource_id: "events".into(),
                    generation: 4,
                    resource_kind: "dataset".into(),
                    schema_fingerprint: Some("other-schema".into()),
                    schema: vec!["id".into()],
                    column_types: HashMap::from([("id".into(), "int64".into())]),
                    shape: vec![],
                    dtype: None,
                    byte_length: None,
                })
                .is_err()
        );
        state
            .apply_resource_descriptor(&ResourceDescriptor {
                schema_version: 2,
                resource_id: "events".into(),
                generation: 5,
                resource_kind: "array_data".into(),
                schema_fingerprint: None,
                schema: vec![],
                column_types: HashMap::new(),
                shape: vec![1],
                dtype: Some("f32".into()),
                byte_length: Some(4),
            })
            .unwrap();
        assert_eq!(state.resource_schema_fingerprint("events"), None);
        state.drop_resource("events", 5).unwrap();
        assert_eq!(state.resource_schema_fingerprint("events"), None);
    }

    #[test]
    fn resource_descriptor_rejects_empty_ids_and_zero_dimensions() {
        assert!(
            ResourceDescriptor {
                schema_version: 2,
                resource_id: String::new(),
                generation: 1,
                resource_kind: "dataset".into(),
                schema_fingerprint: None,
                shape: vec![],
                dtype: None,
                byte_length: None,
                schema: vec![],
                column_types: HashMap::new(),
            }
            .validate()
            .is_err()
        );
        assert!(
            ResourceDescriptor {
                schema_version: 2,
                resource_id: "image".into(),
                generation: 1,
                resource_kind: "array_data".into(),
                schema_fingerprint: None,
                shape: vec![32, 0],
                dtype: Some("u8".into()),
                byte_length: Some(32),
                schema: vec![],
                column_types: HashMap::new(),
            }
            .validate()
            .is_err()
        );
        assert!(
            ResourceDescriptor {
                schema_version: 1,
                resource_id: "legacy".into(),
                generation: 1,
                resource_kind: "dataset".into(),
                schema_fingerprint: None,
                schema: vec![],
                column_types: HashMap::new(),
                shape: vec![],
                dtype: None,
                byte_length: None,
            }
            .validate()
            .is_err()
        );
    }

    #[test]
    fn session_state_rejects_stale_resource_generations() {
        let mut state = SessionState::new(
            DEFAULT_HOST_CAPABILITIES
                .iter()
                .map(ToString::to_string)
                .collect(),
        );
        let current = ResourceDescriptor {
            schema_version: 2,
            resource_id: "events".into(),
            generation: 2,
            resource_kind: "dataset".into(),
            schema_fingerprint: Some("schema".into()),
            shape: vec![],
            dtype: None,
            byte_length: None,
            schema: vec![],
            column_types: HashMap::new(),
        };
        state.apply_resource_descriptor(&current).unwrap();
        assert_eq!(state.resource_generation("events"), Some(2));
        let stale = ResourceDescriptor {
            generation: 1,
            ..current
        };
        assert!(matches!(
            state.apply_resource_descriptor(&stale),
            Err(SessionError::StaleResourceGeneration { .. })
        ));
    }

    #[test]
    fn resource_drop_cannot_release_a_newer_generation() {
        let mut state = SessionState::new(vec![]);
        let descriptor = ResourceDescriptor {
            schema_version: 2,
            resource_id: "events".into(),
            generation: 2,
            resource_kind: "dataset".into(),
            schema_fingerprint: None,
            shape: vec![],
            dtype: None,
            byte_length: None,
            schema: vec![],
            column_types: HashMap::new(),
        };
        state.apply_resource_descriptor(&descriptor).unwrap();
        assert!(matches!(
            state.drop_resource("events", 1),
            Err(SessionError::StaleResourceGeneration { .. })
        ));
        state.drop_resource("events", 2).unwrap();
        assert_eq!(state.resource_generation("events"), None);
    }

    #[test]
    fn resource_frame_result_round_trips_with_chunk_identity() {
        let message = HostMessage::ResourceFrameResult {
            resource_id: "events".into(),
            generation: 7,
            sequence: 2,
            byte_length: 65_536,
            complete: false,
            accepted: false,
            error: Some("checksum mismatch".into()),
        };
        let encoded = serde_json::to_vec(&message).unwrap();
        assert_eq!(
            serde_json::from_slice::<HostMessage>(&encoded).unwrap(),
            message
        );
        let wire: Value = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(wire["type"], "resource_frame_result");
        assert_eq!(wire["resource_id"], "events");
        assert_eq!(wire["generation"], 7);
        assert_eq!(wire["sequence"], 2);
        assert_eq!(wire["byte_length"], 65_536);
        assert_eq!(wire["accepted"], false);
        assert_eq!(wire["error"], "checksum mismatch");
    }

    #[test]
    fn mesh_frame_result_round_trips_with_chunk_identity() {
        let message = HostMessage::MeshFrameResult {
            resource_id: "surface-field".into(),
            generation: 4,
            sequence: 1,
            byte_length: 32_768,
            complete: true,
            accepted: true,
            error: None,
        };
        let encoded = serde_json::to_vec(&message).unwrap();
        assert_eq!(
            serde_json::from_slice::<HostMessage>(&encoded).unwrap(),
            message
        );
        let wire: Value = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(wire["type"], "mesh_frame_result");
        assert_eq!(wire["resource_id"], "surface-field");
        assert_eq!(wire["generation"], 4);
        assert_eq!(wire["sequence"], 1);
        assert_eq!(wire["byte_length"], 32_768);
        assert_eq!(wire["complete"], true);
        assert_eq!(wire["accepted"], true);
        assert_eq!(wire["error"], Value::Null);
    }
}
