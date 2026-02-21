//! # `http_client`
//!
//! This crate provides a lightweight, extensible HTTP client wrapper based on
//! `reqwest` crate. It offers simple request and response handling, automatic
//! URL resolution, configurable timeouts, and serialization/deserialization
//! using `serde`.
//!
//! ## Example
//!
//! ```no_run
//! use http_client::{Body, Client, Options, Request, Method};
//! use serde_json::json;
//!
//! #[tokio::main(flavor = "current_thread")]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let client = Client::new(Options {
//!         base_url: "https://httpbin.org".into(),
//!         content_type: Some("application/json".into()),
//!         timeout: None,
//!     })?;
//!
//!     let body = json!({ "key": "value" });
//!     let req = Request {
//!         url: "/post".into(),
//!         method: Method::POST,
//!         content_type: None,
//!         headers: None,
//!         body: Some(Body::Json(body)),
//!     };
//!
//!     let resp = client.send_request(req).await?;
//!     println!("Status: {}", resp.status_code);
//!     Ok(())
//! }
//! ```

mod error;
mod client;
mod multipart;

pub use error::Error;
pub use client::{Body, Client, Method, Options, Request, Response};
pub use multipart::{Part, Form};
