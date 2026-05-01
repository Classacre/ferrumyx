//! Secure credential handling and key generation

use rand::prelude::*;
use std::io::{self, Write};

/// Generate a secure random password
pub fn generate_secure_password(length: usize) -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                            abcdefghijklmnopqrstuvwxyz\
                            0123456789\
                            !@#$%^&*()_+-=[]{}|;:,.<>?";

    let mut rng = rand::thread_rng();
    let password: String = (0..length)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect();

    password
}

/// Generate a secure random key (hex-encoded)
pub fn generate_secure_key(length: usize) -> String {
    let mut rng = rand::thread_rng();
    let mut bytes = vec![0u8; length];
    rng.fill_bytes(&mut bytes);

    hex::encode(bytes)
}

/// Generate a cryptographically secure token
pub fn generate_secure_token() -> String {
    use rand::distributions::Alphanumeric;
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect()
}

/// Mask a string for display (show first and last few characters)
pub fn mask_string(input: &str, visible_chars: usize) -> String {
    if input.len() <= visible_chars * 2 {
        "*".repeat(input.len())
    } else {
        format!(
            "{}...{}",
            &input[..visible_chars],
            &input[input.len() - visible_chars..]
        )
    }
}

/// Validate API key format for different providers
pub fn validate_api_key(provider: &str, api_key: &str) -> Result<(), String> {
    if api_key.is_empty() {
        return Ok(()); // Allow empty keys for optional providers
    }

    match provider {
        "openai" => {
            if !api_key.starts_with("sk-") {
                return Err("OpenAI API keys should start with 'sk-'".to_string());
            }
            if api_key.len() < 20 {
                return Err("OpenAI API key seems too short".to_string());
            }
        }
        "anthropic" => {
            if !api_key.starts_with("sk-ant-") {
                return Err("Anthropic API keys should start with 'sk-ant-'".to_string());
            }
            if api_key.len() < 20 {
                return Err("Anthropic API key seems too short".to_string());
            }
        }
        "gemini" => {
            if api_key.len() < 20 {
                return Err("Gemini API key seems too short".to_string());
            }
        }
        "groq" => {
            if !api_key.starts_with("gsk_") {
                return Err("Groq API keys should start with 'gsk_'".to_string());
            }
        }
        "together" => {
            if api_key.len() < 20 {
                return Err("Together AI API key seems too short".to_string());
            }
        }
        _ => {
            if api_key.len() < 10 {
                return Err("API key seems too short".to_string());
            }
        }
    }

    Ok(())
}

/// Validate JWT secret strength
pub fn validate_jwt_secret(secret: &str) -> Result<(), String> {
    if secret.len() < 32 {
        return Err("JWT secret must be at least 32 characters long".to_string());
    }

    // Check for basic complexity
    let has_upper = secret.chars().any(|c| c.is_uppercase());
    let has_lower = secret.chars().any(|c| c.is_lowercase());
    let has_digit = secret.chars().any(|c| c.is_digit(10));
    let has_special = secret.chars().any(|c| !c.is_alphanumeric());

    if !has_upper || !has_lower || !has_digit || !has_special {
        return Err("JWT secret should contain uppercase, lowercase, digits, and special characters".to_string());
    }

    Ok(())
}

/// Validate encryption key strength
pub fn validate_encryption_key(key: &str) -> Result<(), String> {
    if key.len() < 32 {
        return Err("Encryption key must be at least 32 characters long".to_string());
    }

    // For AES-256, we want exactly 32 bytes (64 hex chars) or 44 base64 chars
    if key.len() == 64 && key.chars().all(|c| c.is_ascii_hexdigit()) {
        // Valid hex-encoded 32-byte key
        Ok(())
    } else if key.len() == 44 && base64::decode(key).is_ok() {
        // Valid base64-encoded 32-byte key
        Ok(())
    } else if key.len() >= 32 {
        // Accept any string >= 32 chars as valid
        Ok(())
    } else {
        Err("Encryption key should be at least 32 characters or properly encoded".to_string())
    }
}

/// Secure input prompt with masking
pub fn secure_prompt(prompt: &str) -> Result<String, Box<dyn std::error::Error>> {
    print!("{}: ", prompt);
    io::stdout().flush()?;

    // Use rpassword for secure input
    let password = rpassword::read_password()?;

    Ok(password.trim().to_string())
}

/// Confirm sensitive action
pub fn confirm_sensitive_action(message: &str) -> Result<bool, Box<dyn std::error::Error>> {
    println!("{}", message);
    print!("Type 'yes' to confirm: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    Ok(input.trim().eq_ignore_ascii_case("yes"))
}

/// Hash a password using Argon2
pub fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    use argon2::{
        password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
        Argon2,
    };

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    let hash = argon2
        .hash_password(password.as_bytes(), &salt)?
        .to_string();

    Ok(hash)
}

/// Verify a password against its hash
pub fn verify_password(password: &str, hash: &str) -> Result<bool, argon2::password_hash::Error> {
    use argon2::{
        password_hash::{PasswordHash, PasswordVerifier},
        Argon2,
    };

    let parsed_hash = PasswordHash::new(hash)?;
    let argon2 = Argon2::default();

    Ok(argon2
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_secure_password() {
        let password = generate_secure_password(16);
        assert_eq!(password.len(), 16);
        // Should contain mix of characters
        assert!(password.chars().any(|c| c.is_uppercase()));
        assert!(password.chars().any(|c| c.is_lowercase()));
        assert!(password.chars().any(|c| c.is_digit(10)));
    }

    #[test]
    fn test_generate_secure_key() {
        let key = generate_secure_key(32);
        assert_eq!(key.len(), 64); // 32 bytes * 2 hex chars
        assert!(key.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_mask_string() {
        let input = "verylongpassword";
        let masked = mask_string(input, 3);
        assert_eq!(masked, "ver...ord");
    }

    #[test]
    fn test_validate_openai_key() {
        assert!(validate_api_key("openai", "sk-12345678901234567890").is_ok());
        assert!(validate_api_key("openai", "sk-short").is_err());
        assert!(validate_api_key("openai", "not-sk-").is_err());
    }

    #[test]
    fn test_validate_jwt_secret() {
        assert!(validate_jwt_secret("short").is_err());
        assert!(validate_jwt_secret("VerySecurePassword123!@#").is_ok());
        assert!(validate_jwt_secret("nouppercaseordigit123!").is_err());
    }

    #[test]
    fn test_validate_encryption_key() {
        assert!(validate_encryption_key("short").is_err());
        assert!(validate_encryption_key("a".repeat(32)).is_ok());
        // Test hex key
        let hex_key = "a".repeat(64);
        assert!(validate_encryption_key(&hex_key).is_ok());
    }

    #[tokio::test]
    async fn test_password_hashing() {
        let password = "testpassword123";
        let hash = hash_password(password).unwrap();
        assert!(verify_password(password, &hash).unwrap());
        assert!(!verify_password("wrongpassword", &hash).unwrap());
    }
}