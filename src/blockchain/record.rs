use super::world_state::WorldState;
use blake2::{Blake2b, Digest};
use std::time::Duration;

/// Request to the blockchain
#[derive(Clone, Debug)]
pub struct Record {
    /// Client ID
    pub from: String,

    /// The duration since EPOCH (when it was created)
    pub created_at: Duration,

    /// The type of the record and its additional information
    pub(crate) record: RecordData,
}

/// The operation to be stored on the blockchain
#[derive(Clone, Debug, PartialEq)]
pub enum RecordData {
    CreateStudent(String, u32)
}

impl Record {
    pub fn new(from: String, record_data: RecordData, time: Duration) -> Self {
        Record {
            from,
            record: record_data,
            created_at: time,
        }
    }

    /// Will change the world state
    pub fn execute(&self, world_state: &mut dyn WorldState) -> Result<(), &'static str> {
        return match &self.record {
            RecordData::CreateStudent(id, qualification) => {
                world_state.create_student(id.into(), *qualification)
            }
        };
    }

    /// Calculates the hash using Blake2 hasher
    pub fn calculate_hash(&self) -> Vec<u8> {
        let mut hasher = Blake2b::new();
        let record_as_string = format!("{:?}", (&self.created_at, &self.record, &self.from));

        hasher.update(&record_as_string);
        return Vec::from(hasher.finalize().as_ref());
    }
}
