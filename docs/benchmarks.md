# Native benchmarks (M4)

Four programs, each valid Python, in `bench/`. Each is compiled with `pythrust build` and timed against CPython running the same file. Run them with:

```
cargo build --release && cargo build --release -p pythrust-rt
python3 bench/run.py --runs 7
```

`run.py` fails if a native program's output differs from CPython's, and reports the median wall time, including process start.

## Results

Linux x86-64, 4-core Intel Xeon at 2.80 GHz in a shared cloud container, CPython 3.13.17, release build, 7 runs each. Times are the median with the fastest and slowest run; the ranges are wide (the container is shared), so read the speedups to about one significant figure.

| Benchmark | What it does | Native s | CPython s | Speedup |
|---|---|---:|---:|---:|
| `numeric` | Collatz step counts for 1..10⁶: int arithmetic, branches, calls | 0.637 (0.596–0.692) | 12.989 (12.957–14.516) | 20x |
| `sorting` | in-place quicksort of 300,000 ints, plus a sortedness check | 0.121 (0.112–0.190) | 0.996 (0.785–1.149) | 8x |
| `strings` | 10⁶ small strings built with `+` and `str()`, plus one growing line | 0.452 (0.405–0.496) | 0.586 (0.503–0.703) | 1.3x |
| `tuples` | digit counts and sums via `div_mod` returning a tuple, unpacked: about 6.9×10⁶ tuples | 0.617 (0.554–0.692) | 1.113 (0.962–1.225) | 1.8x |
| `tuples_baseline` | control for `tuples`, below; not one of the four | 0.091 (0.082–0.101) | 1.080 (0.974–1.280) | 12x |

## Reading them

- **Scalar code is fast.** `numeric` is about 20x faster than CPython: checked `i64` arithmetic in registers, against boxed ints and bytecode dispatch.
- **Lists of ints are fast enough.** `sorting` is about 8x: each `xs[i]` is a call into the runtime (bounds check, slot load), not an inline load.
- **String building is at parity.** `strings` spends its time allocating: every `+` and `str()` makes a new string object (malloc, a reference count, a UTF-8 copy), as CPython does, and CPython's allocator is well tuned for exactly this. The growing line is not the cost: without it, one measurement gave 0.42 s native and 0.45 s CPython.
- **Tuples are where native code loses most of its edge.** `tuples_baseline` does the same digit scan as `tuples`, with the same parameter divisor and a call per digit, but no tuple: `div_mod` becomes two calls (`quot`, `rem`), and `digits` packs its count and total into one int that the caller splits. The tuple version takes about 6.8x as long (0.617 s against 0.091 s). Most of that gap is plausibly the boxed tuple (allocate, push two slots, read two fields through the runtime, drop), but the programs still differ: the control makes one more call per digit, which favours the tuple version, and packs and unpacks an int per number, which favours neither much. So the gap is about 76 ns per tuple amortized over the whole program, not an isolated cost of the layout. A micro-benchmark of tuple construction alone would pin it down.

Where values stay in registers, native code is an order of magnitude faster; where it allocates, each allocation is an opaque runtime call and most of the advantage goes. Those runtime calls, and the boxed tuple layout, are what to look at when performance becomes the goal.
