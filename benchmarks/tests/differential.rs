//! Compare only the valid common domain; malformed inputs have stricter contracts.
use overpunch_rx::{Encoding, FieldSpec, FixedDecimal};

#[test]
fn shared_valid_ascii_domain_agrees_with_published_crate() {
    let mut state = 0xD1B5_4A32_D192_ED03_u64;
    for index in 0..50_000_u64 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let magnitude = i64::try_from(state % 1_000_000_000_000_000_000).unwrap();
        let coefficient = if index & 1 == 0 {
            magnitude
        } else {
            -magnitude
        };
        let scale = u8::try_from(index % 18).unwrap();
        let field = FieldSpec::new(18, scale, Encoding::Ascii).unwrap();
        let value = FixedDecimal::new(i128::from(coefficient), scale).unwrap();
        let old_value = rust_decimal::Decimal::new(coefficient, u32::from(scale));
        let mut output = [0; 18];
        field.encode(value, &mut output).unwrap();
        let text = core::str::from_utf8(&output).unwrap();
        assert_eq!(
            overpunch::extract(text, usize::from(scale)).unwrap(),
            old_value
        );
        let old_output = overpunch::format(old_value, usize::from(scale)).unwrap();
        let padded = format!("{old_output:0>18}");
        assert_eq!(text, padded);
        assert_eq!(field.decode(padded.as_bytes()).unwrap(), value);
    }
}
