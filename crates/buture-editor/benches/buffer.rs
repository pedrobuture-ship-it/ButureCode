//! Reproducible standard-library benchmark; reports observations, no asserted timing budget.
use buture_editor::Buffer;
use std::{hint::black_box, time::Instant};
fn main() {
    for size in [10_000, 100_000, 1_000_000] {
        let text = "let value = 42;\n".repeat(size);
        let began = Instant::now();
        let mut buffer = Buffer::from_text(&text);
        let load = began.elapsed();
        let began = Instant::now();
        for _ in 0..1000 {
            let middle = buffer.len_chars() / 2;
            buffer.insert(middle, "x").expect("valid insert");
            buffer.delete(middle..middle + 1).expect("valid remove");
            black_box(buffer.position(middle).expect("mapping"));
        }
        println!(
            "{size} lines: load {load:?}, 1000 insert/remove/map rounds {:?}",
            began.elapsed()
        );
    }
}
