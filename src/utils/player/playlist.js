import { noticeOpen } from '../dialog'
import { useLocalStore } from '../../store/localStore'
import pinia from '../../store/pinia'
import { playerRefs } from './state'
import { addSong, getSongUrl, registerNextTrackHandler } from './playback'

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
} = playerRefs

let pendingPlaylist = null

export function localMusicHandle(list, firstOnly = false) {
  const tracks = list.map((song) => ({
    id: song.id,
    ar: (song.common.artists?.length ? song.common.artists : ['其他']).map((name) => ({
      id: 'local',
      name,
    })),
    url: song.dirPath,
    name: song.common.title,
    localName: song.common.localTitle,
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

export function addToList(listType, tracks) {
  listInfo.value = { id: 'local', type: listType }
  songList.value = [...tracks]
  savePlaylist()
}

export function addLocalMusicTOList(listType, localTracks, playId, playIndex) {
  addToList(listType, localMusicHandle(localTracks))
  addSong(playId, playIndex, true)
  savePlaylist()
}

export function setShuffledList(playAll = false) {
  const shuffled = [...(songList.value || [])]
  for (let index = shuffled.length - 1; index > 0; index -= 1) {
    const target = Math.floor(Math.random() * (index + 1))
    ;[shuffled[index], shuffled[target]] = [shuffled[target], shuffled[index]]
  }
  if (!playAll && songId.value) {
    const current = shuffled.findIndex((track) => track.id === songId.value)
    if (current >= 0) shuffled.unshift(...shuffled.splice(current, 1))
  }
  shuffledList.value = shuffled
  shuffleIndex.value = 0
}

function activeList() {
  return playMode.value === 3 ? shuffledList.value : songList.value
}

function activeIndex() {
  return playMode.value === 3 ? shuffleIndex.value : currentIndex.value
}

function playAt(index) {
  const list = activeList() || []
  if (!list.length) return
  const normalizedIndex = (index + list.length) % list.length
  const track = list[normalizedIndex]
  if (playMode.value === 3) {
    shuffleIndex.value = normalizedIndex
    currentIndex.value = songList.value.findIndex((song) => song.id === track.id)
  } else {
    currentIndex.value = normalizedIndex
  }
  songId.value = track.id
  addSong(track.id, currentIndex.value, true)
}

export function playLast() {
  playAt(activeIndex() - 1)
}

export function playNext() {
  playAt(activeIndex() + 1)
}

registerNextTrackHandler(playNext)

export function changePlayMode() {
  playMode.value = (playMode.value + 1) % 4
  currentMusic.value?.loop(playMode.value === 2)
  if (playMode.value === 3) setShuffledList()
  else {
    shuffledList.value = null
    shuffleIndex.value = 0
  }
  windowApi.changeTrayMusicPlaymode(playMode.value)
}

export function applyPlayMode(mode) {
  playMode.value = mode
  currentMusic.value?.loop(mode === 2)
  if (mode === 3) setShuffledList()
  else {
    shuffledList.value = null
    shuffleIndex.value = 0
  }
}

export function playAll(listType, tracks) {
  addToList(listType, tracks)
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
  if (nextSong.id === songId.value) return

  const existingIndex = songList.value.findIndex((song) => song.id === nextSong.id)
  if (existingIndex >= 0) {
    songList.value.splice(existingIndex, 1)
    if (existingIndex < currentIndex.value) currentIndex.value -= 1
  }
  songList.value.splice(currentIndex.value + 1, 0, nextSong)

  if (playMode.value === 3) {
    const shuffledIndex = (shuffledList.value || []).findIndex((song) => song.id === nextSong.id)
    if (shuffledIndex >= 0) shuffledList.value.splice(shuffledIndex, 1)
    shuffledList.value.splice(shuffleIndex.value + 1, 0, nextSong)
  }
  if (autoplay) playNext()
  else noticeOpen('已添加至下一首', 2)
  if (songList.value.length === 1) addSong(nextSong.id, 0, autoplay)
  savePlaylist()
}

export function addToNextLocal(song, autoplay) {
  addToNext(localMusicHandle([song], true), autoplay)
}

function compactPlaylist() {
  return {
    version: 2,
    songIds: (songList.value || []).map((track) => track.id),
    shuffledSongIds: (shuffledList.value || []).map((track) => track.id),
    currentSongId: songId.value,
    currentIndex: currentIndex.value,
    shuffleIndex: shuffleIndex.value,
  }
}

export function savePlaylist() {
  return windowApi.saveLastPlaylist(JSON.stringify(compactPlaylist()))
}

export async function loadLastSong() {
  pendingPlaylist = await windowApi.getLastPlaylist()
  restorePlaylistFromLibrary(localStore.localMusicList)
}

export function restorePlaylistFromLibrary(library) {
  if (!pendingPlaylist || !library) return
  const saved = pendingPlaylist
  const tracks = localMusicHandle(flattenTracks(library))
  const byId = new Map(tracks.map((track) => [track.id, track]))
  const songIds = saved.songIds || saved.songList?.map((track) => track.id) || []
  const shuffledSongIds = saved.shuffledSongIds
    || saved.shuffledList?.map((track) => track.id)
    || []
  const restored = songIds.map((id) => byId.get(id)).filter(Boolean)
  pendingPlaylist = null
  if (!restored.length) return

  songList.value = restored
  shuffledList.value = shuffledSongIds.map((id) => byId.get(id)).filter(Boolean)
  const selectedId = saved.currentSongId || songId.value
  const savedIndex = restored.findIndex((track) => track.id === selectedId)
  currentIndex.value = savedIndex >= 0 ? savedIndex : 0
  songId.value = restored[currentIndex.value].id
  const resumeAt = progress.value
  getSongUrl(currentIndex.value, false).then(() => {
    if (resumeAt > 0) {
      if (currentMusic.value?.state() === 'loaded') currentMusic.value.seek(resumeAt)
      else currentMusic.value?.once('load', () => currentMusic.value?.seek(resumeAt))
    }
  })
}
