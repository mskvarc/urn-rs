use criterion::{black_box, criterion_group, criterion_main, Criterion};
use urn::percent::{
    decode_f_component, decode_nss, decode_q_component, decode_r_component, encode_f_component,
    encode_nss, encode_q_component, encode_r_component,
};

#[path = "fixtures.rs"]
mod fixtures;

fn bench_encode(c: &mut Criterion) {
    let long_ascii = fixtures::long_nss(1024)
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>();
    let long_unicode: String = fixtures::UNICODE_PLAIN.repeat(64);

    let cases: &[(&str, &str)] = &[
        ("ascii_alnum", fixtures::ASCII_ALNUM),
        ("reserved_heavy", fixtures::RESERVED_HEAVY),
        ("unicode", fixtures::UNICODE_PLAIN),
    ];

    let mut g = c.benchmark_group("encode");
    for (name, input) in cases {
        g.bench_function(format!("nss_{name}"), |b| {
            b.iter(|| black_box(encode_nss(black_box(input))))
        });
        g.bench_function(format!("r_{name}"), |b| {
            b.iter(|| black_box(encode_r_component(black_box(input))))
        });
        g.bench_function(format!("q_{name}"), |b| {
            b.iter(|| black_box(encode_q_component(black_box(input))))
        });
        g.bench_function(format!("f_{name}"), |b| {
            b.iter(|| black_box(encode_f_component(black_box(input))))
        });
    }
    g.bench_function("nss_long_ascii", |b| {
        b.iter(|| black_box(encode_nss(black_box(long_ascii.as_str()))))
    });
    g.bench_function("nss_long_unicode", |b| {
        b.iter(|| black_box(encode_nss(black_box(long_unicode.as_str()))))
    });
    g.bench_function("r_long_unicode", |b| {
        b.iter(|| black_box(encode_r_component(black_box(long_unicode.as_str()))))
    });
    g.finish();
}

fn bench_decode(c: &mut Criterion) {
    let all_triplets = fixtures::all_pct_triplets(1024);
    let mixed = fixtures::mixed_pct(1024);
    let no_triplets: String = fixtures::ASCII_ALNUM.repeat(32);

    let cases: &[(&str, &str)] = &[
        ("no_triplets", fixtures::ASCII_ALNUM),
        ("all_triplets", all_triplets.as_str()),
        ("mixed", mixed.as_str()),
        ("no_triplets_long", no_triplets.as_str()),
    ];

    let mut g = c.benchmark_group("decode");
    for (name, input) in cases {
        g.bench_function(format!("nss_{name}"), |b| {
            b.iter(|| black_box(decode_nss(black_box(input))))
        });
        g.bench_function(format!("r_{name}"), |b| {
            b.iter(|| black_box(decode_r_component(black_box(input))))
        });
        g.bench_function(format!("q_{name}"), |b| {
            b.iter(|| black_box(decode_q_component(black_box(input))))
        });
        g.bench_function(format!("f_{name}"), |b| {
            b.iter(|| black_box(decode_f_component(black_box(input))))
        });
    }
    g.finish();
}

criterion_group!(benches, bench_encode, bench_decode);
criterion_main!(benches);
