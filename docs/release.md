# Release procedure

The `Release` workflow publishes `z58` to crates.io. **One manual run can select
the version, create its tag, verify the exact commit, publish, and create the
GitHub Release.** It also accepts canonical `vX.Y.Z` tag-push events.

The repository secret is named `CARGO_REGISTRY_TOKEN`. The workflow exposes it
only to the `cargo +stable publish --locked` step. An empty secret stops that
step. PR, fork, and CI runs never receive the token. No `cargo login` is used.

## One-click releases

1. Open **GitHub → Actions → Release → Run workflow**.
2. Choose the **main** branch and a `bump` value:
   - `current`: publish the current `Cargo.toml` version (useful for the first
     release, `v0.1.0`). Creates its missing tag automatically.
   - `patch`: `0.1.0 → 0.1.1`.
   - `minor`: `0.1.0 → 0.2.0`.
   - `major`: `0.1.0 → 1.0.0`.
   - `existing`: retry an already-created tag; enter it in the optional `tag`
     field, such as `v0.1.0`.
3. Click **Run workflow** and watch the result. No manual editing of the
   manifest, lockfile, or Git tags is needed. A dry run is not a release receipt.

The version-preparation job writes only `Cargo.toml` and `Cargo.lock` when
bumping, commits the change to `main`, and creates the matching tag. It fails
if the target tag or crates.io version already exists. Only this job can push
with `contents: write`. A `GITHUB_TOKEN`-created tag does not trigger a second
push workflow, so the current workflow continues into the same validated
release jobs. The `existing` mode creates no refs and checks the tag's version.

Pushing a version tag with your own authenticated Git client continues to work;
that path validates the pre-existing tag and never increments the version.

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

All jobs default to `contents: read`; only version/tag preparation and GitHub
Release creation have `contents: write`. Release runs are serialized without cancellation. Failures
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
