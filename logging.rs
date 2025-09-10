use std::time::Instant;
use uuid::Uuid;

/// Generate a correlation ID for request tracing
pub fn generate_correlation_id() -> String {
    Uuid::new_v4().to_string()
}

/// Extract correlation ID from request headers or generate new one
pub fn extract_or_generate_correlation_id(headers: Option<&str>) -> String {
    headers.and_then(|h| h.parse().ok()).unwrap_or_else(|| generate_correlation_id())
}

/// Timer for measuring operation duration
pub struct OperationTimer {
    start: Instant,
    operation: String,
    req_id: String,
}

impl OperationTimer {
    pub fn new(operation: &str, req_id: &str) -> Self {
        Self {
            start: Instant::now(),
            operation: operation.to_string(),
            req_id: req_id.to_string(),
        }
    }

    pub fn elapsed_ms(&self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }

    pub fn log_completion(&self, level: LogLevel, category: &str, message: &str) {
        let duration = self.elapsed_ms();
        match level {
            LogLevel::Debug => {
                tracing::debug!(
                    "{}:{} [{}:{}ms] [req_id:{}] {}",
                    self.get_layer(),
                    self.operation,
                    category,
                    duration,
                    self.req_id,
                    message
                );
            }
            LogLevel::Info => {
                tracing::info!(
                    "{}:{} [{}:{}ms] [req_id:{}] {}",
                    self.get_layer(),
                    self.operation,
                    category,
                    duration,
                    self.req_id,
                    message
                );
            }
            LogLevel::Warn => {
                tracing::warn!(
                    "{}:{} [{}:{}ms] [req_id:{}] {}",
                    self.get_layer(),
                    self.operation,
                    category,
                    duration,
                    self.req_id,
                    message
                );
            }
            LogLevel::Error => {
                tracing::error!(
                    "{}:{} [{}:{}ms] [req_id:{}] {}",
                    self.get_layer(),
                    self.operation,
                    category,
                    duration,
                    self.req_id,
                    message
                );
            }
        }
    }

    fn get_layer(&self) -> &str {
        if self.operation.starts_with("REST:") {
            "REST"
        } else if self.operation.starts_with("SERVICE:") {
            "SERVICE"
        } else if self.operation.starts_with("REPO:") {
            "REPO"
        } else if self.operation.starts_with("MODEL:") {
            "MODEL"
        } else {
            "UNKNOWN"
        }
    }
}

pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

/// Standard error codes
pub mod error_codes {
    // Validation Errors
    pub const VAL_INVALID_FORMAT: &str = "VAL001";
    pub const VAL_MISSING_FIELD: &str = "VAL002";
    pub const VAL_LENGTH_VIOLATION: &str = "VAL003";
    pub const VAL_INVALID_ENUM: &str = "VAL004";
    pub const VAL_BUSINESS_RULE: &str = "VAL005";

    // Business Logic Errors
    pub const BIZ_DUPLICATE: &str = "BIZ001";
    pub const BIZ_NOT_FOUND: &str = "BIZ002";
    pub const BIZ_NOT_ALLOWED: &str = "BIZ003";
    pub const BIZ_QUOTA_EXCEEDED: &str = "BIZ004";
    pub const BIZ_INVALID_STATE: &str = "BIZ005";

    // Database Errors
    pub const DB_CONNECTION_TIMEOUT: &str = "DB001";
    pub const DB_QUERY_FAILED: &str = "DB002";
    pub const DB_TRANSACTION_FAILED: &str = "DB003";
    pub const DB_INDEX_VIOLATION: &str = "DB004";
    pub const DB_SHARD_ERROR: &str = "DB005";
    pub const INT_REPOSITORY_ERROR: &str = "DB006";

    // Security Errors
    pub const SEC_AUTH_FAILED: &str = "SEC001";
    pub const SEC_ACCESS_DENIED: &str = "SEC002";
    pub const SEC_TOKEN_INVALID: &str = "SEC003";
    pub const SEC_RATE_LIMITED: &str = "SEC004";
    pub const SEC_SUSPICIOUS: &str = "SEC005";

