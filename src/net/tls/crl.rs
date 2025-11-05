//! Minimal X.509 CRL parser (structure-only, best-effort)

use alloc::vec::Vec;

use crate::net::tls::certificate::{Asn1Parser, TlsCertificateError};

#[derive(Debug)]
pub struct ParsedCrl {
    pub issuer: Option<Vec<u8>>, // raw DN bytes (simplified)
    pub revoked_serials: Vec<Vec<u8>>, // list of serial numbers
}

/// Parse a minimal CRL: CertificateList ::= SEQUENCE {...}
pub fn parse_crl(der: &[u8]) -> Result<ParsedCrl, TlsCertificateError> {
    let mut p = Asn1Parser::new(der);
    let crl_seq = p.read_sequence()?;
    let mut cp = Asn1Parser::new(crl_seq);

    // tbsCertList
    let tbs = cp.read_sequence()?;
    let mut tp = Asn1Parser::new(tbs);

    // version (optional) -> skip if not Integer
    let pos_save = tp.position();
    let _version = if tp.read_integer().is_ok() { Some(()) } else { tp.set_position(pos_save); None };

    // signature (AlgorithmIdentifier) -> skip
    let _sig_alg = tp.read_sequence().ok();

    // issuer (Name) -> read raw bytes of sequence
    let issuer_seq = tp.read_sequence().ok();
    let issuer_raw = issuer_seq.map(|s| s.to_vec());

    // thisUpdate, nextUpdate (skip time parsing)
    let _this_update = tp.read_utc_time().or_else(|_| tp.read_utf8_string()).ok();
    let _next_update = tp.read_utc_time().or_else(|_| tp.read_utf8_string()).ok();

    // revokedCertificates (SEQUENCE OF)
    let mut revoked_serials = Vec::new();
    if let Ok(revoked_seq) = tp.read_sequence() {
        let mut rp = Asn1Parser::new(revoked_seq);
        // iterate entries: SEQUENCE { userCertificate INTEGER, ... }
        while rp.position() < revoked_seq.len() {
            if let Ok(entry) = rp.read_sequence() {
                let mut ep = Asn1Parser::new(entry);
                if let Ok(serial) = ep.read_integer() {
                    revoked_serials.push(serial);
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }

    Ok(ParsedCrl { issuer: issuer_raw, revoked_serials })
}

/// Check if a serial number (DER INTEGER bytes) is listed as revoked.
pub fn is_serial_revoked(crl: &ParsedCrl, serial_der: &[u8]) -> bool {
    crl.revoked_serials.iter().any(|s| s == serial_der)
}


