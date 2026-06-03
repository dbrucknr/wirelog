use criterion::{Criterion, black_box, criterion_group, criterion_main};
use std::io;

// cargo bench
// open target/criterion/report/index.html

fn wirelog_benchmarks(c: &mut Criterion) {
    let logger = wirelog::Logger::new(io::sink()).level(wirelog::Level::Error);
    c.bench_function("wirelog/disabled_event", |b| {
        b.iter(|| logger.trace().msg(black_box("noop")))
    });

    let logger = wirelog::Logger::new(io::sink());
    c.bench_function("wirelog/single_field", |b| {
        b.iter(|| {
            logger
                .info()
                .str(black_box("key"), black_box("value"))
                .msg(black_box("hello"))
        })
    });

    let logger = wirelog::Logger::new(io::sink());
    c.bench_function("wirelog/ten_fields", |b| {
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

fn tracing_benchmarks(c: &mut Criterion) {
    // try_init returns Err if already set — safe to ignore on repeated runs.
    let _ = tracing_subscriber::fmt()
        .json()
        .with_writer(|| io::sink())
        .try_init();

    // tracing does not have a zero-cost disabled fast-path when a subscriber is present;
    // the callsite interest check is still paid. Benchmark the filtered path anyway.
    c.bench_function("tracing/disabled_event", |b| {
        b.iter(|| tracing::trace!(message = black_box("noop")))
    });

    c.bench_function("tracing/single_field", |b| {
        b.iter(|| tracing::info!(key = black_box("value"), message = black_box("hello")))
    });

    c.bench_function("tracing/ten_fields", |b| {
        b.iter(|| {
            tracing::info!(
                a = "1",
                b = "2",
                c = "3",
                d = 4i64,
                e = 5i64,
                f = 6i64,
                g = true,
                h = false,
                i = 9u64,
                j = 1.5f64,
                "ten"
            )
        })
    });
}

fn wirelog_dynamic_benchmarks(c: &mut Criterion) {
    let logger = wirelog::Logger::boxed(io::sink()).level(wirelog::Level::Error);
    c.bench_function("wirelog_dyn/disabled_event", |b| {
        b.iter(|| logger.trace().msg(black_box("noop")))
    });

    let logger = wirelog::Logger::boxed(io::sink());
    c.bench_function("wirelog_dyn/single_field", |b| {
        b.iter(|| {
            logger
                .info()
                .str(black_box("key"), black_box("value"))
                .msg(black_box("hello"))
        })
    });

    let logger = wirelog::Logger::boxed(io::sink());
    c.bench_function("wirelog_dyn/ten_fields", |b| {
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

criterion_group!(
    benches,
    wirelog_benchmarks,
    wirelog_dynamic_benchmarks,
    tracing_benchmarks
);
criterion_main!(benches);
