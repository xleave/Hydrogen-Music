export function wrappedIndex(index, length) {
  if (!length) return -1
  return (index + length) % length
}

export function shuffledTracks(tracks, currentId, playAll = false, random = Math.random) {
  const shuffled = [...(tracks || [])]
  for (let index = shuffled.length - 1; index > 0; index -= 1) {
    const target = Math.floor(random() * (index + 1))
    ;[shuffled[index], shuffled[target]] = [shuffled[target], shuffled[index]]
  }
  if (!playAll && currentId) {
    const current = shuffled.findIndex((track) => track.id === currentId)
    if (current >= 0) shuffled.unshift(...shuffled.splice(current, 1))
  }
  return shuffled
}

export function queueSelection(playMode, index, songs, shuffled) {
  const active = playMode === 3 ? shuffled : songs
  const normalizedIndex = wrappedIndex(index, active?.length || 0)
  if (normalizedIndex < 0) return null
  const track = active[normalizedIndex]
  return {
    track,
    currentIndex: playMode === 3
      ? (songs || []).findIndex((song) => song.id === track.id)
      : normalizedIndex,
    shuffleIndex: playMode === 3 ? normalizedIndex : null,
  }
}

export function trackEndAction(playMode, currentIndex, trackCount) {
  if (playMode === 0 && currentIndex >= trackCount - 1) return 'stop'
  if (playMode === 2) return 'repeat'
  return 'next'
}
