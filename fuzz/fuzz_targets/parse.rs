//! Fuzz target: see [`urn_rs_fuzz::parse`].
#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| urn_rs_fuzz::parse(data));
