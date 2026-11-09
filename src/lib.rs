#![no_std]
#![forbid(unsafe_code)]
#![deny(missing_docs)]
//! Allocation-free Bitcoin Base58, using only `core`.
//!
//! [`encode`] and [`decode`] accept arbitrary-length slices and a caller-owned
//! `u32` workspace. [`encode32`]/[`encode64`] and [`decode32`]/[`decode64`] use
//! bounded local storage. Fixed decoders require exactly 32 or 64 decoded bytes;
//! they never insert leading zeroes or discard high bytes.
//!
//! Every leading zero byte is one leading `1`. Empty input produces empty output.
//! Decoding rejects every byte outside the Bitcoin alphabet, including whitespace,
//! non-ASCII bytes, and `0`, `O`, `I`, `l`. Error indices are **byte** offsets.
//!
//! # Buffers and errors
//!
//! Success writes only `output[..returned_length]`. The remaining output is
//! unchanged. **Every error leaves the entire output unchanged.** Workspace
//! contents are unspecified after a conversion starts, including on an
//! `OutputTooSmall` error. Invalid characters and insufficient workspace are
//! detected before workspace mutation. Workspaces are not erased afterwards.
//!
//! Capacity helpers return conservative upper bounds, not exact encoded lengths.
//! A smaller output is accepted whenever the actual result fits. Workspace
//! bounds count `u32` elements, not bytes; leading zeroes need no workspace.
//!
//! All paths have the same alphabet and acceptance rules. Invalid-character
//! validation precedes decoding capacity/length errors. If several resource
//! limits are violated, callers should not rely on which capacity error wins.
//! These routines are variable-time and provide no constant-time guarantee.
//!
//! ```
//! let mut text = [0u8; 8];
//! let mut scratch = [0u32; 2];
//! let len = z58::encode(b"hello", &mut text, &mut scratch)?;
//! assert_eq!(&text[..len], b"Cn8eVZg");
//! let mut bytes = [0u8; 5];
//! let n = z58::decode(&text[..len], &mut bytes, &mut scratch)?;
//! assert_eq!(&bytes[..n], b"hello");
//! # Ok::<(), z58::Error>(())
//! ```

use core::fmt;

mod fixed;
mod generic;

pub use fixed::{decode32, decode64, encode32, encode64};
pub use generic::{decode, encode};

/// Maximum encoded length of a 32-byte value (the actual length is variable).
pub const ENCODED_32_CAPACITY: usize = 44;
/// Maximum encoded length of a 64-byte value (the actual length is variable).
pub const ENCODED_64_CAPACITY: usize = 88;

pub(crate) const ALPHABET: [u8; 58] =
    *b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

const fn decode_table() -> [u8; 256] {
    let mut table = [255; 256];
    let mut i = 0;
    while i < ALPHABET.len() {
        table[ALPHABET[i] as usize] = i as u8;
        i += 1;
    }
    table
}

pub(crate) const DECODE: [u8; 256] = decode_table();

/// A conversion or buffer sizing error. Output is unchanged on every error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// A required capacity cannot be represented by `usize`.
    CapacityOverflow,
    /// The actual result does not fit in the output.
    OutputTooSmall {
        /// Exact result length in bytes.
        required: usize,
        /// Provided output length in bytes.
        available: usize,
    },
    /// The workspace is smaller than the bound for the significant input.
    ScratchTooSmall {
        /// Required workspace length in `u32` elements (a conservative bound).
        required: usize,
        /// Provided workspace length in `u32` elements.
        available: usize,
    },
    /// A byte outside the Bitcoin Base58 alphabet.
    InvalidCharacter {
        /// Offset in the original input, measured in bytes.
        index: usize,
        /// Offending byte, including any non-ASCII UTF-8 byte.
        byte: u8,
    },
    /// A fixed-size decoder's final result has the wrong number of bytes.
    InvalidLength {
        /// Required decoded byte length.
        expected: usize,
        /// Actual decoded byte length, including every leading `1`.
        actual: usize,
    },
    /// The significant integer exceeds a fixed-size decoder's capacity.
    Overflow,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::CapacityOverflow => f.write_str("capacity exceeds usize"),
            Self::OutputTooSmall {
                required,
                available,
            } => {
                write!(f, "output needs {required} bytes; provided {available}")
            }
            Self::ScratchTooSmall {
                required,
                available,
            } => {
                write!(
                    f,
                    "workspace needs {required} u32 elements; provided {available}"
                )
            }
            Self::InvalidCharacter { index, byte } => {
                write!(f, "invalid Base58 byte 0x{byte:02x} at byte offset {index}")
            }
            Self::InvalidLength { expected, actual } => {
                write!(f, "expected {expected} decoded bytes; got {actual}")
            }
            Self::Overflow => f.write_str("Base58 integer exceeds fixed-size capacity"),
        }
    }
}

const fn scaled_ceil(n: usize, mul: usize, div: usize) -> Option<usize> {
    // Split before multiplication: the remainder product is bounded by div*mul.
    let whole = match (n / div).checked_mul(mul) {
        Some(value) => value,
        None => return None,
    };
    // The largest remainder numerator is 69_361; widen so 16-bit usize is safe too.
    let remainder = ((n % div) as u64 * mul as u64 + div as u64 - 1) / div as u64;
    whole.checked_add(remainder as usize)
}

/// Conservative encoded byte capacity, `ceil(n * 138 / 100)`; zero for empty input.
///
/// Returns `None` if this bound overflows `usize`. Exact length depends on the
/// value and its leading zeroes; use the length returned by [`encode`].
pub const fn encoded_capacity(input_len: usize) -> Option<usize> {
    scaled_ceil(input_len, 138, 100)
}

/// Conservative decoded byte capacity, including an all-`1` input's leading zeros.
///
/// Every valid `n`-character input decodes to at most `n` bytes, so this bound
/// always fits `usize`. It is not the exact length of an arbitrary input.
pub const fn decoded_capacity(input_len: usize) -> Option<usize> {
    Some(input_len)
}

/// Conservative [`encode`] workspace length, in `u32` elements.
///
/// This is `ceil(n * 138 / 500)` for base-58^5 limbs. Capacity arithmetic is
/// checked; the bound always fits `usize` on supported targets.
pub const fn encode_scratch_len(input_len: usize) -> Option<usize> {
    scaled_ceil(input_len, 138, 500)
}

/// Conservative [`decode`] workspace length, in `u32` elements.
///
/// `ceil(n / 4)` is safe even when every input character is a leading `1`.
/// The addition is arranged to remain valid at `usize::MAX`.
pub const fn decode_scratch_len(input_len: usize) -> Option<usize> {
    Some(input_len / 4 + (input_len % 4 != 0) as usize)
}

pub(crate) fn validate(input: &[u8]) -> Result<(), Error> {
    for (index, &byte) in input.iter().enumerate() {
        if DECODE[byte as usize] == 255 {
            return Err(Error::InvalidCharacter { index, byte });
        }
    }
    Ok(())
}

pub(crate) fn check_output(required: usize, available: usize) -> Result<(), Error> {
    if required > available {
        Err(Error::OutputTooSmall {
            required,
            available,
        })
    } else {
        Ok(())
    }
}

pub(crate) fn check_scratch(required: usize, available: usize) -> Result<(), Error> {
    if required > available {
        Err(Error::ScratchTooSmall {
            required,
            available,
        })
    } else {
        Ok(())
    }
}
