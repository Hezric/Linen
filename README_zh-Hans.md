<div align="center">
  <h1>🧵 Linen</h1>
  <p>把细小而用心的程序，织在一根线上。</p>
  <p><a href="https://github.com/Hezric/Linen/actions/workflows/ci.yml"><img src="https://github.com/Hezric/Linen/actions/workflows/ci.yml/badge.svg?branch=main" alt="持续集成"></a> <a href="https://github.com/Hezric/Linen/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue" alt="MIT 许可"></a></p>
  <p><a href="README.md">English</a> | 简体中文</p>
</div>

Linen 为小型应用提供共享基础。[Earmuff](https://github.com/Hezric/Earmuff) 和 [Homing](https://github.com/Hezric/Homing) 用它共建 Windows 托盘外壳与 Fluent 风格界面。`linen-tray` 提供 Tauri 外壳，`linen-ui` 提供不依赖框架的界面组件，`linen-tooling` 提供素材与可复用工作流；应用状态仍由各自的程序管理。

## 从 GitHub 安装

原生包和前端包请固定到同一个签名发布标签；Linen 不发布到 npm 或 crates.io。原生包需要 Windows 11 和 Tauri 2。

```toml
[dependencies]
linen-tray = { git = "https://github.com/Hezric/Linen", tag = "v0.1.5" }
```

使用 npm 12 的 Vite+ 前端，先在 `.npmrc` 设置 `allow-git=root`，再运行：

```powershell
vp add 'linen-ui@git+https://github.com/Hezric/Linen.git#v0.1.5'
```

```js
import { flyout, statusCard, createMotion } from 'linen-ui';
import 'linen-ui/style.css';
import { buildTray } from 'linen-ui/tooling/tray-icons.mjs';
```

## 交给智能体

```text
克隆 https://github.com/Hezric/Linen，检出签名标签 v0.1.5，先读 AGENTS.md。
准备 Vite+、稳定版 MSVC Rust 和 C++ Build Tools，运行 vp install --frozen-lockfile、
vp run test、cargo test --locked 和 vp run build。应用状态留在使用方，依赖固定到
GitHub 标签。检查库时不要启动或重新配置使用方应用。
```

[托盘外壳](https://github.com/Hezric/Linen/blob/main/docs/features/tray-shell.md) · [仓库工具](https://github.com/Hezric/Linen/blob/main/docs/features/repository-tooling.md) · [参与贡献与自动化测试](https://github.com/Hezric/Linen/blob/main/CONTRIBUTING.md) · [MIT 许可](https://github.com/Hezric/Linen/blob/main/LICENSE) · [Fluent 图形许可](https://github.com/Hezric/Linen/blob/main/packages/linen-ui/glyphs/LICENSE)
