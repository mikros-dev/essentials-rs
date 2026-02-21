//! # third_party
//!
//! This crate provides a flexible abstraction for integrating with external
//! (third-party) HTTP APIs. It supports dynamically building requests, managing
//! dependencies between calls, and extracting values from responses to feed
//! into subsequent requests.
//!
//! ## Features
//! - Declarative request builders for calls and dependencies
//! - Fluent builder-style API for path, query, headers, and body
//! - Response field extractors using [JSONPath](https://goessner.net/articles/JsonPath/)
//! - Dependency resolution and value injection (e.g., bearer tokens)
//!
//! ## Example
//!
//! ```rust:ignore
//! use third_party::builder::*;
//! use third_party::{DependencyTarget, ThirdPartyBuilder};
//!
//! let call = CallBuilder::new("/get", "GET")
//!     .with_url("https://httpbin.org")
//!     .build()
//!     .unwrap();
//!
//! let tp = ThirdPartyBuilder::new("example", "https://httpbin.org")
//!     .with_call("get", call)
//!     .build()
//!     .unwrap();
//!
//! let request = RequestBuilder::new()
//!     .query_arg("foo", "bar")
//!     .build();
//!
//! // You would call `tp.call("get", request).await` here
//! ```

mod builder;
mod error;
mod third_party;

#[cfg(test)]
mod tests;

pub use builder::{CallBuilder, DependencyRequestBuilder, RequestBuilder, ThirdPartyBuilder};
pub use error::Error;
pub use third_party::{Call, DependencyRequest, DependencyTarget, Request, Response, ThirdParty};
pub use http_client::{Form, Part};
