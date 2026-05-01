//! Input validation and connectivity testing

use reqwest::Client;
use std::time::Duration;

/// Validate database URL format
pub fn validate_database_url(url: &str) -> Result<(), String> {
    if url.is_empty() {
        return Err("Database URL cannot be empty".to_string());
    }

    // Basic URL validation
    if !url.contains("://") {
        return Err("Database URL must include protocol (e.g., postgresql://)".to_string());
    }

    // Check for common database protocols
    let valid_protocols = ["postgresql://", "postgres://", "sqlite://", "mysql://", "mariadb://"];
    let has_valid_protocol = valid_protocols.iter().any(|&proto| url.starts_with(proto));

    if !has_valid_protocol {
        return Err(format!(
            "Unsupported database protocol. Supported: {}",
            valid_protocols.join(", ")
        ));
    }

    // For PostgreSQL URLs, check for required components
    if url.starts_with("postgresql://") || url.starts_with("postgres://") {
        if !url.contains("@") {
            return Err("PostgreSQL URL should include credentials (user:password@host)".to_string());
        }

        // Check if password placeholder is used correctly
        if url.contains("${") && !url.contains("}") {
            return Err("Unclosed environment variable placeholder".to_string());
        }
    }

    Ok(())
}

/// Validate LLM provider configuration
pub fn validate_llm_provider(provider: &str, api_key: Option<&str>, base_url: &str) -> Result<(), String> {
    match provider {
        "openai" => {
            if let Some(key) = api_key {
                if key.is_empty() {
                    return Err("OpenAI API key cannot be empty".to_string());
                }
                super::security::validate_api_key("openai", key)?;
            }
            if base_url.is_empty() {
                return Err("OpenAI base URL cannot be empty".to_string());
            }
        }
        "anthropic" => {
            if let Some(key) = api_key {
                if key.is_empty() {
                    return Err("Anthropic API key cannot be empty".to_string());
                }
                super::security::validate_api_key("anthropic", key)?;
            }
            if base_url.is_empty() {
                return Err("Anthropic base URL cannot be empty".to_string());
            }
        }
        "gemini" => {
            if let Some(key) = api_key {
                if key.is_empty() {
                    return Err("Gemini API key cannot be empty".to_string());
                }
                super::security::validate_api_key("gemini", key)?;
            }
            if base_url.is_empty() {
                return Err("Gemini base URL cannot be empty".to_string());
            }
        }
        "ollama" => {
            if base_url.is_empty() {
                return Err("Ollama base URL cannot be empty".to_string());
            }
            // Ollama typically doesn't require API keys
        }
        "openai_compatible" => {
            if base_url.is_empty() {
                return Err("Base URL cannot be empty".to_string());
            }
            if let Some(key) = api_key {
                if !key.is_empty() {
                    // For compatible endpoints, we just check it's not obviously wrong
                    if key.len() < 10 {
                        return Err("API key seems too short for compatible endpoint".to_string());
                    }
                }
            }
        }
        _ => {
            return Err(format!("Unsupported LLM provider: {}", provider));
        }
    }

    Ok(())
}

/// Test database connectivity
pub async fn test_database_connection(url: &str) -> Result<(), String> {
    // This is a basic connectivity test - in a real implementation,
    // you'd want to establish an actual database connection

    if url.starts_with("sqlite://") {
        // For SQLite, just check if the file path is accessible
        let path = url.trim_start_matches("sqlite://");
        if !path.is_empty() && !std::path::Path::new(path).parent().map_or(true, |p| p.exists()) {
            return Err("SQLite database directory does not exist".to_string());
        }
        return Ok(());
    }

    // For network databases, we could attempt a basic connection
    // but for now, we'll just validate the URL format
    if url.contains("localhost") || url.contains("127.0.0.1") {
        // For local development, assume it's OK if the URL is well-formed
        return Ok(());
    }

    // For production URLs, we'd want to test the actual connection
    // but that might require credentials and could be slow
    println!("Note: Database connectivity test skipped for remote URLs");
    Ok(())
}

