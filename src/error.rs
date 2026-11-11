use core::fmt;

/// A recoverable schema, input, or output error. No variant allocates memory.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum Error {
    /// Width is outside `1..=38`.
    InvalidWidth {
        /// Rejected width in bytes.
        width: u8,
    },
    /// Scale exceeds the permitted maximum (field width, or 38 for a value).
    InvalidScale {
        /// Rejected number of fractional digits.
        scale: u8,
        /// Maximum number of fractional digits in this context.
        maximum: u8,
    },
    /// Input or output must have exactly the schema's width.
    LengthMismatch {
        /// Required number of bytes.
        expected: usize,
        /// Supplied number of bytes.
        actual: usize,
    },
    /// A nonterminal byte is not a digit in the selected encoding.
    InvalidDigit {
        /// Zero-based byte offset within the field.
        index: usize,
        /// Rejected byte, without any other record data.
        byte: u8,
    },
    /// The terminal byte is not an accepted sign/digit code.
    InvalidSign {
        /// Zero-based byte offset within the field.
        index: usize,
        /// Rejected byte.
        byte: u8,
    },
    /// An ordinary terminal digit was supplied under the strict sign policy.
    UnsignedNotAllowed,
    /// A negative zero was supplied under the rejecting zero policy.
    NegativeZero,
    /// Exact rescaling would discard nonzero fractional digits.
    PrecisionLoss,
    /// A rescaled value exceeds `i128`, or its magnitude exceeds the field width.
    Overflow,
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidWidth { width } => write!(formatter, "width {width} is outside 1..=38"),
            Self::InvalidScale { scale, maximum } => {
                write!(formatter, "scale {scale} exceeds maximum {maximum}")
            }
            Self::LengthMismatch { expected, actual } => {
                write!(formatter, "expected {expected} bytes, received {actual}")
            }
            Self::InvalidDigit { index, byte } => {
                write!(formatter, "invalid digit 0x{byte:02X} at byte {index}")
            }
            Self::InvalidSign { index, byte } => {
                write!(formatter, "invalid sign 0x{byte:02X} at byte {index}")
            }
            Self::UnsignedNotAllowed => formatter.write_str("terminal overpunch sign required"),
            Self::NegativeZero => formatter.write_str("negative zero is not permitted"),
            Self::PrecisionLoss => formatter.write_str("exact rescaling would lose precision"),
            Self::Overflow => formatter.write_str("value exceeds numeric or field capacity"),
        }
    }
}

impl core::error::Error for Error {}
