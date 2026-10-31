//! Fuzz target: see [`urn_rs_fuzz::percent`].
#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| urn_rs_fuzz::percent(data));
