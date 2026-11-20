"""Does a transfer of very many small files through a daemon meet h2's small-DATA-frame limit?

Usage: probe_transfer_small.py <tree> <scratch> <release dir> <out file>
Pushes the tree to a loopback daemon, pulls it back, and mirrors it again (no-op), saying how each ended.
"""
import os
import shutil
import socket
import subprocess
import sys
import time

tree, work, release, out_path = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4]
module = os.path.join(work, "xfer-module")
pulled = os.path.join(work, "xfer-pulled")
for path in (module, pulled):
    shutil.rmtree(path, ignore_errors=True)
os.makedirs(module)
with socket.socket() as s:
    s.bind(("127.0.0.1", 0))
    port = s.getsockname()[1]
cfg = os.path.join(work, "xfer.toml")
open(cfg, "w").write(
    f'[daemon]\nbind = "127.0.0.1"\nport = {port}\nno_mdns = true\n\n'
    f'[[module]]\nname = "bench"\npath = "{module}"\nread_only = false\ndelegation_allowed = true\n'
)
log_path = os.path.join(work, "xfer-daemon.log")
log = open(log_path, "w")
daemon = subprocess.Popen(
    [release + "/blit-daemon", "--config", cfg, "--bind", "127.0.0.1", "--port", str(port)],
    env=dict(os.environ, STATE_DIRECTORY=os.path.join(work, "xfer-state")),
    stdin=subprocess.DEVNULL, stdout=log, stderr=log,
)
time.sleep(1.0)
blit = [release + "/blit", "--config-dir", os.path.join(work, "xfer-cli")]
remote = f"127.0.0.1:{port}:/bench/t/"
lines = [f"{release.split('/')[-2]}: {tree}"]


def count(root):
    return sum(len(files) for _, _, files in os.walk(root))


try:
    for label, args, check in [
        ("push   copy tree/ HOST:/bench/t/", ["copy", tree + "/", remote], lambda: count(os.path.join(module, "t"))),
        ("mirror tree/ HOST:/bench/t/ (nothing to do)", ["mirror", "--yes", tree + "/", remote], lambda: count(os.path.join(module, "t"))),
        ("pull   copy HOST:/bench/t/ local/", ["copy", remote, pulled + "/"], lambda: count(pulled)),
    ]:
        t = time.perf_counter()
        p = subprocess.run(blit + args, stdin=subprocess.DEVNULL, capture_output=True, env=dict(os.environ, NO_COLOR="1"))
        ms = (time.perf_counter() - t) * 1000
        err = [line for line in p.stderr.decode(errors="replace").split("\n") if line.strip() and not line.startswith("starting")]
        lines.append(f"  {label}: exit {p.returncode}  {ms:9.1f} ms  files there {check()}  stderr: {' | '.join(err[:3])[:260]}")
finally:
    daemon.terminate()
    daemon.wait(timeout=20)
    log.close()
tail = open(log_path).read().strip().split("\n")[2:]
lines.append("  daemon said: " + " / ".join(tail[-4:])[:400])
open(out_path, "a").write("\n".join(lines) + "\n")
for path in (module, pulled):
    shutil.rmtree(path, ignore_errors=True)
