use std::fs;
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;
use tempfile::tempdir;

use serde::Serialize;
use wait_timeout::ChildExt;

#[derive(Serialize)]
struct DaemonConfig {
    daemon: DaemonSection,
    #[serde(rename = "module")]
    modules: Vec<ModuleSection>,
}

#[derive(Serialize)]
struct DaemonSection {
    bind: String,
    port: u16,
    no_mdns: bool,
}

#[derive(Serialize)]
struct ModuleSection {
    name: String,
    path: PathBuf,
    #[serde(default)]
    comment: Option<String>,
    #[serde(default)]
    read_only: bool,
}

fn pick_unused_port() -> u16 {
    TcpListener::bind(("127.0.0.1", 0))
        .expect("bind probe listener")
        .local_addr()
        .expect("listener addr")
        .port()
}

#[test]
fn remote_push_falls_back_to_grpc_when_forced() {
    let work = tempdir().expect("tempdir");
    let workspace = work.path();

    let module_dir = workspace.join("module");
    fs::create_dir_all(&module_dir).expect("module dir");

    let src_dir = workspace.join("src");
    fs::create_dir_all(&src_dir).expect("src dir");
    fs::write(src_dir.join("file.txt"), b"fallback-test").expect("write file");

    let config_dir = workspace.join("cli-config");
    fs::create_dir_all(&config_dir).expect("cli config");

    let port = pick_unused_port();

    let config = DaemonConfig {
        daemon: DaemonSection {
            bind: "127.0.0.1".into(),
            port,
            no_mdns: true,
        },
        modules: vec![ModuleSection {
            name: "test".into(),
            path: module_dir.clone(),
            comment: None,
            read_only: false,
        }],
    };

    let config_path = workspace.join("blitd.toml");
    let toml = toml::to_string(&config).expect("serialize config");
    fs::write(&config_path, toml).expect("write config");

    let exe_path = std::env::current_exe().expect("current_exe");
    let deps_dir = exe_path.parent().expect("test binary directory");
    let bin_dir = deps_dir
        .parent()
        .expect("deps parent directory")
        .to_path_buf();

    let cli_bin = {
        let name = if cfg!(windows) { "blit.exe" } else { "blit" };
        bin_dir.join(name)
    };
    let daemon_bin = {
        let name = if cfg!(windows) {
            "blit-daemon.exe"
        } else {
            "blit-daemon"
        };
        bin_dir.join(name)
    };
    let maybe_target = bin_dir
        .parent()
        .and_then(|p| p.file_name())
        .map(|component| component.to_string_lossy().to_string());

    let mut build = Command::new("cargo");
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root");
    build.current_dir(workspace_root);
    build
        .arg("build")
        .arg("-p")
        .arg("blit-daemon")
        .arg("--bin")
        .arg("blit-daemon");
    if let Some(triple) = maybe_target {
        if triple != "target" {
            build.arg("--target").arg(triple);
        }
    }
    let output = build.output().expect("invoke cargo build for blit-daemon");
    assert!(
        output.status.success(),
        "cargo build blit-daemon failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        daemon_bin.exists(),
        "expected daemon binary at {}",
        daemon_bin.display()
    );

    let daemon_child = Command::new(&daemon_bin)
        .arg("--config")
        .arg(&config_path)
        .arg("--force-grpc-data")
        .arg("--bind")
        .arg("127.0.0.1")
        .arg("--port")
        .arg(port.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn daemon");
    let mut daemon = ChildGuard::new(daemon_child);

    let mut ready = false;
    for _ in 0..50 {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            ready = true;
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
    assert!(ready, "daemon failed to listen on {port}");

    let dest_remote = format!("127.0.0.1:{}:/test/", port);
    // Trailing slash on source: merge contents into module root.
    let src_arg = format!("{}/", src_dir.display());
    let mut cli_cmd = Command::new(&cli_bin);
    cli_cmd
        .arg("--config-dir")
        .arg(&config_dir)
        .arg("mirror")
        .arg("--yes")
        .arg("--force-grpc")
        .arg(&src_arg)
        .arg(&dest_remote);
    let output = run_with_timeout(cli_cmd, Duration::from_secs(120));

    daemon.terminate();

    if !output.status.success() {
        panic!(
            "blit failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("gRPC data fallback") || stdout.contains("[gRPC fallback]"),
        "expected fallback message, got:\n{}",
        stdout
    );

    let dest_file = module_dir.join("file.txt");
    assert!(dest_file.exists(), "remote file missing");
    let bytes = fs::read(&dest_file).expect("read remote file");
    assert_eq!(bytes, b"fallback-test");
}

fn run_with_timeout(mut cmd: Command, timeout: Duration) -> std::process::Output {
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd.spawn().expect("spawn command");

    match child.wait_timeout(timeout).expect("wait for process") {
        Some(_status) => child
            .wait_with_output()
            .expect("collect command output after completion"),
        None => {
            let _ = child.kill();
            let output = child
                .wait_with_output()
                .expect("collect output after killing command");
            panic!(
                "command timed out after {:?}\nstdout:\n{}\nstderr:\n{}",
                timeout,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
}

struct ChildGuard {
    child: Option<std::process::Child>,
}

impl ChildGuard {
    fn new(child: std::process::Child) -> Self {
        Self { child: Some(child) }
    }

    fn terminate(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

// ---------------------------------------------------------------
// w4-2 regression net: the daemon used to queue every needs-upload
// manifest entry into a 262,144-slot channel that nothing read in
// gRPC-fallback mode, so manifest entry #262,145 wedged daemon and
// client forever with no timeout in scope
// (async-push-upload-channel-fallback-wedge). The channel is deleted;
// these tests pin that many-file forced-gRPC pushes complete.
// ---------------------------------------------------------------

/// Spawn a daemon, mirror `file_count` generated files with
/// --force-grpc, assert success, and return how many landed.
fn forced_grpc_mirror_file_count(file_count: usize, timeout: Duration) -> usize {
    let work = tempdir().expect("tempdir");
    let workspace = work.path();

    let module_dir = workspace.join("module");
    fs::create_dir_all(&module_dir).expect("module dir");
    let src_dir = workspace.join("src");
    fs::create_dir_all(&src_dir).expect("src dir");
    for idx in 0..file_count {
        // Shard into subdirs so no single directory holds 262k entries.
        let sub = src_dir.join(format!("d{}", idx / 1024));
        if idx % 1024 == 0 {
            fs::create_dir_all(&sub).expect("shard dir");
        }
        fs::write(sub.join(format!("f{idx}.txt")), b"x").expect("write src file");
    }

    let config_dir = workspace.join("cli-config");
    fs::create_dir_all(&config_dir).expect("cli config");

    let port = pick_unused_port();
    let config = DaemonConfig {
        daemon: DaemonSection {
            bind: "127.0.0.1".into(),
            port,
            no_mdns: true,
        },
        modules: vec![ModuleSection {
            name: "test".into(),
            path: module_dir.clone(),
            comment: None,
            read_only: false,
        }],
    };
    let config_path = workspace.join("blitd.toml");
    fs::write(
        &config_path,
        toml::to_string(&config).expect("serialize config"),
    )
    .expect("write config");

    let exe_path = std::env::current_exe().expect("current_exe");
    let bin_dir = exe_path
        .parent()
        .expect("deps dir")
        .parent()
        .expect("bin dir")
        .to_path_buf();
    let cli_bin = bin_dir.join(if cfg!(windows) { "blit.exe" } else { "blit" });
    let daemon_bin = bin_dir.join(if cfg!(windows) {
        "blit-daemon.exe"
    } else {
        "blit-daemon"
    });

    let daemon_child = Command::new(&daemon_bin)
        .arg("--config")
        .arg(&config_path)
        .arg("--force-grpc-data")
        .arg("--bind")
        .arg("127.0.0.1")
        .arg("--port")
        .arg(port.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn daemon");
    let mut daemon = ChildGuard::new(daemon_child);

    let mut ready = false;
    for _ in 0..50 {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            ready = true;
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
    assert!(ready, "daemon failed to listen on {port}");

    let dest_remote = format!("127.0.0.1:{port}:/test/");
    let src_arg = format!("{}/", src_dir.display());
    let mut cli_cmd = Command::new(&cli_bin);
    cli_cmd
        .arg("--config-dir")
        .arg(&config_dir)
        .arg("mirror")
        .arg("--yes")
        .arg("--force-grpc")
        .arg(&src_arg)
        .arg(&dest_remote);
    let output = run_with_timeout(cli_cmd, timeout);
    daemon.terminate();

    assert!(
        output.status.success(),
        "forced-gRPC mirror of {file_count} files failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    walkdir_count_files(&module_dir)
}

fn walkdir_count_files(root: &std::path::Path) -> usize {
    let mut count = 0;
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).expect("read module dir") {
            let entry = entry.expect("dir entry");
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                count += 1;
            }
        }
    }
    count
}

/// design-4 regression: 2000 files is well past the old failure cliff
/// (≥128 = FILE_LIST_EARLY_FLUSH_ENTRIES failed deterministically,
/// ~100 was timing-flaky). Pre-fix, both sides raced the daemon's
/// manifest loop: the daemon announced Negotiation(tcp_fallback) on
/// the mid-manifest early flush, and a force_grpc client started
/// streaming FileData on the first need-list batch without any
/// negotiation at all — either way the daemon's manifest loop
/// rejected the premature FileData and tore down the push. Fixed by
/// deferring the daemon's announcement to execute_grpc_fallback and
/// gating the client's fallback sends on fallback_negotiated.
#[test]
fn forced_grpc_push_many_files_completes() {
    let landed = forced_grpc_mirror_file_count(2_000, Duration::from_secs(120));
    assert_eq!(landed, 2_000, "every file must land via the gRPC fallback");
}

/// The exact pre-w4-2 wedge: >262,144 needs-upload entries in
/// gRPC-fallback mode. With design-4 fixed this is expected to pass;
/// it stays ignored only for runtime (~270k files, multi-minute).
/// Joint acceptance test for design-4 + w4-2. Run manually:
///   cargo test -p blit-cli --test remote_tcp_fallback -- --ignored
#[test]
#[ignore = "~270k files / multi-minute runtime; run manually"]
fn forced_grpc_push_overflows_old_upload_channel_capacity() {
    let landed = forced_grpc_mirror_file_count(270_000, Duration::from_secs(1800));
    assert_eq!(
        landed, 270_000,
        "pre-w4-2 this hung forever at entry 262,145"
    );
}
