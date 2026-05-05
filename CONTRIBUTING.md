# Contributing to Karoowa

Thank you for your interest in contributing. Karoowa is an open-source
blockchain framework that aims to be operator- and developer-first; we take
contributor experience as seriously as we do node performance.

## Quick links

- [Code of Conduct](CODE_OF_CONDUCT.md)
- [Security policy](SECURITY.md)
- [Release process](RELEASE.md)
- [Decision log](specs/strategy/03_decision_log.md)

## Getting started

1. **Fork** the repository on GitHub and clone your fork.
2. Create a feature branch from `main`:
   ```sh
   git checkout -b feat/<short-description>
   ```
   Branch prefixes: `feat/`, `fix/`, `docs/`, `chore/`, `refactor/`,
   `test/`, `perf/`.
3. Pick a task. Smaller, focused PRs are merged faster than large omnibus
   ones. The current dev plan lives in
   [`specs/development/dev_plan.md`](specs/development/dev_plan.md) (M1-M3)
   and [`specs/development/dev_plan_m4_m6.md`](specs/development/dev_plan_m4_m6.md)
   (M4-M6).

## Local development checks

Before opening a PR, every one of these must pass locally:

```sh
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
cargo deny check
./scripts/check-cross-imports.sh
```

CI runs the same checks plus a multi-OS test matrix and an MSRV build.

## Commit-message convention

We use [Conventional Commits](https://www.conventionalcommits.org/) so
release-notes generation works automatically:

```
<type>(<scope>): <imperative summary>

<optional body>

<optional footer (BREAKING CHANGE: …, Refs: #…)>
```

`type` is one of `feat`, `fix`, `docs`, `chore`, `refactor`, `test`,
`perf`, `ci`, `build`, `style`. `scope` is typically a crate name
(e.g. `karoowa-consensus`) or a milestone (`m6`). Examples:

- `feat(karoowa-consensus): add slashing for double-signing`
- `fix(karoowa-bridge): clamp timeout to u32 to avoid overflow`
- `docs(operator-guide): document HSM key rotation`

Avoid checkpoint commits like `wip` or `fix typo`: squash them before pushing.
A single PR should map to a single, logical commit (or a small chain of
ordered commits if the change is genuinely multi-step).

## Sign-off

We follow the [Developer Certificate of Origin](https://developercertificate.org/).
Every commit must carry a `Signed-off-by` trailer. Add it automatically:

```sh
git commit -s -m "feat(...)"
```

We do **not** require a CLA. Submissions to `core/` are accepted under the
project's dual licence (Apache-2.0 + MIT for downstream compatibility);
submissions to `enterprise/` are accepted under the licence terms in
`LICENSE-ENTERPRISE.md`.

## Open-core boundary

- Code under `core/` is open-source (Apache-2.0).
- Code under `enterprise/` is proprietary (BSL 1.1; see
  `LICENSE-ENTERPRISE.md`).
- **`core/` must never depend on `enterprise/`.** The CI guardrail in
  `scripts/check-cross-imports.sh` enforces this on every PR.
- `enterprise/` may depend on `core/`.

If a feature blurs the boundary, raise it in a draft PR or an RFC issue
before writing the code.

## Pull-request workflow

1. **Open a draft PR early.** Even with a half-finished implementation,
   discussion on direction is cheaper than rework.
2. **Keep PRs small.** Aim for under ~500 changed lines (excluding generated
   code). Split larger changes into a stack of dependent PRs.
3. **Link the dev-plan task** (e.g. `T1.4.2`) when applicable, so the
   reviewer can see the acceptance criteria you're working against.
4. **Add tests.** New behaviour gets unit tests; cross-cutting changes get
   integration tests under `tests/` or a property test (proptest) where
   randomness exposes edge cases.
5. **Update docs.** README, `docs/`, rustdoc: whichever is impacted.
6. **CI must be green** before requesting review.
7. **Reviewer SLA:** at least one of the maintainers in `.github/CODEOWNERS`
   acknowledges within 72 hours; first round of feedback within 7 days.
8. **Merge strategy:** rebase-merge or squash-merge, no merge commits on
   `main`.

## Reporting bugs

Open a [bug report issue](https://github.com/mmxxdynamics/karoowa/issues/new?template=bug_report.yml).
For suspected vulnerabilities, follow [`SECURITY.md`](SECURITY.md) instead.

## Asking questions

Issues are for actionable bugs and feature requests. Open-ended questions
belong in our discussion channel (link in the README). Search closed issues
and the docs first: most questions have been answered before.

## Style notes

- Avoid `unwrap()` outside tests; return a typed error or `anyhow::Result`
  with context.
- Prefer pure functions; isolate side-effects (I/O, randomness, time) at
  module boundaries.
- Internal-only items use `pub(crate)`; the `unreachable_pub` lint catches
  accidental over-exposure.
- Unsafe code requires a `// SAFETY:` comment. Anything new under `unsafe`
  needs explicit reviewer sign-off.

## Recognition

Contributors who land non-trivial changes are listed in `CHANGELOG.md` under
the version their work shipped in, plus the GitHub Release notes.

## Questions?

Open an issue tagged `question`, or check the docs in `specs/`. We'll fold
recurring questions into an FAQ.
