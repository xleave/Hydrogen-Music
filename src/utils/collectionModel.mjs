export const FAVORITES_ID = 'favorites'

const PLAYLIST_ID_PREFIX = 'playlist:'
const MAX_PLAYLIST_NAME_LENGTH = 80

function uniqueTrackIds(values) {
  const seen = new Set()
  const result = []
  for (const value of values || []) {
    if (typeof value !== 'string' || !value || seen.has(value)) continue
    seen.add(value)
    result.push(value)
  }
  return result
}

function loadedPlaylistName(value, fallback) {
  const name = typeof value === 'string' ? value.trim() : ''
  if (!name) return fallback
  return Array.from(name).slice(0, MAX_PLAYLIST_NAME_LENGTH).join('')
}

function playlistNumber(id) {
  if (typeof id !== 'string' || !id.startsWith(PLAYLIST_ID_PREFIX)) return null
  const number = Number(id.slice(PLAYLIST_ID_PREFIX.length))
  return Number.isSafeInteger(number) && number > 0 ? number : null
}

export function emptyCollections() {
  return {
    version: 1,
    nextPlaylistId: 1,
    favoriteTrackIds: [],
    playlists: [],
  }
}

export function normalizeCollections(value) {
  const source = value && typeof value === 'object' ? value : {}
  const usedIds = new Set()
  const playlists = []
  let highestPlaylistNumber = 0

  for (const item of Array.isArray(source.playlists) ? source.playlists : []) {
    const number = playlistNumber(item?.id)
    if (!number || usedIds.has(item.id)) continue
    usedIds.add(item.id)
    highestPlaylistNumber = Math.max(highestPlaylistNumber, number)
    playlists.push({
      id: item.id,
      name: loadedPlaylistName(item.name, `歌单 ${number}`),
      trackIds: uniqueTrackIds(item.trackIds),
    })
  }

  const requestedNextId = Number(source.nextPlaylistId)
  const nextPlaylistId = Number.isSafeInteger(requestedNextId) && requestedNextId > highestPlaylistNumber
    ? requestedNextId
    : highestPlaylistNumber + 1

  return {
    version: 1,
    nextPlaylistId,
    favoriteTrackIds: uniqueTrackIds(source.favoriteTrackIds),
    playlists,
  }
}

export function validatePlaylistName(value) {
  const name = typeof value === 'string' ? value.trim() : ''
  if (!name) throw new Error('歌单名称不能为空')
  if (Array.from(name).length > MAX_PLAYLIST_NAME_LENGTH) {
    throw new Error(`歌单名称不能超过 ${MAX_PLAYLIST_NAME_LENGTH} 个字符`)
  }
  return name
}

export function createPlaylist(collections, name) {
  const current = normalizeCollections(collections)
  const playlistName = validatePlaylistName(name)
  const usedIds = new Set(current.playlists.map((item) => item.id))
  let number = current.nextPlaylistId
  while (usedIds.has(`${PLAYLIST_ID_PREFIX}${number}`)) number += 1

  const playlist = {
    id: `${PLAYLIST_ID_PREFIX}${number}`,
    name: playlistName,
    trackIds: [],
  }
  return {
    collections: {
      ...current,
      nextPlaylistId: number + 1,
      playlists: [...current.playlists, playlist],
    },
    playlist,
  }
}

export function renamePlaylist(collections, playlistId, name) {
  const current = normalizeCollections(collections)
  const playlistName = validatePlaylistName(name)
  let found = false
  const playlists = current.playlists.map((item) => {
    if (item.id !== playlistId) return item
    found = true
    return { ...item, name: playlistName }
  })
  if (!found) throw new Error('歌单不存在')
  return { ...current, playlists }
}

export function deletePlaylist(collections, playlistId) {
  const current = normalizeCollections(collections)
  if (playlistId === FAVORITES_ID) throw new Error('收藏歌单不能删除')
  return {
    ...current,
    playlists: current.playlists.filter((item) => item.id !== playlistId),
  }
}

export function trackIdsFor(collections, playlistId) {
  const current = normalizeCollections(collections)
  if (playlistId === FAVORITES_ID) return current.favoriteTrackIds
  return current.playlists.find((item) => item.id === playlistId)?.trackIds || []
}

export function hasTrack(collections, playlistId, trackId) {
  return trackIdsFor(collections, playlistId).includes(trackId)
}

function replaceTrackIds(collections, playlistId, trackIds) {
  const current = normalizeCollections(collections)
  const nextTrackIds = uniqueTrackIds(trackIds)
  if (playlistId === FAVORITES_ID) {
    return { ...current, favoriteTrackIds: nextTrackIds }
  }

  let found = false
  const playlists = current.playlists.map((item) => {
    if (item.id !== playlistId) return item
    found = true
    return { ...item, trackIds: nextTrackIds }
  })
  if (!found) throw new Error('歌单不存在')
  return { ...current, playlists }
}

export function addTrack(collections, playlistId, trackId) {
  if (typeof trackId !== 'string' || !trackId) throw new Error('曲目标识无效')
  const current = normalizeCollections(collections)
  const trackIds = trackIdsFor(current, playlistId)
  if (trackIds.includes(trackId)) return current
  return replaceTrackIds(current, playlistId, [...trackIds, trackId])
}

export function removeTrack(collections, playlistId, trackId) {
  const current = normalizeCollections(collections)
  const trackIds = trackIdsFor(current, playlistId)
  if (!trackIds.includes(trackId)) return current
  return replaceTrackIds(current, playlistId, trackIds.filter((id) => id !== trackId))
}

export function toggleFavorite(collections, trackId) {
  return hasTrack(collections, FAVORITES_ID, trackId)
    ? removeTrack(collections, FAVORITES_ID, trackId)
    : addTrack(collections, FAVORITES_ID, trackId)
}
