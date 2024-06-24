use serde::Serialize;

#[derive(Serialize)]
pub struct BlockGroup<'a> {
    blocks: Vec<slack_blocks::Block<'a>>,
}

impl<'a> BlockGroup<'a> {
    pub fn new(blocks: Vec<slack_blocks::Block<'a>>) -> Self {
        return Self { blocks };
    }

    pub fn empty() -> Self {
        return Self { blocks: vec![] };
    }

    pub fn add(self: Self, block: slack_blocks::Block<'a>) -> Self {
        let mut blocks = self.blocks;
        blocks.push(block);
        return Self { blocks };
    }
}