/// Test LLM provider connectivity
pub async fn test_llm_provider(provider: &str, api_key: Option<&str>, base_url: &str, model: &str) -> Result<(), String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    match provider {
        "ollama" => {
            // Test Ollama connectivity
            let test_url = format!("{}/api/tags", base_url.trim_end_matches('/'));
            let response = client
                .get(&test_url)
                .send()
                .await
                .map_err(|e| format!("Cannot connect to Ollama at {}: {}", base_url, e))?;

            if !response.status().is_success() {
                return Err(format!("Ollama returned status: {}", response.status()));
            }

            // Check if the requested model is available
            let body: serde_json::Value = response
                .json()
                .await
                .map_err(|e| format!("Failed to parse Ollama response: {}", e))?;

            if let Some(models) = body.get("models").and_then(|m| m.as_array()) {
                let model_exists = models.iter().any(|m| {
                    m.get("name").and_then(|n| n.as_str()) == Some(model)
                });

                if !model_exists {
                    println!("Warning: Model '{}' not found in Ollama. Available models: {}",
                        model,
                        models.iter()
                            .filter_map(|m| m.get("name").and_then(|n| n.as_str()))
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                }
            }
        }
        "openai" => {
            if let Some(key) = api_key {
                // Test OpenAI API connectivity
                let response = client
                    .get("https://api.openai.com/v1/models")
                    .header("Authorization", format!("Bearer {}", key))
                    .send()
                    .await
                    .map_err(|e| format!("Cannot connect to OpenAI API: {}", e))?;

                if response.status() == 401 {
                    return Err("OpenAI API key is invalid".to_string());
                } else if !response.status().is_success() {
                    return Err(format!("OpenAI API returned status: {}", response.status()));
                }
            }
        }
        "anthropic" => {
            if let Some(key) = api_key {
                // Test Anthropic API connectivity
                let response = client
                    .get("https://api.anthropic.com/v1/messages")
                    .header("x-api-key", key)
                    .header("anthropic-version", "2023-06-01")
                    .send()
                    .await
                    .map_err(|e| format!("Cannot connect to Anthropic API: {}", e))?;

                if response.status() == 401 {
                    return Err("Anthropic API key is invalid".to_string());
                } else if response.status() == 403 {
                    return Err("Anthropic API key does not have permission".to_string());
                } else if !response.status().is_success() && response.status() != 400 {
                    // 400 is expected for GET without proper body
                    return Err(format!("Anthropic API returned status: {}", response.status()));
                }
            }
        }
        "gemini" => {
            if let Some(key) = api_key {
                // Test Gemini API connectivity
                let test_url = format!("https://generativelanguage.googleapis.com/v1beta/models?key={}", key);
                let response = client
                    .get(&test_url)
                    .send()
                    .await
                    .map_err(|e| format!("Cannot connect to Gemini API: {}", e))?;

                if response.status() == 400 {
                    return Err("Gemini API key is invalid".to_string());
                } else if !response.status().is_success() {
                    return Err(format!("Gemini API returned status: {}", response.status()));
                }
            }
        }
        "openai_compatible" => {
            if let Some(key) = api_key {
                // Test compatible API connectivity
                let test_url = format!("{}/models", base_url.trim_end_matches('/'));
                let mut request = client.get(&test_url);

                if !key.is_empty() {
                    request = request.header("Authorization", format!("Bearer {}", key));
                }

                let response = request
                    .send()
                    .await
                    .map_err(|e| format!("Cannot connect to API at {}: {}", base_url, e))?;

                if !response.status().is_success() {
                    return Err(format!("API returned status: {}", response.status()));
                }
            }
        }
        _ => {
            return Err(format!("Connectivity test not implemented for provider: {}", provider));
        }
    }

    Ok(())
}

/// Validate port number
pub fn validate_port(port: &str) -> Result<(), String> {
    match port.parse::<u16>() {
        Ok(p) => {
            if p == 0 {
                Err("Port cannot be 0".to_string())
            } else if p < 1024 {
                println!("Warning: Using privileged port {}", p);
                Ok(())
            } else {
                Ok(())
            }
        }
        Err(_) => Err("Invalid port number".to_string()),
    }
}

/// Validate URL format
pub fn validate_url(url: &str) -> Result<(), String> {
    if url.is_empty() {
        return Err("URL cannot be empty".to_string());
    }

    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("URL must start with http:// or https://".to_string());
    }

    // Basic URL validation using url crate if available
    match url::Url::parse(url) {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Invalid URL format: {}", e)),
    }
}

/// Validate email format
pub fn validate_email(email: &str) -> Result<(), String> {
    if email.is_empty() {
        return Err("Email cannot be empty".to_string());
    }

    // Basic email validation
    if !email.contains('@') || !email.contains('.') {
        return Err("Invalid email format".to_string());
    }

    let parts: Vec<&str> = email.split('@').collect();
    if parts.len() != 2 {
        return Err("Invalid email format".to_string());
    }

    let domain_parts: Vec<&str> = parts[1].split('.').collect();
    if domain_parts.len() < 2 {
        return Err("Invalid email domain".to_string());
    }

    Ok(())
}

