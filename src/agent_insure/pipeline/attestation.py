"""Ed25519 attestation signing for data provenance.

Provides cryptographic attestation of validated records using Ed25519
signatures. Creates signed manifests for batches of records.
"""

import base64
import hashlib
import json
from dataclasses import dataclass
from datetime import datetime
from typing import Any

from cryptography.exceptions import InvalidSignature
from cryptography.hazmat.primitives import serialization
from cryptography.hazmat.primitives.asymmetric.ed25519 import (
    Ed25519PrivateKey,
    Ed25519PublicKey,
)


@dataclass
class AttestationManifest:
    """Manifest attesting to a batch of validated records."""

    # Manifest metadata
    manifest_id: str
    created_at: datetime
    schema_version: str
    validator_version: str

    # Record hashes
    record_count: int
    record_hashes: list[str]  # SHA-256 hashes of canonical JSON
    root_hash: str            # Merkle root of record hashes

    # Signature
    signature: str            # Base64 Ed25519 signature
    public_key: str           # Base64 public key for verification

    # Optional metadata
    signer_id: str | None = None
    environment: str | None = None
    notes: str | None = None

    def to_dict(self) -> dict[str, Any]:
        """Convert to dictionary for serialization."""
        return {
            "manifest_id": self.manifest_id,
            "created_at": self.created_at.isoformat(),
            "schema_version": self.schema_version,
            "validator_version": self.validator_version,
            "record_count": self.record_count,
            "record_hashes": self.record_hashes,
            "root_hash": self.root_hash,
            "signature": self.signature,
            "public_key": self.public_key,
            "signer_id": self.signer_id,
            "environment": self.environment,
            "notes": self.notes,
        }

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "AttestationManifest":
        """Create from dictionary."""
        return cls(
            manifest_id=data["manifest_id"],
            created_at=datetime.fromisoformat(data["created_at"]),
            schema_version=data["schema_version"],
            validator_version=data["validator_version"],
            record_count=data["record_count"],
            record_hashes=data["record_hashes"],
            root_hash=data["root_hash"],
            signature=data["signature"],
            public_key=data["public_key"],
            signer_id=data.get("signer_id"),
            environment=data.get("environment"),
            notes=data.get("notes"),
        )


@dataclass
class SignedRecord:
    """A record with its attestation signature."""

    record: dict[str, Any]
    record_hash: str
    signature: str
    public_key: str
    signed_at: datetime

    def to_dict(self) -> dict[str, Any]:
        """Convert to dictionary."""
        return {
            "record": self.record,
            "attestation": {
                "record_hash": self.record_hash,
                "signature": self.signature,
                "public_key": self.public_key,
                "signed_at": self.signed_at.isoformat(),
            },
        }


