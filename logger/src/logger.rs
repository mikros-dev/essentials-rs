/// A structured logger that emits messages using the application's
/// `tracing` middleware with crate-specific prefixes.
///
/// This logger is intended to be used by internal crates to emit
/// logs with consistent formatting, so they integrate seamlessly with
/// the host application's observability tools.
///
/// Each log message is automatically prefixed with the crate and component
/// name (e.g., `my_crate`) to help identify the source.
///
/// # Example
/// ```ignore
/// let logger = Logger::new("my_crate");
/// logger.info("request sent", None);
/// logger.error("request failed", Some(json!({ "code": 500 })));
/// ```
#[derive(Debug, Clone)]
pub struct Logger {
    prefix: String,
}

impl Logger {
    /// Creates a new logger instance with a given crate and component name.
    pub fn new(prefix: &str) -> Self {
        Self {
            prefix: prefix.to_owned(),
        }
    }

    fn log(
        &self,
        level: tracing::Level,
        message: impl Into<String>,
        values: Option<serde_json::Value>,
    ) {
        let msg = format!("[{}] {}", self.prefix, message.into());
        let call_fields = values.and_then(|v| match v {
            serde_json::Value::Object(_) => serde_json::to_string(&v).ok(),
            _ => None,
        });

        match (level, call_fields) {
            (tracing::Level::DEBUG, None) => tracing::debug!(message = msg),
            (tracing::Level::WARN, None) => tracing::warn!(message = msg),
            (tracing::Level::ERROR, None) => tracing::error!(message = msg),
            (_, None) => tracing::info!(message = msg),

            (tracing::Level::DEBUG, Some(values)) => tracing::debug!(%values, message = msg),
            (tracing::Level::WARN, Some(values)) => tracing::warn!(%values , message = msg),
            (tracing::Level::ERROR, Some(values)) => tracing::error!(%values, message = msg),
            (_, Some(values)) => tracing::info!(%values, message = msg),
        }
    }

    /// Logs an informational message.
    pub fn info(&self, message: impl Into<String>, values: Option<serde_json::Value>) {
        self.log(tracing::Level::INFO, message, values)
    }

    /// Logs an error message.
    pub fn error(&self, message: impl Into<String>, values: Option<serde_json::Value>) {
        self.log(tracing::Level::ERROR, message, values)
    }

    /// Logs a warning message.
    pub fn warn(&self, message: impl Into<String>, values: Option<serde_json::Value>) {
        self.log(tracing::Level::WARN, message, values)
    }

    /// Logs a debug message.
    pub fn debug(&self, message: impl Into<String>, values: Option<serde_json::Value>) {
        self.log(tracing::Level::DEBUG, message, values)
    }
}
