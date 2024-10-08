use crate::domain::events::pick_auto_participants;
use crate::views::pick_participant;

use super::helpers;

pub async fn post_picks(picks: Vec<pick_auto_participants::Pick>) {
    for pick in picks.into_iter() {
        let body = pick_participant::view(pick_participant::PickParticipantView {
            source: pick_participant::PickParticipantSource::Scheduler,
            event_id: pick.event_id,
            event_name: pick.event_name,
            channel_id: pick.channel_id,
            user_id: dotenv::var("BOT_NAME").unwrap_or(String::from("Team Picker")),
            user_picked_id: pick.user_id,
            left_count: pick.left_count,
        })
        .to_string();
        helpers::send_authorized_post(
            "https://slack.com/api/chat.postMessage",
            &pick.access_token,
            hyper::Body::from(body),
        )
        .await
        .unwrap_or_else(|err| {
            log::error!("failed to notify pick results: {}", err);
        });
    }
}
