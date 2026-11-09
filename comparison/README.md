# Reproducing z58 measurements

This independent workspace contains competitors, an elementary reference, differential tests, and a `std` benchmark runner. None of its dependencies or files are part of the published `z58` crate. The current stable `bs58` version was checked against its registry index on 2026-11-09; all versions are exact pins in `Cargo.toml`, package checksums are in `Cargo.lock`, and source commits are in `baselines.json`.

## Commands

```sh
cargo test --locked --manifest-path comparison/Cargo.toml
cargo run --release --locked --manifest-path comparison/Cargo.toml -- \
  --smoke --output comparison/results/smoke
cargo run --release --locked --manifest-path comparison/Cargo.toml -- \
  --output comparison/results/portable
RUSTFLAGS='-C target-cpu=native' cargo run --release --locked \
  --manifest-path comparison/Cargo.toml -- \
  --output comparison/results/native
python3 scripts/performance_gate.py comparison/results/native/raw.csv
```

Use distinct output directories for each run. Run on an idle machine, pin the process to a physical core if possible, and retain `raw.csv`, `metadata.txt`, `Cargo.lock`, and `baselines.json` together. The native executable requires the build machine's CPU features. The runner records compiler, OS, CPU, affinity, environment flags and repository state. Shared-runner samples are exploratory. The dedicated fixed-hardware workflow owns strict performance acceptance.

## Workload fixed before optimization

- Lengths: 0, 1, 2, 8, 16, 32, 64, 128, 256, 1024, 4096 bytes.
- Patterns: deterministic random, all zero, all `0xff`, a 75% zero prefix, and rotating `00 11 aa ff` repetitions.
- Sixteen values per length/pattern; xorshift64 seed `0x6a09e667f3bcc909`. Every case has an independent deterministic stream.
- Nine raw samples after warm-up and calibration, with an approximately 2 ms target per sample. Smoke uses two approximately 50 μs samples and a shorter length list.
- Input generation, allocation and correctness checks occur outside timing. Every algorithm receives the same inputs, preallocated output and applicable scratch buffers. The runner validates all outputs against the independent reference before and after timing. `black_box` covers inputs, output buffers and returned lengths.
- The four primary target rows are random encode/decode at 32/64 bytes. The target is at least 2× against the fastest compatible dedicated baseline for **each** row. The rest of the matrix remains in the report, including regressions.

The Rust release profile is identical for z58 and Rust competitors: optimization level 3, thin LTO, one codegen unit. Portable and native results are separate. Braid58's unmodified build script compiles its C kernels at the Cargo optimization level and selects scalar, AVX2 or AVX-512 from the same target feature set. This is a comparison of released implementations, including their implementation languages, not an isolated compiler comparison.

## Algorithms and semantics

| Name | Scope | Role |
| --- | --- | --- |
| `z58-generic` | Any length | Production generic API, including capacity checks |
| `z58-fixed` | 32/64 bytes | Production fixed API, exact decoded length |
| `bs58` 0.5.1 | Any length | Stable generic baseline, Bitcoin alphabet, preallocated `onto` |
| `five8` 1.0.0 | 32/64 bytes | Dedicated Rust baseline, strongest backend enabled by compilation target |
| `braid58` 0.1.0 | 32/64 bytes | Dedicated scalar/AVX2/AVX-512 baseline |
| `fd-bs58` 0.1.0 | 32/64 decode | Additional dedicated decoder; returned inline array copied to output |
| `reference-byte` | Any length | Independent byte-by-byte positional conversion |
| `radix32-single` | Any length | One binary byte / one Base58 digit per pass, radix-58⁵ / u32 limbs |
| `radix32` | Any length | Four binary bytes / five Base58 digits per group, `u64` arithmetic |
| `radix32-u128` | Any length | Same groups with `u128` arithmetic, isolating wider arithmetic overhead |
| `radix64` | Any length | Eight binary bytes / ten Base58 digits per group, `u128` arithmetic |
| `fixed-radix5-serial` | 32/64 bytes | Fixed dot products with serial character expansion/parsing |
| `fixed-radix5-parallel` | 32/64 bytes | Same radix and tables, independent constant quotients and full-group parsing |
| `fixed-radix4-parallel` | 32/64 bytes | Alternative radix-58⁴ fixed matrix |

All timed decoders enforce the Bitcoin alphabet without correcting invalid characters. Fixed decoders require exactly 32 or 64 output bytes and reject overflow; adversarial differential tests check agreement with the generic bs58 decoder. The independent ablations are measured on valid inputs and deliberately retain character validation. They are research routines with prevalidated buffer capacities, so their timings do not include production capacity/error wrappers; they are not production API speed claims.

Braid58's public C encoder is called through a small audited wrapper solely to use caller-provided output directly. It needs one extra byte for its NUL terminator; the reported output excludes that byte. Its Rust decoder already exposes an `into` API. `fd_bs58` encoding returns an allocating `String`, so it is excluded from allocation-free timing rather than compared under different memory semantics. Its decoder remains in the comparison.

## Interpreting ablations

Compare `reference-byte` with `radix32-single` for larger limbs; compare `radix32-single` with `radix32` for multiple input units per pass; compare `radix32` with `radix32-u128` for arithmetic width at unchanged group size; compare `radix32-u128` with `radix64` for larger radix at unchanged intermediate width. Compare production `z58-generic` and `z58-fixed` only on the 32/64 rows to evaluate specialization. Compare `fixed-radix5-serial` and `fixed-radix5-parallel` separately by operation: encoding isolates constant-power quotient expansion, while decoding isolates full-group parsing. Compare the parallel radix-58⁵ and radix-58⁴ variants for a different fixed conversion radix. Production `z58-fixed` additionally uses word-sized leading-zero counts and a short numeric path for values that fit in u64. These comparisons retain the complete data patterns and length matrix. A fixed-width win says nothing about unrelated lengths.

The published Braid58 package marks its source tree `dirty: true` in `.cargo_vcs_info.json`; its immutable crates.io package checksum is therefore the exact reproduction identifier. No competitor source is copied into z58. The benchmark links bs58 (MIT/Apache-2.0), five8 (MIT), Braid58 (MIT) and fd_bs58 (Apache-2.0) only as independent comparison dependencies.
