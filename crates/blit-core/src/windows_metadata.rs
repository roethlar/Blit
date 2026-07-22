//! Contract-v4 Windows file attributes and named `$DATA` streams.
//!
//! The protobuf shapes are platform-neutral so every carrier can validate a
//! peer without trusting Windows path syntax. Filesystem enumeration and apply
//! live behind `cfg(windows)`; a non-Windows destination refuses present
//! Windows metadata instead of silently discarding it.

use std::collections::HashSet;
use std::path::Path;

use eyre::{bail, Context, Result};

#[cfg(any(windows, test))]
use crate::generated::WindowsNamedStream;
use crate::generated::{FileHeader, WindowsFileMetadata};

/// Durable ordinary-file attributes that Blit promises to preserve.
pub const WINDOWS_PRESERVED_ATTRIBUTE_MASK: u32 = 0x0000_0027;
/// Complete subset accepted by `SetFileAttributesW`. Only used while clearing
/// READONLY on an existing destination so unrelated volatile policy bits are
/// not destroyed by preparation.
#[cfg(windows)]
const WINDOWS_SET_FILE_ATTRIBUTES_API_MASK: u32 = 0x0000_3127;
pub const MAX_WINDOWS_NAMED_STREAMS: usize = 64;
pub const MAX_WINDOWS_STREAM_NAME_BYTES: usize = 1024;
pub const MAX_WINDOWS_STREAM_BYTES_PER_FILE: u64 = 2 * 1024 * 1024;

pub fn validate_manifest(metadata: Option<&WindowsFileMetadata>) -> Result<()> {
    if let Some(metadata) = metadata {
        validate_common(metadata, false)?;
    }
    Ok(())
}

pub fn validate_payload(metadata: Option<&WindowsFileMetadata>) -> Result<()> {
    if let Some(metadata) = metadata {
        validate_common(metadata, true)?;
    }
    Ok(())
}

fn validate_common(metadata: &WindowsFileMetadata, payload: bool) -> Result<()> {
    let unsupported = metadata.file_attributes & !WINDOWS_PRESERVED_ATTRIBUTE_MASK;
    if unsupported != 0 {
        bail!("unsupported Windows file-attribute bits 0x{unsupported:08x}");
    }
    if metadata.named_streams.len() > MAX_WINDOWS_NAMED_STREAMS {
        bail!(
            "Windows named-stream count {} exceeds cap {}",
            metadata.named_streams.len(),
            MAX_WINDOWS_NAMED_STREAMS
        );
    }

    let mut names = HashSet::with_capacity(metadata.named_streams.len());
    let mut previous_key: Option<String> = None;
    let mut total = 0u64;
    for stream in &metadata.named_streams {
        validate_stream_name(&stream.name)?;
        let key = stream.name.to_lowercase();
        if !names.insert(key.clone()) {
            bail!("duplicate Windows named stream {:?}", stream.name);
        }
        if previous_key
            .as_ref()
            .is_some_and(|previous| previous >= &key)
        {
            bail!("Windows named streams are not in canonical name order");
        }
        previous_key = Some(key);

        if stream.size > MAX_WINDOWS_STREAM_BYTES_PER_FILE {
            bail!(
                "Windows named stream {:?} size {} exceeds per-file cap {}",
                stream.name,
                stream.size,
                MAX_WINDOWS_STREAM_BYTES_PER_FILE
            );
        }
        total = total
            .checked_add(stream.size)
            .ok_or_else(|| eyre::eyre!("Windows named-stream size overflow"))?;
        if total > MAX_WINDOWS_STREAM_BYTES_PER_FILE {
            bail!(
                "Windows named-stream bytes {} exceed per-file cap {}",
                total,
                MAX_WINDOWS_STREAM_BYTES_PER_FILE
            );
        }
        if stream.checksum.len() != blake3::OUT_LEN {
            bail!(
                "Windows named stream {:?} checksum is {} bytes, expected {}",
                stream.name,
                stream.checksum.len(),
                blake3::OUT_LEN
            );
        }
        if payload {
            if stream.content.len() as u64 != stream.size {
                bail!(
                    "Windows named stream {:?} content is {} bytes, expected {}",
                    stream.name,
                    stream.content.len(),
                    stream.size
                );
            }
            if blake3::hash(&stream.content).as_bytes() != stream.checksum.as_slice() {
                bail!("Windows named stream {:?} checksum mismatch", stream.name);
            }
        } else if !stream.content.is_empty() {
            bail!(
                "manifest Windows named stream {:?} contains payload bytes",
                stream.name
            );
        }
    }
    Ok(())
}

