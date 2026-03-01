use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use std::hint::black_box;
use json_sync::JsonSync;
use shardmap::ShardMap;
use std::path::PathBuf;
use std::time::Duration;

fn bench_path(name: &str, size: usize) -> PathBuf {
    std::env::temp_dir().join(format!("json_sync_bench_{}_{}.json", name, size))
}

fn bench_insert_get_remove(c: &mut Criterion) {
    let mut group = c.benchmark_group("insert_get_remove");
    for size in [10, 100, 1000] {
        group.bench_with_input(BenchmarkId::new("shardmap", size), &size, |b, &size| {
            let path = bench_path("igr", size);
            let _ = std::fs::remove_file(&path);
            let db = JsonSync::<String, i32, ShardMap<String, i32>>::open(&path).unwrap();
            b.iter(|| {
                for i in 0..size {
                    let _ = db.insert(format!("k{i}"), i as i32).unwrap();
                }
                for i in 0..size {
                    black_box(db.get(&format!("k{i}")));
                }
                for i in 0..size {
                    let _ = db.remove(&format!("k{i}")).unwrap();
                }
            });
            let _ = std::fs::remove_file(&path);
        });
    }
}

fn bench_flush(c: &mut Criterion) {
    let mut group = c.benchmark_group("flush");
    group.sample_size(50);
    group.measurement_time(Duration::from_secs(8));
    for size in [100, 1000, 10_000] {
        group.bench_with_input(BenchmarkId::new("shardmap", size), &size, |b, &size| {
            let path = bench_path("flush", size);
            let _ = std::fs::remove_file(&path);
            let db = JsonSync::<String, i32, ShardMap<String, i32>>::open(&path).unwrap();
            for i in 0..size {
                db.insert(format!("k{i}"), i as i32).unwrap();
            }
            b.iter(|| db.flush().unwrap());
            let _ = std::fs::remove_file(&path);
        });
    }
}

fn bench_extend(c: &mut Criterion) {
    let mut group = c.benchmark_group("extend");
    for size in [100, 1000] {
        group.bench_with_input(BenchmarkId::new("shardmap", size), &size, |b, &size| {
            let path = bench_path("extend", size);
            let _ = std::fs::remove_file(&path);
            let db = JsonSync::<String, i32, ShardMap<String, i32>>::open(&path).unwrap();
            let batch: Vec<(String, i32)> =
                (0..size).map(|i| (format!("k{i}"), i as i32)).collect();
            b.iter(|| {
                db.extend(batch.clone()).unwrap();
                db.clear().unwrap();
            });
            let _ = std::fs::remove_file(&path);
        });
    }
}

fn bench_update(c: &mut Criterion) {
    let mut group = c.benchmark_group("update");
    for size in [100, 1000] {
        group.bench_with_input(BenchmarkId::new("shardmap", size), &size, |b, &size| {
            let path = bench_path("update", size);
            let _ = std::fs::remove_file(&path);
            let db = JsonSync::<String, i32, ShardMap<String, i32>>::open(&path).unwrap();
            for i in 0..size {
                db.insert(format!("k{i}"), i as i32).unwrap();
            }
            b.iter(|| {
                for i in 0..size {
                    db.update(&format!("k{i}"), |v| *v += 1).unwrap();
                }
            });
            let _ = std::fs::remove_file(&path);
        });
    }
}

fn bench_clear(c: &mut Criterion) {
    let mut group = c.benchmark_group("clear");
    group.sample_size(50);
    for size in [100, 1000] {
        group.bench_with_input(BenchmarkId::new("shardmap", size), &size, |b, &size| {
            let path = bench_path("clear", size);
            let _ = std::fs::remove_file(&path);
            let db = JsonSync::<String, i32, ShardMap<String, i32>>::open(&path).unwrap();
            b.iter(|| {
                for i in 0..size {
                    db.insert(format!("k{i}"), i as i32).unwrap();
                }
                db.clear().unwrap();
            });
            let _ = std::fs::remove_file(&path);
        });
    }
}

criterion_group!(
    benches,
    bench_insert_get_remove,
    bench_flush,
    bench_extend,
    bench_update,
    bench_clear,
);
criterion_main!(benches);
