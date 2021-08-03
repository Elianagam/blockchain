use super::blockchain::Blockchain;
use super::student::Student;
/// The current state of the blockchain after all Blocks are added
/// Interface into the Blockchain
pub trait WorldState {
    /// Returns all registered student ids
    fn get_student_ids(&self) -> Vec<String>;

    /// Returns a student given the id if it's available (mutable)
    fn get_student_by_id_mut(&mut self, id: &String) -> Option<&mut Student>;

    /// Returns a student given the id if it's available
    fn get_student_by_id(&self, id: &String) -> Option<&Student>;

    /// Adds a new student
    fn create_student(&mut self, id: String, qualification: u32) -> Result<(), &'static str>;
}

impl WorldState for Blockchain {
    fn get_student_ids(&self) -> Vec<String> {
        self.students.keys().map(|s| s.clone()).collect()
    }

    fn get_student_by_id_mut(&mut self, id: &String) -> Option<&mut Student> {
        self.students.get_mut(id)
    }

    fn get_student_by_id(&self, id: &String) -> Option<&Student> {
        self.students.get(id)
    }

    fn create_student(&mut self, id: String, qualification: u32) -> Result<(), &'static str> {
        if qualification >= 1 && qualification <= 10 {
            let acc = Student::new(qualification);
            self.students.insert(id, acc);
            return Ok(());
        } else {
            return Err("Qualification must be between 1 and 10");
        }
    }
}
