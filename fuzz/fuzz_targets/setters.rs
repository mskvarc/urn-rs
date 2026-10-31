//! Fuzz target: see [`urn_rs_fuzz::setters`].
#![no_main]

libfuzzer_sys::fuzz_target!(|input: urn_rs_fuzz::SetterInput| urn_rs_fuzz::setters(&input));
