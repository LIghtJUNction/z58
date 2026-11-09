# Generic conversion and bounds

Base58 interprets the significant input as an unsigned integer, changes its
radix, then restores leading zeroes. Leading zero bytes are not part of the
integer: each contributes one `1` independently. The empty integer contributes
no digits. This distinction determines the exact-length fixed decoder checks.

## Independent reference

`tests/reference/mod.rs` encodes by repeatedly dividing a big-endian byte array
by 58, collecting remainders. Its decoder multiplies a little-endian byte array
by 58 and adds one digit. Both use `std` exclusively in integration tests. They
share neither production limbs nor generated coefficient tables. The first
implementation commit precedes the specialized kernels and grouped input loops.

## Generic encoding

The caller's workspace holds little-endian digits in radix `R = 58^5 =
656356768`. A group of `k <= 4` input bytes gives `v < 256^k`; replacing the
integer `x` by `x * 256^k + v` consumes that entire group in one limb pass.
Each limb step computes `t = limb * factor + carry`, stores `t % R`, and passes
`t / R` to the next limb. Carry is appended until zero; a four-byte group can
need **two** extra limbs.

Since `limb <= R-1` and `carry <= factor-1`,

```text
t <= (R-1)*factor + factor-1 = R*factor-1
  <= 58^5 * 2^32 - 1 < 2^62 < 2^64.
```

The quotient is strictly smaller than `factor`, preserving the carry invariant.
All divisions use compile-time constants. Each complete limb becomes five
characters; the most significant limb emits only its significant characters.
The saved zero count is then prepended. Output writes begin only after computing
and checking the exact character count.

## Generic decoding

After validating the alphabet, the caller's workspace holds little-endian
`u32` limbs in radix `2^32`. A group of `k <= 5` characters gives `v < 58^k`.
Each limb computes `t = limb * 58^k + carry`, stores its low 32 bits, and passes
its high 32 bits onward. Here,

```text
t <= (2^32-1)*factor + factor-1 = 2^32*factor-1
  <= 2^32 * 58^5 - 1 < 2^62 < 2^64.
```

Carry is smaller than `58^5`, so a single new `u32` limb suffices after each
group. The final nonzero limb determines the significant byte count. Output is
written backwards one limb at a time, following the preserved leading zeroes.
Every invalid input byte is rejected before workspace mutation.

## Capacity arithmetic

For a nonzero `n`-byte integer, `ceil(n * log(256)/log(58))` bounds the number of
characters. The rational `138/100` is a strict upper bound on that logarithm
ratio (`256^100 < 58^138`). Including zero prefixes cannot increase this bound
because the ratio exceeds one. Thus `ceil(138*n/100)` is conservative for every
input, with zero returned for an empty slice.

Encoding needs at most `ceil(138*n/500)` radix-`58^5` workspace limbs. A valid
`m`-character string decodes to at most `m` bytes including leading `1`s, so
`ceil(m/4)` `u32` workspace elements suffice. Helpers operate on lengths, not
contents; they never claim to calculate exact encoded or decoded lengths.

The scaled bounds split `n` into quotient and remainder before multiplication.
The quotient product and final addition are checked. The largest remainder
calculation is `499*138+499 = 69361`; it uses `u64`, also avoiding overflow on
16-bit `usize`. After division it is at most 138, so narrowing is exact. The
decoder helper uses `m/4 + (m%4 != 0)`, avoiding
the overflow of `(m+3)/4`. Final significant lengths use checked multiplication
and addition. Tests check helper values against `u128` arithmetic, including
`usize::MAX` and values around the overflow threshold.

## Complexity and ownership

Generic conversion is quadratic in the number of significant input units.
Grouping reduces limb passes and large-radix limbs reduce their width; the
algorithm still has quadratic long-input behavior. Workspace is linear and
owned by the caller. There is no recursion or stack array whose size depends
on runtime input length. Zero-prefix scans and all-zero inputs are linear.

The specialized kernels use fixed 32/64-byte dimensions and bounded local
arrays. Their derivation, partial-sum bounds and provenance are documented in
[fixed-kernel.md](fixed-kernel.md). Both implementations are safe Rust under
`#![forbid(unsafe_code)]`; no hardware-specific unsafe backend is shipped.

Every public conversion preserves the entire output on failure. Workspace
contents may change after valid conversion starts, including when output is too
small. Success changes only the exact output prefix. Routines do not erase
workspaces and are variable-time.

## Evidence

`tests/correctness.rs` covers independent known vectors, all 256 one-byte and
65536 two-byte values, all 3364 pairs of alphabet digits, deterministic random
and structured inputs through 4096 bytes, all fixed-size zero-prefix counts,
invalid bytes at each tested position, exact capacities, atomic failures, and
fixed integer/length boundaries. A 65536-character zero-prefix case checks
long input handling without large stack storage. The independent comparison
workspace adds current competitor and algorithm-ablation checks. Performance
claims must cite its recorded samples and hardware conditions separately.
