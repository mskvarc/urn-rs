//! Fuzz target: see [`urn_rs_fuzz::differential`].
#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| urn_rs_fuzz::differential(data));
