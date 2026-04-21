use core::hash::{Hash, Hasher};
use criterion::{Criterion, criterion_group, criterion_main};
use std::{collections::hash_map::DefaultHasher, hint::black_box};
use urn_rs::{Urn, UrnBuilder};

#[path = "fixtures.rs"]
mod fixtures;

fn bench_builder(c: &mut Criterion) {
    let mut g = c.benchmark_group("builder");

    g.bench_function("build_minimal", |b| {
        b.iter(|| black_box(UrnBuilder::new(black_box("example"), black_box("1234:5678")).build().unwrap()))
    });

    g.bench_function("build_all_components", |b| {
        b.iter(|| {
            black_box(
                UrnBuilder::new(black_box("example"), black_box("foo"))
                    .r_component(Some(black_box("res:x")))
                    .q_component(Some(black_box("a=1&b=2")))
                    .f_component(Some(black_box("frag")))
                    .build()
                    .unwrap(),
            )
        })
    });

    g.finish();
}

fn bench_accessors(c: &mut Criterion) {
    let urn = Urn::try_from(fixtures::ALL_COMPONENTS).unwrap();
    let urn_minimal = Urn::try_from(fixtures::MINIMAL).unwrap();

    let mut g = c.benchmark_group("accessors");
    g.bench_function("as_str", |b| b.iter(|| black_box(black_box(&urn).as_str())));
    g.bench_function("nid", |b| b.iter(|| black_box(black_box(&urn).nid())));
    g.bench_function("nss", |b| b.iter(|| black_box(black_box(&urn).nss())));
    g.bench_function("r_component", |b| b.iter(|| black_box(black_box(&urn).r_component())));
    g.bench_function("q_component", |b| b.iter(|| black_box(black_box(&urn).q_component())));
    g.bench_function("f_component", |b| b.iter(|| black_box(black_box(&urn).f_component())));
    g.finish();

    let mut g = c.benchmark_group("eq_hash");
    let urn_a = urn.clone();
    let urn_b = urn.clone();
    g.bench_function("eq_same_all_components", |b| b.iter(|| black_box(black_box(&urn_a) == black_box(&urn_b))));
    g.bench_function("eq_same_minimal", |b| {
        let m = urn_minimal.clone();
        b.iter(|| black_box(black_box(&urn_minimal) == black_box(&m)))
    });
    g.bench_function("hash_all_components", |b| {
        b.iter(|| {
            let mut h = DefaultHasher::new();
            black_box(&urn).hash(&mut h);
            black_box(h.finish())
        })
    });
    g.finish();
}

criterion_group!(benches, bench_builder, bench_accessors);
criterion_main!(benches);
