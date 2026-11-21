"""UTILITY_VERBS_LOCAL_PARITY AC-7, slice 1: du and df over 200,000 empty files in 2,000 directories.

Usage: bench_uv1.py <work dir> <base release dir> <slice release dir>
Each build's own daemon serves the tree on loopback; every figure is the best of RUNS wall-clock runs.
"""
import os
import shutil
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


def timed(argv, cwd=None):
    best, out = None, b""
    for _ in range(RUNS):
        start = time.perf_counter()
        done = subprocess.run(argv, cwd=cwd, env=ENV, stdin=subprocess.DEVNULL, capture_output=True)
        took = (time.perf_counter() - start) * 1000
        if done.returncode != 0:
            return None, done.stderr.decode(errors="replace").strip()[:200]
        best = took if best is None else min(best, took)
        out = done.stdout
    return best, out


def report(label, argv, cwd=None):
    took, out = timed(argv, cwd)
    if took is None:
        print(f"{label:<58} FAILED: {out}")
        return None
    lines = out.count(b"\n")
    print(f"{label:<58} {took:8.1f} ms   {lines:>7} lines   {len(out):>9} bytes")
    return out


class Daemon:
    def __init__(self, release, name):
        self.port = free_port()
        self.state = os.path.join(work, f"{name}-state")
        config = os.path.join(work, f"{name}.toml")
        with open(config, "w") as fh:
            fh.write(
                f'[daemon]\nbind = "127.0.0.1"\nport = {self.port}\nno_mdns = true\n\n'
                f'[[module]]\nname = "bench"\npath = "{tree}"\nread_only = true\ndelegation_allowed = true\n'
            )
        self.proc = subprocess.Popen(
            [os.path.join(release, "blit-daemon"), "--config", config, "--bind", "127.0.0.1", "--port", str(self.port)],
            env=dict(ENV, STATE_DIRECTORY=self.state),
            stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
        )
        self.blit = [os.path.join(release, "blit"), "--config-dir", os.path.join(work, f"{name}-cli")]
        deadline = time.time() + 20
        while time.time() < deadline:
            with socket.socket() as sock:
                if sock.connect_ex(("127.0.0.1", self.port)) == 0:
                    return
            time.sleep(0.05)
        raise SystemExit(f"{name} daemon did not start")

    def remote(self, below=""):
        return f"127.0.0.1:{self.port}:/bench/{below}"

    def stop(self):
        self.proc.terminate()
        self.proc.wait(timeout=10)


print(f"tree: {tree} (2,000 directories, 200,000 empty files); best of {RUNS}; LC_ALL=C.UTF-8")
print("--- the system commands, in the tree's parent")
local_s = report("du -s tree", ["du", "-s", "tree"], work)
local_d = report("du tree", ["du", "tree"], work)
local_a = report("du -a tree", ["du", "-a", "tree"], work)
report("df -k tree", ["df", "-k", "tree"], work)

print("--- blit at the base (before this slice)")
old = Daemon(base, "base")
try:
    report("blit du HOST:/bench/", old.blit + ["du", old.remote()])
    report("blit du --max-depth 1 HOST:/bench/", old.blit + ["du", "--max-depth", "1", old.remote()])
    report("blit df HOST:/bench/", old.blit + ["df", old.remote()])
finally:
    old.stop()

print("--- blit with this slice")
cur = Daemon(new, "slice")
try:
    for label, options, local in [("du -s", ["-s"], local_s), ("du", [], local_d), ("du -a", ["-a"], local_a)]:
        out = report(f"blit {label} HOST:/bench/", cur.blit + ["du", *options, cur.remote()])
        if out is not None and local is not None:
            same = out.replace(cur.remote().encode(), b"tree/").replace(b"tree//", b"tree/") == local.replace(b"\ttree\n", b"\ttree/\n")
            ours = sorted(line.split(b"\t")[0] for line in out.splitlines())
            theirs = sorted(line.split(b"\t")[0] for line in local.splitlines())
            print(f"    lines {out.count(10)} vs {local.count(10)}; sizes equal as multisets: {ours == theirs}; byte-identical after renaming the operand: {same}")
    report("blit df HOST:/bench/", cur.blit + ["df", cur.remote()])
finally:
    cur.stop()
for name in ("base-state", "slice-state", "base-cli", "slice-cli"):
    shutil.rmtree(os.path.join(work, name), ignore_errors=True)
