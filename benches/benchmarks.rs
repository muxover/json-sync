//! Benchmarks: insert/get/remove cycle and flush for ShardMap backend.
//!
//! Run with: `cargo bench`

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use json_sync::JsonSync;
use shardmap::ShardMap;
use std::path::PathBuf;
use std::time::Duration;

fn temp_bench_path(name: &str, size: usize) -> PathBuf {
    std::env::temp_dir().join(format!("json_sync_bench_{}_{}.json", name, size))
}

fn bench_insert_get_remove_shardmap(c: &mut Criterion) {
    let mut group = c.benchmark_group("insert_get_remove");
    for size in [10, 100, 1000] {
        group.bench_with_input(BenchmarkId::new("shardmap", size), &size, |b, &size| {
            let path = temp_bench_path("igr", size);
            let _ = std::fs::remove_file(&path);
            let db = JsonSync::<String, i32, ShardMap<String, i32>>::open(&path).unwrap();
            b.iter(|| {
                for i in 0..size {
                    let _ = db.insert(format!("k{}", i), i.try_into().unwrap()).unwrap();
                }
                for i in 0..size {
                    let _ = black_box(db.get(&format!("k{}", i)));
                }
                for i in 0..size {
                    let _ = db.remove(&format!("k{}", i)).unwrap();
                }
            });
            let _ = std::fs::remove_file(&path);
        });
    }
}

fn bench_flush_shardmap(c: &mut Criterion) {
    let mut group = c.benchmark_group("flush");
    group.sample_size(50);
    group.measurement_time(Duration::from_secs(8));
    for size in [100, 1000, 10_000] {
        group.bench_with_input(BenchmarkId::new("shardmap", size), &size, |b, &size| {
            let path = temp_bench_path("flush", size);
            let _ = std::fs::remove_file(&path);
            let db = JsonSync::<String, i32, ShardMap<String, i32>>::open(&path).unwrap();
            for i in 0..size {
                db.insert(format!("k{}", i), i.try_into().unwrap()).unwrap();
            }
            b.iter(|| {
                db.flush().unwrap();
            });
            let _ = std::fs::remove_file(&path);
        });
    }
}

criterion_group!(benches, bench_insert_get_remove_shardmap, bench_flush_shardmap);
criterion_main!(benches);
