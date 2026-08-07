import { playerRefs, otherStore } from './state'
import {
  changeProgressByDragEnd,
  changeProgressByDragStart,
  pauseMusic,
  startMusic,
  stopProgress,
} from './playback'
import {
  applyPlayMode,
  playLast,
  playNext,
  savePlaylist,
} from './playlist'

const { currentMusic, playMode, playing, playlistWidgetShow, progress, volume } = playerRefs
let cleanup = null

function isInside(target, selector) {
  return target instanceof Node && (document.querySelector(selector)?.contains(target) ?? false)
}

export function initializePlayerLifecycle() {
  cleanup?.()
  const disposers = []
  let draggingProgress = false

  const onMouseDown = (event) => {
    if (event.target instanceof Element && event.target.closest('#widget-progress')) {
      changeProgressByDragStart()
      draggingProgress = true
    }
  }
  const onMouseUp = () => {
    if (!draggingProgress) return
    changeProgressByDragEnd(progress.value)
    draggingProgress = false
  }
  const onClick = (event) => {
    if (playlistWidgetShow.value) {
      const selectors = [
        '.playlist-widget',
        '.music-control',
        '.music-other',
        '.playlist-widget-player',
        '.song-control',
        '.context-menu',
        '.item-delete',
      ]
      if (!selectors.some((selector) => isInside(event.target, selector))) {
        playlistWidgetShow.value = false
      }
    }
    if (otherStore.contextMenuShow && !isInside(event.target, '.context-menu')) {
      otherStore.contextMenuShow = false
    }
  }

  window.addEventListener('mousedown', onMouseDown)
  window.addEventListener('mouseup', onMouseUp)
  window.addEventListener('click', onClick)
  disposers.push(
    () => window.removeEventListener('mousedown', onMouseDown),
    () => window.removeEventListener('mouseup', onMouseUp),
    () => window.removeEventListener('click', onClick),
    windowApi.playOrPauseMusic(() => (playing.value ? pauseMusic() : startMusic())),
    windowApi.lastOrNextMusic((event, option) => (option === 'last' ? playLast() : playNext())),
    windowApi.changeMusicPlaymode((event, mode) => applyPlayMode(mode)),
    windowApi.volumeUp(() => {
      volume.value = Math.min(1, volume.value + 0.1)
      currentMusic.value?.volume(volume.value)
    }),
    windowApi.volumeDown(() => {
      volume.value = Math.max(0, volume.value - 0.1)
      currentMusic.value?.volume(volume.value)
    }),
    windowApi.musicProcessControl((event, mode) => {
      if (!currentMusic.value) return
      const delta = mode === 'forward' ? 3 : -3
      const target = Math.max(0, Math.min(currentMusic.value.duration(), progress.value + delta))
      progress.value = target
      currentMusic.value.seek(target)
    }),
    windowApi.beforeTrayHide(savePlaylist),
  )

  if ('mediaSession' in navigator) {
    navigator.mediaSession.setActionHandler('previoustrack', playLast)
    navigator.mediaSession.setActionHandler('nexttrack', playNext)
    navigator.mediaSession.setActionHandler('play', startMusic)
    navigator.mediaSession.setActionHandler('pause', pauseMusic)
  }
  windowApi.playOrPauseMusicCheck(playing.value)
  windowApi.changeTrayMusicPlaymode(playMode.value)

  cleanup = () => {
    for (const dispose of disposers) dispose?.()
    stopProgress()
    if ('mediaSession' in navigator) {
      for (const action of ['previoustrack', 'nexttrack', 'play', 'pause']) {
        navigator.mediaSession.setActionHandler(action, null)
      }
    }
    cleanup = null
  }
  return cleanup
}
