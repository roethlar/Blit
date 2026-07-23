//! Low-frequency, wire-neutral timing events for transfer-session probes.
//!
//! This is deliberately separate from `--trace-data-plane`, whose
//! per-file human-readable output is too intrusive for performance work.
//! Production emission is enabled with `BLIT_TRACE_SESSION_PHASES=1` and
//! correlated across processes with `BLIT_TRACE_RUN_ID`. Tests may inject
//! an in-memory emitter through [`SessionPhaseTrace::capture`].

use serde::Serialize;
use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

const TRACE_ENV: &str = "BLIT_TRACE_SESSION_PHASES";
const RUN_ID_ENV: &str = "BLIT_TRACE_RUN_ID";

/// Semantic role of this endpoint in the transfer. This is intentionally
/// independent of which endpoint initiated the connection.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SessionPhaseRole {
    Source,
    Destination,
}

/// One structured timing event. Optional correlation fields are omitted
/// from JSON rather than written as nulls so rig logs stay compact.
#[derive(Clone, Debug, Serialize)]
pub struct SessionPhaseEvent {
    pub schema: u8,
    pub run_id: String,
    pub session_id: String,
    pub producer_seq: u64,
    pub unix_ns: u128,
    pub elapsed_ns: u128,
    pub endpoint_role: SessionPhaseRole,
    pub initiator_role: SessionPhaseRole,
    pub event: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub epoch: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub socket: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_streams: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub live_streams: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accepted: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sample_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sample_blocked_ns: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sample_elapsed_ns: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sample_streams: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sample_valid: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked_ratio: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefetch_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tcp_buffer_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receiver_ceiling: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peak_streams: Option<u32>,
}

/// Optional event fields shared by the small set of phase hooks.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct SessionPhaseFields {
    pub(crate) action: Option<&'static str>,
    pub(crate) reason: Option<&'static str>,
    pub(crate) epoch: Option<u32>,
    pub(crate) socket: Option<u32>,
    pub(crate) batch: Option<u64>,
    pub(crate) count: Option<u64>,
    pub(crate) target_streams: Option<u32>,
    pub(crate) live_streams: Option<u32>,
    pub(crate) accepted: Option<bool>,
    pub(crate) sample_bytes: Option<u64>,
    pub(crate) sample_blocked_ns: Option<u64>,
    pub(crate) sample_elapsed_ns: Option<u64>,
    pub(crate) sample_streams: Option<u32>,
    pub(crate) sample_valid: Option<bool>,
    pub(crate) blocked_ratio: Option<f64>,
    pub(crate) chunk_bytes: Option<u64>,
    pub(crate) prefetch_count: Option<u32>,
    pub(crate) tcp_buffer_bytes: Option<u64>,
    pub(crate) receiver_ceiling: Option<u32>,
    pub(crate) peak_streams: Option<u32>,
}

type EventEmitter = dyn Fn(SessionPhaseEvent) + Send + Sync + 'static;
type FlushEmitter = dyn Fn() + Send + Sync + 'static;

enum PhaseWriterOutput {
    Line(String),
    Flush,
}

#[derive(Clone)]
struct TraceEmitter {
    run_id: Arc<str>,
    emit: Arc<EventEmitter>,
    flush: Arc<FlushEmitter>,
}

/// An unbound phase-event sink carried by source/destination instruments.
/// The default remains inactive unless the explicit process-level probe
/// environment flag is set.
#[derive(Clone)]
pub struct SessionPhaseTrace {
    emitter: Option<TraceEmitter>,
    allow_env: bool,
}

impl Default for SessionPhaseTrace {
    fn default() -> Self {
        Self {
            emitter: None,
            allow_env: true,
        }
    }
}

impl SessionPhaseTrace {
    /// Build a deterministic capture sink for integration tests and local
    /// diagnostic harnesses. This does not consult process environment.
    pub fn capture(
        run_id: impl Into<String>,
        emit: impl Fn(SessionPhaseEvent) + Send + Sync + 'static,
    ) -> Self {
        Self {
            emitter: Some(TraceEmitter {
                run_id: Arc::from(run_id.into()),
                emit: Arc::new(emit),
                flush: Arc::new(|| {}),
            }),
            allow_env: false,
        }
    }

    /// Force tracing off even when the process-level probe flag is set.
    /// Used by the trace-on/off behavior guard.
    pub fn disabled() -> Self {
        Self {
            emitter: None,
            allow_env: false,
        }
    }

    /// Preserve an injected sink; otherwise enable the low-frequency JSONL
    /// emitter when the debug environment flag is truthy.
    pub(crate) fn or_from_env(self) -> Self {
        self.or_from_env_with(|name| std::env::var(name).ok(), Self::stderr_writer)
    }

