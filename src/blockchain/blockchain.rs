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

        // Checsk if the new block is meant to be appended onto the last block
        if !(block.prev_hash == self.get_last_block_hash()) {
            return Err("The new block has to point to the previous block".into());
        }

        // There has to be at least one transaction inside the queue
        if block.get_transaction_count() == 0 {
            return Err("There has to be at least one transaction inside the block!".into());
        }

        // Rollback if some transactions succeed and others don't (prevent inconsistent states)
        let old_state = self.students.clone();

        // Executes each transaction
        for (i, transaction) in block.records.iter().enumerate() {
            // Execute the transaction
            if let Err(err) = transaction.execute(self) {
                // Recover state on failure
                self.students = old_state;

                // ... and reject the block
                return Err(format!(
                    "Could not execute transaction {} due to `{}`. Rolling back",
                    i + 1,
                    err
                ));
            }
        }

        // Everything went fine... append the block
        self.blocks.push(block);

        Ok(())
    }

    /// Returns the amount of blocks currently stored
    pub fn len(&self) -> usize {
        self.blocks.len()
    }

    /// Returns the hash of the last block
    pub fn get_last_block_hash(&self) -> Option<String> {
        if self.len() == 0 {
            return None;
        }

        self.blocks[self.len() - 1].hash.clone()
    }

    /// Returns the description of the error if the blockchain was tampered or OK in other case
    pub fn check_validity(&self) -> Result<(), String> {
        for (block_num, block) in self.blocks.iter().enumerate() {
            // Check if block saved hash matches to calculated hash
            if !block.verify_own_hash() {
                return Err(format!(
                    "Stored hash for Block #{} \
                    does not match calculated hash",
                    block_num + 1
                )
                .into());
            }

            // Check previous black hash points to actual previous block
            if block_num == 0 {
                // First block should point to nowhere
                if block.prev_hash.is_some() {
                    return Err("The first block has a previous hash set which \
                     it shouldn't"
                        .into());
                }
            } else {
                // Other blocks should point to previous blocks hash
                if block.prev_hash.is_none() {
                    return Err(format!("Block #{} has no previous hash set", block_num + 1).into());
                }

                // Store the values locally to use them within the error message on failure
                let prev_hash_proposed = block.prev_hash.as_ref().unwrap();
                let prev_hash_actual = self.blocks[block_num - 1].hash.as_ref().unwrap();

                if !(&block.prev_hash == &self.blocks[block_num - 1].hash) {
                    return Err(format!(
                        "Block #{} is not connected to previous block (Hashes do \
                    not match. Should be `{}` but is `{}`)",
                        block_num, prev_hash_proposed, prev_hash_actual
                    )
                    .into());
                }
            }
        }
        Ok(())
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
