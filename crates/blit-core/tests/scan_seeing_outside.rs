//! cr-uv7-3: a `TransferSource` written outside `blit-core` — this test
//! binary is such a crate — that walks a disk of its own can record what
//! its walk saw and hand that record back with its scan, for a move's
//! account (`moved::Sightings`, cr-uv5-2, cr-uv6-4). `SourceScan::seeing`
//! was the crate's alone: such a source failed to compile (E0624).

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use blit_core::fs_enum::FileFilter;
use blit_core::generated::FileHeader;
use blit_core::moved::{name_key, sighting, Sightings, Version};
use blit_core::remote::transfer::payload::{PreparedPayload, TransferPayload};
use blit_core::remote::transfer::source::{
    FsTransferSource, OpenedSourceFile, SourceScan, TransferSource,
};
use tokio::sync::mpsc;

/// A source of one file, which it looks at itself.
struct OneFile {
    root: PathBuf,
    inner: FsTransferSource,
}

#[async_trait]
impl TransferSource for OneFile {
    fn scan(
        &self,
        _filter: Option<FileFilter>,
        _unreadable: Arc<Mutex<Vec<blit_core::remote::transfer::source::Unreadable>>>,
    ) -> (mpsc::Receiver<FileHeader>, SourceScan) {
        self.walk(None)
    }

    fn scan_seeing(
        &self,
        _filter: Option<FileFilter>,
        _unreadable: Arc<Mutex<Vec<blit_core::remote::transfer::source::Unreadable>>>,
        _preserve_windows_metadata: bool,
    ) -> (mpsc::Receiver<FileHeader>, SourceScan) {
        let sightings = Arc::new(Sightings::default());
        let (headers, scan) = self.walk(Some(Arc::clone(&sightings)));
        (headers, scan.seeing(sightings))
    }

    async fn prepare_payload(&self, payload: TransferPayload) -> eyre::Result<PreparedPayload> {
        self.inner.prepare_payload(payload).await
    }

    async fn open_file(&self, header: &FileHeader) -> eyre::Result<OpenedSourceFile> {
        self.inner.open_file(header).await
    }

    fn root(&self) -> &Path {
        &self.root
    }
}

impl OneFile {
    fn walk(&self, sightings: Option<Arc<Sightings>>) -> (mpsc::Receiver<FileHeader>, SourceScan) {
        let (tx, rx) = mpsc::channel(4);
        let root = self.root.clone();
        let task = tokio::spawn(async move {
            let metadata = std::fs::symlink_metadata(root.join("f"))?;
            if let Some(sightings) = sightings {
                sightings.publish(vec![sighting(b"f", &Version::of(&metadata))]);
            }
            let header = FileHeader {
                relative_path: "f".to_string(),
                size: metadata.len(),
                ..FileHeader::default()
            };
            tx.send(header)
                .await
                .map_err(|_| eyre::eyre!("nobody listening"))?;
            Ok(1)
        });
        (rx, SourceScan::new(task))
    }
}

#[tokio::test]
async fn a_source_outside_the_crate_hands_its_walks_record_back() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().to_path_buf();
    std::fs::write(root.join("f"), b"one file").unwrap();
    let source = OneFile {
        root: root.clone(),
        inner: FsTransferSource::new(root.clone()),
    };
    let (mut headers, mut scan) = source.scan_seeing(None, Arc::default(), true);
    let mut listed = Vec::new();
    while let Some(header) = headers.recv().await {
        listed.push(name_key(header.relative_path.as_bytes()));
    }
    scan.finish().await.expect("the walk");
    let sightings = scan.sightings().expect("the record handed back");
    let account = sightings.account(&root, &listed);
    assert_eq!(account.len(), 1);

    // A plain scan hands nothing back.
    let (mut headers, mut scan) = source.scan(None, Arc::default());
    while headers.recv().await.is_some() {}
    scan.finish().await.expect("the walk");
    assert!(scan.sightings().is_none());
}
