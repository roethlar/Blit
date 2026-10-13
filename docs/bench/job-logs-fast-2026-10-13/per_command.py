"""Per-command overhead: N interleaved runs of old and new copying one 4 KiB
file into a fresh destination, each binary with its own --config-dir."""
import os, shutil, statistics, subprocess, sys, time

old_bin, new_bin, root, runs = sys.argv[1], sys.argv[2], sys.argv[3], int(sys.argv[4])
os.makedirs(root, exist_ok=True)
src = os.path.join(root, "src")
os.makedirs(src, exist_ok=True)
with open(os.path.join(src, "f.bin"), "wb") as fh:
    fh.write(b"X" * 4096)
times = {"old": [], "new": []}
for run in range(1, runs + 1):
    for side, binary in (("old", old_bin), ("new", new_bin)):
        cfg = os.path.join(root, f"cfg_{side}")
        os.makedirs(cfg, exist_ok=True)
        dst = os.path.join(root, f"dst_{side}")
        shutil.rmtree(dst, ignore_errors=True)
        t0 = time.perf_counter_ns()
        done = subprocess.run([binary, "--config-dir", cfg, "copy", src + "/", dst + "/", "--yes"],
                              stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        ms = (time.perf_counter_ns() - t0) / 1e6
        if done.returncode != 0:
            sys.exit(f"{side} run {run} failed: {done.returncode}")
        times[side].append(ms)
        print(f"run={run} {side}={ms:.2f}ms", file=sys.stderr)
for side in ("old", "new"):
    print(f"{side} median={statistics.median(times[side]):.2f}ms "
          f"min={min(times[side]):.2f}ms max={max(times[side]):.2f}ms n={len(times[side])}")
print(f"delta median={statistics.median(times['new']) - statistics.median(times['old']):.2f}ms")
