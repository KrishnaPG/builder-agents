//! Merkle tree wrapper for artifact content hashing
//!
//! Provides [`ArtifactMerkleTree`] - a thin wrapper around `rs_merkle` crate
//! for incremental content hashing and verification.

use crate::hash::ContentHash;
use rs_merkle::{Hasher, MerkleTree as RsMerkleTree};

/// Wrapper around rs_merkle with ContentHash integration
pub struct ArtifactMerkleTree {
    inner: RsMerkleTree<Blake3Hasher>,
}

impl std::fmt::Debug for ArtifactMerkleTree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ArtifactMerkleTree")
            .field("leaf_count", &self.leaf_count())
            .field("root", &self.root())
            .finish()
    }
}

impl Clone for ArtifactMerkleTree {
    fn clone(&self) -> Self {
        // Rebuild from leaves since RsMerkleTree doesn't implement Clone
        let leaves: Vec<_> = match self.inner.leaves() {
            Some(leaves) => leaves.to_vec(),
            None => Vec::new(),
        };
        Self {
            inner: RsMerkleTree::from_leaves(&leaves),
        }
    }
}

impl ArtifactMerkleTree {
    /// Create empty tree
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: RsMerkleTree::new(),
        }
    }

    /// Build from leaf hashes
    ///
    /// # Performance
    /// O(n) where n = number of leaves
    #[inline]
    #[must_use]
    pub fn from_leaves(leaves: &[ContentHash]) -> Self {
        let leaves: Vec<_> = leaves.iter().map(|h| *h.as_bytes()).collect();
        Self {
            inner: RsMerkleTree::from_leaves(&leaves),
        }
    }

    /// Root hash of the tree
    ///
    /// Returns zero hash for empty tree.
    #[inline]
    #[must_use]
    pub fn root(&self) -> ContentHash {
        match self.inner.root() {
            Some(root) => ContentHash::new(root),
            None => ContentHash::default(),
        }
    }

    /// Number of leaves
    #[inline]
    #[must_use]
    pub fn leaf_count(&self) -> usize {
        self.inner.leaves().map_or(0, |leaves| leaves.len())
    }

    /// Check if tree is empty
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.leaf_count() == 0
    }

    /// Append a new leaf
    ///
    /// Note: This rebuilds the tree. For batch updates, collect leaves
    /// and use [`from_leaves`](Self::from_leaves).
    pub fn append(&mut self, leaf: ContentHash) {
        let mut leaves: Vec<_> = self
            .inner
            .leaves()
            .map_or_else(Vec::new, |l| l.to_vec());
        leaves.push(*leaf.as_bytes());
        self.inner = RsMerkleTree::from_leaves(&leaves);
    }

    /// Get leaf at index
    #[inline]
    #[must_use]
    pub fn get_leaf(&self, index: usize) -> Option<ContentHash> {
        self.inner
            .leaves()
            .and_then(|leaves| {
                leaves.get(index).map(|&bytes| ContentHash::new(bytes))
            })
    }

    /// Generate proof for leaf at index
    ///
    /// # Panics
    /// Panics if index >= leaf_count
    #[inline]
    #[must_use]
    pub fn proof(&self, leaf_index: usize) -> MerkleProof {
        let proof = self.inner.proof(&[leaf_index]);
        MerkleProof { inner: proof }
    }

    /// Verify a proof
    ///
    /// # Arguments
    /// - `leaf`: The leaf hash to verify
    /// - `leaf_index`: Index of the leaf
    /// - `proof`: The merkle proof
    #[inline]
    #[must_use]
    pub fn verify(&self, leaf: ContentHash, leaf_index: usize, proof: &MerkleProof) -> bool {
        proof.verify(leaf, leaf_index, self.root(), self.leaf_count())
    }
}

impl Default for ArtifactMerkleTree {
    fn default() -> Self {
        Self::new()
    }
}

/// Merkle proof for verification
pub struct MerkleProof {
    inner: rs_merkle::MerkleProof<Blake3Hasher>,
}

impl std::fmt::Debug for MerkleProof {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MerkleProof").finish()
    }
}

impl MerkleProof {
    /// Verify this proof
    ///
    /// # Arguments
    /// - `leaf`: The leaf hash
    /// - `leaf_index`: Index in the tree
    /// - `root`: Expected root hash
    /// - `total_leaves`: Total number of leaves in tree
    #[inline]
    #[must_use]
    pub fn verify(
        &self,
        leaf: ContentHash,
        leaf_index: usize,
        root: ContentHash,
        total_leaves: usize,
    ) -> bool {
        self.inner.verify(
            *root.as_bytes(),
            &[leaf_index],
            &[*leaf.as_bytes()],
            total_leaves,
        )
    }
}

