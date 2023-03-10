use chrono::Utc;
use handlebars::Handlebars;
use hmac::{Hmac, Mac};
use hyper::{HeaderMap, Request};
use hyper_tls::HttpsConnector;
use sha2::Sha256;

pub fn render_template(
    template: &str,
    context: serde_json::Value,
) -> Result<String, Box<dyn std::error::Error>> {
    let reg = Handlebars::new();
    Ok(reg.render_template(&template, &context)?)
}

pub async fn send_post(url: &str, body: hyper::Body) -> Result<(), Box<dyn std::error::Error>> {
    let https = HttpsConnector::new();
    let client = hyper::Client::builder().build(https);

    let req = Request::builder()
        .method(hyper::Method::POST)
        .uri(url)
        .header("Content-Type", "application/json")
        .body(body)?;

    log::trace!("sending action response to {}: {:?}", url, &req);

    let res = client.request(req).await?;

    let res_str = format!("{:?}", res);
    let body = hyper::body::to_bytes(res).await;

    log::trace!("response received from request to {}: {}: {:?}", url, res_str, body);

    Ok(())
}

pub async fn send_authorized_post(url: &str, token: &str, body: hyper::Body) -> Result<(), Box<dyn std::error::Error>> {
    let https = HttpsConnector::new();
    let client = hyper::Client::builder().build(https);

    let req = Request::builder()
        .method(hyper::Method::POST)
        .uri(url)
        .header("Content-Type", "application/json")
        .header("Authorization", String::from("Bearer ") + token)
        .body(body)?;

    log::trace!("sending authorized request to {}: {:?}", url, &req);

    let res = client.request(req).await?;

    let res_str = format!("{:?}", res);
    let body = hyper::body::to_bytes(res).await;

    log::trace!("authorized response received from request to {}: {}: {:?}", url, res_str, body);

    Ok(())
}

pub fn verify_signature(headers: HeaderMap, body: String, secret: &str) -> bool {
    if !headers.contains_key("x-slack-request-timestamp")
        || !headers.contains_key("x-slack-signature")
    {
        log::trace!("unable to find authentication headers");
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
        log::trace!("request is too old");
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
        log::trace!("webhook signature mismatch");
        return false;
    }

    log::trace!("webhook signature verified");
    true
}

fn calculate_signature(base_str: &str, secret: &str) -> String {
    let mut mac =
        Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");
    mac.update(base_str.as_bytes());
    let result = mac.finalize().into_bytes();
    format!("v0={}", hex::encode(result))
}
