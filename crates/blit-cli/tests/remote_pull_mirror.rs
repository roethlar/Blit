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
        .expect("bind probe listener")
        .local_addr()
        .expect("listener addr")
        .port()
}

#[cfg(unix)]
#[test]
fn remote_pull_mirror_purges_extraneous_local_files() {
    let work = tempdir().expect("tempdir");
    let workspace = work.path();

    let module_dir = workspace.join("module");
    fs::create_dir_all(&module_dir).expect("module dir");
    fs::write(module_dir.join("server.txt"), b"from-server").expect("write server file");

    let dest_dir = workspace.join("dest");
    fs::create_dir_all(&dest_dir).expect("dest dir");
    fs::write(dest_dir.join("extra.txt"), b"stale").expect("write extra file");

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
    let toml = toml::to_string(&config).expect("serialize config");
    fs::write(&config_path, toml).expect("write config");

    let exe_path = std::env::current_exe().expect("current_exe");
    let deps_dir = exe_path.parent().expect("test binary directory");
    let bin_dir = deps_dir
        .parent()
        .expect("deps parent directory")
        .to_path_buf();

    let cli_bin = {
        let name = if cfg!(windows) {
            "blit-cli.exe"
        } else {
            "blit-cli"
        };
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

    let remote_src = format!("127.0.0.1:{}:/test/", port);
    let mut cli_cmd = Command::new(&cli_bin);
    cli_cmd
        .arg("--config-dir")
        .arg(&config_dir)
        .arg("mirror")
        .arg("--yes")
        .arg(&remote_src)
        .arg(&dest_dir);
    let output = run_with_timeout(cli_cmd, Duration::from_secs(120));

    daemon.terminate();

    if !output.status.success() {
        panic!(
            "blit-cli failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let server_file = dest_dir.join("server.txt");
    assert!(
        server_file.exists(),
        "expected server file copied to {}",
        server_file.display()
    );
    let bytes = fs::read(&server_file).expect("read server file");
    assert_eq!(bytes, b"from-server");

    assert!(
        !dest_dir.join("extra.txt").exists(),
        "extraneous local file should be purged"
    );
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

/// Spawn a daemon with the given module mapping and return the
/// child guard plus the chosen port. Builds the daemon binary if
/// necessary.
fn spawn_daemon(workspace: &std::path::Path, module_dir: &std::path::Path) -> (ChildGuard, u16) {
    let port = pick_unused_port();
    let config = DaemonConfig {
        daemon: DaemonSection {
            bind: "127.0.0.1".into(),
            port,
            no_mdns: true,
        },
        modules: vec![ModuleSection {
            name: "test".into(),
            path: module_dir.to_path_buf(),
            comment: None,
            read_only: false,
            use_chroot: false,
        }],
    };
    let config_path = workspace.join("blitd.toml");
    let toml = toml::to_string(&config).expect("serialize config");
    fs::write(&config_path, toml).expect("write config");

    let exe_path = std::env::current_exe().expect("current_exe");
    let deps_dir = exe_path.parent().expect("test binary directory");
    let bin_dir = deps_dir.parent().expect("deps parent").to_path_buf();
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

    let child = Command::new(&daemon_bin)
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
    let guard = ChildGuard::new(child);

    let mut ready = false;
    for _ in 0..50 {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            ready = true;
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
    assert!(ready, "daemon failed to listen on {port}");
    (guard, port)
}

fn cli_bin() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let bin_dir = exe
        .parent()
        .expect("deps")
        .parent()
        .expect("target")
        .to_path_buf();
    bin_dir.join(if cfg!(windows) {
        "blit-cli.exe"
    } else {
        "blit-cli"
    })
}

#[cfg(unix)]
#[test]
fn remote_pull_mirror_filtered_subset_preserves_out_of_scope_files() {
    // Closes F4: the daemon's filtered source manifest doesn't
    // include `*.tmp` files, but the client's dest has one. Under
    // FilteredSubset (default), out-of-scope files survive — only
    // in-scope client files absent from the source set are purged.
    let work = tempdir().expect("tempdir");
    let workspace = work.path();

    let module_dir = workspace.join("module");
    fs::create_dir_all(&module_dir).unwrap();
    fs::write(module_dir.join("keep.txt"), b"from-server").unwrap();
    fs::write(module_dir.join("server.tmp"), b"server tmp").unwrap();

    let dest_dir = workspace.join("dest");
    fs::create_dir_all(&dest_dir).unwrap();
    fs::write(dest_dir.join("dont-touch.tmp"), b"local tmp").unwrap();
    fs::write(dest_dir.join("extra.txt"), b"local extra").unwrap();

    let config_dir = workspace.join("cli-config");
    fs::create_dir_all(&config_dir).unwrap();

    let (mut daemon, port) = spawn_daemon(workspace, &module_dir);

    let remote_src = format!("127.0.0.1:{}:/test/", port);
    let mut cmd = Command::new(cli_bin());
    cmd.arg("--config-dir")
        .arg(&config_dir)
        .arg("mirror")
        .arg("--yes")
        .arg("--exclude")
        .arg("*.tmp")
        .arg(&remote_src)
        .arg(&dest_dir);
    let output = run_with_timeout(cmd, Duration::from_secs(120));
    daemon.terminate();
    if !output.status.success() {
        panic!(
            "mirror failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // In-scope file copied
    assert_eq!(fs::read(dest_dir.join("keep.txt")).unwrap(), b"from-server");
    // In-scope local file absent on filtered source: deleted
    assert!(
        !dest_dir.join("extra.txt").exists(),
        "in-scope extraneous file should be purged"
    );
    // Out-of-scope local file: preserved (the source filter excluded
    // *.tmp, so blit pretends to have no opinion about local *.tmp files)
    assert!(
        dest_dir.join("dont-touch.tmp").exists(),
        "out-of-scope local file must survive FilteredSubset mirror"
    );
    // Out-of-scope server file: never transferred
    assert!(!dest_dir.join("server.tmp").exists());
}

#[cfg(unix)]
#[test]
fn remote_pull_mirror_delete_scope_all_purges_out_of_scope() {
    // `--delete-scope all` switches to MirrorMode::All — every dest
    // file absent from the (filtered) source set is deleted, even
    // those outside the filter scope. Sharp tool, opt-in.
    let work = tempdir().expect("tempdir");
    let workspace = work.path();

    let module_dir = workspace.join("module");
    fs::create_dir_all(&module_dir).unwrap();
    fs::write(module_dir.join("keep.txt"), b"from-server").unwrap();

    let dest_dir = workspace.join("dest");
    fs::create_dir_all(&dest_dir).unwrap();
    fs::write(dest_dir.join("dont-touch.tmp"), b"local tmp").unwrap();
    fs::write(dest_dir.join("extra.txt"), b"local extra").unwrap();

    let config_dir = workspace.join("cli-config");
    fs::create_dir_all(&config_dir).unwrap();

    let (mut daemon, port) = spawn_daemon(workspace, &module_dir);

    let remote_src = format!("127.0.0.1:{}:/test/", port);
    let mut cmd = Command::new(cli_bin());
    cmd.arg("--config-dir")
        .arg(&config_dir)
        .arg("mirror")
        .arg("--yes")
        .arg("--exclude")
        .arg("*.tmp")
        .arg("--delete-scope")
        .arg("all")
        .arg(&remote_src)
        .arg(&dest_dir);
    let output = run_with_timeout(cmd, Duration::from_secs(120));
    daemon.terminate();
    if !output.status.success() {
        panic!(
            "mirror failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    assert_eq!(fs::read(dest_dir.join("keep.txt")).unwrap(), b"from-server");
    // Both extra and out-of-scope tmp deleted under MirrorMode::All
    assert!(!dest_dir.join("extra.txt").exists());
    assert!(!dest_dir.join("dont-touch.tmp").exists());
}
