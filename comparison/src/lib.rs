//! Independent comparison code; this crate is excluded from the published package.

pub use z58::Error;
#[path = "ablations/radix4_parallel.rs"]
pub mod fixed_radix4_parallel;
#[path = "ablations/radix5_parallel.rs"]
pub mod fixed_radix5_parallel;
#[path = "ablations/radix5_serial.rs"]
pub mod fixed_radix5_serial;
#[path = "../../src/fixed_tables.rs"]
mod fixed_tables;

fn validate(input: &[u8]) -> Result<(), Error> {
    for (index, &byte) in input.iter().enumerate() {
        if DECODE[byte as usize] == 255 {
            return Err(Error::InvalidCharacter { index, byte });
        }
    }
    Ok(())
}

pub const ALPHABET: &[u8; 58] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
pub const DECODE: [u8; 256] = decode_table();
const fn decode_table() -> [u8; 256] {
    let mut table = [255; 256];
    let mut i = 0;
    while i < 58 {
        table[ALPHABET[i] as usize] = i as u8;
        i += 1;
    }
    table
}
pub const SEED: u64 = 0x6a09_e667_f3bc_c909;
pub const LENGTHS: &[usize] = &[0, 1, 2, 8, 16, 32, 64, 128, 256, 1024, 4096];
pub const PATTERNS: &[&str] = &["random", "zero", "ff", "leading-zero", "repeat"];
pub const CORPUS: usize = 16;

