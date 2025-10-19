use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc,
};

use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;
use tokio::time::Duration;

use eyre::{eyre, Result};

use crate::transfer_plan::{Plan, TransferTask};

#[derive(Clone, Copy, Debug, Default)]
pub struct Sample {
    pub bytes: u64,
    pub ms: u128,
}

pub struct SchedulerOptions {
    pub ludicrous_speed: bool,
    pub progress: bool,
    pub byte_drain: Option<Arc<dyn Fn() -> u64 + Send + Sync>>,
    pub initial_streams: Option<usize>,
    pub max_streams: Option<usize>,
}

pub struct WorkerParams {
    pub idx: usize,
    pub chunk_bytes: usize,
    pub progress: bool,
    pub rx_shared: Arc<Mutex<mpsc::Receiver<TransferTask>>>,
    pub remaining: Arc<AtomicUsize>,
    pub active: Arc<AtomicUsize>,
    pub exit_tokens: Arc<AtomicUsize>,
    pub stat_tx: mpsc::UnboundedSender<Sample>,
}

pub trait WorkerFactory: Send + Sync {
    fn spawn_worker(&self, params: WorkerParams) -> JoinHandle<Result<()>>;
}

pub struct TaskStreamSender {
    tx: mpsc::Sender<TransferTask>,
    remaining: Arc<AtomicUsize>,
    closed: Arc<AtomicBool>,
}

impl TaskStreamSender {
    pub fn send_blocking(&self, task: TransferTask) -> Result<()> {
        self.remaining.fetch_add(1, Ordering::Relaxed);
        self.tx
            .blocking_send(task)
            .map_err(|_| eyre!("transfer task receiver dropped"))
    }

    pub async fn send(&self, task: TransferTask) -> Result<()> {
        self.remaining.fetch_add(1, Ordering::Relaxed);
        self.tx
            .send(task)
            .await
            .map_err(|_| eyre!("transfer task receiver dropped"))
    }

    pub fn remaining(&self) -> Arc<AtomicUsize> {
        Arc::clone(&self.remaining)
    }

    pub fn closed_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.closed)
    }
}

impl Drop for TaskStreamSender {
    fn drop(&mut self) {
        self.closed.store(true, Ordering::SeqCst);
    }
}

pub fn create_task_stream(capacity: usize) -> (TaskStreamSender, mpsc::Receiver<TransferTask>) {
    let (tx, rx) = mpsc::channel::<TransferTask>(capacity);
    let remaining = Arc::new(AtomicUsize::new(0));
    let closed = Arc::new(AtomicBool::new(false));
    (
        TaskStreamSender {
            tx,
            remaining: Arc::clone(&remaining),
            closed,
        },
        rx,
    )
}

pub async fn execute_plan(
    factory: &dyn WorkerFactory,
    plan: Plan,
    chunk_bytes: usize,
    options: SchedulerOptions,
) -> Result<()> {
    let total_tasks = plan.tasks.len().max(1);
    let (task_sender, rx_tasks) = create_task_stream(total_tasks);
    let remaining = task_sender.remaining();
    let closed_flag = task_sender.closed_flag();

    for task in plan.tasks.into_iter() {
        task_sender.send(task).await?;
    }
    drop(task_sender);

    execute_streaming_with_receiver(
        factory,
        rx_tasks,
        chunk_bytes,
        options,
        remaining,
        closed_flag,
    )
    .await
}

pub async fn execute_streaming_plan(
    factory: &dyn WorkerFactory,
    chunk_bytes: usize,
    options: SchedulerOptions,
    task_receiver: mpsc::Receiver<TransferTask>,
    remaining: Arc<AtomicUsize>,
    closed_flag: Arc<AtomicBool>,
) -> Result<()> {
    execute_streaming_with_receiver(
        factory,
        task_receiver,
        chunk_bytes,
        options,
        remaining,
        closed_flag,
    )
    .await
}

