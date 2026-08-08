import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { listen } from '@tauri-apps/api/event'

const callbacks = new Map()
let scanRequestId = 0
let exiting = false

function subscribe(name, callback) {
  if (!callbacks.has(name)) callbacks.set(name, new Set())
  callbacks.get(name).add(callback)
  return () => callbacks.get(name)?.delete(callback)
}

function emit(name, ...args) {
  const results = []
  for (const callback of callbacks.get(name) || []) results.push(callback({}, ...args))
  return results
}

async function runExitFlush(playlist) {
  if (exiting) return
  exiting = true
  try {
    if (playlist !== undefined) await invoke('save_last_playlist', { playlist })
    const pending = emit('beforeQuit')
    await Promise.allSettled(pending.map((value) => Promise.resolve(value)))
    if (document.querySelector('.settings-page')) {
      await new Promise((resolve) => setTimeout(resolve, 800))
    }
    await invoke('quit_app')
  } catch (error) {
    exiting = false
    throw error
  }
}

async function scanLocalMusic(params = {}) {
  const requestId = ++scanRequestId
  try {
    const result = await invoke('scan_local_music', { requestId })
    if (requestId !== scanRequestId) return null
    emit('localMusicCount', result.count)
    emit('localMusicFiles', { ...result, type: params.type })
    return result
  } catch (error) {
    if (requestId !== scanRequestId || String(error).includes('stale music scan')) return null
    throw error
  }
}

const noop = () => {}

export function installWindowApi() {
  const appWindow = getCurrentWindow()
  const trayListener = listen('tray-hide', () => emit('beforeTrayHide'))
  const mediaListener = listen('media-control', (event) => emit('systemMediaControl', event.payload))
  const exitListener = listen('app-exit-requested', () => {
    runExitFlush().catch((error) => console.error('[exit flush]', error))
  })

  window.windowApi = {
    windowMin: () => appWindow.minimize(),
    windowMax: async () => (await appWindow.isMaximized()) ? appWindow.unmaximize() : appWindow.maximize(),
    windowClose: () => appWindow.close(),
    toRegister: () => invoke('open_project_page'),
    beforeQuit: (callback) => subscribe('beforeQuit', callback),
    beforeTrayHide: (callback) => subscribe('beforeTrayHide', callback),
    exitApp: (playlist) => runExitFlush(playlist),
    scanLocalMusic,
    localMusicFiles: (callback) => subscribe('localMusicFiles', callback),
    localMusicCount: (callback) => subscribe('localMusicCount', callback),
    getLocalMusicImage: (filePath) => invoke('read_cover', { filePath }),
    getLocalMusicLyric: (filePath) => invoke('read_lyrics', { filePath }),
    audioLoad: (filePath, autoplay, volume, requestId) => invoke('audio_load', { filePath, autoplay, volume, requestId }),
    audioPlay: () => invoke('audio_play'),
    audioPause: () => invoke('audio_pause'),
    audioSeek: (position) => invoke('audio_seek', { position }),
    audioSetVolume: (volume) => invoke('audio_set_volume', { volume }),
    audioStatus: () => invoke('audio_status'),
    audioStop: () => invoke('audio_stop'),
    setSystemMediaMetadata: (metadata) => invoke('media_set_metadata', metadata),
    setSystemMediaVolume: (volume) => invoke('media_set_volume', { volume }),
    systemMediaControl: (callback) => subscribe('systemMediaControl', callback),
    setSettings: (settings) => invoke('set_settings', { settings }),
    getSettings: () => invoke('get_settings'),
    listSystemFonts: () => invoke('list_system_fonts'),
    openFile: () => invoke('select_local_folder'),
    openLocalFolder: (filePath) => invoke('reveal_music_file', { filePath }),
    saveLastPlaylist: (playlist) => invoke('save_last_playlist', { playlist }),
    getLastPlaylist: () => invoke('get_last_playlist'),
    reportFrontendError: (source, detail) => invoke('report_frontend_error', { source, detail }),
    copyTxt: (txt) => navigator.clipboard.writeText(txt),
    playOrPauseMusic: (callback) => subscribe('playOrPauseMusic', callback),
    lastOrNextMusic: (callback) => subscribe('lastOrNextMusic', callback),
    changeMusicPlaymode: (callback) => subscribe('changeMusicPlaymode', callback),
    volumeUp: (callback) => subscribe('volumeUp', callback),
    volumeDown: (callback) => subscribe('volumeDown', callback),
    musicProcessControl: (callback) => subscribe('musicProcessControl', callback),
    hidePlayer: (callback) => subscribe('hidePlayer', callback),
    lyricControl: (callback) => subscribe('lyricControl', callback),
    playOrPauseMusicCheck: (playing) => invoke('media_set_playback', { playing }),
    changeTrayMusicPlaymode: noop,
    registerShortcuts: noop,
    unregisterShortcuts: noop,
  }

  return async () => {
    callbacks.clear()
    const unlisteners = await Promise.all([trayListener, mediaListener, exitListener])
    unlisteners.forEach((unlisten) => unlisten())
  }
}
