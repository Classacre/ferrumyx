//! Mock implementations for testing

use std::collections::HashMap;
use serde_json::Value;

/// Simple mock HTTP client for testing
pub struct MockHttpClient {
    responses: HashMap<String, Value>,
    errors: HashMap<String, String>,
}

impl MockHttpClient {
    pub fn new() -> Self {
        Self {
            responses: HashMap::new(),
            errors: HashMap::new(),
        }
    }

    pub fn mock_response(&mut self, url: &str, response: Value) {
        self.responses.insert(url.to_string(), response);
    }

    pub fn mock_error(&mut self, url: &str, error: &str) {
        self.errors.insert(url.to_string(), error.to_string());
    }

    pub async fn get(&self, url: &str) -> Result<Value, anyhow::Error> {
        if let Some(error) = self.errors.get(url) {
            return Err(anyhow::anyhow!("Mock error: {}", error));
        }

        if let Some(response) = self.responses.get(url) {
            Ok(response.clone())
        } else {
            Err(anyhow::anyhow!("No mock response configured for URL: {}", url))
        }
    }
}

impl Default for MockHttpClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Mock database for testing
pub struct MockDatabase {
    data: HashMap<String, Value>,
}

impl MockDatabase {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    pub async fn get(&self, key: &str) -> Result<Option<Value>, anyhow::Error> {
        Ok(self.data.get(key).cloned())
    }

    pub async fn set(&mut self, key: String, value: Value) -> Result<(), anyhow::Error> {
        self.data.insert(key, value);
        Ok(())
    }

    pub async fn delete(&mut self, key: &str) -> Result<bool, anyhow::Error> {
        Ok(self.data.remove(key).is_some())
    }
}

impl Default for MockDatabase {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_http_client() {
        let mut client = MockHttpClient::new();
        let response = serde_json::json!({"status": "ok"});

        client.mock_response("http://test.com/api", response.clone());

        let result = client.get("http://test.com/api").await.unwrap();
        assert_eq!(result, response);

        client.mock_error("http://error.com/api", "Network error");
        let error_result = client.get("http://error.com/api").await;
        assert!(error_result.is_err());
    }

    #[tokio::test]
    async fn test_mock_database() {
        let mut db = MockDatabase::new();
        let value = serde_json::json!({"name": "test"});

        db.set("key1".to_string(), value.clone()).await.unwrap();
        let retrieved = db.get("key1").await.unwrap().unwrap();
        assert_eq!(retrieved, value);

        let deleted = db.delete("key1").await.unwrap();
        assert!(deleted);

        let not_found = db.get("key1").await.unwrap();
        assert!(not_found.is_none());
    }
}