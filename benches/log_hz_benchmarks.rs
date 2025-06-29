use criterion::{Criterion, criterion_group, criterion_main};
use log_hz::*;
use std::{hint::black_box, sync::LazyLock};

// Original mutex-based log_hz macro for comparison
macro_rules! log_hz_mutex {
    ($level:expr, $rate:expr, $($arg:tt)+) => {
        {
            static TIC: std::sync::Mutex<Option<std::time::Instant>> = std::sync::Mutex::new(None);
            static INTERVAL: std::sync::LazyLock<std::time::Duration> = std::sync::LazyLock::new(|| std::time::Duration::from_secs_f32(1.0 / ($rate as f32)));
            let mut tick = TIC.lock().expect("log_hz mutex was poisoned, this should never happen");
            match *tick {
                None => {
                    *tick = Some(std::time::Instant::now());
                    log!($level, $($arg)+);
                },
                Some(ref mut tick) => {
                    let now = std::time::Instant::now();
                    let time_since_last = now.duration_since(*tick);
                    if time_since_last > *INTERVAL {
                        *tick = now;
                        log!($level, $($arg)+);
                    }
                }
            }
        }
    }
}
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

use crate::log_hz;

fn benchmark_mutex_vs_lockfree(c: &mut Criterion) {
    setup_logger();

    let mut group = c.benchmark_group("log_hz_mutex_vs_lockfree");

    // Benchmark mutex-based version
    group.bench_function("mutex_version", |b| {
        b.iter(|| {
            log_hz_mutex!(log::Level::Info, 1.0, "Benchmark message {}", black_box(42));
        });
    });

    // Benchmark lock-free version
    group.bench_function("lockfree_version", |b| {
        b.iter(|| {
            log_hz!(log::Level::Info, 1.0, "Benchmark message {}", black_box(42));
        });
    });

    group.finish();
}

criterion_group!(benches, benchmark_mutex_vs_lockfree);
criterion_main!(benches);
