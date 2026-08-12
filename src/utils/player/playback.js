import { markRaw } from 'vue'
import { convertFileSrc } from '@tauri-apps/api/core'
import { playerRefs } from './state'
import { loadLocalLyrics, prepareLyricsForTrackChange, resetLyricAnimation, revealLyrics } from './lyrics'
import { PlaybackClock } from './playbackClock.mjs'
import { trackEndAction } from './playbackOrder.mjs'

const {
  currentIndex,
  currentMusic,
  localCoverUrl,
  playMode,
  playing,
  progress,
  songId,
  songList,
  time,
  volume,
} = playerRefs
const PROGRESS_POLL_INTERVAL_MS = 200
const NATIVE_STATUS_INTERVAL_MS = 1_000
const BACKGROUND_PROGRESS_RENDER_INTERVAL_MS = 1_000
let progressTimer = null
let statusPending = false
let lastProgressRenderAt = 0
let sequentialPlaybackEnded = false
let nextTrackHandler = null
let playbackCheckpointHandler = null
let trackRequestId = 0
let playPending = false

function reportAudioError(source, error) {
  const detail = error instanceof Error ? error.message : String(error)
  console.error(`[${source}]`, error)
  windowApi.reportFrontendError(source, detail).catch((reportError) => console.error('[audio error reporter]', reportError))
}

function syncSystemPlayback(status) {
  return windowApi.setSystemMediaPlayback(status.playing, status.position)
    .catch((error) => reportAudioError('media.playback', error))
}

function checkpointPlayback() {
  try {
    const result = playbackCheckpointHandler?.()
    result?.catch?.((error) => console.error('[playback checkpoint]', error))
  } catch (error) {
    console.error('[playback checkpoint]', error)
  }
}

function invalidateTrackRequest() {
  trackRequestId += 1
  playPending = false
  return trackRequestId
}

class NativeMusic {
  constructor(status) {
    this.clock = new PlaybackClock(status, performance.now())
    this.loaded = true
    this.looping = false
    this.endHandled = false
  }
  applyStatus(status) {
    this.clock.apply(status, performance.now())
    return status
  }
  play() { return windowApi.audioPlay().then((status) => this.applyStatus(status)) }
  pause() { return windowApi.audioPause().then((status) => this.applyStatus(status)) }
  seek(position) {
    if (position === undefined) return this.estimatedPosition()
    const target = Math.max(0, Math.min(Number(position) || 0, this.duration()))
    this.endHandled = false
    windowApi.audioSeek(target).then((status) => {
      this.applyStatus(status)
      if (this === currentMusic.value) progress.value = status.position
      syncSystemPlayback(status)
      checkpointPlayback()
    }).catch((error) => {
      reportAudioError('audio.seek', error)
      this.sync().then((status) => {
        if (this === currentMusic.value) progress.value = status.position
      }).catch(() => {})
    })
    return target
  }
  duration() { return this.clock.duration }
  estimatedPosition(now = performance.now()) { return this.clock.estimate(now) }
  shouldReconcile(now = performance.now()) {
    return this.clock.shouldReconcile(now, NATIVE_STATUS_INTERVAL_MS)
      || this.clock.needsEndConfirmation(now, PROGRESS_POLL_INTERVAL_MS / 1_000)
  }
  volume(value) {
    if (value === undefined) return volume.value
    windowApi.audioSetVolume(value).catch((error) => reportAudioError('audio.volume', error))
    windowApi.setSystemMediaVolume(value).catch((error) => reportAudioError('media.volume', error))
    return value
  }
  loop(value) { if (value === undefined) return this.looping; this.looping = value; return value }
  state() { return this.loaded ? 'loaded' : 'unloaded' }
  once(event, handler) { if (event === 'load' && this.loaded) queueMicrotask(handler) }
  sync() { return windowApi.audioStatus().then((status) => this.applyStatus(status)) }
  unload() {
    this.loaded = false
    if (currentMusic.value !== this) return Promise.resolve()
    invalidateTrackRequest()
    return windowApi.audioStop().catch((error) => reportAudioError('audio.stop', error))
  }
}

