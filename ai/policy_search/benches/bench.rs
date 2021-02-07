#![allow(unused_imports)]

use criterion::{black_box, criterion_group, criterion_main, Criterion};
// use mycrate::fibonacci;

// pub fn criterion_benchmark(c: &mut Criterion) {
//     c.bench_function("fib 20", |b| b.iter(|| fibonacci(black_box(20))));
// }

pub fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("run_profiling_test", |b| {
        b.iter(|| bak_card_game::run_profiling_test(5, 1_000))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
