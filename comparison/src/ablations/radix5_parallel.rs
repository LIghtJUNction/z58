// Fixed-size base conversion using independently generated integer powers.
// Radix-dot-product prior art: Firedancer fd_base58 (Apache-2.0) and five8 (MIT).
// Comparison-only ablation; see docs/fixed-kernel.md for the bounds.
use crate::fixed_tables as tables;
use crate::{Error, ALPHABET, DECODE};

#[inline]
fn words<const N: usize, const W: usize>(input: &[u8; N]) -> [u64; W] {
    let mut out = [0; W];
    for (word, chunk) in out.iter_mut().rev().zip(input.chunks_exact(4)) {
        *word = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]) as u64;
    }
    out
}

#[inline]
fn finish_encode<const N: usize, const D: usize, const T: usize>(
    input: &[u8; N],
    digits: &[u32; D],
    output: &mut [u8],
) -> Result<usize, Error> {
    let mut text = [0u8; T];
    for (value, chunk) in digits.iter().rev().zip(text.chunks_exact_mut(5)) {
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
    let zeros = input.iter().take_while(|&&b| b == 0).count();
    let skip = text.iter().take_while(|&&b| b == b'1').count() - zeros;
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
fn prepare<const N: usize, const D: usize>(input: &[u8]) -> Result<([u64; D], usize), Error> {
    let zeros = input.iter().take_while(|&&b| b == b'1').count();
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
    for (word, chunk) in words.iter().rev().zip(bytes.chunks_exact_mut(4)) {
        chunk.copy_from_slice(&word.to_be_bytes());
    }
    let binary_zeros = bytes.iter().take_while(|&&b| b == 0).count();
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

#[inline]
pub fn encode32(input: &[u8; 32], output: &mut [u8]) -> Result<usize, Error> {
    let digits = tables::encode32(&words::<32, 8>(input));
    finish_encode::<32, 9, 45>(input, &digits, output)
}
#[inline]
pub fn decode32(input: &[u8], output: &mut [u8; 32]) -> Result<(), Error> {
    let (digits, zeros) = prepare::<32, 9>(input)?;
    let (words, carry) = tables::decode32(&digits);
    finish_decode(&words, carry, zeros, output)
}

#[inline]
pub fn encode64(input: &[u8; 64], output: &mut [u8]) -> Result<usize, Error> {
    let digits = tables::encode64(&words::<64, 16>(input));
    finish_encode::<64, 18, 90>(input, &digits, output)
}
#[inline]
pub fn decode64(input: &[u8], output: &mut [u8; 64]) -> Result<(), Error> {
    let (digits, zeros) = prepare::<64, 18>(input)?;
    let (words, carry) = tables::decode64(&digits);
    finish_decode(&words, carry, zeros, output)
}
