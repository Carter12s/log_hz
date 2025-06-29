//! # Log Hz
//! A logging library that provides macros for logging at a throttled rate.
//!
//! This crate was inspired by experience with [ROS_LOG_THROTTLE](https://docs.ros.org/en/jade/api/rosconsole/html/console_8h.html) from ROS 1.
//!
//! In robotics applications we often have loops running at fixed (often very high) rates.
//! In these loops log messages can be very useful for debugging, but can also quickly flood the log with duplicate information.
//! Throttled logging macros that prevent excess log spam can be very useful.
//!
//! ```no_run,rust
//! use log_hz::*;
//!
//! fn get_io_pin() -> Result<u8, String> {
//!   Err("Your IO Device Isn't Connected!".to_string())
//! }
//!
//! fn main() {
//!   loop {
//!     let io_pin = get_io_pin();
//!     match io_pin {
//!       Ok(pin) => {
//!         debug_hz!(1.0, "IO Pin State: {}", pin);
//!       },
//!       Err(e) => {
//!         error_hz!(1.0, "Failed to get IO Pin: {}", e);
//!       }
//!     }
//!   }
//! }
//! ```
//!
//! This crate provides the following throttled logging macros, matching their equivalents from the `log` crate:
//! [error_hz!], [warn_hz!], [info_hz!], [debug_hz!], and [trace_hz!].
//!
//! The rate is specified in Hz, and can be any expression that can be compile time cast to a `f32` with `as f32`.
//!
//! ```rust
//! use log_hz::*;
//!
//! fn main() {
//!   // Rate can't be dynamic:
//!   let rate = 1.0;
//!   // info_hz!(rate, "Hello, world!"); // This won't compile
//!
//!   // Instead, use a literal or something that can be compile time cast to a f32
//!   info_hz!(1.0, "Hello, world!");
//!   info_hz!(1.0f32, "Hello, world!");
//!   info_hz!(1, "Hello, world!");
//!
//!   const fn load_rate() -> f32 { 1.0 }
//!   info_hz!(load_rate(), "Hello, world!");
//!
//!   static RATE: f32 = 1.0;
//!   info_hz!(RATE, "Hello, world!");
//! }
//! ```
//!
//! It also re-exports all of the `log` crate's macros and functions, so you can use them as you normally would without needing to import it separately:
//!
//! ```rust
//! // Cargo.toml only contains `log_hz = "0.1.0"`, no need to import `log` separately
//! use log_hz::*;
//!
//! fn main() {
//!   // This comes from the log crate
//!   error!("This is an error message");
//!   // This comes from the log_hz crate
//!   error_hz!(1.0, "This is an error message that will only log once per second");
//! }
//! ```
//!
//! This crate is compatible with all the amazing loggers that the `log` crate is compatible with. An abbreviated list include:
//! - [env_logger](https://crates.io/crates/env_logger) - Extremely common logger for getting started with Enviroment variable configuration using `RUST_LOG`.
//! - [simple_logger](https://crates.io/crates/simple_logger) - Good for simple logging needs, manually configured in code.
//! - [flexi_logger](https://crates.io/crates/flexi_logger) - Good for combining multiple loggers, and logging to multiple destinations.
//! - [log4rs](https://crates.io/crates/log4rs) - Rust equivalent of the popular Java logging framework, or log4cxx the C++ equivalent.
//! - [slog](https://crates.io/crates/slog) - Big logging ecosystem with a ton of power.
//! - [testing_logger](https://crates.io/crates/testing_logger) - If you want to unit test your log messages.
//!
//! You will need to initialize a logger before log messages from this crate will be visible.
//! See the documentation for the logger you are using for more information.
//!
//! Note: This crate uses `std::time::Instant` to track time, which is not available in `no_std` environments.
//! If you're interested in alternative timing backends for this crate, feel free to open an issue or PR to add them behind features.

pub use log::*;

/// Log a message at [Level::Error] at a throttled rate, first call will always log.
#[macro_export]
macro_rules! error_hz {
    ($rate:expr,$($arg:tt)+) => { $crate::log_hz!($crate::Level::Error, $rate, $($arg)+); }
}

/// Log a message at [Level::Warn] at a throttled rate, first call will always log.
#[macro_export]
macro_rules! warn_hz {
    ($rate:expr,$($arg:tt)+) => { $crate::log_hz!($crate::Level::Warn, $rate, $($arg)+); }
}

/// Log a message at [Level::Info] at a throttled rate, first call will always log.
#[macro_export]
macro_rules! info_hz {
    ($rate:expr,$($arg:tt)+) => { $crate::log_hz!($crate::Level::Info, $rate, $($arg)+); }
}

/// Log a message at [Level::Debug] at a throttled rate, first call will always log.
#[macro_export]
macro_rules! debug_hz {
    ($rate:expr,$($arg:tt)+) => { $crate::log_hz!($crate::Level::Debug, $rate, $($arg)+); }
}

