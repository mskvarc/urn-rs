use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;
use urn::{Urn, UrnSlice};

#[path = "fixtures.rs"]
mod fixtures;

fn bench_serde(c: &mut Criterion) {
    let inputs: &[(&str, &str)] = &[
        ("minimal", fixtures::MINIMAL),
        ("normalized", fixtures::NORMALIZED),
        ("all_components", fixtures::ALL_COMPONENTS),
        ("nbn", fixtures::NBN),
        ("weather", fixtures::WEATHER),
    ];

    let mut g = c.benchmark_group("serde");

    for (name, s) in inputs {
        let urn = Urn::try_from(*s).unwrap();
        let json = serde_json::to_string(&urn).unwrap();

        g.bench_function(format!("urn_to_string_{name}"), |b| {
            b.iter(|| black_box(serde_json::to_string(black_box(&urn)).unwrap()))
        });
        g.bench_function(format!("urn_from_str_{name}"), |b| {
            b.iter(|| {
                let v: Urn = serde_json::from_str(black_box(json.as_str())).unwrap();
                black_box(v)
            })
        });

        let slice = UrnSlice::try_from(*s).unwrap();
        g.bench_function(format!("slice_to_string_{name}"), |b| {
            b.iter(|| black_box(serde_json::to_string(black_box(&slice)).unwrap()))
        });
        g.bench_function(format!("slice_from_str_{name}"), |b| {
            b.iter(|| {
                let v: UrnSlice<'_> = serde_json::from_str(black_box(json.as_str())).unwrap();
                black_box(v)
            })
        });
    }

    g.finish();
}

criterion_group!(benches, bench_serde);
criterion_main!(benches);
