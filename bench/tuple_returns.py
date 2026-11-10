"""Reproduce #806 timings and assert tuple-allocation elimination/fallback.

Linux/GNU linker only; standard Python and cc, no production instrumentation.
Build both compiler/runtime pairs first. Example:
 python3.13 bench/tuple_returns.py --before /abs/base/pythrust \
   --after /abs/current/target/release/pythrust --runs 9 --output result.json
--verify-only checks allocation regressions without timing the large workloads.
"""
import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import statistics
import subprocess
import sys
import tempfile
import time

ROOT = Path(__file__).resolve().parent.parent


def run(cmd, env=None):
    done = subprocess.run(cmd, env=env, capture_output=True)
    if done.returncode:
        raise RuntimeError(f"{cmd}: {done.stderr.decode()} {done.stdout.decode()}")
    return done


def sample(cmd, count, env=None):
    times, outputs = [], set()
    for _ in range(count):
        start = time.perf_counter()
        done = run(cmd, env)
        times.append(time.perf_counter() - start)
        outputs.add(done.stdout)
    assert len(outputs) == 1, (cmd, outputs)
    return dict(median=statistics.median(times), minimum=min(times),
                maximum=max(times), samples=times), outputs.pop()


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--before", type=Path)
    ap.add_argument("--after", type=Path, required=True)
    ap.add_argument("--runs", type=int, default=9)
    ap.add_argument("--verify-only", action="store_true")
    ap.add_argument("--output", type=Path)
    args = ap.parse_args()
    if sys.platform != "linux":
        ap.error("allocation probe requires Linux/GNU ld --wrap")
    assert args.runs > 0
    cc = shutil.which("cc")
    assert cc
    compilers = {"after": args.after.resolve()}
    if args.before:
        compilers = {"before": args.before.resolve(), **compilers}
    result = dict(generated_at=datetime.now(timezone.utc).isoformat(),
        compilers={name: dict(path=str(path),
            compiler_sha256=hashlib.sha256(path.read_bytes()).hexdigest(),
            runtime_sha256=hashlib.sha256((path.parent / "libpythrust_rt.a").read_bytes()).hexdigest())
            for name, path in compilers.items()},
        environment=dict(platform=platform.platform(),
        python=sys.version, rustc=run(["rustc", "-Vv"]).stdout.decode(),
        cpu=Path("/proc/cpuinfo").read_text().split("model name", 1)[-1].splitlines()[0],
        affinity=sorted(os.sched_getaffinity(0)), runs=args.runs,
        cc=run([cc, "--version"]).stdout.decode().splitlines()[0]), cases={})
    pair = "def pair(a: int, b: int) -> tuple[int, int]:\n    return a, b\n"
    cases = {
        "eligible": (pair + "total = 0\nfor i in range(100):\n    a, b = pair(i, 1)\n    total += a + b\nprint(total)\n", 100, 0),
        "fallback": (pair + "total = 0\nfor i in range(100):\n    t = pair(i, 1)\n    total += t[0] + t[1]\nprint(total)\n", 100, 100),
        "mixed": (pair + "total = 0\nfor i in range(100):\n    a, b = pair(i, 1)\n    t = pair(i, 1)\n    total += a + b + t[0] + t[1]\nprint(total)\n", 200, 100),
        "paths": ("def pair(n: int) -> tuple[int, int]:\n    text = str(n)\n    if n == 0:\n        return -9223372036854775808, 9223372036854775807\n    try:\n        return len(text), 10 // (n - 1)\n    except ZeroDivisionError:\n        return 100, 200\nfor i in range(100):\n    a, b = pair(i % 3)\nprint(a, b)\n", 100, 0),
        "startup": ("pass\n", 0, 0),
    }
    if not args.verify_only:
        cases.update(tuple_unpack=((ROOT / "bench/tuple_unpack.pyr").read_text(), 4000000, 0),
                     tuples=((ROOT / "bench/tuples.pyr").read_text(), 6888889, 0))
    with tempfile.TemporaryDirectory(prefix="pythrust-806-") as tmp:
        tmp = Path(tmp)
        obj = tmp / "counts.o"
        run([cc, "-O2", "-c", str(ROOT / "bench/tuple_allocations.c"), "-o", str(obj)])
        wrap = tmp / "linker"
        wrap.mkdir()
        # Driver resolves cc through PATH. The wrapper affects only the
        # separately built allocation probe, never timed executables.
        wrapper = wrap / "cc"
        wrapper.write_text("#!/usr/bin/env python3\nimport os, sys\nos.execv(" + repr(cc) +
            ", [" + repr(cc) + ", *sys.argv[1:], " + repr(str(obj)) +
            ", '-Wl,--wrap=pt_seq_new,--wrap=malloc,--wrap=calloc,--wrap=realloc'])\n")
        wrapper.chmod(0o700)
        for name, (source, before_count, after_count) in cases.items():
            expected = run([sys.executable, "-c", source]).stdout
            row = {"sha256": hashlib.sha256(source.encode()).hexdigest(),
                   "stdout": expected.decode(), "versions": {}}
            for version, compiler in compilers.items():
                work = tmp / (name + "-" + version)
                work.mkdir()
                src = work / "case.pyr"
                src.write_text(source)
                env = dict(os.environ, PYTHRUST_RT=str(compiler.parent / "libpythrust_rt.a"))
                build_times, _ = sample([str(compiler), "build", str(src)], args.runs, env)
                binary = src.with_suffix("")
                # Warm-up, then steady executable process samples. Startup
                # is measured separately; compile/link is excluded.
                assert run([str(binary)]).stdout == expected
                execution, stdout = sample([str(binary)], args.runs)
                assert stdout == expected
                probe_env = dict(env, PATH=str(wrap) + os.pathsep + os.environ["PATH"])
                run([str(compiler), "build", str(src)], probe_env)
                probe = run([str(binary)])
                assert probe.stdout == expected
                counts = dict((key, int(value)) for key, value in
                              (line.split("=") for line in probe.stderr.decode().splitlines()))
                allocation_count = counts["pair_tuple_allocations"]
                wanted = before_count if version == "before" else after_count
                assert allocation_count == wanted, (name, version, allocation_count, wanted)
                row["versions"][version] = dict(compile_link=build_times,
                    process_execution=execution, **counts)
            if "before" in row["versions"]:
                removed = row["versions"]["before"]["heap_allocations"] - row["versions"]["after"]["heap_allocations"]
                assert removed == 2 * (before_count - after_count), (name, removed)
            result["cases"][name] = row
    counts = {name: row["versions"]["after"]["heap_allocations"]
              for name, row in result["cases"].items()}
    assert counts["fallback"] - counts["eligible"] == 200, counts
    assert counts["mixed"] == counts["fallback"], counts
    rendered = json.dumps(result, indent=2) + "\n"
    if args.output:
        args.output.write_text(rendered)
    print(rendered)


if __name__ == "__main__":
    main()
