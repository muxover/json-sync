//! Example: JsonSync with DashMap backend.
//!
//! Run with: `cargo run --example with_dashmap --features dashmap`

use dashmap::DashMap;
use json_sync::{JsonSync, FlushPolicy};
use std::time::Duration;

fn main() -> Result<(), json_sync::Error> {
    let path = "with_dashmap_example.json";
    let _ = std::fs::remove_file(path);

    // DashMap backend with async flush every 2 seconds
    let db = JsonSync::<String, u64, DashMap<String, u64>>::open_with_policy(
        path,
        FlushPolicy::Async(Duration::from_secs(2)),
    )?;

    db.insert("counter".to_string(), 0)?;
    db.insert("counter".to_string(), 1)?;
    println!("counter = {:?}", db.get(&"counter".to_string()));
    db.insert("scores".to_string(), 100)?;
    db.flush()?;
    println!("Entries: {:?}", db.iter());
    drop(db);
    let _ = std::fs::remove_file(path);
    Ok(())
}
