use json_sync::{JsonSyncBuilder, DirtyStrategy};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

#[test]
fn test_per_key_tracking() {
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
        .path("test_per_key.json")
        .manual_flush()
        .dirty_strategy(DirtyStrategy::PerKey)
        .build::<String, i32>()
        .unwrap();

    sync.attach(get_data, load_data).unwrap();
    
    // Mark multiple keys as dirty
    sync.mark_dirty("key1");
    sync.mark_dirty("key2");
    sync.mark_dirty("key3");
    
    let stats = sync.stats();
    assert_eq!(stats.dirty_count, 3);
    
    // Clear and verify
    sync.flush().unwrap();
    let stats = sync.stats();
    assert_eq!(stats.dirty_count, 0);
}

#[test]
fn test_per_shard_tracking() {
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
        .path("test_per_shard.json")
        .manual_flush()
        .dirty_strategy(DirtyStrategy::PerShard)
        .shard_count(16)
        .build::<String, i32>()
        .unwrap();

    sync.attach(get_data, load_data).unwrap();
    
    // Mark multiple shards as dirty
    sync.mark_shard_dirty(0);
    sync.mark_shard_dirty(1);
    sync.mark_shard_dirty(2);
    
    let stats = sync.stats();
    assert_eq!(stats.dirty_count, 3);
    
    // Clear and verify
    sync.flush().unwrap();
    let stats = sync.stats();
    assert_eq!(stats.dirty_count, 0);
}

#[test]
fn test_concurrent_dirty_marking() {
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

    let sync = Arc::new(JsonSyncBuilder::new()
        .path("test_concurrent.json")
        .manual_flush()
        .dirty_strategy(DirtyStrategy::PerKey)
        .build::<String, i32>()
        .unwrap());

    sync.attach(get_data, load_data).unwrap();
    
    // Spawn multiple threads marking keys as dirty
    let mut handles = vec![];
    for i in 0..10 {
        let sync = Arc::clone(&sync);
        let handle = std::thread::spawn(move || {
            for j in 0..10 {
                sync.mark_dirty(&format!("key_{}_{}", i, j));
            }
        });
        handles.push(handle);
    }
    
    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }
    
    // Verify all keys were marked
    let stats = sync.stats();
    assert_eq!(stats.dirty_count, 100);
}

