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

struct DualDaemonContext {
    _work: tempfile::TempDir,
    workspace: PathBuf,
    daemon_a_port: u16,
    daemon_b_port: u16,
    _daemon_a: ChildGuard,
    _daemon_b: ChildGuard,
    cli_bin: PathBuf,
    config_dir: PathBuf,
    module_a_dir: PathBuf,
    module_b_dir: PathBuf,
}

impl DualDaemonContext {
    fn new() -> Self {
        let work = tempdir().expect("tempdir");
        let workspace = work.path().to_path_buf();

        let module_a_dir = workspace.join("module_a");
        fs::create_dir_all(&module_a_dir).expect("module a dir");
        
        let module_b_dir = workspace.join("module_b");
        fs::create_dir_all(&module_b_dir).expect("module b dir");

        let config_dir = workspace.join("cli-config");
        fs::create_dir_all(&config_dir).expect("cli config");

        let port_a = pick_unused_port();
        let port_b = pick_unused_port();
        // Ensure ports are different
        assert_ne!(port_a, port_b, "ports must be different");

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
        
        // Ensure daemon is built (shared step)
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
        let output = build.output().expect("invoke cargo build for blit-daemon");
        assert!(output.status.success(), "cargo build blit-daemon failed");

        let daemon_a = Self::spawn_daemon(&workspace, &daemon_bin, port_a, "daemon_a", &module_a_dir);
        let daemon_b = Self::spawn_daemon(&workspace, &daemon_bin, port_b, "daemon_b", &module_b_dir);

        Self {
            _work: work,
            workspace,
            daemon_a_port: port_a,
            daemon_b_port: port_b,
            _daemon_a: daemon_a,
            _daemon_b: daemon_b,
            cli_bin,
            config_dir,
            module_a_dir,
            module_b_dir,
        }
    }

    fn spawn_daemon(workspace: &PathBuf, bin: &PathBuf, port: u16, name: &str, module_path: &PathBuf) -> ChildGuard {
        let config = DaemonConfig {
            daemon: DaemonSection {
                bind: "127.0.0.1".into(),
                port,
                no_mdns: true,
            },
            modules: vec![ModuleSection {
                name: "test".into(),
                path: module_path.clone(),
                comment: None,
                read_only: false,
                use_chroot: false,
            }],
        };

        let config_path = workspace.join(format!("{}.toml", name));
        let toml = toml::to_string(&config).expect("serialize config");
        fs::write(&config_path, toml).expect("write config");

        let child = Command::new(bin)
            .arg("--config")
            .arg(&config_path)
            .arg("--bind")
            .arg("127.0.0.1")
            .arg("--port")
            .arg(port.to_string())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn daemon");
        
        let mut ready = false;
        for _ in 0..50 {
            if TcpStream::connect(("127.0.0.1", port)).is_ok() {
                ready = true;
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }
        assert!(ready, "daemon {} failed to listen on {}", name, port);

        ChildGuard::new(child)
    }
}

#[cfg(unix)]
#[test]
fn test_remote_to_remote_mirror() {
    let ctx = DualDaemonContext::new();
    
    // Setup source file in Daemon A
    let src_file = ctx.module_a_dir.join("remote_src.txt");
    fs::write(&src_file, b"remote-to-remote-payload").expect("write src file");

    // Construct remote endpoints
    // Daemon A (Source): 127.0.0.1:PORT_A:/test/
    // Daemon B (Dest): 127.0.0.1:PORT_B:/test/
    let src_remote = format!("127.0.0.1:{}:/test/", ctx.daemon_a_port);
    let dest_remote = format!("127.0.0.1:{}:/test/", ctx.daemon_b_port);

    let mut cli_cmd = Command::new(&ctx.cli_bin);
    cli_cmd
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("mirror")
        .arg("--yes") // Skip confirmation prompt
        .arg("--trace-data-plane") // Optional, to see logs
        .arg(&src_remote)
        .arg(&dest_remote);
    
    let output = run_with_timeout(cli_cmd, Duration::from_secs(60));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    println!("STDOUT:\n{}", stdout);
    println!("STDERR:\n{}", stderr);

    assert!(output.status.success(), "blit-cli failed with status: {}", output.status);

    // Verify file arrived at Daemon B
    let dest_file = ctx.module_b_dir.join("remote_src.txt");
    if !dest_file.exists() {
        println!("Destination file missing at {}", dest_file.display());
        println!("Listing contents of module_b_dir:");
        let _ = Command::new("ls").arg("-R").arg(&ctx.module_b_dir).status();
        panic!("destination file missing");
    }
    
    let bytes = fs::read(&dest_file).expect("read dest file");
    assert_eq!(bytes, b"remote-to-remote-payload");
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
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
