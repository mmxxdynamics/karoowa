<!--
Thanks for the PR. Keep it small and focused — one change per PR.
Read CONTRIBUTING.md for the full development checks before opening.
-->

## Summary

<!-- One paragraph: what this PR changes and why. -->

## Linked task / issue

<!-- e.g. Closes #123, or Refs T1.4.2 in specs/development/dev_plan.md -->

## Type of change

- [ ] `feat` — new behaviour
- [ ] `fix` — bug fix
- [ ] `docs` — documentation only
- [ ] `refactor` — no functional change
- [ ] `perf` — performance improvement
- [ ] `test` — adds or updates tests
- [ ] `chore` / `build` / `ci` — infra
- [ ] **Breaking change** (explain below)

## Breaking-change description

<!-- Required if you ticked Breaking change above. State the breakage and the migration path. -->

## Checklist

- [ ] PR title follows [Conventional Commits](https://www.conventionalcommits.org/)
- [ ] `cargo test --workspace` passes locally
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes locally
- [ ] `cargo fmt --all -- --check` passes locally
- [ ] `cargo deny check` passes locally
- [ ] `./scripts/check-cross-imports.sh` passes locally
- [ ] Tests added or updated for new behaviour
- [ ] Docs (README, `docs/`, rustdoc) updated where relevant
- [ ] `CHANGELOG.md` updated under `[Unreleased]`
- [ ] Commits are signed-off (`git commit -s`)

## Reviewer notes

<!-- Anything reviewers should look at first, design questions, screenshots, benchmarks. -->
