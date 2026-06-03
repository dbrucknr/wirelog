use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::io;
use wirelog::{Logger, NonBlocking};

// cargo bench --bench non_blocking --features non-blocking
// open target/criterion/report/index.html

fn wirelog_nb_benchmarks(c: &mut Criterion) {
    // Large capacity so the channel never fills during the benchmark — we want to
    // measure the caller-side cost (encoding + channel send), not contention.
    // io::sink() means the background thread drains instantly, keeping the channel
    // near-empty regardless of throughput.
    let nb = NonBlocking::new(io::sink(), 1 << 16);
    let logger = Logger::new(nb);

    c.bench_function("wirelog_nb/single_field", |b| {
        b.iter(|| {
            logger
                .info()
                .str(black_box("key"), black_box("value"))
                .msg(black_box("hello"))
        })
    });

    let nb = NonBlocking::new(io::sink(), 1 << 16);
    let logger = Logger::new(nb);

    c.bench_function("wirelog_nb/ten_fields", |b| {
        b.iter(|| {
            logger
                .info()
                .str("a", "1")
                .str("b", "2")
                .str("c", "3")
                .int("d", 4)
                .int("e", 5)
                .int("f", 6)
                .bool("g", true)
                .bool("h", false)
                .uint("i", 9)
                .float("j", 1.5)
                .msg("ten")
        })
    });
}

criterion_group!(benches, wirelog_nb_benchmarks);
criterion_main!(benches);
