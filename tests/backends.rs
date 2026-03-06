use json_sync::JsonSync;
use parking_lot::RwLock;
use shardmap::ShardMap;
use std::collections::HashMap;

fn temp_path(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("json_sync_test_{}.json", name))
}

#[test]
fn shardmap_insert_get_remove_flush_persist() {
    let path = temp_path("sm_persist");
    let _ = std::fs::remove_file(&path);

    {
        let db = JsonSync::<String, i32, ShardMap<String, i32>>::open(&path).unwrap();
        assert!(db.insert("a".into(), 1).unwrap().is_none());
        assert_eq!(db.get(&"a".into()), Some(1));
        assert_eq!(db.insert("a".into(), 2).unwrap(), Some(1));
        assert_eq!(db.get(&"a".into()), Some(2));
        assert_eq!(db.remove(&"a".into()).unwrap(), Some(2));
        assert_eq!(db.get(&"a".into()), None);
        db.insert("b".into(), 3).unwrap();
        db.flush().unwrap();
    }

    let db2 = JsonSync::<String, i32, ShardMap<String, i32>>::open(&path).unwrap();
    assert_eq!(db2.get(&"a".into()), None);
    assert_eq!(db2.get(&"b".into()), Some(3));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn shardmap_iter_snapshot() {
    let path = temp_path("sm_iter");
    let _ = std::fs::remove_file(&path);
    let db = JsonSync::<String, u8, ShardMap<String, u8>>::open(&path).unwrap();
    db.insert("x".into(), 10).unwrap();
    db.insert("y".into(), 20).unwrap();
    let mut entries = db.iter();
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(entries, vec![("x".into(), 10), ("y".into(), 20)]);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn len_contains_key_is_empty() {
    let path = temp_path("len_contains");
    let _ = std::fs::remove_file(&path);
    let db = JsonSync::<String, i32, ShardMap<String, i32>>::open(&path).unwrap();

    assert!(db.is_empty());
    assert_eq!(db.len(), 0);
    assert!(!db.contains_key(&"a".into()));

    db.insert("a".into(), 1).unwrap();
    assert!(!db.is_empty());
    assert_eq!(db.len(), 1);
    assert!(db.contains_key(&"a".into()));
    assert!(!db.contains_key(&"z".into()));

    db.insert("b".into(), 2).unwrap();
    assert_eq!(db.len(), 2);
    db.remove(&"a".into()).unwrap();
    assert_eq!(db.len(), 1);
    assert!(!db.contains_key(&"a".into()));
    assert!(db.contains_key(&"b".into()));

    db.remove(&"b".into()).unwrap();
    assert!(db.is_empty());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rwlock_hashmap_crud() {
    let path = temp_path("rwlock");
    let _ = std::fs::remove_file(&path);
    let db = JsonSync::<String, i32, RwLock<HashMap<String, i32>>>::open(&path).unwrap();
    db.insert("k".into(), 100).unwrap();
    assert_eq!(db.get(&"k".into()), Some(100));
    assert!(db.contains_key(&"k".into()));
    assert_eq!(db.len(), 1);
    db.remove(&"k".into()).unwrap();
    assert!(db.is_empty());
    db.flush().unwrap();
    let _ = std::fs::remove_file(&path);
}

#[cfg(feature = "dashmap")]
mod dashmap_tests {
    use super::temp_path;
    use dashmap::DashMap;
    use json_sync::JsonSync;

    #[test]
    fn dashmap_crud() {
        let path = temp_path("dashmap_crud");
        let _ = std::fs::remove_file(&path);
        let db = JsonSync::<String, i32, DashMap<String, i32>>::open(&path).unwrap();
        db.insert("a".into(), 1).unwrap();
        assert_eq!(db.get(&"a".into()), Some(1));
        assert!(db.contains_key(&"a".into()));
        assert_eq!(db.len(), 1);
        db.remove(&"a".into()).unwrap();
        assert!(db.is_empty());
        db.flush().unwrap();
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn dashmap_iter_snapshot() {
        let path = temp_path("dashmap_iter");
        let _ = std::fs::remove_file(&path);
        let db = JsonSync::<String, u8, DashMap<String, u8>>::open(&path).unwrap();
        db.insert("p".into(), 1).unwrap();
        db.insert("q".into(), 2).unwrap();
        let mut entries = db.iter();
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        assert_eq!(entries, vec![("p".into(), 1), ("q".into(), 2)]);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn dashmap_persist_and_reload() {
        let path = temp_path("dashmap_persist");
        let _ = std::fs::remove_file(&path);
        {
            let db = JsonSync::<String, String, DashMap<String, String>>::open(&path).unwrap();
            db.insert("key".into(), "val".into()).unwrap();
            db.flush().unwrap();
        }
        let db = JsonSync::<String, String, DashMap<String, String>>::open(&path).unwrap();
        assert_eq!(db.get(&"key".into()), Some("val".into()));
        let _ = std::fs::remove_file(&path);
    }
}
