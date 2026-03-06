use dashmap::DashMap;
use json_sync::{FlushPolicy, JsonSync};
use std::time::Duration;

fn main() -> Result<(), json_sync::Error> {
    let path = std::env::temp_dir().join("json_sync_example_dashmap.json");

    let db = JsonSync::<String, u64, DashMap<String, u64>>::open_with_policy(
        &path,
        FlushPolicy::Async(Duration::from_secs(2)),
    )?;

    db.insert("counter".into(), 0)?;
    for _ in 0..10 {
        db.update(&"counter".into(), |v| *v += 1)?;
    }
    println!("counter = {:?}", db.get(&"counter".into()));

    db.extend(vec![("x".into(), 100), ("y".into(), 200)])?;
    println!("keys = {:?}", db.keys());

    db.flush()?;
    let _ = std::fs::remove_file(&path);
    Ok(())
}
