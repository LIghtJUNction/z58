mod reference;

use z58::{Error, decode, encode};

fn check(input: &[u8]) {
    let expected = reference::encode(input);
    assert_eq!(reference::decode(&expected).unwrap(), input);
    let mut output = vec![0xa5; z58::encoded_capacity(input.len()).unwrap() + 7];
    let mut scratch = vec![0xdead_beef; z58::encode_scratch_len(input.len()).unwrap()];
    let len = encode(input, &mut output, &mut scratch).unwrap();
    assert_eq!(&output[..len], expected);
    assert!(output[len..].iter().all(|&b| b == 0xa5));
    let mut decoded = vec![0xa5; input.len() + 7];
    let mut scratch = vec![0xdead_beef; z58::decode_scratch_len(expected.len()).unwrap()];
    let len = decode(&expected, &mut decoded, &mut scratch).unwrap();
    assert_eq!(&decoded[..len], input);
    assert!(decoded[len..].iter().all(|&b| b == 0xa5));
    if input.len() == 32 {
        let value: &[u8; 32] = input.try_into().unwrap();
        let mut text = [0xa5; 50];
        let len = z58::encode32(value, &mut text).unwrap();
        assert_eq!(&text[..len], expected);
        assert!(text[len..].iter().all(|&b| b == 0xa5));
        let mut bytes = [0xa5; 32];
        z58::decode32(&expected, &mut bytes).unwrap();
        assert_eq!(&bytes, input);
    } else if input.len() == 64 {
        let value: &[u8; 64] = input.try_into().unwrap();
        let mut text = [0xa5; 94];
        let len = z58::encode64(value, &mut text).unwrap();
        assert_eq!(&text[..len], expected);
        assert!(text[len..].iter().all(|&b| b == 0xa5));
        let mut bytes = [0xa5; 64];
        z58::decode64(&expected, &mut bytes).unwrap();
        assert_eq!(&bytes, input);
    }
}

#[test]
fn known_vectors() {
    // Bitcoin Core's public base58_encode_decode.json and common literal vectors.
    for (bytes, text) in [
        (&b""[..], &b""[..]),
        (&[0][..], &b"1"[..]),
        (&[0, 0, 0, 1][..], &b"1112"[..]),
        (&[0x61][..], &b"2g"[..]),
        (&[0x62, 0x62, 0x62][..], &b"a3gV"[..]),
        (&[0x63, 0x63, 0x63][..], &b"aPEr"[..]),
        (&[0x57, 0x2e, 0x47, 0x94][..], &b"3EFU7m"[..]),
        (&[0x10, 0xc8, 0x51, 0x1e][..], &b"Rt5zm"[..]),
        (&[0xff][..], &b"5Q"[..]),
        (&b"hello"[..], &b"Cn8eVZg"[..]),
        (&b"hello world"[..], &b"StV1DL6CwTryKyV"[..]),
        (&b"Hello World"[..], &b"JxF12TrwUP45BMd"[..]),
    ] {
        assert_eq!(reference::encode(bytes), text);
        assert_eq!(reference::decode(text).unwrap(), bytes);
        check(bytes);
    }
}

#[test]
fn exhaustive_single_and_double_bytes() {
    for byte in 0..=u8::MAX {
        check(&[byte]);
    }
    for word in 0..=u16::MAX {
        check(&word.to_be_bytes());
    }
}

#[test]
fn exhaustive_two_base58_digits() {
    let alphabet = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
    for &a in alphabet {
        for &b in alphabet {
            let input = [a, b];
            let expected = reference::decode(&input).unwrap();
            let mut output = [0xa5; 2];
            let n = decode(&input, &mut output, &mut [0; 1]).unwrap();
            assert_eq!(&output[..n], expected);
        }
    }
}

#[test]
fn deterministic_random_and_patterns() {
    let mut seed = 0x58d1_ff3a_b82c_917du64;
    for len in [
        0, 1, 2, 3, 4, 5, 7, 8, 15, 16, 31, 32, 33, 63, 64, 65, 128, 256, 1024, 4096,
    ] {
        check(&vec![0; len]);
        check(&vec![255; len]);
        check(&vec![0x55; len]);
        check(&reference::random(&mut seed, len));
        let mut leading = vec![0; len];
        if len != 0 {
            leading[len - 1] = 255;
        }
        check(&leading);
    }
    for i in 0..2000 {
        let len = if i % 3 == 0 {
            32
        } else if i % 3 == 1 {
            64
        } else {
            i % 257
        };
        let mut bytes = reference::random(&mut seed, len);
        let prefix = if len == 0 { 0 } else { i % len };
        bytes[..prefix].fill(0);
        check(&bytes);
    }
}

#[test]
fn every_fixed_leading_zero_count() {
    for n in [32, 64] {
        for zeros in 0..=n {
            let mut bytes = vec![255; n];
            bytes[..zeros].fill(0);
            check(&bytes);
            if zeros < n {
                bytes[zeros] = 1;
                check(&bytes);
            }
        }
    }
}

