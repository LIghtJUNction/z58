use crate::{check_output, decode, encode, validate, Error};

/// Encode exactly 32 bytes, returning a variable length of 32 through 44 bytes.
/// Output is unchanged on error; bytes after the returned length are untouched.
pub fn encode32(input: &[u8; 32], output: &mut [u8]) -> Result<usize, Error> {
    encode(input, output, &mut [0; 9])
}

/// Encode exactly 64 bytes, returning a variable length of 64 through 88 bytes.
/// Output is unchanged on error; bytes after the returned length are untouched.
pub fn encode64(input: &[u8; 64], output: &mut [u8]) -> Result<usize, Error> {
    encode(input, output, &mut [0; 18])
}

/// Decode strict Base58 into exactly 32 bytes without padding or truncation.
/// Invalid characters take precedence; numeric overflow and wrong length are errors.
/// The entire output is unchanged on any error.
pub fn decode32(input: &[u8], output: &mut [u8; 32]) -> Result<(), Error> {
    decode_fixed(input, output, 44)
}

/// Decode strict Base58 into exactly 64 bytes without padding or truncation.
/// Invalid characters take precedence; numeric overflow and wrong length are errors.
/// The entire output is unchanged on any error.
pub fn decode64(input: &[u8], output: &mut [u8; 64]) -> Result<(), Error> {
    decode_fixed(input, output, 88)
}

fn decode_fixed<const N: usize>(
    input: &[u8],
    output: &mut [u8; N],
    max: usize,
) -> Result<(), Error> {
    validate(input)?;
    let zeros = input.iter().take_while(|&&b| b == b'1').count();
    let body = &input[zeros..];
    if body.len() > max {
        return Err(Error::Overflow);
    }
    let mut bytes = [0; 64];
    let size = match decode(body, &mut bytes[..N], &mut [0; 22]) {
        Err(Error::OutputTooSmall { .. }) => return Err(Error::Overflow),
        result => result?,
    };
    let actual = zeros.checked_add(size).ok_or(Error::CapacityOverflow)?;
    if actual != N {
        return Err(Error::InvalidLength {
            expected: N,
            actual,
        });
    }
    check_output(actual, output.len())?;
    output[..zeros].fill(0);
    output[zeros..].copy_from_slice(&bytes[..size]);
    Ok(())
}
