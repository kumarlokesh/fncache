#[cfg(all(test, feature = "test-utils", feature = "memory"))]
mod tests {
    use super::*;
    use crate::{
        backends::memory::MemoryBackend,
        invalidation::{AsyncCacheInvalidation, CacheInvalidation, InvalidationCache, Tag},
    };
    use std::time::Duration;
    use tokio::runtime::Runtime;

    #[test]
    fn test_register_with_tags() {
        let cache = InvalidationCache::new(MemoryBackend::new());
        let key = "user:123".to_string();
        let tags = vec![Tag::new("user"), Tag::new("profile")];

        // Register key with tags
        cache.register_key_with_tags(&key, tags.clone());

        // Check internal state through getter methods
        let tag_map = cache.get_tag_map();
        assert!(tag_map.contains_key(&Tag::new("user")));
        assert!(tag_map.contains_key(&Tag::new("profile")));

        let user_keys = tag_map.get(&Tag::new("user")).unwrap();
        assert!(user_keys.contains(&key));

        let profile_keys = tag_map.get(&Tag::new("profile")).unwrap();
        assert!(profile_keys.contains(&key));
    }

    #[test]
    fn test_register_with_prefixes() {
        let cache = InvalidationCache::new(MemoryBackend::new());
        let key = "users:123:profile".to_string();

        // Register key (this automatically registers prefixes)
        cache.register_key_with_tags(&key, Vec::<Tag>::new());

        // Check prefixes are registered
        let prefix_map = cache.get_prefix_map();
        assert!(prefix_map.contains_key(&"users".to_string()));
        assert!(prefix_map.contains_key(&"users:123".to_string()));

        let users_keys = prefix_map.get(&"users".to_string()).unwrap();
        assert!(users_keys.contains(&key));

        let user_123_keys = prefix_map.get(&"users:123".to_string()).unwrap();
        assert!(user_123_keys.contains(&key));
    }

    #[tokio::test]
    async fn test_invalidate_tag() {
        let cache = InvalidationCache::new(MemoryBackend::new());
        let key1 = "user:123".to_string();
        let key2 = "user:456".to_string();

        // Store values with tags
        cache
            .set_with_tags(key1.clone(), "value1", None, vec![Tag::new("user")])
            .await
            .unwrap();

        cache
            .set_with_tags(
                key2.clone(),
                "value2",
                None,
                vec![Tag::new("user"), Tag::new("vip")],
            )
            .await
            .unwrap();

        // Verify values exist
        assert_eq!(
            cache.get::<String>(&key1).await.unwrap(),
            Some("value1".to_string())
        );
        assert_eq!(
            cache.get::<String>(&key2).await.unwrap(),
            Some("value2".to_string())
        );

        // Invalidate by tag
        AsyncCacheInvalidation::invalidate_tag(&cache, &Tag::new("user"))
            .await
            .unwrap();

        // Verify values are gone
        assert_eq!(cache.get::<String>(&key1).await.unwrap(), None);
        assert_eq!(cache.get::<String>(&key2).await.unwrap(), None);
    }

    #[tokio::test]
    async fn test_invalidate_prefix() {
        let cache = InvalidationCache::new(MemoryBackend::new());
        let key1 = "users:123:profile".to_string();
        let key2 = "users:123:settings".to_string();
        let key3 = "users:456:profile".to_string();

        // Store values
        cache
            .set_with_tags(key1.clone(), "profile1", None, Vec::<Tag>::new())
            .await
            .unwrap();
        cache
            .set_with_tags(key2.clone(), "settings1", None, Vec::<Tag>::new())
            .await
            .unwrap();
        cache
            .set_with_tags(key3.clone(), "profile2", None, Vec::<Tag>::new())
            .await
            .unwrap();

        // Verify values exist
        assert_eq!(
            cache.get::<String>(&key1).await.unwrap(),
            Some("profile1".to_string())
        );
        assert_eq!(
            cache.get::<String>(&key2).await.unwrap(),
            Some("settings1".to_string())
        );
        assert_eq!(
            cache.get::<String>(&key3).await.unwrap(),
            Some("profile2".to_string())
        );

        // Invalidate by prefix
        AsyncCacheInvalidation::invalidate_prefix(&cache, "users:123")
            .await
            .unwrap();

        // Verify specific keys are gone, but others remain
        assert_eq!(cache.get::<String>(&key1).await.unwrap(), None);
        assert_eq!(cache.get::<String>(&key2).await.unwrap(), None);
        assert_eq!(
            cache.get::<String>(&key3).await.unwrap(),
            Some("profile2".to_string())
        );
    }

    #[test]
    fn test_sync_invalidate_tag() {
        let cache = InvalidationCache::new(MemoryBackend::new());
        let key1 = "user:123".to_string();
        let key2 = "user:456".to_string();
        let rt = Runtime::new().unwrap();

        // Store values with tags
        rt.block_on(async {
            cache
                .set_with_tags(key1.clone(), "value1", None, vec![Tag::new("user")])
                .await
                .unwrap();

            cache
                .set_with_tags(
                    key2.clone(),
                    "value2",
                    None,
                    vec![Tag::new("user"), Tag::new("vip")],
                )
                .await
                .unwrap();

            // Verify values exist
            assert_eq!(
                cache.get::<String>(&key1).await.unwrap(),
                Some("value1".to_string())
            );
            assert_eq!(
                cache.get::<String>(&key2).await.unwrap(),
                Some("value2".to_string())
            );
        });

        // Invalidate by tag using sync API
        CacheInvalidation::invalidate_tag(&cache, &Tag::new("user")).unwrap();

        // Verify values are gone
        rt.block_on(async {
            assert_eq!(cache.get::<String>(&key1).await.unwrap(), None);
            assert_eq!(cache.get::<String>(&key2).await.unwrap(), None);
        });
    }
}
