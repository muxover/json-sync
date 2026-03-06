use json_sync::{FlushPolicy, JsonSync};
use shardmap::ShardMap;
use std::time::Duration;

fn main() -> Result<(), json_sync::Error> {
    let path = std::env::temp_dir().join("json_sync_example_builder.json");

    // pretty-printed JSON + async flush every 5 seconds
    let db = JsonSync::<String, String, ShardMap<String, String>>::builder(&path)
        .pretty(true)
        .policy(FlushPolicy::Async(Duration::from_secs(5)))
        .build()?;

    db.insert("name".into(), "json-sync".into())?;
    db.insert("version".into(), "0.1.0".into())?;
    db.insert("status".into(), "awesome".into())?;
    db.flush()?;

    // the file on disk is now nicely indented
    let contents = std::fs::read_to_string(db.path())?;
    println!("On-disk JSON:\n{contents}");

    println!("\nDebug output: {db:?}");

    let _ = std::fs::remove_file(&path);
    Ok(())
}
