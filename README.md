# Hydrogen Music

Hydrogen Music 是一个保留原版界面的本地桌面音乐播放器。当前分支已将 Electron/Node.js 后端替换为 Tauri 2 + Rust，停止提供在线搜索、账号、云盘、下载、在线歌词和更新检查，所有音乐数据只从用户选择的本地目录读取。

## 功能

- 扫描一个或多个本地音乐目录及其子目录
- 按文件夹、歌手和专辑浏览本地音乐
- 读取标题、歌手、专辑、年份、流派、时长、码率、采样率和位深等标签
- 读取音频文件内嵌封面
- 读取音频文件内嵌歌词
- 读取音频文件同目录、同文件名的 UTF-8 `.lrc` 歌词
- 保存设置、播放队列和上次播放状态
- 顺序播放、列表循环、单曲循环和随机播放

歌词加载顺序固定为：音频文件内嵌歌词优先，同名 `.lrc` 文件其次。没有网络歌词回退。

```text
Music/
├── Example.flac
└── Example.lrc
```

## 本地格式

扫描器识别以下扩展名：

`mp3`、`flac`、`wav`、`aac`、`m4a`、`ogg`、`opus`、`wma`、`ape`、`alac`、`aiff`、`mp2`、`mpc`、`wv`、`speex`

Rust 标签读取覆盖 MP3/ID3、FLAC/Vorbis Comments、MP4/iTunes、OGG、Opus、WAV/RIFF、AIFF、APE、MPC 和 WavPack。无法解析标签时，歌曲仍会以文件名加入音乐库；具体音频能否播放由操作系统 WebView 的媒体解码能力决定。

## 技术结构

- 原有 Vue 3 页面、组件、CSS、图片和字体保持不变
- Tauri 2 负责桌面窗口、文件选择和本地文件协议
- Rust 负责目录扫描、音频标签、封面、歌词和本地状态持久化
- Lofty 负责音频元数据解析
- Howler.js 通过 Tauri 本地文件协议播放音频

主要目录：

```text
src/                    原有 Vue 界面与播放器逻辑
src/platform/           Tauri 与原 windowApi 的兼容层
src-tauri/src/          Rust 本地音乐库与状态实现
src-tauri/tauri.conf.json
```

## 开发

需要 Node.js 18+、Rust stable，以及 [Tauri 2 对应平台依赖](https://v2.tauri.app/start/prerequisites/)。

```bash
npm install
npm run start
```

仅构建前端：

```bash
npm run build
```

构建桌面安装包：

```bash
npm run dist
```

## 隐私与网络边界

应用不启动本地 HTTP 服务，也不请求音乐平台 API。旧界面中与在线服务相关的入口为保持 UI 完整而继续存在，但调用会由本地兼容层直接拒绝，不会发起网络连接。

## 参考

本地格式范围、标签读取策略和歌词优先级参考了 [myune_music_material](https://github.com/xleave/myune_music_material)。

## License

[Apache License 2.0](LICENSE)
