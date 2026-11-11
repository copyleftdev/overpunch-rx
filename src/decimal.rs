use crate::{Error, MAX_DIGITS, POWERS_OF_TEN};

/// An exact value: `coefficient * 10^(-scale)`, with negative zero preserved.
///
/// Equality is **representation equality**: scale and the sign of zero matter.
/// This is a transport value, not a general-purpose decimal arithmetic type.
/// Construction accepts all `i128` coefficients; encoding additionally enforces
/// field capacity. A coefficient of `12345` at scale `2` represents `123.45`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[must_use]
pub struct FixedDecimal {
    coefficient: i128,
    scale: u8,
    negative_zero: bool,
}

impl FixedDecimal {
    /// Creates an exact value. Zero is positive; use [`Self::negative_zero`] otherwise.
    ///
    /// # Errors
    /// Returns [`Error::InvalidScale`] if `scale > 38`.
    pub const fn new(coefficient: i128, scale: u8) -> Result<Self, Error> {
        if scale > MAX_DIGITS {
            return Err(Error::InvalidScale {
                scale,
                maximum: MAX_DIGITS,
            });
        }
        Ok(Self {
            coefficient,
            scale,
            negative_zero: false,
        })
    }

    /// Creates negative zero for lossless interchange.
    ///
    /// # Errors
    /// Returns [`Error::InvalidScale`] if `scale > 38`.
    pub const fn negative_zero(scale: u8) -> Result<Self, Error> {
        match Self::new(0, scale) {
            Ok(mut value) => {
                value.negative_zero = true;
                Ok(value)
            }
            Err(error) => Err(error),
        }
    }

    /// Returns the signed integer coefficient. Both zero representations return 0.
    #[must_use]
    pub const fn coefficient(self) -> i128 {
        self.coefficient
    }

    /// Returns the number of fractional decimal digits.
    #[must_use]
    pub const fn scale(self) -> u8 {
        self.scale
    }

    /// Returns whether this value has a negative sign, including negative zero.
    #[must_use]
    pub const fn is_negative(self) -> bool {
        self.coefficient < 0 || self.negative_zero
    }

    /// Changes scale exactly. No implicit rounding or truncation is performed.
    ///
    /// # Errors
    /// Returns [`Error::InvalidScale`] above 38, [`Error::Overflow`] if scaling up
    /// exceeds `i128`, or [`Error::PrecisionLoss`] if scaling down discards digits.
    pub fn rescale(self, scale: u8) -> Result<Self, Error> {
        if scale > MAX_DIGITS {
            return Err(Error::InvalidScale {
                scale,
                maximum: MAX_DIGITS,
            });
        }
        let coefficient = if scale >= self.scale {
            let factor = POWERS_OF_TEN[usize::from(scale - self.scale)];
            self.coefficient
                .checked_mul(factor)
                .ok_or(Error::Overflow)?
        } else {
            let factor = POWERS_OF_TEN[usize::from(self.scale - scale)];
            if self.coefficient % factor != 0 {
                return Err(Error::PrecisionLoss);
            }
            self.coefficient / factor
        };
        Ok(Self {
            coefficient,
            scale,
            negative_zero: self.negative_zero,
        })
    }
}
