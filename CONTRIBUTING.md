# Contributing

Veloura's contribution policy, branch model, pull request requirements,
coding standards, connector-contribution rules, and safety review criteria
are defined in
[`docs/27-repository-and-contributing.md`](docs/27-repository-and-contributing.md).
Read that document before opening a pull request.

Quick summary while the codebase is still small:

- `main` is releasable; use short-lived feature branches.
- Run `cargo fmt`, `cargo clippy --workspace -- -D warnings`, and
  `cargo test --workspace` before opening a PR — CI enforces all three.
- Use typed errors (`thiserror`), avoid logging unredacted external data or
  local paths, and keep network access behind connector interfaces once
  connectors exist.
- Describe scope, non-goals, privacy impact, and security impact in the PR
  description.
