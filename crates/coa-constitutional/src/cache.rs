//! Content-addressed artifact cache using moka
//!
//! Provides high-performance, concurrent caching of artifacts by their content hash.

use coa_artifact::{Artifact, ArtifactType, ContentHash};
use moka::future::Cache;
use std::any::{Any, TypeId};
use std::fmt::Debug;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

/// Statistics for cache performance monitoring
#[derive(Debug, Clone, Copy, Default)]
pub struct CacheStats {
    /// Number of entries in cache
    pub entry_count: u64,
}

/// Content-addressed artifact cache
///
/// Stores artifacts by their content hash, enabling:
/// - Deduplication (same content = same hash = single entry)
/// - Fast lookup by hash
/// - Automatic eviction based on LRU
/// - Time-based expiration (TTL)
#[derive(Debug, Clone)]
pub struct ArtifactCache {
    inner: Cache<ContentHash, Arc<dyn Any + Send + Sync>>,
}

impl ArtifactCache {
    /// Create new cache with max capacity
    #[inline]
    #[must_use]
    pub fn new(max_capacity: u64) -> Self {
        Self {
            inner: Cache::new(max_capacity),
        }
    }

    /// Create cache with time-based expiration
    #[inline]
    #[must_use]
    pub fn with_ttl(max_capacity: u64, ttl: Duration) -> Self {
        Self {
            inner: Cache::builder()
                .max_capacity(max_capacity)
                .time_to_live(ttl)
                .build(),
        }
    }

    /// Insert artifact into cache
    #[inline]
    pub async fn insert<T: ArtifactType>(&self, hash: ContentHash, artifact: Artifact<T>) {
        self.inner.insert(hash, Arc::new(artifact)).await;
    }

    /// Get artifact from cache
    #[inline]
    #[must_use]
    pub async fn get<T: ArtifactType>(&self, hash: &ContentHash) -> Option<Artifact<T>> {
        self.inner
            .get(hash)
            .await
            .and_then(|arc| arc.downcast_ref::<Artifact<T>>().cloned())
    }

    /// Get or compute artifact
    pub async fn get_or_insert_with<T, F, Fut>(
        &self,
        hash: ContentHash,
        f: F,
    ) -> Artifact<T>
    where
        T: ArtifactType,
        F: FnOnce() -> Fut,
        Fut: Future<Output = Artifact<T>>,
    {
        // Check cache first
        if let Some(cached) = self.get::<T>(&hash).await {
            return cached;
        }

        // Compute the artifact
        let artifact = f().await;

        // Insert into cache
        self.insert(hash, artifact.clone()).await;

        artifact
    }

    /// Try to get or compute artifact
    pub async fn try_get_or_insert_with<T, E, F, Fut>(
        &self,
        hash: ContentHash,
        f: F,
    ) -> Result<Artifact<T>, E>
    where
        T: ArtifactType,
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<Artifact<T>, E>>,
    {
        // First try to get from cache
        if let Some(cached) = self.get::<T>(&hash).await {
            return Ok(cached);
        }

        // Compute the artifact
        let artifact = f().await?;

        // Insert into cache
        self.insert(hash, artifact.clone()).await;

        Ok(artifact)
    }

    /// Invalidate cache entry
    #[inline]
    pub async fn invalidate(&self, hash: &ContentHash) {
        self.inner.invalidate(hash).await;
    }

    /// Invalidate all entries
    #[inline]
    pub fn invalidate_all(&self) {
        self.inner.invalidate_all();
    }

    /// Check if cache contains hash
    #[inline]
    #[must_use]
    pub async fn contains(&self, hash: &ContentHash) -> bool {
        self.inner.get(hash).await.is_some()
    }

    /// Get cache statistics
    #[inline]
    #[must_use]
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            entry_count: self.inner.entry_count(),
        }
    }

    /// Get approximate entry count
    #[inline]
    #[must_use]
    pub fn entry_count(&self) -> u64 {
        self.inner.entry_count()
    }
}

impl Default for ArtifactCache {
    /// Create cache with default capacity (10,000 entries)
    fn default() -> Self {
        Self::new(10_000)
    }
}

