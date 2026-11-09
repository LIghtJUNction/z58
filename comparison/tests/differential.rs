use z58_comparison::*;

fn check(input: &[u8]) {
    let capacity = z58::encoded_capacity(input.len()).unwrap();
    let mut expected = vec![0; capacity];
    let mut output = vec![0xa5; capacity];
    let mut byte_work = vec![0; capacity];
    let mut wide_work = vec![0; capacity];
    let mut work = vec![0; z58::encode_scratch_len(input.len()).unwrap()];
    let n = bs58::encode(input).onto(&mut expected[..]).unwrap();
    expected.truncate(n);
    let r = reference_encode(input, &mut output, &mut byte_work);
    assert_eq!(&output[..r], expected);
    let r = z58::encode(input, &mut output, &mut work).unwrap();
    assert_eq!(&output[..r], expected);
    let r = radix32_single_encode(input, &mut output, &mut wide_work);
    assert_eq!(&output[..r], expected);
    let r = radix32_encode(input, &mut output, &mut wide_work);
    assert_eq!(&output[..r], expected);
    let r = radix32_u128_encode(input, &mut output, &mut wide_work);
    assert_eq!(&output[..r], expected);
    let r = radix64_encode(input, &mut output, &mut wide_work);
    assert_eq!(&output[..r], expected);
    work.resize(z58::decode_scratch_len(n).unwrap(), 0);
    let r = z58::decode(&expected, &mut output, &mut work).unwrap();
    assert_eq!(&output[..r], input);
    let r = reference_decode(&expected, &mut output, &mut byte_work).unwrap();
    assert_eq!(&output[..r], input);
    let r = radix32_single_decode(&expected, &mut output, &mut wide_work).unwrap();
    assert_eq!(&output[..r], input);
    let r = radix32_decode(&expected, &mut output, &mut wide_work).unwrap();
    assert_eq!(&output[..r], input);
    let r = radix32_u128_decode(&expected, &mut output, &mut wide_work).unwrap();
    assert_eq!(&output[..r], input);
    let r = radix64_decode(&expected, &mut output, &mut wide_work).unwrap();
    assert_eq!(&output[..r], input);
    if input.len() == 32 {
        let len = fixed_radix5_serial::encode32(input.try_into().unwrap(), &mut output).unwrap();
        assert_eq!(&output[..len], expected);
        let mut out = [0; 32];
        fixed_radix5_serial::decode32(&expected, &mut out).unwrap();
        assert_eq!(out, input);

        let len = fixed_radix5_parallel::encode32(input.try_into().unwrap(), &mut output).unwrap();
        assert_eq!(&output[..len], expected);
        let mut out = [0; 32];
        fixed_radix5_parallel::decode32(&expected, &mut out).unwrap();
        assert_eq!(out, input);

        let len = fixed_radix4_parallel::encode32(input.try_into().unwrap(), &mut output).unwrap();
        assert_eq!(&output[..len], expected);
        let mut out = [0; 32];
        fixed_radix4_parallel::decode32(&expected, &mut out).unwrap();
        assert_eq!(out, input);

        let mut b = [0; 45];
        let len = braid_encode32(input.try_into().unwrap(), &mut b);
        assert_eq!(&b[..len], expected);
        let mut b = [0; 44];
        let len = five8::encode_32(input.try_into().unwrap(), &mut b) as usize;
        assert_eq!(&b[..len], expected);
        let len = z58::encode32(input.try_into().unwrap(), &mut b).unwrap();
        assert_eq!(&b[..len], expected);
        let mut out = [0; 32];
        five8::decode_32(&expected, &mut out).unwrap();
        assert_eq!(out, input);
        braid58::decode_32_into(&expected, &mut out).unwrap();
        assert_eq!(out, input);
        assert_eq!(fd_bs58::decode_32(&expected).unwrap(), input);
        z58::decode32(&expected, &mut out).unwrap();
        assert_eq!(out, input);
    } else if input.len() == 64 {
        let len = fixed_radix5_serial::encode64(input.try_into().unwrap(), &mut output).unwrap();
        assert_eq!(&output[..len], expected);
        let mut out = [0; 64];
        fixed_radix5_serial::decode64(&expected, &mut out).unwrap();
        assert_eq!(out, input);

        let len = fixed_radix5_parallel::encode64(input.try_into().unwrap(), &mut output).unwrap();
        assert_eq!(&output[..len], expected);
        let mut out = [0; 64];
        fixed_radix5_parallel::decode64(&expected, &mut out).unwrap();
        assert_eq!(out, input);

        let len = fixed_radix4_parallel::encode64(input.try_into().unwrap(), &mut output).unwrap();
        assert_eq!(&output[..len], expected);
        let mut out = [0; 64];
        fixed_radix4_parallel::decode64(&expected, &mut out).unwrap();
        assert_eq!(out, input);

        let mut b = [0; 89];
        let len = braid_encode64(input.try_into().unwrap(), &mut b);
        assert_eq!(&b[..len], expected);
        let mut b = [0; 88];
        let len = five8::encode_64(input.try_into().unwrap(), &mut b) as usize;
        assert_eq!(&b[..len], expected);
        let len = z58::encode64(input.try_into().unwrap(), &mut b).unwrap();
        assert_eq!(&b[..len], expected);
        let mut out = [0; 64];
        five8::decode_64(&expected, &mut out).unwrap();
        assert_eq!(out, input);
        braid58::decode_64_into(&expected, &mut out).unwrap();
        assert_eq!(out, input);
        assert_eq!(fd_bs58::decode_64(&expected).unwrap(), input);
        z58::decode64(&expected, &mut out).unwrap();
        assert_eq!(out, input);
    }
}