pub fn validate_stream_name(name: &str) -> Result<()> {
    if name.is_empty() {
        bail!("Windows named-stream name is empty");
    }
    if name.len() > MAX_WINDOWS_STREAM_NAME_BYTES {
        bail!(
            "Windows named-stream name is {} bytes, exceeds cap {}",
            name.len(),
            MAX_WINDOWS_STREAM_NAME_BYTES
        );
    }
    if name == "." || name == ".." {
        bail!("unsafe Windows named-stream name {name:?}");
    }
    if name
        .chars()
        .any(|ch| ch.is_control() || matches!(ch, '\0' | ':' | '/' | '\\'))
    {
        bail!("unsafe Windows named-stream name {name:?}");
    }
    Ok(())
}

pub fn validate_payload_against_manifest(
    payload: Option<&WindowsFileMetadata>,
    manifest: Option<&WindowsFileMetadata>,
) -> Result<()> {
    validate_manifest(manifest)?;
    validate_payload(payload)?;
    match (payload, manifest) {
        (None, None) => Ok(()),
        (Some(_), None) => bail!("payload added Windows metadata absent from the manifest"),
        (None, Some(_)) => bail!("payload omitted Windows metadata declared by the manifest"),
        (Some(payload), Some(manifest)) => {
            if payload.file_attributes != manifest.file_attributes {
                bail!("payload Windows attributes changed after the manifest");
            }
            if payload.named_streams.len() != manifest.named_streams.len() {
                bail!("payload Windows named-stream set changed after the manifest");
            }
            for (payload_stream, manifest_stream) in
                payload.named_streams.iter().zip(&manifest.named_streams)
            {
                if payload_stream.name != manifest_stream.name
                    || payload_stream.size != manifest_stream.size
                    || payload_stream.checksum != manifest_stream.checksum
                {
                    bail!(
                        "payload Windows named stream {:?} changed after the manifest",
                        manifest_stream.name
                    );
                }
            }
            Ok(())
        }
    }
}

pub fn payload_bytes(header: &FileHeader) -> u64 {
    header
        .windows_metadata
        .as_ref()
        .map(|metadata| {
            metadata
                .named_streams
                .iter()
                .map(|stream| stream.size)
                .fold(0u64, u64::saturating_add)
        })
        .unwrap_or(0)
}

pub fn hydrate_payload_header(source_path: &Path, header: &mut FileHeader) -> Result<()> {
    let Some(manifest) = header.windows_metadata.as_ref() else {
        return Ok(());
    };
    validate_manifest(Some(manifest))?;
    let payload = read_payload(source_path)?;
    validate_payload_against_manifest(payload.as_ref(), Some(manifest))
        .with_context(|| format!("Windows metadata changed for {}", source_path.display()))?;
    header.windows_metadata = payload;
    Ok(())
}

#[cfg(windows)]
pub fn read_manifest(path: &Path) -> Result<Option<WindowsFileMetadata>> {
    let mut metadata = read_windows_metadata(path, false)?;
    for stream in &mut metadata.named_streams {
        stream.content.clear();
    }
    validate_manifest(Some(&metadata))?;
    Ok(Some(metadata))
}

#[cfg(not(windows))]
pub fn read_manifest(_path: &Path) -> Result<Option<WindowsFileMetadata>> {
    Ok(None)
}

