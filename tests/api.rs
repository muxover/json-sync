use json_sync::{FlushPolicy, JsonSync};
use shardmap::ShardMap;
use std::time::Duration;

fn temp_path(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("json_sync_test_{}.json", name))
}

// ---- clear ------------------------------------------------------------------

#[test]
fn clear_removes_all_entries() {
    let path = temp_path("clear");
    let _ = std::fs::remove_file(&path);
    let db = JsonSync::<String, i32, ShardMap<String, i32>>::open(&path).unwrap();
    db.insert("a".into(), 1).unwrap();
    db.insert("b".into(), 2).unwrap();
    assert_eq!(db.len(), 2);

    db.clear().unwrap();
    assert!(db.is_empty());
    assert_eq!(db.get(&"a".into()), None);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn clear_on_empty_store_is_fine() {
    let path = temp_path("clear_empty");
    let _ = std::fs::remove_file(&path);
    let db = JsonSync::<String, i32, ShardMap<String, i32>>::open(&path).unwrap();
    db.clear().unwrap();
    assert!(db.is_empty());
    let _ = std::fs::remove_file(&path);
}

// ---- keys / values ----------------------------------------------------------

#[test]
fn keys_and_values() {
    let path = temp_path("keys_vals");
    let _ = std::fs::remove_file(&path);
    let db = JsonSync::<String, i32, ShardMap<String, i32>>::open(&path).unwrap();
    db.insert("x".into(), 10).unwrap();
    db.insert("y".into(), 20).unwrap();

    let mut keys = db.keys();
    keys.sort();
    assert_eq!(keys, vec!["x".to_string(), "y".to_string()]);

    let mut vals = db.values();
    vals.sort();
    assert_eq!(vals, vec![10, 20]);
    let _ = std::fs::remove_file(&path);
}

// ---- extend -----------------------------------------------------------------

#[test]
fn extend_bulk_insert() {
    let path = temp_path("extend");
    let _ = std::fs::remove_file(&path);
    let db = JsonSync::<String, i32, ShardMap<String, i32>>::open(&path).unwrap();

    let batch: Vec<(String, i32)> = (0..50).map(|i| (format!("k{i}"), i)).collect();
    db.extend(batch).unwrap();
    assert_eq!(db.len(), 50);
    assert_eq!(db.get(&"k0".into()), Some(0));
    assert_eq!(db.get(&"k49".into()), Some(49));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn extend_overwrites_existing() {
    let path = temp_path("extend_overwrite");
    let _ = std::fs::remove_file(&path);
    let db = JsonSync::<String, i32, ShardMap<String, i32>>::open(&path).unwrap();
    db.insert("a".into(), 1).unwrap();

    db.extend(vec![("a".into(), 99), ("b".into(), 2)]).unwrap();
    assert_eq!(db.get(&"a".into()), Some(99));
    assert_eq!(db.get(&"b".into()), Some(2));
    let _ = std::fs::remove_file(&path);
}

// ---- update -----------------------------------------------------------------

#[test]
fn update_existing_key() {
    let path = temp_path("update_exists");
    let _ = std::fs::remove_file(&path);
    let db = JsonSync::<String, i32, ShardMap<String, i32>>::open(&path).unwrap();
    db.insert("counter".into(), 10).unwrap();

    let found = db.update(&"counter".into(), |v| *v += 5).unwrap();
    assert!(found);
    assert_eq!(db.get(&"counter".into()), Some(15));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn update_missing_key_returns_false() {
    let path = temp_path("update_missing");
    let _ = std::fs::remove_file(&path);
    let db = JsonSync::<String, i32, ShardMap<String, i32>>::open(&path).unwrap();

    let found = db.update(&"nope".into(), |v| *v += 1).unwrap();
    assert!(!found);
    assert!(db.is_empty());
    let _ = std::fs::remove_file(&path);
}

// ---- get_or_insert ----------------------------------------------------------

#[test]
fn get_or_insert_when_present() {
    let path = temp_path("goi_present");
    let _ = std::fs::remove_file(&path);
    let db = JsonSync::<String, i32, ShardMap<String, i32>>::open(&path).unwrap();
    db.insert("key".into(), 42).unwrap();

    let val = db.get_or_insert("key".into(), 999).unwrap();
    assert_eq!(val, 42);
    assert_eq!(db.len(), 1);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn get_or_insert_when_absent() {
    let path = temp_path("goi_absent");
    let _ = std::fs::remove_file(&path);
    let db = JsonSync::<String, i32, ShardMap<String, i32>>::open(&path).unwrap();

    let val = db.get_or_insert("key".into(), 42).unwrap();
    assert_eq!(val, 42);
    assert_eq!(db.get(&"key".into()), Some(42));
    let _ = std::fs::remove_file(&path);
}

// ---- get_or_insert_with -----------------------------------------------------

#[test]
fn get_or_insert_with_when_present() {
    let path = temp_path("goiw_present");
    let _ = std::fs::remove_file(&path);
    let db = JsonSync::<String, i32, ShardMap<String, i32>>::open(&path).unwrap();
    db.insert("key".into(), 10).unwrap();

    let val = db
        .get_or_insert_with("key".into(), || panic!("should not be called"))
        .unwrap();
    assert_eq!(val, 10);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn get_or_insert_with_when_absent() {
    let path = temp_path("goiw_absent");
    let _ = std::fs::remove_file(&path);
    let db = JsonSync::<String, i32, ShardMap<String, i32>>::open(&path).unwrap();

    let val = db.get_or_insert_with("key".into(), || 7 * 6).unwrap();
    assert_eq!(val, 42);
    assert_eq!(db.get(&"key".into()), Some(42));
    let _ = std::fs::remove_file(&path);
}

// ---- builder ----------------------------------------------------------------

#[test]
fn builder_pretty_json() {
    let path = temp_path("builder_pretty");
    let _ = std::fs::remove_file(&path);

    let db = JsonSync::<String, i32, ShardMap<String, i32>>::builder(&path)
        .pretty(true)
        .build()
        .unwrap();
    db.insert("hello".into(), 1).unwrap();
    db.flush().unwrap();

    let raw = std::fs::read_to_string(&path).unwrap();
    // pretty JSON has newlines and indentation
    assert!(raw.contains('\n'));
    assert!(raw.contains("  "));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn builder_compact_json() {
    let path = temp_path("builder_compact");
    let _ = std::fs::remove_file(&path);

    let db = JsonSync::<String, i32, ShardMap<String, i32>>::builder(&path)
        .pretty(false)
        .build()
        .unwrap();
    db.insert("hello".into(), 1).unwrap();
    db.flush().unwrap();

    let raw = std::fs::read_to_string(&path).unwrap();
    // compact JSON fits on one line
    assert!(!raw.contains('\n'));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn builder_with_policy() {
    let path = temp_path("builder_policy");
    let _ = std::fs::remove_file(&path);

    let db = JsonSync::<String, i32, ShardMap<String, i32>>::builder(&path)
        .policy(FlushPolicy::Immediate)
        .build()
        .unwrap();
    db.insert("x".into(), 1).unwrap();
    drop(db);

    // immediate policy should have flushed on insert
    let db2 = JsonSync::<String, i32, ShardMap<String, i32>>::open(&path).unwrap();
    assert_eq!(db2.get(&"x".into()), Some(1));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn builder_with_async_policy() {
    let path = temp_path("builder_async");
    let _ = std::fs::remove_file(&path);

    let db = JsonSync::<String, i32, ShardMap<String, i32>>::builder(&path)
        .policy(FlushPolicy::Async(Duration::from_secs(60)))
        .build()
        .unwrap();
    db.insert("z".into(), 99).unwrap();
    db.flush().unwrap();
    drop(db);
    let _ = std::fs::remove_file(&path);
}

// ---- debug ------------------------------------------------------------------

#[test]
fn debug_impls_dont_panic() {
    let path = temp_path("debug");
    let _ = std::fs::remove_file(&path);
    let handle = JsonSync::<String, i32, ShardMap<String, i32>>::open(&path).unwrap();

    let dbg_store = format!("{:?}", *handle);
    assert!(dbg_store.contains("JsonSync"));
    assert!(dbg_store.contains("path"));

    let dbg_handle = format!("{:?}", handle);
    assert!(dbg_handle.contains("JsonSync"));

    let builder = JsonSync::<String, i32, ShardMap<String, i32>>::builder(&path);
    let dbg_builder = format!("{:?}", builder);
    assert!(dbg_builder.contains("JsonSyncBuilder"));

    let _ = std::fs::remove_file(&path);
}

// ---- clear + flush integration ----------------------------------------------

#[test]
fn clear_then_flush_persists_empty() {
    let path = temp_path("clear_flush");
    let _ = std::fs::remove_file(&path);
    {
        let db = JsonSync::<String, i32, ShardMap<String, i32>>::open(&path).unwrap();
        db.insert("a".into(), 1).unwrap();
        db.flush().unwrap();
        db.clear().unwrap();
        db.flush().unwrap();
    }
    let db = JsonSync::<String, i32, ShardMap<String, i32>>::open(&path).unwrap();
    assert!(db.is_empty());
    let _ = std::fs::remove_file(&path);
}
