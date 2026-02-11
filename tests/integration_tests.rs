use json_sync::{JsonSyncBuilder, DirtyStrategy};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

#[test]
fn test_basic_attach_and_flush() {
    let map = Arc::new(RwLock::new(HashMap::new()));
    
    let get_data = {
        let map = Arc::clone(&map);
        Arc::new(move || {
            map.read().clone()
        })
    };

    let load_data = {
        let map = Arc::clone(&map);
        Arc::new(move |data: HashMap<String, i32>| {
            *map.write() = data;
        })
    };

    let sync = JsonSyncBuilder::new()
        .path("test_integration.json")
        .manual_flush()
        .build::<String, i32>()
        .unwrap();

    sync.attach(get_data, load_data).unwrap();
    
    // Insert some data
    map.write().insert("key1".to_string(), 1);
    map.write().insert("key2".to_string(), 2);
    
    // Mark as dirty and flush
    sync.mark_dirty("key1");
    sync.mark_dirty("key2");
    assert!(sync.flush().is_ok());
    
    // Verify file exists
    assert!(std::path::Path::new("test_integration.json").exists());
    
    // Clean up
    let _ = std::fs::remove_file("test_integration.json");
}

#[test]
fn test_load_from_file() {
    // Create a test file
    let test_data = r#"{"key1":1,"key2":2}"#;
    std::fs::write("test_load.json", test_data).unwrap();
    
    let map = Arc::new(RwLock::new(HashMap::new()));
    
    let get_data = {
        let map = Arc::clone(&map);
        Arc::new(move || {
            map.read().clone()
        })
    };

    let load_data = {
        let map = Arc::clone(&map);
        Arc::new(move |data: HashMap<String, i32>| {
            *map.write() = data;
        })
    };

    let sync = JsonSyncBuilder::new()
        .path("test_load.json")
        .manual_flush()
        .build::<String, i32>()
        .unwrap();

    sync.attach(get_data, load_data).unwrap();
    assert!(sync.load_from_file().is_ok());
    
    // Verify data was loaded
    assert_eq!(map.read().get("key1"), Some(&1));
    assert_eq!(map.read().get("key2"), Some(&2));
    
    // Clean up
    let _ = std::fs::remove_file("test_load.json");
}

#[test]
fn test_stats() {
    let map = Arc::new(RwLock::new(HashMap::new()));
    
    let get_data = {
        let map = Arc::clone(&map);
        Arc::new(move || {
            map.read().clone()
        })
    };

    let load_data = {
        let map = Arc::clone(&map);
        Arc::new(move |data: HashMap<String, i32>| {
            *map.write() = data;
        })
    };

    let sync = JsonSyncBuilder::new()
        .path("test_stats.json")
        .manual_flush()
        .dirty_strategy(DirtyStrategy::PerKey)  // Use PerKey so mark_dirty works
        .build::<String, i32>()
        .unwrap();

    sync.attach(get_data, load_data).unwrap();
    
    let stats = sync.stats();
    assert_eq!(stats.flush_count, 0);
    assert_eq!(stats.dirty_count, 0);
    
    sync.mark_dirty("key1");
    let stats = sync.stats();
    assert_eq!(stats.dirty_count, 1);
    
    // Insert some data before flushing
    map.write().insert("key1".to_string(), 1);
    sync.flush().unwrap();
    let stats = sync.stats();
    assert_eq!(stats.flush_count, 1);
    assert!(stats.last_flush_time.is_some());
    
    // Clean up
    let _ = std::fs::remove_file("test_stats.json");
}

#[test]
fn test_dirty_strategies() {
    // Test per-key strategy
    let sync = JsonSyncBuilder::new()
        .path("test_per_key.json")
        .manual_flush()
        .dirty_strategy(DirtyStrategy::PerKey)
        .build::<String, i32>()
        .unwrap();
    
    sync.mark_dirty("key1");
    sync.mark_dirty("key2");
    let stats = sync.stats();
    assert_eq!(stats.dirty_count, 2);
    
    // Test per-shard strategy
    let sync = JsonSyncBuilder::new()
        .path("test_per_shard.json")
        .manual_flush()
        .dirty_strategy(DirtyStrategy::PerShard)
        .shard_count(16)
        .build::<String, i32>()
        .unwrap();
    
    sync.mark_shard_dirty(0);
    sync.mark_shard_dirty(1);
    let stats = sync.stats();
    assert_eq!(stats.dirty_count, 2);
}