/// Type-aware cache key for type-safe caching
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypedCacheKey {
    hash: ContentHash,
    type_id: TypeId,
}

impl TypedCacheKey {
    /// Create typed key for artifact type
    #[inline]
    #[must_use]
    pub fn new<T: ArtifactType>(hash: ContentHash) -> Self {
        Self {
            hash,
            type_id: TypeId::of::<T>(),
        }
    }

    /// Get content hash
    #[inline]
    #[must_use]
    pub fn hash(&self) -> &ContentHash {
        &self.hash
    }

    /// Get type ID
    #[inline]
    #[must_use]
    pub fn type_id(&self) -> TypeId {
        self.type_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use coa_artifact::{ArtifactType, ContentHash};

    #[derive(Debug, Clone)]
    struct TestArtifact;

    #[derive(Debug, Clone, PartialEq)]
    struct TestContent {
        data: String,
    }

    impl coa_artifact::__private::Sealed for TestArtifact {}

    impl ArtifactType for TestArtifact {
        type Content = TestContent;

        fn hash(content: &Self::Content) -> ContentHash {
            ContentHash::compute(content.data.as_bytes())
        }

        const TYPE_ID: &'static str = "test";
    }

    #[tokio::test]
    async fn cache_insert_and_get() {
        let cache = ArtifactCache::new(100);
        let content = TestContent {
            data: "test content".to_string(),
        };
        let artifact = Artifact::new(content).unwrap();
        let hash = *artifact.hash();

        cache.insert(hash, artifact.clone()).await;

        let retrieved = cache.get::<TestArtifact>(&hash).await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().hash(), &hash);
    }

    #[tokio::test]
    async fn cache_returns_none_for_missing() {
        let cache = ArtifactCache::new(100);
        let hash = ContentHash::compute(b"missing");

        let retrieved = cache.get::<TestArtifact>(&hash).await;
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn cache_get_or_insert_with() {
        let cache = ArtifactCache::new(100);
        let hash = ContentHash::compute(b"compute me");

        let call_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let call_count_clone = call_count.clone();

        let artifact = cache
            .get_or_insert_with(hash, || async {
                call_count_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let content = TestContent {
                    data: "computed".to_string(),
                };
                Artifact::new(content).unwrap()
            })
            .await;

        assert_eq!(artifact.content().data, "computed");
        assert_eq!(call_count.load(std::sync::atomic::Ordering::SeqCst), 1);

        // Second call should use cache
        let artifact2 = cache
            .get_or_insert_with(hash, || async {
                call_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                unreachable!("should use cached value")
            })
            .await;

        assert_eq!(artifact2.content().data, "computed");
        assert_eq!(call_count.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn cache_invalidation() {
        let cache = ArtifactCache::new(100);
        let content = TestContent {
            data: "test".to_string(),
        };
        let artifact = Artifact::new(content).unwrap();
        let hash = *artifact.hash();

        cache.insert(hash, artifact).await;
        assert!(cache.contains(&hash).await);

        cache.invalidate(&hash).await;
        assert!(!cache.contains(&hash).await);
    }

    #[tokio::test]
    async fn cache_stats() {
        let cache = ArtifactCache::new(100);

        for i in 0..5 {
            let content = TestContent {
                data: format!("content {}", i),
            };
            let artifact = Artifact::new(content).unwrap();
            cache.insert(*artifact.hash(), artifact).await;
        }

        let stats = cache.stats();
        assert_eq!(stats.entry_count, 5);
    }

    #[tokio::test]
    async fn cache_default_capacity() {
        let cache = ArtifactCache::default();
        let content = TestContent {
            data: "test".to_string(),
        };
        let artifact = Artifact::new(content).unwrap();
        let hash = *artifact.hash();

        cache.insert(hash, artifact).await;
        assert!(cache.get::<TestArtifact>(&hash).await.is_some());
    }

    #[test]
    fn typed_cache_key_creation() {
        let hash = ContentHash::compute(b"test");
        let key = TypedCacheKey::new::<TestArtifact>(hash);

        assert_eq!(key.hash(), &hash);
        assert_eq!(key.type_id(), TypeId::of::<TestArtifact>());
    }
}
