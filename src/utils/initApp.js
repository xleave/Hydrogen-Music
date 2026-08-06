import pinia from '../store/pinia'
import { loadLastSong } from './player'
import { scanMusic } from './locaMusic'
import { usePlayerStore } from '../store/playerStore'
import { useLocalStore } from '../store/localStore'
import { storeToRefs } from 'pinia'
import { insertCustomFontStyle } from './setFont'

const playerStore = usePlayerStore(pinia)
const localStore = useLocalStore(pinia)
const { lyricSize, tlyricSize, rlyricSize, lyricInterludeTime } = storeToRefs(playerStore)

export function initSettings() {
  windowApi.getSettings().then((settings) => {
    lyricSize.value = settings.music.lyricSize
    tlyricSize.value = settings.music.tlyricSize
    rlyricSize.value = settings.music.rlyricSize
    lyricInterludeTime.value = settings.music.lyricInterlude
    localStore.localFolderSettings = settings.local.localFolder
    localStore.quitApp = settings.other.quitApp
    if (localStore.localFolderSettings.length && !localStore.localMusicFolder) {
      scanMusic({ type: 'local', refresh: false })
    }
    if (!localStore.localFolderSettings.length) {
      localStore.localMusicFolder = null
      localStore.localMusicList = null
      localStore.localMusicClassify = null
    }
    insertCustomFontStyle(settings.other.customFont)
  })
}

export function init() {
  initSettings()
  loadLastSong()
}
