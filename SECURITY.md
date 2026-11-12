# Security Policy

## Reporting a Vulnerability

Do not report security issues through public issues or pull requests.

Report them privately through GitHub, using [Report a vulnerability](https://github.com/mskvarc/urn-rs/security/advisories/new).

This covers memory unsafety, and any input that makes the crate panic, loop without end or allocate without bound, because the crate parses untrusted input.

Include the affected version, the enabled Cargo features, a description of the issue and its impact, and steps or a minimal code sample that reproduce it.

Once a fix is published to crates.io, the advisory is made public and a [RustSec](https://rustsec.org) advisory is filed.