/// Validate file path exists and is accessible
pub fn validate_file_path(path: &str) -> Result<(), String> {
    if path.is_empty() {
        return Err("File path cannot be empty".to_string());
    }

    let path_obj = std::path::Path::new(path);

    if !path_obj.exists() {
        return Err(format!("File does not exist: {}", path));
    }

    if !path_obj.is_file() {
        return Err(format!("Path is not a file: {}", path));
    }

    // Check if file is readable
    match std::fs::File::open(path) {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Cannot read file {}: {}", path, e)),
    }
}

/// Validate directory path exists and is accessible
pub fn validate_directory_path(path: &str) -> Result<(), String> {
    if path.is_empty() {
        return Err("Directory path cannot be empty".to_string());
    }

    let path_obj = std::path::Path::new(path);

    if !path_obj.exists() {
        return Err(format!("Directory does not exist: {}", path));
    }

    if !path_obj.is_dir() {
        return Err(format!("Path is not a directory: {}", path));
    }

    // Check if directory is readable
    match std::fs::read_dir(path) {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Cannot read directory {}: {}", path, e)),
    }
}

/// Run comprehensive validation on configuration
pub async fn validate_configuration(config_path: &std::path::Path, env_path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    println!("Validating configuration files...");

    // Validate TOML configuration file
    if config_path.exists() {
        let config_content = std::fs::read_to_string(config_path)?;
        let config: ferrumyx_config::FerrumyxConfig = toml::from_str(&config_content)?;

        // Validate the configuration
        config.validate()?;
        println!("✅ TOML configuration is valid");
    } else {
        println!("⚠️  TOML configuration file not found: {}", config_path.display());
    }

    // Validate environment file
    if env_path.exists() {
        let env_content = std::fs::read_to_string(env_path)?;
        validate_env_file(&env_content)?;
        println!("✅ Environment file is valid");
    } else {
        println!("⚠️  Environment file not found: {}", env_path.display());
    }

    Ok(())
}

/// Validate .env file content
fn validate_env_file(content: &str) -> Result<(), Box<dyn std::error::Error>> {
    for (line_num, line) in content.lines().enumerate() {
        let line = line.trim();

        // Skip comments and empty lines
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Check for key=value format
        if !line.contains('=') {
            return Err(format!("Invalid format at line {}: {}", line_num + 1, line).into());
        }

        let parts: Vec<&str> = line.splitn(2, '=').collect();
        if parts.len() != 2 {
            return Err(format!("Invalid key-value pair at line {}: {}", line_num + 1, line).into());
        }

        let key = parts[0].trim();
        let _value = parts[1].trim();

        // Validate key format
        if key.is_empty() {
            return Err(format!("Empty key at line {}: {}", line_num + 1, line).into());
        }

        if key.chars().any(|c| !c.is_alphanumeric() && c != '_') {
            return Err(format!("Invalid key format at line {}: {}", line_num + 1, key).into());
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_database_url() {
        assert!(validate_database_url("postgresql://user:pass@localhost:5432/db").is_ok());
        assert!(validate_database_url("sqlite://./data/db.sqlite").is_ok());
        assert!(validate_database_url("invalid").is_err());
        assert!(validate_database_url("postgresql://nouser").is_err());
    }

    #[test]
    fn test_validate_url() {
        assert!(validate_url("https://example.com").is_ok());
        assert!(validate_url("http://localhost:3000").is_ok());
        assert!(validate_url("invalid").is_err());
        assert!(validate_url("").is_err());
    }

    #[test]
    fn test_validate_email() {
        assert!(validate_email("user@example.com").is_ok());
        assert!(validate_email("test.email@domain.co.uk").is_ok());
        assert!(validate_email("invalid").is_err());
        assert!(validate_email("@example.com").is_err());
    }

    #[test]
    fn test_validate_port() {
        assert!(validate_port("8080").is_ok());
        assert!(validate_port("80").is_ok());
        assert!(validate_port("0").is_err());
        assert!(validate_port("99999").is_err());
        assert!(validate_port("invalid").is_err());
    }

    #[tokio::test]
    async fn test_validate_env_file() {
        let valid_content = r#"
# Comment
KEY1=value1
KEY2=value with spaces
KEY_3=123
"#;
        assert!(validate_env_file(valid_content).is_ok());

        let invalid_content = "invalid line without equals";
        assert!(validate_env_file(invalid_content).is_err());
    }
}