#[test]
fn exact_output_capacity_and_atomic_errors() {
    for input in [
        &b""[..],
        &b"a"[..],
        &b"hello"[..],
        &[0; 32][..],
        &[255; 64][..],
    ] {
        let expected = reference::encode(input);
        let mut scratch = vec![0; z58::encode_scratch_len(input.len()).unwrap()];
        let mut exact = vec![0; expected.len()];
        assert_eq!(
            encode(input, &mut exact, &mut scratch).unwrap(),
            expected.len()
        );
        assert_eq!(exact, expected);
        if !expected.is_empty() {
            let mut small = vec![0xa5; expected.len() - 1];
            assert_eq!(
                encode(input, &mut small, &mut scratch),
                Err(Error::OutputTooSmall {
                    required: expected.len(),
                    available: small.len(),
                })
            );
            assert!(small.iter().all(|&b| b == 0xa5));
        }
        let mut scratch = vec![0; z58::decode_scratch_len(expected.len()).unwrap()];
        let mut exact = vec![0; input.len()];
        assert_eq!(
            decode(&expected, &mut exact, &mut scratch).unwrap(),
            input.len()
        );
        assert_eq!(exact, input);
        if !input.is_empty() {
            let mut small = vec![0xa5; input.len() - 1];
            assert_eq!(
                decode(&expected, &mut small, &mut scratch),
                Err(Error::OutputTooSmall {
                    required: input.len(),
                    available: small.len(),
                })
            );
            assert!(small.iter().all(|&b| b == 0xa5));
        }
    }
    let mut out = [0xa5; 100];
    let mut scratch = [0xdead_beef; 1];
    assert!(matches!(
        encode(&[255; 32], &mut out, &mut scratch),
        Err(Error::ScratchTooSmall { .. })
    ));
    assert_eq!(out, [0xa5; 100]);
    assert_eq!(scratch, [0xdead_beef; 1]);
    assert!(matches!(
        decode(&[b'z'; 44], &mut out, &mut scratch),
        Err(Error::ScratchTooSmall { .. })
    ));
    assert_eq!(out, [0xa5; 100]);
    assert_eq!(scratch, [0xdead_beef; 1]);
    assert_eq!(encode(&[0; 32], &mut out[..32], &mut []), Ok(32));
    assert_eq!(decode(&[b'1'; 32], &mut out[..32], &mut []), Ok(32));
    for capacity in 0..44 {
        let mut out = [0xa5; 44];
        assert_eq!(
            z58::encode32(&[255; 32], &mut out[..capacity]),
            Err(Error::OutputTooSmall {
                required: 44,
                available: capacity,
            })
        );
        assert_eq!(out, [0xa5; 44]);
    }
    for capacity in 0..88 {
        let mut out = [0xa5; 88];
        assert_eq!(
            z58::encode64(&[255; 64], &mut out[..capacity]),
            Err(Error::OutputTooSmall {
                required: 88,
                available: capacity,
            })
        );
        assert_eq!(out, [0xa5; 88]);
    }
}

#[test]
fn rejects_every_invalid_byte_at_every_position() {
    let alphabet = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
    for len in [1, 32, 44, 64, 88] {
        for index in 0..len {
            for byte in 0..=255u8 {
                if alphabet.contains(&byte) {
                    continue;
                }
                let mut text = vec![b'1'; len];
                text[index] = byte;
                let expected = Err(Error::InvalidCharacter { index, byte });
                let mut output = [0xa5; 128];
                let mut scratch = [0xdead_beef; 32];
                assert_eq!(decode(&text, &mut output, &mut scratch), expected);
                assert_eq!(output, [0xa5; 128]);
                assert_eq!(scratch, [0xdead_beef; 32]);
                let mut out32 = [0xa5; 32];
                let mut out64 = [0xa5; 64];
                assert_eq!(z58::decode32(&text, &mut out32), expected.map(|_| ()));
                assert_eq!(z58::decode64(&text, &mut out64), expected.map(|_| ()));
                assert_eq!(out32, [0xa5; 32]);
                assert_eq!(out64, [0xa5; 64]);
            }
        }
    }
    assert_eq!(
        decode(b"12O0", &mut [], &mut []),
        Err(Error::InvalidCharacter {
            index: 2,
            byte: b'O'
        })
    );
    assert_eq!(
        decode("1é".as_bytes(), &mut [], &mut []),
        Err(Error::InvalidCharacter {
            index: 1,
            byte: 0xc3
        })
    );
}

fn fixed_error(input: &[u8], expected32: Error, expected64: Error) {
    let mut a = [0xa5; 32];
    let mut b = [0xa5; 64];
    assert_eq!(z58::decode32(input, &mut a), Err(expected32));
    assert_eq!(z58::decode64(input, &mut b), Err(expected64));
    assert_eq!(a, [0xa5; 32]);
    assert_eq!(b, [0xa5; 64]);
}

