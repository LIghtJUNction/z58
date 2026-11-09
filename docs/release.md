# Release procedure

The `Release` workflow publishes the existing `z58` package name to crates.io.
It accepts canonical `vX.Y.Z` tags only. A manual run takes an **existing** tag;
it rejects branch names, missing tags, prerelease suffixes, version mismatches,
and tags that no longer identify the checked commit.

The repository secret is named `CARGO_REGISTRY_TOKEN`. The workflow exposes it
only to the `cargo +stable publish --locked` step. An empty secret stops that
step. PR, fork, and CI runs never receive the token. No `cargo login` is used.

## Triggering a release

1. Set the package version in `Cargo.toml` and commit its matching `Cargo.lock`.
2. Push a matching version tag at the intended commit, or manually select an
   already-existing version tag in the Release workflow.
3. Follow the workflow result. A dry-run message is not a publication receipt.

The workflow resolves the tag to an exact SHA, checks its manifest, and runs the
complete CI matrix with every checkout pinned to that SHA. The publish job
checks out the same SHA and repeats the tag identity check. It then runs:

```sh
python3 scripts/check_package.py  # includes cargo package --list --locked
cargo +stable package --locked
cargo +stable publish --dry-run --locked
cargo +stable publish --locked
```

Publication requires an unused registry version and a clean archive whose
`.cargo_vcs_info.json` matches the checked commit. After upload, the workflow
waits for the crates.io sparse index and checks the version's package SHA-256
and yank status. A separate job creates the GitHub Release only after that
confirmation. It repeats the tag and registry checks and uses `--verify-tag`.

All jobs default to `contents: read`; only GitHub Release creation has
`contents: write`. Release runs are serialized without cancellation. Failures
are propagated, and a published version is never overwritten. If publication
already succeeded but a later step failed, the workflow stops on that immutable
registry version on retry; it never pretends a second upload succeeded.

## Checks and hardware

CI runs stable and Rust 1.70.0 debug/release/doc tests on native x86-64 and
AArch64 runners, stable tooling, bare-metal builds, a Miri scalar selection,
zero-dependency/package checks, and portable/native competitor tests and smoke
benchmarks. The actual package excludes this release tooling and all comparison
dependencies. Generated arithmetic tables are reproduced and checked in CI.

The separate performance workflow requires a self-hosted Linux x86-64 runner
labelled `z58-benchmark` and a `Z58_BENCH_CPU` repository variable matching its
CPU model. It retains complete portable/native samples and requires at least
2x against the strongest dedicated baseline in **all four** predeclared random
32/64-byte encode/decode cases. Shared GitHub runners run correctness and smoke
checks without timing thresholds. The recorded shared-host measurements missed
the performance target; see [the complete results](../comparison/RESULTS.md).

Actions are pinned to verified upstream commits: checkout v7.0.1 at
`3d3c42e5aac5ba805825da76410c181273ba90b1`, upload-artifact v7.0.2 at
`cf430e030ddbb5b0abf93d22962f4752f3646cd9`.
