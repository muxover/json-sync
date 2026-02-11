use json_sync::{load_from_file, SerializationFormat};
use std::collections::HashMap;

#[test]
fn test_load_valid_json() {
    // Create a valid JSON file
    let test_data = r#"{"key1":1,"key2":2,"key3":3}"#;
    std::fs::write("test_recovery.json", test_data).unwrap();
    
    let data: HashMap<String, i32> = load_from_file(
        "test_recovery.json",
        SerializationFormat::Json
    ).unwrap();
    
    assert_eq!(data.get("key1"), Some(&1));
    assert_eq!(data.get("key2"), Some(&2));
    assert_eq!(data.get("key3"), Some(&3));
    
    // Clean up
    let _ = std::fs::remove_file("test_recovery.json");
}

#[test]
fn test_load_empty_file() {
    // Create an empty file
    std::fs::write("test_empty.json", "").unwrap();
    
    let data: HashMap<String, i32> = load_from_file(
        "test_empty.json",
        SerializationFormat::Json
    ).unwrap();
    
    assert!(data.is_empty());
    
    // Clean up
    let _ = std::fs::remove_file("test_empty.json");
}

#[test]
fn test_load_nonexistent_file() {
    let result: Result<HashMap<String, i32>, _> = load_from_file(
        "nonexistent.json",
        SerializationFormat::Json
    );
    
    assert!(result.is_err());
}

#[test]
fn test_validate_json_file() {
    use json_sync::validate_json_file;
    
    // Valid JSON
    let test_data = r#"{"key1":1,"key2":2}"#;
    std::fs::write("test_validate.json", test_data).unwrap();
    
    assert!(validate_json_file("test_validate.json").is_ok());
    
    // Invalid JSON
    let invalid_data = r#"{"key1":1,"key2":}"#;
    std::fs::write("test_invalid.json", invalid_data).unwrap();
    
    assert!(validate_json_file("test_invalid.json").is_err());
    
    // Clean up
    let _ = std::fs::remove_file("test_validate.json");
    let _ = std::fs::remove_file("test_invalid.json");
}

