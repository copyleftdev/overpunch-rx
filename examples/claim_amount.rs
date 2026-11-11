//! Translate one ASCII pharmacy amount into EBCDIC zoned decimal.
use overpunch_rx::{Encoding, FieldSpec, ZeroPolicy};

fn main() -> Result<(), overpunch_rx::Error> {
    let ascii =
        FieldSpec::new(8, 2, Encoding::Ascii)?.with_zero_policy(ZeroPolicy::NormalizePositive);
    let ebcdic =
        FieldSpec::new(8, 2, Encoding::Ebcdic)?.with_zero_policy(ZeroPolicy::NormalizePositive);
    let value = ascii.decode(b"0001234N")?;
    let mut output = [0; 8];
    ebcdic.encode(value, &mut output)?;
    println!(
        "coefficient={}, scale={}, EBCDIC={output:02X?}",
        value.coefficient(),
        value.scale()
    );
    Ok(())
}
