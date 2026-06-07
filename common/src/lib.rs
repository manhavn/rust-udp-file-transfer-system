/// Encodes a `u64` number into a sequence of bytes (each byte in 0..=254) using a greedy decimal grouping
/// so that when the decimal representations of the bytes are concatenated, they reform the original number.
pub fn encode_number_to_bytes(val: u64) -> Vec<u8> {
    let s = val.to_string();
    let mut bytes = Vec::new();
    let mut chars = s.as_bytes();

    while !chars.is_empty() {
        if chars[0] == b'0' {
            bytes.push(0);
            chars = &chars[1..];
        } else {
            let mut taken = false;
            // Try to take a 3-digit chunk
            if chars.len() >= 3 {
                if let Ok(chunk_str) = std::str::from_utf8(&chars[..3]) {
                    if let Ok(num) = chunk_str.parse::<u16>() {
                        if num <= 254 {
                            bytes.push(num as u8);
                            chars = &chars[3..];
                            taken = true;
                        }
                    }
                }
            }
            // Try to take a 2-digit chunk
            if !taken && chars.len() >= 2 {
                if let Ok(chunk_str) = std::str::from_utf8(&chars[..2]) {
                    if let Ok(num) = chunk_str.parse::<u8>() {
                        if num <= 254 {
                            bytes.push(num);
                            chars = &chars[2..];
                            taken = true;
                        }
                    }
                }
            }
            // Take a 1-digit chunk
            if !taken {
                let num = (chars[0] - b'0') as u8;
                bytes.push(num);
                chars = &chars[1..];
            }
        }
    }
    bytes
}

/// Decodes a sequence of bytes (each byte in 0..=254) back to a `u64`
/// by formatting each byte as a decimal string, concatenating them, and parsing.
pub fn decode_bytes_to_number(bytes: &[u8]) -> Result<u64, String> {
    let s: String = bytes.iter().map(|b| b.to_string()).collect();
    s.parse::<u64>().map_err(|e| format!("Failed to parse '{}' as u64: {}", s, e))
}

/// Formats a sequence of bytes (like the packet code) as a concatenated decimal string unique ID.
pub fn bytes_to_unique_id(bytes: &[u8]) -> String {
    bytes.iter().map(|b| b.to_string()).collect()
}

/// Generates the packet code bytes from a file hash.
/// It takes the first 10 bytes (or all available if less than 10) and maps them to `0..=254` using `byte % 255`.
pub fn generate_packet_code_from_hash(hash_bytes: &[u8]) -> Vec<u8> {
    let take_len = hash_bytes.len().min(10);
    hash_bytes[0..take_len]
        .iter()
        .map(|&b| b % 255)
        .collect()
}

/// Representation of a parsed UDP transmission packet.
#[derive(Debug)]
pub struct UdpPacket<'a> {
    pub status: u8, // 1 for sending, 0 for end
    pub packet_code: &'a [u8],
    pub seek_begin: u64,
    pub data: &'a [u8],
}

impl<'a> UdpPacket<'a> {
    /// Parses a raw byte buffer into a `UdpPacket`.
    pub fn parse(buf: &'a [u8]) -> Result<Self, String> {
        if buf.len() < 4 {
            return Err("Packet too short".to_string());
        }
        let status = buf[0];
        if buf[1] != 255 {
            return Err("Invalid status separator".to_string());
        }

        let mut idx = 2;
        // Find packet code separator
        while idx < buf.len() && buf[idx] != 255 {
            idx += 1;
        }
        if idx >= buf.len() {
            return Err("Missing packet code separator".to_string());
        }
        let packet_code = &buf[2..idx];

        idx += 1; // skip separator

        // Find seek begin separator
        let seek_begin_start = idx;
        while idx < buf.len() && buf[idx] != 255 {
            idx += 1;
        }
        if idx >= buf.len() {
            return Err("Missing seek begin separator".to_string());
        }
        let seek_begin_bytes = &buf[seek_begin_start..idx];
        let seek_begin = decode_bytes_to_number(seek_begin_bytes)?;

        idx += 1; // skip separator
        let data = &buf[idx..];

        Ok(UdpPacket {
            status,
            packet_code,
            seek_begin,
            data,
        })
    }

