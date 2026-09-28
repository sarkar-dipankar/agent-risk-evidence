//! Cryptographic Attestation for Agent Insure
//!
//! This crate provides Ed25519-based cryptographic attestation for
//! ensuring data provenance and integrity.
//!
//! # Example
//!
//! ```rust,ignore
//! use ai_attestation::{Attester, AttestationManifest};
//!
//! let attester = Attester::generate();
//! let manifest = attester.attest_batch(&records)?;
//! ```

use chrono::{DateTime, Utc};
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use ed25519_dalek::Verifier as Ed25519Verifier;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub mod merkle;
pub use merkle::{MerkleTree, ProofStep};

/// Attestation errors
#[derive(Debug, Error)]
pub enum AttestationError {
    #[error("Signature verification failed")]
    VerificationFailed,

    #[error("Invalid key format")]
    InvalidKey,

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Merkle root mismatch")]
    MerkleRootMismatch,

    #[error("Merkle inclusion proof failed")]
    InclusionProofFailed,

    #[error("Invalid hex encoding: {0}")]
    InvalidHex(String),
}

/// Cryptographic attestation manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttestationManifest {
    /// Manifest version
    pub version: String,

    /// When the attestation was created
    pub attested_at: DateTime<Utc>,

    /// SHA-256 hash of the attested data
    pub content_hash: String,

    /// Number of records attested
    pub record_count: usize,

    /// Schema version of attested records
    pub schema_version: String,

    /// Ed25519 signature (base64 encoded)
    pub signature: String,

    /// Public key used for verification (base64 encoded)
    pub public_key: String,
}

/// Cryptographic attestation manifest backed by a Merkle root.
///
/// This is the manifest format used by the higher-throughput
/// `attest_batch_with_merkle` path.  Rather than hashing every record
/// into a single content hash, the attester builds a Merkle tree over
/// per-record JSON serializations and signs the root.  Verifiers can
/// then check inclusion of individual records with O(log n) proofs
/// without re-fetching the full batch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleManifest {
    /// Manifest version
    pub version: String,

    /// When the attestation was created
    pub attested_at: DateTime<Utc>,

    /// Hex-encoded Merkle root of the per-record leaves
    pub merkle_root: String,

    /// Number of records attested
    pub record_count: usize,

    /// Schema version of attested records
    pub schema_version: String,

    /// Ed25519 signature (base64 encoded) over
    /// `schema_version || record_count || hex(root)`.
    pub signature: String,

    /// Public key used for verification (base64 encoded)
    pub public_key: String,
}

/// Attester for creating cryptographic attestations
pub struct Attester {
    signing_key: SigningKey,
}

impl Attester {
    /// Generate a new attester with a random key pair
    pub fn generate() -> Self {
        let signing_key = SigningKey::generate(&mut OsRng);
        Self { signing_key }
    }

    /// Create an attester from an existing signing key
    pub fn from_key(key_bytes: &[u8; 32]) -> Self {
        let signing_key = SigningKey::from_bytes(key_bytes);
        Self { signing_key }
    }

    /// Get the public key for verification
    pub fn public_key(&self) -> VerifyingKey {
        self.signing_key.verifying_key()
    }

    /// Get the public key as base64 string
    pub fn public_key_base64(&self) -> String {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD.encode(self.public_key().as_bytes())
    }

    /// Attest a batch of records
    pub fn attest_batch<T: Serialize>(
        &self,
        records: &[T],
        schema_version: &str,
    ) -> Result<AttestationManifest, AttestationError> {
        // Serialize and hash the records
        let json = serde_json::to_string(records)?;
        let hash = Sha256::digest(json.as_bytes());
        let content_hash = hex::encode(hash);

        // Create the message to sign
        let message = format!(
            "agent-insure-attestation:v1:{}:{}:{}",
            schema_version,
            records.len(),
            content_hash
        );

        // Sign the message
        let signature = self.signing_key.sign(message.as_bytes());

        use base64::Engine;
        let signature_base64 =
            base64::engine::general_purpose::STANDARD.encode(signature.to_bytes());

        Ok(AttestationManifest {
            version: "1.0".to_string(),
            attested_at: Utc::now(),
            content_hash,
            record_count: records.len(),
            schema_version: schema_version.to_string(),
            signature: signature_base64,
            public_key: self.public_key_base64(),
        })
    }