#[test]
fn every_byte_and_pair_against_bs58() {
    for byte in 0..=255 {
        check(&[byte]);
    }
    for pair in 0u32..=65535 {
        check(&(pair as u16).to_be_bytes());
    }
}

#[test]
fn predetermined_patterns_and_large_inputs() {
    for &len in LENGTHS {
        for &pattern in PATTERNS {
            for value in corpus(len, pattern)
                .iter()
                .take(if len > 256 { 2 } else { 16 })
            {
                check(value);
            }
        }
    }
}

#[test]
fn fixed_random_and_every_leading_zero_count() {
    let mut state = SEED;
    for n in [32, 64] {
        for i in 0..2048 {
            let mut input = vec![0; n];
            for b in &mut input {
                *b = next_random(&mut state) as u8;
            }
            input[..i % (n + 1)].fill(0);
            check(&input);
        }
    }
}

fn strict_agreement(text: &[u8]) {
    let mut standard = vec![0; text.len()];
    let accepted = bs58::decode(text).onto(&mut standard[..]);
    let mut work = vec![0; z58::decode_scratch_len(text.len()).unwrap()];
    let mut generic = vec![0; text.len()];
    let actual = z58::decode(text, &mut generic, &mut work);
    assert_eq!(actual.is_ok(), accepted.is_ok(), "generic {text:?}");
    if let (Ok(a), Ok(b)) = (actual, &accepted) {
        assert_eq!(&generic[..a], &standard[..*b]);
    }
    let want32 = accepted.as_ref().is_ok_and(|&n| n == 32);
    let want64 = accepted.as_ref().is_ok_and(|&n| n == 64);
    let mut out32 = [0xa5; 32];
    let mut out64 = [0xa5; 64];
    assert_eq!(
        z58::decode32(text, &mut out32).is_ok(),
        want32,
        "32 {text:?}"
    );
    assert_eq!(
        z58::decode64(text, &mut out64).is_ok(),
        want64,
        "64 {text:?}"
    );
    assert_eq!(
        fd_bs58::decode_32(text).is_ok(),
        want32,
        "fd-bs58 32 {text:?}"
    );
    assert_eq!(
        fd_bs58::decode_64(text).is_ok(),
        want64,
        "fd-bs58 64 {text:?}"
    );
    let mut ablation = [0; 32];
    assert_eq!(
        fixed_radix5_serial::decode32(text, &mut ablation).is_ok(),
        want32
    );
    if want32 {
        assert_eq!(ablation, out32);
    }
    let mut ablation = [0; 64];
    assert_eq!(
        fixed_radix5_serial::decode64(text, &mut ablation).is_ok(),
        want64
    );
    if want64 {
        assert_eq!(ablation, out64);
    }
    let mut ablation = [0; 32];
    assert_eq!(
        fixed_radix5_parallel::decode32(text, &mut ablation).is_ok(),
        want32
    );
    if want32 {
        assert_eq!(ablation, out32);
    }
    let mut ablation = [0; 64];
    assert_eq!(
        fixed_radix5_parallel::decode64(text, &mut ablation).is_ok(),
        want64
    );
    if want64 {
        assert_eq!(ablation, out64);
    }
    let mut ablation = [0; 32];
    assert_eq!(
        fixed_radix4_parallel::decode32(text, &mut ablation).is_ok(),
        want32
    );
    if want32 {
        assert_eq!(ablation, out32);
    }
    let mut ablation = [0; 64];
    assert_eq!(
        fixed_radix4_parallel::decode64(text, &mut ablation).is_ok(),
        want64
    );
    if want64 {
        assert_eq!(ablation, out64);
    }
    let mut five32 = [0; 32];
    let mut five64 = [0; 64];
    assert_eq!(
        five8::decode_32(text, &mut five32).is_ok(),
        want32,
        "five8 32 {text:?}"
    );
    assert_eq!(
        five8::decode_64(text, &mut five64).is_ok(),
        want64,
        "five8 64 {text:?}"
    );
    let mut braid32 = [0; 32];
    let mut braid64 = [0; 64];
    assert_eq!(
        braid58::decode_32_into(text, &mut braid32).is_ok(),
        want32,
        "braid58 32 {text:?}"
    );
    assert_eq!(
        braid58::decode_64_into(text, &mut braid64).is_ok(),
        want64,
        "braid58 64 {text:?}"
    );
    if want32 {
        assert_eq!(out32, standard[..32]);
        assert_eq!(five32, out32);
        assert_eq!(braid32, out32);
    }
    if want64 {
        assert_eq!(out64, standard[..64]);
        assert_eq!(five64, out64);
        assert_eq!(braid64, out64);
    }
}

#[test]
fn strict_rejection_and_fixed_width_agreement() {
    for n in 0..100 {
        strict_agreement(&vec![b'1'; n]);
        strict_agreement(&vec![b'z'; n]);
    }
    for &n in &[32, 64] {
        let mut text = vec![0; z58::encoded_capacity(n).unwrap()];
        let len = bs58::encode(vec![255; n]).onto(&mut text[..]).unwrap();
        text.truncate(len);
        for index in 0..text.len() {
            let saved = text[index];
            for byte in 0..=255 {
                if ALPHABET.contains(&byte) {
                    continue;
                }
                text[index] = byte;
                strict_agreement(&text);
            }
            text[index] = saved;
        }
    }
    let mut state = SEED;
    for i in 0..4096 {
        let mut text = vec![0; i % 97];
        for ch in &mut text {
            let random = next_random(&mut state);
            *ch = if i % 3 == 0 {
                random as u8
            } else {
                ALPHABET[random as usize % 58]
            };
        }
        strict_agreement(&text);
    }
}