async fn execute_streaming_with_receiver(
    factory: &dyn WorkerFactory,
    rx_tasks: mpsc::Receiver<TransferTask>,
    chunk_bytes: usize,
    options: SchedulerOptions,
    remaining: Arc<AtomicUsize>,
    closed_flag: Arc<AtomicBool>,
) -> Result<()> {
    let rx_shared = Arc::new(Mutex::new(rx_tasks));
    let active = Arc::new(AtomicUsize::new(0));
    let exit_tokens = Arc::new(AtomicUsize::new(0));
    let (stat_tx, mut stat_rx) = mpsc::unbounded_channel::<Sample>();

    let initial_streams = options.initial_streams.unwrap_or(4); // Start with 4 workers (was 2-3)
    let max_streams_base = if options.ludicrous_speed {
        16 // Was: 12
    } else {
        12 // Was: 8
    };

    // Bound by CPU cores * 2
    let cpu_bound = num_cpus::get() * 2;
    let max_streams = options
        .max_streams
        .unwrap_or(max_streams_base.min(cpu_bound));

    let mut handles: Vec<JoinHandle<Result<()>>> = Vec::new();
    for idx in 0..initial_streams {
        let params = WorkerParams {
            idx,
            chunk_bytes,
            progress: options.progress,
            rx_shared: Arc::clone(&rx_shared),
            remaining: Arc::clone(&remaining),
            active: Arc::clone(&active),
            exit_tokens: Arc::clone(&exit_tokens),
            stat_tx: stat_tx.clone(),
        };
        handles.push(factory.spawn_worker(params));
    }

    let mut ewma_gbps: f64 = 0.0;

    loop {
        let mut tick_bytes: u64 = 0;
        while let Ok(Sample { bytes, .. }) = stat_rx.try_recv() {
            tick_bytes = tick_bytes.saturating_add(bytes);
        }
        if let Some(drain) = &options.byte_drain {
            tick_bytes = tick_bytes.saturating_add(drain());
        }

        // Scale to 1-second throughput (tick is 250ms, so multiply by 4)
        let bytes_per_second = tick_bytes * 4;
        let tick_gbps = (bytes_per_second as f64) * 8.0 / 1e9;
        if tick_gbps > 0.0 {
            ewma_gbps = if ewma_gbps == 0.0 {
                tick_gbps
            } else {
                0.3 * tick_gbps + 0.7 * ewma_gbps
            };
        }

        let rem = remaining.load(Ordering::Relaxed);
        let act = active.load(Ordering::Relaxed);
        if rem == 0 && closed_flag.load(Ordering::SeqCst) {
            break;
        }

        // Simplified aggressive scaling: add workers until saturated or maxed out
        let should_scale_up = (ewma_gbps < 9.0 || rem > act) && act < max_streams;

        if should_scale_up {
            let idx = handles.len();
            let params = WorkerParams {
                idx,
                chunk_bytes,
                progress: options.progress,
                rx_shared: Arc::clone(&rx_shared),
                remaining: Arc::clone(&remaining),
                active: Arc::clone(&active),
                exit_tokens: Arc::clone(&exit_tokens),
                stat_tx: stat_tx.clone(),
            };
            handles.push(factory.spawn_worker(params));
        }

        tokio::time::sleep(Duration::from_millis(250)).await; // Faster scaling response (was 1000ms)
    }

    // Aggregate errors from all workers instead of stopping at first failure
    const MAX_ERRORS_DETAILED: usize = 50;
    const MAX_ERROR_MESSAGE_BYTES: usize = 64 * 1024; // 64KB limit

    let mut errors = Vec::new();
    for handle in handles {
        match handle.await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => errors.push(e),
            Err(e) => errors.push(eyre!("worker panic: {}", e)),
        }
    }

    if !errors.is_empty() {
        let total_errors = errors.len();

        // Build error message with limits to prevent unbounded growth
        let mut error_msg = String::new();
        let mut bytes_used = 0usize;

        for (idx, err) in errors.iter().enumerate() {
            if idx >= MAX_ERRORS_DETAILED {
                // Truncate detailed errors after limit
                let remaining = total_errors - idx;
                error_msg.push_str(&format!("... and {} more error(s)", remaining));
                break;
            }

            let err_str = err.to_string();
            let err_bytes = err_str.len();

            if bytes_used + err_bytes + 2 > MAX_ERROR_MESSAGE_BYTES {
                // Would exceed byte limit
                let remaining = total_errors - idx;
                error_msg.push_str(&format!("... and {} more error(s) (truncated)", remaining));
                break;
            }

            if idx > 0 {
                error_msg.push_str("; ");
                bytes_used += 2;
            }
            error_msg.push_str(&err_str);
            bytes_used += err_bytes;
        }

        let summary = format!("{} worker(s) failed: {}", total_errors, error_msg);
        return Err(eyre!(summary));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transfer_plan::TransferTask;
    use std::path::PathBuf;

    // Mock WorkerFactory for testing error aggregation
    struct MockWorkerFactory {
        worker_results: Vec<Result<(), String>>,
    }

    impl WorkerFactory for MockWorkerFactory {
        fn spawn_worker(&self, params: WorkerParams) -> JoinHandle<Result<()>> {
            let idx = params.idx;
            let result = self.worker_results.get(idx).cloned().unwrap_or(Ok(()));

            tokio::spawn(async move {
                params.active.fetch_add(1, Ordering::Relaxed);
                // Consume all tasks from the channel
                let mut rx = params.rx_shared.lock().await;
                while rx.recv().await.is_some() {
                    params.remaining.fetch_sub(1, Ordering::Relaxed);
                }
                drop(rx);

                // Return the predetermined result
                let outcome = match result {
                    Ok(()) => Ok(()),
                    Err(e) => Err(eyre!("{}", e)),
                };

                params.active.fetch_sub(1, Ordering::Relaxed);
                outcome
            })
        }
    }

    #[tokio::test]
    async fn test_single_worker_error_propagation() {
        let factory = MockWorkerFactory {
            worker_results: vec![Ok(()), Err("Permission denied: /test/file.txt".to_string())],
        };

        let plan = Plan {
            tasks: vec![
                TransferTask::Large {
                    path: PathBuf::from("file1.txt"),
                },
                TransferTask::Large {
                    path: PathBuf::from("file2.txt"),
                },
            ],
            chunk_bytes: 1024 * 1024,
        };

        let opts = SchedulerOptions {
            ludicrous_speed: false,
            progress: false,
            byte_drain: None,
            initial_streams: Some(2),
            max_streams: Some(2),
        };

        let result = execute_plan(&factory, plan, 1024 * 1024, opts).await;

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        println!("err_msg: {}", err_msg);
        assert!(err_msg.contains("1 worker(s) failed"));
        assert!(err_msg.contains("Permission denied"));
    }

    #[tokio::test]
    async fn test_multiple_worker_errors_aggregated() {
        let factory = MockWorkerFactory {
            worker_results: vec![
                Ok(()),
                Err("I/O error: /path/file1.txt".to_string()),
                Ok(()),
                Err("Disk full: /path/file2.txt".to_string()),
                Err("Permission denied: /path/file3.txt".to_string()),
            ],
        };

        let plan = Plan {
            tasks: vec![
                TransferTask::Large {
                    path: PathBuf::from("file1.txt"),
                },
                TransferTask::Large {
                    path: PathBuf::from("file2.txt"),
                },
                TransferTask::Large {
                    path: PathBuf::from("file3.txt"),
                },
                TransferTask::Large {
                    path: PathBuf::from("file4.txt"),
                },
                TransferTask::Large {
                    path: PathBuf::from("file5.txt"),
                },
            ],
            chunk_bytes: 1024 * 1024,
        };

        let opts = SchedulerOptions {
            ludicrous_speed: false,
            progress: false,
            byte_drain: None,
            initial_streams: Some(5),
            max_streams: Some(5),
        };

        let result = execute_plan(&factory, plan, 1024 * 1024, opts).await;

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("3 worker(s) failed"));
        assert!(err_msg.contains("I/O error"));
        assert!(err_msg.contains("Disk full"));
        assert!(err_msg.contains("Permission denied"));
    }

    #[tokio::test]
    async fn test_all_workers_succeed() {
        let factory = MockWorkerFactory {
            worker_results: vec![Ok(()), Ok(()), Ok(())],
        };

        let plan = Plan {
            tasks: vec![
                TransferTask::Large {
                    path: PathBuf::from("file1.txt"),
                },
                TransferTask::Large {
                    path: PathBuf::from("file2.txt"),
                },
                TransferTask::Large {
                    path: PathBuf::from("file3.txt"),
                },
            ],
            chunk_bytes: 1024 * 1024,
        };

        let opts = SchedulerOptions {
            ludicrous_speed: false,
            progress: false,
            byte_drain: None,
            initial_streams: Some(3),
            max_streams: Some(3),
        };

        let result = execute_plan(&factory, plan, 1024 * 1024, opts).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_error_message_truncation() {
        // Create a factory with many errors
        let mut worker_results = Vec::new();
        for i in 0..100 {
            worker_results.push(Err(format!("Error on worker {}: File operation failed", i)));
        }

        let factory = MockWorkerFactory { worker_results };

        let mut tasks = Vec::new();
        for i in 0..100 {
            tasks.push(TransferTask::Large {
                path: PathBuf::from(format!("file{}.txt", i)),
            });
        }

        let plan = Plan {
            tasks,
            chunk_bytes: 1024 * 1024,
        };

        let opts = SchedulerOptions {
            ludicrous_speed: false,
            progress: false,
            byte_drain: None,
            initial_streams: Some(100),
            max_streams: Some(100),
        };

        let result = execute_plan(&factory, plan, 1024 * 1024, opts).await;

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("100 worker(s) failed"));
        // Should contain truncation message since we have 100 errors (> 50 limit)
        assert!(err_msg.contains("and") && err_msg.contains("more error"));
    }

    #[tokio::test]
    async fn test_large_error_message_byte_limit() {
        // Create errors with very long messages
        let mut worker_results = Vec::new();
        let long_error = "Error: ".to_string() + &"x".repeat(10000);
        for _ in 0..20 {
            worker_results.push(Err(long_error.clone()));
        }

        let factory = MockWorkerFactory { worker_results };

        let mut tasks = Vec::new();
        for i in 0..20 {
            tasks.push(TransferTask::Large {
                path: PathBuf::from(format!("file{}.txt", i)),
            });
        }

        let plan = Plan {
            tasks,
            chunk_bytes: 1024 * 1024,
        };

        let opts = SchedulerOptions {
            ludicrous_speed: false,
            progress: false,
            byte_drain: None,
            initial_streams: Some(20),
            max_streams: Some(20),
        };

        let result = execute_plan(&factory, plan, 1024 * 1024, opts).await;

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();

        // Error message should be truncated to stay within 64KB limit
        assert!(
            err_msg.len() < 70 * 1024,
            "Error message should be under 70KB"
        );
        assert!(err_msg.contains("20 worker(s) failed"));
        assert!(err_msg.contains("truncated") || err_msg.contains("more error"));
    }
}
