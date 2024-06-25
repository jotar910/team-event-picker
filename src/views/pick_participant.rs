use serde_json::Value;
use slack_blocks::{
    blocks::{Actions, Section},
    elems::{button::Style, Button},
    text,
};

use super::entities::{BlockGroup, Response};

pub struct PickParticipantView {
    pub event_id: u32,
    pub event_name: String,
    pub user_id: String,
    pub user_picked_id: String,
    pub left_count: usize,
    pub source: PickParticipantSource,
}

pub enum PickParticipantSource {
    Pick,
    Repick,
    Scheduler,
    Skip,
}

pub struct PickParticipantResult {
    pub name: String,
}

pub fn view(data: PickParticipantView) -> Value {
    let blocks = BlockGroup::empty()
        .add(
            Section::builder()
                .text(text::Mrkdwn::from_text(
                    match data.source {
                       PickParticipantSource::Pick =>
                         format!(
                            "<@{}> randomly picked <@{}> for the event *{}* ({} left)\n\t\t_Source: Manual Pick_",
                             data.user_id, data.user_picked_id, data.event_name, data.left_count
                            ),
                       PickParticipantSource::Repick =>
                         format!(
                            "<@{}> repicked <@{}> for the event *{}* ({} left)\n\t\t_Source: Repick_",
                             data.user_id, data.user_picked_id, data.event_name, data.left_count
                            ),
                       PickParticipantSource::Scheduler =>
                         format!(
                            "<@{}> randomly picked <@{}> for the event *{}* ({} left)\n\t\t_Source: Automatic scheduler_",
                             data.user_id, data.user_picked_id, data.event_name, data.left_count
                            ),
                       PickParticipantSource::Skip =>
                         format!(
                            "<@{}> skipped previous pick! <@a{}> is the new random pick for the event *{}* ({} left)\n\t\t_Source: Manual Skip_",
                             data.user_id, data.user_picked_id, data.event_name, data.left_count
                            ),
                    }
                ))
                .build()
                .into(),
        )
        .add(
            Actions::builder()
                .element(
                    Button::builder()
                        .text("Skip")
                        .action_id("pick")
                        .value(data.event_id.to_string())
                        .build(),
                )
                .element(
                    Button::builder()
                        .text(text::Plain::from_text("Repick"))
                        .action_id("repick")
                        .value(data.event_id.to_string())
                        .build(),
                )
                .element(
                    Button::builder()
                        .text(text::Plain::from_text("Cancel"))
                        .action_id("cancel")
                        .value(data.event_id.to_string())
                        .style(Style::Danger)
                        .build(),
                )
                .build()
                .into(),
        );

    return serde_json::to_value(Response::in_channel(blocks)).expect("should serialize");
}
