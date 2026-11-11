// Fixed-size base conversion using independently generated integer powers.
// Radix-dot-product prior art: Firedancer fd_base58 (Apache-2.0) and five8 (MIT).
// See docs/fixed-kernel.md and tools/gen_fixed.py for bounds and derivation.
use crate::{ALPHABET, DECODE, Error};
#[path = "fixed_tables.rs"]
mod tables;

#[inline]
fn leading(input: &[u8], byte: u8) -> usize {
    let mask = u64::from_ne_bytes([byte; 8]);
    let mut count = 0;
    let (chunks, remainder) = input.as_chunks::<8>();
    for chunk in chunks {
        let value = u64::from_be_bytes([
            chunk[0], chunk[1], chunk[2], chunk[3], chunk[4], chunk[5], chunk[6], chunk[7],
        ]) ^ mask;
        if value != 0 {
            return count + value.leading_zeros() as usize / 8;
        }
        count += 8;
    }
    count
        + remainder
            .iter()
            .take_while(|&&b| b == byte)
            .count()
}

#[inline]
fn short_encode<const N: usize>(
    input: &[u8; N],
    zeros: usize,
    output: &mut [u8],
) -> Result<usize, Error> {
    let mut value = 0u64;
    for &byte in &input[zeros..] {
        value = (value << 8) | byte as u64;
    }
    let mut digits = [0u8; 11];
    let mut start = digits.len();
    while value != 0 {
        start -= 1;
        digits[start] = ALPHABET[(value % 58) as usize];
        value /= 58;
    }
    let required = zeros + digits.len() - start;
    if output.len() < required {
        return Err(Error::OutputTooSmall {
            required,
            available: output.len(),
        });
    }
    output[..zeros].fill(b'1');
    output[zeros..required].copy_from_slice(&digits[start..]);
    Ok(required)
}

#[inline]
fn short_decode<const N: usize>(
    input: &[u8],
    zeros: usize,
    output: &mut [u8; N],
) -> Result<(), Error> {
    let mut value = 0u64;
    for (i, &byte) in input[zeros..].iter().enumerate() {
        let digit = DECODE[byte as usize];
        if digit == 255 {
            return Err(Error::InvalidCharacter {
                index: zeros + i,
                byte,
            });
        }
        value = value * 58 + digit as u64;
    }
    let size = (64 - value.leading_zeros() as usize).div_ceil(8);
    let actual = zeros + size;
    if actual != N {
        return Err(Error::InvalidLength {
            expected: N,
            actual,
        });
    }
    let bytes = value.to_be_bytes();
    output[..zeros].fill(0);
    output[zeros..].copy_from_slice(&bytes[8 - size..]);
    Ok(())
}

#[inline]
fn words<const N: usize, const W: usize>(input: &[u8; N]) -> [u64; W] {
    let mut out = [0; W];
    for (word, chunk) in out.iter_mut().rev().zip(input.as_chunks::<4>().0) {
        *word = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]) as u64;
    }
    out
}

#[inline]
fn finish_encode<const D: usize, const T: usize>(
    zeros: usize,
    digits: &[u32; D],
    output: &mut [u8],
) -> Result<usize, Error> {
    let mut text = [0u8; T];
    for (value, chunk) in digits.iter().rev().zip(text.as_chunks_mut::<5>().0) {
        let value = *value;
        let q1 = value / 58;
        let q2 = value / 3364;
        let q3 = value / 195112;
        let q4 = value / 11316496;
        chunk[4] = ALPHABET[(value - q1 * 58) as usize];
        chunk[3] = ALPHABET[(q1 - q2 * 58) as usize];
        chunk[2] = ALPHABET[(q2 - q3 * 58) as usize];
        chunk[1] = ALPHABET[(q3 - q4 * 58) as usize];
        chunk[0] = ALPHABET[q4 as usize];
    }
    let skip = leading(&text, b'1') - zeros;
    let required = T - skip;
    if output.len() < required {
        return Err(Error::OutputTooSmall {
            required,
            available: output.len(),
        });
    }
    output[..required].copy_from_slice(&text[skip..]);
    Ok(required)
}

