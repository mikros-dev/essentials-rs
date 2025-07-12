use std::collections::HashMap;
use std::fmt::Display;
use std::time::{Duration, Instant};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::error::Error;

/// Configuration options for the [`Client`].
#[derive(Clone)]
pub struct Options {
    /// The base URL for all requests.
    pub base_url: String,

    /// Optional content type header to use (e.g., `"application/json"`).
    pub content_type: Option<String>,

    /// Optional request timeout duration. Defaults to 30 seconds.
    pub timeout: Option<Duration>,
}

/// An HTTP client that wraps [`reqwest::Client`] and provides a simplified
/// interface.
pub struct Client {
    base_url: String,
    content_type: String,
    inner: reqwest::Client,
}

impl Client {
    /// Creates a new client from the given [`Options`].
    pub fn new(opts: Options) -> Result<Self, Error> {
        let inner = reqwest::Client::builder()
            .timeout(opts.timeout.unwrap_or(Duration::from_secs(30)))
            .build()?;

        Ok(Self {
            base_url: opts.base_url,
            content_type: opts.content_type.unwrap_or("application/json".to_string()),
            inner,
        })
    }

    fn build_url(&self, url: &str) -> String {
        if url.starts_with("http://") || url.starts_with("https://") {
            return url.to_string();
        }

        format!("{}{}", self.base_url, url)
    }

    /// Sends an HTTP request with an optional body and returns a [`Response`].
    pub async fn send_request<T: Serialize + ?Sized>(
        &self,
        request: Request,
        body: Option<&T>,
    ) -> Result<Response, Error> {
        let now = Instant::now();
        let mut builder = self
            .inner
            .request(request.method.into(), self.build_url(&request.url));

        let content_type = request
            .content_type
            .as_deref()
            .unwrap_or(&self.content_type);
        builder = builder.header("Content-Type", content_type);

        if let Some(headers) = request.headers {
            for (k, v) in &headers {
                builder = builder.header(k, v);
            }
        }

        if let Some(b) = body {
            let serialized = serde_json::to_vec(b)?;
            builder = builder.body(serialized);
        }

        let resp = builder.send().await?;
        let status_code = resp.status().as_u16();
        let headers = resp
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.as_bytes().to_vec()))
            .collect::<HashMap<_, _>>();

        let body = resp.bytes().await?.to_vec();
        let elapsed = now.elapsed().as_millis() as i64;

        Ok(Response {
            body,
            headers,
            status_code,
            time: elapsed,
        })
    }
}

/// Represents an HTTP request to be sent by the [`Client`].
#[derive(Debug)]
pub struct Request {
    /// The full or relative URL for the request.
    pub url: String,

    /// The HTTP method to use (GET, POST, etc.).
    pub method: Method,

    /// Optional override for the content type header.
    pub content_type: Option<String>,

    /// Optional HTTP headers to include in the request.
    pub headers: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone)]
pub enum Method {
    GET,
    PATCH,
    POST,
    PUT,
    DELETE,
}

impl Display for Method {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            Method::GET => "GET",
            Method::PATCH => "PATCH",
            Method::POST => "POST",
            Method::PUT => "PUT",
            Method::DELETE => "DELETE",
        }
        .to_string();

        write!(f, "{str}")
    }
}

impl TryFrom<&str> for Method {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.to_uppercase().as_str() {
            "DELETE" => Ok(Method::DELETE),
            "GET" => Ok(Method::GET),
            "PATCH" => Ok(Method::PATCH),
            "POST" => Ok(Method::POST),
            "PUT" => Ok(Method::PUT),
            _ => Err(Error::InvalidMethod(value.to_string())),
        }
    }
}

impl From<Method> for http::Method {
    fn from(m: Method) -> Self {
        match m {
            Method::GET => http::Method::GET,
            Method::PATCH => http::Method::PATCH,
            Method::POST => http::Method::POST,
            Method::PUT => http::Method::PUT,
            Method::DELETE => http::Method::DELETE,
        }
    }
}

/// Represents an HTTP response returned by the [`Client`].
#[derive(Debug, Clone, Deserialize)]
pub struct Response {
    /// The raw response body.
    pub body: Vec<u8>,

    /// The response headers as raw bytes.
    pub headers: HashMap<String, Vec<u8>>,

    /// The HTTP status code.
    pub status_code: u16,

    /// The total elapsed request time in milliseconds.
    pub time: i64,
}

impl Response {
    pub fn has_body(&self) -> bool {
        !self.body.is_empty()
    }

    pub fn deserialize<T: DeserializeOwned>(&self) -> Result<T, Error> {
        Ok(serde_json::from_slice(&self.body)?)
    }

    pub fn get_utf8_header(&self, name: &str) -> Option<&str> {
        self.headers
            .get(name)
            .and_then(|v| std::str::from_utf8(v).ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_client() -> Client {
        Client::new(Options {
            base_url: "https://httpbin.org".to_string(),
            content_type: Some("application/json".to_string()),
            timeout: Some(Duration::from_secs(10)),
        })
        .expect("failed to create client")
    }

    #[tokio::test]
    async fn test_empty_post_request() {
        let client = test_client();
        let response = client
            .send_request::<()>(
                Request {
                    url: "/post".to_string(),
                    method: Method::POST,
                    content_type: None,
                    headers: None,
                },
                None,
            )
            .await;

        assert!(response.is_ok());
        let response = response.unwrap();
        assert_eq!(response.status_code, 200);
    }

    #[tokio::test]
    async fn test_post_request() {
        let client = test_client();
        let data = serde_json::json!({
            "version": 1,
            "name": "Alice",
        });

        let response = client
            .send_request(
                Request {
                    url: "/post".to_string(),
                    method: Method::POST,
                    content_type: None,
                    headers: None,
                },
                Some(&data),
            )
            .await;

        assert!(response.is_ok());
        let response = response.unwrap();
        assert_eq!(response.status_code, 200);
    }

    #[tokio::test]
    async fn test_get_request() {
        let client = test_client();
        let response = client
            .send_request::<()>(
                Request {
                    url: "/get".to_string(),
                    method: Method::GET,
                    content_type: None,
                    headers: None,
                },
                None,
            )
            .await;

        assert!(response.is_ok());
        let response = response.unwrap();
        assert_eq!(response.status_code, 200);
        assert!(response.has_body());
    }

    #[tokio::test]
    async fn test_delete_request() {
        let client = test_client();
        let response = client
            .send_request::<()>(
                Request {
                    url: "/delete".to_string(),
                    method: Method::DELETE,
                    content_type: None,
                    headers: None,
                },
                None,
            )
            .await;

        assert!(response.is_ok());
        let response = response.unwrap();
        assert_eq!(response.status_code, 200);
    }

    #[tokio::test]
    async fn test_put_request() {
        let client = test_client();
        let data = serde_json::json!({ "update": true });
        let response = client
            .send_request(
                Request {
                    url: "/put".to_string(),
                    method: Method::PUT,
                    content_type: None,
                    headers: None,
                },
                Some(&data),
            )
            .await;

        assert!(response.is_ok());
        let response = response.unwrap();
        assert_eq!(response.status_code, 200);
    }

    #[tokio::test]
    async fn test_patch_request() {
        let client = test_client();
        let data = serde_json::json!({ "patched": true });
        let response = client
            .send_request(
                Request {
                    url: "/patch".to_string(),
                    method: Method::PATCH,
                    content_type: None,
                    headers: None,
                },
                Some(&data),
            )
            .await;

        assert!(response.is_ok());
        let response = response.unwrap();
        assert_eq!(response.status_code, 200);
    }
}
