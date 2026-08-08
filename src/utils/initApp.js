import { ref } from 'vue'
import pinia from '../store/pinia'
import { loadLastSong } from './player/playlist'
import { scanMusic } from './locaMusic'
import { usePlayerStore } from '../store/playerStore'
import { useLocalStore } from '../store/localStore'
import { storeToRefs } from 'pinia'
import { insertCustomFontStyle } from './setFont'

const playerStore = usePlayerStore(pinia)
const localStore = useLocalStore(pinia)
const { lyricSize, tlyricSize, rlyricSize, lyricInterludeTime } = storeToRefs(playerStore)

export const settingsSaveState = ref('saved')
let settingsEditRevision = 0
let settingsSavedRevision = 0
let pendingSettingsPayload = null
let settingsSaveTimer = null
let settingsSaveLoop = null
let settingsInitialized = false

function folderSignature(folders) {
  return [...(folders || [])]
    .map((folder) => String(folder))
    .sort()
    .join('\u0000')
}

function invalidateLocalLibrary() {
  localStore.localDirectoryTree = null
  localStore.localMusicList = null
  localStore.localMusicClassify = null
  localStore.currentSelectedInfo = null
  localStore.currentSelectedSongs = null
  localStore.currentSelectedFilePicUrl = null
  localStore.detailRequestId += 1
}

export function resetSettingsPersistence() {
  if (settingsSaveTimer) clearTimeout(settingsSaveTimer)
  settingsSaveTimer = null
  pendingSettingsPayload = null
  settingsEditRevision = 0
  settingsSavedRevision = 0
  settingsSaveState.value = 'saved'
}

async function drainSettingsSaves() {
  try {
    while (settingsSavedRevision < settingsEditRevision) {
      const revision = settingsEditRevision
      const payload = pendingSettingsPayload
      await windowApi.setSettings(payload)
      settingsSavedRevision = revision
    }
    settingsSaveState.value = 'saved'
  } catch (error) {
    settingsSaveState.value = 'failed'
    throw error
  } finally {
    settingsSaveLoop = null
  }

  if (settingsSavedRevision < settingsEditRevision) return flushSettings()
}

export function scheduleSettingsSave(settings, delay = 650) {
  pendingSettingsPayload = JSON.stringify(settings)
  settingsEditRevision += 1
  settingsSaveState.value = 'saving'
  if (settingsSaveTimer) clearTimeout(settingsSaveTimer)
  settingsSaveTimer = setTimeout(() => {
    settingsSaveTimer = null
    flushSettings().catch((error) => console.error('[settings.save]', error))
  }, delay)
}

export function flushSettings() {
  if (settingsSaveTimer) {
    clearTimeout(settingsSaveTimer)
    settingsSaveTimer = null
  }
  if (settingsSavedRevision >= settingsEditRevision) {
    settingsSaveState.value = 'saved'
    return Promise.resolve()
  }
  if (!settingsSaveLoop) settingsSaveLoop = drainSettingsSaves()
  return settingsSaveLoop
}

export async function initSettings() {
  const settings = await windowApi.getSettings()
  if (!settings) return null

  lyricSize.value = settings.music.lyricSize
  tlyricSize.value = settings.music.tlyricSize
  rlyricSize.value = settings.music.rlyricSize
  lyricInterludeTime.value = settings.music.lyricInterlude

  const previousFolders = folderSignature(localStore.localFolderSettings)
  const nextFolders = [...(settings.local.localFolder || [])]
  const rootsChanged = settingsInitialized && previousFolders !== folderSignature(nextFolders)
  localStore.localFolderSettings = nextFolders
  localStore.quitApp = settings.other.quitApp
  settingsInitialized = true

  if (rootsChanged) invalidateLocalLibrary()
  if (nextFolders.length && (rootsChanged || !localStore.localDirectoryTree)) {
    if (!rootsChanged && !localStore.localDirectoryTree) {
      await windowApi.getCachedLibrary({ type: 'local' })
        .catch((error) => console.error('[library cache]', error))
    }
    // Always reconcile with the filesystem in the background. The SQLite
    // metadata index makes unchanged tracks cheap, while a valid snapshot lets
    // the UI and pending playback restore become usable immediately.
    scanMusic({ type: 'local', refresh: rootsChanged })
  }
  if (!nextFolders.length) invalidateLocalLibrary()

  insertCustomFontStyle(settings.other.customFont)
  windowApi.registerShortcuts(settings.shortcuts || [], Boolean(settings.other.globalShortcuts))
    .catch((error) => console.error('[shortcuts.register]', error))
  return settings
}

export function init() {
  initSettings().catch((error) => console.error('[settings.init]', error))
  loadLastSong().catch((error) => console.error('[playlist.restore]', error))
}
