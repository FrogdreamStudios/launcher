//! Simple hex encoding.

/// Converts a byte slice to a lowercase hexadecimal string.
pub fn encode<T: AsRef<[u8]>>(data: T) -> String {
    let data = data.as_ref();
    let mut result = String::with_capacity(data.len() * 2);

    for &byte in data {
        result.push(char::from_digit((byte >> 4) as u32, 16).unwrap());
        result.push(char::from_digit((byte & 0xf) as u32, 16).unwrap());
    }

    result
}
