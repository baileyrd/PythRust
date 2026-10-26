"""Times each benchmark natively and under CPython; stdlib only.

Usage: python3 bench/run.py [--runs N]

Builds every bench/*.pyr with target/release/pythrust (run
`cargo build --release && cargo build --release -p pythrust-rt` first),
checks the native output equals CPython's, and prints the median wall
time of each over N runs, with the fastest and slowest run.
"""

import argparse
import pathlib
import shutil
import statistics
import subprocess
import sys
import tempfile
import time

ROOT = pathlib.Path(__file__).resolve().parent.parent
PYTHRUST = ROOT / "target" / "release" / "pythrust"


def timed(cmd: list[str], runs: int) -> tuple[list[float], bytes]:
    """Seconds of each of `runs` runs, and the output, which must not vary."""
    times, outputs = [], set()
    for _ in range(runs):
        start = time.perf_counter()
        done = subprocess.run(cmd, capture_output=True, check=True)
        times.append(time.perf_counter() - start)
        outputs.add(done.stdout)
    if len(outputs) != 1:
        raise SystemExit(f"{cmd}: output varies between runs")
    return times, outputs.pop()


def spread(times: list[float]) -> str:
    """Median, with the range in parentheses."""
    return f"{statistics.median(times):.3f} ({min(times):.3f}-{max(times):.3f})"


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--runs", type=int, default=5)
    runs = parser.parse_args().runs
    head = f"{'benchmark':<16} {'native s (min-max)':>24} {'cpython s (min-max)':>25}"
    print(f"{head} {'speedup':>8}")
    with tempfile.TemporaryDirectory() as tmp:
        for src in sorted((ROOT / "bench").glob("*.pyr")):
            copy = pathlib.Path(tmp) / src.name
            shutil.copy(src, copy)
            subprocess.run([str(PYTHRUST), "build", str(copy)], check=True)
            native, out = timed([str(copy.with_suffix(""))], runs)
            cpython, expected = timed([sys.executable, str(src)], runs)
            if out != expected:
                raise SystemExit(f"{src.name}: native output differs from CPython")
            speedup = statistics.median(cpython) / statistics.median(native)
            print(f"{src.stem:<16} {spread(native):>24} {spread(cpython):>25} {speedup:>7.1f}x")


if __name__ == "__main__":
    main()
