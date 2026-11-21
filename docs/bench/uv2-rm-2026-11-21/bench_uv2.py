"""UTILITY_VERBS_LOCAL_PARITY AC-7, slice 2: removing 200,000 empty files in 2,000 directories.

Usage: bench_uv2.py <work dir> <base release dir> <slice release dir>

For each run a fresh tree is made under the folder a daemon serves, then removed three ways, each timed
wall-clock from the command's start to its exit: the system `rm -rf`, the base build's `blit rm --yes`
(the request that deleted a tree without being told to), and the slice's `blit rm -rf`. Each build's own
daemon serves on loopback. The figure reported is the median of RUNS runs, with the best beside it.
"""
import os
import shutil
import socket
import statistics
import subprocess
import sys
import time

work, base, new = sys.argv[1], sys.argv[2], sys.argv[3]
RUNS = 5
ENV = dict(os.environ, LC_ALL="C.UTF-8")
os.makedirs(work, exist_ok=True)


def make_tree(root):
    for d in range(2000):
        folder = os.path.join(root, "t", f"d{d:04}")
        os.makedirs(folder)
        for f in range(100):
            open(os.path.join(folder, f"f{f:03}"), "w").close()


def free_port():
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


class Daemon:
    def __init__(self, release, name):
        self.port = free_port()
        self.module = os.path.join(work, f"{name}-module")
        shutil.rmtree(self.module, ignore_errors=True)
        os.makedirs(self.module)
        config = os.path.join(work, f"{name}.toml")
        with open(config, "w") as fh:
            fh.write(
                f'[daemon]\nbind = "127.0.0.1"\nport = {self.port}\nno_mdns = true\n\n'
                f'[[module]]\nname = "bench"\npath = "{self.module}"\nread_only = false\ndelegation_allowed = true\n'
            )
        self.proc = subprocess.Popen(
            [os.path.join(release, "blit-daemon"), "--config", config, "--bind", "127.0.0.1", "--port", str(self.port)],
            env=dict(ENV, STATE_DIRECTORY=os.path.join(work, f"{name}-state")),
            stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
        )
        self.blit = [os.path.join(release, "blit"), "--config-dir", os.path.join(work, f"{name}-cli")]
        deadline = time.time() + 20
        while time.time() < deadline:
            try:
                socket.create_connection(("127.0.0.1", self.port), timeout=0.2).close()
                return
            except OSError:
                time.sleep(0.05)
        raise SystemExit(f"{name}: daemon did not listen")

    def target(self):
        return f"127.0.0.1:{self.port}:/bench/t"

    def stop(self):
        self.proc.kill()
        self.proc.wait()


def timed(root, argv, cwd=None):
    took = []
    for _ in range(RUNS):
        make_tree(root)
        start = time.perf_counter()
        done = subprocess.run(argv, cwd=cwd, env=ENV, stdin=subprocess.DEVNULL, capture_output=True)
        took.append((time.perf_counter() - start) * 1000)
        left = os.path.exists(os.path.join(root, "t"))
        if done.returncode != 0 or left:
            return f"FAILED (exit {done.returncode}, tree left: {left}): {done.stderr.decode(errors='replace').strip()[:160]}"
    return f"{statistics.median(took):8.1f} ms median   {min(took):8.1f} ms best"


system_root = os.path.join(work, "system")
shutil.rmtree(system_root, ignore_errors=True)
os.makedirs(system_root)
version = lambda release: subprocess.run([os.path.join(release, "blit"), "--version"], capture_output=True, text=True).stdout.strip()
print(f"base:  {version(base)}\nslice: {version(new)}")
print(subprocess.run(["rm", "--version"], capture_output=True, text=True).stdout.splitlines()[0])
print(f"200,000 empty files in 2,000 directories, a fresh tree for every run; {RUNS} runs each")
print(f"{'rm -rf t  (the system command)':<52} {timed(system_root, ['rm', '-rf', 't'], cwd=system_root)}")
for label, release, options in [("base", base, ["--yes"]), ("slice", new, ["-rf"])]:
    daemon = Daemon(release, label)
    try:
        line = f"blit rm {' '.join(options)} HOST:/bench/t  ({label} build)"
        print(f"{line:<52} {timed(daemon.module, [*daemon.blit, 'rm', *options, daemon.target()])}")
    finally:
        daemon.stop()