#[cfg(windows)]
fn read_payload(path: &Path) -> Result<Option<WindowsFileMetadata>> {
    let metadata = read_windows_metadata(path, true)?;
    validate_payload(Some(&metadata))?;
    Ok(Some(metadata))
}

#[cfg(not(windows))]
fn read_payload(path: &Path) -> Result<Option<WindowsFileMetadata>> {
    bail!(
        "cannot hydrate Windows metadata from non-Windows source {}",
        path.display()
    )
}

pub fn destination_matches(path: &Path, expected: Option<&WindowsFileMetadata>) -> Result<bool> {
    let Some(expected) = expected else {
        return Ok(true);
    };
    validate_manifest(Some(expected))?;
    destination_matches_impl(path, expected)
}

#[cfg(windows)]
fn destination_matches_impl(path: &Path, expected: &WindowsFileMetadata) -> Result<bool> {
    let actual = read_windows_metadata(path, false)?;
    validate_manifest(Some(&actual))?;
    Ok(actual == *expected)
}

#[cfg(not(windows))]
fn destination_matches_impl(path: &Path, _expected: &WindowsFileMetadata) -> Result<bool> {
    bail!(
        "destination {} cannot preserve Windows file metadata on this platform",
        path.display()
    )
}

pub fn replace_streams(path: &Path, metadata: Option<&WindowsFileMetadata>) -> Result<u64> {
    let Some(metadata) = metadata else {
        return Ok(0);
    };
    validate_payload(Some(metadata))?;
    replace_streams_impl(path, metadata)
}

pub fn apply_attributes(path: &Path, metadata: Option<&WindowsFileMetadata>) -> Result<()> {
    let Some(metadata) = metadata else {
        return Ok(());
    };
    validate_payload(Some(metadata))?;
    apply_attributes_impl(path, metadata)
}

/// Apply the durable attribute mask and require readback convergence. Some
/// filesystems can return success from the setter without retaining every bit;
/// that is an honest file failure, not a successful transfer that should loop
/// forever on the next comparison.
#[cfg(any(windows, test))]
fn set_and_verify_attributes(
    path: &Path,
    desired: u32,
    set: impl FnOnce(u32) -> Result<()>,
    read: impl FnOnce() -> Result<u32>,
) -> Result<()> {
    set(desired)?;
    let actual = read()? & WINDOWS_PRESERVED_ATTRIBUTE_MASK;
    if actual != desired {
        bail!(
            "Windows attributes did not converge on {}: requested 0x{desired:08x}, read back 0x{actual:08x}",
            path.display()
        );
    }
    Ok(())
}

/// Clear a pre-existing read-only bit before replacing unnamed or named data.
/// The exact source attribute set is restored by [`apply_attributes`] after all bytes land.
pub fn prepare_destination(path: &Path, metadata: Option<&WindowsFileMetadata>) -> Result<()> {
    if metadata.is_none() {
        return Ok(());
    }
    #[cfg(not(windows))]
    {
        prepare_destination_impl(path)
    }
    #[cfg(windows)]
    {
        if path.exists() {
            prepare_destination_impl(path)
        } else {
            Ok(())
        }
    }
}

#[cfg(not(windows))]
fn replace_streams_impl(path: &Path, _metadata: &WindowsFileMetadata) -> Result<u64> {
    bail!(
        "destination {} cannot preserve Windows file metadata on this platform",
        path.display()
    )
}

#[cfg(not(windows))]
fn apply_attributes_impl(path: &Path, _metadata: &WindowsFileMetadata) -> Result<()> {
    bail!(
        "destination {} cannot preserve Windows file metadata on this platform",
        path.display()
    )
}

#[cfg(not(windows))]
fn prepare_destination_impl(path: &Path) -> Result<()> {
    bail!(
        "destination {} cannot preserve Windows file metadata on this platform",
        path.display()
    )
}