    /// Attest a batch of records by building a Merkle tree over the
    /// per-record JSON serializations and signing the root.
    ///
    /// The signed payload is `schema_version || record_count || hex(root)`
    /// concatenated with a fixed separator, mirroring the existing
    /// `attest_batch` style.  The returned [`MerkleManifest`] carries
    /// the hex-encoded root rather than a single content hash so
    /// downstream consumers can supply per-record inclusion proofs
    /// without re-attesting.
    pub fn attest_batch_with_merkle<T: Serialize>(
        &self,
        records: &[T],
        schema_version: &str,
    ) -> Result<MerkleManifest, AttestationError> {
        let leaves = records_to_leaves(records)?;
        let tree = MerkleTree::build(&leaves);
        let root = tree.root();
        let root_hex = hex::encode(root);

        let message = signing_payload(schema_version, records.len(), &root_hex);
        let signature = self.signing_key.sign(message.as_bytes());

        use base64::Engine;
        let signature_base64 =
            base64::engine::general_purpose::STANDARD.encode(signature.to_bytes());

        Ok(MerkleManifest {
            version: "1.0".to_string(),
            attested_at: Utc::now(),
            merkle_root: root_hex,
            record_count: records.len(),
            schema_version: schema_version.to_string(),
            signature: signature_base64,
            public_key: self.public_key_base64(),
        })
    }

    /// Verify a single-record inclusion proof against a known root.
    ///
    /// Returns `true` iff serialising `record` to canonical JSON and
    /// running the proof yields exactly `root_hex`.  Bad hex decoding
    /// or proof entries are treated as a failed verification rather
    /// than an error so the API is easy to use in benchmark loops.
    pub fn verify_inclusion<T: Serialize>(
        record: &T,
        index: usize,
        proof: &[merkle::ProofStep],
        root_hex: &str,
    ) -> bool {
        let leaf = match serde_json::to_vec(record) {
            Ok(b) => b,
            Err(_) => return false,
        };
        let Ok(root_bytes) = hex::decode(root_hex) else {
            return false;
        };
        let Ok(root_arr): Result<[u8; 32], _> = root_bytes.try_into() else {
            return false;
        };
        MerkleTree::verify_inclusion(&leaf, index, proof, &root_arr)
    }
}

/// Canonical "what was signed" payload for the Merkle attestation
/// path.  Kept as a free function so verifiers can reproduce it
/// without instantiating an `Attester`.
fn signing_payload(schema_version: &str, record_count: usize, root_hex: &str) -> String {
    format!(
        "agent-insure-attestation:v1-merkle:{}:{}:{}",
        schema_version, record_count, root_hex
    )
}

/// Serialize every record to JSON bytes for use as Merkle leaves.
fn records_to_leaves<T: Serialize>(records: &[T]) -> Result<Vec<Vec<u8>>, AttestationError> {
    records
        .iter()
        .map(|r| serde_json::to_vec(r).map_err(AttestationError::SerializationError))
        .collect()
}

/// Verifier for checking attestation signatures
pub struct AttestationVerifier;

impl AttestationVerifier {
    /// Verify an attestation manifest against the original data
    pub fn verify<T: Serialize>(
        manifest: &AttestationManifest,
        records: &[T],
    ) -> Result<bool, AttestationError> {
        // Verify record count
        if records.len() != manifest.record_count {
            return Ok(false);
        }

        // Verify content hash
        let json =
            serde_json::to_string(records).map_err(AttestationError::SerializationError)?;
        let hash = Sha256::digest(json.as_bytes());
        let content_hash = hex::encode(hash);

        if content_hash != manifest.content_hash {
            return Ok(false);
        }

        // Reconstruct the signed message
        let message = format!(
            "agent-insure-attestation:v1:{}:{}:{}",
            manifest.schema_version, manifest.record_count, manifest.content_hash
        );

        // Decode public key
        use base64::Engine;
        let public_key_bytes = base64::engine::general_purpose::STANDARD
            .decode(&manifest.public_key)
            .map_err(|_| AttestationError::InvalidKey)?;

        let public_key_array: [u8; 32] = public_key_bytes
            .try_into()
            .map_err(|_| AttestationError::InvalidKey)?;

        let verifying_key = VerifyingKey::from_bytes(&public_key_array)
            .map_err(|_| AttestationError::InvalidKey)?;

        // Decode signature
        let signature_bytes = base64::engine::general_purpose::STANDARD
            .decode(&manifest.signature)
            .map_err(|_| AttestationError::InvalidKey)?;

        let signature_array: [u8; 64] = signature_bytes
            .try_into()
            .map_err(|_| AttestationError::InvalidKey)?;

        let signature = Signature::from_bytes(&signature_array);

        // Verify
        verifying_key
            .verify(message.as_bytes(), &signature)
            .map_err(|_| AttestationError::VerificationFailed)?;

        Ok(true)
    }

