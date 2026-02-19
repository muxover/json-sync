//! Integration tests: insert/get/remove/flush/iter across backends (ShardMap, RwLock<HashMap>).

use json_sync::JsonSync;
use parking_lot::RwLock;
use shardmap::ShardMap;
use std::collections::HashMap;

fn temp_path(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("json_sync_test_{}.json", name))
}

#[test]
fn shardmap_insert_get_remove_flush_persist() {
    let path = temp_path("shardmap_persist");
    let _ = std::fs::remove_file(&path);

    {
        let db = JsonSync::<String, i32, ShardMap<String, i32>>::open(&path).unwrap();
        assert!(db.insert("a".to_string(), 1).unwrap().is_none());
        assert_eq!(db.get(&"a".to_string()), Some(1));
        assert_eq!(db.insert("a".to_string(), 2).unwrap(), Some(1));
        assert_eq!(db.get(&"a".to_string()), Some(2));
        assert_eq!(db.remove(&"a".to_string()).unwrap(), Some(2));
        assert_eq!(db.get(&"a".to_string()), None);
        db.insert("b".to_string(), 3).unwrap();
        db.flush().unwrap();
    }

    let db2 = JsonSync::<String, i32, ShardMap<String, i32>>::open(&path).unwrap();
    assert_eq!(db2.get(&"a".to_string()), None);
    assert_eq!(db2.get(&"b".to_string()), Some(3));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn shardmap_iter_snapshot() {
    let path = temp_path("shardmap_iter");
    let _ = std::fs::remove_file(&path);
    let db = JsonSync::<String, u8, ShardMap<String, u8>>::open(&path).unwrap();
    db.insert("x".to_string(), 10).unwrap();
    db.insert("y".to_string(), 20).unwrap();
    let mut entries: Vec<_> = db.iter();
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(entries, vec![("x".to_string(), 10), ("y".to_string(), 20)]);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rwlock_hashmap_insert_get_remove_flush() {
    let path = temp_path("rwlock");
    let _ = std::fs::remove_file(&path);
    let db =
        JsonSync::<String, i32, RwLock<HashMap<String, i32>>>::open(&path).unwrap();
    db.insert("k".to_string(), 100).unwrap();
    assert_eq!(db.get(&"k".to_string()), Some(100));
    db.remove(&"k".to_string()).unwrap();
    db.flush().unwrap();
    let _ = std::fs::remove_file(&path);
}
