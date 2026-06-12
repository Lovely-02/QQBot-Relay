use ed25519_dalek::{Signer, SigningKey};
use hmac::{Hmac, Mac};
use sha2::Sha256;

/// 为 QQ 机器人平台 Webhook 验证生成 Ed25519 签名。
/// 密钥会被填充/截断至 32 字节并用作 Ed25519 私钥。
pub fn generate_ed25519_signature(bot_secret: &[u8], event_ts: &str, plain_token: &str) -> Vec<u8> {
    let mut key_bytes = [0u8; 32];
    let len = bot_secret.len().min(32);
    key_bytes[..len].copy_from_slice(&bot_secret[..len]);
    let signing_key = SigningKey::from_bytes(&key_bytes);
    let message = format!("{}{}", event_ts, plain_token);
    signing_key.sign(message.as_bytes()).to_bytes().to_vec()
}

/// 验证 Webhook 请求的 HMAC-SHA256 签名。
/// 计算 hmac-sha256(secret, timestamp + nonce + body) 并与提供的签名进行比较。
pub fn verify_signature(
    secret: &str,
    signature: &str,
    timestamp: &str,
    nonce: &str,
    body: &str,
) -> bool {
    let timestamp_value = match timestamp.parse::<i64>() {
        Ok(value) => value,
        Err(_) => return false,
    };
    if (chrono::Utc::now().timestamp() - timestamp_value).abs() > 300 {
        return false;
    }

    let mut mac = match Hmac::<Sha256>::new_from_slice(secret.as_bytes()) {
        Ok(m) => m,
        Err(_) => return false,
    };
    mac.update(timestamp.as_bytes());
    mac.update(nonce.as_bytes());
    mac.update(body.as_bytes());
    let provided = match hex::decode(signature) {
        Ok(value) => value,
        Err(_) => return false,
    };
    mac.verify_slice(&provided).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hmac_signature_rejects_stale_timestamps() {
        let timestamp = (chrono::Utc::now().timestamp() - 301).to_string();
        assert!(!verify_signature("secret", "00", &timestamp, "nonce", "{}"));
    }

    #[test]
    fn hmac_signature_accepts_current_timestamp() {
        let secret = "secret";
        let timestamp = chrono::Utc::now().timestamp().to_string();
        let nonce = "nonce";
        let body = "{}";
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(timestamp.as_bytes());
        mac.update(nonce.as_bytes());
        mac.update(body.as_bytes());
        let signature = hex::encode(mac.finalize().into_bytes());

        assert!(verify_signature(
            secret, &signature, &timestamp, nonce, body
        ));
    }
}