#[cfg(windows)]
mod windows_io {
    use std::ffi::{OsStr, OsString};
    use std::io::Read;
    use std::os::windows::ffi::OsStrExt;

    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{ERROR_HANDLE_EOF, ERROR_INVALID_PARAMETER};
    use windows::Win32::Storage::FileSystem::{
        FindClose, FindFirstStreamW, FindNextStreamW, FindStreamInfoStandard, GetFileAttributesW,
        SetFileAttributesW, FILE_ATTRIBUTE_NORMAL, FILE_ATTRIBUTE_READONLY,
        FILE_FLAGS_AND_ATTRIBUTES, INVALID_FILE_ATTRIBUTES, WIN32_FIND_STREAM_DATA,
    };

    use super::*;
    use crate::win_fs::ensure_long_path;

    struct FindHandle(windows::Win32::Foundation::HANDLE);

    impl Drop for FindHandle {
        fn drop(&mut self) {
            unsafe {
                let _ = FindClose(self.0);
            }
        }
    }

    pub(super) fn read_windows_metadata(
        path: &Path,
        include_content: bool,
    ) -> Result<WindowsFileMetadata> {
        let attributes = file_attributes(path)? & WINDOWS_PRESERVED_ATTRIBUTE_MASK;
        let mut named_streams = enumerate_named_streams(path, include_content)?;
        named_streams.sort_by(|left, right| {
            left.name
                .to_lowercase()
                .cmp(&right.name.to_lowercase())
                .then_with(|| left.name.cmp(&right.name))
        });
        let metadata = WindowsFileMetadata {
            file_attributes: attributes,
            named_streams,
        };
        if include_content {
            validate_payload(Some(&metadata))?;
        } else {
            validate_manifest(Some(&metadata))?;
        }
        Ok(metadata)
    }

    fn file_attributes(path: &Path) -> Result<u32> {
        let wide = wide_path(path);
        let attributes = unsafe { GetFileAttributesW(PCWSTR(wide.as_ptr())) };
        if attributes == INVALID_FILE_ATTRIBUTES {
            return Err(std::io::Error::last_os_error())
                .with_context(|| format!("reading attributes for {}", path.display()));
        }
        Ok(attributes)
    }