pub fn next_random(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

/// The workload is fixed before comparing implementations. Every case owns its
/// deterministic stream so adding an algorithm cannot change any input.
pub fn corpus(length: usize, pattern: &str) -> Vec<Vec<u8>> {
    let mut state = SEED ^ (length as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15);
    (0..CORPUS)
        .map(|i| {
            let mut data = vec![0; length];
            for (j, byte) in data.iter_mut().enumerate() {
                let random = (next_random(&mut state) >> 32) as u8;
                *byte = match pattern {
                    "random" => random,
                    "zero" => 0,
                    "ff" => 255,
                    "leading-zero" => {
                        if j < length * 3 / 4 {
                            0
                        } else {
                            random
                        }
                    }
                    "repeat" => [0x00, 0x11, 0xaa, 0xff][(j + i) % 4],
                    _ => panic!("unknown pattern"),
                };
            }
            data
        })
        .collect()
}

/// Independent elementary base conversion, using one byte and one base58 digit
/// at a time. Work is little-endian; the maximum intermediate is 57*256+255.
pub fn reference_encode(input: &[u8], out: &mut [u8], work: &mut [u8]) -> usize {
    let zeroes = input.iter().take_while(|&&b| b == 0).count();
    let mut used = 0;
    for &byte in &input[zeroes..] {
        let mut carry = u32::from(byte);
        for digit in &mut work[..used] {
            carry += u32::from(*digit) * 256;
            *digit = (carry % 58) as u8;
            carry /= 58;
        }
        while carry != 0 {
            work[used] = (carry % 58) as u8;
            used += 1;
            carry /= 58;
        }
    }
    out[..zeroes].fill(b'1');
    for i in 0..used {
        out[zeroes + i] = ALPHABET[work[used - 1 - i] as usize];
    }
    zeroes + used
}

pub fn reference_decode(input: &[u8], out: &mut [u8], work: &mut [u8]) -> Result<usize, usize> {
    let zeroes = input.iter().take_while(|&&b| b == b'1').count();
    let mut used = 0;
    for (index, &ch) in input.iter().enumerate().skip(zeroes) {
        let digit = DECODE[ch as usize];
        if digit == 255 {
            return Err(index);
        }
        let mut carry = u32::from(digit);
        for byte in &mut work[..used] {
            carry += u32::from(*byte) * 58;
            *byte = carry as u8;
            carry >>= 8;
        }
        while carry != 0 {
            work[used] = carry as u8;
            used += 1;
            carry >>= 8;
        }
    }
    out[..zeroes].fill(0);
    for i in 0..used {
        out[zeroes + i] = work[used - 1 - i];
    }
    Ok(zeroes + used)
}

// Experimental ablations. A u128 intermediate holds (B-1)*2^64 + (2^64-1)
// because B=58^10<2^59, so it is strictly below 2^123. The u32 variant is
// included separately to measure grouping while retaining the same wrappers.
fn grouped_encode<const GROUP: usize, const DIGITS: usize, const BASE: u64>(
    input: &[u8],
    out: &mut [u8],
    work: &mut [u64],
) -> usize {
    let zeroes = input.iter().take_while(|&&b| b == 0).count();
    let mut used = 0;
    for chunk in input[zeroes..].chunks(GROUP) {
        let factor = 1u128 << (8 * chunk.len());
        let mut carry = chunk.iter().fold(0u128, |n, b| n * 256 + u128::from(*b));
        for limb in &mut work[..used] {
            let n = u128::from(*limb) * factor + carry;
            *limb = (n % u128::from(BASE)) as u64;
            carry = n / u128::from(BASE);
        }
        while carry != 0 {
            work[used] = (carry % u128::from(BASE)) as u64;
            used += 1;
            carry /= u128::from(BASE);
        }
    }
    out[..zeroes].fill(b'1');
    if used == 0 {
        return zeroes;
    }
    let mut top = work[used - 1];
    let mut top_digits = 0;
    while top != 0 {
        top /= 58;
        top_digits += 1;
    }
    let len = zeroes + (used - 1) * DIGITS + top_digits;
    let mut end = len;
    for (i, &limb) in work[..used].iter().enumerate() {
        let count = if i + 1 == used { top_digits } else { DIGITS };
        let mut n = limb;
        for _ in 0..count {
            end -= 1;
            out[end] = ALPHABET[(n % 58) as usize];
            n /= 58;
        }
    }
    len
}

fn grouped_decode<const GROUP: usize, const BITS: usize>(
    input: &[u8],
    out: &mut [u8],
    work: &mut [u64],
) -> Result<usize, usize> {
    let zeroes = input.iter().take_while(|&&b| b == b'1').count();
    let mut used = 0;
    let mask = (1u128 << BITS) - 1;
    for (chunk_index, chunk) in input[zeroes..].chunks(GROUP).enumerate() {
        let mut carry = 0u128;
        let mut factor = 1u128;
        for (offset, &byte) in chunk.iter().enumerate() {
            let digit = DECODE[byte as usize];
            if digit == 255 {
                return Err(zeroes + chunk_index * GROUP + offset);
            }
            carry = carry * 58 + u128::from(digit);
            factor *= 58;
        }
        for limb in &mut work[..used] {
            let n = u128::from(*limb) * factor + carry;
            *limb = (n & mask) as u64;
            carry = n >> BITS;
        }
        while carry != 0 {
            work[used] = (carry & mask) as u64;
            used += 1;
            carry >>= BITS;
        }
    }
    out[..zeroes].fill(0);
    if used == 0 {
        return Ok(zeroes);
    }
    let top_bytes = ((64 - work[used - 1].leading_zeros()) as usize).div_ceil(8);
    let len = zeroes + (used - 1) * (BITS / 8) + top_bytes;
    let mut end = len;
    for (i, &limb) in work[..used].iter().enumerate() {
        let count = if i + 1 == used { top_bytes } else { BITS / 8 };
        let mut n = limb;
        for _ in 0..count {
            end -= 1;
            out[end] = n as u8;
            n >>= 8;
        }
    }
    Ok(len)
}

fn radix32_encode_group<const GROUP: usize>(
    input: &[u8],
    out: &mut [u8],
    work: &mut [u64],
) -> usize {
    const BASE: u64 = 656_356_768;
    let zeroes = input.iter().take_while(|&&b| b == 0).count();
    let mut used = 0;
    for chunk in input[zeroes..].chunks(GROUP) {
        let factor = 1u64 << (8 * chunk.len());
        let mut carry = chunk.iter().fold(0u64, |n, b| n * 256 + u64::from(*b));
        for limb in &mut work[..used] {
            // (58^5-1)*2^32 + (2^32-1) < 2^62.
            let n = *limb * factor + carry;
            *limb = n % BASE;
            carry = n / BASE;
        }
        while carry != 0 {
            work[used] = carry % BASE;
            used += 1;
            carry /= BASE;
        }
    }
    out[..zeroes].fill(b'1');
    if used == 0 {
        return zeroes;
    }
    let mut top = work[used - 1];
    let mut top_digits = 0;
    while top != 0 {
        top /= 58;
        top_digits += 1;
    }
    let len = zeroes + (used - 1) * 5 + top_digits;
    let mut end = len;
    for (i, &limb) in work[..used].iter().enumerate() {
        let count = if i + 1 == used { top_digits } else { 5 };
        let mut n = limb;
        for _ in 0..count {
            end -= 1;
            out[end] = ALPHABET[(n % 58) as usize];
            n /= 58;
        }
    }
    len
}

pub fn radix32_encode(input: &[u8], out: &mut [u8], work: &mut [u64]) -> usize {
    radix32_encode_group::<4>(input, out, work)
}
pub fn radix32_single_encode(input: &[u8], out: &mut [u8], work: &mut [u64]) -> usize {
    radix32_encode_group::<1>(input, out, work)
}
pub fn radix32_u128_encode(input: &[u8], out: &mut [u8], work: &mut [u64]) -> usize {
    grouped_encode::<4, 5, 656_356_768>(input, out, work)
}
pub fn radix64_encode(input: &[u8], out: &mut [u8], work: &mut [u64]) -> usize {
    grouped_encode::<8, 10, 430_804_206_899_405_824>(input, out, work)
}
fn radix32_decode_group<const GROUP: usize>(
    input: &[u8],
    out: &mut [u8],
    work: &mut [u64],
) -> Result<usize, usize> {
    let zeroes = input.iter().take_while(|&&b| b == b'1').count();
    let mut used = 0;
    for (group, chunk) in input[zeroes..].chunks(GROUP).enumerate() {
        let mut carry = 0u64;
        let mut factor = 1u64;
        for (offset, &ch) in chunk.iter().enumerate() {
            let digit = DECODE[ch as usize];
            if digit == 255 {
                return Err(zeroes + group * GROUP + offset);
            }
            carry = carry * 58 + u64::from(digit);
            factor *= 58;
        }
        for limb in &mut work[..used] {
            // (2^32-1)*58^5 + (58^5-1) < 2^62.
            let n = *limb * factor + carry;
            *limb = n & 0xffff_ffff;
            carry = n >> 32;
        }
        while carry != 0 {
            work[used] = carry & 0xffff_ffff;
            used += 1;
            carry >>= 32;
        }
    }
    out[..zeroes].fill(0);
    if used == 0 {
        return Ok(zeroes);
    }
    let top_bytes = ((64 - work[used - 1].leading_zeros()) as usize).div_ceil(8);
    let len = zeroes + (used - 1) * 4 + top_bytes;
    let mut end = len;
    for (i, &limb) in work[..used].iter().enumerate() {
        let count = if i + 1 == used { top_bytes } else { 4 };
        let mut n = limb;
        for _ in 0..count {
            end -= 1;
            out[end] = n as u8;
            n >>= 8;
        }
    }
    Ok(len)
}
pub fn radix32_decode(input: &[u8], out: &mut [u8], work: &mut [u64]) -> Result<usize, usize> {
    radix32_decode_group::<5>(input, out, work)
}
pub fn radix32_single_decode(
    input: &[u8],
    out: &mut [u8],
    work: &mut [u64],
) -> Result<usize, usize> {
    radix32_decode_group::<1>(input, out, work)
}
pub fn radix32_u128_decode(input: &[u8], out: &mut [u8], work: &mut [u64]) -> Result<usize, usize> {
    grouped_decode::<5, 32>(input, out, work)
}
pub fn radix64_decode(input: &[u8], out: &mut [u8], work: &mut [u64]) -> Result<usize, usize> {
    grouped_decode::<10, 64>(input, out, work)
}

// Call Braid58's unmodified public C API to give it caller-owned output without
// the additional inline-result-to-slice copy required by its Rust encode API.
// This FFI is confined to comparison; z58 has no C code or unsafe code.
unsafe extern "C" {
    fn braid58_encode_32(input: *const u8, output: *mut std::ffi::c_char) -> usize;
    fn braid58_encode_64(input: *const u8, output: *mut std::ffi::c_char) -> usize;
}
pub fn braid_encode32(input: &[u8; 32], output: &mut [u8; 45]) -> usize {
    // SAFETY: These arrays meet upstream's exact input size and max output+NUL
    // capacity, do not overlap, and braid58's build.rs selects the target ISA.
    unsafe { braid58_encode_32(input.as_ptr(), output.as_mut_ptr().cast()) }
}
pub fn braid_encode64(input: &[u8; 64], output: &mut [u8; 89]) -> usize {
    // SAFETY: As above, with the corresponding 64-byte ABI capacities.
    unsafe { braid58_encode_64(input.as_ptr(), output.as_mut_ptr().cast()) }
}
