//! Tests for key derivation functionality

use crate::backends::memory::MemoryBackend;
use crate::backends::CacheBackend;
use crate::key_derivation::generate_compile_time_key;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_runtime_vs_compile_time_keys() {
        let rt_key = generate_runtime_key("test_function", &[&10, &"hello"]);
        let ct_key = generate_compile_time_key(
            "test_function",
            "fncache::key_derivation_tests::tests",
            &["i32", "&str"],
            "String",
        );

        assert_ne!(rt_key, ct_key.to_string());
    }

    #[tokio::test]
    async fn test_generate_compile_time_key_consistency() {
        let key1 = generate_compile_time_key(
            "my_function",
            "fncache::test",
            &["String", "i32", "bool"],
            "Result<Vec<u8>, Error>",
        );

        let key2 = generate_compile_time_key(
            "my_function",
            "fncache::test",
            &["String", "i32", "bool"],
            "Result<Vec<u8>, Error>",
        );

        assert_eq!(key1, key2);

        let key3 = generate_compile_time_key(
            "other_function",
            "fncache::test",
            &["String", "i32", "bool"],
            "Result<Vec<u8>, Error>",
        );

        assert_ne!(key1, key3);

        let key4 = generate_compile_time_key(
            "my_function",
            "fncache::other_module",
            &["String", "i32", "bool"],
            "Result<Vec<u8>, Error>",
        );

        assert_ne!(key1, key4);
    }

    fn generate_runtime_key(fn_name: &str, args: &[&dyn std::fmt::Debug]) -> String {
        let args_str = format!("{:?}", args);
        format!("{}-{}", fn_name, args_str)
    }

    #[tokio::test]
    async fn test_key_derivation_with_backend() {
        let backend = MemoryBackend::new();
        let runtime_key = generate_runtime_key("my_test", &[&42, &"test"]);
        backend
            .set(runtime_key.clone(), vec![1, 2, 3], None)
            .await
            .unwrap();

        let compile_time_key = generate_compile_time_key(
            "my_test",
            "fncache::key_derivation_tests",
            &["i32", "&str"],
            "Vec<u8>",
        )
        .to_string();
        backend
            .set(compile_time_key.clone(), vec![4, 5, 6], None)
            .await
            .unwrap();

        let rt_value = backend.get(&runtime_key).await.unwrap();
        let ct_value = backend.get(&compile_time_key).await.unwrap();

        assert_eq!(rt_value, Some(vec![1, 2, 3]));
        assert_eq!(ct_value, Some(vec![4, 5, 6]));
        assert_ne!(runtime_key, compile_time_key);
    }
}
