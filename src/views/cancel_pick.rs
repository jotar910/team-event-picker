use serde_json::Value;
use slack_blocks::blocks::Section;
use slack_blocks::elems::Button;
use slack_blocks::text;

use super::entities::{BlockGroup, Response};

pub struct CancelPickView {
    pub channel_id: String,
    pub user_id: String,
    pub event_id: u32,
    pub event_name: String,
}

pub fn view(data: CancelPickView) -> Value {
    let blocks = BlockGroup::empty().channel(data.channel_id).add(
        Section::builder()
            .text(text::Mrkdwn::from_text(format!(
                "<@{}> cancelled previous pick for the event *{}*\n\t\t_Source: Cancel_",
                data.user_id, data.event_name
            )))
            .accessory(
                Button::builder()
                    .text("Pick again")
                    .action_id("cancel_pick_actions:pick")
                    .value(data.event_id.to_string())
                    .build(),
            )
            .build()
            .into(),
    );
    return serde_json::to_value(Response::in_channel(blocks)).expect("should serialize");
}
