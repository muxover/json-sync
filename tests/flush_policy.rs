//! Integration tests: flush policies (Immediate, Manual, Async).

use json_sync::{JsonSync, FlushPolicy};
use shardmap::ShardMap;
use std::time::Duration;

fn temp_path(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("json_sync_test_{}.json", name))
}

#[test]
fn manual_flush_only_on_call() {
    let path = temp_path("manual");
    let _ = std::fs::remove_file(&path);
    let db = JsonSync::<String, i32, ShardMap<String, i32>>::open(&path).unwrap();
    db.insert("a".to_string(), 1).unwrap();
    // File may not exist or be empty until flush
    db.flush().unwrap();
    drop(db);
    let db2 = JsonSync::<String, i32, ShardMap<String, i32>>::open(&path).unwrap();
    assert_eq!(db2.get(&"a".to_string()), Some(1));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn immediate_flush_after_mutate() {
    let path = temp_path("immediate");
    let _ = std::fs::remove_file(&path);
    let db = JsonSync::<String, i32, ShardMap<String, i32>>::open_with_policy(
        &path,
        FlushPolicy::Immediate,
    )
    .unwrap();
    db.insert("x".to_string(), 42).unwrap();
    drop(db);
    let db2 = JsonSync::<String, i32, ShardMap<String, i32>>::open(&path).unwrap();
    assert_eq!(db2.get(&"x".to_string()), Some(42));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn async_flush_worker_graceful_drop() {
    let path = temp_path("async_drop");
    let _ = std::fs::remove_file(&path);
    let db = JsonSync::<String, i32, ShardMap<String, i32>>::open_with_policy(
        &path,
        FlushPolicy::Async(Duration::from_secs(60)),
    )
    .unwrap();
    db.insert("q".to_string(), 7).unwrap();
    db.flush().unwrap();
    drop(db);
    let _ = std::fs::remove_file(&path);
}
