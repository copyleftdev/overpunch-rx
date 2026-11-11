use crate::{Error, FixedDecimal, MAX_DIGITS, POWERS_OF_TEN};

/// Wire alphabet. Neither variant performs text transcoding.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Encoding {
    /// Digits `0..9`, positive `{ABCDEFGHI`, negative `}JKLMNOPQR`.
    Ascii,
    /// Zoned decimal: digit bytes `F0..F9`, positive `C0..C9`, negative `D0..D9`.
    /// Alternate sign nibbles A, B and E are intentionally rejected.
    Ebcdic,
}

/// Controls whether the terminal position must contain an explicit sign.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SignPolicy {
    /// Require a positive or negative overpunch code (the default).
    RequireOverpunch,
    /// Also accept an ordinary terminal digit as positive. Encoding stays signed.
    AllowUnsignedPositive,
}

/// Controls negative zero on both decoding and encoding.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ZeroPolicy {
    /// Preserve the sign of zero (the default).
    Preserve,
    /// Convert negative zero to positive zero.
    NormalizePositive,
    /// Reject negative zero with [`Error::NegativeZero`].
    RejectNegative,
}

/// A validated, reusable fixed-width numeric field definition.
///
/// Width includes the terminal sign/digit byte. Scale cannot exceed width.
/// Private fields prevent callers from bypassing the constructor's bounds.
/// All decode/encode loops visit at most 38 bytes (P10-2).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[must_use]
pub struct FieldSpec {
    width: u8,
    scale: u8,
    encoding: Encoding,
    sign_policy: SignPolicy,
    zero_policy: ZeroPolicy,
}

impl FieldSpec {
    /// Validates a schema with strict signs and preserved negative zero.
    ///
    /// These defaults are part of the API contract. Use the policy setters when
    /// the interchange specification calls for unsigned fields or normalized zero.
    ///
    /// # Errors
    /// Returns [`Error::InvalidWidth`] outside `1..=38` and
    /// [`Error::InvalidScale`] when `scale > width`.
    pub const fn new(width: u8, scale: u8, encoding: Encoding) -> Result<Self, Error> {
        if width == 0 || width > MAX_DIGITS {
            return Err(Error::InvalidWidth { width });
        }
        if scale > width {
            return Err(Error::InvalidScale {
                scale,
                maximum: width,
            });
        }
        Ok(Self {
            width,
            scale,
            encoding,
            sign_policy: SignPolicy::RequireOverpunch,
            zero_policy: ZeroPolicy::Preserve,
        })
    }

    /// Selects terminal sign acceptance without changing the encoding alphabet.
    pub const fn with_sign_policy(mut self, policy: SignPolicy) -> Self {
        self.sign_policy = policy;
        self
    }

    /// Selects negative-zero handling on input and output.
    pub const fn with_zero_policy(mut self, policy: ZeroPolicy) -> Self {
        self.zero_policy = policy;
        self
    }

    /// Returns the exact input/output width in bytes.
    #[must_use]
    pub const fn width(self) -> u8 {
        self.width
    }

    /// Returns the implied number of fractional digits.
    #[must_use]
    pub const fn scale(self) -> u8 {
        self.scale
    }

    /// Decodes exactly one field, rejecting whitespace, decimal points, misplaced
    /// signs, mixed encodings and malformed bytes. Does not trim or allocate.
    ///
    /// # Errors
    /// Returns a length, digit, sign, or negative-zero error. Invalid digit errors
    /// include a byte offset relative to this field. Prefix validation precedes
    /// terminal validation, so the earliest malformed byte is reported.
    pub fn decode(&self, input: &[u8]) -> Result<FixedDecimal, Error> {
        self.check_length(input.len())?;
        let last = usize::from(self.width) - 1;
        let mut coefficient = decode_prefix(&input[..last], self.encoding)?;
        let (digit, negative) =
            decode_terminal(input[last], last, self.encoding, self.sign_policy)?;
        // At most 38 digits: 10^38 - 1 < i128::MAX. Neither operation can overflow.
        coefficient = coefficient * 10 + i128::from(digit);
        let negative = self.zero_sign(coefficient == 0, negative)?;
        if negative && coefficient == 0 {
            FixedDecimal::negative_zero(self.scale)
        } else {
            FixedDecimal::new(
                if negative { -coefficient } else { coefficient },
                self.scale,
            )
        }
    }

