# Measured results — 2026-11-09

The predeclared 2× target against the strongest dedicated baseline was **not met**. The final portable and native runs retain all four failed primary cases, every other length/pattern, every ablation and every raw sample. These are exploratory performance measurements on a shared virtual machine, pinned to CPU 8. The `verified` directory name identifies verified source reproduction and outputs; strict performance acceptance still requires the dedicated fixed hardware.

## Exact source and environment

Both final runs use commit [`baf8f42af22f5dd30820f545ba07bf82d87ef561`](https://github.com/LIghtJUNction/z58/commit/baf8f42af22f5dd30820f545ba07bf82d87ef561). Each result directory contains an exact `source-snapshot.json` with 21 source/configuration files and SHA-256 hashes. Every file hash recorded by the benchmark was checked against that snapshot. Changes to later report text do not alter the measured source.

Intel Xeon Platinum 8573C, Linux 6.18.57 x86-64, Rust 1.99.1 (LLVM 23.1.1), GCC 14.2.0. Rust flags were empty for portable and `-C target-cpu=native` for native. The native build enables AVX2 in five8 and AVX-512 in Braid58. Each run contains 8,100 raw samples across 900 algorithm/operation/length/pattern groups. The predetermined corpus and timing method are in [README.md](README.md).

The portable run’s largest random 32/64-byte sample range was `braid58` 32-byte encode: 37.04–1584.41 ns/op (42.78× spread). Medians below retain all nine samples. The full minima, maxima and individual observations remain available; these shared-host ratios describe this run, not hardware-independent guarantees.
The native run’s largest random 32/64-byte sample range was `z58-fixed` 32-byte encode: 55.22–159.75 ns/op (2.89× spread). Medians below retain all nine samples. The full minima, maxima and individual observations remain available; these shared-host ratios describe this run, not hardware-independent guarantees.

## Fixed 32/64-byte primary cases

Times are median nanoseconds per operation. Speedup is fastest dedicated baseline time divided by z58 time; below 1 means z58 was slower.

| Profile | Bytes | Operation | z58 fixed | five8 | Braid58 | fd_bs58 | Strongest baseline / z58 | 2× target |
| --- | ---: | --- | ---: | ---: | ---: | ---: | ---: | --- |
| portable | 32 | encode | 49.38 | 85.32 | 41.68 | — | 0.84× | Miss |
| portable | 32 | decode | 39.42 | 79.64 | 44.50 | 159.16 | 1.13× | Miss |
| portable | 64 | encode | 126.22 | 199.97 | 125.13 | — | 0.99× | Miss |
| portable | 64 | decode | 79.75 | 256.29 | 94.26 | 338.29 | 1.18× | Miss |
| native | 32 | encode | 55.25 | 67.10 | 31.36 | — | 0.57× | Miss |
| native | 32 | decode | 36.98 | 54.18 | 26.02 | 76.67 | 0.70× | Miss |
| native | 64 | encode | 108.24 | 99.93 | 79.73 | — | 0.74× | Miss |
| native | 64 | decode | 99.90 | 127.67 | 32.05 | 180.72 | 0.32× | Miss |

Braid58’s native vector kernels are the strongest measured comparison. z58’s fixed conversion still evaluates scalar dot products and carry chains; the measurements identify vectorized conversion and scheduling as the next performance question. The dedicated-codec 2× goal remains open. No universal-speed or SIMD implementation claim follows from these results.

## Generic lengths, including short-input regressions

This table uses random inputs and compares the production generic API with the preallocated bs58 API. Fixed-width functions are excluded. All five data patterns remain in the complete CSV reports.

| Bytes | Portable encode | Portable decode | Native encode | Native decode |
| ---: | ---: | ---: | ---: | ---: |
| 0 | 0.17× | 0.57× | 0.64× | 0.47× |
| 1 | 0.42× | 0.50× | 0.46× | 0.63× |
| 2 | 0.81× | 0.81× | 0.64× | 0.78× |
| 8 | 2.56× | 1.78× | 2.32× | 1.53× |
| 16 | 4.06× | 2.22× | 3.77× | 2.20× |
| 32 | 9.75× | 1.77× | 7.08× | 3.95× |
| 64 | 18.31× | 7.03× | 10.91× | 6.74× |
| 128 | 25.51× | 11.53× | 20.65× | 12.14× |
| 256 | 24.71× | 17.52× | 26.23× | 13.97× |
| 1024 | 19.20× | 22.80× | 19.38× | 18.86× |
| 4096 | 24.87× | 20.72× | 17.87× | 20.01× |

Empty and very short calls can regress against bs58, where wrapper and capacity-check costs dominate useful work. Large random inputs benefit from grouped conversion, but the generic algorithm remains quadratic: each pass visits more limbs as values grow. Long-input rows are retained through 4,096 bytes; these measurements do not imply linear scaling.

## Independent optimization measurements

The following portable random-input ratios compare research routines under common wrappers. Production capacity-error handling is separately measured in `z58-generic`. The arithmetic-width column is elapsed-time overhead; the other columns are speedups.

| Bytes | Operation | Grouped / one-unit gain | u128 overhead at same groups | Larger groups within u128 gain |
| ---: | --- | ---: | ---: | ---: |
| 32 | encode | 2.16× | 2.41× time | 2.54× |
| 32 | decode | 1.94× | 1.83× time | 1.28× |
| 64 | encode | 2.27× | 2.62× time | 2.83× |
| 64 | decode | 2.47× | 1.92× time | 1.59× |
| 256 | encode | 3.10× | 3.52× time | 4.97× |
| 256 | decode | 3.69× | 2.10× time | 2.67× |
| 1024 | encode | 4.47× | 2.77× time | 3.63× |
| 1024 | decode | 4.80× | 2.08× time | 3.23× |
| 4096 | encode | 4.82× | 2.64× time | 3.73× |
| 4096 | decode | 5.43× | 2.56× time | 6.58× |

The experiments isolate group size, arithmetic width and fixed-size specialization. At unchanged group size, u128 arithmetic costs more than u64 in these samples. Larger u128 groups recover some of that cost and improve long decoding; short inputs do not consistently benefit. The production API uses u32 scratch limbs and u64 arithmetic, while the larger-radix experiment remains available for reproduction.

| Fixed case | Serial radix-58⁵ | Parallel radix-58⁵ | Parallel radix-58⁴ |
| --- | ---: | ---: | ---: |
| 32 encode | 69.77 ns | 48.76 ns | 55.36 ns |
| 32 decode | 88.84 ns | 33.78 ns | 36.82 ns |
| 64 encode | 121.63 ns | 120.03 ns | 158.23 ns |
| 64 decode | 116.47 ns | 96.85 ns | 94.91 ns |

Parallel quotient expansion and full-group parsing are measured independently by operation. The alternative radix 58⁴ does not improve all four fixed operations consistently across both runs. Production retains radix 58⁵, word-sized zero-prefix counting and a u64 path for short numeric values. All zero-heavy cases remain in the complete matrix.

## Reproduction command

Check out the measured commit in a separate directory, then run the locked comparison project. Its benchmark sources and dependency lockfile are also preserved in each `source-snapshot.json`.

```sh
git clone https://github.com/LIghtJUNction/z58.git z58-reproduce
cd z58-reproduce
git checkout baf8f42af22f5dd30820f545ba07bf82d87ef561
cargo run --release --locked --manifest-path comparison/Cargo.toml -- --output /tmp/z58-portable
RUSTFLAGS='-C target-cpu=native' cargo run --release --locked --manifest-path comparison/Cargo.toml -- --output /tmp/z58-native
```

## Complete final evidence

- [Portable raw samples](results/verified-2026-11-09-portable/raw.csv), [all portable medians/ranges](results/verified-2026-11-09-portable/summary.csv), [portable environment and source hashes](results/verified-2026-11-09-portable/metadata.txt), [portable gate result](results/verified-2026-11-09-portable/gate.json), [portable exact source](results/verified-2026-11-09-portable/source-snapshot.json).
- [Native raw samples](results/verified-2026-11-09-native/raw.csv), [all native medians/ranges](results/verified-2026-11-09-native/summary.csv), [native environment and source hashes](results/verified-2026-11-09-native/metadata.txt), [native gate result](results/verified-2026-11-09-native/gate.json), [native exact source](results/verified-2026-11-09-native/source-snapshot.json).
- [Exact baseline versions, package checksums, upstream commits and licenses](baselines.json). Each result directory also contains the dependency lockfile.

Every algorithm’s output was verified against the independent reference before and after timing. Portable and native differential tests also check accepted/rejected byte strings, including every invalid byte value at every position of valid 32/64-byte encodings, extra leading ones, overflow and wrong final widths. Braid58’s published package reports a dirty source tree, so its registry package checksum is the exact dependency identifier.

## Earlier exploratory runs retained

The earlier runs preceded final harness lint cleanup and the capacity helper’s wider remainder arithmetic. Their observations were not overwritten or removed. Their hashes and partial Git states are preserved as historical experiment metadata; the exact-source snapshots above identify the final reproducible measurement.

- [Earlier portable raw data](results/exploratory-2026-11-09-portable/raw.csv), [summary](results/exploratory-2026-11-09-portable/summary.csv), [metadata](results/exploratory-2026-11-09-portable/metadata.txt), [failed target result](results/exploratory-2026-11-09-portable/gate.json).
- [Earlier native raw data](results/exploratory-2026-11-09-native/raw.csv), [summary](results/exploratory-2026-11-09-native/summary.csv), [metadata](results/exploratory-2026-11-09-native/metadata.txt), [failed target result](results/exploratory-2026-11-09-native/gate.json).
