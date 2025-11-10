use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose, Engine as _};
use std::error::Error as StdError;

/// Encryption key derived from environment variable
pub struct EncryptionKey {
    cipher: Aes256Gcm,
}

impl EncryptionKey {
    /// Create encryption key from environment variable
    pub fn from_env() -> Result<Self, Box<dyn StdError + Send + Sync>> {
        let key_base64 = std::env::var("ENCRYPTION_KEY")
            .map_err(|_| "ENCRYPTION_KEY environment variable not set")?;

        let key_bytes = general_purpose::STANDARD
            .decode(key_base64)
            .map_err(|e| format!("Failed to decode encryption key: {}", e))?;

        if key_bytes.len() != 32 {
            return Err("Encryption key must be 32 bytes (256 bits)".into());
        }

        let cipher = Aes256Gcm::new_from_slice(&key_bytes)
            .map_err(|e| format!("Failed to create cipher: {}", e))?;

        Ok(Self { cipher })
    }

    /// Generate a new random encryption key (for initialization)
    pub fn generate() -> String {
        let key = Aes256Gcm::generate_key(&mut OsRng);
        general_purpose::STANDARD.encode(key)
    }

    /// Encrypt plaintext and return (ciphertext, nonce) as base64 strings
    pub fn encrypt(&self, plaintext: &str) -> Result<(String, String), Box<dyn StdError + Send + Sync>> {
        // Generate random nonce
        let nonce_bytes = Aes256Gcm::generate_nonce(&mut OsRng);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Encrypt
        let ciphertext = self.cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| format!("Encryption failed: {}", e))?;

        // Encode to base64
        let ciphertext_b64 = general_purpose::STANDARD.encode(&ciphertext);
        let nonce_b64 = general_purpose::STANDARD.encode(nonce);

        Ok((ciphertext_b64, nonce_b64))
    }

    /// Decrypt ciphertext using the provided nonce (both as base64 strings)
    pub fn decrypt(&self, ciphertext_b64: &str, nonce_b64: &str) -> Result<String, Box<dyn StdError + Send + Sync>> {
        // Decode from base64
        let ciphertext = general_purpose::STANDARD
            .decode(ciphertext_b64)
            .map_err(|e| format!("Failed to decode ciphertext: {}", e))?;

        let nonce_bytes = general_purpose::STANDARD
            .decode(nonce_b64)
            .map_err(|e| format!("Failed to decode nonce: {}", e))?;

        let nonce = Nonce::from_slice(&nonce_bytes);

        // Decrypt
        let plaintext = self.cipher
            .decrypt(nonce, ciphertext.as_ref())
            .map_err(|e| format!("Decryption failed: {}", e))?;

        String::from_utf8(plaintext)
            .map_err(|e| format!("Invalid UTF-8: {}", e).into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_roundtrip() {
        // Set a test encryption key
        std::env::set_var("ENCRYPTION_KEY", EncryptionKey::generate());

        let key = EncryptionKey::from_env().unwrap();
        let plaintext = "sensitive_cookie_value";

        let (ciphertext, nonce) = key.encrypt(plaintext).unwrap();
        let decrypted = key.decrypt(&ciphertext, &nonce).unwrap();

        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_generate_key() {
        let key1 = EncryptionKey::generate();
        let key2 = EncryptionKey::generate();

        // Keys should be different
        assert_ne!(key1, key2);

        // Keys should be valid base64 and decode to 32 bytes
        let decoded = general_purpose::STANDARD.decode(&key1).unwrap();
        assert_eq!(decoded.len(), 32);
    }
}
