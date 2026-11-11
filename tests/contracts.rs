//! Public API contracts and regressions for malformed legacy data.

use overpunch_rx::{Encoding, Error, FieldSpec, FixedDecimal, SignPolicy, ZeroPolicy};

#[test]
fn published_pharmacy_vectors() -> Result<(), Error> {
    // Independent examples from the NCPDP/TPA documentation, with implied cents.
    for (wire, coefficient) in [
        ("00000000035A", 351),
        ("00000000035P", -357),
        ("999E", 9995),
        ("25}", -250),
    ] {
        let width = u8::try_from(wire.len()).map_err(|_| Error::Overflow)?;
        let field = FieldSpec::new(width, 2, Encoding::Ascii)?;
        let expected = FixedDecimal::new(coefficient, 2)?;
        assert_eq!(field.decode(wire.as_bytes())?, expected);
        let mut output = [0xAA; 38];
        field.encode(expected, &mut output[..wire.len()])?;
        assert_eq!(&output[..wire.len()], wire.as_bytes());
    }
    Ok(())
}

#[test]
fn schemas_reject_invalid_bounds() {
    for width in [0, 39, 255] {
        assert_eq!(
            FieldSpec::new(width, 0, Encoding::Ascii),
            Err(Error::InvalidWidth { width })
        );
    }
    assert_eq!(
        FieldSpec::new(3, 4, Encoding::Ascii),
        Err(Error::InvalidScale {
            scale: 4,
            maximum: 3
        })
    );
    assert_eq!(
        FixedDecimal::new(0, 39),
        Err(Error::InvalidScale {
            scale: 39,
            maximum: 38
        })
    );
    assert_eq!(
        FixedDecimal::negative_zero(255),
        Err(Error::InvalidScale {
            scale: 255,
            maximum: 38
        })
    );
}

#[test]
fn strict_signs_and_unsigned_opt_in() -> Result<(), Error> {
    for (encoding, unsigned, signed) in [
        (Encoding::Ascii, *b"123", *b"12C"),
        (Encoding::Ebcdic, [0xF1, 0xF2, 0xF3], [0xF1, 0xF2, 0xC3]),
    ] {
        let strict = FieldSpec::new(3, 2, encoding)?;
        assert_eq!(strict.decode(&unsigned), Err(Error::UnsignedNotAllowed));
        let field = strict.with_sign_policy(SignPolicy::AllowUnsignedPositive);
        assert_eq!(field.decode(&unsigned)?.coefficient(), 123);
        let mut output = [0; 3];
        field.encode(field.decode(&unsigned)?, &mut output)?;
        assert_eq!(output, signed);
    }
    Ok(())
}

#[test]
fn misplaced_signs_spaces_unicode_and_mixed_encodings_are_errors() -> Result<(), Error> {
    let field = FieldSpec::new(3, 2, Encoding::Ascii)?;
    for wire in [
        *b"J2C",
        *b"1}C",
        *b" 2C",
        *b"1.C",
        *b"+2C",
        [0xF1, b'2', b'C'],
    ] {
        assert!(matches!(
            field.decode(&wire),
            Err(Error::InvalidDigit { .. })
        ));
    }
    assert_eq!(
        field.decode(b"12!"),
        Err(Error::InvalidSign {
            index: 2,
            byte: b'!'
        })
    );
    assert_eq!(
        field.decode("éC".as_bytes()),
        Err(Error::InvalidDigit {
            index: 0,
            byte: 0xC3
        })
    );
    // An internal sign must never be mistaken for an ordinary digit.
    assert_eq!(
        field.decode(b"J2C"),
        Err(Error::InvalidDigit {
            index: 0,
            byte: b'J'
        })
    );
    Ok(())
}

#[test]
fn negative_zero_is_explicit_in_both_directions() -> Result<(), Error> {
    for (encoding, negative_wire, positive_wire) in [
        (Encoding::Ascii, *b"00}", *b"00{"),
        (Encoding::Ebcdic, [0xF0, 0xF0, 0xD0], [0xF0, 0xF0, 0xC0]),
    ] {
        let field = FieldSpec::new(3, 2, encoding)?;
        let negative = field.decode(&negative_wire)?;
        assert_eq!(negative, FixedDecimal::negative_zero(2)?);
        assert!(negative.is_negative());
        assert_ne!(negative, FixedDecimal::new(0, 2)?);
        let mut output = [0; 3];
        field.encode(negative, &mut output)?;
        assert_eq!(output, negative_wire);
        let normalized = field.with_zero_policy(ZeroPolicy::NormalizePositive);
        assert_eq!(normalized.decode(&negative_wire)?, FixedDecimal::new(0, 2)?);
        normalized.encode(negative, &mut output)?;
        assert_eq!(output, positive_wire);
        let rejecting = field.with_zero_policy(ZeroPolicy::RejectNegative);
        assert_eq!(rejecting.decode(&negative_wire), Err(Error::NegativeZero));
        assert_eq!(
            rejecting.encode(negative, &mut output),
            Err(Error::NegativeZero)
        );
        assert_eq!(output, positive_wire);
    }
    Ok(())
}

