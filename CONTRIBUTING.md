# Contributing to Karoowa

Thank you for your interest in contributing to Karoowa.

## Getting started

1. Fork the repository
2. Clone your fork and create a feature branch: `git checkout -b feat/my-feature`
3. Check the current dev plan: [`specs/development/dev_plan.md`](specs/development/dev_plan.md)
4. Pick a task, implement it, and ensure all checks pass

## Development checks

Before submitting a PR, run these locally:

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
./scripts/check-cross-imports.sh
```

All five must pass. CI runs the same checks on every PR.

## Open-core boundary

- Code under `core/` is open source (Apache 2.0).
- Code under `enterprise/` is proprietary.
- **`core/` must never import from `enterprise/`.** The CI guardrail script (`scripts/check-cross-imports.sh`) enforces this. `enterprise/` may import from `core/`.

## Pull request guidelines

- Link your PR to a task in `specs/development/dev_plan.md` when applicable.
- Include tests for new functionality.
- Keep PRs focused — one task per PR is ideal.
- Run `cargo fmt --all` before committing.

## Code of conduct

This project follows the [Contributor Covenant 2.1](CODE_OF_CONDUCT.md).

## Questions?

Open an issue or check the project documentation in `specs/`.
