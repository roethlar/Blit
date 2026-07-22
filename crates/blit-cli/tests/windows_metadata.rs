#![cfg(windows)]

//! rel-4 end-to-end guard: carrier selection must not change Windows
//! attributes or named `$DATA` stream fidelity. Fixtures are 32 one-byte files
//! plus tiny ADS content; this is a correctness probe, not a write benchmark.

use std::ffi::{OsStr, OsString};
use std::fs;
use std::os::windows::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

mod common;
use common::{cli_bin, run_with_timeout, TestContext};

const REQUIRED_ATTRIBUTES: u32 = 0x1 | 0x2 | 0x4 | 0x20; // READONLY | HIDDEN | SYSTEM | ARCHIVE
const ADS_CONTENT: &[u8] = b"rel-4-ads";

fn named_stream_path(path: &Path, name: &str) -> PathBuf {
    let mut value: OsString = path.as_os_str().to_owned();
    value.push(OsStr::new(":"));
    value.push(OsStr::new(name));
    value.into()
}

fn set_attributes(path: &Path, enabled: bool) {
    let switch = if enabled { "+" } else { "-" };
    let status = Command::new("attrib")
        .arg(format!("{switch}R"))
        .arg(format!("{switch}H"))
        .arg(format!("{switch}S"))
        .arg(format!("{switch}A"))
        .arg(path)
        .status()
        .expect("run attrib");
    assert!(status.success(), "attrib failed for {}", path.display());
}

fn create_metadata_file(path: &Path) {
    fs::write(path, b"x").expect("write primary stream");
    fs::write(named_stream_path(path, "meta"), ADS_CONTENT).expect("write ADS");
    set_attributes(path, true);
}

fn assert_metadata(path: &Path) {
    assert_eq!(fs::read(path).expect("read primary stream"), b"x");
    assert_eq!(
        fs::read(named_stream_path(path, "meta")).expect("read ADS"),
        ADS_CONTENT
    );
    let attributes = fs::metadata(path).expect("metadata").file_attributes();
    assert_eq!(
        attributes & REQUIRED_ATTRIBUTES,
        REQUIRED_ATTRIBUTES,
        "durable attributes missing on {}: 0x{attributes:08x}",
        path.display()
    );
}

fn erase_metadata_but_match_mtime(source: &Path, destination: &Path) {
    set_attributes(destination, false);
    fs::remove_file(named_stream_path(destination, "meta")).expect("remove destination ADS");
    let source_time = filetime::FileTime::from_last_modification_time(
        &fs::metadata(source).expect("source metadata"),
    );
    filetime::set_file_mtime(destination, source_time).expect("match destination mtime");
}

fn add_oversized_stale_stream_but_match_mtime(source: &Path, destination: &Path) {
    set_attributes(destination, false);
    let stale = fs::File::create(named_stream_path(destination, "stale"))
        .expect("create stale destination ADS");
    stale
        .set_len(2 * 1024 * 1024 + 1)
        .expect("make stale destination ADS exceed the contract cap");
    let source_time = filetime::FileTime::from_last_modification_time(
        &fs::metadata(source).expect("source metadata"),
    );
    filetime::set_file_mtime(destination, source_time).expect("match destination mtime");
}

fn create_batch(root: &Path) -> PathBuf {
    fs::create_dir_all(root).expect("create source batch");
    let metadata_file = root.join("f00.bin");
    create_metadata_file(&metadata_file);
    for index in 1..32 {
        fs::write(root.join(format!("f{index:02}.bin")), b"x").expect("write batch member");
    }
    metadata_file
}

fn run_local_copy(src: &Path, dst: &Path) {
    let mut command = Command::new(cli_bin());
    command.arg("copy").arg("--yes").arg(src).arg(dst);
    let output = run_with_timeout(command, Duration::from_secs(30));
    assert!(
        output.status.success(),
        "local copy failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn local_single_and_tar_batch_preserve_attributes_and_ads() {
    let temp = tempfile::tempdir().expect("tempdir");

    let single = temp.path().join("single.bin");
    let single_dest = temp.path().join("single-dest.bin");
    create_metadata_file(&single);
    run_local_copy(&single, &single_dest);
    assert_metadata(&single_dest);
    erase_metadata_but_match_mtime(&single, &single_dest);
    run_local_copy(&single, &single_dest);
    assert_metadata(&single_dest);
    add_oversized_stale_stream_but_match_mtime(&single, &single_dest);
    run_local_copy(&single, &single_dest);
    assert_metadata(&single_dest);
    assert!(
        fs::metadata(named_stream_path(&single_dest, "stale")).is_err(),
        "oversized stale destination stream must be removed by replacement"
    );

    let batch = temp.path().join("batch");
    let batch_dest = temp.path().join("batch-dest");
    let batch_metadata_file = create_batch(&batch);
    let batch_arg = PathBuf::from(format!("{}\\", batch.display()));
    run_local_copy(&batch_arg, &batch_dest);
    assert_metadata(&batch_dest.join("f00.bin"));

    set_attributes(&single, false);
    set_attributes(&single_dest, false);
    set_attributes(&batch_metadata_file, false);
    set_attributes(&batch_dest.join("f00.bin"), false);
}

#[test]
fn remote_single_and_tar_batch_preserve_attributes_and_ads() {
    let ctx = TestContext::new();

    let single = ctx.workspace.join("single.bin");
    create_metadata_file(&single);
    let single_remote = format!("127.0.0.1:{}:/test/single.bin", ctx.daemon_port);
    let mut single_command = Command::new(&ctx.cli_bin);
    single_command
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("copy")
        .arg(&single)
        .arg(&single_remote);
    let single_output = run_with_timeout(single_command, Duration::from_secs(30));
    assert!(single_output.status.success(), "remote single copy failed");
    assert_metadata(&ctx.module_dir.join("single.bin"));
    erase_metadata_but_match_mtime(&single, &ctx.module_dir.join("single.bin"));
    let mut repair_command = Command::new(&ctx.cli_bin);
    repair_command
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("copy")
        .arg(&single)
        .arg(&single_remote);
    let repair_output = run_with_timeout(repair_command, Duration::from_secs(30));
    assert!(
        repair_output.status.success(),
        "remote metadata repair failed"
    );
    assert_metadata(&ctx.module_dir.join("single.bin"));

    let batch = ctx.workspace.join("batch");
    let batch_metadata_file = create_batch(&batch);
    let batch_remote = format!("127.0.0.1:{}:/test/batch/", ctx.daemon_port);
    let batch_arg = format!("{}\\", batch.display());
    let mut batch_command = Command::new(&ctx.cli_bin);
    batch_command
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("copy")
        .arg(batch_arg)
        .arg(batch_remote);
    let batch_output = run_with_timeout(batch_command, Duration::from_secs(30));
    assert!(
        batch_output.status.success(),
        "remote batch copy failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&batch_output.stdout),
        String::from_utf8_lossy(&batch_output.stderr)
    );
    assert_metadata(&ctx.module_dir.join("batch/f00.bin"));

    set_attributes(&single, false);
    set_attributes(&ctx.module_dir.join("single.bin"), false);
    set_attributes(&batch_metadata_file, false);
    set_attributes(&ctx.module_dir.join("batch/f00.bin"), false);
}
