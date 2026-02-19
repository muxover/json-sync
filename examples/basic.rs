//! Basic example: open a JSON-backed store (ShardMap backend), insert, get, remove, flush.

use json_sync::JsonSync;
use shardmap::ShardMap;

fn main() -> Result<(), json_sync::Error> {
    let path = "basic_example.json";
    let db = JsonSync::<String, String, ShardMap<String, String>>::open(path)?;

    db.insert("key1".to_string(), "value1".to_string())?;
    db.insert("key2".to_string(), "value2".to_string())?;

    let val = db.get(&"key1".to_string());
    println!("key1 = {:?}", val);

    db.remove(&"key1".to_string())?;
    db.flush()?;

    println!("Entries: {:?}", db.iter());
    let _ = std::fs::remove_file(path);
    Ok(())
}
