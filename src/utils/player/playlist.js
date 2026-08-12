import { noticeOpen } from '../dialog'
import { useLocalStore } from '../../store/localStore'
import pinia from '../../store/pinia'
import { playerRefs } from './state'
import { addSong, getSongUrl, registerNextTrackHandler } from './playback'
import { buildPlaylistCheckpoint } from './playlistCheckpoint.mjs'
import { queueSelection, shuffledTracks } from './playbackOrder.mjs'

const localStore = useLocalStore(pinia)
const {
  currentIndex,
  currentMusic,
  listInfo,
  playMode,
  progress,
  shuffleIndex,
  shuffledList,
  songId,
  songList,
  volume,
} = playerRefs

let pendingPlaylist = null
let playlistHydrated = false
let saveRevision = 0
let savedRevision = 0
let pendingSnapshot = null
let pendingSignature = null
let pendingStructureRevision = null
let lastSavedSignature = null
let saveLoop = null
let structureRevision = 0
let persistedStructureRevision = -1
let inFlightStructureRevision = null
let cachedStructureRevision = -1
let cachedSongCount = -1
let cachedShuffleCount = -1
let cachedSongIdsJson = '[]'
let cachedShuffledIdsJson = '[]'

function markStructureChanged() {
  structureRevision += 1
}

export function localMusicHandle(list, firstOnly = false) {
  const tracks = list.map((song) => ({
    id: song.id,
    ar: (song.common.artists?.length ? song.common.artists : ['其他']).map((name) => ({ id: 'local', name })),
    url: song.dirPath,
    name: song.common.title,
    localName: song.common.localTitle,
    album: song.common.album || '',
    type: 'local',
    sampleRate: (song.format.sampleRate || 0) / 1000,
    bitsPerSample: song.format.bitsPerSample || 0,
    bitrate: Math.round((song.format.bitrate || 0) / 1000),
  }))
  return firstOnly ? tracks[0] : tracks
}

function flattenTracks(nodes, result = []) {
  for (const node of nodes || []) {
    if (node.children) flattenTracks(node.children, result)
    else if (node.id && node.common && node.format) result.push(node)
  }
  return result
}

function supersedePendingRestore() {
  pendingPlaylist = null
  playlistHydrated = true
}

export function addToList(listType, tracks) {
  supersedePendingRestore()
  listInfo.value = { id: 'local', type: listType }
  songList.value = [...tracks]
  markStructureChanged()
  savePlaylist()
}

export function addLocalMusicTOList(listType, localTracks, playId, playIndex) {
  addToList(listType, localMusicHandle(localTracks))
  addSong(playId, playIndex, true)
  savePlaylist()
}

export function markPlaylistCleared() {
  supersedePendingRestore()
  markStructureChanged()
  savePlaylist()
}

export function setShuffledList(playAll = false) {
  shuffledList.value = shuffledTracks(songList.value, songId.value, playAll)
  shuffleIndex.value = 0
  markStructureChanged()
}

function activeList() { return playMode.value === 3 ? shuffledList.value : songList.value }
function activeIndex() { return playMode.value === 3 ? shuffleIndex.value : currentIndex.value }

function playAt(index) {
  const selection = queueSelection(playMode.value, index, songList.value, shuffledList.value)
  if (!selection) return
  const { track } = selection
  if (playMode.value === 3) {
    shuffleIndex.value = selection.shuffleIndex
    currentIndex.value = selection.currentIndex
  } else {
    currentIndex.value = selection.currentIndex
  }
  songId.value = track.id
  addSong(track.id, currentIndex.value, true)
  savePlaylist()
}

export function playLast() { playAt(activeIndex() - 1) }
export function playNext() { playAt(activeIndex() + 1) }

registerNextTrackHandler(playNext)

