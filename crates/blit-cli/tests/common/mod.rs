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
pub struct DaemonConfig {
    pub daemon: DaemonSection,
    #[serde(rename = "module")]
    pub modules: Vec<ModuleSection>,
}

#[derive(Serialize)]
pub struct DaemonSection {
    pub bind: String,
    pub port: u16,
    pub no_mdns: bool,
}

#[derive(Serialize)]
pub struct ModuleSection {
    pub name: String,
    pub path: PathBuf,
    #[serde(default)]
    pub comment: Option<String>,
    #[serde(default)]
    pub read_only: bool,
    #[serde(default)]
    pub use_chroot: bool,
}

pub fn pick_unused_port() -> u16 {
    TcpListener::bind(("127.0.0.1", 0))
        .expect("bind probe listener")
        .local_addr()
        .expect("listener addr")
        .port()
}

pub struct TestContext {
    pub _work: tempfile::TempDir,
    pub workspace: PathBuf,
    pub daemon_port: u16,
    pub daemon: ChildGuard,
    pub cli_bin: PathBuf,
    pub config_dir: PathBuf,
    pub module_dir: PathBuf,
}

impl TestContext {
    pub fn new() -> Self {
        let work = tempdir().expect("tempdir");
        let workspace = work.path().to_path_buf();

        let module_dir = workspace.join("module");
        fs::create_dir_all(&module_dir).expect("module dir");

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

        // Ensure daemon is built
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

        let daemon_child = Command::new(&daemon_bin)
            .arg("--config")
            .arg(&config_path)
            .arg("--bind")
            .arg("127.0.0.1")
            .arg("--port")
            .arg(port.to_string())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped()) // Capture stderr for debugging
            .spawn()
            .expect("spawn daemon");
        let daemon = ChildGuard::new(daemon_child);

        let mut ready = false;
        for _ in 0..50 {
            if TcpStream::connect(("127.0.0.1", port)).is_ok() {
                ready = true;
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }
        assert!(ready, "daemon failed to listen on {port}");

        Self {
            _work: work,
            workspace,
            daemon_port: port,
            daemon,
            cli_bin,
            config_dir,
            module_dir,
        }
    }
}

pub fn run_with_timeout(mut cmd: Command, timeout: Duration) -> std::process::Output {
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

pub struct ChildGuard {
    pub child: Option<std::process::Child>,
}

impl ChildGuard {
    pub fn new(child: std::process::Child) -> Self {
        Self { child: Some(child) }
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
