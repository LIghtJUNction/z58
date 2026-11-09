# Measured results — 2026-11-09

The predeclared 2× target against the strongest dedicated baseline was **not met**. Both portable and native runs retain all four failed primary cases, every other length/pattern, every ablation and every raw sample. These are exploratory measurements on a shared virtual machine, pinned to CPU 8; they are not fixed-hardware performance acceptance.

## Environment and reproducibility

Intel Xeon Platinum 8573C, Linux 6.18.57 x86-64, Rust 1.99.1 (LLVM 23.1.1), GCC 14.2.0. Rust flags were empty for portable and `-C target-cpu=native` for native. The native build enables AVX2 in five8 and AVX-512 in Braid58. Each run contains 8,100 raw samples across 900 algorithm/operation/length/pattern groups. The full predetermined corpus and timing method are in [README.md](README.md).

The native run showed substantial host noise: for example, random 32-byte generic encoding ranged from 108.90 to 582.40 ns/op and random 64-byte generic encoding had one 4,960.79 ns/op sample among roughly 238 ns/op samples. The report uses medians without removing outliers. Minima and maxima are retained in each `summary.csv`; every observation remains in `raw.csv`. Treat the ratios as this run’s observations rather than hardware-independent guarantees.

## Fixed 32/64-byte primary cases

Times below are median nanoseconds per operation. Speedup is fastest dedicated baseline time divided by z58 time; a value below 1 means z58 was slower.

| Profile | Bytes | Operation | z58 fixed | five8 | Braid58 | fd_bs58 | Strongest baseline / z58 | 2× target |
| --- | ---: | --- | ---: | ---: | ---: | ---: | ---: | --- |
| portable | 32 | encode | 47.18 | 93.31 | 40.82 | — | 0.87× | Miss |
| portable | 32 | decode | 33.44 | 76.14 | 53.97 | 91.95 | 1.61× | Miss |
| portable | 64 | encode | 119.54 | 199.97 | 121.85 | — | 1.02× | Miss |
| portable | 64 | decode | 75.47 | 215.12 | 96.54 | 279.77 | 1.28× | Miss |
| native | 32 | encode | 64.74 | 73.15 | 33.57 | — | 0.52× | Miss |
| native | 32 | decode | 40.81 | 54.18 | 22.15 | 72.29 | 0.54× | Miss |
| native | 64 | encode | 96.57 | 105.17 | 78.47 | — | 0.81× | Miss |
| native | 64 | decode | 89.28 | 111.85 | 37.94 | 209.79 | 0.42× | Miss |

The result supports retaining a small scalar implementation with explicit bounds, while leaving the 2× dedicated-codec goal open. Braid58’s native vector kernels are the strongest measured comparison. z58’s fixed conversion still evaluates scalar dot products and carry chains; the comparison identifies vectorized conversion and scheduling as the next relevant performance question. No SIMD implementation or universal-speed claim is inferred from these measurements.

## Generic lengths, including short-input regressions

This table uses random inputs and compares the production generic API with the preallocated bs58 API. Fixed-width functions are excluded from these ratios.

| Bytes | Portable encode | Portable decode | Native encode | Native decode |
| ---: | ---: | ---: | ---: | ---: |
| 0 | 0.16× | 0.62× | 0.50× | 0.40× |
| 1 | 0.37× | 0.56× | 0.60× | 0.96× |
| 2 | 0.89× | 1.06× | 0.62× | 0.59× |
| 8 | 3.03× | 1.43× | 2.59× | 1.41× |
| 16 | 4.92× | 2.50× | 1.92× | 2.07× |
| 32 | 7.93× | 4.47× | 15.99× | 3.89× |
| 64 | 14.24× | 10.60× | 18.13× | 8.71× |
| 128 | 21.31× | 14.16× | 15.74× | 9.94× |
| 256 | 23.43× | 16.93× | 22.18× | 16.40× |
| 1024 | 25.28× | 20.96× | 27.61× | 20.24× |
| 4096 | 24.60× | 19.36× | 19.69× | 18.13× |

