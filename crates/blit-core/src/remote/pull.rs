use std::path::{Path, PathBuf};

use eyre::{bail, eyre, Context, Result};
use tokio::fs::{self, File};
use tokio::io::AsyncWriteExt;

use crate::generated::blit_client::BlitClient;
use crate::generated::{pull_chunk, FileData, PullRequest};
use crate::remote::endpoint::{RemoteEndpoint, RemotePath};

#[derive(Debug, Default, Clone)]
pub struct RemotePullReport {
    pub files_transferred: usize,
    pub bytes_transferred: u64,
    pub downloaded_paths: Vec<PathBuf>,
}

pub struct RemotePullClient {
    endpoint: RemoteEndpoint,
    client: BlitClient<tonic::transport::Channel>,
}

impl RemotePullClient {
    pub async fn connect(endpoint: RemoteEndpoint) -> Result<Self> {
        let uri = endpoint.control_plane_uri();
        let client = BlitClient::connect(uri.clone())
            .await
            .map_err(|err| eyre!("failed to connect to {}: {}", uri, err))?;

        Ok(Self { endpoint, client })
    }

    pub async fn pull(
        &mut self,
        dest_root: &Path,
        force_grpc: bool,
        track_paths: bool,
    ) -> Result<RemotePullReport> {
        if !dest_root.exists() {
            fs::create_dir_all(dest_root).await.with_context(|| {
                format!("creating destination directory {}", dest_root.display())
            })?;
        }

        let (module, rel_path) = match &self.endpoint.path {
            RemotePath::Module { module, rel_path } => (module.clone(), rel_path.clone()),
            RemotePath::Root { rel_path } => (String::new(), rel_path.clone()),
            RemotePath::Discovery => {
                bail!("remote source must specify a module (server:/module/...)");
            }
        };

        let path_str = if rel_path.as_os_str().is_empty() {
            ".".to_string()
        } else {
            normalize_for_request(&rel_path)
        };

        let pull_request = PullRequest {
            module,
            path: path_str,
            force_grpc,
        };

        let mut stream = self
            .client
            .pull(pull_request)
            .await
            .map_err(|status| eyre!(status.message().to_string()))?
            .into_inner();

        let mut report = RemotePullReport::default();
        let mut active_file: Option<(File, PathBuf)> = None;

        while let Some(chunk) = stream
            .message()
            .await
            .map_err(|status| eyre!(status.message().to_string()))?
        {
            match chunk.payload {
                Some(pull_chunk::Payload::FileHeader(header)) => {
                    finalize_active_file(&mut active_file).await?;

                    let relative_path = sanitize_relative_path(&header.relative_path)?;
                    let dest_path = dest_root.join(&relative_path);
                    if let Some(parent) = dest_path.parent() {
                        fs::create_dir_all(parent)
                            .await
                            .with_context(|| format!("creating directory {}", parent.display()))?;
                    }

                    let file = File::create(&dest_path)
                        .await
                        .with_context(|| format!("creating {}", dest_path.display()))?;

                    if track_paths {
                        report.downloaded_paths.push(relative_path.clone());
                    }

                    active_file = Some((file, dest_path));
                    report.files_transferred += 1;
                }
                Some(pull_chunk::Payload::FileData(FileData { content })) => {
                    let (file, path) = active_file
                        .as_mut()
                        .ok_or_else(|| eyre!("received file data without a preceding header"))?;
                    file.write_all(&content)
                        .await
                        .with_context(|| format!("writing {}", path.display()))?;
                    report.bytes_transferred += content.len() as u64;
                }
                None => {}
            }
        }

        finalize_active_file(&mut active_file).await?;

        Ok(report)
    }
}

async fn finalize_active_file(active: &mut Option<(File, PathBuf)>) -> Result<()> {
    if let Some((file, _)) = active.take() {
        file.sync_all().await?;
    }
    Ok(())
}

fn sanitize_relative_path(raw: &str) -> Result<PathBuf> {
    if raw.is_empty() {
        bail!("server sent empty relative path");
    }

    let path = Path::new(raw);
    if path.is_absolute() {
        bail!("server returned absolute path: {}", raw);
    }

    use std::path::Component;
    if path
        .components()
        .any(|c| matches!(c, Component::ParentDir | Component::Prefix(_)))
    {
        bail!(
            "server returned parent directory component in path: {}",
            raw
        );
    }

    Ok(path.to_path_buf())
}

fn normalize_for_request(path: &Path) -> String {
    if path.as_os_str().is_empty() {
        ".".to_string()
    } else {
        path.iter()
            .map(|component| component.to_string_lossy())
            .collect::<Vec<_>>()
            .join("/")
    }
}
