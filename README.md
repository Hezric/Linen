# Linen

A common thread for small, considered programs.

[简体中文](README_zh-Hans.md)

Linen is Hezric's shared foundation for open-source software. Its first consumers,
[Earmuff](https://github.com/Hezric/Earmuff) and
[Homing](https://github.com/Hezric/Homing), share a Windows tray shell and a quiet,
Fluent-inspired interface.

## Packages

| Package | Purpose |
| --- | --- |
| `linen-tray` | Tauri 2 tray menus, live theme and taskbar artwork, flyout placement, dismissal, native DWM styling and motion. |
| `linen-ui` | Framework-free UI components, design tokens, motion and shared Fluent glyphs, for any UI application. |
| `linen-tooling` | Tray raster/ICO generation, Chinese font subsetting and reusable Windows CI/release workflows. |

Packages live independently in `packages/linen-tray`, `packages/linen-ui` and
`tooling`. The repository-root JavaScript manifest is the Git-install entry point
for `linen-ui`, with tooling available through explicit subpath imports. It does
not load native Rust code. A CLI such as Lineup depends on none of these packages.

Add a package only when two programs need the same implementation. In particular,
`linen-core` will be added when two programs share non-UI behavior; it does not
exist today. Audio, Wi-Fi, application state and product-specific content belong
to the consuming applications.

## Install from GitHub

Linen is distributed through signed GitHub release tags, rather than package
registries. Pin both consumers to the same release.

```toml
[dependencies]
linen-tray = { git = "https://github.com/Hezric/Linen", tag = "v0.1.1" }
```

For a Vite+ frontend using npm 12, set `allow-git=root` in that frontend's
`.npmrc` first. This permits Git dependencies declared by the project itself.

```powershell
#requires -Version 7.0
vp add 'linen-ui@git+https://github.com/Hezric/Linen.git#v0.1.1'
```

```js
import { flyout, statusCard, createMotion } from 'linen-ui';
import 'linen-ui/style.css';
import { buildTray } from 'linen-ui/tooling/tray-icons.mjs';
```

The native crate requires Windows 11 and Tauri 2. UI and tooling consumers choose
their own application content. The source exports are the API; both apps provide
working integration examples. Neither npm nor crates.io receives a release.

### Use with an agent

Give an agent this prompt to prepare a checkout:

```text
Clone https://github.com/Hezric/Linen and check out signed tag v0.1.1.
Read AGENTS.md. Install Vite+ from its official guide and the stable MSVC
Rust toolchain with C++ Build Tools. Run vp install --frozen-lockfile,
vp run test, cargo test --locked, and vp run build. Keep application state
in consumers and use the GitHub tag when adding dependencies. Do not launch
or reconfigure a consumer application as part of checking the library.
```

## Develop

Use the current stable Rust toolchain and [Vite+](https://viteplus.dev/guide/).
Vite+ manages the pinned Node.js LTS runtime and package manager. On Windows,
install the MSVC C++ Build Tools and WebView2.

```powershell
#requires -Version 7.0
vp install --frozen-lockfile
cargo fmt -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
vp run test
vp run build
```

The reusable `windows-checks.yml` workflow accepts a Cargo manifest, a frontend
flag and a build command. `windows-release.yml` packages and verifies signed
releases. Consumers pin these workflows to the same tag as the library.

Repository policy, release reminders and Dependabot automation are shared too;
see [repository tooling](docs/features/repository-tooling.md) for their contracts.

[Contributing](CONTRIBUTING.md) · [Shell behavior](docs/features/tray-shell.md)
· [MIT license](LICENSE) · [Fluent glyph license](packages/linen-ui/glyphs/LICENSE)
