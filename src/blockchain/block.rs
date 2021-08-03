use super::record::Record;
extern crate blake2;
use blake2::{Blake2b, Digest};

/// A single part of the blockchain that contains a list of student records
#[derive(Clone, Debug)]
pub struct Block {
    /// The list of all the records in the block
    pub(crate) records: Vec<Record>,

    /// The hash that connects the blocks together
    pub prev_hash: Option<String>,

    /// Hash of the current block
    pub hash: Option<String>,
}

impl Block {
    pub fn new(prev_hash: Option<String>) -> Self {
        Block {
            hash: None,
            prev_hash,
            records: Vec::new(),
        }
    }

    /// Calculates the hash of the whole block using all the records
    /// of the block and the block itself
    pub fn calculate_hash(&self) -> Vec<u8> {
        let mut hasher = Blake2b::new();

        for record in self.records.iter() {
            hasher.update(record.calculate_hash())
        }

        let block_as_string = format!("{:?}", (&self.prev_hash));
        hasher.update(&block_as_string);

        return Vec::from(hasher.finalize().as_ref());
    }

    /// Appends a new record to the list
    pub fn add_record(&mut self, record: Record) {
        self.records.push(record);
        self.update_hash();
    }

    /// The amount of records in the block
    pub fn get_records_count(&self) -> usize {
        self.records.len()
    }

    /// Updates the current hash based on all the records and the block itself
    pub(crate) fn update_hash(&mut self) {
        self.hash = Some(byte_vector_to_string(&self.calculate_hash()));
    }

    /// Checks if the hash is set and equals the internal calculated hash of the block
    pub fn verify_own_hash(&self) -> bool {
        return self.hash.is_some() && 
                self.hash.as_ref().unwrap().eq(&byte_vector_to_string(&self.calculate_hash()));
    }
}

/// Takes an array of bytes and returns a string by taking every byte as a character
pub fn byte_vector_to_string(arr: &Vec<u8>) -> String {
    arr.iter().map(|&c| c as char).collect()
}
