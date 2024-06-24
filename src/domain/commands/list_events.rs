use std::sync::Arc;

use crate::{
    domain::events::find_all_events,
    repository::event::Repository,
    slack::{helpers, templates::Error},
};

use slack_blocks::{
    blocks::{Actions, Header, Section},
    compose::Opt,
    elems::{button::Style, Button},
    text,
};

use super::entities::BlockGroup;

pub async fn execute(
    repo: Arc<dyn Repository>,
    channel: String,
    reached_limit: bool,
) -> Result<serde_json::Value, Error> {
    let events = find_all_events::execute(repo, find_all_events::Request { channel })
        .await?
        .data;
    let mut blocks = BlockGroup::empty()
        .add(
            Header::builder()
                .text("Checkout your events!")
                .build()
                .into(),
        )
        .add(
            Section::builder()
                .text(text::Mrkdwn::from_text(
                    "Here, you can manage all of your events with ease.",
                ))
                .build()
                .into(),
        );
    for event in events {
        blocks = blocks.add(
            Section::builder()
                .text(text::Mrkdwn::from_text(format!(
                    "[{}]: *{}*",
                    event.id, event.name
                )))
                .fields(vec![
                    text::Plain::from_text(helpers::fmt_timestamp(event.timestamp, event.timezone))
                        .into(),
                    text::Plain::from_text(event.repeat.to_string()).into(),
                ])
                .accessory(
                    slack_blocks::elems::overflow::Overflow::builder()
                        .options(vec![
                            Opt::builder()
                                .text(text::Plain::from_text("Pick randomly"))
                                .value("pick")
                                .build(),
                            Opt::builder()
                                .text(text::Plain::from_text("Show details"))
                                .value("show")
                                .build(),
                            Opt::builder()
                                .text(text::Plain::from_text("Edit event"))
                                .value("edit")
                                .build(),
                            Opt::builder()
                                .text(text::Plain::from_text("Delete event"))
                                .value("delete")
                                .build(),
                        ])
                        .action_id("list_event_actions")
                        .build(),
                )
                .block_id(event.id.to_string())
                .build()
                .into(),
        );
    }
    let close_action = Button::builder()
        .text("Close")
        .value("close")
        .action_id("close")
        .build();
    if !reached_limit {
        blocks = blocks.add(
            Actions::builder()
                .element(
                    Button::builder()
                        .text("Create a new event")
                        .value("add_event")
                        .action_id("add_event")
                        .style(Style::Primary)
                        .build(),
                )
                .element(close_action)
                .block_id("list_events_actions")
                .build()
                .into(),
        );
    } else {
        blocks = blocks.add(
            Actions::builder()
                .element(close_action)
                .block_id("list_events_actions")
                .build()
                .into(),
        );
    }
    let response = serde_json::to_value(blocks).expect("should serialize");
    return Ok(response);
}