export function registerNextTrackHandler(handler) { nextTrackHandler = handler }
export function registerPlaybackCheckpointHandler(handler) { playbackCheckpointHandler = handler }
function currentTrack() { return songList.value?.[currentIndex.value] ?? null }

function shouldRenderProgress() {
  const now = performance.now()
  if (document.hasFocus() || now - lastProgressRenderAt >= BACKGROUND_PROGRESS_RENDER_INTERVAL_MS) {
    lastProgressRenderAt = now
    return true
  }
  return false
}

export function stopProgress() {
  if (progressTimer !== null) clearInterval(progressTimer)
  progressTimer = null
  lastProgressRenderAt = 0
}

function updateProgress() {
  if (!playing.value) return
  const music = currentMusic.value
  if (!music) return
  const now = performance.now()
  const estimatedPosition = music.estimatedPosition(now)
  if (shouldRenderProgress()) progress.value = estimatedPosition
  time.value = Math.floor(music.duration())
  if (statusPending || !music.shouldReconcile(now)) return
  statusPending = true
  music.sync().then((status) => {
    if (music !== currentMusic.value) return
    const position = Math.min(status.position, status.duration)
    if (status.ended || shouldRenderProgress()) progress.value = position
    time.value = Math.floor(status.duration)
    if (status.ended && !music.endHandled) {
      music.endHandled = true
      handleTrackEnd()
    }
  }).catch((error) => {
    stopProgress()
    playing.value = false
    reportAudioError('audio.status', error)
  }).finally(() => {
    statusPending = false
  })
}

export function startProgress() {
  stopProgress()
  updateProgress()
  progressTimer = setInterval(updateProgress, PROGRESS_POLL_INTERVAL_MS)
}

function handleTrackEnd() {
  stopProgress()
  const action = trackEndAction(playMode.value, currentIndex.value, songList.value.length)
  if (action === 'stop') {
    playing.value = false
    sequentialPlaybackEnded = true
    syncSystemPlayback({ playing: false, position: progress.value })
    checkpointPlayback()
    return
  }
  if (action === 'repeat') {
    progress.value = 0
    resetLyricAnimation()
    getSongUrl(currentIndex.value, true)
    return
  }
  nextTrackHandler?.()
}

export async function play(filePath, autoplay, requestId = trackRequestId) {
  const status = await windowApi.audioLoad(filePath, autoplay, volume.value, requestId)
  if (requestId !== trackRequestId) return null
  const music = markRaw(new NativeMusic(status))
  music.loop(playMode.value === 2)
  currentMusic.value = music
  time.value = Math.floor(status.duration)
  updateMediaSession(requestId).catch((error) => reportAudioError('media.metadata', error))
  playing.value = status.playing
  playPending = false
  syncSystemPlayback(status)
  if (status.playing) startProgress()
  return music
}

export async function updateMediaSession(requestId = trackRequestId) {
  const track = currentTrack()
  if (!track) return false
  const metadata = {
    title: track.name || track.localName || '',
    artist: (track.ar || []).map((artist) => artist.name).join(', '),
    album: track.album || '',
  }
  if ('mediaSession' in navigator && 'MediaMetadata' in window) {
    navigator.mediaSession.metadata = new MediaMetadata(metadata)
  }

  const result = await windowApi.setSystemMediaMetadata({
    ...metadata,
    duration: time.value,
    filePath: track.url,
  })
  if (requestId !== trackRequestId || !result?.applied) return false

  const cover = result.coverPath ? convertFileSrc(result.coverPath) : null
  localCoverUrl.value = cover
  if ('mediaSession' in navigator && 'MediaMetadata' in window) {
    if (cover) metadata.artwork = [{ src: cover }]
    navigator.mediaSession.metadata = new MediaMetadata(metadata)
  }
  return true
}

