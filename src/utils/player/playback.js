import { Howl, Howler } from 'howler'
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
let audioRequestId = 0
let currentAudioUrl = null

const audioTypes = {
  mp3: 'audio/mpeg',
  flac: 'audio/flac',
  wav: 'audio/wav',
  aac: 'audio/aac',
  m4a: 'audio/mp4',
  ogg: 'audio/ogg',
  opus: 'audio/ogg',
}

function audioFormat(filePath) {
  return filePath.split('.').pop()?.toLowerCase() || 'mp3'
}

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

export function play(url, autoplay, format) {
  if (currentMusic.value) {
    currentMusic.value.unload()
    Howler.unload()
  }
  if (currentAudioUrl) URL.revokeObjectURL(currentAudioUrl)
  currentAudioUrl = url

  currentMusic.value = new Howl({
    src: [url],
    format: [format],
    autoplay,
    html5: true,
    preload: true,
    loop: playMode.value === 2,
    volume: volume.value,
    onend: handleTrackEnd,
    onloaderror: (_, error) => {
      console.error('[audio load]', error)
      windowApi.reportFrontendError('audio.load', String(error)).catch((reportError) => {
        console.error('[audio error reporter]', reportError)
      })
    },
    onplayerror: (_, error) => {
      console.error('[audio play]', error)
      windowApi.reportFrontendError('audio.play', String(error)).catch((reportError) => {
        console.error('[audio error reporter]', reportError)
      })
      currentMusic.value?.once('unlock', () => currentMusic.value?.play())
    },
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
  const requestId = ++audioRequestId

  windowApi.getLocalMusicImage(track.url).then((cover) => {
    if (requestId !== audioRequestId) return
    localBase64Img.value = cover
    updateMediaSession()
  })
  loadLocalLyrics(track.url).then(() => {
    if (requestId === audioRequestId) revealLyrics()
  })

  const audioData = await windowApi.getLocalMusicAudio(track.url)
  if (requestId !== audioRequestId) return
  const format = audioFormat(track.url)
  const blob = new Blob([audioData], { type: audioTypes[format] || 'application/octet-stream' })
  play(URL.createObjectURL(blob), autoplay, format)
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
  if (currentAudioUrl) URL.revokeObjectURL(currentAudioUrl)
  currentAudioUrl = null
  nextTrackHandler = null
}
