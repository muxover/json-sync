use json_sync::JsonSync;
use shardmap::ShardMap;

fn temp_path(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("json_sync_test_{}.json", name))
}

#[test]
fn open_missing_file_creates_empty() {
    let path = temp_path("missing");
    let _ = std::fs::remove_file(&path);
    let db = JsonSync::<String, i32, ShardMap<String, i32>>::open(&path).unwrap();
    assert!(db.iter().is_empty());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn path_accessor() {
    let path = temp_path("path_acc");
    let _ = std::fs::remove_file(&path);
    let db = JsonSync::<String, i32, ShardMap<String, i32>>::open(&path).unwrap();
    assert_eq!(db.path(), path.as_path());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn persist_and_reload_roundtrip() {
    let path = temp_path("roundtrip");
    let _ = std::fs::remove_file(&path);
    {
        let db = JsonSync::<String, String, ShardMap<String, String>>::open(&path).unwrap();
        db.insert("k1".into(), "v1".into()).unwrap();
        db.insert("k2".into(), "v2".into()).unwrap();
        db.flush().unwrap();
    }
    let db = JsonSync::<String, String, ShardMap<String, String>>::open(&path).unwrap();
    assert_eq!(db.get(&"k1".into()), Some("v1".into()));
    assert_eq!(db.get(&"k2".into()), Some("v2".into()));
    let _ = std::fs::remove_file(&path);
}
