# Repository tooling

Consumers pin reusable workflows under `.github/workflows/` to the same signed
GitHub release tag as the library packages.

`windows-checks.yml` runs formatting, clippy and Rust tests, with optional Vite+
frontend installation, tests and an explicit build command. `project-policy.yml`
validates scoped commit/PR messages and branch names. Caller CI retains the
required `test` and `policy` aggregate jobs; both run even after a dependency fails
and fail unless their reusable job succeeded. Required checks remain bound to
GitHub Actions in the repository ruleset.

`windows-release.yml` verifies the existing signed annotated tag, its release
commit subject and main ancestry, then runs checks, builds and invokes the
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
