//! Strict codecs for fixed-width signed overpunch fields.
//!
//! Values are exact integer coefficients with a decimal scale. The core has no
//! dependencies, allocator, I/O, floating point, or unsafe code. Every operational
//! loop is bounded by [`MAX_DIGITS`]. Fields must fit in 38 decimal digits.
//!
//! ```
//! use overpunch_rx::{Encoding, FieldSpec, FixedDecimal};
//!
//! let field = FieldSpec::new(8, 2, Encoding::Ascii)?;
//! let amount = field.decode(b"0001234N")?;
//! assert_eq!(amount.coefficient(), -12345); // -123.45
//! assert_eq!(amount.scale(), 2);
//! let mut record = [0; 8];
//! field.encode(FixedDecimal::new(-12345, 2)?, &mut record)?;
//! assert_eq!(&record, b"0001234N");
//! # Ok::<(), overpunch_rx::Error>(())
//! ```
//!
//! This is a numeric field codec, not a complete COBOL PICTURE parser or an
//! NCPDP message validator. The caller must select the field width, scale,
//! encoding and sign/zero policies required by its actual interchange contract.

#![no_std]
#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod decimal;
mod error;
mod field;

pub use decimal::FixedDecimal;
pub use error::Error;
pub use field::{Encoding, FieldSpec, SignPolicy, ZeroPolicy};

/// Maximum field width and decimal scale, in digits.
///
/// `10^38 - 1` fits in `i128`, so validated fields cannot overflow the decoder.
pub const MAX_DIGITS: u8 = 38;

// P10-2: This compile-time loop has exactly 38 iterations. All entries fit i128.
const fn powers_of_ten() -> [i128; 39] {
    let mut powers = [1; 39];
    let mut index = 1;
    while index < powers.len() {
        powers[index] = powers[index - 1] * 10;
        index += 1;
    }
    powers
}

pub(crate) const POWERS_OF_TEN: [i128; 39] = powers_of_ten();

const _: () = assert!(POWERS_OF_TEN[MAX_DIGITS as usize] < i128::MAX);
