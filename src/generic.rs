use crate::{
    check_output, check_scratch, decode_scratch_len, encode_scratch_len, validate, Error, ALPHABET,
    DECODE,
};

const RADIX: u64 = 656_356_768; // 58^5
const POW58: [u64; 6] = [1, 58, 3_364, 195_112, 11_316_496, RADIX];

/// Encode arbitrary bytes with caller-owned output and `u32` workspace.
///
/// Returns the actual encoded length. See [`crate::encode_scratch_len`] for a
/// workspace upper bound and the crate documentation for mutation guarantees.
pub fn encode(input: &[u8], output: &mut [u8], scratch: &mut [u32]) -> Result<usize, Error> {
    let zeros = input.iter().take_while(|&&b| b == 0).count();
    let body = &input[zeros..];
    let needed = encode_scratch_len(body.len()).ok_or(Error::CapacityOverflow)?;
    check_scratch(needed, scratch.len())?;
    let mut used = 0;
    for block in body.chunks(4) {
        let mut carry = 0u64;
        for &byte in block {
            carry = (carry << 8) | u64::from(byte);
        }
        let factor = 1u64 << (8 * block.len());
        for limb in &mut scratch[..used] {
            // limb < 58^5 and carry < factor <= 2^32:
            // value <= 58^5 * 2^32 - 1 < 2^62; quotient < factor.
            let value = u64::from(*limb) * factor + carry;
            *limb = (value % RADIX) as u32;
            carry = value / RADIX;
        }
        while carry != 0 {
            scratch[used] = (carry % RADIX) as u32;
            carry /= RADIX;
            used += 1;
        }
    }
    let digits = if used == 0 {
        0
    } else {
        let mut n = scratch[used - 1];
        let mut top = 0;
        while n != 0 {
            n /= 58;
            top += 1;
        }
        (used - 1)
            .checked_mul(5)
            .and_then(|n| n.checked_add(top))
            .ok_or(Error::CapacityOverflow)?
    };
    let len = zeros.checked_add(digits).ok_or(Error::CapacityOverflow)?;
    check_output(len, output.len())?;
    output[..zeros].fill(b'1');
    let mut end = len;
    for &limb in &scratch[..used] {
        let mut value = limb;
        let count = core::cmp::min(5, end - zeros);
        for _ in 0..count {
            end -= 1;
            output[end] = ALPHABET[(value % 58) as usize];
            value /= 58;
        }
    }
    Ok(len)
}

/// Decode strict Bitcoin Base58 with caller-owned output and `u32` workspace.
///
/// Returns the actual decoded length, preserving every leading `1` as a zero.
/// See [`crate::decode_scratch_len`] and the crate-level mutation guarantees.
pub fn decode(input: &[u8], output: &mut [u8], scratch: &mut [u32]) -> Result<usize, Error> {
    validate(input)?;
    let zeros = input.iter().take_while(|&&b| b == b'1').count();
    let body = &input[zeros..];
    let needed = decode_scratch_len(body.len()).ok_or(Error::CapacityOverflow)?;
    check_scratch(needed, scratch.len())?;
    let mut used = 0;
    for block in body.chunks(5) {
        let mut carry = 0u64;
        for &byte in block {
            carry = carry * 58 + u64::from(DECODE[byte as usize]);
        }
        let factor = POW58[block.len()];
        for limb in &mut scratch[..used] {
            // limb <= 2^32-1 and carry < factor <= 58^5:
            // value <= 2^32 * 58^5 - 1 < 2^62; high 32 bits < factor.
            let value = u64::from(*limb) * factor + carry;
            *limb = value as u32;
            carry = value >> 32;
        }
        if carry != 0 {
            scratch[used] = carry as u32;
            used += 1;
        }
    }
    let bytes = if used == 0 {
        0
    } else {
        (used - 1)
            .checked_mul(4)
            .and_then(|n| n.checked_add(4 - scratch[used - 1].leading_zeros() as usize / 8))
            .ok_or(Error::CapacityOverflow)?
    };
    let len = zeros.checked_add(bytes).ok_or(Error::CapacityOverflow)?;
    check_output(len, output.len())?;
    output[..zeros].fill(0);
    let mut end = len;
    for &limb in &scratch[..used] {
        for byte in limb
            .to_le_bytes()
            .iter()
            .take(core::cmp::min(4, end - zeros))
        {
            end -= 1;
            output[end] = *byte;
        }
    }
    Ok(len)
}
