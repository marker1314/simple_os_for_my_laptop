//! Minimal PEM decoder for certificates

use alloc::vec::Vec;

fn b64_val(b: u8) -> Option<u8> {
    match b {
        b'A'..=b'Z' => Some(b - b'A'),
        b'a'..=b'z' => Some(b - b'a' + 26),
        b'0'..=b'9' => Some(b - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

/// Decode a PEM-encoded certificate (returns DER bytes)
pub fn decode_pem_cert(pem: &[u8]) -> Option<Vec<u8>> {
    // Find header/footer
    let s = pem;
    let head = b"-----BEGIN CERTIFICATE-----";
    let tail = b"-----END CERTIFICATE-----";
    // naive subslice search to avoid external deps
    fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
        if needle.is_empty() { return Some(0); }
        for i in 0..=hay.len().saturating_sub(needle.len()) {
            if &hay[i..i+needle.len()] == needle { return Some(i); }
        }
        None
    }
    let start = find(s, head)? + head.len();
    let end_rel = find(&s[start..], tail)?;
    let end = start + end_rel;
    let body = &s[start..end];

    // Strip whitespace
    let mut buf = Vec::with_capacity(body.len());
    for &c in body {
        if !(c == b'\r' || c == b'\n' || c == b' ' || c == b'\t') {
            buf.push(c);
        }
    }

    // Base64 decode
    let mut out = Vec::with_capacity(buf.len() * 3 / 4);
    let mut i = 0;
    while i + 4 <= buf.len() {
        let a = buf[i];
        let b = buf[i + 1];
        let c = buf[i + 2];
        let d = buf[i + 3];
        i += 4;

        if d == b'=' {
            if c == b'=' {
                let v0 = b64_val(a)? as u32;
                let v1 = b64_val(b)? as u32;
                let n = (v0 << 18) | (v1 << 12);
                out.push(((n >> 16) & 0xFF) as u8);
                break;
            } else {
                let v0 = b64_val(a)? as u32;
                let v1 = b64_val(b)? as u32;
                let v2 = b64_val(c)? as u32;
                let n = (v0 << 18) | (v1 << 12) | (v2 << 6);
                out.push(((n >> 16) & 0xFF) as u8);
                out.push(((n >> 8) & 0xFF) as u8);
                break;
            }
        } else {
            let v0 = b64_val(a)? as u32;
            let v1 = b64_val(b)? as u32;
            let v2 = b64_val(c)? as u32;
            let v3 = b64_val(d)? as u32;
            let n = (v0 << 18) | (v1 << 12) | (v2 << 6) | v3;
            out.push(((n >> 16) & 0xFF) as u8);
            out.push(((n >> 8) & 0xFF) as u8);
            out.push((n & 0xFF) as u8);
        }
    }

    Some(out)
}


