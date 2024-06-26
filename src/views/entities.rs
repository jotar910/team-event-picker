use serde::Serialize;

#[derive(Serialize)]
pub struct BlockGroup<'a> {
    blocks: Vec<slack_blocks::Block<'a>>,
    replace_original: bool,
    #[serde(skip_serializing_if = "Option::is_none", rename = "channel")]
    channel_id: Option<String>,
}

impl<'a> BlockGroup<'a> {
    pub fn empty() -> Self {
        return Self {
            blocks: vec![],
            replace_original: true,
            channel_id: None,
        };
    }

    pub fn add(mut self: Self, block: slack_blocks::Block<'a>) -> Self {
        self.blocks.push(block);
        return self;
    }

    pub fn channel(mut self: Self, channel_id: String) -> Self {
        self.channel_id = Some(channel_id);
        return self;
    }
}

#[derive(Serialize)]
pub struct Response<'a> {
    #[serde(flatten)]
    data: BlockGroup<'a>,
    delete_original: bool,
    response_type: &'a str,
}

impl<'a> Response<'a> {
    pub fn in_channel(data: BlockGroup<'a>) -> Self {
        return Self {
            data,
            delete_original: true,
            response_type: "in_channel",
        };
    }

    pub fn ephemeral(data: BlockGroup<'a>) -> Self {
        return Self {
            data,
            delete_original: true,
            response_type: "ephemeral",
        };
    }
}