    fn or_from_env_with(
        self,
        read: impl FnMut(&str) -> Option<String>,
        writer: impl FnOnce(String) -> Self,
    ) -> Self {
        if self.emitter.is_some() || !self.allow_env {
            return self;
        }
        let Some(run_id) = trace_env_run_id(read) else {
            return self;
        };
        writer(run_id)
    }

    fn stderr_writer(run_id: String) -> Self {
        let stderr = std::io::stderr();
        Self::threaded_writer(run_id, move |output| {
            let mut stderr = stderr.lock();
            match output {
                PhaseWriterOutput::Line(line) => {
                    let _ = writeln!(stderr, "{line}");
                }
                PhaseWriterOutput::Flush => {
                    let _ = stderr.flush();
                }
            }
        })
    }

    fn threaded_writer(
        run_id: String,
        mut output: impl FnMut(PhaseWriterOutput) + Send + 'static,
    ) -> Self {
        enum WriterMessage {
            Event(Box<SessionPhaseEvent>),
            Flush(std::sync::mpsc::SyncSender<()>),
        }

        let (tx, rx) = std::sync::mpsc::channel::<WriterMessage>();
        let spawned = std::thread::Builder::new()
            .name("blit-session-phase".into())
            .spawn(move || {
                while let Ok(message) = rx.recv() {
                    match message {
                        WriterMessage::Event(event) => {
                            output(PhaseWriterOutput::Line(json_line(&event)));
                        }
                        WriterMessage::Flush(done) => {
                            output(PhaseWriterOutput::Flush);
                            let _ = done.send(());
                        }
                    }
                }
            });
        if spawned.is_err() {
            return Self::disabled();
        }
        let event_tx = tx.clone();
        let flush_tx = tx;
        Self {
            emitter: Some(TraceEmitter {
                run_id: Arc::from(run_id),
                emit: Arc::new(move |event| {
                    let _ = event_tx.send(WriterMessage::Event(Box::new(event)));
                }),
                flush: Arc::new(move || {
                    let (done_tx, done_rx) = std::sync::mpsc::sync_channel(0);
                    if flush_tx.send(WriterMessage::Flush(done_tx)).is_ok() {
                        let _ = done_rx.recv();
                    }
                }),
            }),
            allow_env: false,
        }
    }

    pub(crate) fn bind(
        &self,
        session_token: &[u8],
        endpoint_role: SessionPhaseRole,
        initiator_role: SessionPhaseRole,
    ) -> Option<BoundSessionPhaseTrace> {
        let emitter = self.emitter.clone()?;
        Some(BoundSessionPhaseTrace {
            emitter,
            session_id: session_trace_id(session_token),
            endpoint_role,
            initiator_role,
            origin: Instant::now(),
            producer_seq: Arc::new(AtomicU64::new(0)),
        })
    }
}

pub(super) fn trace_env_run_id(mut read: impl FnMut(&str) -> Option<String>) -> Option<String> {
    let enabled = read(TRACE_ENV).is_some_and(|value| {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    });
    enabled.then(|| read(RUN_ID_ENV).unwrap_or_else(|| format!("pid-{}", std::process::id())))
}

pub(super) fn session_trace_id(session_token: &[u8]) -> Arc<str> {
    let digest = blake3::hash(session_token).to_hex();
    Arc::from(&digest.as_str()[..16])
}

fn json_line(event: &SessionPhaseEvent) -> String {
    match serde_json::to_string(event) {
        Ok(line) => format!("[session-phase] {line}"),
        Err(err) => format!("[session-phase] serialization_error={err}"),
    }
}

/// A phase trace bound to one endpoint of one TCP transfer session.
#[derive(Clone)]
pub(crate) struct BoundSessionPhaseTrace {
    emitter: TraceEmitter,
    session_id: Arc<str>,
    endpoint_role: SessionPhaseRole,
    initiator_role: SessionPhaseRole,
    origin: Instant,
    producer_seq: Arc<AtomicU64>,
}

pub(crate) struct SessionPhaseStamp {
    instant: Instant,
    unix_ns: u128,
    producer_seq: u64,
}

impl BoundSessionPhaseTrace {
    pub(crate) fn event(&self, event: &'static str, fields: SessionPhaseFields) {
        self.emit_at(self.stamp(), event, fields);
    }

    pub(crate) fn stamp(&self) -> SessionPhaseStamp {
        SessionPhaseStamp {
            instant: Instant::now(),
            unix_ns: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos(),
            producer_seq: self.producer_seq.fetch_add(1, Ordering::Relaxed),
        }
    }

    /// Capture the queue-admission time before releasing the payload to a
    /// worker, then emit it after the send commits. This preserves the
    /// causal timestamp without putting logging on the queue's critical
    /// path before the payload exists.
    pub(crate) fn first_payload_queued_at(&self, at: SessionPhaseStamp) {
        self.emit_at(at, "first_payload_queued", SessionPhaseFields::default());
    }

