use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use urn::Urn;

#[path = "fixtures.rs"]
mod fixtures;

fn bench_setters(c: &mut Criterion) {
    let mut g = c.benchmark_group("setters");

    g.bench_function("set_nid", |b| {
        b.iter_batched(
            || Urn::try_from(fixtures::MINIMAL).unwrap(),
            |mut u| {
                u.set_nid(black_box("other-nid")).unwrap();
                black_box(u);
            },
            BatchSize::SmallInput,
        )
    });

    g.bench_function("set_nss", |b| {
        b.iter_batched(
            || Urn::try_from(fixtures::MINIMAL).unwrap(),
            |mut u| {
                u.set_nss(black_box("new-nss-value")).unwrap();
                black_box(u);
            },
            BatchSize::SmallInput,
        )
    });

    g.bench_function("set_r_component_add", |b| {
        b.iter_batched(
            || Urn::try_from(fixtures::MINIMAL).unwrap(),
            |mut u| {
                u.set_r_component(Some(black_box("res:x"))).unwrap();
                black_box(u);
            },
            BatchSize::SmallInput,
        )
    });

    g.bench_function("set_r_component_remove", |b| {
        b.iter_batched(
            || Urn::try_from(fixtures::ALL_COMPONENTS).unwrap(),
            |mut u| {
                u.set_r_component(None).unwrap();
                black_box(u);
            },
            BatchSize::SmallInput,
        )
    });

    g.bench_function("set_q_component_add", |b| {
        b.iter_batched(
            || Urn::try_from(fixtures::MINIMAL).unwrap(),
            |mut u| {
                u.set_q_component(Some(black_box("a=1&b=2"))).unwrap();
                black_box(u);
            },
            BatchSize::SmallInput,
        )
    });

    g.bench_function("set_f_component_add", |b| {
        b.iter_batched(
            || Urn::try_from(fixtures::MINIMAL).unwrap(),
            |mut u| {
                u.set_f_component(Some(black_box("frag"))).unwrap();
                black_box(u);
            },
            BatchSize::SmallInput,
        )
    });

    g.bench_function("set_f_component_replace", |b| {
        b.iter_batched(
            || Urn::try_from(fixtures::ALL_COMPONENTS).unwrap(),
            |mut u| {
                u.set_f_component(Some(black_box("other-frag"))).unwrap();
                black_box(u);
            },
            BatchSize::SmallInput,
        )
    });

    g.finish();
}

criterion_group!(benches, bench_setters);
criterion_main!(benches);
