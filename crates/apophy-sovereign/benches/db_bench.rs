use criterion::{black_box, criterion_group, criterion_main, Criterion};

#[path = "../src/db.rs"]
mod db;
use db::SovereignDb;

fn bench_fleet_counts(c: &mut Criterion) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bench.db");
    let db = SovereignDb::new(&path).unwrap();
    db.migrate().unwrap();

    // Populate database
    for i in 0..100 {
        db.register_agent(&format!("s{}", i), "S", "strategic", "D", "[]", "P", None).unwrap();
    }
    for i in 0..500 {
        db.register_agent(&format!("t{}", i), "T", "tactical", "D", "[]", "P", None).unwrap();
    }
    for i in 0..2000 {
        db.register_agent(&format!("o{}", i), "O", "operational", "D", "[]", "P", None).unwrap();
    }

    c.bench_function("fleet_counts", |b| {
        b.iter(|| {
            black_box(db.fleet_counts().unwrap())
        })
    });
}

criterion_group!(benches, bench_fleet_counts);
criterion_main!(benches);