export async function getSongUrl(index, autoplay) {
  const track = songList.value?.[index]
  if (!track) return null
  const requestId = ++trackRequestId

  loadLocalLyrics(track.url, requestId, () => trackRequestId)
    .then((applied) => { if (applied) revealLyrics() })
    .catch((error) => reportAudioError('lyrics.load', error))

  try {
    return await play(track.url, autoplay, requestId)
  } catch (error) {
    if (requestId === trackRequestId) {
      playPending = false
      playing.value = currentMusic.value
        ? await currentMusic.value.sync().then((status) => status.playing).catch(() => false)
        : false
      reportAudioError('audio.load', error)
    }
    return null
  }
}

export function addSong(id, index, autoplay = false) {
  const list = songList.value || []
  const targetIndex = Number.isInteger(index) ? index : list.findIndex((track) => track.id === id)
  if (!list[targetIndex]) return
  stopProgress()
  playPending = autoplay
  progress.value = 0
  prepareLyricsForTrackChange()
  currentIndex.value = targetIndex
  songId.value = list[targetIndex].id
  getSongUrl(targetIndex, autoplay)
}

export function startMusic() {
  if (!currentMusic.value) return
  if (playMode.value === 0 && currentIndex.value === songList.value.length - 1 && sequentialPlaybackEnded) {
    sequentialPlaybackEnded = false
    nextTrackHandler?.()
    return
  }
  if (!playing.value && !playPending) {
    playPending = true
    const music = currentMusic.value
    music.play().then((status) => {
      if (music !== currentMusic.value) return
      playPending = false
      playing.value = status.playing
      syncSystemPlayback(status)
      if (status.playing) startProgress()
      checkpointPlayback()
    }).catch((error) => {
      playPending = false
      if (String(error).includes('audio output device faulted')) {
        getSongUrl(currentIndex.value, true)
        return
      }
      reportAudioError('audio.play', error)
    })
  }
  resetLyricAnimation(700)
}

export function pauseMusic() {
  stopProgress()
  const hadPendingLoad = playPending
  if (!playing.value && !hadPendingLoad) return

  invalidateTrackRequest()
  playing.value = false

  if (currentMusic.value) {
    currentMusic.value.pause()
      .then((status) => {
        syncSystemPlayback(status)
        checkpointPlayback()
      })
      .catch((error) => {
        if (!hadPendingLoad || !String(error).includes('no audio is loaded')) reportAudioError('audio.pause', error)
      })
  } else {
    windowApi.audioStop()
      .then(() => windowApi.setSystemMediaStopped())
      .catch((error) => reportAudioError('audio.stop', error))
  }
}

export async function stopMusic() {
  stopProgress()
  const hadPendingLoad = playPending
  invalidateTrackRequest()
  playing.value = false
  progress.value = 0

  try {
    if (currentMusic.value) {
      try {
        const paused = await windowApi.audioPause()
        currentMusic.value.applyStatus(paused)
      } catch (error) {
        if (!hadPendingLoad || !String(error).includes('no audio is loaded')) throw error
      }

      try {
        const reset = await windowApi.audioSeek(0)
        currentMusic.value.applyStatus(reset)
        currentMusic.value.endHandled = false
        progress.value = reset.position
      } catch (error) {
        if (!String(error).includes('no audio is loaded')) throw error
      }
    } else {
      await windowApi.audioStop()
    }
  } catch (error) {
    reportAudioError('audio.stop-state', error)
  } finally {
    try {
      await windowApi.setSystemMediaStopped()
    } catch (error) {
      reportAudioError('media.stop', error)
    }
    checkpointPlayback()
  }
}

export function changeProgress(toTime) {
  if (!currentMusic.value) return
  resetLyricAnimation()
  progress.value = currentMusic.value.seek(toTime)
}
export function changeProgressByDragStart() { stopProgress() }
export function changeProgressByDragEnd(toTime) { changeProgress(toTime); if (playing.value) startProgress() }
export function disposePlayback() {
  stopProgress()
  invalidateTrackRequest()
  currentMusic.value?.unload()
  nextTrackHandler = null
  playbackCheckpointHandler = null
}
