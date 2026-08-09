import assert from 'node:assert/strict'
import test from 'node:test'

import {
  FAVORITES_ID,
  addTrack,
  createPlaylist,
  deletePlaylist,
  emptyCollections,
  hasTrack,
  normalizeCollections,
  removeTrack,
  renamePlaylist,
  toggleFavorite,
  trackIdsFor,
  validatePlaylistName,
} from '../src/utils/collectionModel.mjs'

test('favorite membership toggles without duplicating a track', () => {
  const empty = emptyCollections()
  const selected = toggleFavorite(empty, 'track:one')
  const selectedAgain = addTrack(selected, FAVORITES_ID, 'track:one')

  assert.deepEqual(selectedAgain.favoriteTrackIds, ['track:one'])
  assert.equal(hasTrack(selectedAgain, FAVORITES_ID, 'track:one'), true)

  const cleared = toggleFavorite(selectedAgain, 'track:one')
  assert.deepEqual(cleared.favoriteTrackIds, [])
})

test('user playlists receive stable monotonic ids and keep requested song order', () => {
  const first = createPlaylist(emptyCollections(), ' 通勤 ').collections
  const secondResult = createPlaylist(first, '安静')
  let collections = secondResult.collections

  collections = addTrack(collections, 'playlist:2', 'track:b')
  collections = addTrack(collections, 'playlist:2', 'track:a')
  collections = addTrack(collections, 'playlist:2', 'track:b')

  assert.deepEqual(collections.playlists.map(({ id, name }) => ({ id, name })), [
    { id: 'playlist:1', name: '通勤' },
    { id: 'playlist:2', name: '安静' },
  ])
  assert.deepEqual(trackIdsFor(collections, 'playlist:2'), ['track:b', 'track:a'])
  assert.equal(collections.nextPlaylistId, 3)
})

test('rename, removal and deletion preserve the other playlists', () => {
  let collections = createPlaylist(emptyCollections(), 'A').collections
  collections = createPlaylist(collections, 'B').collections
  collections = addTrack(collections, 'playlist:1', 'track:a')
  collections = addTrack(collections, 'playlist:2', 'track:b')
  collections = renamePlaylist(collections, 'playlist:2', 'B2')
  collections = removeTrack(collections, 'playlist:1', 'track:a')
  collections = deletePlaylist(collections, 'playlist:1')

  assert.deepEqual(collections.playlists, [
    { id: 'playlist:2', name: 'B2', trackIds: ['track:b'] },
  ])
  assert.throws(() => deletePlaylist(collections, FAVORITES_ID), /不能删除/)
})

test('loading collections repairs duplicate ids and advances the next id', () => {
  const collections = normalizeCollections({
    nextPlaylistId: 1,
    favoriteTrackIds: ['track:a', 'track:a'],
    playlists: [
      { id: 'playlist:4', name: '一', trackIds: [] },
      { id: 'playlist:4', name: '重复', trackIds: [] },
      { id: 'bad', name: '无效', trackIds: [] },
    ],
  })

  assert.deepEqual(collections.favoriteTrackIds, ['track:a'])
  assert.deepEqual(collections.playlists.map((item) => item.id), ['playlist:4'])
  assert.equal(collections.nextPlaylistId, 5)
})

test('playlist names follow the actual naming contract', () => {
  assert.equal(validatePlaylistName('  夜间  '), '夜间')
  assert.throws(() => validatePlaylistName('   '), /不能为空/)
  assert.throws(() => validatePlaylistName('歌'.repeat(81)), /80/)
})
