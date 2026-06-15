# Sotto

> *sotto voce* — 在下方，低声。
>
> 一句每日感悟，悬于桌面图标层，所有窗口之下。

Sotto 在每天首次启动时调用本机的 `claude` 或 `codex` CLI，生成一段克制而深刻的人生感悟，渲染在屏幕右下角一块磨砂面板上。窗口层级被压到桌面图标层（`kCGDesktopIconWindowLevel`）——它不抢焦点、不进 Dock、不出现在 `Cmd+Tab`，被任何普通应用窗口自然遮挡。

## 设计要点

- **CLI 优先级**：`claude -p` → fallback `codex exec`。两者都不可用时显示提示。
- **每日一句**：缓存在 `~/Library/Application Support/app.sotto.daily/daily.json`，跨过本地 00:05 自动生成新的一句。
- **窗口姿态**：透明、无装饰、不抢焦点、所有 Spaces 可见、不随 Spaces 滑动、不参与 `Cmd+\``。
- **极简交互**：双击窗口手动刷新。
- **极简视觉**：宋体 / Noto Serif，半透明磨砂玻璃，1.4 秒淡入。

## 运行

依赖：
- Rust（已在 PATH）
- Node 18+
- pnpm
- macOS 11+

```sh
pnpm install
pnpm tauri dev      # 开发模式
pnpm tauri build    # 打包 .app
```

首次 `cargo build` 会拉 Tauri 全家桶，需要几分钟。

## 配置

目前没有 GUI 配置面板。要调整：

| 想改什么 | 改哪里 |
|---|---|
| 提示词（人生感悟的风格） | `src-tauri/src/cli.rs::PROMPT` |
| 每天重新生成的时间 | `src-tauri/src/lib.rs::schedule_daily_refresh` 里的 `(0, 5, 0)` |
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
