import { parseLyrics } from '../lyrics/lrcParser.mjs'
import { playerRefs, playerStore } from './state'

const { isLyricDelay, lyricAnimationRevision, lyricShow, playerChangeSong, widgetState } = playerRefs
let delayTimer = null

export async function loadLocalLyrics(filePath, requestId, currentRequestId) {
  const value = await windowApi.getLocalMusicLyric(filePath)
  if (requestId !== currentRequestId()) return false
  playerStore.applyParsedLyrics(parseLyrics(value ? { lrc: { lyric: value } } : null))
  return true
}

export function resetLyricAnimation(delay = 600) {
  lyricAnimationRevision.value += 1
  isLyricDelay.value = false
  clearTimeout(delayTimer)
  delayTimer = setTimeout(() => { isLyricDelay.value = true }, delay)
}

export function prepareLyricsForTrackChange() {
  playerStore.clearLyrics()
  if (lyricShow.value) {
    lyricShow.value = false
    playerChangeSong.value = true
  }
}

export function revealLyrics() {
  if (!lyricShow.value && !widgetState.value) {
    lyricShow.value = true
    playerChangeSong.value = false
  }
}

export function disposeLyrics() {
  clearTimeout(delayTimer)
  delayTimer = null
}
