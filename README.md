# uucode

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=flat&logo=rust&logoColor=white)](https://www.rust-lang.org/)

> Claude Code 状态栏增强工具 - 让你的 Claude Code 状态栏更强大、更美观

uucode 是一个专为 [Claude Code](https://claude.ai/code) 设计的状态栏增强工具，提供实时用量监控、成本追踪、Git 信息展示等丰富功能，支持完全自定义的主题和布局。

## 预览

![uucode 状态栏效果](assets/img1.png)

## 核心功能

### 状态栏增强
- **实时用量监控** - 显示 API 使用情况和账单信息
- **上下文窗口追踪** - 实时显示上下文使用百分比和 token 数量
- **成本统计** - 跟踪当前会话和总计成本
- **Git 集成** - 显示当前分支、仓库状态
- **模型信息** - 展示当前使用的 AI 模型
- **会话管理** - 显示会话 ID 和状态
- **目录信息** - 显示当前工作目录

### 可视化配置
- **TUI 配置界面** - 交互式终端界面，无需手动编辑配置文件
- **主题系统** - 内置多种预设主题，支持自定义颜色方案
- **图标选择器** - 丰富的图标库，自定义状态栏外观
- **实时预览** - 配置时即时预览效果

### Claude Code 增强
- **自动配置** - 一键配置 Claude Code settings.json
- **代码补丁** - 禁用上下文警告、ESC 中断提示等干扰信息
- **状态栏自动刷新** - 定期更新状态栏数据（30秒间隔）
- **Wrapper 模式** - 无缝注入 Claude Code，增强功能

## 安装

### 前置要求
- Rust 1.70+ (如果从源码构建)
- Claude Code CLI

### 从源码构建

```bash
# 克隆仓库
git clone https://github.com/hollegjx/uucode.git
cd uucode

# 构建项目
cargo build --release

# 安装到系统
cargo install --path .
```

### 二进制安装

从 [Releases](https://github.com/hollegjx/uucode/releases) 页面下载对应平台的二进制文件。

## 快速开始

### 1. 初始化配置

```bash
uucode --init
```

这将：
- 创建默认配置文件 `~/.claude/uucode/config.toml`
- 自动配置 Claude Code 的 `settings.json`

### 2. 配置状态栏（可选）

启动交互式配置界面：

```bash
uucode --config
```

或直接使用预设主题：

```bash
uucode --theme dracula
```

### 3. 应用 Claude Code 补丁（可选）

禁用 Claude Code 的上下文警告等干扰提示：

```bash
# 查找 Claude Code cli.js 文件路径
# Windows: %LOCALAPPDATA%\claude-ai\app-x.x.x\resources\app\dist\cli.js
# macOS: ~/Library/Application Support/Claude/app-x.x.x/resources/app/dist/cli.js
# Linux: ~/.config/Claude/app-x.x.x/resources/app/dist/cli.js

uucode --patch /path/to/cli.js
```

补丁功能：
- 禁用上下文容量低警告
- 禁用 ESC 中断显示
- 启用详细模式
- 添加状态栏自动刷新（30秒）

### 4. 启动 Wrapper 模式（推荐）

使用 uucode 包装器启动 Claude Code：

```bash
uucode --wrap [其他 Claude Code 参数]
```

## 使用方法

### 命令行选项

```bash
uucode [选项]

选项：
  -c, --config              启动 TUI 配置界面
  -t, --theme <THEME>       使用指定主题
      --print               打印当前配置
      --init                初始化配置文件
      --check               检查配置有效性
  -u, --update              检查更新
      --patch <PATH>        补丁 Claude Code cli.js 文件
      --wrap                启动 wrapper 模式
  -h, --help                显示帮助信息
  -V, --version             显示版本信息
```

### 配置文件

配置文件位于 `~/.claude/uucode/config.toml`，支持自定义：

- 状态栏段（segments）的顺序和内容
- 颜色方案和主题
- 图标和分隔符
- 显示格式和样式

示例配置：

```toml
[theme]
primary_color = "#89b4fa"
secondary_color = "#f38ba8"
success_color = "#a6e3a1"
warning_color = "#f9e2af"
error_color = "#f38ba8"

[segments.model]
enabled = true
icon = ""
format = "{icon} {model}"

[segments.cost]
enabled = true
icon = "$"
format = "{icon} {cost}"
```

### 管道输入模式

uucode 可以接收 Claude Code 的 JSON 数据并生成状态栏：

```bash
echo '{"model":"sonnet-4.5","context":{"used":68200,"total":200000}}' | uucode
```

## 状态栏段说明

| 段名称 | 说明 | 示例 |
|--------|------|------|
| `model` | 当前使用的 AI 模型 | Sonnet 4.5 |
| `directory` | 当前工作目录 | CCometixLine |
| `git` | Git 分支信息 | main |
| `context_window` | 上下文窗口使用情况 | 34.1% · 68.2k tokens |
| `cost` | 成本统计 | $8.68/$20 |
| `session` | 会话 ID | session-123 |
| `output_style` | 输出样式 | normal |
| `uucode_status` | uucode 服务状态 | 88code正版授权服务器 |
| `uucode_usage` | uucode 用量信息 | $8.68/$20 |
| `uucode_subscription` | uucode 订阅信息 | FREE ¥66.6/年 |

## 主题

uucode 内置多个预设主题，可通过 `--theme` 参数使用：

- `default` - 默认主题
- `dracula` - Dracula 配色
- `nord` - Nord 配色
- `solarized` - Solarized 配色
- `monokai` - Monokai 配色
- `gruvbox` - Gruvbox 配色

## 开发

### 项目结构

```
uucode/
├── src/
│   ├── api/            # API 客户端
│   ├── auto_config/    # Claude Code 自动配置
│   ├── config/         # 配置管理
│   ├── core/           # 核心功能
│   │   └── segments/   # 状态栏段实现
│   ├── ui/             # TUI 界面
│   │   ├── components/ # UI 组件
│   │   └── themes/     # 主题系统
│   ├── utils/          # 工具函数
│   ├── wrapper/        # Wrapper 模式
│   ├── cli.rs          # 命令行解析
│   ├── lib.rs          # 库入口
│   └── main.rs         # 程序入口
├── assets/             # 资源文件
├── Cargo.toml          # 项目配置
└── README.md
```

### 构建特性

```bash
# 构建所有功能
cargo build --release

# 仅构建核心功能（无 TUI）
cargo build --release --no-default-features

# 构建时启用 TUI
cargo build --release --features tui

# 构建时启用自更新
cargo build --release --features self-update
```

### 运行测试

```bash
cargo test
```

## 常见问题

### Q: 状态栏不显示？
A: 请确保已正确配置 Claude Code 的 `settings.json`，可运行 `uucode --init` 自动配置。

### Q: 如何还原 Claude Code 补丁？
A: 补丁操作会自动创建备份文件 `cli.js.backup`，只需将其复制回原位置即可还原。

### Q: 配置文件在哪里？
A: 配置文件位于：
- Windows: `%USERPROFILE%\.claude\uucode\config.toml`
- macOS/Linux: `~/.claude/uucode/config.toml`

### Q: 如何自定义状态栏？
A: 运行 `uucode --config` 启动交互式配置界面，或直接编辑配置文件。

## 贡献

欢迎贡献代码、报告问题或提出建议！

1. Fork 本仓库
2. 创建你的特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交你的更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 开启一个 Pull Request

## 许可证

本项目采用 MIT 许可证 - 查看 [LICENSE](LICENSE) 文件了解详情。

## 致谢

- [Claude Code](https://claude.ai/code) - Anthropic 的 AI 编程助手
- [ratatui](https://github.com/ratatui-org/ratatui) - 优秀的 Rust TUI 库
- 所有贡献者和用户

## 联系方式

- GitHub Issues: [https://github.com/hollegjx/uucode/issues](https://github.com/hollegjx/uucode/issues)

---

如果 uucode 对你有帮助，请给个 Star ⭐️
