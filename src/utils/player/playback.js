import { markRaw } from 'vue'
import { playerRefs } from './state'
import { loadLocalLyrics, prepareLyricsForTrackChange, resetLyricAnimation, revealLyrics } from './lyrics'

const { coverUrl, currentIndex, currentMusic, localBase64Img, playMode, playing, progress, songId, songList, time, volume } = playerRefs
let progressFrame = null
let lastProgressUpdate = 0
let statusPending = false
let sequentialPlaybackEnded = false
let nextTrackHandler = null
let playbackCheckpointHandler = null
let audioRequestId = 0
let playPending = false

function reportAudioError(source, error) {
  const detail = error instanceof Error ? error.message : String(error)
  console.error(`[${source}]`, error)
  windowApi.reportFrontendError(source, detail).catch((reportError) => console.error('[audio error reporter]', reportError))
}

function checkpointPlayback() {
  try {
    const result = playbackCheckpointHandler?.()
    result?.catch?.((error) => console.error('[playback checkpoint]', error))
  } catch (error) {
    console.error('[playback checkpoint]', error)
  }
}

class NativeMusic {
  constructor(status) { this.position = status.position; this.trackDuration = status.duration; this.loaded = true; this.looping = false; this.endHandled = false }
  applyStatus(status) { this.position = status.position; this.trackDuration = status.duration; return status }
  play() { return windowApi.audioPlay().then((status) => this.applyStatus(status)) }
  pause() { return windowApi.audioPause().then((status) => this.applyStatus(status)) }
  seek(position) {
    if (position === undefined) return this.position
    this.position = position
    this.endHandled = false
    windowApi.audioSeek(position).then((status) => {
      this.applyStatus(status)
      windowApi.playOrPauseMusicCheck(status.playing)
      checkpointPlayback()
    }).catch((error) => reportAudioError('audio.seek', error))
    return position
  }
  duration() { return this.trackDuration }
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
  unload() { this.loaded = false; if (currentMusic.value !== this) return Promise.resolve(); return windowApi.audioStop().catch((error) => reportAudioError('audio.stop', error)) }
}

export function registerNextTrackHandler(handler) { nextTrackHandler = handler }
export function registerPlaybackCheckpointHandler(handler) { playbackCheckpointHandler = handler }
function currentTrack() { return songList.value?.[currentIndex.value] ?? null }

export function stopProgress() { if (progressFrame !== null) cancelAnimationFrame(progressFrame); progressFrame = null; lastProgressUpdate = 0 }

function updateProgress(timestamp = 0) {
  if (playing.value) progressFrame = requestAnimationFrame(updateProgress)
  if (statusPending || (lastProgressUpdate !== 0 && timestamp - lastProgressUpdate < 200)) return
  const music = currentMusic.value
  if (!music) return
  lastProgressUpdate = timestamp
  statusPending = true
  music.sync().then((status) => {
    if (music !== currentMusic.value) return
    progress.value = Math.min(status.position, status.duration)
    time.value = Math.floor(status.duration)
    if (status.ended && !music.endHandled) { music.endHandled = true; handleTrackEnd() }
  }).catch((error) => { stopProgress(); playing.value = false; reportAudioError('audio.status', error) }).finally(() => { statusPending = false })
}

export function startProgress() { stopProgress(); updateProgress() }

function handleTrackEnd() {
  stopProgress()
  if (playMode.value === 0 && currentIndex.value >= songList.value.length - 1) {
    playing.value = false
    sequentialPlaybackEnded = true
    windowApi.playOrPauseMusicCheck(false)
    checkpointPlayback()
    return
  }
  if (playMode.value === 2) { progress.value = 0; resetLyricAnimation(); getSongUrl(currentIndex.value, true); return }
  nextTrackHandler?.()
}

export async function play(filePath, autoplay, requestId = audioRequestId) {
  const status = await windowApi.audioLoad(filePath, autoplay, volume.value, requestId)
  if (requestId !== audioRequestId) return null
  const music = markRaw(new NativeMusic(status))
  music.loop(playMode.value === 2)
  currentMusic.value = music
  time.value = Math.floor(status.duration)
  updateMediaSession()
  playing.value = status.playing
  playPending = false
  windowApi.playOrPauseMusicCheck(status.playing)
  if (status.playing) startProgress()
  return music
}

export function updateMediaSession() {
  const track = currentTrack()
  if (!track) return
  coverUrl.value = localBase64Img.value || null
  const metadata = { title: track.name || track.localName || '', artist: (track.ar || []).map((artist) => artist.name).join(', '), album: track.album || '' }
  windowApi.setSystemMediaMetadata({ ...metadata, duration: time.value }).catch((error) => reportAudioError('media.metadata', error))
  if (!('mediaSession' in navigator) || !('MediaMetadata' in window)) return
  if (coverUrl.value) metadata.artwork = [{ src: coverUrl.value }]
  navigator.mediaSession.metadata = new MediaMetadata(metadata)
}

export async function getSongUrl(index, autoplay) {
  const track = songList.value?.[index]
  if (!track) return null
  const requestId = ++audioRequestId

  windowApi.getLocalMusicImage(track.url).then((cover) => {
    if (requestId !== audioRequestId) return
    localBase64Img.value = cover
    updateMediaSession()
  }).catch((error) => reportAudioError('cover.load', error))

  loadLocalLyrics(track.url, requestId, () => audioRequestId)
    .then((applied) => { if (applied) revealLyrics() })
    .catch((error) => reportAudioError('lyrics.load', error))

  try {
    return await play(track.url, autoplay, requestId)
  } catch (error) {
    if (requestId === audioRequestId) {
      playPending = false
      playing.value = currentMusic.value ? await currentMusic.value.sync().then((status) => status.playing).catch(() => false) : false
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
  if (playMode.value === 0 && currentIndex.value === songList.value.length - 1 && sequentialPlaybackEnded) { sequentialPlaybackEnded = false; nextTrackHandler?.(); return }
  if (!playing.value && !playPending) {
    playPending = true
    const music = currentMusic.value
    music.play().then((status) => {
      if (music !== currentMusic.value) return
      playPending = false
      playing.value = status.playing
      windowApi.playOrPauseMusicCheck(status.playing)
      if (status.playing) startProgress()
      checkpointPlayback()
    }).catch((error) => { playPending = false; reportAudioError('audio.play', error) })
  }
  resetLyricAnimation(700)
}

export function pauseMusic() {
  stopProgress()
  if (!playing.value || !currentMusic.value) return
  const music = currentMusic.value
  playing.value = false
  windowApi.playOrPauseMusicCheck(false)
  music.pause().then(() => checkpointPlayback()).catch((error) => reportAudioError('audio.pause', error))
}

export function changeProgress(toTime) { if (!currentMusic.value) return; resetLyricAnimation(); currentMusic.value.seek(toTime); progress.value = toTime }
export function changeProgressByDragStart() { stopProgress() }
export function changeProgressByDragEnd(toTime) { changeProgress(toTime); if (playing.value) startProgress() }
export function disposePlayback() { stopProgress(); currentMusic.value?.unload(); playPending = false; nextTrackHandler = null; playbackCheckpointHandler = null }