    fn enumerate_named_streams(
        path: &Path,
        include_content: bool,
    ) -> Result<Vec<WindowsNamedStream>> {
        let wide = wide_path(path);
        let mut data = WIN32_FIND_STREAM_DATA::default();
        let handle = match unsafe {
            FindFirstStreamW(
                PCWSTR(wide.as_ptr()),
                FindStreamInfoStandard,
                (&mut data as *mut WIN32_FIND_STREAM_DATA).cast(),
                None,
            )
        } {
            Ok(handle) => FindHandle(handle),
            Err(error)
                if matches!(
                    win32_code(&error),
                    code if code == ERROR_HANDLE_EOF.0 || code == ERROR_INVALID_PARAMETER.0
                ) =>
            {
                return Ok(Vec::new())
            }
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("enumerating streams for {}", path.display()))
            }
        };

        let mut streams = Vec::new();
        let mut total_size = 0u64;
        loop {
            if let Some((name, size)) = parse_stream_data(&data)? {
                if streams.len() >= MAX_WINDOWS_NAMED_STREAMS {
                    bail!(
                        "{} has more than {} Windows named streams",
                        path.display(),
                        MAX_WINDOWS_NAMED_STREAMS
                    );
                }
                total_size = total_size
                    .checked_add(size)
                    .ok_or_else(|| eyre::eyre!("Windows named-stream size overflow"))?;
                if total_size > MAX_WINDOWS_STREAM_BYTES_PER_FILE {
                    bail!(
                        "Windows named-stream bytes on {} exceed per-file cap {}",
                        path.display(),
                        MAX_WINDOWS_STREAM_BYTES_PER_FILE
                    );
                }
                streams.push(read_named_stream(path, name, size, include_content)?);
            }
            data = WIN32_FIND_STREAM_DATA::default();
            match unsafe {
                FindNextStreamW(handle.0, (&mut data as *mut WIN32_FIND_STREAM_DATA).cast())
            } {
                Ok(()) => {}
                Err(error) if win32_code(&error) == ERROR_HANDLE_EOF.0 => break,
                Err(error) => {
                    return Err(error)
                        .with_context(|| format!("enumerating streams for {}", path.display()))
                }
            }
        }
        Ok(streams)
    }

    fn parse_stream_data(data: &WIN32_FIND_STREAM_DATA) -> Result<Option<(String, u64)>> {
        let end = data
            .cStreamName
            .iter()
            .position(|unit| *unit == 0)
            .unwrap_or(data.cStreamName.len());
        let full = String::from_utf16(&data.cStreamName[..end])
            .context("Windows stream name is not valid Unicode")?;
        if full == "::$DATA" {
            return Ok(None);
        }
        let name = full
            .strip_prefix(':')
            .and_then(|value| value.strip_suffix(":$DATA"))
            .ok_or_else(|| eyre::eyre!("unsupported Windows stream type {full:?}"))?;
        validate_stream_name(name)?;
        let size: u64 = data
            .StreamSize
            .try_into()
            .map_err(|_| eyre::eyre!("Windows named stream {name:?} has negative size"))?;
        Ok(Some((name.to_owned(), size)))
    }

    fn read_named_stream(
        path: &Path,
        name: String,
        size: u64,
        include_content: bool,
    ) -> Result<WindowsNamedStream> {
        if size > MAX_WINDOWS_STREAM_BYTES_PER_FILE {
            bail!(
                "Windows named stream {:?} on {} is {} bytes, exceeds cap {}",
                name,
                path.display(),
                size,
                MAX_WINDOWS_STREAM_BYTES_PER_FILE
            );
        }
        let stream_path = named_stream_path(path, &name);
        let mut file = std::fs::File::open(&stream_path).with_context(|| {
            format!(
                "opening Windows named stream {:?} on {}",
                name,
                path.display()
            )
        })?;
        let mut content = Vec::new();
        content
            .try_reserve_exact(size as usize)
            .with_context(|| format!("allocating Windows named stream {:?}", name))?;
        file.by_ref()
            .take(size.saturating_add(1))
            .read_to_end(&mut content)
            .with_context(|| {
                format!(
                    "reading Windows named stream {:?} on {}",
                    name,
                    path.display()
                )
            })?;
        if content.len() as u64 != size {
            bail!(
                "Windows named stream {:?} on {} changed size while reading: expected {}, got {}",
                name,
                path.display(),
                size,
                content.len()
            );
        }
        let checksum = blake3::hash(&content).as_bytes().to_vec();
        if !include_content {
            content.clear();
        }
        Ok(WindowsNamedStream {
            name,
            size,
            checksum,
            content,
        })
    }

    pub(super) fn replace_streams_impl(path: &Path, metadata: &WindowsFileMetadata) -> Result<u64> {
        let current = enumerate_named_streams(path, false)?;
        let desired: HashSet<String> = metadata
            .named_streams
            .iter()
            .map(|stream| stream.name.to_lowercase())
            .collect();
        for stream in current {
            if !desired.contains(&stream.name.to_lowercase()) {
                std::fs::remove_file(named_stream_path(path, &stream.name)).with_context(|| {
                    format!(
                        "deleting stale Windows named stream {:?} on {}",
                        stream.name,
                        path.display()
                    )
                })?;
            }
        }
        for stream in &metadata.named_streams {
            std::fs::write(named_stream_path(path, &stream.name), &stream.content).with_context(
                || {
                    format!(
                        "writing Windows named stream {:?} on {}",
                        stream.name,
                        path.display()
                    )
                },
            )?;
        }
        Ok(metadata
            .named_streams
            .iter()
            .map(|stream| stream.size)
            .sum())
    }

    pub(super) fn apply_attributes_impl(path: &Path, metadata: &WindowsFileMetadata) -> Result<()> {
        let wide = wide_path(path);
        set_and_verify_attributes(
            path,
            metadata.file_attributes,
            |desired| {
                let attributes = if desired == 0 {
                    FILE_ATTRIBUTE_NORMAL
                } else {
                    FILE_FLAGS_AND_ATTRIBUTES(desired)
                };
                unsafe { SetFileAttributesW(PCWSTR(wide.as_ptr()), attributes) }
                    .with_context(|| format!("setting Windows attributes on {}", path.display()))
            },
            || file_attributes(path),
        )
    }

    pub(super) fn prepare_destination_impl(path: &Path) -> Result<()> {
        let current = file_attributes(path)?;
        if current & FILE_ATTRIBUTE_READONLY.0 == 0 {
            return Ok(());
        }
        let cleared = current & WINDOWS_SET_FILE_ATTRIBUTES_API_MASK & !FILE_ATTRIBUTE_READONLY.0;
        let attributes = if cleared == 0 {
            FILE_ATTRIBUTE_NORMAL
        } else {
            FILE_FLAGS_AND_ATTRIBUTES(cleared)
        };
        let wide = wide_path(path);
        unsafe { SetFileAttributesW(PCWSTR(wide.as_ptr()), attributes) }
            .with_context(|| format!("clearing read-only attribute on {}", path.display()))
    }

    fn named_stream_path(path: &Path, name: &str) -> std::path::PathBuf {
        let mut value: OsString = path.as_os_str().to_owned();
        value.push(OsStr::new(":"));
        value.push(OsStr::new(name));
        value.into()
    }

    fn wide_path(path: &Path) -> Vec<u16> {
        ensure_long_path(path)
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    fn win32_code(error: &windows::core::Error) -> u32 {
        error.code().0 as u32 & 0xffff
    }
}

