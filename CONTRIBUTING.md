# Contributing to urn-rs

Thank you for helping improve `urn-rs`. This guide explains how to report a problem, set up the project, and get a change merged.

By taking part you agree to follow the [Code of Conduct](CODE_OF_CONDUCT.md). To report a security issue, follow the [security policy](SECURITY.md) and do not open a public issue.

## Project Principles

Every review comes back to these, so read them before you write code.

- **The RFC decides.** Syntax, normalization and equivalence follow [RFC 8141](https://datatracker.ietf.org/doc/html/rfc8141), and percent-encoding follows [RFC 3986](https://datatracker.ietf.org/doc/html/rfc3986). A change to URN behavior cites the section that justifies it. Where RFC 2141 or another URN library disagrees with RFC 8141, the crate follows RFC 8141.
- **NIDs are checked for syntax only.** The ABNF and the reserved NID `urn` decide which NIDs parse. The NID rules in RFC 8141 §5.1 and §5.2 govern registration, so `urn:X-a:x` and `urn:urn-0:nss` parse. Per-namespace lexical-equivalence rules are not applied either.
- **`no_std` comes first.** Parsing and `UrnSlice` work without `alloc`. The `alloc` feature adds `Urn`, `UrnBuilder` and the percent-encoding functions. Every combination of features builds, passes clippy and passes its tests.
- **`unsafe` is rare and documented.** Each `unsafe` block holds one unsafe operation and a `// SAFETY:` comment that says why it is sound. Clippy denies anything else.
- **No panics a caller can reach.** Any input, valid or not, gives a value or an `Error`. The fuzz targets check this.
- **The public API is a contract.** Every public item is a SemVer commitment, including the items in `urn-rs-core` that `urn-rs` re-exports.
- **The three crates release together.** The `urn!` expansion calls a hidden constructor in `urn-rs-core`, so the crates require each other at the exact same version. The `=` pins in `[workspace.dependencies]` are bumped together with `workspace.package.version`.

## Crates

- `crates/urn-rs`: the facade users depend on. It re-exports the other two.
- `crates/urn-rs-core`: types, parser and percent-encoding, with the tests and benches.
- `crates/urn-rs-static`: the `urn!` proc macro.
- `fuzz`: the cargo-fuzz targets, in a workspace of their own.

## Reporting a Bug

Open an issue with:

- the crate version and the Cargo features you enabled
- a minimal code sample that reproduces the problem
- what you expected and what happened
- if the crate disagrees with an RFC, the section and the text that says so

## Proposing a Change

Open a pull request directly for a bug fix or a documentation fix.

Open an issue first for new functionality, a new public item, a new dependency or a breaking change, so the design is agreed before the code is written.

## Setting Up

The toolchain is pinned in `rust-toolchain.toml`, and `rustup` installs it on first use. Formatting needs nightly `rustfmt`, because `rustfmt.toml` uses unstable import options. cargo-fuzz needs nightly too. The MSRV check needs Rust 1.88.0, the `rust-version` in `Cargo.toml`. The `no_std` check needs a target without a standard library.

```sh
rustup toolchain install nightly --profile minimal --component rustfmt
rustup toolchain install 1.88.0 --profile minimal
rustup target add thumbv7em-none-eabihf
cargo install --locked taplo-cli cargo-deny cargo-machete cargo-fuzz cargo-mutants
```

The default feature is `std`. The others are `alloc`, `serde` and `static`.

## Before You Open a Pull Request

Each check below runs in CI and must pass with no warnings.

| Check | Command |
| --- | --- |
| Format | `cargo +nightly fmt --all --check` |
| Format the fuzz crate | `cargo +nightly fmt --manifest-path fuzz/Cargo.toml --check` |
| TOML format | `taplo fmt --check` |
| Clippy | `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` |
| Clippy on the fuzz crate | `cargo clippy --manifest-path fuzz/Cargo.toml --all-targets --locked -- -D warnings` |
| Clippy per feature | `cargo clippy -p urn-rs-core --all-targets --no-default-features --features "<set>" --locked -- -D warnings`, once for each set: none, `alloc`, `serde` and `alloc,serde`. Then the same for `-p urn-rs` with none, `alloc`, `serde` and `static` |
| Tests | `cargo test --workspace --all-features --locked` |
| Tests per feature | `cargo test -p urn-rs-core --no-default-features --features "<set>" --locked`, once for each set: none, `alloc`, `serde` and `alloc,serde` |
| `no_std` | `cargo build -p urn-rs --no-default-features --target thumbv7em-none-eabihf --locked`, then again with `--features alloc,serde,static` |
| MSRV | `cargo +1.88.0 test --workspace --all-features --locked` and `cargo +1.88.0 check -p urn-rs-core --no-default-features --locked` |
| Rustdoc | `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features --locked`, then again with `-p urn-rs-core --no-default-features` in place of `--workspace --all-features` |
| Rustdoc as docs.rs builds it | `RUSTDOCFLAGS="-D warnings --cfg docsrs" cargo +nightly doc --workspace --no-deps --all-features --locked` |
| Licenses, advisories, bans and sources | `cargo deny check` |
| Unused dependencies | `cargo machete` |
| Fuzz targets | `cargo +nightly fuzz build`, then again with `--no-default-features` |

Run the per-feature checks one package at a time. In a workspace build, dev-dependencies turn on `std`, which hides warnings that only a `no_std` build shows.

The lint levels in `Cargo.toml`, `clippy.toml`, `rustfmt.toml`, `.taplo.toml` and `deny.toml` are project policy. Fix the code, not the lint, and do not lower a level. `#[allow(...)]` is denied. If a lint is wrong for one specific case, use `#[expect(..., reason = "...")]` and say why in the pull request.

## Tests

- New behavior comes with tests: unit tests in a `tests` module next to the code, or integration tests under the crate's `tests/`.
- A change to parsing, normalization, equivalence or encoding adds cases to the conformance corpus in `crates/urn-rs-core/tests/conformance/`: `valid.rs`, `invalid.rs` or `equivalence.rs`. Every case names its source with `.src(...)`, such as the RFC section or the test suite of another URN library.
- The property tests in `crates/urn-rs-core/tests/props/` check urn-rs against `reference.rs`, a parser written straight from the RFC 8141 ABNF. If the grammar changes, `reference.rs` changes with it. Set `PROPTEST_CASES` to run more than the default 256 cases per property.
- A test that only makes sense with or without a feature is gated with `#[cfg(...)]`, so the per-feature test runs cover it.

## Fuzzing

The targets in `fuzz/fuzz_targets/` are `parse`, `percent`, `setters` and `differential`. Each hands its input to a check in `fuzz/src/lib.rs`, which panics when urn-rs disagrees with the reference parser or breaks a round trip.

Seed the corpus from the conformance cases, then run a target with the dictionary:

```sh
cargo run --manifest-path fuzz/Cargo.toml --example seed
cargo +nightly fuzz run parse -- -dict=fuzz/urn.dict -max_total_time=300
```

Add `--no-default-features` to fuzz the `no_std` build. A crash found this way becomes a case in the conformance corpus or a unit test.

CI runs each target for a minute on every push to master. A pull request isn't held up by it, because a fuzzer can find an old bug at any time.

## Mutation Testing

Every mutant cargo-mutants generates must be caught by the tests, except the equivalent mutants listed in `.cargo/mutants.toml`. Each entry there matches one mutant by file, line and column, and says why no test can catch it.

```sh
cargo mutants --all-features
cargo mutants --all-features --file crates/urn-rs-core/src/percent.rs
```

Always run with `--all-features`. cargo-mutants ignores `cfg`, so in another feature set it reports code that was never compiled as missed. When an edit moves a listed line, its mutant shows up as missed: check that the reason still holds, then update the line and column.

CI runs the full set weekly, in four shards, and on demand from the Actions tab.

## Dependencies

- `urn-rs-core` depends only on `thiserror` and the optional `serde`. A new dependency needs an issue first, and one `urn-rs-core` uses has to work without `std`.
- Set `default-features = false` when a dependency's defaults pull in `std` or something the crate does not use, and list only the features it needs.
- Declare full versions (`1.2.3`, never `1` or `1.2`) and no range operators, except the `=` pins between the urn-rs crates. Keep every dependency table in alphabetical order.
- `Cargo.lock` and `fuzz/Cargo.lock` are committed, and CI builds with `--locked`.

## Commits

Commit messages follow [Conventional Commits](https://www.conventionalcommits.org), written in lowercase. The types are `feat`, `fix`, `perf`, `refactor`, `docs`, `test`, `style`, `build`, `ci` and `chore`. Mark a breaking change with `!`, as in `feat!: return a result from the encode iterators`, and say in a `BREAKING CHANGE:` footer what callers have to change.

An issue goes on a line of its own in the last paragraph, never in the header. Write `Closes #N` when the commit finishes the issue and `Refs #N` when it is partial.

### In the Changelog

The changelog and the release notes are built with `git-cliff` from commit headers and footers, with the summary as written, so every commit is also a changelog entry. A commit that does not follow the format is left out.

- A breaking commit is listed once, under Breaking changes, with its summary followed by its `BREAKING CHANGE:` footer.
- The `security` scope, as in `fix(security): ...`, files a commit under Security. A summary that starts with `deprecate` files it under Deprecations.
- Every other commit is grouped by its type, in this order: features, bug fixes, performance, refactors, documentation, dependency updates, build, CI, tests, style and chores.
- `Closes #N` and `Refs #N` become links to the issues.
- Put `@` words in backticks, because GitHub links a bare one to the account of that name.

## Pull Requests

- Keep a pull request to one concern.
- A change to the public API updates every caller, test, doctest and README in the same pull request.
- A `perf` change includes the Criterion results from `cargo bench` before and after. The benches are in `crates/urn-rs-core/benches/`.

## Releasing

A release is cut by pushing a tag: `vX.Y.Z`, or `vX.Y.Z-alpha.N`, `-beta.N` or `-rc.N` for a pre-release. The `release` workflow checks that the tag matches `workspace.package.version` and points at a commit on master, publishes the three crates to crates.io, and creates the GitHub release.

1. Set `workspace.package.version` and the two `=` pins in `[workspace.dependencies]` to the new version.
2. Write the release notes to a Markdown file outside the repository, here `../vX.Y.Z.md`. Use these headings, and leave out any that would be empty:
   - `### Highlights` for the changes worth reading first, described in prose.
   - `### Upgrade notes` for what callers have to change when they move to the release.
   - `### Security` for the write-up of each advisory the release fixes, with its identifiers.

   Write every link as an absolute URL, because Dependabot and Renovate copy the release notes into the update pull requests of the crate's users.
3. Check that `git cliff --bumped-version` prints `vX.Y.Z`. It reads the commits since the last stable release and calls for a major release when one of them is breaking, a minor release when one adds a feature, and a patch release otherwise.
4. Write the changelog with `git cliff --tag vX.Y.Z --with-tag-message "$(cat ../vX.Y.Z.md)" -o CHANGELOG.md`. It rewrites the whole file. Never pass `--unreleased` or `--prepend`, which stop at the newest pre-release tag and leave out the commits before it.
5. Commit the version, `Cargo.lock` and the changelog with a `chore(release): vX.Y.Z` header, which keeps the commit itself out of the changelog, and push it to master.
6. Once CI has passed on that commit, tag it with `git tag -a --cleanup=verbatim -F ../vX.Y.Z.md vX.Y.Z` and push the tag. Without `--cleanup=verbatim`, git deletes every line of the message that starts with `#`, so the headings would be lost.

The GitHub release and the release's entry in `CHANGELOG.md` open with the tag message, followed by the changes since the previous stable release. To see the release body as the workflow will build it, run this before tagging:

```sh
git cliff "$(git describe --tags --abbrev=0 --match 'v[0-9]*' --exclude '*-*')..HEAD" --tag vX.Y.Z --with-tag-message "$(cat ../vX.Y.Z.md)" --strip all
```

A pre-release takes steps 1, 5 and 6 only, with the version set to `X.Y.Z-rc.N` (or `alpha.N`, `beta.N`). Its notes are optional and go in the tag message the same way. `CHANGELOG.md` stays as it is: it lists stable releases only, and a pre-release's changes appear under the stable release that follows it.

To keep a commit out of the changelog, such as a change and its revert in the same release, add its full hash to `.cliffignore` at the root of the repository, one hash per line.

## License

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be triple licensed under the [0BSD license](LICENSE-0BSD.md), the [Apache License, Version 2.0](LICENSE-APACHE.md) and the [MIT license](LICENSE-MIT.md), without any additional terms or conditions.
