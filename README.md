<div align="center">
  <h1>🧵 Linen</h1>
  <p>A common thread for small, considered programs.</p>
  <p><a href="https://github.com/Hezric/Linen/actions/workflows/ci.yml"><img src="https://github.com/Hezric/Linen/actions/workflows/ci.yml/badge.svg?branch=main" alt="CI"></a> <a href="https://github.com/Hezric/Linen/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue" alt="MIT license"></a></p>
  <p>English | <a href="README_zh-Hans.md">简体中文</a></p>
</div>

Linen is a shared foundation for small applications. [Earmuff](https://github.com/Hezric/Earmuff) and [Homing](https://github.com/Hezric/Homing) use its Windows tray shell and Fluent-inspired interface. `linen-tray` provides the Tauri shell, `linen-ui` provides framework-free components, and `linen-tooling` supplies assets and reusable workflows. Application state stays with each consumer.

## Install from GitHub

Pin the native and frontend packages to the same signed release tag; Linen is not published to npm or crates.io. The native crate requires Windows 11 and Tauri 2.

```toml
[dependencies]
linen-tray = { git = "https://github.com/Hezric/Linen", tag = "v0.3.1" }
```

For a Vite+ frontend using npm 12, set `allow-git=root` in `.npmrc`, then run:

```powershell
vp add 'linen-ui@git+https://github.com/Hezric/Linen.git#v0.3.1'
```

```js
import { flyout, statusCard, createMotion } from 'linen-ui';
import 'linen-ui/style.css';
import { buildTray } from 'linen-ui/tooling/tray-icons.mjs';
```

## Use with an agent

```text
Clone https://github.com/Hezric/Linen and check out signed tag v0.3.1.
Read AGENTS.md. Prepare Vite+, stable MSVC Rust and C++ Build Tools.
Run vp install --frozen-lockfile, vp run test, cargo test --locked and vp run build.
Keep application state in consumers and pin dependencies to the GitHub tag.
Do not launch or reconfigure a consumer while checking the library.
```

For current-source development, use the [project build cache](docs/features/build-cache.md).

[Tray shell](https://github.com/Hezric/Linen/blob/main/docs/features/tray-shell.md) · [Repository tooling](https://github.com/Hezric/Linen/blob/main/docs/features/repository-tooling.md) · [Contribute and automated tests](https://github.com/Hezric/Linen/blob/main/CONTRIBUTING.md) · [MIT license](https://github.com/Hezric/Linen/blob/main/LICENSE) · [Fluent glyph license](https://github.com/Hezric/Linen/blob/main/packages/linen-ui/glyphs/LICENSE)
