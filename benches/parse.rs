use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use urn_rs::{Urn, UrnSlice};

#[path = "fixtures.rs"]
mod fixtures;

fn bench_parse(c: &mut Criterion) {
    let long = fixtures::long_urn(1024);
    let mut g = c.benchmark_group("parse");

    for (name, input) in [
        ("slice_from_str_normalized", fixtures::NORMALIZED),
        ("slice_from_str_needs_norm", fixtures::NEEDS_NORM),
        ("slice_from_str_percent_heavy", fixtures::PERCENT_HEAVY),
        ("slice_from_str_all_components", fixtures::ALL_COMPONENTS),
        ("slice_from_str_nbn", fixtures::NBN),
        ("slice_from_str_invalid_double_colon", fixtures::INVALID_DOUBLE_COLON),
        ("slice_from_str_invalid_not_urn", fixtures::INVALID_NOT_URN),
        ("slice_from_str_invalid_bad_nid", fixtures::INVALID_BAD_NID_DASH),
    ] {
        g.bench_function(name, |b| {
            b.iter(|| {
                let _ = black_box(UrnSlice::try_from(black_box(input)));
            })
        });
    }

    g.bench_function("slice_from_str_long", |b| {
        b.iter(|| {
            let _ = black_box(UrnSlice::try_from(black_box(long.as_str())));
        })
    });

    for (name, input) in [
        ("slice_from_mut_str_needs_norm", fixtures::NEEDS_NORM),
        ("slice_from_mut_str_percent_heavy", fixtures::PERCENT_HEAVY),
        ("slice_from_mut_str_normalized", fixtures::NORMALIZED),
    ] {
        g.bench_function(name, |b| {
            b.iter_batched(
                || input.to_owned(),
                |mut owned| {
                    let s: &mut str = owned.as_mut_str();
                    let _ = black_box(UrnSlice::try_from(black_box(s)));
                },
                BatchSize::SmallInput,
            )
        });
    }

    for (name, input) in [
        ("urn_from_str_normalized", fixtures::NORMALIZED),
        ("urn_from_str_needs_norm", fixtures::NEEDS_NORM),
        ("urn_from_str_all_components", fixtures::ALL_COMPONENTS),
        ("urn_from_str_invalid_not_urn", fixtures::INVALID_NOT_URN),
        ("urn_from_str_invalid_double_colon", fixtures::INVALID_DOUBLE_COLON),
    ] {
        g.bench_function(name, |b| {
            b.iter(|| {
                let _ = black_box(Urn::try_from(black_box(input)));
            })
        });
    }

    g.bench_function("urn_from_string_normalized", |b| {
        b.iter_batched(
            || fixtures::NORMALIZED.to_owned(),
            |s| black_box(Urn::try_from(black_box(s))),
            BatchSize::SmallInput,
        )
    });
    g.bench_function("urn_from_string_needs_norm", |b| {
        b.iter_batched(
            || fixtures::NEEDS_NORM.to_owned(),
            |s| black_box(Urn::try_from(black_box(s))),
            BatchSize::SmallInput,
        )
    });

    g.bench_function("urn_from_mut_str_needs_norm", |b| {
        b.iter_batched(
            || fixtures::NEEDS_NORM.to_owned(),
            |mut owned| {
                let s: &mut str = owned.as_mut_str();
                let _ = black_box(Urn::try_from(black_box(s)));
            },
            BatchSize::SmallInput,
        )
    });

    g.finish();
}

criterion_group!(benches, bench_parse);
criterion_main!(benches);
