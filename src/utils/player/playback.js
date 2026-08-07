import { Howl, Howler } from 'howler'
import { convertFileSrc } from '@tauri-apps/api/core'
import { playerRefs } from './state'
import {
  loadLocalLyrics,
  prepareLyricsForTrackChange,
  resetLyricAnimation,
  revealLyrics,
} from './lyrics'

const {
  coverUrl,
  currentIndex,
  currentMusic,
  localBase64Img,
  playMode,
  playing,
  progress,
  songId,
  songList,
  time,
  volume,
} = playerRefs

let progressFrame = null
let lastProgressUpdate = 0
let sequentialPlaybackEnded = false
let nextTrackHandler = null

export function registerNextTrackHandler(handler) {
  nextTrackHandler = handler
}

function currentTrack() {
  return songList.value?.[currentIndex.value] ?? null
}

export function stopProgress() {
  if (progressFrame !== null) cancelAnimationFrame(progressFrame)
  progressFrame = null
  lastProgressUpdate = 0
}

function updateProgress(timestamp = 0) {
  if (timestamp - lastProgressUpdate >= 100 || lastProgressUpdate === 0) {
    const seek = currentMusic.value?.seek()
    if (typeof seek === 'number') progress.value = Math.min(seek, time.value)
    lastProgressUpdate = timestamp
  }
  if (playing.value) progressFrame = requestAnimationFrame(updateProgress)
}

export function startProgress() {
  stopProgress()
  updateProgress()
}

function handleTrackEnd() {
  stopProgress()
  if (playMode.value === 0 && currentIndex.value >= songList.value.length - 1) {
    playing.value = false
    sequentialPlaybackEnded = true
    windowApi.playOrPauseMusicCheck(false)
    return
  }
  if (playMode.value === 2) {
    resetLyricAnimation()
    return
  }
  nextTrackHandler?.()
}

export function play(url, autoplay) {
  if (currentMusic.value) {
    currentMusic.value.unload()
    Howler.unload()
  }

  currentMusic.value = new Howl({
    src: [url],
    autoplay,
    html5: true,
    preload: true,
    format: ['mp3', 'flac', 'wav', 'aac', 'm4a', 'ogg', 'opus'],
    loop: playMode.value === 2,
    volume: volume.value,
    onend: handleTrackEnd,
  })

  currentMusic.value.once('load', () => {
    time.value = Math.floor(currentMusic.value.duration())
  })
  currentMusic.value.on('play', () => {
    startProgress()
    playing.value = true
    windowApi.playOrPauseMusicCheck(true)
  })
  currentMusic.value.on('pause', () => {
    stopProgress()
    playing.value = false
    windowApi.playOrPauseMusicCheck(false)
  })
  currentMusic.value.on('stop', stopProgress)
  currentMusic.value.on('unload', stopProgress)
}

export function updateMediaSession() {
  const track = currentTrack()
  if (!track || !('mediaSession' in navigator) || !('MediaMetadata' in window)) return

  coverUrl.value = localBase64Img.value || null
  const metadata = {
    title: track.name || track.localName || '',
    artist: (track.ar || []).map((artist) => artist.name).join(', '),
  }
  if (coverUrl.value) metadata.artwork = [{ src: coverUrl.value }]
  navigator.mediaSession.metadata = new MediaMetadata(metadata)
}

export async function getSongUrl(index, autoplay) {
  const track = songList.value?.[index]
  if (!track) return

  windowApi.setWindowTile(`${track.name} - ${track.ar?.[0]?.name || '其他'}`)
  const [cover] = await Promise.all([
    windowApi.getLocalMusicImage(track.url),
    loadLocalLyrics(track.url),
  ])
  localBase64Img.value = cover
  updateMediaSession()
  play(convertFileSrc(track.url), autoplay)
  revealLyrics()
}

export function addSong(id, index, autoplay = false) {
  const list = songList.value || []
  const targetIndex = Number.isInteger(index)
    ? index
    : list.findIndex((track) => track.id === id)
  if (!list[targetIndex]) return

  progress.value = 0
  prepareLyricsForTrackChange()
  currentIndex.value = targetIndex
  songId.value = list[targetIndex].id

  const startTrack = () => getSongUrl(targetIndex, autoplay)
  if (!currentMusic.value || volume.value === 0) {
    startTrack()
    return
  }

  const state = currentMusic.value.state()
  if (state === 'loading' || state === 'unloaded') {
    currentMusic.value.unload()
    startTrack()
    return
  }

  currentMusic.value.fade(volume.value, 0, 200)
  currentMusic.value.once('fade', startTrack)
}

export function startMusic() {
  if (!currentMusic.value) return
  if (
    playMode.value === 0
    && currentIndex.value === songList.value.length - 1
    && sequentialPlaybackEnded
    && currentMusic.value.seek() === 0
  ) {
    sequentialPlaybackEnded = false
    nextTrackHandler?.()
    return
  }
  if (!playing.value) currentMusic.value.play()
  resetLyricAnimation(700)
}

export function pauseMusic() {
  stopProgress()
  if (!playing.value || !currentMusic.value) return
  currentMusic.value.fade(volume.value, 0, 200)
  currentMusic.value.once('fade', () => currentMusic.value?.pause())
}

export function changeProgress(toTime) {
  if (!currentMusic.value) return
  resetLyricAnimation()
  currentMusic.value.seek(toTime)
  progress.value = toTime
}

export function changeProgressByDragStart() {
  stopProgress()
}

export function changeProgressByDragEnd(toTime) {
  changeProgress(toTime)
  if (playing.value) startProgress()
}

export function disposePlayback() {
  stopProgress()
  currentMusic.value?.unload()
  nextTrackHandler = null
}
