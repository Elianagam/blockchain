use std::fmt::Debug;
use std::fmt::Display;

#[derive(Clone)]
pub struct Block {
    pub data: String,
}

#[derive(Debug, Clone)]
pub struct Blockchain {
    blocks: Vec<Block>,
}
impl Blockchain {
    pub fn new() -> Self {
        Self { blocks: vec![] }
    }

    pub fn add(&mut self, new_block: Block) {
        self.blocks.push(new_block);
    }

    pub fn get_blocks(&self) -> Vec<Block> {
        self.blocks.clone()
    }
}

impl Display for Blockchain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut bc_fmt = String::new();
        for b in self.blocks.clone() {
            bc_fmt = format!("{}\n\t{}", bc_fmt, b);
        }
        write!(f, "Blockchain:{}", bc_fmt)
    }
}

impl Debug for Block {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.data.to_string())
    }
}

impl Display for Block {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{data: {:?}}}", self.data.to_string())
    }
}
