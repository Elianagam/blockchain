use super::block::Block;
use super::record::Record;
use super::student::Student;
use std::collections::HashMap;
use std::convert::Into;
use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use std::vec::Vec;

/// The Blockchain container
#[derive(Debug, Clone)]
pub struct Blockchain {
    /// Blocks that are already in the blockchain
    pub blocks: Vec<Block>,

    /// The world state
    pub students: HashMap<String, Student>,

    /// Records that should be added to the chain but aren't yet
    pending_records: Vec<Record>,
}

impl Blockchain {
    /// Constructor
    pub fn new() -> Self {
        Blockchain {
            blocks: Vec::new(),
            students: HashMap::new(),
            pending_records: Vec::new(),
        }
    }

    /// Adds a block to the Blockchain
    pub fn append_block(&mut self, block: Block) -> Result<(), String> {
        // Checks if the hash matches the records
        if !block.verify_own_hash() {
            return Err("The block hash is mismatching!".into());
        }

        // Checks if the new block has its previous hash equal to the hash of the last 
        // block of the blockchain
        if !(block.prev_hash == self.get_last_block_hash()) {
            return Err("The new block has to point to the previous block".into());
        }

        // An empty block cannot be added
        if block.get_records_count() == 0 {
            return Err("There has to be at least one record inside the block".into());
        }

        // Rollback if some records where right and others not
        let old_state = self.students.clone();

        // Executes each record
        for (i, record) in block.records.iter().enumerate() {
            if let Err(err) = record.execute(self) {
                self.students = old_state;
                return Err(format!(
                    "Could not execute record {} due to `{}`. Rolling back",
                    i + 1,
                    err
                ));
            }
        }
        self.blocks.push(block);
        Ok(())
    }

    /// Returns the amount of blocks currently stored
    pub fn len(&self) -> usize {
        self.blocks.len()
    }

    // Returns the block in the blockchain
    pub fn get_blocks(&self) -> Vec<Block> {
        self.blocks.clone()
    }

    /// Returns the hash of the last block
    pub fn get_last_block_hash(&self) -> Option<String> {
        if self.len() == 0 {
            return None;
        }

        self.blocks[self.len() - 1].hash.clone()
    }
}

impl Display for Blockchain {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut data = format!("Blockchain:\n\tPadron\tNota");
        for (padron, q) in &self.students {
            data = format!("{}\n\t{}\t{}", data, padron, q.qualification);
        }
        write!(f, "{}", data)
    }
}