/// Blake3 hasher adapter for rs_merkle
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Blake3Hasher;

impl Hasher for Blake3Hasher {
    type Hash = [u8; 32];

    #[inline]
    fn hash(data: &[u8]) -> Self::Hash {
        *blake3::hash(data).as_bytes()
    }
}

/// Iterator over leaf hashes
#[derive(Debug)]
pub struct LeafIterator<'a> {
    inner: std::slice::Iter<'a, [u8; 32]>,
}

impl<'a> Iterator for LeafIterator<'a> {
    type Item = ContentHash;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|bytes| ContentHash::new(*bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_hashes(n: usize) -> Vec<ContentHash> {
        (0..n)
            .map(|i| ContentHash::compute(i.to_string().as_bytes()))
            .collect()
    }

    #[test]
    fn merkle_tree_empty() {
        let tree = ArtifactMerkleTree::new();
        assert!(tree.is_empty());
        assert_eq!(tree.leaf_count(), 0);
        assert!(tree.root().is_zero());
    }

    #[test]
    fn merkle_tree_from_leaves() {
        let leaves = make_hashes(4);
        let tree = ArtifactMerkleTree::from_leaves(&leaves);

        assert_eq!(tree.leaf_count(), 4);
        assert!(!tree.is_empty());
        assert!(!tree.root().is_zero());
    }

    #[test]
    fn merkle_tree_root_deterministic() {
        let leaves = make_hashes(8);
        let tree1 = ArtifactMerkleTree::from_leaves(&leaves);
        let tree2 = ArtifactMerkleTree::from_leaves(&leaves);

        assert_eq!(tree1.root(), tree2.root());
    }

    #[test]
    fn merkle_tree_root_changes_with_leaves() {
        let leaves1 = make_hashes(4);
        let leaves2 = make_hashes(8);

        let tree1 = ArtifactMerkleTree::from_leaves(&leaves1);
        let tree2 = ArtifactMerkleTree::from_leaves(&leaves2);

        assert_ne!(tree1.root(), tree2.root());
    }

    #[test]
    fn merkle_tree_get_leaf() {
        let leaves = make_hashes(4);
        let tree = ArtifactMerkleTree::from_leaves(&leaves);

        assert_eq!(tree.get_leaf(0), Some(leaves[0]));
        assert_eq!(tree.get_leaf(3), Some(leaves[3]));
        assert_eq!(tree.get_leaf(4), None);
    }

    #[test]
    fn merkle_tree_append() {
        let leaves = make_hashes(3);
        let mut tree = ArtifactMerkleTree::from_leaves(&leaves);
        let old_root = tree.root();

        tree.append(leaves[0]);

        assert_eq!(tree.leaf_count(), 4);
        assert_ne!(tree.root(), old_root);
    }

    #[test]
    fn merkle_proof_generation_and_verification() {
        let leaves = make_hashes(8);
        let tree = ArtifactMerkleTree::from_leaves(&leaves);

        let proof = tree.proof(3);
        assert!(tree.verify(leaves[3], 3, &proof));
    }

    #[test]
    fn merkle_proof_fails_for_wrong_leaf() {
        let leaves = make_hashes(8);
        let tree = ArtifactMerkleTree::from_leaves(&leaves);

        let proof = tree.proof(3);
        let wrong_leaf = leaves[4];

        assert!(!tree.verify(wrong_leaf, 3, &proof));
    }

    #[test]
    fn merkle_proof_fails_for_wrong_index() {
        let leaves = make_hashes(8);
        let tree = ArtifactMerkleTree::from_leaves(&leaves);

        let proof = tree.proof(3);

        assert!(!proof.verify(leaves[3], 4, tree.root(), tree.leaf_count()));
    }

    #[test]
    fn merkle_proof_standalone_verification() {
        let leaves = make_hashes(8);
        let tree = ArtifactMerkleTree::from_leaves(&leaves);
        let root = tree.root();

        let proof = tree.proof(5);

        // Verify without tree reference
        assert!(proof.verify(leaves[5], 5, root, tree.leaf_count()));
    }

    #[test]
    fn hasher_blake3_produces_32_bytes() {
        let hash = Blake3Hasher::hash(b"test data");
        assert_eq!(hash.len(), 32);
    }

    #[test]
    fn hasher_blake3_deterministic() {
        let data = b"deterministic";
        let h1 = Blake3Hasher::hash(data);
        let h2 = Blake3Hasher::hash(data);
        assert_eq!(h1, h2);
    }
}
