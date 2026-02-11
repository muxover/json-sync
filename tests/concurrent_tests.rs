use json_sync::JsonSyncBuilder;
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use std::time::Duration;

#[test]
fn test_concurrent_flush() {
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
        .path("test_concurrent_flush.json")
        .manual_flush()
        .build::<String, i32>()
        .unwrap());

    sync.attach(get_data, load_data).unwrap();
    
    // Insert data
    for i in 0..100 {
        map.write().insert(format!("key{}", i), i);
    }
    
    // Spawn multiple threads trying to flush
    let mut handles = vec![];
    for _ in 0..5 {
        let sync = Arc::clone(&sync);
        let handle = std::thread::spawn(move || {
            sync.flush()
        });
        handles.push(handle);
    }
    
    // At least one flush should succeed
    let mut success_count = 0;
    for handle in handles {
        if handle.join().unwrap().is_ok() {
            success_count += 1;
        }
    }
    assert!(success_count > 0, "At least one flush should succeed");
    
    // Verify file exists
    assert!(std::path::Path::new("test_concurrent_flush.json").exists());
    
    // Clean up
    let _ = std::fs::remove_file("test_concurrent_flush.json");
}

#[tokio::test]
async fn test_high_frequency_updates() {
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
        .path("test_high_freq.json")
        .flush_threshold(100)
        .build::<String, i32>()
        .unwrap());

    sync.attach(get_data, load_data).unwrap();
    sync.start().unwrap();
    
    // Simulate high-frequency updates
    for i in 0..1000 {
        map.write().insert(format!("key{}", i), i);
        sync.mark_dirty(&format!("key{}", i));
    }
    
    // Wait for flush
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    sync.stop();
    
    // Verify file exists
    assert!(std::path::Path::new("test_high_freq.json").exists());
    
    // Clean up
    let _ = std::fs::remove_file("test_high_freq.json");
}

