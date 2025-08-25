# wx-uploader

一个用于上传 Markdown 文件到微信公众号的命令行工具，支持多账号管理和 AI 自动生成封面图片。

## 安装

从 crates.io 直接安装：

```bash
cargo install wx-uploader
```

或从源码构建：

```bash
git clone https://github.com/tyrchen/wx-uploader.git
cd wx-uploader
cargo install --path .
```

## 前置条件

您可以通过两种方式配置 wx-uploader：环境变量（单账号）或配置文件（多账号）。

### 方式一：环境变量（单账号）

```bash
# 必需：微信公众号凭证
export WECHAT_APP_ID="your_app_id"
export WECHAT_APP_SECRET="your_app_secret"

# 可选：AI 提供商配置，用于自动生成封面图
# 选项 1：OpenAI（默认）
export OPENAI_API_KEY="your_openai_api_key"

# 选项 2：Google Gemini（支持文本和图像生成）
export AI_PROVIDER="gemini"
export GEMINI_API_KEY="your_gemini_api_key"
```

### 方式二：配置文件（多账号支持）

创建配置文件来管理多个微信公众号：

```bash
# 生成示例配置文件
wx-uploader --init-config config.yaml
```

这会创建一个配置文件，如下所示：

```yaml
# 示例配置文件
accounts:
  personal:
    name: "personal"
    app_id: "your_personal_app_id_here"
    app_secret: "your_personal_app_secret_here"
    description: "个人微信公众号"
  
  work:
    name: "work"
    app_id: "your_work_app_id_here"
    app_secret: "your_work_app_secret_here"
    description: "工作微信公众号"

default_account: "personal"

ai_provider:
  provider: "openai"  # 或 "gemini"
  openai:
    api_key: "your_openai_api_key_here"
  gemini:
    api_key: "your_gemini_api_key_here"

settings:
  verbose: false
```

## 使用方法

### 基础使用（单账号）

```bash
# 上传所有 frontmatter 中没有 `published: true` 的 .md 文件
wx-uploader .

# 从指定目录上传
wx-uploader ./posts

# 启用详细输出
wx-uploader --verbose ./posts

# 使用指定的 AI 提供商
wx-uploader --provider gemini ./posts

# 使用自定义 AI API 密钥
wx-uploader --ai-key your_custom_key ./posts
```

### 多账号使用

```bash
# 列出配置文件中的可用账号
wx-uploader --config config.yaml --list-accounts

# 使用配置文件中的指定账号上传
wx-uploader --config config.yaml --account work ./posts

# 使用默认账号上传
wx-uploader --config config.yaml ./posts

# 为此次上传覆盖 AI 提供商
wx-uploader --config config.yaml --provider gemini --account personal ./posts
```

### 上传指定文件

```bash
# 强制上传指定文件（忽略发布状态）
wx-uploader ./2025/08/01-chat-with-ai.md

# 使用多账号配置上传指定文件
wx-uploader --config config.yaml --account work ./article.md
```

### 命令行选项

```bash
wx-uploader [选项] [路径]

选项:
    -c, --config <文件>        配置文件路径（YAML 或 JSON）
    -a, --account <名称>       使用配置文件中的账号名称
    -p, --provider <提供商>    AI 提供商：openai, gemini [默认: openai]
        --ai-key <密钥>        AI API 密钥（覆盖配置/环境变量）
    -v, --verbose              启用详细输出
        --list-accounts        列出配置文件中的可用账号
        --init-config <文件>   生成示例配置文件
    -h, --help                 显示帮助信息
    -V, --version              显示版本信息
```

## 工作原理

1. 工具扫描带有 YAML frontmatter 的 Markdown 文件
2. 如果文件的 frontmatter 中没有 `published: true`，则会被上传
3. 如果没有指定封面图片且配置了 OpenAI API 密钥，将使用 GPT-5 和 gpt-image-1 生成吉卜力风格的封面图
4. 指定单个文件时，无论其发布状态如何都会被上传
5. 上传成功后，frontmatter 会更新为 `published: draft` 并包含生成的封面文件名（如果有）

## Frontmatter 示例

```yaml
---
title: 我的文章标题
published: draft  # 或 'true' 以跳过上传
cover: cover.png  # 可选，如果缺失且设置了 OpenAI 密钥则自动生成
description: 文章描述
author: 作者姓名
theme: lapis  # 可选主题
---

您的 Markdown 内容在这里...
```

## AI 封面生成

当设置了 `OPENAI_API_KEY` 环境变量时，工具会为没有指定封面的文章自动生成精美的封面图片。

### 工作原理：

