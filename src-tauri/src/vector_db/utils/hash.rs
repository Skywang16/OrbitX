pub fn blake3_hash_str(s: &str) -> String {
    blake3::hash(s.as_bytes()).to_hex().to_string()
}

pub fn blake3_hash_bytes(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}
