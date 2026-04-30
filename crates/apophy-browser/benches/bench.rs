use criterion::{black_box, criterion_group, criterion_main, Criterion};
use apophy_browser::blocker::{ContentBlocker};

fn bench_blocker(c: &mut Criterion) {
    c.bench_function("check_domain_miss", |b| {
        let mut blocker = ContentBlocker::sovereign();
        b.iter(|| {
            let res = blocker.check(black_box("https://example.com/page"), black_box("https://example.com"));
            black_box(res);
        })
    });

    c.bench_function("check_domain_hit", |b| {
        let mut blocker = ContentBlocker::sovereign();
        b.iter(|| {
            let res = blocker.check(black_box("https://coinhive.com/lib/coinhive.min.js"), black_box("https://example.com"));
            black_box(res);
        })
    });
}

criterion_group!(benches, bench_blocker);
criterion_main!(benches);
