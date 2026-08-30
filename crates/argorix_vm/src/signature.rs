//! Detached signatures over an evidence bundle.
//!
//! Digest verification establishes that a bundle and the artifacts it names
//! agree with each other. It cannot establish who produced them: replacing the
//! bundle together with every artifact by a self-consistent set is
//! indistinguishable from the original. A signature over the bundle's
//! canonical bytes closes that gap, because the bundle transitively covers
//! bytecode, trace, report and source by digest.
//!
//! Only verification lives here, and it needs a public key. Signing lives in
//! the separate `argorix-sign` binary so the runtime never handles private key
//! material.

use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

pub const SIGNATURE_VERSION: &str = "1.0";
pub const SIGNATURE_ALGORITHM: &str = "ed25519";

/// A detached signature, stored beside the bundle it covers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceSignature {
    pub signature_version: String,
    pub algorithm: String,
    /// Hex-encoded Ed25519 public key of the producer.
    pub public_key: String,
    /// Canonical digest of the bundle the signature covers, for diagnostics.
    /// It is never trusted in place of verifying the signature itself.
    pub bundle_digest: String,
    /// Hex-encoded Ed25519 signature over the bundle's canonical bytes.
    pub signature: String,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SignatureError {
    #[error("unsupported signature_version `{0}`")]
    UnsupportedVersion(String),
    #[error("unsupported signature algorithm `{0}`")]
    UnsupportedAlgorithm(String),
    #[error("malformed {field}: {reason}")]
    Malformed { field: &'static str, reason: String },
    #[error("signature was produced by a key that is not the trust anchor")]
    UntrustedKey,
    #[error("signature does not verify against the bundle")]
    BadSignature,
}

fn decode(field: &'static str, value: &str) -> Result<Vec<u8>, SignatureError> {
    if !value.len().is_multiple_of(2) {
        return Err(SignatureError::Malformed {
            field,
            reason: "odd number of hex digits".into(),
        });
    }
    (0..value.len())
        .step_by(2)
        .map(|index| {
            u8::from_str_radix(&value[index..index + 2], 16).map_err(|error| {
                SignatureError::Malformed {
                    field,
                    reason: error.to_string(),
                }
            })
        })
        .collect()
}

pub fn encode_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Read a hex-encoded 32-byte key from a file, ignoring surrounding whitespace.
pub fn read_key_file(path: &Path) -> Result<[u8; 32], SignatureError> {
    let text = std::fs::read_to_string(path).map_err(|error| SignatureError::Malformed {
        field: "key file",
        reason: error.to_string(),
    })?;
    let bytes = decode("key file", text.trim())?;
    bytes.try_into().map_err(|_| SignatureError::Malformed {
        field: "key file",
        reason: "expected 32 bytes".into(),
    })
}

/// Verify a detached signature over `canonical` against a trust anchor.
///
/// Fails closed on every path: an unsupported version, a foreign key, or a
/// signature that does not verify are all errors, never warnings.
pub fn verify_signature(
    signature: &EvidenceSignature,
    canonical: &[u8],
    trust_anchor: &[u8; 32],
) -> Result<(), SignatureError> {
    if signature.signature_version != SIGNATURE_VERSION {
        return Err(SignatureError::UnsupportedVersion(
            signature.signature_version.clone(),
        ));
    }
    if signature.algorithm != SIGNATURE_ALGORITHM {
        return Err(SignatureError::UnsupportedAlgorithm(
            signature.algorithm.clone(),
        ));
    }

    let public_key: [u8; 32] = decode("public_key", &signature.public_key)?
        .try_into()
        .map_err(|_| SignatureError::Malformed {
            field: "public_key",
            reason: "expected 32 bytes".into(),
        })?;
    if &public_key != trust_anchor {
        return Err(SignatureError::UntrustedKey);
    }

    let raw: [u8; 64] = decode("signature", &signature.signature)?
        .try_into()
        .map_err(|_| SignatureError::Malformed {
            field: "signature",
            reason: "expected 64 bytes".into(),
        })?;

    let key = VerifyingKey::from_bytes(&public_key).map_err(|error| SignatureError::Malformed {
        field: "public_key",
        reason: error.to_string(),
    })?;
    key.verify_strict(canonical, &Signature::from_bytes(&raw))
        .map_err(|_| SignatureError::BadSignature)
}

/// Conventional location of the detached signature for a bundle.
pub fn signature_path(bundle_path: &Path) -> std::path::PathBuf {
    let mut name = bundle_path
        .file_name()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| "bundle".into());
    name.push_str(".sig.json");
    bundle_path.with_file_name(name)
}
