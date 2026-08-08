import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { listen } from '@tauri-apps/api/event'

const callbacks = new Map()
let scanRequestId = 0
let exiting = false
let localShortcuts = []
let localShortcutListenerInstalled = false

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

function dispatchShortcut(action) {
  switch (action) {
    case 'play': emit('playOrPauseMusic'); break
    case 'last': emit('lastOrNextMusic', 'last'); break
    case 'next': emit('lastOrNextMusic', 'next'); break
    case 'volumeUp': emit('volumeUp'); break
    case 'volumeDown': emit('volumeDown'); break
    case 'processForward': emit('musicProcessControl', 'forward'); break
    case 'processBack': emit('musicProcessControl', 'back'); break
  }
}

function shouldIgnoreLocalShortcut(event) {
  const target = event.target
  if (!(target instanceof Element)) return false
  if (target.closest('input, textarea, select, [contenteditable="true"]')) return true
  return false
}

function normalizeShortcutKey(event) {
  if (event.key === ' ') return 'Space'
  if (event.key?.startsWith('Arrow')) return event.key.slice(5)
  if (/^F\d{1,2}$/i.test(event.key || '')) return event.key.toUpperCase()
  if (/^Numpad\d$/.test(event.code || '')) return `num${event.code.slice(-1)}`
  if (/^Key[A-Z]$/.test(event.code || '')) return event.code.slice(3)
  if (/^Digit\d$/.test(event.code || '')) return event.code.slice(5)
  return event.key?.length === 1 ? event.key : event.key
}

function shortcutMatches(event, shortcut) {
  const tokens = String(shortcut || '').split('+').map((token) => token.trim()).filter(Boolean)
  if (!tokens.length) return false

  let needsCtrl = false
  let needsMetaOrCtrl = false
  let needsShift = false
  let needsAlt = false
  let expectedKey = null

  for (const token of tokens) {
    if (token === 'CommandOrControl') needsMetaOrCtrl = true
    else if (token === 'Control' || token === 'Ctrl') needsCtrl = true
    else if (token === 'Shift') needsShift = true
    else if (token === 'Alt' || token === 'Option') needsAlt = true
    else expectedKey = token
  }

  if (needsMetaOrCtrl ? !(event.ctrlKey || event.metaKey) : (event.ctrlKey || event.metaKey) && !needsCtrl) return false
  if (needsCtrl && !event.ctrlKey) return false
  if (needsShift !== event.shiftKey) return false
  if (needsAlt !== event.altKey) return false
  if (!expectedKey) return false

  const actualKey = normalizeShortcutKey(event)
  return String(actualKey).toLocaleLowerCase() === String(expectedKey).toLocaleLowerCase()
}

function handleLocalShortcut(event) {
  if (event.repeat || shouldIgnoreLocalShortcut(event)) return
  for (const binding of localShortcuts) {
    if (!shortcutMatches(event, binding.shortcut)) continue
    event.preventDefault()
    event.stopPropagation()
    dispatchShortcut(binding.id)
    return
  }
}

function setLocalShortcuts(shortcuts) {
  localShortcuts = (shortcuts || [])
    .filter((item) => item?.id && item?.shortcut)
    .map((item) => ({ id: item.id, shortcut: item.shortcut }))
  if (!localShortcutListenerInstalled) {
    window.addEventListener('keydown', handleLocalShortcut, true)
    localShortcutListenerInstalled = true
  }
}

function clearLocalShortcuts() {
  localShortcuts = []
  if (localShortcutListenerInstalled) {
    window.removeEventListener('keydown', handleLocalShortcut, true)
    localShortcutListenerInstalled = false
  }
}

async function registerShortcuts(shortcuts = [], globalEnabled = false) {
  setLocalShortcuts(shortcuts)
  const globalBindings = globalEnabled
    ? shortcuts
      .filter((item) => item?.id && item?.globalShortcut)
      .map((item) => ({ id: item.id, shortcut: item.globalShortcut }))
    : []
  return invoke('register_shortcuts', { shortcuts: globalBindings })
}

async function unregisterShortcuts() {
  clearLocalShortcuts()
  return invoke('unregister_shortcuts')
}

async function runExitFlush(playlist) {
  if (exiting) return
  exiting = true
  try {
    if (playlist !== undefined) await invoke('save_last_playlist', { playlist })
    const pending = emit('beforeQuit')
    await Promise.allSettled(pending.map((value) => Promise.resolve(value)))
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
  const shortcutListener = listen('shortcut-action', (event) => dispatchShortcut(event.payload))
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
    audioLoad: (filePath, autoplay, volume) => invoke('audio_load', { filePath, autoplay, volume }),
    audioPlay: () => invoke('audio_play'),
    audioPause: () => invoke('audio_pause'),
    audioSeek: (position) => invoke('audio_seek', { position }),
    audioSetVolume: (volume) => invoke('audio_set_volume', { volume }),
    audioStatus: () => invoke('audio_status'),
    audioStop: () => invoke('audio_stop'),
    setSystemMediaMetadata: (metadata) => invoke('media_set_metadata', metadata),
    setSystemMediaVolume: (volume) => invoke('media_set_volume', { volume }),
    setSystemMediaStopped: () => invoke('media_set_stopped'),
    clearSystemMedia: () => invoke('media_clear'),
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
    registerShortcuts,
    unregisterShortcuts,
  }

  return async () => {
    clearLocalShortcuts()
    callbacks.clear()
    await invoke('unregister_shortcuts').catch(() => {})
    const unlisteners = await Promise.all([trayListener, mediaListener, shortcutListener, exitListener])
    unlisteners.forEach((unlisten) => unlisten())
  }
}