#[cfg(windows)]
use windows_io::{
    apply_attributes_impl, prepare_destination_impl, read_windows_metadata, replace_streams_impl,
};

#[cfg(test)]
mod tests {
    use super::*;

    fn stream(name: &str, content: &[u8]) -> WindowsNamedStream {
        WindowsNamedStream {
            name: name.to_owned(),
            size: content.len() as u64,
            checksum: blake3::hash(content).as_bytes().to_vec(),
            content: content.to_vec(),
        }
    }

    #[test]
    fn manifest_and_payload_shapes_are_distinct_and_match() {
        let payload = WindowsFileMetadata {
            file_attributes: 0x23,
            named_streams: vec![stream("meta", b"payload")],
        };
        let mut manifest = payload.clone();
        manifest.named_streams[0].content.clear();
        validate_manifest(Some(&manifest)).unwrap();
        validate_payload(Some(&payload)).unwrap();
        validate_payload_against_manifest(Some(&payload), Some(&manifest)).unwrap();
        assert!(validate_manifest(Some(&payload)).is_err());
        assert!(validate_payload(Some(&manifest)).is_err());
    }

    #[test]
    fn rejects_unsafe_and_case_folded_duplicate_names() {
        for name in [
            "",
            ".",
            "..",
            "bad:name",
            "bad/name",
            "bad\\name",
            "bad\0name",
        ] {
            assert!(validate_stream_name(name).is_err(), "accepted {name:?}");
        }
        let metadata = WindowsFileMetadata {
            file_attributes: 0,
            named_streams: vec![stream("Meta", b"a"), stream("meta", b"b")],
        };
        assert!(validate_payload(Some(&metadata)).is_err());
    }

