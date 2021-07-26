extern crate blake2;
use blake2::{Blake2b, Digest};

use crate::Record;
use serde::{Serialize, Deserialize};

/// One single part of the blockchain that contains a list of records
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Block {
    /// Actions that this block includes (at least one)
    pub(crate) records: Vec<Record>,

    /// Connects the blocks together
    pub prev_hash: Option<String>,

    /// Hash of the current block (in order to save the last block from being tampered)
    pub hash: Option<String>
}

impl Block {
    pub fn new(prev_hash: Option<String>) -> Self {
        Block {
            hash: None,
            prev_hash,
            records: Vec::new(),
        }
    }

    /// Will calculate the hash of the whole block including records Blake2 hasher
    pub fn calculate_hash(&self) -> Vec<u8> {
        let mut hasher = Blake2b::new();

        for record in self.records.iter() {
            hasher.update(record.calculate_hash())
        }

        let block_as_string = format!("{:?}", (&self.prev_hash));
        hasher.update(&block_as_string);

        return Vec::from(hasher.finalize().as_ref());
    }

    /// Appends a record to the queue
    pub fn add_record(&mut self, record: Record) {
        self.records.push(record);
        self.update_hash();
    }

    /// Will return the amount of transactions
    pub fn get_transaction_count(&self) -> usize {
        self.records.len()
    }

    /// Will update the hash field by including all transactions currently inside
    /// the public modifier is only for the demonstration of attacks
    pub(crate) fn update_hash(&mut self) {
        self.hash = Some(byte_vector_to_string(&self.calculate_hash()));
    }

    /// Checks if the hash is set and matches the blocks interna
    pub fn verify_own_hash(&self) -> bool {
        if self.hash.is_some() && // Hash set
            self.hash.as_ref().unwrap().eq(
                &byte_vector_to_string(
                    &self.calculate_hash())) { // Hash equals calculated hash

            return true;
        }
        false
    }
}

/// Will take an array of bytes and transform it into a string by interpreting every byte
/// as an character
pub fn byte_vector_to_string(arr: &Vec<u8>) -> String {
    arr.iter().map(|&c| c as char).collect()
}