    // Integration Errors
    pub const INT_TIMEOUT: &str = "INT001";
    pub const INT_SERVICE_ERROR: &str = "INT002";
    pub const INT_FORMAT_MISMATCH: &str = "INT003";
    pub const INT_UNAVAILABLE: &str = "INT004";
}

/// Macro for standardized logging with all enhancements
#[macro_export]
macro_rules! log_enhanced {
    (
        debug,
        $layer:expr,
        $operation:expr,
        $category:expr,
        $req_id:expr,
        $($arg:tt)*
    ) => {
        tracing::debug!("{}:{} [{}] [req_id:{}] {}", $layer, $operation, $category, $req_id, format!($($arg)*))
    };
    (
        info,
        $layer:expr,
        $operation:expr,
        $category:expr,
        $req_id:expr,
        $($arg:tt)*
    ) => {
        tracing::info!("{}:{} [{}] [req_id:{}] {}", $layer, $operation, $category, $req_id, format!($($arg)*))
    };
    (
        warn,
        $layer:expr,
        $operation:expr,
        $category:expr,
        $req_id:expr,
        $($arg:tt)*
    ) => {
        tracing::warn!("{}:{} [{}] [req_id:{}] {}", $layer, $operation, $category, $req_id, format!($($arg)*))
    };
    (
        error,
        $layer:expr,
        $operation:expr,
        $category:expr,
        $req_id:expr,
        $($arg:tt)*
    ) => {
        tracing::error!("{}:{} [{}] [req_id:{}] {}", $layer, $operation, $category, $req_id, format!($($arg)*))
    };

    // With timing
    (
        debug,
        $layer:expr,
        $operation:expr,
        $category:expr,
        $req_id:expr,
        $duration_ms:expr,
        $($arg:tt)*
    ) => {
        tracing::debug!("{}:{} [{}:{}ms] [req_id:{}] {}", $layer, $operation, $category, $duration_ms, $req_id, format!($($arg)*))
    };
    (
        info,
        $layer:expr,
        $operation:expr,
        $category:expr,
        $req_id:expr,
        $duration_ms:expr,
        $($arg:tt)*
    ) => {
        tracing::info!("{}:{} [{}:{}ms] [req_id:{}] {}", $layer, $operation, $category, $duration_ms, $req_id, format!($($arg)*))
    };
    (
        warn,
        $layer:expr,
        $operation:expr,
        $category:expr,
        $req_id:expr,
        $duration_ms:expr,
        $($arg:tt)*
    ) => {
        tracing::warn!("{}:{} [{}:{}ms] [req_id:{}] {}", $layer, $operation, $category, $duration_ms, $req_id, format!($($arg)*))
    };
    (
        error,
        $layer:expr,
        $operation:expr,
        $category:expr,
        $req_id:expr,
        $duration_ms:expr,
        $($arg:tt)*
    ) => {
        tracing::error!("{}:{} [{}:{}ms] [req_id:{}] {}", $layer, $operation, $category, $duration_ms, $req_id, format!($($arg)*))
    };
}

/// Specialized security logging
#[macro_export]
macro_rules! log_security {
    (
        $level:ident,
        $operation:expr,
        $event:expr,
        $req_id:expr,
        $($arg:tt)*
    ) => {
        tracing::$level!("SECURITY:{} [{}] [req_id:{}] {}", $operation, $event, $req_id, format!($($arg)*))
    };

    // With user context
    (
        $level:ident,
        $operation:expr,
        $event:expr,
        $req_id:expr,
        $user_id:expr,
        $user_role:expr,
        $($arg:tt)*
    ) => {
        tracing::$level!("SECURITY:{} [{}] [req_id:{}] [user_id:{}] [user_role:{}] {}", 
            $operation, $event, $req_id, $user_id, $user_role, format!($($arg)*))
    };
}
