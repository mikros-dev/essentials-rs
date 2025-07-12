//! # logger
//!
//! This crate provides a lightweight, structured logger intended for use by internal
//! crates within a larger application. It leverages the [`tracing`] crate to emit logs
//! with consistent formatting and structured values.
//!
//! ## Features
//!
//! - Automatically prefixes log messages with the crate and component name.
//! - Supports structured logging with optional `serde_json::Value` payloads.
//! - Integrates seamlessly with tracing subscribers configured in the main application.
//! - Simple API for common log levels: `info`, `warn`, `error`, and `debug`.
//!
//! ## Usage
//!
//! To create a logger instance and emit logs:
//!
//! ```rust
//! use logger::Logger;
//! use serde_json::json;
//!
//! let logger = Logger::new("my_crate", "http_client");
//!
//! logger.info("Request sent", None);
//! logger.debug("Parsed response", Some(json!({ "status": 200, "ok": true })));
//! logger.error("Request failed", Some(json!({ "code": 500, "message": "Internal server error" })));
//! ```
//!
//! ## Integration with `tracing`
//!
//! Ensure your application has set up a `tracing` subscriber (e.g., `tracing_subscriber::fmt`)
//! so logs are properly captured and displayed:
//!
//! ```rust:ignore
//! tracing_subscriber::fmt::init();
//! ```
//!
//! ## When to Use
//!
//! This crate is ideal for internal libraries and services that want to adopt the
//! application's logging format without configuring tracing individually.
//!
//! [`tracing`]: https://docs.rs/tracing

mod logger;

pub use logger::Logger;
