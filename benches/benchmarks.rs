use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use json_sync::{JsonSyncBuilder, DirtyStrategy, SerializationFormat};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use std::time::Duration;

fn bench_flush_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("flush");

    // Test with different data sizes
    for size in [100, 1000, 10000, 100000].iter() {
        group.bench_with_input(
            BenchmarkId::new("flush", size),
            size,
            |b, &size| {
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
                    .path("bench_flush.json")
                    .manual_flush()
                    .build::<String, i32>()
                    .unwrap();

                sync.attach(get_data, load_data).unwrap();

                // Insert data
                for i in 0..size {
                    map.write().insert(format!("key_{}", i), i as i32);
                    sync.mark_dirty(&format!("key_{}", i));
                }

                b.iter(|| {
                    black_box(sync.flush().unwrap());
                });

                let _ = std::fs::remove_file("bench_flush.json");
            },
        );
    }

    group.finish();
}

fn bench_serialization_formats(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialization");

    let data: HashMap<String, i32> = (0..10000)
        .map(|i| (format!("key_{}", i), i))
        .collect();

    // JSON serialization
    group.bench_function("json", |b| {
        let serializer = json_sync::serializer::SerializerImpl::new(SerializationFormat::Json);
        b.iter(|| {
            black_box(serializer.serialize(&data).unwrap());
        });
    });

    // Binary serialization (if feature enabled)
    #[cfg(feature = "binary")]
    {
        group.bench_function("binary", |b| {
            let serializer = json_sync::serializer::SerializerImpl::new(SerializationFormat::Binary);
            b.iter(|| {
                black_box(serializer.serialize(&data).unwrap());
            });
        });
    }

    group.finish();
}

fn bench_dirty_tracking_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("dirty_tracking");

    // Per-key tracking
    group.bench_function("per_key", |b| {
        let sync = JsonSyncBuilder::new()
            .path("bench_per_key.json")
            .manual_flush()
            .dirty_strategy(DirtyStrategy::PerKey)
            .build::<String, i32>()
            .unwrap();

        b.iter(|| {
            for i in 0..1000 {
                sync.mark_dirty(&format!("key_{}", i));
            }
        });
    });

    // Per-shard tracking
    group.bench_function("per_shard", |b| {
        let sync = JsonSyncBuilder::new()
            .path("bench_per_shard.json")
            .manual_flush()
            .dirty_strategy(DirtyStrategy::PerShard)
            .shard_count(16)
            .build::<String, i32>()
            .unwrap();

        b.iter(|| {
            for i in 0..1000 {
                sync.mark_shard_dirty(i % 16);
            }
        });
    });

    group.finish();
}

fn bench_recovery_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("recovery");

    // Create test file
    let data: HashMap<String, i32> = (0..10000)
        .map(|i| (format!("key_{}", i), i))
        .collect();

    let serializer = json_sync::serializer::SerializerImpl::new(SerializationFormat::Json);
    let bytes = serializer.serialize(&data).unwrap();
    std::fs::write("bench_recovery.json", &bytes).unwrap();

    group.bench_function("load_from_file", |b| {
        b.iter(|| {
            let data: HashMap<String, i32> = json_sync::load_from_file(
                "bench_recovery.json",
                SerializationFormat::Json
            ).unwrap();
            black_box(data);
        });
    });

    let _ = std::fs::remove_file("bench_recovery.json");
    group.finish();
}

fn bench_concurrent_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent");

    group.bench_function("concurrent_flush", |b| {
        b.iter_custom(|iters| {
            let start = std::time::Instant::now();
            for _ in 0..iters {
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

                let sync = Arc::new(JsonSyncBuilder::new()
                    .path("bench_concurrent.json")
                    .manual_flush()
                    .build::<String, i32>()
                    .unwrap());

                sync.attach(get_data, load_data).unwrap();

                // Insert data
                for i in 0..1000 {
                    map.write().insert(format!("key_{}", i), i as i32);
                }

                // Spawn multiple threads
                let mut handles = vec![];
                for _ in 0..4 {
                    let sync = Arc::clone(&sync);
                    let handle = std::thread::spawn(move || {
                        sync.flush().unwrap();
                    });
                    handles.push(handle);
                }

                for handle in handles {
                    handle.join().unwrap();
                }

                let _ = std::fs::remove_file("bench_concurrent.json");
            }
            start.elapsed()
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_flush_performance,
    bench_serialization_formats,
    bench_dirty_tracking_overhead,
    bench_recovery_performance,
    bench_concurrent_operations
);
criterion_main!(benches);
