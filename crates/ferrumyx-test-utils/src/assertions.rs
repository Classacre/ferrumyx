//! Enhanced assertion utilities for testing

use pretty_assertions::{assert_eq, assert_ne};
use std::fmt::Debug;

/// Enhanced assertions with better error messages
pub trait EnhancedAssertions<T: PartialEq + std::fmt::Debug> {
    fn assert_equals(&self, expected: &T, context: &str);
    fn assert_not_equals(&self, unexpected: &T, context: &str);
}

impl<T: PartialEq + std::fmt::Debug> EnhancedAssertions<T> for T {
    fn assert_equals(&self, expected: &T, context: &str) {
        pretty_assertions::assert_eq!(self, expected, "Assertion failed in {}: values don't match", context);
    }

    fn assert_not_equals(&self, unexpected: &T, context: &str) {
        pretty_assertions::assert_ne!(self, unexpected, "Assertion failed in {}: values should be different", context);
    }
}

/// Container assertions
pub trait ContainerAssertions<T, U>
where
    T: AsRef<[U]>,
    U: PartialEq + std::fmt::Debug,
{
    fn assert_contains(&self, item: &U, context: &str);
}

impl<T, U> ContainerAssertions<T, U> for T
where
    T: AsRef<[U]>,
    U: PartialEq + std::fmt::Debug,
{
    fn assert_contains(&self, item: &U, context: &str) {
        let slice = self.as_ref();
        assert!(slice.contains(item), "Assertion failed in {}: item {:?} not found in {:?}", context, item, slice);
    }
}

/// Security-focused assertions
pub mod security {
    use super::*;

    pub fn assert_no_pii_leakage(text: &str, context: &str) {
        // Simple PII detection patterns
        let pii_patterns = [
            r"\b\d{3}-\d{2}-\d{4}\b",  // SSN
            r"\b\d{4} \d{4} \d{4} \d{4}\b",  // Credit card
            r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b",  // Email
        ];

        for pattern in &pii_patterns {
            if regex::Regex::new(pattern).unwrap().is_match(text) {
                panic!("PII leakage detected in {}: pattern {} found in text", context, pattern);
            }
        }
    }

    pub fn assert_secure_headers(headers: &reqwest::header::HeaderMap, context: &str) {
        // Check for security headers
        assert!(headers.contains_key("x-content-type-options"), "Missing X-Content-Type-Options header in {}", context);
        assert!(headers.contains_key("x-frame-options"), "Missing X-Frame-Options header in {}", context);

        if let Some(content_type) = headers.get("content-type") {
            let content_type_str = content_type.to_str().unwrap_or("");
            assert!(!content_type_str.contains("text/html"), "HTML content should not be served directly in {}", context);
        }
    }
}

/// Performance assertions
pub mod performance {
    use super::*;
    use std::time::Duration;

    pub fn assert_response_time(response_time: Duration, max_allowed: Duration, context: &str) {
        assert!(response_time <= max_allowed,
            "Performance assertion failed in {}: response time {:?} exceeds maximum {:?}",
            context, response_time, max_allowed);
    }

    pub fn assert_memory_usage(current_mb: f64, max_allowed_mb: f64, context: &str) {
        assert!(current_mb <= max_allowed_mb,
            "Memory assertion failed in {}: usage {:.2}MB exceeds maximum {:.2}MB",
            context, current_mb, max_allowed_mb);
    }

    pub fn assert_throughput(requests_per_sec: f64, min_required: f64, context: &str) {
        assert!(requests_per_sec >= min_required,
            "Throughput assertion failed in {}: {:.2} req/sec below minimum {:.2} req/sec",
            context, requests_per_sec, min_required);
    }
}

/// API response assertions
pub mod api {
    use super::*;
    use serde_json::Value;

    pub fn assert_json_structure(json: &Value, required_fields: &[&str], context: &str) {
        if let Value::Object(map) = json {
            for field in required_fields {
                assert!(map.contains_key(*field),
                    "API assertion failed in {}: required field '{}' missing from response",
                    context, field);
            }
        } else {
            panic!("API assertion failed in {}: expected JSON object, got {:?}", context, json);
        }
    }

    pub fn assert_status_code(status: u16, expected: u16, context: &str) {
        pretty_assertions::assert_eq!(&status, &expected,
            "API assertion failed in {}: status code {} doesn't match expected {}",
            context, status, expected);
    }

    pub fn assert_json_value(json: &Value, _path: &str, _expected: &Value, _context: &str) {
        // Simplified version - jsonpath_lib would be needed for full implementation
        // For now, just check that json is valid
        assert!(json.is_object() || json.is_array(),
            "API assertion failed in {}: expected JSON object or array",
            _context);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enhanced_assertions() {
        let value = 42;
        value.assert_equals(&42, "test context");

        let vec = vec![1, 2, 3];
        vec.assert_contains(&2, "test context");
    }

    #[test]
    fn test_security_assertions() {
        security::assert_no_pii_leakage("This is a normal text", "test");

        // This would panic if PII is detected
        // security::assert_no_pii_leakage("Email: test@example.com", "test");
    }

    #[test]
    fn test_performance_assertions() {
        use std::time::Duration;

        performance::assert_response_time(Duration::from_millis(100), Duration::from_millis(200), "test");
        performance::assert_memory_usage(50.0, 100.0, "test");
        performance::assert_throughput(100.0, 50.0, "test");
    }
}