    /// Serializes a UDP packet into a byte vector.
    pub fn serialize(status: u8, packet_code: &[u8], seek_begin: u64, data: &[u8]) -> Vec<u8> {
        let mut pkt = Vec::new();
        pkt.push(status);
        pkt.push(255);
        pkt.extend_from_slice(packet_code);
        pkt.push(255);
        let seek_bytes = encode_number_to_bytes(seek_begin);
        pkt.extend_from_slice(&seek_bytes);
        pkt.push(255);
        pkt.extend_from_slice(data);
        pkt
    }
}

/// Representation of a parsed ACK packet sent back by the server.
#[derive(Debug)]
pub struct AckPacket<'a> {
    pub packet_code: &'a [u8],
    pub seek_begin: u64,
    pub bytes_received: u64,
}

impl<'a> AckPacket<'a> {
    /// Parses a raw byte buffer into an `AckPacket`.
    pub fn parse(buf: &'a [u8]) -> Result<Self, String> {
        let mut idx = 0;
        // Find packet code separator
        while idx < buf.len() && buf[idx] != 255 {
            idx += 1;
        }
        if idx >= buf.len() {
            return Err("Missing packet code separator in ACK".to_string());
        }
        let packet_code = &buf[..idx];

        idx += 1; // skip separator

        // Find seek begin separator
        let seek_begin_start = idx;
        while idx < buf.len() && buf[idx] != 255 {
            idx += 1;
        }
        if idx >= buf.len() {
            return Err("Missing seek begin separator in ACK".to_string());
        }
        let seek_begin_bytes = &buf[seek_begin_start..idx];
        let seek_begin = decode_bytes_to_number(seek_begin_bytes)?;

        idx += 1; // skip separator

        // Find bytes received separator
        let rec_bytes_start = idx;
        while idx < buf.len() && buf[idx] != 255 {
            idx += 1;
        }
        if idx >= buf.len() {
            return Err("Missing bytes received separator in ACK".to_string());
        }
        let rec_bytes_bytes = &buf[rec_bytes_start..idx];
        let bytes_received = decode_bytes_to_number(rec_bytes_bytes)?;

        Ok(AckPacket {
            packet_code,
            seek_begin,
            bytes_received,
        })
    }

    /// Serializes an ACK packet into a byte vector.
    pub fn serialize(packet_code: &[u8], seek_begin: u64, bytes_received: u64) -> Vec<u8> {
        let mut ack = Vec::new();
        ack.extend_from_slice(packet_code);
        ack.push(255);
        let seek_bytes = encode_number_to_bytes(seek_begin);
        ack.extend_from_slice(&seek_bytes);
        ack.push(255);
        let rec_bytes = encode_number_to_bytes(bytes_received);
        ack.extend_from_slice(&rec_bytes);
        ack.push(255);
        ack
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encoding_decoding() {
        let cases = vec![0, 16384, 1020085001163, 123456789, 999];
        for val in cases {
            let encoded = encode_number_to_bytes(val);
            assert!(encoded.iter().all(|&b| b <= 254));
            let decoded = decode_bytes_to_number(&encoded).unwrap();
            assert_eq!(val, decoded);
        }
    }

    #[test]
    fn test_packet_serialization() {
        let code = vec![125, 254, 65, 0, 110, 48];
        let data = vec![1, 2, 3, 4];
        let serialized = UdpPacket::serialize(1, &code, 16384, &data);
        
        let parsed = UdpPacket::parse(&serialized).unwrap();
        assert_eq!(parsed.status, 1);
        assert_eq!(parsed.packet_code, &code[..]);
        assert_eq!(parsed.seek_begin, 16384);
        assert_eq!(parsed.data, &data[..]);
    }
}
