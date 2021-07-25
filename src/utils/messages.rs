#![allow(dead_code)]

pub const REGISTER_MSG: &str = "register";
pub const NEW_NODE: &str = "new_node";
pub const END: &str = "-";
pub const ACQUIRE_MSG: &str = "acquire\n";
pub const RELEASE_MSG: &str = "release\n";
pub const DISCONNECT_MSG: &str = "";
pub const DISCOVER_MSG: &str = "discover\n";
pub const BLOCKCHAIN: &str = "blockchain";
pub const CLOSE: &str = "close";
pub const BLOCKCHAIN_MSG: &str = "blockchain";

// Mensaje devuelto por el lider cuando esta ok el recibo del dato
pub const ACK_MSG: &str = "ack";
