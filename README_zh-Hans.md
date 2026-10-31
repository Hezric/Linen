# Linen

把细小而用心的程序，织在一根线上。

[English](README.md)

Linen 是 Hezric 开源软件的共享基础。最早的两个使用者
[Earmuff](https://github.com/Hezric/Earmuff) 与
[Homing](https://github.com/Hezric/Homing)，共用 Windows 托盘外壳和简洁的 Fluent 风格界面。

## 包

| 包 | 用途 |
| --- | --- |
| `linen-tray` | Tauri 2 托盘菜单、随主题和任务栏模式更新的图标、浮窗定位与关闭、DWM 原生外观和动画。 |
| `linen-ui` | 不依赖前端框架的组件、设计变量、动画和共用 Fluent 图形，可用于任何界面程序。 |
| `linen-tooling` | 托盘 PNG/ICO 生成、中文字体裁剪、可复用 Windows CI 与发布工作流。 |

三个包分别位于 `packages/linen-tray`、`packages/linen-ui` 和 `tooling`。
仓库根目录的 JavaScript 清单是从 Git 安装 `linen-ui` 的入口；工具通过显式子路径导入，
不会加载 Rust 原生代码。Lineup 这样的命令行程序不依赖其中任何一个包。

只有两个程序需要同一份实现时，才增加共享包。等到两个程序共用非界面能力时，再增加
`linen-core`；现在不创建。音频、Wi-Fi、应用状态和各自的业务内容留在使用方。

## 从 GitHub 安装

Linen 通过 GitHub 上签名的发布标签分发，不发布到 npm 或 crates.io。
原生层与前端应固定到同一个版本。

```toml
[dependencies]
linen-tray = { git = "https://github.com/Hezric/Linen", tag = "v0.1.4" }
```

使用 npm 12 的 Vite+ 前端，先在自己的 `.npmrc` 中设置 `allow-git=root`，
只允许项目清单直接声明的 Git 依赖，再执行：

```powershell
#requires -Version 7.0
vp add 'linen-ui@git+https://github.com/Hezric/Linen.git#v0.1.4'
```

```js
import { flyout, statusCard, createMotion } from 'linen-ui';
import 'linen-ui/style.css';
import { buildTray } from 'linen-ui/tooling/tray-icons.mjs';
```

原生包要求 Windows 11 与 Tauri 2。界面和工具包不决定应用内容；源码导出就是接口，
两个应用提供完整接入示例。所有版本只从 GitHub 获取。

### 交给智能体

可以把下面的提示完整交给智能体：

```text
克隆 https://github.com/Hezric/Linen，检出签名标签 v0.1.4。
先读 AGENTS.md。按官方指南准备 Vite+、稳定版 MSVC Rust 工具链和 C++ Build Tools。
运行 vp install --frozen-lockfile、vp run test、cargo test --locked 和 vp run build。
应用状态留在使用方；接入依赖时固定 GitHub 标签。检查库时不要启动或重新配置使用方应用。
```

## 开发

使用最新稳定版 Rust 和 [Vite+](https://viteplus.dev/guide/)。Vite+ 管理固定的 Node.js LTS
与包管理器版本。Windows 还需要 MSVC C++ Build Tools 和 WebView2。

```powershell
#requires -Version 7.0
vp install --frozen-lockfile
cargo fmt -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
vp run test
vp run build
```

可复用的 `windows-checks.yml` 接收 Cargo 清单路径、是否有前端和构建命令。
`windows-release.yml` 负责打包并校验签名发布。使用方把工作流与库固定到同一个标签。

仓库策略、发布提醒和 Dependabot 自动化也由这里共用；
具体约定见[仓库工具](docs/features/repository-tooling.md)。

[参与开发](CONTRIBUTING.md) · [托盘行为](docs/features/tray-shell.md)
· [MIT 许可](LICENSE) · [Fluent 图形许可](packages/linen-ui/glyphs/LICENSE)