class AttestationManager:
    """Manages Ed25519 signing and verification of records.

    Provides cryptographic attestation for validated insurance data records.
    Uses Ed25519 for fast, small signatures.

    Example:
        # Generate keys
        private_key, public_key = AttestationManager.generate_keypair()

        # Create manager
        manager = AttestationManager(private_key)

        # Sign records
        manifest = manager.create_manifest(
            records=[record1, record2],
            schema_version="1.0.0",
            validator_version="1.0.0"
        )

        # Verify
        is_valid = manager.verify_manifest(manifest)
    """

    def __init__(
        self,
        private_key: Ed25519PrivateKey,
        signer_id: str | None = None,
    ):
        """Initialize with Ed25519 private key.

        Args:
            private_key: Ed25519 private key for signing
            signer_id: Optional identifier for the signer
        """
        self.private_key = private_key
        self.public_key = private_key.public_key()
        self.signer_id = signer_id

    @staticmethod
    def generate_keypair() -> tuple[Ed25519PrivateKey, Ed25519PublicKey]:
        """Generate a new Ed25519 keypair.

        Returns:
            Tuple of (private_key, public_key)
        """
        private_key = Ed25519PrivateKey.generate()
        public_key = private_key.public_key()
        return private_key, public_key

    @staticmethod
    def load_private_key(key_bytes: bytes) -> Ed25519PrivateKey:
        """Load a private key from bytes.

        Args:
            key_bytes: Raw private key bytes (32 bytes)

        Returns:
            Ed25519PrivateKey
        """
        return Ed25519PrivateKey.from_private_bytes(key_bytes)

    @staticmethod
    def load_public_key(key_bytes: bytes) -> Ed25519PublicKey:
        """Load a public key from bytes.

        Args:
            key_bytes: Raw public key bytes (32 bytes)

        Returns:
            Ed25519PublicKey
        """
        return Ed25519PublicKey.from_public_bytes(key_bytes)

    def get_public_key_bytes(self) -> bytes:
        """Get raw public key bytes."""
        return self.public_key.public_bytes(
            encoding=serialization.Encoding.Raw,
            format=serialization.PublicFormat.Raw,
        )

    def get_public_key_base64(self) -> str:
        """Get base64-encoded public key."""
        return base64.b64encode(self.get_public_key_bytes()).decode("ascii")

    def create_manifest(
        self,
        records: list[dict[str, Any]],
        schema_version: str,
        validator_version: str,
        environment: str | None = None,
        notes: str | None = None,
    ) -> AttestationManifest:
        """Create a signed attestation manifest for a batch of records.

        Args:
            records: List of record dictionaries to attest
            schema_version: Schema version used for validation
            validator_version: Validator version used
            environment: Optional environment identifier
            notes: Optional notes about this attestation

        Returns:
            Signed AttestationManifest
        """
        # Hash each record
        record_hashes = [self._hash_record(r) for r in records]

        # Compute Merkle root
        root_hash = self._compute_merkle_root(record_hashes)

        # Create manifest ID
        manifest_id = self._generate_manifest_id(root_hash, schema_version)

        # Create signature payload
        payload = self._create_signature_payload(
            manifest_id=manifest_id,
            schema_version=schema_version,
            validator_version=validator_version,
            record_count=len(records),
            root_hash=root_hash,
        )

        # Sign
        signature = self._sign(payload)

        return AttestationManifest(
            manifest_id=manifest_id,
            created_at=datetime.now(),
            schema_version=schema_version,
            validator_version=validator_version,
            record_count=len(records),
            record_hashes=record_hashes,
            root_hash=root_hash,
            signature=signature,
            public_key=self.get_public_key_base64(),
            signer_id=self.signer_id,
            environment=environment,
            notes=notes,
        )

    def sign_record(self, record: dict[str, Any]) -> SignedRecord:
        """Sign an individual record.

        Args:
            record: Record dictionary to sign

        Returns:
            SignedRecord with attestation
        """
        record_hash = self._hash_record(record)
        signature = self._sign(record_hash.encode("utf-8"))

        return SignedRecord(
            record=record,
            record_hash=record_hash,
            signature=signature,
            public_key=self.get_public_key_base64(),
            signed_at=datetime.now(),
        )

    def verify_manifest(
        self,
        manifest: AttestationManifest,
        records: list[dict[str, Any]] | None = None,
    ) -> bool:
        """Verify an attestation manifest.

        Args:
            manifest: Manifest to verify
            records: Optional records to verify against hashes

        Returns:
            True if signature and hashes are valid
        """
        try:
            # Load public key from manifest
            public_key_bytes = base64.b64decode(manifest.public_key)
            public_key = Ed25519PublicKey.from_public_bytes(public_key_bytes)

            # Recreate signature payload
            payload = self._create_signature_payload(
                manifest_id=manifest.manifest_id,
                schema_version=manifest.schema_version,
                validator_version=manifest.validator_version,
                record_count=manifest.record_count,
                root_hash=manifest.root_hash,
            )

            # Verify signature
            signature_bytes = base64.b64decode(manifest.signature)
            public_key.verify(signature_bytes, payload)

            # Optionally verify record hashes
            if records is not None:
                if len(records) != manifest.record_count:
                    return False

                for i, record in enumerate(records):
                    expected_hash = manifest.record_hashes[i]
                    actual_hash = self._hash_record(record)
                    if actual_hash != expected_hash:
                        return False

                # Verify Merkle root
                computed_root = self._compute_merkle_root(manifest.record_hashes)
                if computed_root != manifest.root_hash:
                    return False

            return True

        except (InvalidSignature, ValueError, IndexError):
            return False

    def verify_signature(
        self,
        record: dict[str, Any],
        signature: str,
        public_key_b64: str,
    ) -> bool:
        """Verify an Ed25519 signature on a record.

        Args:
            record: Record dictionary that was signed
            signature: Base64-encoded signature
            public_key_b64: Base64-encoded public key

        Returns:
            True if signature is valid
        """
        try:
            public_key_bytes = base64.b64decode(public_key_b64)
            public_key = Ed25519PublicKey.from_public_bytes(public_key_bytes)

            record_hash = self._hash_record(record)
            signature_bytes = base64.b64decode(signature)

            public_key.verify(signature_bytes, record_hash.encode("utf-8"))
            return True

        except (InvalidSignature, ValueError):
            return False

    def _sign(self, data: bytes) -> str:
        """Sign data and return base64 signature."""
        signature = self.private_key.sign(data)
        return base64.b64encode(signature).decode("ascii")

    def _hash_record(self, record: dict[str, Any]) -> str:
        """Leaf hash per RFC 6962: SHA-256(0x00 || canonical_json_bytes).

        The 0x00 prefix byte provides leaf/internal-node domain separation,
        preventing second-preimage attacks that confuse leaves with internal
        Merkle nodes. Output is the hex-encoded digest for JSON serialization.
        """
        canonical = self._canonical_json(record)
        return hashlib.sha256(b"\x00" + canonical.encode("utf-8")).hexdigest()

    def _canonical_json(self, obj: Any) -> str:
        """Canonical JSON: sorted keys, no whitespace, str() fallback."""
        return json.dumps(obj, sort_keys=True, separators=(",", ":"), default=str)

    def _compute_merkle_root(self, hashes: list[str]) -> str:
        """Compute Merkle root per RFC 6962 (Certificate Transparency).

        Conventions:
        - Empty input: SHA-256(b'') as the empty-tree sentinel.
        - Single leaf: that leaf hash unchanged.
        - Internal nodes: SHA-256(0x01 || left_bytes || right_bytes), on raw
          bytes (not hex), with 0x01 prefix for domain separation.
        - Odd-length levels: the unpaired last node is promoted up unchanged,
          NOT duplicated (RFC 6962 §2.1).

        Inputs and outputs are hex strings for serialization; we decode to
        bytes for the actual hashing.
        """
        if not hashes:
            return hashlib.sha256(b"").hexdigest()
        if len(hashes) == 1:
            return hashes[0]

        current: list[bytes] = [bytes.fromhex(h) for h in hashes]
        while len(current) > 1:
            next_level: list[bytes] = []
            i = 0
            while i + 1 < len(current):
                combined = hashlib.sha256(
                    b"\x01" + current[i] + current[i + 1]
                ).digest()
                next_level.append(combined)
                i += 2
            if i < len(current):  # odd: promote unpaired tail up
                next_level.append(current[i])
            current = next_level
        return current[0].hex()

    def _create_signature_payload(
        self,
        manifest_id: str,
        schema_version: str,
        validator_version: str,
        record_count: int,
        root_hash: str,
    ) -> bytes:
        """Create deterministic payload for signing."""
        payload = {
            "manifest_id": manifest_id,
            "schema_version": schema_version,
            "validator_version": validator_version,
            "record_count": record_count,
            "root_hash": root_hash,
        }
        return self._canonical_json(payload).encode("utf-8")

    def _generate_manifest_id(self, root_hash: str, schema_version: str) -> str:
        """Generate unique manifest ID."""
        timestamp = datetime.now().isoformat()
        input_str = f"{root_hash}:{schema_version}:{timestamp}"
        hash_prefix = hashlib.sha256(input_str.encode()).hexdigest()[:12]
        return f"manifest_{hash_prefix}"


