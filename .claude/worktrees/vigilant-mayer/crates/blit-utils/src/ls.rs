use crate::cli::ListArgs;
use crate::util::{
    Endpoint, format_bytes, metadata_mtime_seconds, parse_endpoint_or_local, rel_path_to_string,
};
use blit_core::generated::blit_client::BlitClient;
use blit_core::generated::{FileInfo, ListRequest};
use blit_core::remote::endpoint::{RemoteEndpoint, RemotePath};
use eyre::{Context, Result, bail};
use serde::Serialize;
use std::fs;
use std::path::Path;

#[derive(Serialize)]
struct DirEntryJson {
    name: String,
    is_dir: bool,
    size: u64,
    mtime_seconds: i64,
}

impl DirEntryJson {
    fn from_proto(info: &FileInfo) -> Self {
        Self {
            name: info.name.clone(),
            is_dir: info.is_dir,
            size: info.size,
            mtime_seconds: info.mtime_seconds,
        }
    }

    fn from_fs(name: std::ffi::OsString, meta: &fs::Metadata) -> Self {
        let is_dir = meta.is_dir();
        let size = if is_dir { 0 } else { meta.len() };
        let mtime_seconds = metadata_mtime_seconds(meta).unwrap_or(0);
        Self {
            name: name.to_string_lossy().into_owned(),
            is_dir,
            size,
            mtime_seconds,
        }
    }

    fn from_path(name: Option<&std::ffi::OsStr>, meta: &fs::Metadata) -> Self {
        let default = std::ffi::OsStr::new(".");
        Self::from_fs(name.unwrap_or(default).to_os_string(), meta)
    }
}

pub async fn run_ls(args: ListArgs) -> Result<()> {
    match parse_endpoint_or_local(&args.target) {
        Endpoint::Local(path) => list_local_path(&path, args.json),
        Endpoint::Remote(remote) => list_remote_path(remote, args.json).await,
    }
}

fn list_local_path(path: &Path, json: bool) -> Result<()> {
    let metadata =
        fs::metadata(path).with_context(|| format!("reading metadata for {}", path.display()))?;

    if json {
        let entries_json = if metadata.is_dir() {
            let mut entries = Vec::new();
            for entry in fs::read_dir(path)
                .with_context(|| format!("reading directory {}", path.display()))?
            {
                let entry =
                    entry.with_context(|| format!("reading entry in {}", path.display()))?;
                let meta = entry
                    .metadata()
                    .with_context(|| format!("reading metadata for {}", entry.path().display()))?;
                entries.push(DirEntryJson::from_fs(entry.file_name(), &meta));
            }
            entries
        } else {
            vec![DirEntryJson::from_path(path.file_name(), &metadata)]
        };
        println!("{}", serde_json::to_string_pretty(&entries_json)?);
        return Ok(());
    }

    if metadata.is_dir() {
        println!("Listing {}:", path.display());
        let mut entries: Vec<_> = fs::read_dir(path)
            .with_context(|| format!("reading directory {}", path.display()))?
            .collect::<Result<Vec<_>, _>>()
            .with_context(|| format!("collecting entries for {}", path.display()))?;
        entries.sort_by_key(|entry| entry.path());
        for entry in entries {
            let meta = entry
                .metadata()
                .with_context(|| format!("reading metadata for {}", entry.path().display()))?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if meta.is_dir() {
                println!("DIR  {:>12} {}/", "-", name);
            } else {
                println!("FILE {:>12} {}", format_bytes(meta.len()), name);
            }
        }
    } else {
        println!(
            "FILE {:>12} {}",
            format_bytes(metadata.len()),
            path.display()
        );
    }
    Ok(())
}

async fn list_remote_path(remote: RemoteEndpoint, json: bool) -> Result<()> {
    let (module, rel_path) = match &remote.path {
        RemotePath::Module { module, rel_path } => (module.clone(), rel_path.clone()),
        RemotePath::Root { .. } => {
            bail!("listing root exports (server://...) is not supported yet");
        }
        RemotePath::Discovery => {
            bail!("listing a bare host requires `list-modules` or module/path syntax");
        }
    };

    let path_str = rel_path_to_string(&rel_path);

    let uri = remote.control_plane_uri();
    let mut client = BlitClient::connect(uri.clone())
        .await
        .with_context(|| format!("connecting to {}", uri))?;
    let response = client
        .list(ListRequest {
            module: module.clone(),
            path: path_str.clone(),
        })
        .await
        .map_err(|status| eyre::eyre!(status.message().to_string()))?
        .into_inner();

    if json {
        let entries: Vec<_> = response
            .entries
            .iter()
            .map(DirEntryJson::from_proto)
            .collect();
        println!("{}", serde_json::to_string_pretty(&entries)?);
    } else if response.entries.is_empty() {
        println!(
            "No entries under {}:/{}",
            module,
            if path_str.is_empty() { "" } else { &path_str }
        );
    } else {
        println!(
            "Listing {}:/{}:",
            module,
            if path_str.is_empty() { "" } else { &path_str }
        );
        for entry in response.entries {
            let indicator = if entry.is_dir { "DIR " } else { "FILE" };
            let size_str = if entry.is_dir {
                "-".to_string()
            } else {
                format_bytes(entry.size)
            };
            println!("{} {:>12} {}", indicator, size_str, entry.name);
        }
    }

    Ok(())
}