    /// Writes a fully zero-padded field into an exactly sized caller buffer.
    ///
    /// Rescaling is exact. The entire output is unchanged on **every** error.
    /// A bounded 38-byte scratch buffer stages output before the final copy.
    ///
    /// # Errors
    /// Returns [`Error::LengthMismatch`], [`Error::PrecisionLoss`],
    /// [`Error::Overflow`], or [`Error::NegativeZero`] without modifying output.
    pub fn encode(&self, value: FixedDecimal, output: &mut [u8]) -> Result<(), Error> {
        self.check_length(output.len())?;
        let scaled = value.rescale(self.scale)?;
        let coefficient = scaled.coefficient();
        let limit = POWERS_OF_TEN[usize::from(self.width)];
        if coefficient <= -limit || coefficient >= limit {
            return Err(Error::Overflow);
        }
        let negative = self.zero_sign(coefficient == 0, scaled.is_negative())?;
        let mut scratch = [0; MAX_DIGITS as usize];
        let target = &mut scratch[..usize::from(self.width)];
        encode_digits(coefficient.unsigned_abs(), negative, self.encoding, target)?;
        output.copy_from_slice(target);
        Ok(())
    }

    fn check_length(self, actual: usize) -> Result<(), Error> {
        let expected = usize::from(self.width);
        if actual != expected {
            return Err(Error::LengthMismatch { expected, actual });
        }
        Ok(())
    }

    fn zero_sign(self, is_zero: bool, negative: bool) -> Result<bool, Error> {
        if is_zero && negative {
            return match self.zero_policy {
                ZeroPolicy::Preserve => Ok(true),
                ZeroPolicy::NormalizePositive => Ok(false),
                ZeroPolicy::RejectNegative => Err(Error::NegativeZero),
            };
        }
        Ok(negative)
    }
}

// Keep byte loops independent of schema state. Only decode() calls this helper,
// after validating the exact width. Its maximum iteration count is 37 (P10-2).
fn decode_prefix(input: &[u8], encoding: Encoding) -> Result<i128, Error> {
    debug_assert!(input.len() < usize::from(MAX_DIGITS));
    let zero = match encoding {
        Encoding::Ascii => b'0',
        Encoding::Ebcdic => 0xF0,
    };
    if input.len() <= 19 {
        return decode_chunk(input, zero, 0).map(i128::from);
    }
    // Two chunks suffice for the 37-byte prefix. 19 digits fit in u64, keeping
    // the common path native-width and paying for wide arithmetic only once.
    let high = decode_chunk(&input[..19], zero, 0)?;
    let low = decode_chunk(&input[19..], zero, 19)?;
    Ok(i128::from(high) * POWERS_OF_TEN[input.len() - 19] + i128::from(low))
}

fn decode_chunk(input: &[u8], zero: u8, offset: usize) -> Result<u64, Error> {
    debug_assert!(input.len() <= 19);
    debug_assert!(zero == b'0' || zero == 0xF0);
    let mut coefficient = 0_u64;
    for (index, &byte) in input.iter().enumerate() {
        let digit = byte.wrapping_sub(zero);
        if digit > 9 {
            return Err(Error::InvalidDigit {
                index: offset + index,
                byte,
            });
        }
        coefficient = coefficient * 10 + u64::from(digit);
    }
    Ok(coefficient)
}

