# TinyTodo

TinyTodo 是一个本地优先的 Windows 桌面待办工具，使用 Tauri 2、Vue 3、TypeScript、Rust 和 SQLite。

## 功能

- 今天、收集箱、计划、已完成和自定义清单
- 快速添加、日期和时间、优先级、备注、自动保存
- 拖拽排序、跨任务搜索、清单重命名和删除
- 系统托盘常驻，关闭主窗口时收起到托盘
- 全局快捷记录窗口，可在设置中切换快捷键
- 任务数据保存在本机 SQLite 数据库，不需要登录或网络

## 开发运行

需要 Node.js、pnpm、Rust stable、Windows C++ Build Tools 和 WebView2。

```powershell
pnpm install
pnpm tauri dev
```

## 测试和构建

```powershell
pnpm test
pnpm build
cd src-tauri
cargo test
cd ..
pnpm tauri build --no-bundle
```

Windows 可执行文件输出到 `src-tauri/target/release/tinytodo.exe`。启用 installer bundling 后，可用 Tauri CLI 生成 MSI 或 NSIS 安装包。

## Windows 发布与自动更新

推送版本标签（例如 `v0.2.0`）后，GitHub Actions 会构建 Windows x64 的 NSIS 和 MSI 安装包，并创建 GitHub Release。NSIS 安装包用于应用内自动更新；MSI 可供手动安装。应用设置中的“检查更新”会从最新 Release 检查并安装更新。

首次发布前，在 GitHub 仓库 **Settings → Secrets and variables → Actions** 添加 `TAURI_SIGNING_PRIVATE_KEY`，值为本机 Tauri 更新器私钥文件的完整内容。当前私钥文件保存在 `%LOCALAPPDATA%\TinyTodo\tinytodo-updater.key`；请安全备份，绝不要提交或分享。当前密钥未设置密码，因此不需要 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`；若以后换成带密码的密钥，再添加该 Secret。

发布新版本时，在仓库根目录执行：

```powershell
pnpm release:prepare 0.2.0
git diff
git add package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "chore: release v0.2.0"
git tag v0.2.0
git push origin main
git push origin v0.2.0
```

版本标签必须与 `package.json`、`src-tauri/tauri.conf.json` 和 Rust crate 版本一致。发布工作流会校验版本，构建签名安装包，生成 `latest.json` 并上传到 Release。只要 Release 使用同一更新器私钥签名，已安装的 Windows 版本就能验证并安装它。

## 快捷键

默认尝试 `Ctrl + Shift + Space`。若被其他程序占用，启动时会依次尝试 `Ctrl + Alt + Space` 和 `Alt + Shift + T`。当前生效组合显示在侧栏，也可以在右上角设置中更改。

## 数据

数据库文件名为 `tinytodo.sqlite3`，保存在 Tauri 为当前应用分配的本地应用数据目录中。清单默认提供“工作”“学习”“生活”；删除清单只会解除任务关联，任务本身会保留在收集箱。
