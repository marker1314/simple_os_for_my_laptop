//! TLS configuration flags

/// If true, TLS handshake will abort on certificate verification failure.
pub const STRICT_CERT_VERIFY: bool = true;

/// If true, certificate chain validation must succeed (issuer/subject, key usage, etc.).
pub const STRICT_CHAIN: bool = true;

/// If true, CRL must be provided and validated; otherwise fail verification.
pub const REQUIRE_CRL: bool = false;


