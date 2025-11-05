//! Common OID constants and helpers

/// Return a readable name for a given DER-encoded OID bytes.
pub fn oid_name(oid: &[u8]) -> Option<&'static str> {
    // Very small mapping; extend as needed.
    // OIDs are compared by DER content bytes.
    match oid {
        // sha1: 1.3.14.3.2.26
        b"\x06\x05+\x0e\x03\x02\x1a" => Some("sha1"),
        // sha256: 2.16.840.1.101.3.4.2.1
        b"\x06\t`\x86H\x01e\x03\x04\x02\x01" => Some("sha256"),
        // rsaEncryption: 1.2.840.113549.1.1.1
        b"\x06\t*\x86H\x86\xf7\r\x01\x01\x01" => Some("rsaEncryption"),
        // sha256WithRSAEncryption: 1.2.840.113549.1.1.11
        b"\x06\t*\x86H\x86\xf7\r\x01\x01\x0b" => Some("sha256WithRSA"),
        // sha1WithRSAEncryption: 1.2.840.113549.1.1.5
        b"\x06\t*\x86H\x86\xf7\r\x01\x01\x05" => Some("sha1WithRSA"),
        _ => None,
    }
}


