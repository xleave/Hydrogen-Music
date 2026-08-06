import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
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

  window.windowApi = {
    windowMin: () => appWindow.minimize(),
    windowMax: async () => (await appWindow.isMaximized()) ? appWindow.unmaximize() : appWindow.maximize(),
    windowClose: () => appWindow.close(),
    toRegister: (url) => openUrl(url),
    beforeQuit: (callback) => subscribe('beforeQuit', callback),
    exitApp: async (playlist) => {
      await invoke('save_last_playlist', { playlist })
      await appWindow.close()
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
