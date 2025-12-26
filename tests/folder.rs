use reactor::Runtime;
use reactor::fs::Folder;

use std::fs;
use std::io;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_temp_base() -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let base = std::env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let pid = std::process::id();
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);

    base.join(format!("reactor_folder_test_{}_{}_{}", pid, nanos, seq))
}

#[test]
fn folder_create_single() {
    let mut rt = Runtime::new();

    let base = unique_temp_base();
    let base_str = base.to_string_lossy().into_owned();

    let base_for_async = base_str.clone();
    rt.block_on(async move {
        let folder = Folder::create(&base_for_async)
            .await
            .expect("create single");
        assert_eq!(folder.path(), base_for_async);
    });

    let meta = fs::metadata(&base_str).expect("metadata");
    assert!(meta.is_dir());

    fs::remove_dir(&base_str).expect("cleanup");
}

#[test]
fn folder_create_all_nested_and_idempotent() {
    let mut rt = Runtime::new();

    let base = unique_temp_base();
    let nested = base.join("a").join("b").join("c");
    let base_str = base.to_string_lossy().into_owned();
    let nested_str = nested.to_string_lossy().into_owned();

    let nested_for_async = nested_str.clone();
    rt.block_on(async move {
        let folder = Folder::create_all(&nested_for_async)
            .await
            .expect("create_all");
        assert_eq!(folder.path(), nested_for_async);

        // Idempotent: creating again should succeed
        Folder::create_all(&nested_for_async)
            .await
            .expect("create_all idempotent");
    });

    let meta = fs::metadata(&nested_str).expect("metadata nested");
    assert!(meta.is_dir());

    // Cleanup entire tree
    fs::remove_dir_all(&base_str).expect("cleanup nested");
}

#[test]
fn folder_create_fails_when_exists() {
    let mut rt = Runtime::new();

    let base = unique_temp_base();
    let base_str = base.to_string_lossy().into_owned();

    let base_for_async = base_str.clone();
    rt.block_on(async move {
        Folder::create(&base_for_async).await.expect("first create");

        let err = Folder::create(&base_for_async)
            .await
            .err()
            .expect("expected error");
        assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);
    });

    fs::remove_dir(&base_str).expect("cleanup");
}

#[test]
fn folder_exists_api() {
    let mut rt = Runtime::new();

    let base = unique_temp_base();
    let base_str = base.to_string_lossy().into_owned();

    let base_for_async = base_str.clone();
    let folder: Folder = rt.block_on(async move {
        let f = Folder::create(&base_for_async).await.expect("create");
        assert!(f.exists());

        f
    });

    // After deletion, exists() should be false
    fs::remove_dir(&base_str).expect("cleanup");

    assert!(!folder.exists());
}