/// Log a message at [Level::Trace] at a throttled rate, first call will always log.
#[macro_export]
macro_rules! trace_hz {
    ($rate:expr,$($arg:tt)+) => { $crate::log_hz!($crate::Level::Trace, $rate, $($arg)+); }
}

/// Log a message at the specified level at a throttled rate, first call will always log.
///
/// This version uses an AtomicU64 and a compare-and-swap loop to manage the throttling in a lock-free manner.
/// It provides better performance than the mutex-based version, especially under high contention.
///
/// An optional `coarsetime` feature can be enabled to use a faster, but less precise, time source
/// on platforms that support it (currently Linux with `CLOCK_MONOTONIC_COARSE`).
#[macro_export]
macro_rules! log_hz {
    ($level:expr, $rate:expr, $($arg:tt)+) => {
        // Inner scope to encapsulate static variables
        {
            use std::sync::atomic::{AtomicU64, Ordering};
            use std::sync::LazyLock;

            #[cfg(not(feature = "coarsetime"))]
            use std::time::Instant;
            #[cfg(feature = "coarsetime")]
            use coarsetime::Instant;

            // A shared, static start time for the process.
            // Using LazyLock ensures it's initialized only once.
            static START_TIME: LazyLock<Instant> = LazyLock::new(Instant::now);

            // The interval between log messages in nanoseconds.
            // Calculated once and cached. A rate of 0 or less disables logging.
            static INTERVAL_NS: LazyLock<u64> = LazyLock::new(|| {
                let rate_f64 = $rate as f64;
                if rate_f64 > 0.0 {
                    (1.0 / rate_f64 * 1_000_000_000.0) as u64
                } else {
                    u64::MAX
                }
            });

            // The timestamp of the last log, stored as nanoseconds since START_TIME.
            // Initialized to 0, which ensures the first log message always gets through.
            static LAST_LOG_NS: AtomicU64 = AtomicU64::new(0);

            // --- Fast Path ---
            // This is the most common path, executed on every call to the macro.
            // It's designed to be as cheap as possible.

            // First, perform a quick, optimistic check to see if we should log.
            // We use `Relaxed` ordering because it's the cheapest, and we're not
            // yet synchronizing memory. We just want to bail out early if possible.
            let last_ns = LAST_LOG_NS.load(Ordering::Relaxed);
            let now = Instant::now();
            let elapsed_ns = now.duration_since(*START_TIME).as_nanos() as u64;

            // Check if enough time has passed since the last log.
            // `saturating_sub` prevents a panic in the rare case of time moving backward.
            if elapsed_ns.saturating_sub(last_ns) >= *INTERVAL_NS {
                // --- Slow Path ---
                // We might get to log. Now we need to ensure only one thread does.
                // We use a `compare_exchange` to atomically update the timestamp.
                // This operation attempts to replace `last_ns` with `elapsed_ns` only if
                // the current value is still `last_ns`.
                //
                // Ordering::AcqRel (Acquire-Release):
                //   - If successful, this creates a memory barrier that ensures:
                //     1. (Acquire) Any writes from other threads that happened before are visible now.
                //     2. (Release) The log message we are about to write will be visible to
                //        other threads that later access this atomic variable.
                // Ordering::Relaxed (on failure):
                //   - If we fail, it means another thread won the race. We don't need to
                //     synchronize memory, so we use the cheapest ordering.
                if LAST_LOG_NS.compare_exchange(last_ns, elapsed_ns, Ordering::AcqRel, Ordering::Relaxed).is_ok() {
                    // We successfully updated the timestamp, so we have the "right" to log.
                    $crate::log!($level, $($arg)+);
                }
                // If the `compare_exchange` failed, another thread logged in the tiny
                // window between our `load` and `compare_exchange`. We simply do nothing,
                // which correctly throttles the message.
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_hz() {
        log_hz!(Level::Info, 1.0, "Hello, world!");
    }

    #[test]
    fn test_log_variants() {
        error_hz!(1.0, "Hello, world!");
        warn_hz!(1.0, "Hello, world!");
        info_hz!(1.0, "Hello, world!");
        debug_hz!(1.0, "Hello, world!");
        trace_hz!(1.0, "Hello, world!");
    }

    #[test]
    fn rate_filtering_works() {
        testing_logger::setup();
        for _ in 0..10 {
            info_hz!(1.0, "Hello, world!");
        }
        testing_logger::validate(|captured_logs| {
            assert_eq!(captured_logs.len(), 1);
        });
    }

    #[test]
    fn rate_filtering_works_2() {
        testing_logger::setup();
        for i in 0..10 {
            // Sleep for 1 second on the last iteration
            if i == 9 {
                std::thread::sleep(std::time::Duration::from_millis(200));
            }
            info_hz!(10.0, "Hello, world!");
        }
        testing_logger::validate(|captured_logs| {
            // Should log once for first time, and once for last iteration since we slept
            assert_eq!(captured_logs.len(), 2);
        });
    }

    #[test]
    fn integer_literals_acceptable_for_rate() {
        info_hz!(1, "Hello, world!");
    }
}
