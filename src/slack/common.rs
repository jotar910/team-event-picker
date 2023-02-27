use chrono::Utc;
use hmac::{Hmac, Mac};
use hyper::HeaderMap;
use sha2::Sha256;

pub fn verify_signature(headers: HeaderMap, body: String, secret: &str) -> bool {
    if !headers.contains_key("x-slack-request-timestamp")
        || !headers.contains_key("x-slack-signature")
    {
        log::debug!("unable to find authentication headers");
        return false;
    }

    let timestamp: i64 = headers
        .get("x-slack-request-timestamp")
        .unwrap()
        .to_str()
        .unwrap_or("")
        .parse()
        .unwrap_or(0);

    // verify that the timestamp does not differ from local time by more than five minutes
    if (Utc::now().timestamp() - timestamp).abs() > 300 {
        log::debug!("request is too old");
        return false;
    }

    let base_str = format!("v0:{}:{}", timestamp, body);

    let expected_signature = calculate_signature(&base_str, secret);

    let received_signature: String = headers
        .get("x-slack-signature")
        .unwrap()
        .to_str()
        .unwrap_or("")
        .to_string();

    // match the two signatures
    if expected_signature != received_signature {
        log::debug!("webhook signature mismatch");
        return false;
    }

    log::debug!("webhook signature verified");
    true
}

fn calculate_signature(base_str: &str, secret: &str) -> String {
    let mut mac =
        Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");
    mac.update(base_str.as_bytes());
    let result = mac.finalize().into_bytes();
    format!("v0={}", hex::encode(result))
}
