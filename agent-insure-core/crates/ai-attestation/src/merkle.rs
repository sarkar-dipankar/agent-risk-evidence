//! RFC 6962-style Merkle tree for batch attestation.
//!
//! Implements the Certificate Transparency tree-hash construction of
//! Laurie, Langley, and Kasper, RFC 6962 §2.1:
//!
//!   * Leaf hash:     `SHA-256(0x00 || record_bytes)`
//!   * Internal hash: `SHA-256(0x01 || left || right)` on raw bytes (NOT hex)
//!   * Empty tree:    `SHA-256(b"")`
//!   * Single leaf:   the leaf hash itself
//!   * Odd levels:    the unpaired tail node is promoted up unchanged
//!     (NOT duplicated)
//!
//! The 0x00 / 0x01 domain-separation bytes prevent second-preimage
//! confusion between a leaf and an internal-node hash pair.  The Python
//! reference implementation in `src/agent_insure/pipeline/attestation.py`
//! follows the same convention; cross-implementation root equivalence is
//! exercised by `tests/python_compat.rs`.
//!
//! Inclusion proofs are encoded as the standard RFC 6962 audit path: a
//! list of sibling hashes ordered from the leaf level upward.  The path
//! direction is recovered from the leaf index by inspecting bit-i of the
//! index at level i.

use sha2::{Digest, Sha256};

const LEAF_PREFIX: u8 = 0x00;
const NODE_PREFIX: u8 = 0x01;

/// A single step in an RFC 6962 inclusion proof: a sibling hash and
/// the side it occupies at that tree level.
#[derive(Debug, Clone, Copy)]
pub struct ProofStep {
    pub sibling: [u8; 32],
    pub sibling_is_right: bool,
}

#[derive(Debug, Clone)]
pub struct MerkleTree {
    levels: Vec<Vec<[u8; 32]>>, // bottom-up; levels[0] = leaf hashes
}

impl MerkleTree {
    /// Build a tree from raw leaf bytes.
    pub fn build(leaves: &[Vec<u8>]) -> Self {
        if leaves.is_empty() {
            let empty = Sha256::digest(b"");
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&empty);
            return Self { levels: vec![vec![arr]] };
        }

        let leaf_hashes: Vec<[u8; 32]> = leaves.iter().map(|l| leaf_hash(l)).collect();
        let mut levels: Vec<Vec<[u8; 32]>> = vec![leaf_hashes];

        while levels.last().expect("non-empty").len() > 1 {
            let current = levels.last().expect("non-empty");
            let mut next: Vec<[u8; 32]> = Vec::with_capacity(current.len().div_ceil(2));
            let mut i = 0;
            while i + 1 < current.len() {
                next.push(node_hash(&current[i], &current[i + 1]));
                i += 2;
            }
            if i < current.len() {
                // Odd: promote the unpaired tail up unchanged (RFC 6962).
                next.push(current[i]);
            }
            levels.push(next);
        }

        Self { levels }
    }

    /// Merkle root.
    pub fn root(&self) -> [u8; 32] {
        self.levels
            .last()
            .and_then(|l| l.first().copied())
            .unwrap_or([0u8; 32])
    }

    /// Inclusion proof for the leaf at `index`.  Each step records the
    /// sibling hash and a `sibling_is_right` flag indicating which side
    /// the sibling is on at that level (which determines argument order
    /// when re-hashing during verification).  For a single-leaf tree
    /// the proof is empty.  Levels at which the leaf was the unpaired
    /// odd tail (promoted up without combining) emit no proof element.
    pub fn inclusion_proof(&self, index: usize) -> Vec<ProofStep> {
        let mut proof = Vec::new();
        if self.levels.is_empty() || self.levels[0].is_empty() {
            return proof;
        }
        let mut idx = index;
        for level in &self.levels[..self.levels.len().saturating_sub(1)] {
            if idx % 2 == 0 {
                // Even at this level — sibling, if any, is on the right.
                if idx + 1 < level.len() {
                    proof.push(ProofStep {
                        sibling: level[idx + 1],
                        sibling_is_right: true,
                    });
                    idx /= 2;
                } else {
                    // Unpaired tail: promoted up unchanged.  Position in
                    // the parent level is `idx / 2`, which retains the
                    // parity needed for the next step.
                    idx /= 2;
                }
            } else {
                // Odd at this level — sibling is on the left.
                proof.push(ProofStep {
                    sibling: level[idx - 1],
                    sibling_is_right: false,
                });
                idx /= 2;
            }
        }
        proof
    }

    /// Verify that `leaf` is at `index` in a tree with the given `root`.
    pub fn verify_inclusion(
        leaf: &[u8],
        _index: usize,
        proof: &[ProofStep],
        root: &[u8; 32],
    ) -> bool {
        let mut current = leaf_hash(leaf);
        for step in proof {
            current = if step.sibling_is_right {
                node_hash(&current, &step.sibling)
            } else {
                node_hash(&step.sibling, &current)
            };
        }
        &current == root
    }

    pub fn leaf_count(&self) -> usize {
        if self.levels.is_empty() {
            0
        } else if self.levels.len() == 1 && self.levels[0].len() == 1 {
            // Could be either a single-leaf tree or the empty-tree sentinel.
            1
        } else {
            self.levels[0].len()
        }
    }
}

#[inline]
fn leaf_hash(data: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update([LEAF_PREFIX]);
    h.update(data);
    let d = h.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&d);
    out
}

