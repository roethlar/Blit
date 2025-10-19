use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use eyre::{eyre, Result};
use tokio::time::{self, MissedTickBehavior};

use crate::transfer_engine::TaskStreamSender;
use crate::transfer_facade::PlannerEvent;

use super::LocalMirrorOptions;

pub(super) struct PlannerDriveSummary {
    pub(super) enumerated_files: usize,
    pub(super) total_bytes: u64,
}

pub(super) async fn drive_planner_events(
    options: &LocalMirrorOptions,
    mut events: tokio::sync::mpsc::UnboundedReceiver<PlannerEvent>,
    task_sender: TaskStreamSender,
    remaining: Arc<AtomicUsize>,
    closed_flag: Arc<AtomicBool>,
    stall_timeout: Duration,
    heartbeat: Duration,
) -> Result<PlannerDriveSummary> {
    let mut last_planner_activity = Instant::now();
    let mut last_worker_remaining = remaining.load(Ordering::Relaxed);
    let mut last_worker_activity = Instant::now();
    let mut enumerated_files = 0usize;
    let mut total_bytes = 0u64;

    let mut ticker = time::interval(heartbeat);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

    let mut sender = Some(task_sender);

    loop {
        tokio::select! {
            maybe_event = events.recv() => {
                match maybe_event {
                    Some(PlannerEvent::Task(task)) => {
                        if let Some(ref s) = sender {
                            s.send(task).await?;
                        }
                        last_planner_activity = Instant::now();
                    }
                    Some(PlannerEvent::Progress { enumerated_files: files, total_bytes: bytes }) => {
                        enumerated_files = files;
                        total_bytes = bytes;
                        last_planner_activity = Instant::now();
                        if options.verbose {
                            eprintln!("Planning… {} file(s), {} bytes", files, bytes);
                        }
                    }
                    None => {
                        break;
                    }
                }
            }
            _ = ticker.tick() => {
                let now = Instant::now();
                let current_remaining = remaining.load(Ordering::Relaxed);
                if current_remaining < last_worker_remaining {
                    last_worker_remaining = current_remaining;
                    last_worker_activity = now;
                }

                if now.duration_since(last_planner_activity) >= stall_timeout
                    && now.duration_since(last_worker_activity) >= stall_timeout
                    && (!closed_flag.load(Ordering::SeqCst) || current_remaining > 0)
                {
                    return Err(eyre!("planner or workers stalled for > {:?}", stall_timeout));
                }
            }
        }
    }

    drop(sender.take());

    Ok(PlannerDriveSummary {
        enumerated_files,
        total_bytes,
    })
}
