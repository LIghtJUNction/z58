# z58

Bitcoin-alphabet Base58 for bytes. Always `no_std`, only `core`, no allocation,
no unsafe code, and no dependencies of any kind. Stable Rust; MSRV **1.70.0**.

```rust
let key = [0u8; 32];
let mut text = [0u8; z58::ENCODED_32_CAPACITY];
let len = z58::encode32(&key, &mut text).unwrap();
assert_eq!(&text[..len], &[b'1'; 32]);
let mut decoded = [0u8; 32];
z58::decode32(&text[..len], &mut decoded).unwrap();
assert_eq!(decoded, key);
```

`encode` / `decode` accept arbitrary-length slices, a caller-owned output slice,
and a caller-owned `&mut [u32]` workspace. `encoded_capacity`, `decoded_capacity`,
`encode_scratch_len`, and `decode_scratch_len` return checked **upper bounds**
(`Option<usize>`), not content-dependent exact lengths. Workspace sizes count
`u32` elements. The actual output length comes from the conversion result.

`encode32` / `encode64` and `decode32` / `decode64` need no external workspace.
Their stack storage is bounded. Encoding produces variable-length text, at most
44 / 88 bytes. Fixed decoders require exactly 32 / 64 decoded bytes: no padding,
high-byte truncation, checksum, or address interpretation.

Every leading zero byte maps to one leading `1`; empty input maps to empty
output. Decoding rejects non-ASCII, whitespace, `0/O/I/l`, and every other byte
outside `123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz`.
Invalid-character offsets count bytes. Numeric overflow, wrong fixed decoded
length, insufficient output/workspace, and capacity overflow have distinct
errors. Invalid characters are checked before decoding capacity errors.

**Every error preserves the complete output.** Successful calls modify only
the returned output prefix. Workspace contents may change when conversion starts,
including if the final output is too small, and are not erased afterwards.
Invalid characters and insufficient workspace leave the workspace unchanged.
The algorithms are variable-time; they are not cryptographic primitives.

Run `cargo test --locked` and `cargo test --release --locked` for known vectors,
independent reference comparisons, byte/pair exhaustions, fixed-size differential
tests, and boundary/error checks. The separate `comparison/` project contains
competitor tests and reproducible benchmarks; it is excluded from this workspace
and the published package. Fixed-size and arbitrary-length results are reported
separately. No performance ratio is promised without measured evidence.

Licensed under [MIT](LICENSE).
