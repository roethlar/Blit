use base64::{engine::general_purpose, Engine as _};
use eyre::{bail, eyre, Result};
use std::collections::{HashMap, VecDeque};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, UNIX_EPOCH};
use tokio::sync::mpsc;
use tokio::task;
use tonic::Status;

use crate::enumeration::{EntryKind, FileEnumerator};
use crate::fs_enum::FileFilter;
use crate::generated::client_push_request::Payload as ClientPayload;
use crate::generated::{ClientPushRequest, FileHeader, ManifestComplete, ServerPushResponse};

pub fn drain_pending_headers(
    queue: &mut VecDeque<String>,
    lookup: &HashMap<String, FileHeader>,
) -> Vec<FileHeader> {
    let mut headers = Vec::new();
    while let Some(rel) = queue.front() {
        if let Some(header) = lookup.get(rel) {
            headers.push(header.clone());
            queue.pop_front();
        } else {
            break;
        }
    }
    headers
}

pub async fn send_payload(
    tx: &mpsc::Sender<ClientPushRequest>,
    payload: ClientPayload,
) -> Result<()> {
    tx.send(ClientPushRequest {
        payload: Some(payload),
    })
    .await
    .map_err(|_| eyre!("failed to send push request payload"))
}

pub fn map_status(status: Status) -> eyre::Report {
    eyre!(status.message().to_string())
}

pub fn normalize_relative_path(path: &Path) -> String {
    let raw = path.to_string_lossy();
    #[cfg(windows)]
    {
        raw.replace('\\', "/")
    }
    #[cfg(not(windows))]
    {
        raw.into_owned()
    }
}

pub fn unix_seconds(metadata: &std::fs::Metadata) -> i64 {
    match metadata.modified() {
        Ok(time) => match time.duration_since(UNIX_EPOCH) {
            Ok(dur) => dur.as_secs() as i64,
            Err(err) => {
                let duration = err.duration();
                -(duration.as_secs() as i64)
            }
        },
        Err(_) => 0,
    }
}

pub fn permissions_mode(metadata: &std::fs::Metadata) -> u32 {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode()
    }
    #[cfg(not(unix))]
    {
        let _ = metadata;
        0
    }
}

pub fn decode_token(token: &str) -> Result<Vec<u8>> {
    general_purpose::STANDARD_NO_PAD
        .decode(token.as_bytes())
        .map_err(|err| eyre!("failed to decode negotiation token: {err}"))
}

pub fn spawn_manifest_task(
    root: PathBuf,
    filter: FileFilter,
    unreadable: Arc<Mutex<Vec<String>>>,
) -> (mpsc::Receiver<FileHeader>, task::JoinHandle<Result<u64>>) {
    let (manifest_tx, manifest_rx) = mpsc::channel::<FileHeader>(64);
    let handle = task::spawn_blocking(move || -> Result<u64> {
        let enumerator = FileEnumerator::new(filter);
        let start = Instant::now();
        let mut last_log = start;
        let mut enumerated: u64 = 0;
        let unreadable = unreadable;
        enumerator.enumerate_local_streaming(&root, |entry| {
            if let EntryKind::File { size } = entry.kind {
                let rel = normalize_relative_path(&entry.relative_path);
                let absolute = entry.absolute_path.clone();

                if let Err(err) = std::fs::File::open(&absolute) {
                    match err.kind() {
                        ErrorKind::PermissionDenied => {
                            record_unreadable_entry(&unreadable, &rel, "permission denied");
                            return Ok(());
                        }
                        ErrorKind::NotFound => {
                            record_unreadable_entry(&unreadable, &rel, "not found");
                            return Ok(());
                        }
                        _ => {
                            return Err(eyre!(format!(
                                "manifest open {}: {}",
                                absolute.display(),
                                err
                            )));
                        }
                    }
                }

                let mtime = unix_seconds(&entry.metadata);
                let permissions = permissions_mode(&entry.metadata);
                let header = FileHeader {
                    relative_path: rel,
                    size,
                    mtime_seconds: mtime,
                    permissions,
                };
                manifest_tx
                    .blocking_send(header)
                    .map_err(|_| eyre!("failed to queue manifest entry"))?;
                enumerated += 1;
                if last_log.elapsed() >= Duration::from_secs(1) {
                    println!("Enumerated {} entries… (streaming manifest)", enumerated);
                    last_log = Instant::now();
                }
            }
            Ok(())
        })?;
        println!(
            "Manifest enumeration complete in {:.2?} ({} entries)",
            start.elapsed(),
            enumerated
        );
        Ok(enumerated)
    });

    (manifest_rx, handle)
}

pub fn record_unreadable_entry(list: &Arc<Mutex<Vec<String>>>, rel: &str, reason: &str) {
    eprintln!("[push] skipping '{}' ({})", rel, reason);
    if let Ok(mut guard) = list.lock() {
        guard.push(format!("{} ({})", rel, reason));
    }
}

pub fn spawn_response_task(
    mut stream: tonic::Streaming<ServerPushResponse>,
) -> (
    mpsc::Receiver<Result<ServerPushResponse, eyre::Report>>,
    task::JoinHandle<()>,
) {
    let (response_tx, response_rx) = mpsc::channel::<Result<ServerPushResponse, eyre::Report>>(32);
    let task = tokio::spawn(async move {
        loop {
            match stream.message().await {
                Ok(Some(msg)) => {
                    if response_tx.send(Ok(msg)).await.is_err() {
                        break;
                    }
                }
                Ok(None) => break,
                Err(status) => {
                    let _ = response_tx.send(Err(map_status(status))).await;
                    break;
                }
            }
        }
    });
    (response_rx, task)
}

pub fn destination_path(rel: &Path) -> String {
    if rel.as_os_str().is_empty() {
        String::new()
    } else {
        rel.iter()
            .map(|component| component.to_string_lossy())
            .collect::<Vec<_>>()
            .join("/")
    }
}

pub async fn send_manifest_complete(tx: &mpsc::Sender<ClientPushRequest>) -> Result<()> {
    send_payload(tx, ClientPayload::ManifestComplete(ManifestComplete {})).await
}

pub fn module_and_path(
    endpoint: &crate::remote::endpoint::RemoteEndpoint,
) -> Result<(String, PathBuf)> {
    use crate::remote::endpoint::RemotePath;
    match &endpoint.path {
        RemotePath::Module { module, rel_path } => Ok((module.clone(), rel_path.clone())),
        RemotePath::Root { rel_path } => Ok((String::new(), rel_path.clone())),
        RemotePath::Discovery => bail!("remote destination missing module specification"),
    }
}
