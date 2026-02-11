/// Payload builder example showing how to read batches of entries
/// for payload creation.
/// 
/// This example demonstrates efficient batch processing of synced data.
use json_sync::JsonSyncBuilder;
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Payload Builder Example");
    println!("=======================\n");

    // Create a thread-safe map
    let map = Arc::new(RwLock::new(HashMap::new()));

    // Create callbacks
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

    // Create JsonSync
    let sync = JsonSyncBuilder::new()
        .path("payload_example.json")
        .manual_flush()
        .build()?;

    sync.attach(get_data, load_data)?;

    // Insert some sample data
    println!("Inserting sample data...");
    for i in 0..100 {
        let key = format!("user_{}", i);
        let value = i * 10;
        map.write().insert(key.clone(), value);
        sync.mark_dirty(&key);
    }

    // Flush to disk
    sync.flush()?;
    println!("Data flushed to disk\n");

    // Load data from file
    sync.load_from_file()?;
    println!("Data loaded from file\n");

    // Build payloads in batches
    println!("Building payloads in batches...");
    let batch_size = 10;
    let mut batch_num = 0;

    // Collect entries into owned values
    let entries: Vec<(String, i32)> = {
        let map_guard = map.read();
        map_guard.iter().map(|(k, v)| (k.clone(), *v)).collect()
    };
    
    for chunk in entries.chunks(batch_size) {
        batch_num += 1;
        let mut payload = HashMap::new();
        
        for (key, value) in chunk {
            payload.insert(key.clone(), *value);
        }

        println!("Batch {}: {} entries", batch_num, payload.len());
        // In a real application, you would send this payload somewhere
        // e.g., send_to_api(&payload);
    }

    println!("\nTotal batches: {}", batch_num);

    // Example: Build a specific payload for a subset of keys
    println!("\nBuilding specific payload...");
    let mut specific_payload = HashMap::new();
    for i in 0..5 {
        let key = format!("user_{}", i);
        if let Some(value) = map.read().get(&key) {
            specific_payload.insert(key, *value);
        }
    }
    println!("Specific payload: {:?}", specific_payload);

    // Clean up
    let _ = std::fs::remove_file("payload_example.json");

    Ok(())
}

