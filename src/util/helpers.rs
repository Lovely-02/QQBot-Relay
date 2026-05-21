use ed25519_dalek::{Signer, SigningKey};
use hmac::{Hmac, Mac};
use sha2::Sha256;

/// Generate Ed25519 signature for QQ Bot platform webhook verification.
/// The secret is padded/truncated to 32 bytes and used as the Ed25519 private key.
pub fn generate_ed25519_signature(bot_secret: &[u8], event_ts: &str, plain_token: &str) -> Vec<u8> {
    let mut key_bytes = [0u8; 32];
    let len = bot_secret.len().min(32);
    key_bytes[..len].copy_from_slice(&bot_secret[..len]);
    let signing_key = SigningKey::from_bytes(&key_bytes);
    let message = format!("{}{}", event_ts, plain_token);
    signing_key.sign(message.as_bytes()).to_bytes().to_vec()
}

/// Verify HMAC-SHA256 signature for webhook requests.
/// Computes hmac-sha256(secret, timestamp + nonce + body) and compares with the provided signature.
pub fn verify_signature(secret: &str, signature: &str, timestamp: &str, nonce: &str, body: &str) -> bool {
    let mut mac = match Hmac::<Sha256>::new_from_slice(secret.as_bytes()) {
        Ok(m) => m,
        Err(_) => return false,
    };
    mac.update(timestamp.as_bytes());
    mac.update(nonce.as_bytes());
    mac.update(body.as_bytes());
    let computed = hex::encode(mac.finalize().into_bytes());
    tracing::debug!(
        "[签名验证] secret={}, timestamp={}, nonce={}, body_len={}, received={}, computed={}, match={}",
        secret, timestamp, nonce, body.len(), signature, computed, computed == signature
    );
    computed == signature
}
