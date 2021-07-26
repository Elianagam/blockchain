use std::str;

const MSG_EOF: char = '\n';

pub fn encode_to_bytes(msg: &str) -> Vec<u8> {
    let mut message = String::from(msg.clone());
    message.push(MSG_EOF);
    message.into_bytes()
}

pub fn decode_from_bytes(payload: Vec<u8>) -> String {
    let data = str::from_utf8(&payload)
        .unwrap()
        .split(MSG_EOF)
        .collect::<Vec<&str>>()[0];
    data.to_string()
}
