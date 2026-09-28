//! Cross-implementation compatibility test against the Python
//! attestation pipeline, RFC 6962 convention.
//!
//! Both `agent_insure.pipeline.attestation` (Python) and this crate
//! (Rust) implement the Certificate Transparency tree-hash:
//!
//!   * Leaf hash:     SHA-256(0x00 || record_bytes)
//!   * Internal hash: SHA-256(0x01 || left || right)   (raw bytes)
//!   * Empty tree:    SHA-256(b"")
//!   * Single leaf:   the leaf hash itself
//!   * Odd-length levels: the unpaired tail node is promoted up
//!     unchanged (RFC 6962 §2.1).
//!
//! Reference hashes below were produced by the Python implementation
//! and are reproduced by running:
//!
//! ```python
//! import hashlib
//! def lh(x): return hashlib.sha256(b"\x00" + x).digest()
//! def nh(l, r): return hashlib.sha256(b"\x01" + l + r).digest()
//! def root(leaves):
//!     if not leaves: return hashlib.sha256(b"").hexdigest()
//!     h = [lh(x) for x in leaves]
//!     if len(h) == 1: return h[0].hex()
//!     while len(h) > 1:
//!         n = []; i = 0
//!         while i + 1 < len(h):
//!             n.append(nh(h[i], h[i+1])); i += 2
//!         if i < len(h): n.append(h[i])
//!         h = n
//!     return h[0].hex()
//! print(root([b"a", b"b", b"c", b"d"]))
//! # -> 33376a3bd63e9993708a84ddfe6c28ae58b83505dd1fed711bd924ec5a6239f0
//! ```
//!
//! If either side ever changes the Merkle convention this test fails
//! loudly; the contract is that a Rust-signed manifest verifies in
//! Python and vice versa.

use ai_attestation::MerkleTree;

const PY_ROOT_ABCD: &str = "33376a3bd63e9993708a84ddfe6c28ae58b83505dd1fed711bd924ec5a6239f0";
const PY_ROOT_ABC:  &str = "36642e73c2540ab121e3a6bf9545b0a24982cd830eb13d3cd19de3ce6c021ec1";
const PY_ROOT_A:    &str = "022a6979e6dab7aa5ae4c3e5e45f7e977112a7e63593820dbec1ec738a24f93c";
const PY_ROOT_EMPTY:&str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

#[test]
fn matches_python_four_leaf_root() {
    let leaves: Vec<Vec<u8>> = [b"a", b"b", b"c", b"d"].iter().map(|s| s.to_vec()).collect();
    let tree = MerkleTree::build(&leaves);
    assert_eq!(hex::encode(tree.root()), PY_ROOT_ABCD);
}

#[test]
fn matches_python_three_leaf_odd_root() {
    let leaves: Vec<Vec<u8>> = [b"a", b"b", b"c"].iter().map(|s| s.to_vec()).collect();
    let tree = MerkleTree::build(&leaves);
    assert_eq!(hex::encode(tree.root()), PY_ROOT_ABC);
}

#[test]
fn matches_python_single_leaf_root() {
    let tree = MerkleTree::build(&[b"a".to_vec()]);
    assert_eq!(hex::encode(tree.root()), PY_ROOT_A);
}

#[test]
fn matches_python_empty_root() {
    let tree = MerkleTree::build(&[]);
    assert_eq!(hex::encode(tree.root()), PY_ROOT_EMPTY);
}
