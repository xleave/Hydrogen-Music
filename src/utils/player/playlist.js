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
  volume,
} = playerRefs

let pendingPlaylist = null

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

function activeList() { return playMode.value === 3 ? shuffledList.value : songList.value }
function activeIndex() { return playMode.value === 3 ? shuffleIndex.value : currentIndex.value }

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
  if (autoplay) playNext()
  else noticeOpen('已添加至下一首', 2)
  if (songList.value.length === 1) addSong(nextSong.id, 0, autoplay)
  savePlaylist()
}

export function addToNextLocal(song, autoplay) { addToNext(localMusicHandle([song], true), autoplay) }

function compactPlaylist() {
  return {
    version: 3,
    songIds: (songList.value || []).map((track) => track.id),
    shuffledSongIds: (shuffledList.value || []).map((track) => track.id),
    currentSongId: songId.value,
    currentIndex: currentIndex.value,
    shuffleIndex: shuffleIndex.value,
    progress: Number.isFinite(progress.value) ? Math.max(0, progress.value) : 0,
    volume: Number.isFinite(volume.value) ? Math.max(0, Math.min(1, volume.value)) : 0.3,
    playMode: Number.isInteger(playMode.value) ? Math.max(0, Math.min(3, playMode.value)) : 0,
  }
}

export function savePlaylist() {
  // null 表示音乐库/播放队列尚未恢复完成，不能用空快照覆盖上次有效状态。
  if (songList.value === null) return Promise.resolve()
  return windowApi.saveLastPlaylist(JSON.stringify(compactPlaylist()))
}

export async function loadLastSong() {
  pendingPlaylist = await windowApi.getLastPlaylist()
  if (pendingPlaylist) {
    if (Number.isFinite(pendingPlaylist.volume)) volume.value = Math.max(0, Math.min(1, pendingPlaylist.volume))
    if (Number.isInteger(pendingPlaylist.playMode)) playMode.value = Math.max(0, Math.min(3, pendingPlaylist.playMode))
  }
  restorePlaylistFromLibrary(localStore.localMusicList)
}

export function restorePlaylistFromLibrary(library) {
  if (!pendingPlaylist || !library) return
  const saved = pendingPlaylist
  const tracks = localMusicHandle(flattenTracks(library))
  const byId = new Map(tracks.map((track) => [track.id, track]))
  const songIds = saved.songIds || saved.songList?.map((track) => track.id) || []
  const shuffledSongIds = saved.shuffledSongIds || saved.shuffledList?.map((track) => track.id) || []
  const restored = songIds.map((id) => byId.get(id)).filter(Boolean)
  pendingPlaylist = null
  if (!restored.length) return

  songList.value = restored
  shuffledList.value = shuffledSongIds.map((id) => byId.get(id)).filter(Boolean)
  const fallbackIndex = Math.max(0, Math.min(Number(saved.currentIndex) || 0, restored.length - 1))
  const selectedId = saved.currentSongId || restored[fallbackIndex].id
  const savedIndex = restored.findIndex((track) => track.id === selectedId)
  currentIndex.value = savedIndex >= 0 ? savedIndex : fallbackIndex
  songId.value = restored[currentIndex.value].id
  shuffleIndex.value = Math.max(0, Math.min(Number(saved.shuffleIndex) || 0, Math.max(0, shuffledList.value.length - 1)))
  if (playMode.value === 3 && !shuffledList.value.length) setShuffledList()

  const resumeAt = Number.isFinite(saved.progress) ? Math.max(0, saved.progress) : 0
  progress.value = resumeAt
  getSongUrl(currentIndex.value, false).then(() => {
    if (resumeAt > 0) {
      if (currentMusic.value?.state() === 'loaded') currentMusic.value.seek(resumeAt)
      else currentMusic.value?.once('load', () => currentMusic.value?.seek(resumeAt))
    }
    windowApi.audioSetVolume(volume.value).catch((error) => console.error('[restore volume]', error))
    windowApi.setSystemMediaVolume(volume.value).catch((error) => console.error('[restore media volume]', error))
  })
}
