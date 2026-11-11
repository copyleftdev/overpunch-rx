use std::hint::black_box;
use std::time::Instant;

use overpunch_rx::{Encoding, FieldSpec, FixedDecimal};

const ITERATIONS: u32 = 250_000;
const SAMPLES: usize = 9;

fn measure(mut operation: impl FnMut()) -> (f64, f64, f64) {
    for _ in 0..10_000 {
        operation();
    }
    let mut samples = [0.0; SAMPLES];
    for sample in &mut samples {
        let start = Instant::now();
        for _ in 0..ITERATIONS {
            operation();
        }
        *sample = start.elapsed().as_secs_f64() * 1e9 / f64::from(ITERATIONS);
    }
    samples.sort_by(f64::total_cmp);
    (samples[0], samples[SAMPLES / 2], samples[SAMPLES - 1])
}

fn report(label: &str, timings: (f64, f64, f64)) {
    println!(
        "{label:32} {:9.2} {:9.2} {:9.2}",
        timings.0, timings.1, timings.2
    );
}

fn main() {
    println!(
        "nanoseconds/operation; min / median / max; {SAMPLES} samples x {ITERATIONS} iterations"
    );
    // Each baseline case has <=18 digits, safely within the old crate's i64 parser.
    // Input/metadata/output black boxes prevent constant folding and dead-code elimination.
    for (wire, coefficient) in [
        ("1234N", -12345_i64),
        ("1234567890A", 12345678901),
        ("12345678901234567Q", -123456789012345678),
    ] {
        let field = FieldSpec::new(u8::try_from(wire.len()).unwrap(), 2, Encoding::Ascii).unwrap();
        let value = FixedDecimal::new(i128::from(coefficient), 2).unwrap();
        let old_value = rust_decimal::Decimal::new(coefficient, 2);
        // Establish that the compared operations encode exactly the same value/bytes.
        assert_eq!(overpunch::extract(wire, 2).unwrap(), old_value);
        assert_eq!(field.decode(wire.as_bytes()).unwrap(), value);
        let mut output = [0; 38];
        field.encode(value, &mut output[..wire.len()]).unwrap();
        assert_eq!(&output[..wire.len()], wire.as_bytes());
        assert_eq!(
            overpunch::format(old_value, 2).unwrap().as_bytes(),
            wire.as_bytes()
        );
        println!("\nASCII width={}, scale=2", wire.len());
        report(
            "overpunch-rx decode",
            measure(|| {
                // Deliberately discard the consumed result after the optimizer barrier.
                let _ = black_box(black_box(field).decode(black_box(wire.as_bytes())).unwrap());
            }),
        );
        report(
            "overpunch 0.3 decode",
            measure(|| {
                black_box(overpunch::extract(black_box(wire), black_box(2)).unwrap());
            }),
        );
        report(
            "overpunch-rx encode (buffer)",
            measure(|| {
                black_box(field)
                    .encode(black_box(value), black_box(&mut output[..wire.len()]))
                    .unwrap();
                black_box(&output[..wire.len()]);
            }),
        );
        report(
            "overpunch 0.3 encode (String)",
            measure(|| {
                black_box(overpunch::format(black_box(old_value), black_box(2)).unwrap());
            }),
        );
    }
    let field = FieldSpec::new(38, 2, Encoding::Ascii).unwrap();
    let value = FixedDecimal::new(10_i128.pow(38) - 1, 2).unwrap();
    let mut wire = [0; 38];
    field.encode(value, &mut wire).unwrap();
    println!("\n38-digit boundary (outside old crate's i64 parser range)");
    report(
        "overpunch-rx decode",
        measure(|| {
            // Deliberately discard the consumed result after the optimizer barrier.
            let _ = black_box(black_box(field).decode(black_box(&wire)).unwrap());
        }),
    );
    report(
        "overpunch-rx encode",
        measure(|| {
            black_box(field)
                .encode(black_box(value), black_box(&mut wire))
                .unwrap();
            black_box(&wire);
        }),
    );
}