    #[test]
    fn rejects_count_size_attribute_and_checksum_overruns() {
        let too_many = WindowsFileMetadata {
            file_attributes: 0,
            named_streams: (0..=MAX_WINDOWS_NAMED_STREAMS)
                .map(|index| stream(&format!("s{index:03}"), b""))
                .collect(),
        };
        assert!(validate_payload(Some(&too_many)).is_err());

        let mut oversized = stream("meta", b"");
        oversized.size = MAX_WINDOWS_STREAM_BYTES_PER_FILE + 1;
        let oversized = WindowsFileMetadata {
            file_attributes: 0,
            named_streams: vec![oversized],
        };
        assert!(validate_payload(Some(&oversized)).is_err());

        let unsupported = WindowsFileMetadata {
            file_attributes: 0x8000_0000,
            named_streams: vec![],
        };
        assert!(validate_manifest(Some(&unsupported)).is_err());

        let aggregate = WindowsFileMetadata {
            file_attributes: 0,
            named_streams: vec![
                WindowsNamedStream {
                    name: "a".into(),
                    size: MAX_WINDOWS_STREAM_BYTES_PER_FILE / 2 + 1,
                    checksum: vec![0; blake3::OUT_LEN],
                    content: vec![],
                },
                WindowsNamedStream {
                    name: "b".into(),
                    size: MAX_WINDOWS_STREAM_BYTES_PER_FILE / 2 + 1,
                    checksum: vec![0; blake3::OUT_LEN],
                    content: vec![],
                },
            ],
        };
        assert!(validate_manifest(Some(&aggregate)).is_err());

        let mut bad_hash = stream("meta", b"payload");
        bad_hash.checksum[0] ^= 1;
        let bad_hash = WindowsFileMetadata {
            file_attributes: 0,
            named_streams: vec![bad_hash],
        };
        assert!(validate_payload(Some(&bad_hash)).is_err());
    }

    #[test]
    fn attribute_contract_is_durable_and_requires_readback_convergence() {
        let durable = WindowsFileMetadata {
            file_attributes: WINDOWS_PRESERVED_ATTRIBUTE_MASK,
            named_streams: Vec::new(),
        };
        validate_manifest(Some(&durable)).expect("all four durable bits are supported");

        for volatile in [0x0000_0100, 0x0000_1000, 0x0000_2000] {
            let metadata = WindowsFileMetadata {
                file_attributes: volatile,
                named_streams: Vec::new(),
            };
            assert!(
                validate_manifest(Some(&metadata)).is_err(),
                "volatile attribute 0x{volatile:08x} entered the durable contract"
            );
        }

        let requested = 0x0000_0023;
        let mut set_value = None;
        let error = set_and_verify_attributes(
            Path::new("destination.bin"),
            requested,
            |attributes| {
                set_value = Some(attributes);
                Ok(())
            },
            // Simulate a filesystem that accepts the call but drops HIDDEN.
            || Ok(requested & !0x2),
        )
        .expect_err("successful setter without durable readback must fail");
        assert_eq!(set_value, Some(requested));
        assert!(format!("{error:#}").contains("did not converge"));

        set_and_verify_attributes(
            Path::new("destination.bin"),
            requested,
            |_| Ok(()),
            // Non-contract attributes do not affect convergence.
            || Ok(requested | 0x0000_2100),
        )
        .unwrap();
    }

    #[test]
    fn payload_change_from_manifest_is_rejected() {
        let mut manifest = WindowsFileMetadata {
            file_attributes: 0x20,
            named_streams: vec![stream("meta", b"old")],
        };
        manifest.named_streams[0].content.clear();
        let payload = WindowsFileMetadata {
            file_attributes: 0x20,
            named_streams: vec![stream("meta", b"new")],
        };
        assert!(validate_payload_against_manifest(Some(&payload), Some(&manifest)).is_err());
    }

    #[cfg(not(windows))]
    #[test]
    fn non_windows_destination_refuses_metadata_before_creating_a_file() {
        let temp = tempfile::tempdir().unwrap();
        let destination = temp.path().join("missing.bin");
        let metadata = WindowsFileMetadata {
            file_attributes: 0,
            named_streams: vec![],
        };
        assert!(prepare_destination(&destination, Some(&metadata)).is_err());
        assert!(!destination.exists());
    }
}
