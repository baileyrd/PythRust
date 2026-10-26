"""Times each benchmark natively and under CPython; stdlib only.

Usage: python3 bench/run.py [--runs N]

Builds every bench/*.pyr with target/release/pythrust (run
`cargo build --release && cargo build --release -p pythrust-rt` first),
checks the native output equals CPython's, and prints the median wall
time of each over N runs.
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


def timed(cmd: list[str], runs: int) -> tuple[float, bytes]:
    """Median seconds over `runs`, and the output, which must not vary."""
    times, outputs = [], set()
    for _ in range(runs):
        start = time.perf_counter()
        done = subprocess.run(cmd, capture_output=True, check=True)
        times.append(time.perf_counter() - start)
        outputs.add(done.stdout)
    if len(outputs) != 1:
        raise SystemExit(f"{cmd}: output varies between runs")
    return statistics.median(times), outputs.pop()


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--runs", type=int, default=5)
    runs = parser.parse_args().runs
    print(f"{'benchmark':<16} {'native s':>9} {'cpython s':>10} {'speedup':>8}")
    with tempfile.TemporaryDirectory() as tmp:
        for src in sorted((ROOT / "bench").glob("*.pyr")):
            copy = pathlib.Path(tmp) / src.name
            shutil.copy(src, copy)
            subprocess.run([str(PYTHRUST), "build", str(copy)], check=True)
            native, out = timed([str(copy.with_suffix(""))], runs)
            cpython, expected = timed([sys.executable, str(src)], runs)
            if out != expected:
                raise SystemExit(f"{src.name}: native output differs from CPython")
            print(f"{src.stem:<16} {native:>9.3f} {cpython:>10.3f} {cpython / native:>7.1f}x")


if __name__ == "__main__":
    main()
