//! WPA2 CCMP (AES-CCM) skeleton

#[derive(Debug)]
pub enum CcmpError { NotImplemented }

pub fn ccmp_encrypt(_ptk: &[u8], _pn: &[u8;6], _aad: &[u8], _plain: &[u8]) -> Result<alloc::vec::Vec<u8>, CcmpError> {
    Err(CcmpError::NotImplemented)
}

pub fn ccmp_decrypt(_ptk: &[u8], _pn: &[u8;6], _aad: &[u8], _cipher: &[u8]) -> Result<alloc::vec::Vec<u8>, CcmpError> {
    Err(CcmpError::NotImplemented)
}


