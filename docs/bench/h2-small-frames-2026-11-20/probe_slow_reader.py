"""`blit <verb args> HOST:/bench/` with a reader that takes its output slowly: does the command still finish?

Usage: probe_slow_reader.py <tree> <scratch> <release dir> <out file> <stall seconds> <verb> [args...]
The reader takes nothing for <stall seconds>, then everything.
"""
import os
import socket
import subprocess
import sys
import time

tree, work, release, out_path, stall = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4], float(sys.argv[5])
verb_args = sys.argv[6:]
with socket.socket() as s:
    s.bind(("127.0.0.1", 0))
    port = s.getsockname()[1]
cfg = os.path.join(work, "probe.toml")
open(cfg, "w").write(
    f'[daemon]\nbind = "127.0.0.1"\nport = {port}\nno_mdns = true\n\n'
    f'[[module]]\nname = "bench"\npath = "{tree}"\nread_only = true\ndelegation_allowed = true\n'
)
log = open(os.path.join(work, "probe-daemon.log"), "w")
daemon = subprocess.Popen(
    [release + "/blit-daemon", "--config", cfg, "--bind", "127.0.0.1", "--port", str(port)],
    env=dict(os.environ, STATE_DIRECTORY=os.path.join(work, "probe-state")),
    stdin=subprocess.DEVNULL, stdout=log, stderr=log,
)
time.sleep(1.0)
lines = [f"{release.split('/')[-2]}: blit {' '.join(verb_args)} HOST:/bench/, reader stalls {stall}s"]
try:
    for i in range(3):
        t = time.perf_counter()
        p = subprocess.Popen(
            [release + "/blit", "--config-dir", os.path.join(work, "probe-cli"), *verb_args, f"127.0.0.1:{port}:/bench/"],
            stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.PIPE, env=dict(os.environ, NO_COLOR="1"),
        )
        time.sleep(stall)
        out, err = p.communicate()
        ms = (time.perf_counter() - t) * 1000
        said = " | ".join(err.decode(errors="replace").split("\n")[:3])
        lines.append(f"  run {i}: exit {p.returncode}  {ms:8.1f} ms  stdout lines {out.count(10)}  stderr: {said[:200]}")
finally:
    daemon.terminate()
    daemon.wait(timeout=10)
    log.close()
open(out_path, "a").write("\n".join(lines) + "\n")
