//! Shared GitHub webhook HMAC verification (repository hooks + GitHub App deliveries).

use hmac::{Hmac, Mac};
use sha2::Sha256;
use subtle::ConstantTimeEq;

use crate::error::ApiError;

type HmacSha256 = Hmac<Sha256>;

/// Verify `X-Hub-Signature-256: sha256=<hex>` against the raw body (GitHub repository + App webhooks).
pub fn verify_github_webhook_hmac_sha256(
    secret: &str,
    body: &[u8],
    sig_header: Option<&str>,
) -> Result<(), ApiError> {
    let sig_header = sig_header.ok_or(ApiError::Unauthorized)?;
    let prefix = "sha256=";
    if !sig_header.starts_with(prefix) {
        return Err(ApiError::Unauthorized);
    }
    let sig_hex = &sig_header[prefix.len()..];
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).map_err(|_| ApiError::Internal)?;
    mac.update(body);
    let expected = mac.finalize().into_bytes();
    let got = hex::decode(sig_hex).map_err(|_| ApiError::Unauthorized)?;
    if got.len() != expected.len()
        || expected.as_slice().ct_eq(got.as_slice()).unwrap_u8() != 1
    {
        return Err(ApiError::Unauthorized);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hmac_accepts_valid_signature() {
        let secret = "mysecret";
        let body = br#"{"action":"opened"}"#;
        type Inner = hmac::Hmac<sha2::Sha256>;
        let mut mac = Inner::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body);
        let h = hex::encode(mac.finalize().into_bytes());
        let hdr = format!("sha256={h}");
        assert!(verify_github_webhook_hmac_sha256(secret, body, Some(&hdr)).is_ok());
    }

    #[test]
    fn hmac_rejects_bad_sig() {
        let err = verify_github_webhook_hmac_sha256(
            "s",
            b"{}",
            Some("sha256=0000000000000000000000000000000000000000000000000000000000000000"),
        );
        assert!(err.is_err());
    }
}
