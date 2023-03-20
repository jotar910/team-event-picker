use serde_json::json;

use crate::domain::pick_auto_participants;

use super::{helpers, templates};

pub async fn join_channel(token: &str, channel: &str) -> Result<(), Box<dyn std::error::Error>> {
    let body = json!({ "channel": channel }).to_string();
    helpers::send_authorized_post(
        "https://slack.com/api/conversations.join",
        &token,
        hyper::Body::from(body),
    )
    .await
}

pub async fn post_picks(picks: Vec<pick_auto_participants::Pick>) {
    for pick in picks.into_iter() {
        let body = templates::pick_auto(
            pick.channel_url.clone(),
            pick.event_id,
            pick.event_name.clone(),
            pick.user_name.clone(),
        )
        .unwrap_or(
            json!({
                "channel": pick.channel_url,
                "text": format!("Picked <@{}> for {}", pick.user_name, pick.event_name),
            })
            .to_string(),
        );
        match helpers::send_authorized_post(
            "https://slack.com/api/chat.postMessage",
            &pick.access_token,
            hyper::Body::from(body),
        )
        .await
        {
            Ok(res) => res,
            Err(err) => {
                log::error!("failed to notify pick results: {}", err);
            }
        };
    }
}
