#![no_main]

use libfuzzer_sys::fuzz_target;
use overpunch_rx::{Encoding, FieldSpec, FixedDecimal, SignPolicy, ZeroPolicy};

fuzz_target!(|data: &[u8]| {
    if data.len() < 24 {
        return;
    }
    let encoding = if data[0] & 1 == 0 {
        Encoding::Ascii
    } else {
        Encoding::Ebcdic
    };
    let sign = if data[1] & 1 == 0 {
        SignPolicy::RequireOverpunch
    } else {
        SignPolicy::AllowUnsignedPositive
    };
    let zero = match data[2] % 3 {
        0 => ZeroPolicy::Preserve,
        1 => ZeroPolicy::NormalizePositive,
        _ => ZeroPolicy::RejectNegative,
    };
    // Also exercise invalid schema parameters, not only valid formats.
    let width = data[3] % 40;
    let scale = data[4] % 40;
    let Ok(field) = FieldSpec::new(width, scale, encoding) else {
        return;
    };
    let field = field.with_sign_policy(sign).with_zero_policy(zero);
    if let Ok(value) = field.decode(&data[24..]) {
        let mut output = [0; 38];
        let target = &mut output[..usize::from(width)];
        field.encode(value, target).unwrap();
        assert_eq!(field.decode(target).unwrap(), value);
    }
    let mut coefficient_bytes = [0; 16];
    coefficient_bytes.copy_from_slice(&data[8..24]);
    let coefficient = i128::from_le_bytes(coefficient_bytes);
    let value_scale = data[5] % 40;
    let value = if data[6] & 1 == 0 {
        FixedDecimal::new(coefficient, value_scale)
    } else {
        FixedDecimal::negative_zero(value_scale)
    };
    let Ok(value) = value else { return };
    // Wrong output lengths must also preserve every byte.
    let length = usize::from(data[7] % 40);
    let mut output = [0xAA; 40];
    if field.encode(value, &mut output[..length]).is_err() {
        assert_eq!(output, [0xAA; 40]);
    } else {
        let decoded = field.decode(&output[..length]).unwrap();
        let scaled = value.rescale(scale).unwrap();
        if scaled.coefficient() == 0 && zero == ZeroPolicy::NormalizePositive {
            assert_eq!(decoded, FixedDecimal::new(0, scale).unwrap());
        } else {
            assert_eq!(decoded, scaled);
        }
        assert_eq!(&output[length..], &[0xAA; 40][length..]);
    }
});
