/// High-throughput example simulating high-frequency updates on large datasets.
/// 
/// This example demonstrates JsonSync's ability to handle high-frequency
/// updates with minimal overhead, especially when used with ShardMap.
/// 
/// For best performance, use ShardMap instead of HashMap:
/// See https://github.com/muxover/shardmap
use json_sync::{JsonSyncBuilder, DirtyStrategy};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use std::time::{Duration, Instant};
use std::thread;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("High-Throughput Example - Simulating high-frequency updates");
    println!("============================================================\n");

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

    // Configure JsonSync for high-throughput
    // Note: For best performance, use ShardMap instead of HashMap
    // See https://github.com/muxover/shardmap
    let sync = Arc::new(JsonSyncBuilder::new()
        .path("high_throughput_example.json")
        .flush_threshold(10000)  // Flush after 10K changes
        .batch_size(1000)
        .dirty_strategy(DirtyStrategy::PerShard)
        .shard_count(64)
        .build::<String, i32>()?);

    sync.attach(get_data, load_data)?;
    sync.start()?;

    // Simulate high-frequency updates
    let num_entries = 1_200_000;
    let num_threads = 8;
    let updates_per_thread = num_entries / num_threads;

    println!("Starting {} threads with {} updates each...", num_threads, updates_per_thread);
    let start = Instant::now();

    let mut handles = vec![];
    for thread_id in 0..num_threads {
        let map = Arc::clone(&map);
        let sync = Arc::clone(&sync);
        let handle = thread::spawn(move || {
            let thread_start = thread_id * updates_per_thread;
            for i in 0..updates_per_thread {
                let key = format!("key_{}", thread_start + i);
                let value = (thread_start + i) as i32;
                
                map.write().insert(key.clone(), value);
                sync.mark_dirty(&key);
            }
        });
        handles.push(handle);
    }

    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }

    let elapsed = start.elapsed();
    let total_updates = num_entries;
    let rps = total_updates as f64 / elapsed.as_secs_f64();

    println!("\nCompleted {} updates in {:?}", total_updates, elapsed);
    println!("Throughput: {:.2} updates/second", rps);
    println!("Average latency: {:.2} µs per update", elapsed.as_micros() as f64 / total_updates as f64);

    // Wait for flush to complete
    println!("\nWaiting for flush to complete...");
    thread::sleep(Duration::from_secs(2));
    sync.stop();

    // Check statistics
    let stats = sync.stats();
    println!("\nStatistics:");
    println!("  Flush count: {}", stats.flush_count);
    println!("  Dirty count: {}", stats.dirty_count);
    println!("  Total bytes written: {}", stats.total_bytes_written);
    println!("  Map size: {}", map.read().len());

    // Verify file exists
    if std::path::Path::new("high_throughput_example.json").exists() {
        let file_size = std::fs::metadata("high_throughput_example.json")?.len();
        println!("  File size: {} bytes", file_size);
    }

    // Clean up
    let _ = std::fs::remove_file("high_throughput_example.json");

    Ok(())
}

