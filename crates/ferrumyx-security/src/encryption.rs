//! Encryption utilities for security compliance

use aes_gcm::{Aes256Gcm, Key, Nonce, KeyInit};
use aes_gcm::aead::Aead;
use argon2::{Argon2, PasswordHasher};
use argon2::password_hash::{PasswordHash, PasswordVerifier, SaltString};
use ring::digest::{Context, SHA256};
use rand::Rng;
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};

/// Encryption manager for handling cryptographic operations
pub struct EncryptionManager {
    key: Key<Aes256Gcm>,
}

impl EncryptionManager {
    /// Create new encryption manager
    pub fn new() -> anyhow::Result<Self> {
        // In production, this should use a proper key management system
        // For now, derive key from environment or generate one
        let key_bytes = if let Ok(key_str) = std::env::var("FERRUMYX_ENCRYPTION_KEY") {
            hex::decode(key_str)?
        } else {
            // Generate a random key for development (NOT for production!)
            rand::thread_rng().gen::<[u8; 32]>().to_vec()
        };

        let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
        Ok(Self { key: *key })
    }

    /// Encrypt data
    pub fn encrypt(&self, data: &[u8]) -> anyhow::Result<String> {
        let cipher = Aes256Gcm::new(&self.key);
        let nonce_bytes: [u8; 12] = rand::thread_rng().gen();
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher.encrypt(nonce, data)
            .map_err(|e| anyhow::anyhow!("Encryption failed: {:?}", e))?;

        // Combine nonce and ciphertext
        let mut result = nonce_bytes.to_vec();
        result.extend(ciphertext);

        Ok(BASE64.encode(result))
    }

    /// Decrypt data
    pub fn decrypt(&self, encrypted_data: &str) -> anyhow::Result<Vec<u8>> {
        let cipher = Aes256Gcm::new(&self.key);
        let data = BASE64.decode(encrypted_data)?;

        if data.len() < 12 {
            return Err(anyhow::anyhow!("Invalid encrypted data"));
        }

        let nonce = Nonce::from_slice(&data[..12]);
        let ciphertext = &data[12..];

        let plaintext = cipher.decrypt(nonce, ciphertext)
            .map_err(|e| anyhow::anyhow!("Decryption failed: {:?}", e))?;

        Ok(plaintext)
    }

    /// Hash data using SHA-256
    pub fn hash_data(&self, data: &[u8]) -> String {
        let mut context = Context::new(&SHA256);
        context.update(data);
        let digest = context.finish();
        hex::encode(digest.as_ref())
    }

    /// Hash password using Argon2
    pub fn hash_password(&self, password: &str) -> anyhow::Result<String> {
        let salt = SaltString::generate(&mut rand::thread_rng());
        let argon2 = Argon2::default();

        let password_hash = argon2.hash_password(password.as_bytes(), &salt)
            .map_err(|e| anyhow::anyhow!("Password hashing failed: {:?}", e))?
            .to_string();

        Ok(password_hash)
    }

    /// Verify password hash
    pub fn verify_password(&self, password: &str, hash: &str) -> anyhow::Result<bool> {
        let parsed_hash = PasswordHash::new(hash)
            .map_err(|e| anyhow::anyhow!("Invalid password hash: {:?}", e))?;
        let argon2 = Argon2::default();

        Ok(argon2.verify_password(password.as_bytes(), &parsed_hash).is_ok())
    }

    /// Generate secure random token
    pub fn generate_token(&self) -> String {
        let random_bytes: [u8; 32] = rand::thread_rng().gen();
        hex::encode(random_bytes)
    }
}

/// Key management utilities
pub struct KeyManager;

impl KeyManager {
    /// Rotate encryption keys (for key rotation compliance)
    pub fn rotate_keys(old_manager: &EncryptionManager) -> anyhow::Result<EncryptionManager> {
        // In production, this would involve:
        // 1. Generate new key
        // 2. Re-encrypt existing data with new key
        // 3. Update key references
        // 4. Securely destroy old key

        // For now, just create a new manager
        EncryptionManager::new()
    }

    /// Validate key strength
    pub fn validate_key_strength(key: &[u8]) -> KeyStrength {
        match key.len() {
            16 => KeyStrength::Weak, // 128-bit
            24 => KeyStrength::Medium, // 192-bit
            32 => KeyStrength::Strong, // 256-bit
            _ => KeyStrength::Invalid,
        }
    }
}

/// Key strength levels
#[derive(Debug, Clone, PartialEq)]
pub enum KeyStrength {
    Invalid,
    Weak,
    Medium,
    Strong,
}