function finite(value, fallback, minimum, maximum) {
  const number = Number(value)
  return Number.isFinite(number) ? Math.max(minimum, Math.min(maximum, number)) : fallback
}

function index(value) {
  return Number.isInteger(value) ? Math.max(0, value) : 0
}

export function buildPlaylistCheckpoint(state, includeStructure) {
  const structureRevision = index(state.structureRevision)
  const scalars = {
    version: 4,
    structureRevision,
    currentSongId: state.currentSongId ?? null,
    currentIndex: index(state.currentIndex),
    shuffleIndex: index(state.shuffleIndex),
    progress: finite(state.progress, 0, 0, Number.MAX_SAFE_INTEGER),
    volume: finite(state.volume, 0.3, 0, 1),
    playMode: Math.min(3, index(state.playMode)),
  }
  const signature = JSON.stringify(scalars)
  if (!includeStructure) return { payload: signature, signature, structureRevision: null }

  const songIds = state.songIdsJson ?? JSON.stringify(state.songIds || [])
  const shuffledSongIds = state.shuffledSongIdsJson ?? JSON.stringify(state.shuffledSongIds || [])
  return {
    payload: `${signature.slice(0, -1)},"songIds":${songIds},"shuffledSongIds":${shuffledSongIds}}`,
    signature,
    structureRevision,
  }
}
