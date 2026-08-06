# Hydrogen Music

Hydrogen Music 是一款面向本地音乐库的桌面播放器。项目保留原版 Vue 3 界面与交互风格，将 Electron/Node.js 桌面后端重写为 Tauri 2 + Rust，并移除在线音乐平台依赖，让音乐扫描、标签读取、封面、歌词和播放队列都围绕本地文件工作。

当前版本不提供账号登录、在线搜索、云盘、音乐下载、在线歌词、在线更新检查或本地 HTTP 服务。旧界面中与在线功能相关的视觉入口仍被保留，以保证 UI 文件没有删改；这些入口不会发起网络请求。

## 当前功能

- 添加一个或多个音乐目录，递归扫描其中的本地音频文件
- 按目录、歌手和专辑整理本地音乐
- 读取标题、歌手、专辑、专辑歌手、年份、日期、流派、时长、码率、采样率和位深
- 读取音频标签中的内嵌封面
- 读取音频文件中的内嵌歌词
- 读取音频文件旁的同名 UTF-8 `.lrc` 歌词
- 保存应用设置、播放队列和上次播放状态
- 支持顺序播放、列表循环、单曲循环和随机播放
- 支持 Windows、Linux 和 macOS 的 Tauri 桌面构建，并提供 Fedora RPM 构建产物

歌词读取顺序为：

1. 音频文件内嵌歌词
2. 同目录、同文件名的 `.lrc` 文件

例如：

```text
Music/
├── Example.flac
└── Example.lrc
```

没有本地歌词时保持无歌词状态，不会回退到网络歌词源。

## 支持的本地文件

音乐库扫描器识别以下扩展名：

`mp3`、`flac`、`wav`、`aac`、`m4a`、`ogg`、`opus`、`wma`、`ape`、`alac`、`aiff`、`mp2`、`mpc`、`wv`、`speex`

Rust 标签解析覆盖 MP3/ID3、FLAC/Vorbis Comments、MP4/iTunes、OGG、Opus、WAV/RIFF、AIFF、APE、MPC 和 WavPack。遇到无法解析标签的文件时，歌曲仍会使用文件名加入音乐库，不会中断整个目录扫描。

最终播放能力取决于操作系统 WebView 提供的媒体解码器。MP3、FLAC、WAV、AAC/M4A、OGG 和 Opus 是播放器优先支持的格式；WMA、APE 等格式在不同系统上的可播放性可能不同。

## 架构

```text
Vue 3 原版 UI
    │
    ├── src/platform/windowApi.js    原 Electron API 兼容层
    │
    └── Tauri IPC
            │
            ├── library.rs           目录扫描与音频元数据
            ├── lib.rs               封面、歌词与 Tauri 命令
            └── storage.rs           设置和播放队列持久化
```

- `src/`：原有 Vue 页面、组件、样式和播放器交互
- `src/platform/`：Tauri 调用适配与网络功能关闭入口
- `src-tauri/`：Rust 桌面后端、权限和打包配置
- `.github/workflows/`：前端构建与 Rust 编译检查

## 本次重写的优化点

- 用 Rust 替换 Electron 主进程、预加载脚本和 Node.js 文件扫描代码
- 删除内置网易云 API 服务、Axios 网络通道、下载器和自动更新器
- 使用稳定的文件路径作为本地歌曲标识，刷新音乐库后不会生成无意义的随机 ID
- 目录扫描失败与单个标签解析失败分开处理，单个异常文件不会破坏整个音乐库
- 将内嵌歌词和独立 `.lrc` 歌词接回现有歌词组件
- 从音频标签直接生成封面 Data URL，不写入额外缓存文件
- 设置和播放队列由 Rust 写入系统应用数据目录
- 通过 Tauri 本地资源协议加载音乐文件，不启动本地 Web 服务
- 保留原版 `.vue`、CSS、图片和字体，不进行 UI 重构
- GitHub Actions 使用 Node.js 24 执行前端生产构建和 Rust `cargo check`
- 在 Fedora 42 容器中构建 RPM，并将安装包作为 CI 产物上传

## 开发环境

需要：

- Node.js 24
- Rust stable
- 当前平台对应的 [Tauri 2 系统依赖](https://v2.tauri.app/start/prerequisites/)

安装依赖：

```bash
npm install
```

启动桌面开发环境：

```bash
npm run start
```

仅检查 Vue 前端生产构建：

```bash
npm run build
```

检查 Rust 后端：

```bash
cargo check --manifest-path src-tauri/Cargo.toml
```

构建桌面安装包：

```bash
npm run dist
```

## Fedora RPM

每次向 `agent/rust-local-player` 分支推送提交时，GitHub Actions 都会在 Fedora 42 容器中完成原生 RPM 构建。构建成功后，可在对应的 Actions 运行页面下载名为 `hydrogen-music-fedora-rpm` 的产物。

在 Fedora 本机编译时，先安装系统依赖：

```bash
sudo dnf install webkit2gtk4.1-devel openssl-devel curl wget file \
  libappindicator-gtk3-devel librsvg2-devel libxdo-devel \
  gcc gcc-c++ make rpm-build patchelf nodejs npm
```

然后安装前端依赖并构建 RPM：

```bash
npm ci
npm run tauri -- build --bundles rpm
```

生成的安装包位于 `src-tauri/target/release/bundle/rpm/`。

## 隐私边界

播放器只读取用户主动选择的音乐目录，并在系统应用数据目录保存设置和播放状态。音乐扫描、元数据、封面和歌词处理均在本机完成。应用不会上传音乐文件，也不会请求第三方音乐服务。

## 致谢

- 感谢 [Kaidesuyo/Hydrogen-Music](https://github.com/Kaidesuyo/Hydrogen-Music) 原项目作者与贡献者提供完整的界面设计、播放器交互和项目基础。
- 感谢 [xleave/myune_music_material](https://github.com/xleave/myune_music_material) 提供本地音乐格式、元数据处理和歌词读取策略参考。
- 感谢 [Tauri](https://tauri.app/)、[Vue.js](https://vuejs.org/)、[Lofty](https://github.com/Serial-ATA/lofty-rs) 与 [Howler.js](https://howlerjs.com/) 等开源项目。

## 许可证

本项目继续使用 [Apache License 2.0](LICENSE)。