export function changePlayMode() {
  playMode.value = (playMode.value + 1) % 4
  currentMusic.value?.loop(playMode.value === 2)
  if (playMode.value === 3) setShuffledList()
  else {
    shuffledList.value = null
    shuffleIndex.value = 0
    markStructureChanged()
  }
  windowApi.changeTrayMusicPlaymode(playMode.value)
  savePlaylist()
}

export function applyPlayMode(mode) {
  const normalizedMode = Number.isInteger(mode) ? Math.max(0, Math.min(3, mode)) : 0
  playMode.value = normalizedMode
  currentMusic.value?.loop(normalizedMode === 2)
  if (normalizedMode === 3) setShuffledList()
  else {
    shuffledList.value = null
    shuffleIndex.value = 0
    markStructureChanged()
  }
  savePlaylist()
}

export function playAll(listType, tracks) {
  addToList(listType, tracks)
  if (!songList.value?.length) return
  if (playMode.value === 3) {
    setShuffledList(true)
    playAt(0)
  } else {
    addSong(songList.value[0].id, 0, true)
  }
  savePlaylist()
}

export function addToNext(nextSong, autoplay) {
  if (!nextSong) return
  if (!songList.value) songList.value = []
  supersedePendingRestore()
  if (nextSong.id === songId.value) return

  const existingIndex = songList.value.findIndex((song) => song.id === nextSong.id)
  if (existingIndex >= 0) {
    songList.value.splice(existingIndex, 1)
    if (existingIndex < currentIndex.value) currentIndex.value -= 1
  }
  songList.value.splice(currentIndex.value + 1, 0, nextSong)

  if (playMode.value === 3) {
    if (!shuffledList.value) shuffledList.value = []
    const shuffledIndex = shuffledList.value.findIndex((song) => song.id === nextSong.id)
    if (shuffledIndex >= 0) shuffledList.value.splice(shuffledIndex, 1)
    shuffledList.value.splice(shuffleIndex.value + 1, 0, nextSong)
  }
  markStructureChanged()
  if (autoplay) playNext()
  else noticeOpen('已添加至下一首', 2)
  if (songList.value.length === 1) addSong(nextSong.id, 0, autoplay)
  savePlaylist()
}

export function addToNextLocal(song, autoplay) { addToNext(localMusicHandle([song], true), autoplay) }

function refreshStructureCacheIfNeeded() {
  const songs = songList.value || []
  const shuffled = shuffledList.value || []
  if (
    cachedStructureRevision === structureRevision
    && cachedSongCount === songs.length
    && cachedShuffleCount === shuffled.length
  ) return

  cachedSongIdsJson = JSON.stringify(songs.map((track) => track.id))
  cachedShuffledIdsJson = JSON.stringify(shuffled.map((track) => track.id))
  cachedStructureRevision = structureRevision
  cachedSongCount = songs.length
  cachedShuffleCount = shuffled.length
}

function compactPlaylistCheckpoint() {
  refreshStructureCacheIfNeeded()
  const includeStructure = structureRevision !== persistedStructureRevision
    && structureRevision !== inFlightStructureRevision
  return buildPlaylistCheckpoint({
    structureRevision,
    songIdsJson: cachedSongIdsJson,
    shuffledSongIdsJson: cachedShuffledIdsJson,
    currentSongId: songId.value,
    currentIndex: currentIndex.value,
    shuffleIndex: shuffleIndex.value,
    progress: progress.value,
    volume: volume.value,
    playMode: playMode.value,
  }, includeStructure)
}

async function drainPlaylistSaves() {
  try {
    while (savedRevision < saveRevision) {
      const revision = saveRevision
      const payload = pendingSnapshot
      const signature = pendingSignature
      const payloadStructureRevision = pendingStructureRevision
      inFlightStructureRevision = payloadStructureRevision
      try {
        await windowApi.saveLastPlaylist(payload)
        if (payloadStructureRevision !== null) {
          persistedStructureRevision = Math.max(persistedStructureRevision, payloadStructureRevision)
        }
        lastSavedSignature = signature
        savedRevision = revision
      } finally {
        inFlightStructureRevision = null
      }
    }
  } finally {
    saveLoop = null
  }
  if (savedRevision < saveRevision) return ensureSaveLoop()
}

