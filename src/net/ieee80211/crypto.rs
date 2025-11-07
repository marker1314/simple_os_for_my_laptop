//! WPA2-PSK key derivation (PBKDF2-HMAC-SHA1)

#[derive(Debug)]
pub enum CryptoError { Invalid }

fn hmac_sha1(key: &[u8], data: &[u8]) -> [u8; 20] {
    use crate::net::tls::sha::Sha1Context;
    let mut k = [0u8; 64];
    if key.len() > 64 {
        let hash = Sha1Context::hash(key);
        k[..20].copy_from_slice(&hash);
    } else {
        k[..key.len()].copy_from_slice(key);
    }
    let mut ipad = [0x36u8; 64];
    let mut opad = [0x5cu8; 64];
    for i in 0..64 { ipad[i] ^= k[i]; opad[i] ^= k[i]; }
    let mut inner = Sha1Context::new();
    inner.update(&ipad);
    inner.update(data);
    let ihash = inner.finalize();
    let mut outer = Sha1Context::new();
    outer.update(&opad);
    outer.update(&ihash);
    outer.finalize()
}

/// PBKDF2-HMAC-SHA1(passphrase, ssid, 4096, 32)
pub fn derive_psk(passphrase: &str, ssid: &str) -> Result<[u8; 32], CryptoError> {
    let salt = ssid.as_bytes();
    let pw = passphrase.as_bytes();
    if pw.is_empty() || salt.is_empty() { return Err(CryptoError::Invalid); }
    const ITER: u32 = 4096;
    let mut out = [0u8; 32];
    // Two blocks (SHA1 output 20 bytes → need 32 bytes)
    for block_index in 1..=2 {
        // U1 = HMAC(pw, salt || INT(block_index))
        let mut msg = alloc::vec::Vec::with_capacity(salt.len() + 4);
        msg.extend_from_slice(salt);
        msg.extend_from_slice(&(block_index as u32).to_be_bytes());
        let mut u = hmac_sha1(pw, &msg);
        let mut t = u;
        for _ in 2..=ITER {
            u = hmac_sha1(pw, &u);
            for i in 0..20 { t[i] ^= u[i]; }
        }
        let start = (block_index - 1) * 20;
        let end = core::cmp::min(start + 20, 32);
        out[start as usize..end as usize].copy_from_slice(&t[..(end - start) as usize]);
    }
    Ok(out)
}

/// PTK derivation placeholder (WPA2 PRF)
pub fn derive_ptk(_psk: &[u8; 32], _anonce: &[u8; 32], _snonce: &[u8; 32], _aa: &[u8; 6], _sa: &[u8; 6]) -> Result<[u8; 64], CryptoError> {
    Ok([0u8; 64])
}