Empty and one-byte calls have a measurable regression against bs58, where fixed wrapper and capacity-check costs dominate useful work. Large random inputs benefit from grouped conversion, but the generic algorithm remains quadratic: larger values require more limbs on every pass. Long-input raw rows are retained through 4,096 bytes; these results do not imply linear scaling.

## Independent optimization measurements

The following portable random-input ratios compare the research routines under common wrappers. Production capacity-error handling remains separately measured in `z58-generic`. A speedup below 1 identifies a regression.

| Bytes | Operation | Grouped / one-unit gain | u128 overhead at same groups | Larger groups within u128 gain |
| ---: | --- | ---: | ---: | ---: |
| 32 | encode | 2.33× | 2.62× time | 2.56× |
| 32 | decode | 1.91× | 1.86× time | 1.53× |
| 64 | encode | 3.49× | 2.93× time | 2.77× |
| 64 | decode | 1.97× | 1.68× time | 1.77× |
| 256 | encode | 3.86× | 2.94× time | 3.32× |
| 256 | decode | 3.76× | 2.19× time | 3.01× |
| 1024 | encode | 6.04× | 2.95× time | 3.72× |
| 1024 | decode | 5.49× | 2.70× time | 5.15× |
| 4096 | encode | 4.47× | 3.15× time | 4.74× |
| 4096 | decode | 4.04× | 2.09× time | 7.35× |

The experiments isolate group size, arithmetic width and fixed-size specialization. At unchanged group size, u128 arithmetic costs more than u64 here. Larger u128 groups recover that cost and improve long decoding; short inputs do not consistently benefit. The production API currently keeps u32 scratch limbs and u64 arithmetic; the larger-radix experiment remains reproducible in this comparison workspace.

| Fixed case | Serial radix-58⁵ | Parallel radix-58⁵ | Parallel radix-58⁴ |
| --- | ---: | ---: | ---: |
| 32 encode | 59.91 ns | 46.66 ns | 49.00 ns |
| 32 decode | 45.74 ns | 36.19 ns | 31.30 ns |
| 64 encode | 147.02 ns | 113.43 ns | 122.78 ns |
| 64 decode | 118.99 ns | 76.40 ns | 93.36 ns |

Parallel quotient expansion and full-group parsing are measured independently by operation. Radix 58⁴ does not improve all four fixed operations, so it remains an ablation instead of replacing the production radix 58⁵ kernel. Production adds word-sized zero-prefix counting and a u64 path for short numeric values; zero-heavy inputs stay in the complete result matrix.

## Complete evidence

- [Portable raw samples](results/exploratory-2026-11-09-portable/raw.csv), [all portable medians/ranges](results/exploratory-2026-11-09-portable/summary.csv), [portable environment and source hashes](results/exploratory-2026-11-09-portable/metadata.txt), [portable gate result](results/exploratory-2026-11-09-portable/gate.json).
- [Native raw samples](results/exploratory-2026-11-09-native/raw.csv), [all native medians/ranges](results/exploratory-2026-11-09-native/summary.csv), [native environment and source hashes](results/exploratory-2026-11-09-native/metadata.txt), [native gate result](results/exploratory-2026-11-09-native/gate.json).
- [Exact baseline versions, package checksums, upstream commits and licenses](baselines.json). Each result directory also contains the dependency lockfile.

The native release differential suite passed after compilation with host ISA features. It checks the same outputs and accepted/rejected byte strings as portable tests, including all invalid byte values at every position of valid 32/64-byte encodings, extra leading ones, overflow and wrong final widths. Braid58’s measured package reports a dirty source tree, so its registry package checksum is the exact source identifier.

Ablation table-module sharing and `div_ceil` spelling were cleaned up after these measurements; production source hashes in both runs identify the measured kernel. No observed timing row was deleted or overwritten.