#[test]
fn fixed_lengths_and_integer_overflow() {
    for actual in [0, 1, 31, 33, 63, 65, 128, 65536] {
        fixed_error(
            &vec![b'1'; actual],
            Error::InvalidLength {
                expected: 32,
                actual,
            },
            Error::InvalidLength {
                expected: 64,
                actual,
            },
        );
    }
    for actual in [1, 31, 33, 63, 65] {
        let text = reference::encode(&vec![255; actual]);
        let e32 = if actual > 32 {
            Error::Overflow
        } else {
            Error::InvalidLength {
                expected: 32,
                actual,
            }
        };
        let e64 = if actual > 64 {
            Error::Overflow
        } else {
            Error::InvalidLength {
                expected: 64,
                actual,
            }
        };
        fixed_error(&text, e32, e64);
    }
    for n in [32, 64] {
        let mut overflow = vec![0; n + 1];
        overflow[0] = 1; // exactly 2^(8*N)
        let text = reference::encode(&overflow);
        let mut out32 = [0xa5; 32];
        let mut out64 = [0xa5; 64];
        if n == 32 {
            assert_eq!(z58::decode32(&text, &mut out32), Err(Error::Overflow));
            assert_eq!(out32, [0xa5; 32]);
        } else {
            assert_eq!(z58::decode64(&text, &mut out64), Err(Error::Overflow));
            assert_eq!(out64, [0xa5; 64]);
        }
        let mut extra_zero = vec![0];
        extra_zero.extend_from_slice(&vec![255; n]);
        let text = reference::encode(&extra_zero);
        if n == 32 {
            assert_eq!(
                z58::decode32(&text, &mut out32),
                Err(Error::InvalidLength {
                    expected: 32,
                    actual: 33
                })
            );
        } else {
            assert_eq!(
                z58::decode64(&text, &mut out64),
                Err(Error::InvalidLength {
                    expected: 64,
                    actual: 65
                })
            );
        }
    }
    fixed_error(&[b'z'; 89], Error::Overflow, Error::Overflow);
}

#[test]
fn checked_capacity_boundaries() {
    for n in (0..10000).chain([
        usize::MAX,
        usize::MAX - 1,
        usize::MAX / 138,
        usize::MAX / 2,
        usize::MAX / 138 * 100,
        usize::MAX / 138 * 100 + 99,
    ]) {
        let wide = n as u128;
        let cap = (wide * 138 + 99) / 100;
        let expected = usize::try_from(cap).ok();
        assert_eq!(z58::encoded_capacity(n), expected);
        assert_eq!(z58::decoded_capacity(n), Some(n));
        assert_eq!(
            z58::encode_scratch_len(n),
            Some(((wide * 138 + 499) / 500) as usize)
        );
        assert_eq!(
            z58::decode_scratch_len(n),
            Some((wide / 4 + u128::from(n % 4 != 0)) as usize)
        );
    }
}

#[test]
fn miri_smoke() {
    known_vectors();
    checked_capacity_boundaries();
    let mut seed = 0x8b6d_73c4_5a21_9eff;
    for len in [3, 4, 5, 9, 16, 31, 32, 33, 63, 64, 65, 256] {
        check(&reference::random(&mut seed, len));
    }
    exact_output_capacity_and_atomic_errors();
    for n in [32, 64] {
        for zeros in [0, 1, n - 1, n] {
            let mut bytes = vec![255; n];
            bytes[..zeros].fill(0);
            check(&bytes);
            let expected = reference::encode(&bytes);
            let mut exact = vec![0; expected.len()];
            if n == 32 {
                assert_eq!(
                    z58::encode32(bytes.as_slice().try_into().unwrap(), &mut exact),
                    Ok(expected.len())
                );
            } else {
                assert_eq!(
                    z58::encode64(bytes.as_slice().try_into().unwrap(), &mut exact),
                    Ok(expected.len())
                );
            }
            assert_eq!(exact, expected);
        }
        let mut overflow = vec![0; n + 1];
        overflow[0] = 1;
        let text = reference::encode(&overflow);
        if n == 32 {
            let mut output = [0xa5; 32];
            assert_eq!(z58::decode32(&text, &mut output), Err(Error::Overflow));
            assert_eq!(output, [0xa5; 32]);
        } else {
            let mut output = [0xa5; 64];
            assert_eq!(z58::decode64(&text, &mut output), Err(Error::Overflow));
            assert_eq!(output, [0xa5; 64]);
        }
    }
    for actual in [31, 33, 63, 65] {
        fixed_error(
            &vec![b'1'; actual],
            Error::InvalidLength {
                expected: 32,
                actual,
            },
            Error::InvalidLength {
                expected: 64,
                actual,
            },
        );
    }
    fixed_error(
        b"0",
        Error::InvalidCharacter {
            index: 0,
            byte: b'0',
        },
        Error::InvalidCharacter {
            index: 0,
            byte: b'0',
        },
    );
    fixed_error(&[b'z'; 88], Error::Overflow, Error::Overflow);
}