    pub(crate) fn socket_first_write(&self, epoch: u32, socket: u32) {
        self.event(
            "first_socket_write",
            SessionPhaseFields {
                epoch: Some(epoch),
                socket: Some(socket),
                ..Default::default()
            },
        );
    }

    pub(crate) fn socket_first_payload_received(&self, epoch: u32, socket: u32) {
        self.event(
            "first_payload_received",
            SessionPhaseFields {
                epoch: Some(epoch),
                socket: Some(socket),
                ..Default::default()
            },
        );
    }

    pub(crate) fn flush(&self) {
        (self.emitter.flush)();
    }

    fn emit_at(&self, at: SessionPhaseStamp, event: &'static str, fields: SessionPhaseFields) {
        let record = SessionPhaseEvent {
            schema: 1,
            run_id: self.emitter.run_id.to_string(),
            session_id: self.session_id.to_string(),
            producer_seq: at.producer_seq,
            unix_ns: at.unix_ns,
            elapsed_ns: at.instant.saturating_duration_since(self.origin).as_nanos(),
            endpoint_role: self.endpoint_role,
            initiator_role: self.initiator_role,
            event,
            action: fields.action,
            reason: fields.reason,
            epoch: fields.epoch,
            socket: fields.socket,
            batch: fields.batch,
            count: fields.count,
            target_streams: fields.target_streams,
            live_streams: fields.live_streams,
            accepted: fields.accepted,
            sample_bytes: fields.sample_bytes,
            sample_blocked_ns: fields.sample_blocked_ns,
            sample_elapsed_ns: fields.sample_elapsed_ns,
            sample_streams: fields.sample_streams,
            sample_valid: fields.sample_valid,
            blocked_ratio: fields.blocked_ratio,
            chunk_bytes: fields.chunk_bytes,
            prefetch_count: fields.prefetch_count,
            tcp_buffer_bytes: fields.tcp_buffer_bytes,
            receiver_ceiling: fields.receiver_ceiling,
            peak_streams: fields.peak_streams,
        };
        (self.emitter.emit)(record);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
    use std::sync::{Arc, Mutex};

    #[test]
    fn production_env_config_uses_the_canonical_flag_and_run_id() {
        let mut requested = Vec::new();
        let run_id = trace_env_run_id(|name| {
            requested.push(name.to_string());
            match name {
                TRACE_ENV => Some("true".into()),
                RUN_ID_ENV => Some("rig-w-pair-7".into()),
                _ => None,
            }
        });
        assert_eq!(run_id.as_deref(), Some("rig-w-pair-7"));
        assert_eq!(
            requested,
            vec![TRACE_ENV.to_string(), RUN_ID_ENV.to_string()]
        );

        let mut requested = Vec::new();
        assert_eq!(
            trace_env_run_id(|name| {
                requested.push(name.to_string());
                Some("off".into())
            }),
            None
        );
        assert_eq!(requested, vec![TRACE_ENV.to_string()]);
        assert!(SessionPhaseTrace::default().allow_env);
        assert!(!SessionPhaseTrace::disabled().allow_env);
    }

    #[test]
    fn production_writer_emits_safe_prefixed_json_and_flushes() {
        let lines: Arc<Mutex<Vec<String>>> = Arc::default();
        let flushes = Arc::new(AtomicUsize::new(0));
        let writer_lines = Arc::clone(&lines);
        let writer_flushes = Arc::clone(&flushes);
        let trace = SessionPhaseTrace::default().or_from_env_with(
            |name| match name {
                TRACE_ENV => Some("1".into()),
                RUN_ID_ENV => Some("unit-run".into()),
                _ => None,
            },
            move |run_id| {
                SessionPhaseTrace::threaded_writer(run_id, move |output| match output {
                    PhaseWriterOutput::Line(line) => writer_lines.lock().unwrap().push(line),
                    PhaseWriterOutput::Flush => {
                        writer_flushes.fetch_add(1, AtomicOrdering::Relaxed);
                    }
                })
            },
        );
        let bound = trace
            .bind(
                b"never-log-this-session-token",
                SessionPhaseRole::Source,
                SessionPhaseRole::Destination,
            )
            .unwrap();
        bound.event("probe", SessionPhaseFields::default());
        bound.flush();
        assert_eq!(flushes.load(AtomicOrdering::Relaxed), 1);
        let line = lines.lock().unwrap().pop().unwrap();
        let json = line.strip_prefix("[session-phase] ").unwrap();
        let value: serde_json::Value = serde_json::from_str(json).unwrap();
        assert_eq!(value["schema"], 1);
        assert_eq!(value["run_id"], "unit-run");
        assert_eq!(value["endpoint_role"], "SOURCE");
        let session_id = value["session_id"].as_str().unwrap();
        assert_eq!(session_id.len(), 16);
        assert!(!line.contains("never-log-this-session-token"));
    }
}
