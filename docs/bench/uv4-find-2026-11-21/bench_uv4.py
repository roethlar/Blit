"""`find` over 200,000 empty files in 2,000 directories: the tests that ask each entry about itself, and
`-delete` — the system's, and each build's through its own loopback daemon.

Usage: bench_uv4.py <where> <label=release dir>...   Best and median of RUNS, milliseconds.
Everything is made in a directory of this script's own inside <where> (`blit-bench-uv4`), which is all it
ever removes: <where> itself, and anything else in it, is left alone (cr-uv4-6 — it used to remove the
directory it was given, whole, before it began)."""
import os
import shutil
import socket
import statistics
import subprocess
import sys
import time

work = os.path.join(sys.argv[1], "blit-bench-uv4")
builds = [arg.split("=", 1) for arg in sys.argv[2:]]
RUNS = 5
DIRS, FILES = 2000, 100
shutil.rmtree(work, ignore_errors=True)
module = os.path.join(work, "module")
os.makedirs(module)


def plant():
    tree = os.path.join(module, "tree")
    shutil.rmtree(tree, ignore_errors=True)
    for d in range(DIRS):
        at = os.path.join(tree, f"d{d:04d}")
        os.makedirs(at)
        for f in range(FILES):
            os.close(os.open(os.path.join(at, f"f{f:03d}"), os.O_CREAT | os.O_WRONLY, 0o644))
    os.sync()
    return tree


def timed(argv, env=None, cwd=None, lines=None):
    started = time.perf_counter()
    done = subprocess.run(argv, stdin=subprocess.DEVNULL, capture_output=True, env=env, cwd=cwd)
    took = (time.perf_counter() - started) * 1000
    assert done.returncode == 0, (argv[:4], done.stderr.decode(errors="replace")[:400])
    if lines is not None:
        assert done.stdout.count(b"\n") == lines, (argv[:6], done.stdout.count(b"\n"), lines)
    return took


def report(label, times):
    print(f"{label:44} best {min(times):7.1f}   median {statistics.median(times):7.1f}   ({', '.join(f'{t:.0f}' for t in times)})")


EVERY = DIRS * FILES + DIRS + 1
READS = [
    ("every path", [], EVERY),
    ("-size -1 (every entry asked)", ["-size", "-1"], DIRS * FILES),
    ("-type f -mtime -1 (files asked)", ["-type", "f", "-mtime", "-1"], DIRS * FILES),
    ("-name 'f00*' -size -1 (20,000 asked)", ["-name", "f00*", "-size", "-1"], DIRS * 10),
]

plant()
for label, expression, lines in READS:
    report(f"system find, {label}", [timed(["find", "tree"] + expression, cwd=module, lines=lines) for _ in range(RUNS)])
times = []
for _ in range(RUNS):
    tree = plant()
    times.append(timed(["find", "tree", "-delete"], cwd=module))
    assert not os.path.exists(tree)
report("system find -delete", times)
times = []
for _ in range(RUNS):
    tree = plant()
    times.append(timed(["find", "tree", "-name", "f00*", "-delete"], cwd=module))
report("system find -name 'f00*' -delete", times)

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
        f'[[module]]\nname = "bench"\npath = "{module}"\nread_only = false\n'
    )
    log = open(os.path.join(work, f"{label}-daemon.log"), "w")
    daemon = subprocess.Popen(
        [release + "/blit-daemon", "--config", cfg, "--bind", "127.0.0.1", "--port", str(port)],
        env=dict(os.environ, STATE_DIRECTORY=os.path.join(work, f"{label}-state")),
        stdin=subprocess.DEVNULL, stdout=log, stderr=log,
    )
    time.sleep(1.0)
    target = f"127.0.0.1:{port}:/bench/tree"
    try:
        plant()
        for name, expression, lines in READS:
            if label == "before" and expression and expression[0] != "-name" and name != "every path":
                continue  # the build before this slice has neither test
            if label == "before" and ("-size" in expression or "-mtime" in expression):
                continue
            report(f"blit find ({label}), {name}", [timed([release + "/blit", "find", target] + expression, env, lines=lines) for _ in range(RUNS)])
        if label != "before":
            times = []
            for _ in range(RUNS):
                tree = plant()
                times.append(timed([release + "/blit", "find", target, "-delete", "-y"], env))
                assert not os.path.exists(tree)
            report(f"blit find ({label}) -delete", times)
            times = []
            for _ in range(RUNS):
                tree = plant()
                times.append(timed([release + "/blit", "find", target, "-name", "f00*", "-delete", "-y"], env))
            report(f"blit find ({label}) -name 'f00*' -delete", times)
            times = []
            for _ in range(RUNS):
                tree = plant()
                times.append(timed([release + "/blit", "rm", "-rf", target], env))
            report(f"blit rm -rf ({label}), for scale", times)
    finally:
        daemon.terminate()
        daemon.wait(timeout=10)
        log.close()
print("the tree's filesystem:", subprocess.run(["stat", "-f", "-c", "%T", work], capture_output=True, text=True).stdout.strip())
shutil.rmtree(work, ignore_errors=True)
