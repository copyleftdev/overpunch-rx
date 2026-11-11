# Codec design and bounds

## Boundary model

`FieldSpec::new` validates `1 <= width <= 38` and `scale <= width`. Its fields
are private, and policy changes do not alter width or scale. `FixedDecimal`
similarly keeps its scale in `0..=38` and only permits the negative-zero flag
when the coefficient is zero. There are no public unchecked constructors.

The core performs no I/O and holds no global mutable state. It can be reused
across threads without synchronization. Data-dependent validation is not
constant-time and the library is not intended for secret-number processing.

## Decode

1. Check exact byte length before visiting input.
2. Validate ordinary digit bytes throughout the prefix; report the earliest error.
3. Decode the last byte through a compile-time 256-entry alphabet table.
4. Apply explicit unsigned and zero policies.
5. Return an exact signed coefficient and the schema scale.

Prefix chunks contain at most 19 digits. Their largest value is `10^19 - 1`,
below `u64::MAX`. At most two chunks cover a 37-byte prefix. The only wide
combination remains below `10^37`; appending the final digit yields at most
`10^38 - 1`, below `i128::MAX`. Applying a negative sign is therefore safe.
No overflow-mode setting is needed for correctness of these bounded operations.

## Encode

1. Validate output length.
2. Rescale with checked multiplication or exact division (nonzero remainders fail).
3. Check `-10^width < coefficient < 10^width` before taking its magnitude.
4. Apply the zero policy.
5. Write digit bytes into a 38-byte local scratch array, then replace the terminal
   digit with the selected sign code.
6. Copy the fully prepared field to the output buffer exactly once.

Every fallible operation precedes the copy. On failure the caller's buffer is
untouched; on success every field byte is initialized, including leading zeros.
The copy costs at most 38 bytes and buys a simple transactional output contract.

Most magnitudes take a native `u64` digit-writing path. Larger values are split
once into high and low base-`10^19` chunks, each fitting in `u64`. Repeated
128-bit division is avoided. The high/low slices partition the output; total
digit-loop work remains at most 38 iterations. Narrowing conversions are checked.

## Resource accounting

| Operation | Maximum explicit loop work | Operational memory |
| --- | --- | --- |
| Decode | 37 prefix digits + constant-time terminal lookup | Scalar locals; no scratch array |
| Encode | 38 digit writes + 38-byte bounded copy | 38-byte scratch + scalar locals |
| Rescale | No explicit loop | Scalar locals; one checked multiply or divide/remainder |
| Build decimal power table | 38 iterations, compile time only | 39 `i128` values in read-only data |
| Build terminal tables | 10 iterations per alphabet, compile time only | Two 256-byte read-only tables |

Rust/library/compiler intrinsics used for copying and wide arithmetic have their
own implementations. The source bounds are not a measured worst-case execution
time or an exact compiler-generated stack-size proof. Division timing may differ
substantially on embedded targets. Cross-compilation verifies availability, not
embedded performance.

## Deliberate limitations

The library has one signed trailing-overpunch model. It does not parse arbitrary
PICTURE syntax or try to autodetect a code page. It rejects values needing more
than 38 digits even though some 39-digit integers fit `i128`. No rounding mode is
hidden in formatting; any rounding must happen explicitly before calling it.

Both debug assertions and tests check internal invariants. Untrusted input is
handled by `Result`, not assertions. No formal panic-freedom, WCET or safety
certification claim is made. The design supports manual reasoning about bounds,
which complements compiler, lint, interpreter and fuzz evidence.