#[inline]
fn parse<const D: usize>(input: &[u8]) -> Result<[u64; D], Error> {
    let mut digits = [0u64; D];
    let mut bad = 0;
    let mut chunks = input.rchunks_exact(5);
    for (group, chunk) in digits.iter_mut().zip(chunks.by_ref()) {
        let a = DECODE[chunk[0] as usize];
        let b = DECODE[chunk[1] as usize];
        let c = DECODE[chunk[2] as usize];
        let d = DECODE[chunk[3] as usize];
        let e = DECODE[chunk[4] as usize];
        bad |= a | b | c | d | e;
        *group =
            a as u64 * 11316496 + b as u64 * 195112 + c as u64 * 3364 + d as u64 * 58 + e as u64;
    }
    let rest = chunks.remainder();
    if !rest.is_empty() {
        let mut value = 0;
        for &byte in rest {
            let digit = DECODE[byte as usize];
            bad |= digit;
            value = value * 58 + digit as u64;
        }
        digits[input.len() / 5] = value;
    }
    if bad & 128 != 0 {
        crate::validate(input)?;
    }
    Ok(digits)
}

#[inline]
fn prepare<const N: usize, const D: usize>(
    input: &[u8],
    zeros: usize,
) -> Result<([u64; D], usize), Error> {
    let body = &input[zeros..];
    if body.len() > if N == 32 { 44 } else { 88 } {
        crate::validate(input)?;
        return Err(Error::Overflow);
    }
    let digits = parse(body).map_err(|error| match error {
        Error::InvalidCharacter { index, byte } => Error::InvalidCharacter {
            index: index + zeros,
            byte,
        },
        other => other,
    })?;
    Ok((digits, zeros))
}

#[inline]
fn finish_decode<const N: usize, const W: usize>(
    words: &[u32; W],
    high: u64,
    zeros: usize,
    output: &mut [u8; N],
) -> Result<(), Error> {
    if high != 0 {
        return Err(Error::Overflow);
    }
    let mut bytes = [0u8; N];
    for (word, chunk) in words.iter().rev().zip(bytes.as_chunks_mut::<4>().0) {
        chunk.copy_from_slice(&word.to_be_bytes());
    }
    let binary_zeros = leading(&bytes, 0);
    let actual = zeros + (N - binary_zeros);
    if actual != N {
        return Err(Error::InvalidLength {
            expected: N,
            actual,
        });
    }
    *output = bytes;
    Ok(())
}

/// Encode exactly 32 bytes, returning a variable length of 32 through 44 bytes.
/// Output is unchanged on error; bytes after the returned length are untouched.
#[inline]
pub fn encode32(input: &[u8; 32], output: &mut [u8]) -> Result<usize, Error> {
    let zeros = leading(input, 0);
    if zeros >= 32 - 8 {
        return short_encode(input, zeros, output);
    }
    let digits = tables::encode32(&words::<32, 8>(input));
    finish_encode::<9, 45>(zeros, &digits, output)
}

/// Decode strict Base58 into exactly 32 bytes without padding or truncation.
/// Invalid characters take precedence; numeric overflow and wrong length are errors.
/// The entire output is unchanged on any error.
#[inline]
pub fn decode32(input: &[u8], output: &mut [u8; 32]) -> Result<(), Error> {
    let zeros = leading(input, b'1');
    if input.len() - zeros <= 10 {
        return short_decode(input, zeros, output);
    }
    let (digits, zeros) = prepare::<32, 9>(input, zeros)?;
    let (words, carry) = tables::decode32(&digits);
    finish_decode(&words, carry, zeros, output)
}

/// Encode exactly 64 bytes, returning a variable length of 64 through 88 bytes.
/// Output is unchanged on error; bytes after the returned length are untouched.
#[inline]
pub fn encode64(input: &[u8; 64], output: &mut [u8]) -> Result<usize, Error> {
    let zeros = leading(input, 0);
    if zeros >= 64 - 8 {
        return short_encode(input, zeros, output);
    }
    let digits = tables::encode64(&words::<64, 16>(input));
    finish_encode::<18, 90>(zeros, &digits, output)
}

/// Decode strict Base58 into exactly 64 bytes without padding or truncation.
/// Invalid characters take precedence; numeric overflow and wrong length are errors.
/// The entire output is unchanged on any error.
#[inline]
pub fn decode64(input: &[u8], output: &mut [u8; 64]) -> Result<(), Error> {
    let zeros = leading(input, b'1');
    if input.len() - zeros <= 10 {
        return short_decode(input, zeros, output);
    }
    let (digits, zeros) = prepare::<64, 18>(input, zeros)?;
    let (words, carry) = tables::decode64(&digits);
    finish_decode(&words, carry, zeros, output)
}
