"""Times every bench/*.py under CPython and as Rust (py2rs build), best of 3.

    python3 bench/run.py [names...]

Prints a Markdown table; the outputs of both must be equal.
"""
import os
import subprocess
import sys
import tempfile
import time

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
PY = os.environ.get("PY2RS_PYTHON", "python3.14")
PY2RS = os.path.join(ROOT, "target", "release", "py2rs")
OUT = os.environ.get("BENCH_DIR", os.path.join(tempfile.gettempdir(), "py2rs-bench"))


def best(cmd: list[str]) -> tuple[float, bytes]:
    times = []
    out = b""
    for _ in range(3):
        t = time.perf_counter()
        out = subprocess.run(cmd, check=True, capture_output=True).stdout
        times.append(time.perf_counter() - t)
    return min(times), out


def main() -> None:
    names = sys.argv[1:] or sorted(f[:-3] for f in os.listdir(HERE) if f.endswith(".py") and f != "run.py")
    print("| program | CPython (s) | Rust (s) | speedup |")
    print("|---|---:|---:|---:|")
    for n in names:
        src = os.path.join(HERE, n + ".py")
        b = subprocess.run([PY2RS, "build", src, "-o", os.path.join(OUT, n)], capture_output=True, text=True)
        if b.returncode != 0:
            print(f"| {n} | | build failed | |")
            sys.stderr.write(b.stderr)
            continue
        binary = b.stdout.strip().splitlines()[-1]
        tp, op = best([PY, src])
        tr, orr = best([binary])
        same = "" if op == orr else " (OUTPUT DIFFERS)"
        print(f"| {n} | {tp:.3f} | {tr:.3f} | {tp / tr:.1f}x{same} |")


if __name__ == "__main__":
    main()
