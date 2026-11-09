# Fixed-size conversion

The 32- and 64-byte entry points use generated integer-power transforms with
bounded local arrays. They accept the same Bitcoin alphabet as the general
algorithm, preserve every leading zero, and commit output only after validation.
No handwritten SIMD, `unsafe`, heap allocation, or input-dependent stack array is
used. Compiler vectorization follows the caller's target flags.

## Integer representation

Let `B = 2^32` and `R = 58^5 = 656,356,768`. Input bytes become little-endian
arrays of 8 or 16 base-`B` words. The encoded number needs 9 or 18 base-`R`
groups. The generated coefficients are exact integers:

```text
E[k,j] = floor(B^j / R^k) mod R
D[k,j] = floor(R^j / B^k) mod B
```

Encoding forms each column `sum_j(input_word[j] * E[k,j])`, adds the carry
from the preceding column, stores the remainder modulo `R`, and passes the
quotient onward. Decoding applies `D`, retaining each low 32-bit word and
passing `value >> 32` onward. A nonzero final decoding carry is `Overflow`;
it is never discarded.

The generator reconstructs every represented power and asserts equality with
`B^j` or `R^j`. It also computes exact worst-case bounds, including carry,
using Python integers. The generated library uses only `u64` and `u32`.
Regeneration is a development command, never a build step:

```sh
python3 tools/gen_fixed.py --check
python3 tools/gen_fixed.py
```

## Overflow bounds

All coefficients and terms are nonnegative. Therefore, substituting the largest
possible word or group into a column bounds every partial sum as well as its
final sum. The carry bound is propagated from the preceding column rather than
assumed independently.

| Operation | Largest relevant accumulation, including carry | Treatment |
| --- | ---: | --- |
| 32-byte encoding | 9,797,816,699,155,577,822 | `u64` |
| 64-byte encoding, ordinary columns | 16,803,551,296,732,646,175 | `u64` |
| 64-byte encoding, column 1 before splitting | 26,701,617,621,929,087,163 | Split before addition |
| Column 1, first partial sum | 9,797,816,711,879,798,298 | `u64` |
| Column 1, second partial sum | 16,903,800,910,049,288,865 | `u64` |
| 32-byte decoding | 8,769,783,620,215,767,039 | `u64` |
| 64-byte decoding | 18,415,967,191,221,403,647 | `u64` |

`u64::MAX` is 18,446,744,073,709,551,615. The exceptional encoding column is
split at word 8 into `low` and `high`; the incoming carry is included in `low`.
It computes:

```text
rem = low % R + high % R
digit = rem % R
carry = low / R + high / R + rem / R
```

The remainder sum is at most `2 * (R - 1)`. The generator checks both partial
sums and the resulting carry. No overflowing sum is evaluated, in either debug
or release builds. Low-word casts during decoding intentionally extract
`value mod 2^32`; the high part remains in the separately checked carry.

## Character conversion and leading zeros

Each normalized radix group is below `58^5`. Independent divisions by
`58`, `58^2`, `58^3`, and `58^4` produce the five base-58 digits. Indexes into
the 58-character alphabet therefore remain in `0..58`.

The temporary encoded arrays have 45 or 90 characters. Their unnecessary
leading `1` characters are removed, then the input's actual leading zero count
is preserved. These temporary widths exceed the mathematical maximum widths
44 and 88. For every significant byte width `m <= N`, the difference between
its maximum base-58 width and `m` is no greater than `T - N`; this guarantees
that the computed skip is nonnegative. All-zero inputs have exactly `N`
characters, and a larger caller buffer receives no extra characters.

Decoding strips leading `1` characters only for the numeric transform, then
requires `leading_ones + significant_bytes == N`. Every nonzero base-58 digit
sequence has no more decoded bytes than input characters, so this addition
is bounded by the original input length. A numeric value that fits in `N`
bytes can still produce `InvalidLength` because of its explicit leading zeros.

The leading-byte counter examines eight bytes at a time using safe byte loads,
XOR with a repeated-byte mask, and `leading_zeros`. A final partial chunk is
scanned byte by byte. It never reads beyond the slice.

## Validation and short values

Full five-character groups are decoded with five table lookups and a polynomial
in powers of 58. The decode table maps every invalid byte to 255; valid digits
are below 58. An OR of the mapped digits detects invalid bytes using bit 7.
On failure, a forward validation pass reports the first invalid original byte
position, including the leading-zero offset. Oversized significant inputs are
also validated before returning `Overflow`.

Even before validation, a five-character group is at most
`255 * (58^5 - 1) / 57 = 2,936,332,905`, so assembling invalid groups cannot
overflow. Those values never enter the transform: an invalid-byte error is
returned first. ASCII whitespace, non-ASCII bytes, and `0/O/I/l` are rejected.

Values with at most eight significant input bytes use direct `u64` encoding.
They need at most eleven base-58 digits, so an eleven-byte local buffer suffices.
Values with at most ten significant characters use direct `u64` decoding;
`58^10 = 430,804,206,899,405,824 < 2^59`. These shortcuts preserve the same
length and validation rules. They reduce work for zero and long-zero-prefix
inputs without changing random-input acceptance.

## Reproducible alternatives

The independent byte-at-a-time reference, grouped general algorithm, and fixed
transforms are measured separately. `comparison/src/ablations/` also retains:

- `radix5_serial`: repeated quotient extraction and character-at-a-time parsing.
- `radix5_parallel`: independent quotient extraction and full five-character
  parsing, with ordinary prefix scans and no short-value shortcut.
- `radix4_parallel`: the same approach with `58^4` groups. Its complete dot
  products fit in `u64` without splitting, but it needs more groups.

The radix-4 source is reproducible with:

```sh
python3 tools/gen_fixed.py --radix-digits 4 \
  --output comparison/src/ablations/radix4_tables.rs --check
```

All alternatives are comparison-only code. Reported timings must identify the
compiler target and dataset; results for 32/64-byte fixed inputs do not imply
performance at arbitrary lengths. The benchmark report records both gains and
regressions against the strongest measured specialized baseline.

## Prior art and attribution

The radix-power dot-product method is established prior art. The implementations
reviewed during development were:

- [Firedancer `fd_base58.c`](https://github.com/firedancer-io/firedancer/blob/0643e123f7bff4d5f5fee45f33b5c6bad24517d7/src/ballet/base58/fd_base58.c),
  Copyright 2022 Firedancer Contributors, under
  [Apache-2.0](https://github.com/firedancer-io/firedancer/blob/0643e123f7bff4d5f5fee45f33b5c6bad24517d7/LICENSE).
- [five8](https://github.com/kevinheavey/five8/tree/6da4b702a83ade02076ca3e349ab4222a2c5637a),
  Copyright 2024 Kevin Heavey, under
  [MIT](https://github.com/kevinheavey/five8/blob/6da4b702a83ade02076ca3e349ab4222a2c5637a/LICENSE).

The constants here are generated independently from integer powers; source
kernels and SIMD code were not copied or linked. These credits describe the
algorithm's provenance and do not claim the underlying mathematics as a new
invention. The independently authored z58 implementation uses the repository's
MIT license.
