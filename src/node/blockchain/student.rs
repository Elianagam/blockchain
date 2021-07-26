/// Represents a student on the blockchain
/// Is the primary part of the "world state" of the blockchain
/// (the final status after performing all blocks in order)
#[derive(Clone, Debug)]
pub struct Student {
    /// qualification
    pub qualification: u32
}

impl Student {
    /// Constructor
    pub fn new(qualification: u32) -> Self {
        Student {
            qualification
        }
    }
}