"""`ls` of one directory holding 200,000 empty files, and of one holding 1,000: the system's, and each
build's through its own loopback daemon.

Usage: bench_uv5.py <where> <label=release dir>...   Best and median of RUNS, milliseconds.
Everything is made in a directory of this script's own inside <where> (`blit-bench-uv5`), which is all it
ever removes: <where> itself, and anything else in it, is left alone."""
import os
import shutil
import socket
import statistics
import subprocess
import sys
import time

work = os.path.join(sys.argv[1], "blit-bench-uv5")
builds = [arg.split("=", 1) for arg in sys.argv[2:]]
RUNS = 5
shutil.rmtree(work, ignore_errors=True)
module = os.path.join(work, "module")
os.makedirs(module)
for name, count in (("big", 200_000), ("small", 1_000)):
    at = os.path.join(module, name)
    os.makedirs(at)
    for f in range(count):
        os.close(os.open(os.path.join(at, f"f{f:06d}"), os.O_CREAT | os.O_WRONLY, 0o644))
os.sync()


def timed(argv, env=None, cwd=None, lines=None, may_fail=False):
    started = time.perf_counter()
    done = subprocess.run(argv, stdin=subprocess.DEVNULL, capture_output=True, env=env, cwd=cwd)
    took = (time.perf_counter() - started) * 1000
    if done.returncode != 0:
        if may_fail:
            return None, done.stderr.decode(errors="replace").strip().splitlines()[-1][:200]
        raise AssertionError((argv[:4], done.stderr.decode(errors="replace")[:400]))
    if lines is not None:
        assert done.stdout.count(b"\n") == lines, (argv[:6], done.stdout.count(b"\n"), lines)
    return took, None


def report(label, times):
    print(f"{label:46} best {min(times):7.1f}   median {statistics.median(times):7.1f}   ({', '.join(f'{t:.0f}' for t in times)})")


CASES = [
    ("big", [], 200_000),
    ("big", ["-l"], 200_001),
    ("small", [], 1_000),
    ("small", ["-l"], 1_001),
]
ls_env = dict(os.environ, LC_ALL="C.UTF-8")
for name, options, lines in CASES:
    label = f"system ls {' '.join(options)} {name}".replace("  ", " ")
    report(label, [timed(["ls"] + options + [name], ls_env, cwd=module, lines=lines)[0] for _ in range(RUNS)])

env = dict(os.environ, NO_COLOR="1")
for name in ("HOME", "XDG_CONFIG_HOME", "XDG_DATA_HOME", "XDG_STATE_HOME", "XDG_CACHE_HOME"):
    env[name] = os.path.join(work, "home")
os.makedirs(env["HOME"], exist_ok=True)
for label, release in builds:
    with socket.socket() as s:
        s.bind(("127.0.0.1", 0))
        port = s.getsockname()[1]
    cfg = os.path.join(work, f"{label}.toml")
    open(cfg, "w").write(
        f'[daemon]\nbind = "127.0.0.1"\nport = {port}\nno_mdns = true\n\n'
        f'[[module]]\nname = "bench"\npath = "{module}"\nread_only = true\n'
    )
    log = open(os.path.join(work, f"{label}-daemon.log"), "w")
    daemon = subprocess.Popen(
        [release + "/blit-daemon", "--config", cfg, "--bind", "127.0.0.1", "--port", str(port)],
        env=dict(os.environ, STATE_DIRECTORY=os.path.join(work, f"{label}-state")),
        stdin=subprocess.DEVNULL, stdout=log, stderr=log,
    )
    time.sleep(1.0)
    try:
        for name, options, lines in CASES:
            if label == "before":
                if options:
                    continue  # the build before this slice has one form, a table with a header
                lines += 1
            what = f"blit ls {' '.join(options)} {name} ({label})".replace("  ", " ")
            times = []
            for _ in range(RUNS):
                took, why = timed([release + "/blit", "ls"] + options + [f"127.0.0.1:{port}:/bench/{name}/"], env, lines=lines, may_fail=label == "before")
                if took is None:
                    print(f"{what:46} FAILED: {why}")
                    break
                times.append(took)
            if times:
                report(what, times)
    finally:
        daemon.terminate()
        daemon.wait(timeout=10)
        log.close()
print("the tree's filesystem:", subprocess.run(["stat", "-f", "-c", "%T", work], capture_output=True, text=True).stdout.strip())
shutil.rmtree(work, ignore_errors=True)
