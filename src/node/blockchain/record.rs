use std::time::SystemTime;
mod world_state;
use world_state::WorldState;
use blake2::{Blake2b, Digest};
use serde::{Serialize, Deserialize};


/// Request to the blockchain
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Record {
    /// Client ID
    pub from: String,

    /// The time the record was created
    pub created_at: SystemTime,

    /// The type of the record and its additional information
    pub(crate) record: RecordData,
}

/// The operation to be stored on the chain
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum RecordData {

    CreateStudent(String, u32),

    UpdateQualification { student: String, qualification: u32 },
}

impl Record {
    pub fn new(from: String, record_data: RecordData, time: SystemTime) -> Self {
        Record {
            from,
            record: record_data,
            created_at: time //SystemTime::now()
        }
    }

    /// Will change the world state according to the transactions commands
    pub fn execute(&self, world_state: &mut dyn WorldState) -> Result<(), &'static str> {

        // match is like a switch (pattern matching) in C++ or Java
        // We will check for the type of transaction here and execute its logic
        return match &self.record {

            RecordData::CreateStudent(id, qualification) => {
                world_state.create_student(id.into(), *qualification)
            }

            RecordData::UpdateQualification { student, qualification } => {
                // Get the student (must exist)
                return if let Some(student) = world_state.get_student_by_id_mut(student) {
                    student.qualification = *qualification;
                    Ok(())
                } else {
                    Err("Student does not exist")
                };
            }
        };
    }

    /// Will calculate the hash using Blake2 hasher
    pub fn calculate_hash(&self) -> Vec<u8> {
        let mut hasher = Blake2b::new();
        let record_as_string = format!("{:?}", (&self.created_at, &self.record,
                                                     &self.from));

        hasher.update(&record_as_string);
        return Vec::from(hasher.finalize().as_ref());
    }
}