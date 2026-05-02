//! Integration test for the F11 / R15-F1 pull-checksum ack flow.
//!
//! Spins up a daemon with `--no-server-checksums`, then runs
//! `blit copy server:/test/file.txt dest --checksum`. The pull-sync
//! handshake should bail with an ack-mismatch error rather than
//! silently degrading to size+mtime.

use std::fs;
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use serde::Serialize;
use tempfile::tempdir;
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
    #[serde(default)]
    use_chroot: bool,
}

fn pick_unused_port() -> u16 {
    TcpListener::bind(("127.0.0.1", 0))
        .expect("bind probe")
        .local_addr()
        .expect("listener addr")
        .port()
}

fn run_with_timeout(mut cmd: Command, timeout: Duration) -> std::process::Output {
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd.spawn().expect("spawn command");
    match child.wait_timeout(timeout).expect("wait_timeout") {
        Some(_) => child.wait_with_output().expect("collect output"),
        None => {
            let _ = child.kill();
            let output = child.wait_with_output().expect("collect output");
            panic!(
                "command timed out after {:?}\nstdout:\n{}\nstderr:\n{}",
                timeout,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
}

struct ChildGuard(Option<std::process::Child>);
impl Drop for ChildGuard {
    fn drop(&mut self) {
        if let Some(mut c) = self.0.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
    }
}

#[cfg(unix)]
#[test]
fn pull_checksum_rejected_when_daemon_disables_checksums() {
    // R15-F1 regression. Daemon advertises checksums disabled
    // via `--no-server-checksums`; a pull with `--checksum` must
    // bail at the ack rather than silently using size+mtime.
    let work = tempdir().expect("tempdir");
    let workspace = work.path();

    let module_dir = workspace.join("module");
    fs::create_dir_all(&module_dir).expect("module dir");
    fs::write(module_dir.join("payload.txt"), b"hello").expect("payload");

    let dest_dir = workspace.join("dest");
    fs::create_dir_all(&dest_dir).expect("dest dir");

    let config_dir = workspace.join("cli-config");
    fs::create_dir_all(&config_dir).expect("cli-config");

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
            use_chroot: false,
        }],
    };
    let config_path = workspace.join("blitd.toml");
    fs::write(&config_path, toml::to_string(&config).unwrap()).expect("write config");

    // Locate the binaries.
    let exe = std::env::current_exe().expect("current_exe");
    let bin_dir = exe
        .parent()
        .expect("test bin dir")
        .parent()
        .expect("deps parent")
        .to_path_buf();
    let cli_bin = bin_dir.join(if cfg!(windows) {
        "blit-cli.exe"
    } else {
        "blit-cli"
    });
    let daemon_bin = bin_dir.join(if cfg!(windows) {
        "blit-daemon.exe"
    } else {
        "blit-daemon"
    });
    let maybe_target = bin_dir
        .parent()
        .and_then(|p| p.file_name())
        .map(|c| c.to_string_lossy().to_string());
    let mut build = Command::new("cargo");
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root");
    build.current_dir(&workspace_root);
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
    let output = build.output().expect("invoke cargo build");
    assert!(
        output.status.success(),
        "cargo build blit-daemon failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Spawn daemon WITH --no-server-checksums.
    let daemon_child = Command::new(&daemon_bin)
        .arg("--config")
        .arg(&config_path)
        .arg("--bind")
        .arg("127.0.0.1")
        .arg("--port")
        .arg(port.to_string())
        .arg("--no-server-checksums")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn daemon");
    let _daemon = ChildGuard(Some(daemon_child));

    let mut ready = false;
    for _ in 0..50 {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            ready = true;
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
    assert!(ready, "daemon failed to listen on {port}");

    // Run `blit copy --checksum server:/test/payload.txt dest_dir`.
    let remote_src = format!("127.0.0.1:{port}:/test/payload.txt");
    let mut cli_cmd = Command::new(&cli_bin);
    cli_cmd
        .arg("--config-dir")
        .arg(&config_dir)
        .arg("copy")
        .arg("--yes")
        .arg("--checksum")
        .arg(&remote_src)
        .arg(&dest_dir);
    let output = run_with_timeout(cli_cmd, Duration::from_secs(60));

    assert!(
        !output.status.success(),
        "client should have refused --checksum against a daemon with checksums disabled"
    );
    let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
    assert!(
        stderr.contains("checksum") && stderr.contains("disabled"),
        "expected ack-mismatch error mentioning checksum/disabled, got stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Belt-and-suspenders: the file must NOT have been silently
    // copied via the size+mtime fallback. The handshake bailed
    // before any data flowed.
    assert!(
        !dest_dir.join("payload.txt").exists(),
        "no file should have been transferred when the handshake bailed"
    );
}

#[cfg(unix)]
#[test]
fn pull_checksum_succeeds_when_daemon_enables_checksums() {
    // Companion: same setup minus `--no-server-checksums`. The
    // pull should succeed and copy the file. Proves the
    // capability check doesn't false-positive when the daemon
    // does support checksums.
    let work = tempdir().expect("tempdir");
    let workspace = work.path();
    let module_dir = workspace.join("module");
    fs::create_dir_all(&module_dir).expect("module dir");
    fs::write(module_dir.join("payload.txt"), b"hello").expect("payload");

    let dest_dir = workspace.join("dest");
    fs::create_dir_all(&dest_dir).expect("dest dir");

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
            use_chroot: false,
        }],
    };
    let config_path = workspace.join("blitd.toml");
    fs::write(&config_path, toml::to_string(&config).unwrap()).expect("write config");

    let exe = std::env::current_exe().expect("current_exe");
    let bin_dir = exe
        .parent()
        .expect("test bin dir")
        .parent()
        .expect("deps parent")
        .to_path_buf();
    let cli_bin = bin_dir.join(if cfg!(windows) {
        "blit-cli.exe"
    } else {
        "blit-cli"
    });
    let daemon_bin = bin_dir.join(if cfg!(windows) {
        "blit-daemon.exe"
    } else {
        "blit-daemon"
    });

    // Spawn daemon WITHOUT --no-server-checksums.
    let daemon_child = Command::new(&daemon_bin)
        .arg("--config")
        .arg(&config_path)
        .arg("--bind")
        .arg("127.0.0.1")
        .arg("--port")
        .arg(port.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn daemon");
    let _daemon = ChildGuard(Some(daemon_child));

    let mut ready = false;
    for _ in 0..50 {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            ready = true;
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
    assert!(ready, "daemon failed to listen on {port}");

    let remote_src = format!("127.0.0.1:{port}:/test/payload.txt");
    let mut cli_cmd = Command::new(&cli_bin);
    cli_cmd
        .arg("--config-dir")
        .arg(&config_dir)
        .arg("copy")
        .arg("--yes")
        .arg("--checksum")
        .arg(&remote_src)
        .arg(&dest_dir);
    let output = run_with_timeout(cli_cmd, Duration::from_secs(60));

    if !output.status.success() {
        panic!(
            "checksum-enabled daemon pull should succeed, got:\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    assert!(dest_dir.join("payload.txt").exists());
    assert_eq!(fs::read(dest_dir.join("payload.txt")).unwrap(), b"hello");
}
