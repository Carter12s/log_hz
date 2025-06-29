use criterion::{Criterion, criterion_group, criterion_main};
use log_hz::*;
use std::{hint::black_box, sync::LazyLock};

// Mock logger for benchmarking
struct MockLogger;

impl log::Log for MockLogger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, _record: &log::Record) {
        // Do nothing - we're just measuring the macro overhead
    }

    fn flush(&self) {}
}

// Set up logger once at the beginning
static LOGGER: LazyLock<MockLogger> = LazyLock::new(|| {
    let logger = MockLogger;
    // Try to set the logger, but don't panic if it's already set
    let _ = log::set_logger(Box::leak(Box::new(logger)));
    log::set_max_level(log::LevelFilter::Trace);
    MockLogger
});

fn setup_logger() {
    // Just access the static logger to ensure it's initialized
    let _ = &*LOGGER;
}

fn benchmark_lockfree_coarsetime(c: &mut Criterion) {
    setup_logger();

    let mut group = c.benchmark_group("log_hz_mutex_vs_lockfree");

    // Benchmark coarsetime version
    group.bench_function("lockfree_version_coarsetime", |b| {
        b.iter(|| {
            log_hz!(log::Level::Info, 1.0, "Benchmark message {}", black_box(42));
        });
    });
    group.finish();
}

criterion_group!(benches, benchmark_lockfree_coarsetime);
criterion_main!(benches);