def export_private_key(private_key: Ed25519PrivateKey, password: bytes | None = None) -> bytes:
    """Export private key in PEM format.

    Args:
        private_key: Key to export
        password: Optional encryption password

    Returns:
        PEM-encoded private key bytes
    """
    encryption = (
        serialization.BestAvailableEncryption(password)
        if password
        else serialization.NoEncryption()
    )

    return private_key.private_bytes(
        encoding=serialization.Encoding.PEM,
        format=serialization.PrivateFormat.PKCS8,
        encryption_algorithm=encryption,
    )


def load_private_key_pem(pem_data: bytes, password: bytes | None = None) -> Ed25519PrivateKey:
    """Load private key from PEM format.

    Args:
        pem_data: PEM-encoded key data
        password: Password if key is encrypted

    Returns:
        Ed25519PrivateKey
    """
    private_key = serialization.load_pem_private_key(pem_data, password=password)
    if not isinstance(private_key, Ed25519PrivateKey):
        raise TypeError("Expected an Ed25519 private key")
    return private_key


def export_public_key(public_key: Ed25519PublicKey) -> bytes:
    """Export public key in PEM format.

    Args:
        public_key: Key to export

    Returns:
        PEM-encoded public key bytes
    """
    return public_key.public_bytes(
        encoding=serialization.Encoding.PEM,
        format=serialization.PublicFormat.SubjectPublicKeyInfo,
    )


def load_public_key_pem(pem_data: bytes) -> Ed25519PublicKey:
    """Load public key from PEM format.

    Args:
        pem_data: PEM-encoded key data

    Returns:
        Ed25519PublicKey
    """
    public_key = serialization.load_pem_public_key(pem_data)
    if not isinstance(public_key, Ed25519PublicKey):
        raise TypeError("Expected an Ed25519 public key")
    return public_key
