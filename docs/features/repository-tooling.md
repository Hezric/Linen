# Repository tooling

Consumers pin reusable workflows under `.github/workflows/` to signed Linen
release tags. Workflow pins can advance independently of the Cargo/npm library pins.

`windows-checks.yml` runs formatting, clippy and Rust tests, with optional Vite+
frontend installation, tests and an explicit build command. `project-policy.yml`
validates scoped commit/PR messages and branch names. Caller CI retains the
required `test` and `policy` aggregate jobs; both report even after a dependency fails.
The `test` aggregate requires scope detection to succeed and verifies either all
heavy jobs succeeded or all were skipped for a confirmed docs-only change. Required checks remain bound to
GitHub Actions in the repository ruleset.

`change-scope.yml` compares the complete Git diff with an exact caller-owned
allowlist of files that are neither packaged nor read by a build/test. Unknown
events/ranges, empty diffs, renames from code, and every unlisted path run full CI.
Consumers exclude README, changelog, notices, assets, scripts and manifests.
Linen archives its entire tracked source tree, so its own allowlist stays empty.

`windows-release.yml` verifies the existing signed annotated tag, its target and
main ancestry, then checks every field in the caller's `version-fields` declaration
against the tag without `v`. The JSON declaration maps file names to arrays of yq
expressions and includes manifest and lockfile package versions. Any mismatch,
missing or ambiguous value fails. Commit subjects do not determine eligibility.
It then runs checks, builds and invokes the
consumer's `scripts/package.ps1`. It publishes archives, SHA256SUMS and the
matching changelog section. Version tags remain immutable.

`release-reminder.yml` maintains one advisory bot issue and never releases
automatically. Its dispatch scenarios exercise thresholds without rewriting Git
history. `dependabot-automerge.yml` enables squash auto-merge only for the exact
Dependabot author, runs no pull-request source, and leaves the repository's
required checks and signature rules in force. Consumer wrappers supply events and
the minimum read/write permissions each reusable workflow declares.

Weekly grouped updates cover GitHub Actions, npm and Cargo through each
repository's `.github/dependabot.yml`. Actual bot PRs and release workflows are
verified on GitHub; local application tests do not prove hosted automation.
