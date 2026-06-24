# Sotto

> *sotto voce* — 在下方，低声。
>
> 一句每日感悟，悬于桌面图标层，所有窗口之下。

Sotto 在每天首次启动时调用本机的 `claude` 或 `codex` CLI，生成一句克制、具体、带一点余味的日常观察，渲染在屏幕右下角一块磨砂面板上。窗口层级被压到桌面图标层（`kCGDesktopIconWindowLevel`）——它不抢焦点、不进 Dock、不出现在 `Cmd+Tab`，被任何普通应用窗口自然遮挡。

## 设计要点

- **CLI 优先级**：`claude -p` → fallback `codex exec`。两者都不可用时显示提示。
- **每日一句**：缓存在 `~/Library/Application Support/app.sotto.daily/daily.json`，跨过本地 00:05 自动生成新的一句。
- **内容避重**：生成前会给 AI 一个可自由偏离的灵感起点、观察角度、语气和语言质地，并参考最近历史，避开常见格言句式和重复意象。
- **千人千面**：支持风格包、生成节律、语气强度和私有关键词，让每台机器长出自己的内容口味。
- **分享卡片**：一键下载 1080×1440 极简 PNG，底部带 Sotto 安装入口，适合朋友圈传播。
- **历史回看**：保留最近内容，喜欢的句子可以回看、复用和分享。
- **轻量赞赏**：设置页内置赞赏入口，独立弹层展示，不打断主体验。
- **窗口姿态**：透明、无装饰、不抢焦点、所有 Spaces 可见、不随 Spaces 滑动、不参与 `Cmd+\``。
- **极简交互**：右上角按钮手动刷新。
- **极简视觉**：宋体 / Noto Serif，半透明磨砂玻璃，1.4 秒淡入。

## 安装（普通用户）

> macOS 11 +，需要本机已经装好 [`claude`](https://docs.claude.com/en/docs/claude-code/overview) 或 `codex` CLI 并能在终端跑。

如果首次打开被 macOS 拦住，最直接的处理方式是：

```sh
xattr -dr com.apple.quarantine /Applications/Sotto.app
open /Applications/Sotto.app
```

只对你确认来自本仓库 Release 的 Sotto 使用这条命令。

1. 到 [Releases](https://github.com/jokeuncle/sotto/releases) 下载最新的 `Sotto_*.dmg`
2. 打开 dmg，把 **Sotto.app** 拖到 `Applications`
3. **首次启动**：在访达里**右键（或 Control + 点击）Sotto.app → 打开**，弹窗里再点一次"打开"
   （应用未做 Apple 公证，第一次必须走这条路绕过 Gatekeeper；之后双击就行）
4. Sotto 不会出现在 Dock 和 `Cmd+Tab` 里。窗口出现在主屏右下角，每天首次启动会调你机器上的 `claude` 生成一句话；想换一句就把鼠标移到面板上、点右上角那个淡淡的刷新图标。

上面的 `xattr` 命令会移除下载文件的 quarantine 标记；更彻底的方案是后续版本补 Apple Developer ID 签名和 notarization。

如果 Sotto 报"未找到可用的 claude 或 codex CLI"：你的 CLI 不在常见路径里。打开终端 `which claude` 查路径，然后参考 [已知坑](#已知坑) 那一节把路径加到代码里。

## Homebrew 安装

推荐使用个人 tap：

```sh
brew tap jokeuncle/sotto
brew install --cask sotto
```

也可以不 tap，直接安装 cask 文件：

```sh
brew install --cask https://raw.githubusercontent.com/jokeuncle/sotto/main/Casks/sotto.rb
```

这个 cask 会在安装后自动清理 quarantine 标记。少数情况下如果首次打开仍被 Gatekeeper 拦截，处理方式同上：

```sh
xattr -dr com.apple.quarantine /Applications/Sotto.app
open /Applications/Sotto.app
```

对应 cask 的 `caveats` 应明确提示：

```ruby
caveats <<~EOS
  First launch note:
  The cask automatically clears quarantine after install. If macOS still says
  "damaged", "can't be opened", or blocks the first launch, run:

    xattr -dr com.apple.quarantine /Applications/Sotto.app
    open /Applications/Sotto.app

  Only run this for a Sotto release you trust.
EOS
```

## 从源码构建

依赖：Rust、Node 18+、pnpm、macOS 11+。

```sh
pnpm install
pnpm tauri dev      # 开发模式：1 秒内开窗，热重载
pnpm tauri build    # 打包：输出在 src-tauri/target/release/bundle/{macos,dmg}
```

首次 `cargo build` 会拉 Tauri 全家桶，需要几分钟。

## 配置

大部分偏好可以在应用右上角设置里调整。开发时也可以改这些位置：

| 想改什么 | 改哪里 |
|---|---|
| 提示词素材池、禁用句式、质量过滤 | `src-tauri/src/cli.rs` |
| 默认风格、节律、语气和私有关键词 | 应用内右上角设置按钮 |
| 历史记录数量 | `src-tauri/src/lib.rs` 的 `HISTORY_PROMPT_LIMIT` / `HISTORY_STORE_LIMIT` |
| 自动生成时段 | `src-tauri/src/lib.rs::SCHEDULE_SLOTS` |
| 窗口位置 | `src-tauri/src/lib.rs::position_bottom_right` |
| 窗口尺寸 | `src-tauri/tauri.conf.json` 的 `windows[0].width/height` |
| 视觉样式 | `src/style.css` |

## 已知坑

- **CLI 找不到**：macOS 下 GUI app 启动时不继承 shell 的 PATH。`src-tauri/src/cli.rs::enriched_path` 已经把常见路径（`/opt/homebrew/bin`、`~/.local/bin`、`~/.cargo/bin` 等）都加进来，但如果你的 `claude` / `codex` 装在别处，需要在 `enriched_path` 里补一行。
- **桌面图标层级**：极少数 macOS 版本下，把窗口压到 `kCGDesktopIconWindowLevel` 会让窗口点击事件被 Finder 抢走。如果出现这种情况，把 `src-tauri/src/macos.rs::pin_to_desktop_level` 里的 level key 从 `18` 改成 `4`（普通窗口层），代价是不再"沉到桌面"。
- **icon 是占位图**：`src-tauri/icons/*` 是脚本生成的纯色光点。要换成真正的 logo，用 `pnpm tauri icon path/to/source.png` 重新生成。

## 目录结构

```
sotto/
├── index.html
├── src/
│   ├── main.js          # invoke get_today / 监听 aphorism 事件
│   └── style.css        # 极简磨砂面板
├── src-tauri/
│   ├── Cargo.toml
│   ├── tauri.conf.json  # 窗口配置（transparent / decorations:false / focus:false）
│   ├── capabilities/
│   ├── icons/           # 占位图
│   └── src/
│       ├── main.rs
│       ├── lib.rs       # setup + 定时任务 + commands
│       ├── cli.rs       # claude / codex 调用 + PATH 兜底
│       ├── storage.rs   # 当日缓存
│       └── macos.rs     # NSApplicationActivationPolicyAccessory + NSWindow level
└── package.json
```
