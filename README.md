# Hydrogen Music

Hydrogen Music 是由 **xleave** 维护的本地桌面音乐播放器，当前版本为 **0.7.0**。应用使用 Vue 3 保留既有播放器视觉风格，以 Tauri 2 和 Rust 提供本地目录扫描、音频元数据读取、封面与歌词加载、设置保存和桌面打包能力。

应用启动后直接进入本地音乐界面，不需要账号、登录、Cookie、Token 或权限鉴定。项目不包含网易云音乐、云盘、在线搜索、在线推荐、在线播放、音乐下载、在线视频和自动更新功能，也不会启动本地 HTTP 服务。

## 功能

- 添加一个或多个本地音乐目录并递归扫描
- 按文件夹、专辑和歌手浏览音乐
- 读取标题、歌手、专辑、年份、流派、时长、码率、采样率和位深
- 读取音频文件中的内嵌封面
- 读取音频文件中的内嵌歌词
- 加载音频旁同名的 UTF-8 `.lrc` 文件
- 保存播放队列、播放位置、音量、播放模式和应用设置
- 支持顺序播放、列表循环、单曲循环和随机播放
- 通过 Tauri 本地资源协议播放文件
- 在 GitHub Actions 中构建 Fedora 42 x86_64 RPM

歌词读取顺序：

1. 音频文件内嵌歌词
2. 同目录、同文件名的 `.lrc` 文件

```text
Music/
├── Example.flac
└── Example.lrc
```

没有歌词时保持无歌词状态，不访问网络歌词源。

## 支持格式

音乐库扫描器识别：

`mp3`、`flac`、`wav`、`aac`、`m4a`、`ogg`、`opus`、`wma`、`ape`、`alac`、`aiff`、`mp2`、`mpc`、`wv`、`speex`

Rust 后端负责解析 MP3/ID3、FLAC/Vorbis Comments、MP4/iTunes、OGG、Opus、WAV/RIFF、AIFF、APE、MPC 和 WavPack 元数据。无法读取标签时仍会使用文件名加入音乐库。

最终解码能力由操作系统 WebView 的媒体支持决定。MP3、FLAC、WAV、AAC/M4A、OGG 和 Opus 是优先支持格式。

## 项目结构

- `src/`：本地音乐界面、播放器和 Tauri 调用适配
- `src-tauri/`：Rust 目录扫描、标签解析、歌词读取和本地持久化
- `.github/workflows/`：前端检查、Rust 检查和 Fedora RPM 构建

## 架构与优化

当前版本围绕纯本地播放完成了以下调整：

- 删除账号登录、Cookie 鉴权和登录路由
- 删除网易云 API、云盘、推荐、搜索、收藏和下载模块
- 删除在线音乐视频和远程封面地址
- 启动路径固定进入本地音乐库
- 使用稳定文件路径生成本地歌曲标识
- 扫描、标签、封面和歌词处理全部在本机完成
- 统一前端、Tauri 和 Rust 版本号为 `0.7.0`
- GitHub Actions 使用 Node.js 24，并在 Fedora 42 环境生成 RPM

播放器核心按职责拆分为播放控制、播放队列、歌词、时间格式化和生命周期管理模块。全局事件与播放进度调度具备明确的注册、取消和热更新清理流程，避免重复监听和残留定时任务。

面向大型本地音乐库，歌曲列表使用虚拟滚动；播放队列仅持久化歌曲标识和播放索引，并在扫描完成后从本地元数据重建，减少 JSON 序列化、IPC 传输和磁盘写入。歌词渲染使用 Vue 样式绑定和二分定位当前行，不再逐行直接修改 DOM。

Rust 后端在内存中缓存设置并在修改时同步落盘，窗口关闭判断不再重复读取配置文件。前端未捕获异常会写入应用日志目录的 `frontend-errors.log`，Rust panic 会写入同目录的 `crash.log`，便于定位静默失败。

## 开发

环境要求：

- Node.js 24
- Rust stable
- 当前系统对应的 [Tauri 2 系统依赖](https://v2.tauri.app/start/prerequisites/)

```bash
npm ci
npm run start
```

执行前端构建和 Rust 检查：

```bash
npm run build
cargo check --manifest-path src-tauri/Cargo.toml
```

## Fedora RPM

Fedora 本机依赖：

```bash
sudo dnf install webkit2gtk4.1-devel openssl-devel curl wget file \
  libappindicator-gtk3-devel librsvg2-devel libxdo-devel \
  gcc gcc-c++ make rpm-build patchelf nodejs npm
```

构建安装包：

```bash
npm ci
npm run tauri -- build --bundles rpm
```

RPM 输出目录为 `src-tauri/target/release/bundle/rpm/`。推送到 `agent/rust-local-player` 分支后，也可从对应的 GitHub Actions 运行页面下载 `hydrogen-music-fedora-rpm` 产物。

## 隐私

播放器只读取用户主动选择的音乐目录，并在系统应用数据目录保存设置和播放状态。音乐文件、标签、封面、歌词和播放记录不会上传到网络服务。

## 致谢

感谢 [xleave/myune_music_material](https://github.com/xleave/myune_music_material) 提供本地音乐格式、元数据和歌词读取策略参考；感谢 Tauri、Vue.js、Lofty、Howler.js 及相关开源项目的贡献者。

## 许可证

本项目使用 [MIT License](LICENSE)。
