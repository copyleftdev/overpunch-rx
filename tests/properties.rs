//! Exhaustive byte-alphabet checks and reproducible generated inputs.

use overpunch_rx::{Encoding, Error, FieldSpec, FixedDecimal, SignPolicy, ZeroPolicy};

#[test]
fn every_byte_in_every_position_has_an_explicit_classification() -> Result<(), Error> {
    for encoding in [Encoding::Ascii, Encoding::Ebcdic] {
        let (zero, positive, negative) = match encoding {
            Encoding::Ascii => (b'0', *b"{ABCDEFGHI", *b"}JKLMNOPQR"),
            Encoding::Ebcdic => (
                0xF0,
                [0xC0, 0xC1, 0xC2, 0xC3, 0xC4, 0xC5, 0xC6, 0xC7, 0xC8, 0xC9],
                [0xD0, 0xD1, 0xD2, 0xD3, 0xD4, 0xD5, 0xD6, 0xD7, 0xD8, 0xD9],
            ),
        };
        for policy in [
            SignPolicy::RequireOverpunch,
            SignPolicy::AllowUnsignedPositive,
        ] {
            let field = FieldSpec::new(3, 2, encoding)?.with_sign_policy(policy);
            for position in 0..3 {
                for byte in 0..=u8::MAX {
                    let mut input = [zero, zero, positive[0]];
                    input[position] = byte;
                    let expected_valid = if position < 2 {
                        (zero..=zero + 9).contains(&byte)
                    } else {
                        positive.contains(&byte)
                            || negative.contains(&byte)
                            || (policy == SignPolicy::AllowUnsignedPositive
                                && (zero..=zero + 9).contains(&byte))
                    };
                    assert_eq!(
                        field.decode(&input).is_ok(),
                        expected_valid,
                        "{encoding:?} {policy:?} {input:?}"
                    );
                    if let Ok(value) = field.decode(&input) {
                        let mut output = [0; 3];
                        field.encode(value, &mut output)?;
                        assert_eq!(field.decode(&output)?, value);
                    }
                }
            }
        }
    }
    Ok(())
}

// An independent text-based oracle: the core uses byte arithmetic, this uses the
// standard library's decimal formatter and then replaces only its final digit.
fn reference_ascii(value: FixedDecimal, width: usize) -> Vec<u8> {
    let mut text = format!("{:0width$}", value.coefficient().unsigned_abs()).into_bytes();
    let terminal = text.len() - 1;
    let digit = usize::from(text[terminal] - b'0');
    text[terminal] = if value.is_negative() {
        b"}JKLMNOPQR"[digit]
    } else {
        b"{ABCDEFGHI"[digit]
    };
    text
}

#[test]
fn exhaustive_small_values_match_independent_formatter() -> Result<(), Error> {
    for width in 1..=3 {
        let field = FieldSpec::new(width, 0, Encoding::Ascii)?;
        let limit = 10_i128.pow(u32::from(width));
        for coefficient in (1 - limit)..limit {
            let value = FixedDecimal::new(coefficient, 0)?;
            let reference = reference_ascii(value, usize::from(width));
            let mut output = [0; 3];
            field.encode(value, &mut output[..usize::from(width)])?;
            assert_eq!(&output[..usize::from(width)], reference);
            assert_eq!(field.decode(&reference)?, value);
        }
    }
    Ok(())
}

fn next_random(state: &mut u64) -> u64 {
    // Fixed-seed xorshift for reproducibility, not for security or randomness claims.
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

#[test]
fn generated_values_cover_all_widths_scales_and_both_encodings() -> Result<(), Error> {
    let mut random = 0xD1B5_4A32_D192_ED03;
    for width in 1..=38 {
        for scale in 0..=width {
            for _ in 0..16 {
                let bits = (u128::from(next_random(&mut random)) << 64)
                    | u128::from(next_random(&mut random));
                let magnitude = bits % 10_u128.pow(u32::from(width));
                let mut coefficient = i128::try_from(magnitude).map_err(|_| Error::Overflow)?;
                if next_random(&mut random) & 1 == 1 {
                    coefficient = -coefficient;
                }
                let value = FixedDecimal::new(coefficient, scale)?;
                for encoding in [Encoding::Ascii, Encoding::Ebcdic] {
                    let field = FieldSpec::new(width, scale, encoding)?;
                    let mut output = [0; 38];
                    let target = &mut output[..usize::from(width)];
                    field.encode(value, target)?;
                    assert_eq!(field.decode(target)?, value);
                    if encoding == Encoding::Ascii {
                        assert_eq!(target, reference_ascii(value, usize::from(width)));
                    }
                }
            }
        }
    }
    Ok(())
}

#[test]
fn arbitrary_buffers_and_coefficients_do_not_panic_or_partially_write() -> Result<(), Error> {
    let mut random = 0xDB4F_0B91_75AE_2165;
    for _ in 0..20_000 {
        let bytes = next_random(&mut random).to_le_bytes();
        let width = bytes[0] % 38 + 1;
        let scale = bytes[1] % (width + 1);
        let encoding = if bytes[2] & 1 == 0 {
            Encoding::Ascii
        } else {
            Encoding::Ebcdic
        };
        let zero_policy = match bytes[3] % 3 {
            0 => ZeroPolicy::Preserve,
            1 => ZeroPolicy::NormalizePositive,
            _ => ZeroPolicy::RejectNegative,
        };
        let sign_policy = if bytes[4] & 1 == 0 {
            SignPolicy::RequireOverpunch
        } else {
            SignPolicy::AllowUnsignedPositive
        };
        let field = FieldSpec::new(width, scale, encoding)?
            .with_zero_policy(zero_policy)
            .with_sign_policy(sign_policy);
        let mut input = [0; 64];
        for chunk in input.chunks_exact_mut(8) {
            chunk.copy_from_slice(&next_random(&mut random).to_le_bytes());
        }
        let length = usize::from(bytes[5] % 64);
        if let Ok(value) = field.decode(&input[..length]) {
            let mut output = [0; 38];
            field.encode(value, &mut output[..usize::from(width)])?;
            assert_eq!(field.decode(&output[..usize::from(width)])?, value);
        }
        let mut bits = [0; 16];
        bits.copy_from_slice(&input[..16]);
        let value = FixedDecimal::new(i128::from_le_bytes(bits), bytes[6] % 39)?;
        let mut output = [0xAA; 38];
        let result = field.encode(value, &mut output[..usize::from(width)]);
        if result.is_err() {
            assert_eq!(output, [0xAA; 38]);
        } else {
            assert_eq!(
                field.decode(&output[..usize::from(width)])?,
                value.rescale(scale)?
            );
        }
    }
    Ok(())
}