1. **内容分析**：GPT-5-mini 分析您的 Markdown 内容以创建生动的场景描述
2. **提示词生成**：创建优化的提示词，专注于吉卜力风格的艺术作品
3. **图像生成**：gpt-image-1 生成高质量的 16:9 宽高比封面图片
4. **自动保存**：下载并保存图片到与 Markdown 文件相同的目录
5. **元数据更新**：使用生成的封面文件名更新 frontmatter

### 特性：

- **吉卜力风格**：美丽的艺术美学，柔和的色彩和自然元素
- **内容感知**：场景描述基于您的实际文章内容
- **高质量**：1536x1024 分辨率，优化用于网页显示
- **自动命名**：生成的文件使用唯一名称以防止冲突
- **优雅降级**：如果图像生成失败，继续正常上传流程
- **Base64 支持**：同时处理 URL 和 base64 编码的图像响应

### 输出示例：

对于一篇关于"构建 Rust 应用程序"的文章，AI 可能会生成这样的场景：
> "一个舒适的工作坊，充满了精致的齿轮和发光的机械工具，工匠正在仔细组装发条装置。温暖的金色光线透过高窗洒进来，照亮了像萤火虫一样在尘埃中闪烁的漂浮锈粒子。"

这会变成一幅美丽的吉卜力风格封面图片，视觉化地呈现您的内容。

## 功能特性

- 📝 **批量上传**：处理整个目录的 Markdown 文件
- 🎨 **AI 封面生成**：使用 OpenAI 最新模型自动生成封面图片
- 🔄 **智能处理**：跳过已发布的文章
- 📊 **进度跟踪**：带有彩色状态指示器的清晰控制台输出
- 🛡️ **错误恢复**：优雅地处理 API 失败
- 🔐 **安全**：API 密钥仅存储在环境变量中

## 开发

### 运行测试

项目包含全面的单元测试和集成测试：

```bash
# 运行所有测试
cargo test

# 带输出运行测试
cargo test -- --nocapture

# 运行特定测试模块
cargo test test_frontmatter

# 仅运行集成测试
cargo test --test integration_tests
```

### 代码质量

```bash
# 运行 clippy 进行代码检查
cargo clippy --all-targets --all-features

# 检查安全漏洞
cargo audit

# 格式化代码
cargo fmt

# 生成文档
cargo doc --open
```

### 项目结构

```
wx-uploader/
├── src/
│   ├── main.rs          # CLI 入口点
│   ├── lib.rs           # 公共 API
│   ├── cli.rs           # 命令行接口
│   ├── error.rs         # 错误处理
│   ├── models.rs        # 数据结构
│   ├── markdown.rs      # Markdown 解析
│   ├── openai.rs        # AI 集成
│   ├── output.rs        # 控制台输出格式化
│   └── wechat.rs        # 微信 API 集成
└── tests/
    └── integration_tests.rs  # 集成测试
```

## 注意事项

- 目录扫描时会跳过带有 `published: true` 的文件
- 单文件上传总是强制上传，无论发布状态如何
- 工具在更新时会保留所有其他 frontmatter 字段
- 封面图片保存在与 Markdown 文件相同的目录中
- 支持 published 字段的字符串（`"true"`）和布尔值（`true`）格式
- 配置文件支持 YAML（`.yaml`, `.yml`）和 JSON（`.json`）格式
- 账号切换无缝，无需重启工具

## 配置优先级

工具按以下优先级顺序进行配置：

1. **命令行标志**（最高优先级）：`--provider`、`--ai-key`、`--verbose`
2. **配置文件**：来自 `--config` 文件的设置
3. **环境变量**：`WECHAT_APP_ID`、`OPENAI_API_KEY` 等
4. **默认值**（最低优先级）

## 多账号工作流程

### 设置工作流程
```bash
# 1. 生成配置模板
wx-uploader --init-config my-accounts.yaml

# 2. 编辑文件，填入您的实际凭证
# 将占位符值替换为真实的 app_id、app_secret 和 API 密钥

# 3. 列出配置的账号
wx-uploader --config my-accounts.yaml --list-accounts

# 4. 使用指定账号测试上传
wx-uploader --config my-accounts.yaml --account personal ./test-article.md
```

### 日常使用工作流程
```bash
# 上传个人博客文章
wx-uploader --config my-accounts.yaml --account personal ./blog/

# 使用不同的 AI 提供商上传工作文章
wx-uploader --config my-accounts.yaml --account work --provider gemini ./work-posts/

# 使用默认账号快速上传
wx-uploader --config my-accounts.yaml ./quick-post.md
```

## 许可证

MIT

## 贡献

欢迎贡献！请随时提交 Pull Request。