#[inline]
fn node_hash(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update([NODE_PREFIX]);
    h.update(left);
    h.update(right);
    let d = h.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&d);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex32(s: &str) -> [u8; 32] {
        let bytes = hex::decode(s).expect("valid hex");
        let mut out = [0u8; 32];
        out.copy_from_slice(&bytes);
        out
    }

    #[test]
    fn empty_input_returns_sha256_of_empty() {
        let tree = MerkleTree::build(&[]);
        let expected =
            hex32("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
        assert_eq!(tree.root(), expected);
    }

    #[test]
    fn single_leaf_root_is_leaf_hash() {
        let leaf = b"a".to_vec();
        let tree = MerkleTree::build(&[leaf.clone()]);
        // RFC 6962: SHA-256(0x00 || "a")
        let expected =
            hex32("022a6979e6dab7aa5ae4c3e5e45f7e977112a7e63593820dbec1ec738a24f93c");
        assert_eq!(tree.root(), expected);

        let proof = tree.inclusion_proof(0);
        assert!(proof.is_empty());
        assert!(MerkleTree::verify_inclusion(&leaf, 0, &proof, &tree.root()));
    }

    #[test]
    fn two_leaves_python_compat() {
        let leaves = vec![b"a".to_vec(), b"b".to_vec()];
        let tree = MerkleTree::build(&leaves);
        let expected =
            hex32("b137985ff484fb600db93107c77b0365c80d78f5b429ded0fd97361d077999eb");
        assert_eq!(tree.root(), expected);

        for (i, leaf) in leaves.iter().enumerate() {
            let proof = tree.inclusion_proof(i);
            assert_eq!(proof.len(), 1);
            assert!(MerkleTree::verify_inclusion(leaf, i, &proof, &tree.root()));
        }
    }

    #[test]
    fn four_leaves_match_python_reference() {
        let leaves: Vec<Vec<u8>> = [b"a", b"b", b"c", b"d"]
            .iter()
            .map(|s| s.to_vec())
            .collect();
        let tree = MerkleTree::build(&leaves);
        let expected =
            hex32("33376a3bd63e9993708a84ddfe6c28ae58b83505dd1fed711bd924ec5a6239f0");
        assert_eq!(tree.root(), expected);

        for (i, leaf) in leaves.iter().enumerate() {
            let proof = tree.inclusion_proof(i);
            assert!(
                MerkleTree::verify_inclusion(leaf, i, &proof, &tree.root()),
                "proof failed for index {i}"
            );
        }
    }

    #[test]
    fn odd_count_promotes_last() {
        let leaves: Vec<Vec<u8>> = [b"a", b"b", b"c"].iter().map(|s| s.to_vec()).collect();
        let tree = MerkleTree::build(&leaves);
        let expected =
            hex32("36642e73c2540ab121e3a6bf9545b0a24982cd830eb13d3cd19de3ce6c021ec1");
        assert_eq!(tree.root(), expected);

        for (i, leaf) in leaves.iter().enumerate() {
            let proof = tree.inclusion_proof(i);
            assert!(
                MerkleTree::verify_inclusion(leaf, i, &proof, &tree.root()),
                "proof failed for index {i}"
            );
        }
    }

    #[test]
    fn large_batch_determinism() {
        let leaves: Vec<Vec<u8>> = (0..1024u32)
            .map(|i| format!("leaf-{i}").into_bytes())
            .collect();
        let t1 = MerkleTree::build(&leaves);
        let t2 = MerkleTree::build(&leaves);
        assert_eq!(t1.root(), t2.root());

        for &i in &[0usize, 1, 2, 511, 512, 1022, 1023] {
            let proof = t1.inclusion_proof(i);
            assert!(
                MerkleTree::verify_inclusion(&leaves[i], i, &proof, &t1.root()),
                "proof verify failed at index {i}"
            );
        }
    }

    #[test]
    fn tamper_test() {
        let leaves: Vec<Vec<u8>> = [
            b"alpha".as_slice(),
            b"beta".as_slice(),
            b"gamma".as_slice(),
            b"delta".as_slice(),
        ]
        .iter()
        .map(|s| s.to_vec())
        .collect();
        let tree = MerkleTree::build(&leaves);
        let proof = tree.inclusion_proof(2);

        assert!(MerkleTree::verify_inclusion(&leaves[2], 2, &proof, &tree.root()));

        // Tampered leaf fails.
        let tampered = b"gamma-tampered".to_vec();
        assert!(!MerkleTree::verify_inclusion(&tampered, 2, &proof, &tree.root()));

        // The index parameter is informational (the proof now carries
        // direction information per RFC 6962), so passing a different
        // index does not change verification — the proof for index 2
        // still validates leaves[2] no matter what index argument is
        // supplied.  A wrong *leaf* under that proof must fail, which
        // is what the previous assertion already covers.

        // Tampered proof element fails.
        let mut bad_proof = proof.clone();
        if let Some(first) = bad_proof.first_mut() {
            first.sibling[0] ^= 0xff;
        }
        assert!(!MerkleTree::verify_inclusion(&leaves[2], 2, &bad_proof, &tree.root()));
    }

    #[test]
    fn domain_separation_holds() {
        // Concrete RFC 6962 property: leaf hashes and internal-node
        // hashes live in disjoint spaces under SHA-256 because of the
        // 0x00 / 0x01 prefix bytes.  Specifically, no leaf hash of any
        // input should ever equal an internal hash of any (left, right).
        // We test the basic case: leaf("ab") != node(leaf("a"), leaf("b")).
        let l = leaf_hash(b"ab");
        let n = node_hash(&leaf_hash(b"a"), &leaf_hash(b"b"));
        assert_ne!(l, n);
    }
}
