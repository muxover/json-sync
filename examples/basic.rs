use json_sync::JsonSync;
use shardmap::ShardMap;

fn main() -> Result<(), json_sync::Error> {
    let path = std::env::temp_dir().join("json_sync_example_basic.json");
    let db = JsonSync::<String, i32, ShardMap<String, i32>>::open(&path)?;

    // insert / get / remove
    db.insert("apples".into(), 3)?;
    db.insert("bananas".into(), 5)?;
    println!("apples  = {:?}", db.get(&"apples".into()));
    println!("bananas = {:?}", db.get(&"bananas".into()));

    // update in place
    db.update(&"apples".into(), |n| *n += 1)?;
    println!("apples after update = {:?}", db.get(&"apples".into()));

    // get_or_insert
    let oranges = db.get_or_insert("oranges".into(), 0)?;
    println!("oranges (default 0) = {oranges}");

    // bulk insert
    db.extend(vec![("grapes".into(), 12), ("lemons".into(), 7)])?;

    // snapshots
    println!("keys   = {:?}", db.keys());
    println!("values = {:?}", db.values());
    println!("len    = {}", db.len());
    println!("empty? = {}", db.is_empty());

    // persist and clean up
    db.flush()?;
    db.clear()?;
    println!("after clear: len = {}", db.len());

    let _ = std::fs::remove_file(&path);
    Ok(())
}
