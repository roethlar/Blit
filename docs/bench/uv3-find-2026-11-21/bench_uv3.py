"""UTILITY_VERBS_LOCAL_PARITY AC-7, slice 3: find over 200,000 empty files in 2,000 directories.

Usage: bench_uv3.py <work dir> <base release dir> <slice release dir>

The tree is made once under <work dir>/tree. Each build's own daemon serves it on loopback, read-only.
Every figure is the best of RUNS wall-clock runs, from the command's start to its exit, output read whole.
The base build's `find` takes no test of `find`'s; its one listing is set beside the system's.
"""
import os
import socket
import subprocess
import sys
import time

work, base, new = sys.argv[1], sys.argv[2], sys.argv[3]
RUNS = 5
tree = os.path.join(work, "tree")
if not os.path.isdir(tree):
    for d in range(2000):
        folder = os.path.join(tree, f"d{d:04}")
        os.makedirs(folder)
        for f in range(100):
            open(os.path.join(folder, f"f{f:03}"), "w").close()
ENV = dict(os.environ, LC_ALL="C.UTF-8")


def free_port():
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


def report(label, argv, cwd=None):
    best, out = None, b""
    for _ in range(RUNS):
        start = time.perf_counter()
        done = subprocess.run(argv, cwd=cwd, env=ENV, stdin=subprocess.DEVNULL, capture_output=True)
        took = (time.perf_counter() - start) * 1000
        if done.returncode != 0:
            print(f"{label:<62} FAILED: {done.stderr.decode(errors='replace').strip()[:160]}")
            return
        best = took if best is None else min(best, took)
        out = done.stdout
    print(f"{label:<62} {best:8.1f} ms   {out.count(bytes([10])):>7} lines   {len(out):>9} bytes")


class Daemon:
    def __init__(self, release, name):
        self.port = free_port()
        config = os.path.join(work, f"{name}.toml")
        with open(config, "w") as fh:
            fh.write(
                f'[daemon]\nbind = "127.0.0.1"\nport = {self.port}\nno_mdns = true\n\n'
                f'[[module]]\nname = "bench"\npath = "{tree}"\nread_only = true\ndelegation_allowed = true\n'
            )
        self.proc = subprocess.Popen(
            [os.path.join(release, "blit-daemon"), "--config", config, "--bind", "127.0.0.1", "--port", str(self.port)],
            env=dict(ENV, STATE_DIRECTORY=os.path.join(work, f"{name}-state")),
            stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
        )
        self.blit = [os.path.join(release, "blit"), "--config-dir", os.path.join(work, f"{name}-cli")]
        self.target = f"127.0.0.1:{self.port}:/bench/"
        deadline = time.time() + 20
        while time.time() < deadline:
            try:
                socket.create_connection(("127.0.0.1", self.port), timeout=0.2).close()
                return
            except OSError:
                time.sleep(0.05)
        raise SystemExit(f"{name}: daemon did not listen")

    def stop(self):
        self.proc.kill()
        self.proc.wait()


version = lambda release: subprocess.run([os.path.join(release, "blit"), "--version"], capture_output=True, text=True).stdout.strip()
print(f"base:  {version(base)}\nslice: {version(new)}")
print(subprocess.run(["find", "--version"], capture_output=True, text=True).stdout.splitlines()[0])
print(f"200,000 empty files in 2,000 directories; best of {RUNS}")
report("find .                       (the system command)", ["find", "."], cwd=tree)
report("find . -name 'f00*'          (the system command)", ["find", ".", "-name", "f00*"], cwd=tree)
report("find . -type d               (the system command)", ["find", ".", "-type", "d"], cwd=tree)
daemon = Daemon(base, "base")
try:
    report("blit find HOST:/bench/                       (base build)", [*daemon.blit, "find", daemon.target])
    report("blit find HOST:/bench/ --pattern 'f00*'      (base build)", [*daemon.blit, "find", daemon.target, "--pattern", "f00*"])
finally:
    daemon.stop()
daemon = Daemon(new, "slice")
try:
    report("blit find HOST:/bench/                       (slice build)", [*daemon.blit, "find", daemon.target])
    report("blit find HOST:/bench/ -name 'f00*'          (slice build)", [*daemon.blit, "find", daemon.target, "-name", "f00*"])
    report("blit find HOST:/bench/ -type d               (slice build)", [*daemon.blit, "find", daemon.target, "-type", "d"])
finally:
    daemon.stop()
