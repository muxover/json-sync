/// Basic example showing how to use JsonSync with a HashMap.
use json_sync::{JsonSyncBuilder, DirtyStrategy};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a thread-safe map
    let map = Arc::new(RwLock::new(HashMap::new()));

    // Create callbacks for getting and loading data
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

    // Create and configure JsonSync
    let sync = JsonSyncBuilder::new()
        .path("basic_example.json")
        .flush_interval(Duration::from_secs(5))
        .batch_size(100)
        .dirty_strategy(DirtyStrategy::PerKey)
        .build()?;

    // Attach the map
    sync.attach(get_data, load_data)?;

    // Insert some data
    println!("Inserting data...");
    map.write().insert("key1".to_string(), 1);
    map.write().insert("key2".to_string(), 2);
    map.write().insert("key3".to_string(), 3);

    // Mark entries as dirty when they change
    sync.mark_dirty("key1");
    sync.mark_dirty("key2");
    sync.mark_dirty("key3");

    // Flush manually
    println!("Flushing to disk...");
    sync.flush()?;
    println!("Flush complete!");

    // Check statistics
    let stats = sync.stats();
    println!("Flush count: {}", stats.flush_count);
    println!("Dirty count: {}", stats.dirty_count);
    println!("Total bytes written: {}", stats.total_bytes_written);

    // Load data from file
    println!("\nLoading data from file...");
    sync.load_from_file()?;
    println!("Data loaded!");

    // Verify data
    println!("\nMap contents:");
    for (key, value) in map.read().iter() {
        println!("  {}: {}", key, value);
    }

    // Clean up
    let _ = std::fs::remove_file("basic_example.json");

    Ok(())
}

