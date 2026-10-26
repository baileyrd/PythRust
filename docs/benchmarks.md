# Native benchmarks (M4)

Four programs, each valid Python, in `bench/`. Each is compiled with `pythrust build` and timed against CPython running the same file. Run them with:

```
cargo build --release && cargo build --release -p pythrust-rt
python3 bench/run.py --runs 7
```

`run.py` fails if a native program's output differs from CPython's, and reports the median wall time, including process start.

## Results

Linux x86-64, 4-core Intel Xeon at 2.80 GHz (a shared cloud container, so expect a few percent of noise), CPython 3.13.17, release build, median of 7 runs:

| Benchmark | What it does | Native s | CPython s | Speedup |
|---|---|---:|---:|---:|
| `numeric` | Collatz step counts for 1..10⁶: int arithmetic, branches, calls | 0.648 | 12.157 | 18.8x |
| `sorting` | in-place quicksort of 300,000 ints, plus a sortedness check | 0.125 | 0.919 | 7.3x |
| `strings` | 10⁶ small strings built with `+` and `str()`, plus one growing line | 0.490 | 0.588 | 1.2x |
| `tuples` | digit counts and sums via a `div_mod` returning a tuple, unpacked: ~7×10⁶ tuples | 0.642 | 1.056 | 1.6x |
| `tuples_baseline` | the same results without tuples (control, not one of the four) | 0.054 | 1.018 | 18.9x |

## Reading them

- **Scalar code is fast.** `numeric` and `tuples_baseline` run about 19x faster than CPython: checked `i64` arithmetic in registers, against boxed ints and bytecode dispatch.
- **Lists of ints are fast enough.** `sorting` is 7x: each `xs[i]` is a call into the runtime (bounds check, slot load), not an inline load.
- **String building is at parity.** `strings` spends its time allocating: every `+` and `str()` makes a new string object (malloc, a reference count, a UTF-8 copy), as CPython does, and CPython's allocator is well tuned for exactly this. The growing line is not the cost: without it, native and CPython take 0.42 s and 0.45 s.
- **Boxed tuples cost about 12x.** `tuples` and `tuples_baseline` compute the same thing; the tuple version is 0.64 s against 0.054 s, so about 85 ns per tuple: allocate, push two slots, read two fields through the runtime, drop. That is the cost of Nano's accepted boxed-tuple layout (`native.md`), and it leaves tuple-heavy code only 1.6x faster than CPython. Returning small tuples in registers (unboxing, planned for M5) is the fix the numbers point at; the layout is in no ABI, so it can change without breaking anything.

The common thread: native code wins where values stay in registers and loses most of its edge wherever it allocates, because each allocation is an opaque runtime call. Escape analysis and unboxing (M5) are where the next speedups are.
