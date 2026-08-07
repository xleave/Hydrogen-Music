import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { listen } from '@tauri-apps/api/event'
import { open } from '@tauri-apps/plugin-dialog'
import { openUrl, revealItemInDir } from '@tauri-apps/plugin-opener'

const callbacks = new Map()

function subscribe(name, callback) {
  callbacks.set(name, callback)
}

function emit(name, ...args) {
  callbacks.get(name)?.({}, ...args)
}

async function scanLocalMusic(params) {
  const settings = await invoke('get_settings')
  const folders = settings.local.localFolder
  const result = await invoke('scan_local_music', { folders })
  emit('localMusicCount', result.count)
  emit('localMusicFiles', { ...result, type: params.type })
}

const noop = () => {}

export function installWindowApi() {
  const appWindow = getCurrentWindow()

  // 监听 Rust 发来的 tray-hide 事件：隐藏到托盘前只保存播放列表，不退出
  listen('tray-hide', () => {
    emit('beforeTrayHide')
  })

  window.windowApi = {
    windowMin: () => appWindow.minimize(),
    windowMax: async () => (await appWindow.isMaximized()) ? appWindow.unmaximize() : appWindow.maximize(),
    // 直接关闭窗口：Rust on_window_event 会根据 quitApp 设置决定隐藏到托盘还是退出
    windowClose: () => appWindow.close(),
    toRegister: (url) => openUrl(url),
    // beforeQuit 事件由 tray-hide 触发，用于在驻留托盘前保存播放列表
    beforeQuit: (callback) => subscribe('beforeQuit', callback),
    // beforeTrayHide：仅保存播放列表，不退出（托盘隐藏时触发）
    beforeTrayHide: (callback) => subscribe('beforeTrayHide', callback),
    // exitApp：保存播放列表后调用 quit_app 命令强制退出（绕过 quitApp 设置）
    exitApp: async (playlist) => {
      await invoke('save_last_playlist', { playlist })
      await invoke('quit_app')
    },
    scanLocalMusic,
    localMusicFiles: (callback) => subscribe('localMusicFiles', callback),
    localMusicCount: (callback) => subscribe('localMusicCount', callback),
    getLocalMusicImage: (filePath) => invoke('read_cover', { filePath }),
    getLocalMusicLyric: (filePath) => invoke('read_lyrics', { filePath }),
    setSettings: (settings) => invoke('set_settings', { settings }),
    getSettings: () => invoke('get_settings'),
    openFile: () => open({ directory: true, multiple: false }),
    selectFile: () => open({ directory: false, multiple: false }),
    openLocalFolder: (path) => revealItemInDir(path),
    saveLastPlaylist: (playlist) => invoke('save_last_playlist', { playlist }),
    getLastPlaylist: () => invoke('get_last_playlist'),
    setWindowTile: (title) => appWindow.setTitle(title),
    copyTxt: (txt) => navigator.clipboard.writeText(txt),
    playOrPauseMusic: (callback) => subscribe('playOrPauseMusic', callback),
    lastOrNextMusic: (callback) => subscribe('lastOrNextMusic', callback),
    changeMusicPlaymode: (callback) => subscribe('changeMusicPlaymode', callback),
    volumeUp: (callback) => subscribe('volumeUp', callback),
    volumeDown: (callback) => subscribe('volumeDown', callback),
    musicProcessControl: (callback) => subscribe('musicProcessControl', callback),
    hidePlayer: (callback) => subscribe('hidePlayer', callback),
    lyricControl: (callback) => subscribe('lyricControl', callback),
    playOrPauseMusicCheck: noop,
    changeTrayMusicPlaymode: noop,
    registerShortcuts: noop,
    unregisterShortcuts: noop,
  }
}
