//! Write the conformance corpus (`tests/conformance/`) into `fuzz/corpus/`, one file per input,
//! as the starting corpus of every target:
//!
//! ```text
//! cargo run --manifest-path fuzz/Cargo.toml --example seed
//! ```
//!
//! The differential target gets each equivalence pair as `a\nb` too. Files are named by content,
//! so running this again adds nothing new.
#![expect(clippy::expect_used, reason = "a one-shot tool: failing loudly is the right answer")]

#[expect(dead_code, reason = "only the inputs are used")]
#[path = "../../crates/urn-rs/tests/conformance/case.rs"]
mod case;
#[path = "../../crates/urn-rs/tests/conformance/equivalence.rs"]
mod equivalence;
#[path = "../../crates/urn-rs/tests/conformance/invalid.rs"]
mod invalid;
#[path = "../../crates/urn-rs/tests/conformance/valid.rs"]
mod valid;

use std::{
    collections::hash_map::DefaultHasher,
    fs,
    hash::{Hash, Hasher},
    path::Path,
};

fn write(dir: &Path, input: &str) {
    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    fs::write(dir.join(format!("{:016x}", hasher.finish())), input).expect("write a seed file");
}

fn main() {
    let inputs: Vec<&str> = valid::CASES
        .iter()
        .map(|c| c.input)
        .chain(invalid::CASES.iter().map(|c| c.input))
        .chain(equivalence::CASES.iter().flat_map(|c| [c.a, c.b]))
        .collect();
    let corpus = Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus");
    for target in ["parse", "differential", "percent", "setters"] {
        let dir = corpus.join(target);
        fs::create_dir_all(&dir).expect("create the corpus directory");
        for input in &inputs {
            write(&dir, input);
        }
    }
    let differential = corpus.join("differential");
    for case in equivalence::CASES {
        write(&differential, &format!("{}\n{}", case.a, case.b));
    }
    println!("{} inputs per target in {}", inputs.len(), corpus.display());
}
