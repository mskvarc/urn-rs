<!-- Do not open a pull request for a security vulnerability. Report it privately, as SECURITY.md describes. -->

## Summary

<!-- What the change does and why. For URN behavior, cite the RFC section that requires it. Link the issue it resolves, for example "Closes #12". A new public item, a new dependency or a breaking change needs an agreed issue first. -->

## Checklist

- [ ] The commit messages follow Conventional Commits, in lowercase, with `!` on a breaking change.
- [ ] Every check in CONTRIBUTING.md passes with no warnings, in every feature set it lists.
- [ ] New behavior has tests, and a change to parsing, normalization, equivalence or encoding adds conformance cases that name their source.
- [ ] A change to the public API updates every caller, test, doctest and README in this pull request.
- [ ] Every new `unsafe` block has a `// SAFETY:` comment, and every new equivalent mutant is listed in `.cargo/mutants.toml` with its reason.