    /// Verify a Merkle-based attestation manifest *and* an inclusion
    /// proof for one record in a single call.
    ///
    /// Checks performed, in order:
    /// 1. The manifest signature must validate under `public_key`.
    /// 2. `root` (raw 32 bytes) must equal the hex-decoded
    ///    `manifest.merkle_root`.
    /// 3. `proof` must show that `record` at `index` reconstructs
    ///    `root`.
    ///
    /// This is the function the academic paper's verifier-side
    /// throughput numbers exercise.
    pub fn verify_merkle_inclusion<T: Serialize>(
        manifest: &MerkleManifest,
        public_key: &VerifyingKey,
        root: &[u8; 32],
        proof: &[merkle::ProofStep],
        record: &T,
        index: usize,
    ) -> Result<(), AttestationError> {
        // 1. Signature.
        let message = signing_payload(
            &manifest.schema_version,
            manifest.record_count,
            &manifest.merkle_root,
        );

        use base64::Engine;
        let signature_bytes = base64::engine::general_purpose::STANDARD
            .decode(&manifest.signature)
            .map_err(|_| AttestationError::InvalidKey)?;
        let signature_array: [u8; 64] = signature_bytes
            .try_into()
            .map_err(|_| AttestationError::InvalidKey)?;
        let signature = Signature::from_bytes(&signature_array);
        public_key
            .verify(message.as_bytes(), &signature)
            .map_err(|_| AttestationError::VerificationFailed)?;

        // 2. Root must match what the manifest claims.
        let manifest_root_bytes = hex::decode(&manifest.merkle_root)
            .map_err(|e| AttestationError::InvalidHex(e.to_string()))?;
        if manifest_root_bytes.as_slice() != root.as_slice() {
            return Err(AttestationError::MerkleRootMismatch);
        }

        // 3. Inclusion proof.
        let leaf = serde_json::to_vec(record)?;
        if !MerkleTree::verify_inclusion(&leaf, index, proof, root) {
            return Err(AttestationError::InclusionProofFailed);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attestation_roundtrip() {
        let attester = Attester::generate();

        let records = vec![
            serde_json::json!({"id": "1", "value": 100}),
            serde_json::json!({"id": "2", "value": 200}),
        ];

        let manifest = attester.attest_batch(&records, "1.0.0").unwrap();

        assert_eq!(manifest.record_count, 2);
        assert_eq!(manifest.schema_version, "1.0.0");

        // Verify the attestation
        let valid = AttestationVerifier::verify(&manifest, &records).unwrap();
        assert!(valid);
    }

    #[test]
    fn test_attestation_tamper_detection() {
        let attester = Attester::generate();

        let records = vec![serde_json::json!({"id": "1", "value": 100})];

        let manifest = attester.attest_batch(&records, "1.0.0").unwrap();

        // Tamper with the records
        let tampered_records = vec![serde_json::json!({"id": "1", "value": 999})];

        let valid = AttestationVerifier::verify(&manifest, &tampered_records).unwrap();
        assert!(!valid);
    }

    #[test]
    fn merkle_attestation_roundtrip() {
        let attester = Attester::generate();
        let records: Vec<_> = (0..16)
            .map(|i| serde_json::json!({"id": i.to_string(), "value": i * 7}))
            .collect();

        let manifest = attester
            .attest_batch_with_merkle(&records, "1.0.0")
            .unwrap();
        assert_eq!(manifest.record_count, 16);
        assert_eq!(manifest.schema_version, "1.0.0");
        assert_eq!(manifest.merkle_root.len(), 64);

        // Rebuild the tree on the verifier side and check each record.
        let leaves: Vec<Vec<u8>> = records
            .iter()
            .map(|r| serde_json::to_vec(r).unwrap())
            .collect();
        let tree = MerkleTree::build(&leaves);
        let root = tree.root();
        assert_eq!(hex::encode(root), manifest.merkle_root);

        let public_key = attester.public_key();
        for (i, record) in records.iter().enumerate() {
            let proof = tree.inclusion_proof(i);
            AttestationVerifier::verify_merkle_inclusion(
                &manifest,
                &public_key,
                &root,
                &proof,
                record,
                i,
            )
            .expect("inclusion verify");

            assert!(Attester::verify_inclusion(
                record,
                i,
                &proof,
                &manifest.merkle_root
            ));
        }
    }

    #[test]
    fn merkle_attestation_detects_tamper() {
        let attester = Attester::generate();
        let records: Vec<_> = (0..4)
            .map(|i| serde_json::json!({"id": i.to_string(), "value": i}))
            .collect();
        let manifest = attester
            .attest_batch_with_merkle(&records, "1.0.0")
            .unwrap();

        let leaves: Vec<Vec<u8>> = records
            .iter()
            .map(|r| serde_json::to_vec(r).unwrap())
            .collect();
        let tree = MerkleTree::build(&leaves);
        let root = tree.root();
        let proof = tree.inclusion_proof(2);
        let public_key = attester.public_key();

        // Tampered record fails inclusion check.
        let tampered = serde_json::json!({"id": "2", "value": 9999});
        let err = AttestationVerifier::verify_merkle_inclusion(
            &manifest,
            &public_key,
            &root,
            &proof,
            &tampered,
            2,
        )
        .unwrap_err();
        assert!(matches!(err, AttestationError::InclusionProofFailed));
    }
}
