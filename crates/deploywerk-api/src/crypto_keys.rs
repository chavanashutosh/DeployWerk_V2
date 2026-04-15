//! AES-256-GCM for SSH private key material at rest (nonce || ciphertext).

use aes_gcm::aead::{Aead, AeadCore, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Nonce};

const NONCE_LEN: usize = 12;

#[derive(Debug, thiserror::Error)]
pub enum KeyCryptoError {
    #[error("invalid ciphertext")]
    Invalid,
    #[error("encryption failed")]
    Encrypt,
    #[error("decryption failed")]
    Decrypt,
}

/// Encrypt `plaintext` with `key`. Layout: 12-byte nonce || ciphertext+tag.
pub fn encrypt_private_key(key: &[u8; 32], plaintext: &[u8]) -> Result<Vec<u8>, KeyCryptoError> {
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| KeyCryptoError::Encrypt)?;
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let mut out = nonce.to_vec();
    let ct = cipher
        .encrypt(&nonce, plaintext)
        .map_err(|_| KeyCryptoError::Encrypt)?;
    out.extend(ct);
    Ok(out)
}

pub fn decrypt_private_key(key: &[u8; 32], blob: &[u8]) -> Result<Vec<u8>, KeyCryptoError> {
    if blob.len() <= NONCE_LEN {
        return Err(KeyCryptoError::Invalid);
    }
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| KeyCryptoError::Decrypt)?;
    let nonce = Nonce::from_slice(&blob[..NONCE_LEN]);
    cipher
        .decrypt(nonce, blob[NONCE_LEN..].as_ref())
        .map_err(|_| KeyCryptoError::Decrypt)
}
