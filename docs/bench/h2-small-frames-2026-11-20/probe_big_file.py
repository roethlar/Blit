"""Throughput check: push and pull one large file through a loopback daemon, best of N.

Usage: probe_big_file.py <scratch> <release dir> <out file> <size MiB> <runs>
"""
import os
import shutil
import socket
import subprocess
import sys
import time

work, release, out_path, size_mib, runs = sys.argv[1], sys.argv[2], sys.argv[3], int(sys.argv[4]), int(sys.argv[5])
src = os.path.join(work, "big-src")
os.makedirs(src, exist_ok=True)
big = os.path.join(src, "big.bin")
if not os.path.exists(big) or os.path.getsize(big) != size_mib << 20:
    block = os.urandom(1 << 20)
    with open(big, "wb") as fh:
        for _ in range(size_mib):
            fh.write(block)
module = os.path.join(work, "big-module")
back = os.path.join(work, "big-back")
with socket.socket() as s:
    s.bind(("127.0.0.1", 0))
    port = s.getsockname()[1]
cfg = os.path.join(work, "big.toml")
os.makedirs(module, exist_ok=True)
open(cfg, "w").write(
    f'[daemon]\nbind = "127.0.0.1"\nport = {port}\nno_mdns = true\n\n'
    f'[[module]]\nname = "bench"\npath = "{module}"\nread_only = false\ndelegation_allowed = true\n'
)
log = open(os.path.join(work, "big-daemon.log"), "w")
daemon = subprocess.Popen(
    [release + "/blit-daemon", "--config", cfg, "--bind", "127.0.0.1", "--port", str(port)],
    env=dict(os.environ, STATE_DIRECTORY=os.path.join(work, "big-state")),
    stdin=subprocess.DEVNULL, stdout=log, stderr=log,
)
time.sleep(1.0)
blit = [release + "/blit", "--config-dir", os.path.join(work, "big-cli")]
remote = f"127.0.0.1:{port}:/bench/t/"
best = {"push": None, "pull": None}
try:
    for _ in range(runs):
        shutil.rmtree(os.path.join(module, "t"), ignore_errors=True)
        shutil.rmtree(back, ignore_errors=True)
        for label, args in (("push", ["copy", src + "/", remote]), ("pull", ["copy", remote, back + "/"])):
            t = time.perf_counter()
            p = subprocess.run(blit + args, stdin=subprocess.DEVNULL, capture_output=True)
            ms = (time.perf_counter() - t) * 1000
            if p.returncode != 0:
                raise SystemExit(f"{label} failed: {p.stderr.decode(errors='replace')[:300]}")
            best[label] = ms if best[label] is None else min(best[label], ms)
finally:
    daemon.terminate()
    daemon.wait(timeout=20)
    log.close()
    shutil.rmtree(os.path.join(module, "t"), ignore_errors=True)
    shutil.rmtree(back, ignore_errors=True)
line = f"{release.split('/')[-3]}/{release.split('/')[-2]}: one {size_mib} MiB file, best of {runs}: push {best['push']:.0f} ms ({size_mib * 1000 / best['push']:.0f} MiB/s), pull {best['pull']:.0f} ms ({size_mib * 1000 / best['pull']:.0f} MiB/s)"
open(out_path, "a").write(line + "\n")
