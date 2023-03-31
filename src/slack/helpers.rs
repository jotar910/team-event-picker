use chrono::{DateTime, FixedOffset, Local, NaiveDateTime, Timelike};
use handlebars::Handlebars;
use hyper::{Body, HeaderMap, Request};
use hyper_tls::HttpsConnector;
use serde_json::json;

use crate::domain::timezone::Timezone;

pub fn render_template(
    template: &str,
    context: serde_json::Value,
) -> Result<String, Box<dyn std::error::Error>> {
    let reg = Handlebars::new();
    Ok(reg.render_template(&template, &context)?)
}

pub async fn send_post(
    url: &str,
    body: hyper::Body,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    send_post_with_type(url, body, String::from("application/json")).await
}

pub async fn send_authorized_post(
    url: &str,
    token: &str,
    body: hyper::Body,
) -> Result<(), Box<dyn std::error::Error>> {
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

    log::trace!(
        "authorized response received from request to {}: {}: {:?}",
        url,
        res_str,
        body
    );

    Ok(())
}

pub async fn send_post_with_type(
    url: &str,
    body: hyper::Body,
    content_type: String,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let https = HttpsConnector::new();
    let client = hyper::Client::builder().build(https);

    let req = Request::builder()
        .method(hyper::Method::POST)
        .uri(url)
        .header("Content-Type", content_type)
        .body(body)?;

    log::trace!("sending action response to {}: {:?}", url, &req);

    let response = client.request(req).await?;
    let (parts, body) = response.into_parts();
    let body = response_to_string(body).await?;

    log::trace!(
        "response received from request to {}: {:?}: {}",
        url,
        parts,
        body
    );

    Ok(body)
}

pub fn find_token(headers: &HeaderMap) -> Result<String, hyper::StatusCode> {
    let token = headers
        .get("x-access-token")
        .ok_or_else(|| {
            log::trace!("access token not provided on action handler");
            hyper::StatusCode::UNAUTHORIZED
        })?
        .to_str()
        .map_err(|err| {
            log::trace!("provided invalid access token on action handler: {}", err);
            hyper::StatusCode::UNAUTHORIZED
        })?
        .to_string();
    Ok(token)
}

pub fn find_reached_limit(headers: &HeaderMap) -> Result<bool, hyper::StatusCode> {
    let reached_limit: bool = headers
        .get("x-reached-limit")
        .ok_or_else(|| {
            log::trace!("reached limit state not provided on action handler");
            hyper::StatusCode::BAD_REQUEST
        })?
        .to_str()
        .map_err(|err| {
            log::trace!(
                "provided invalid reached limit state on action handler: {}",
                err
            );
            hyper::StatusCode::BAD_REQUEST
        })?
        .parse()
        .map_err(|err| {
            log::trace!("reached limit state is not a bool: {}", err);
            hyper::StatusCode::BAD_REQUEST
        })?;
    Ok(reached_limit)
}

pub fn to_response(value: &str) -> Result<String, hyper::StatusCode> {
    Ok(json!({ "text": value }).to_string())
}

pub fn to_response_error(value: &str) -> Result<String, hyper::StatusCode> {
    Ok(json!({ "text": value, "response_type": "ephemeral" }).to_string())
}

pub fn fmt_timestamp(timestamp: i64, timezone: Timezone) -> String {
    DateTime::<Local>::from_local(
        NaiveDateTime::from_timestamp_opt(timestamp, 0)
            .unwrap_or(NaiveDateTime::default())
            .with_second(0)
            .unwrap(),
        FixedOffset::east_opt(Timezone::from(timezone).into()).unwrap(),
    )
    .to_string()
}

async fn response_to_string(res: Body) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let body_bytes = hyper::body::to_bytes(res).await?;
    let body_string = String::from_utf8(body_bytes.to_vec())?;
    Ok(body_string)
}