function ensureSaveLoop() {
  if (!saveLoop) saveLoop = drainPlaylistSaves()
  return saveLoop
}

export function savePlaylist() {
  if (!playlistHydrated) return Promise.resolve()
  const checkpoint = compactPlaylistCheckpoint()
  if (saveLoop && checkpoint.signature === pendingSignature) return saveLoop
  if (!saveLoop && checkpoint.signature === lastSavedSignature) return Promise.resolve()
  pendingSnapshot = checkpoint.payload
  pendingSignature = checkpoint.signature
  pendingStructureRevision = checkpoint.structureRevision
  saveRevision += 1
  return ensureSaveLoop()
}

export async function loadLastSong(initialPlaylist) {
  pendingPlaylist = arguments.length ? initialPlaylist : await windowApi.getLastPlaylist()
  if (pendingPlaylist) {
    if (Number.isFinite(pendingPlaylist.volume)) volume.value = Math.max(0, Math.min(1, pendingPlaylist.volume))
    if (Number.isInteger(pendingPlaylist.playMode)) playMode.value = Math.max(0, Math.min(3, pendingPlaylist.playMode))
  } else {
    songList.value = []
    playlistHydrated = true
    markStructureChanged()
  }
  restorePlaylistFromLibrary(localStore.localMusicList)
}

export function restorePlaylistFromLibrary(library) {
  if (!pendingPlaylist || !Array.isArray(library) || !library.length) return
  const saved = pendingPlaylist
  const tracks = localMusicHandle(flattenTracks(library))
  const byId = new Map()
  for (const track of tracks) {
    byId.set(track.id, track)
    // v3 snapshots used the absolute path as the track id. Keep this alias so
    // future compact ids can migrate old snapshots without losing the queue.
    if (track.url) byId.set(track.url, track)
  }
  const songIds = saved.songIds || saved.songList?.map((track) => track.id) || []
  const shuffledSongIds = saved.shuffledSongIds || saved.shuffledList?.map((track) => track.id) || []
  const restored = songIds.map((id) => byId.get(id)).filter(Boolean)
  // An empty result can mean a removable/NAS library is temporarily absent.
  // Keep the pending snapshot until at least one saved track can be resolved,
  // or until an explicit user action supersedes it.
  if (!restored.length) return

  pendingPlaylist = null
  playlistHydrated = true
  songList.value = restored
  shuffledList.value = shuffledSongIds.map((id) => byId.get(id)).filter(Boolean)
  markStructureChanged()
  const fallbackIndex = Math.max(0, Math.min(Number(saved.currentIndex) || 0, restored.length - 1))
  const selectedId = saved.currentSongId || restored[fallbackIndex].id
  const savedIndex = restored.findIndex((track) => track.id === selectedId || track.url === selectedId)
  currentIndex.value = savedIndex >= 0 ? savedIndex : fallbackIndex
  songId.value = restored[currentIndex.value].id
  shuffleIndex.value = Math.max(0, Math.min(Number(saved.shuffleIndex) || 0, Math.max(0, shuffledList.value.length - 1)))
  if (playMode.value === 3 && !shuffledList.value.length) setShuffledList()

  const requestedResume = Number.isFinite(saved.progress) ? Math.max(0, saved.progress) : 0
  getSongUrl(currentIndex.value, false).then((music) => {
    const duration = Number.isFinite(music?.duration?.()) ? Math.max(0, music.duration()) : 0
    const resumeAt = Math.min(requestedResume, duration)
    progress.value = resumeAt
    if (resumeAt > 0 && music?.state() === 'loaded') music.seek(resumeAt)
    windowApi.audioSetVolume(volume.value).catch((error) => console.error('[restore volume]', error))
    windowApi.setSystemMediaVolume(volume.value).catch((error) => console.error('[restore media volume]', error))
  })
}
