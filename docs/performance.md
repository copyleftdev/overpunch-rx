# Performance measurements

Measured locally on 2026-11-11: AMD Ryzen Threadripper PRO 5975WX, x86_64 Linux,
Rust 1.99.0 / LLVM 23.1.1. Default target CPU, release optimization, overflow checks
enabled. No CPU affinity or frequency locking. This is a microbenchmark, not a
production workload, a worst-case bound, or a cross-machine performance guarantee.

| ASCII width | Decode | Encode into caller buffer |
| --- | ---: | ---: |
| 5 | 9.40 ns | 14.30 ns |
| 11 | 13.03 ns | 20.89 ns |
| 18 | 17.87 ns | 27.16 ns |
| 38 | 41.99 ns | 49.07 ns |

These are median timings for complete field operations, including validation.
Encoding reuses a caller-owned buffer and performs no heap allocation.

## Method

The separate `benchmarks` package records its dependencies in a lockfile.
Each operation warms up for 10,000 iterations, then takes
nine samples of 250,000 iterations. The harness reports minimum, median and
maximum time per operation. Inputs, metadata and outputs cross `black_box`
barriers. Result consumption is deliberate, and no result error is discarded.

Before timing, assertions confirm the expected numeric values and wire bytes.
All cases use scale two. The 38-digit case exercises the maximum field capacity.

Cases use fixed values with warm caches. Operation order is fixed, measurements
are not statistically paired, and no confidence intervals are computed. OS noise,
branch prediction and allocator state affect results. EBCDIC, invalid input,
mixed scales, schema construction and full claim processing are not timed here.
Re-run on your actual workload before using these numbers for a capacity plan.

## Why the core is fast

- Compile-time terminal lookup tables avoid a long character dispatch chain.
- Validated width bounds permit provably safe integer accumulation.
- Up to 19-digit chunks use `u64`; wide values need only a bounded combination/split.
- No UTF-8 decoding, floating point, heap allocation or general decimal arithmetic
  is needed on the codec path.
- Fixed schema parameters can be reused across records.

Formatting splits large coefficients into bounded chunks so repeated digit
extraction uses native-width arithmetic. A transactional scratch buffer ensures
that validation failures leave the caller's output unchanged.

Reproduce from the full checkout:

```sh
RUSTFLAGS='-D warnings' cargo bench --manifest-path benchmarks/Cargo.toml --bench compare --locked
```

See [raw benchmark output](evidence/benchmark.log). The benchmark source and
dependency lockfile are kept separately from the dependency-free core.
