/// Represents a student on the blockchain
/// Is the primary part of the "world state" of the blockchain
/// (the final status after performing all blocks in order)
#[derive(Clone, Debug)]
pub struct Student {
    /// qualification
    pub qualification: i32,
}

impl Student {
    /// Constructor
    pub fn new(qualification: i32) -> Self {
        Student { qualification }
    }
}
