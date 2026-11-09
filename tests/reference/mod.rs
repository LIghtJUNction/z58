//! Independent byte-at-a-time reference, deliberately unlike production limbs.

const ALPHABET: &[u8; 58] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

pub fn encode(input: &[u8]) -> Vec<u8> {
    let zeros = input.iter().take_while(|&&b| b == 0).count();
    let mut number = input[zeros..].to_vec();
    let mut start = 0;
    let mut result = Vec::new();
    // Repeated long division of a big-endian byte array; no production tables.
    while start < number.len() {
        let mut remainder = 0u16;
        for byte in &mut number[start..] {
            let value = remainder * 256 + u16::from(*byte);
            *byte = (value / 58) as u8;
            remainder = value % 58;
        }
        result.push(ALPHABET[remainder as usize]);
        while start < number.len() && number[start] == 0 {
            start += 1;
        }
    }
    result.resize(result.len() + zeros, b'1');
    result.reverse();
    result
}

pub fn decode(input: &[u8]) -> Result<Vec<u8>, usize> {
    let zeros = input.iter().take_while(|&&b| b == b'1').count();
    let mut number = Vec::<u8>::new();
    for (index, byte) in input.iter().enumerate() {
        let digit = ALPHABET.iter().position(|b| b == byte).ok_or(index)?;
        let mut carry = digit as u16;
        for byte in &mut number {
            let value = u16::from(*byte) * 58 + carry;
            *byte = value as u8;
            carry = value >> 8;
        }
        if carry != 0 {
            number.push(carry as u8);
        }
    }
    number.resize(number.len() + zeros, 0);
    number.reverse();
    Ok(number)
}

pub fn random(state: &mut u64, len: usize) -> Vec<u8> {
    (0..len)
        .map(|_| {
            *state ^= *state << 13;
            *state ^= *state >> 7;
            *state ^= *state << 17;
            *state as u8
        })
        .collect()
}
