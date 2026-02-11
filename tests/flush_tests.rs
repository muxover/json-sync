use json_sync::JsonSyncBuilder;
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use std::time::Duration;

#[test]
fn test_manual_flush() {
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
        .path("test_manual_flush.json")
        .manual_flush()
        .build::<String, i32>()
        .unwrap();

    sync.attach(get_data, load_data).unwrap();
    
    map.write().insert("key1".to_string(), 1);
    sync.mark_dirty("key1");
    
    assert!(sync.flush().is_ok());
    assert!(std::path::Path::new("test_manual_flush.json").exists());
    
    // Clean up
    let _ = std::fs::remove_file("test_manual_flush.json");
}

#[tokio::test]
async fn test_time_based_flush() {
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
        .path("test_time_flush.json")
        .flush_interval(Duration::from_millis(100))
        .build::<String, i32>()
        .unwrap();

    sync.attach(get_data, load_data).unwrap();
    
    map.write().insert("key1".to_string(), 1);
    sync.mark_dirty("key1");
    
    // Start the scheduler
    assert!(sync.start().is_ok());
    
    // Wait a bit for flush to happen
    tokio::time::sleep(Duration::from_millis(150)).await;
    
    // Stop the scheduler
    sync.stop();
    
    // Verify file was created
    assert!(std::path::Path::new("test_time_flush.json").exists());
    
    // Clean up
    let _ = std::fs::remove_file("test_time_flush.json");
}

#[tokio::test]
async fn test_count_based_flush() {
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
        .path("test_count_flush.json")
        .flush_threshold(5)
        .build::<String, i32>()
        .unwrap();

    sync.attach(get_data, load_data).unwrap();
    
    // Start the scheduler
    assert!(sync.start().is_ok());
    
    // Mark multiple keys as dirty
    for i in 0..5 {
        map.write().insert(format!("key{}", i), i);
        sync.mark_dirty(&format!("key{}", i));
    }
    
    // Wait a bit for flush to happen
    tokio::time::sleep(Duration::from_millis(200)).await;
    
    // Stop the scheduler
    sync.stop();
    
    // Verify file was created
    assert!(std::path::Path::new("test_count_flush.json").exists());
    
    // Clean up
    let _ = std::fs::remove_file("test_count_flush.json");
}