fn decode_terminal(
    byte: u8,
    index: usize,
    encoding: Encoding,
    policy: SignPolicy,
) -> Result<(u8, bool), Error> {
    let (signed, unsigned_zero) = match encoding {
        Encoding::Ascii => (ASCII_TERMINAL[usize::from(byte)], b'0'),
        Encoding::Ebcdic => (EBCDIC_TERMINAL[usize::from(byte)], 0xF0),
    };
    if signed != INVALID {
        return Ok((signed % 10, signed >= 10));
    }
    let digit = byte.wrapping_sub(unsigned_zero);
    if digit > 9 {
        return Err(Error::InvalidSign { index, byte });
    }
    match policy {
        SignPolicy::RequireOverpunch => Err(Error::UnsignedNotAllowed),
        SignPolicy::AllowUnsignedPositive => Ok((digit, false)),
    }
}

// P10-2/P10-3: At most 38 iterations, using only the caller's bounded stack slice.
fn encode_digits(
    magnitude: u128,
    negative: bool,
    encoding: Encoding,
    output: &mut [u8],
) -> Result<(), Error> {
    debug_assert_ne!(output.len(), 0);
    debug_assert!(output.len() <= usize::from(MAX_DIGITS));
    let zero = match encoding {
        Encoding::Ascii => b'0',
        Encoding::Ebcdic => 0xF0,
    };
    if let Ok(small) = u64::try_from(magnitude) {
        encode_chunk(small, zero, output)?;
    } else {
        // A value above u64::MAX needs >=20 digits. The parent has already
        // checked capacity. Each base-10^19 chunk fits in u64.
        let base = 10_000_000_000_000_000_000_u128;
        let high = u64::try_from(magnitude / base).map_err(|_| Error::Overflow)?;
        let low = u64::try_from(magnitude % base).map_err(|_| Error::Overflow)?;
        let split = output.len() - 19;
        encode_chunk(high, zero, &mut output[..split])?;
        encode_chunk(low, zero, &mut output[split..])?;
    }
    let last = output.len() - 1;
    let digit = output[last] - zero;
    output[last] = match (encoding, negative) {
        (Encoding::Ascii, false) => ASCII_POSITIVE[usize::from(digit)],
        (Encoding::Ascii, true) => ASCII_NEGATIVE[usize::from(digit)],
        (Encoding::Ebcdic, false) => 0xC0 + digit,
        (Encoding::Ebcdic, true) => 0xD0 + digit,
    };
    Ok(())
}

fn encode_chunk(mut magnitude: u64, zero: u8, output: &mut [u8]) -> Result<(), Error> {
    debug_assert!(output.len() <= usize::from(MAX_DIGITS));
    debug_assert!(zero == b'0' || zero == 0xF0);
    for slot in output.iter_mut().rev() {
        // The remainder is provably 0..=9; checked conversion keeps this explicit.
        let digit = u8::try_from(magnitude % 10).map_err(|_| Error::Overflow)?;
        magnitude /= 10;
        *slot = zero + digit;
    }
    debug_assert_eq!(magnitude, 0);
    Ok(())
}

const INVALID: u8 = u8::MAX;
const ASCII_POSITIVE: &[u8; 10] = b"{ABCDEFGHI";
const ASCII_NEGATIVE: &[u8; 10] = b"}JKLMNOPQR";

const fn terminal_table(encoding: Encoding) -> [u8; 256] {
    let mut table = [INVALID; 256];
    let mut digit = 0_u8;
    // Exactly ten iterations at compile time; 0..9 = positive, 10..19 = negative.
    while digit < 10 {
        let (positive, negative) = match encoding {
            Encoding::Ascii => (
                ASCII_POSITIVE[digit as usize],
                ASCII_NEGATIVE[digit as usize],
            ),
            Encoding::Ebcdic => (0xC0 + digit, 0xD0 + digit),
        };
        table[positive as usize] = digit;
        table[negative as usize] = digit + 10;
        digit += 1;
    }
    table
}

const ASCII_TERMINAL: [u8; 256] = terminal_table(Encoding::Ascii);
const EBCDIC_TERMINAL: [u8; 256] = terminal_table(Encoding::Ebcdic);