#[test]
fn exact_rescaling_never_rounds() -> Result<(), Error> {
    assert_eq!(
        FixedDecimal::new(123, 1)?.rescale(2)?,
        FixedDecimal::new(1230, 2)?
    );
    assert_eq!(
        FixedDecimal::new(-1230, 2)?.rescale(1)?,
        FixedDecimal::new(-123, 1)?
    );
    assert_eq!(
        FixedDecimal::new(-123, 2)?.rescale(1),
        Err(Error::PrecisionLoss)
    );
    assert_eq!(
        FixedDecimal::new(123, 2)?.rescale(1),
        Err(Error::PrecisionLoss)
    );
    assert_eq!(
        FixedDecimal::new(i128::MAX, 0)?.rescale(1),
        Err(Error::Overflow)
    );
    assert_eq!(
        FixedDecimal::new(i128::MIN, 0)?.rescale(1),
        Err(Error::Overflow)
    );
    assert_eq!(
        FixedDecimal::new(i128::MIN, 38)?.rescale(38)?.coefficient(),
        i128::MIN
    );
    assert_eq!(
        FixedDecimal::negative_zero(0)?.rescale(38)?,
        FixedDecimal::negative_zero(38)?
    );
    assert_eq!(
        FixedDecimal::new(1, 0)?.rescale(39),
        Err(Error::InvalidScale {
            scale: 39,
            maximum: 38
        })
    );
    assert_ne!(FixedDecimal::new(10, 1)?, FixedDecimal::new(100, 2)?);
    Ok(())
}

#[test]
fn output_is_unchanged_for_every_error_class() -> Result<(), Error> {
    let field = FieldSpec::new(3, 2, Encoding::Ascii)?;
    for (value, expected) in [
        (FixedDecimal::new(1000, 2)?, Error::Overflow),
        (FixedDecimal::new(-1000, 2)?, Error::Overflow),
        (FixedDecimal::new(i128::MAX, 0)?, Error::Overflow),
        (FixedDecimal::new(i128::MIN, 2)?, Error::Overflow),
        (FixedDecimal::new(1234, 3)?, Error::PrecisionLoss),
        (FixedDecimal::negative_zero(2)?, Error::NegativeZero),
    ] {
        let mut output = [0xAA; 3];
        let rejecting = field.with_zero_policy(ZeroPolicy::RejectNegative);
        assert_eq!(rejecting.encode(value, &mut output), Err(expected));
        assert_eq!(output, [0xAA; 3]);
    }
    for length in [0, 1, 2, 4, 40] {
        let mut output = [0xAA; 40];
        assert_eq!(
            field.encode(FixedDecimal::new(1, 2)?, &mut output[..length]),
            Err(Error::LengthMismatch {
                expected: 3,
                actual: length
            })
        );
        assert_eq!(output, [0xAA; 40]);
    }
    Ok(())
}

#[test]
fn full_width_and_full_precision_boundaries() -> Result<(), Error> {
    let maximum = 10_i128.pow(38) - 1;
    for scale in [0, 38] {
        let field = FieldSpec::new(38, scale, Encoding::Ascii)?;
        for coefficient in [maximum, -maximum, 0, 1, -1] {
            let value = FixedDecimal::new(coefficient, scale)?;
            let mut output = [0; 38];
            field.encode(value, &mut output)?;
            assert_eq!(field.decode(&output)?, value);
        }
        for coefficient in [10_i128.pow(38), -10_i128.pow(38), i128::MAX, i128::MIN] {
            let mut output = [0xAA; 38];
            assert_eq!(
                field.encode(FixedDecimal::new(coefficient, scale)?, &mut output),
                Err(Error::Overflow)
            );
            assert_eq!(output, [0xAA; 38]);
        }
    }
    let field = FieldSpec::new(1, 1, Encoding::Ascii)?;
    assert_eq!(field.decode(b"J")?, FixedDecimal::new(-1, 1)?);
    assert_eq!(
        field.decode(b""),
        Err(Error::LengthMismatch {
            expected: 1,
            actual: 0
        })
    );
    // Oversized input is rejected before scanning any content.
    assert_eq!(
        field.decode(&[b'9'; 4096]),
        Err(Error::LengthMismatch {
            expected: 1,
            actual: 4096
        })
    );
    Ok(())
}

#[test]
fn errors_are_small_and_do_not_expose_record_contents() {
    assert_eq!(
        Error::InvalidDigit {
            index: 2,
            byte: 255
        }
        .to_string(),
        "invalid digit 0xFF at byte 2"
    );
    assert_eq!(
        Error::PrecisionLoss.to_string(),
        "exact rescaling would lose precision"
    );
    assert!(core::mem::size_of::<Error>() <= 32);
    assert!(core::mem::size_of::<FieldSpec>() <= 8);
}

#[test]
fn native_and_wide_chunk_boundaries_preserve_values_and_error_offsets() -> Result<(), Error> {
    for encoding in [Encoding::Ascii, Encoding::Ebcdic] {
        let field = FieldSpec::new(38, 2, encoding)?;
        for magnitude in [
            10_i128.pow(19) - 1,
            10_i128.pow(19),
            i128::from(u64::MAX),
            i128::from(u64::MAX) + 1,
            10_i128.pow(20),
            10_i128.pow(38) - 1,
        ] {
            for coefficient in [magnitude, -magnitude] {
                let value = FixedDecimal::new(coefficient, 2)?;
                let mut output = [0; 38];
                field.encode(value, &mut output)?;
                assert_eq!(field.decode(&output)?, value);
            }
        }
    }
    let field = FieldSpec::new(38, 2, Encoding::Ascii)?;
    for index in [0, 18, 19, 20, 36] {
        let mut input = [b'0'; 38];
        input[37] = b'{';
        input[index] = b'J';
        assert_eq!(
            field.decode(&input),
            Err(Error::InvalidDigit { index, byte: b'J' })
        );
    }
    Ok(())
}
