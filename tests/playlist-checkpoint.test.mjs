import assert from 'node:assert/strict'
import test from 'node:test'

import { buildPlaylistCheckpoint } from '../src/utils/player/playlistCheckpoint.mjs'

function state(trackCount = 3) {
  const songIds = Array.from({ length: trackCount }, (_, index) => `track:${index}`)
  return {
    structureRevision: 7,
    songIds,
    shuffledSongIds: [...songIds].reverse(),
    currentSongId: songIds[1],
    currentIndex: 1,
    shuffleIndex: 1,
    progress: 43.25,
    volume: 0.4,
    playMode: 3,
  }
}

test('structural playlist checkpoints carry ordered and shuffled ids', () => {
  const checkpoint = buildPlaylistCheckpoint(state(), true)
  const payload = JSON.parse(checkpoint.payload)

  assert.equal(payload.version, 4)
  assert.equal(payload.structureRevision, 7)
  assert.deepEqual(payload.songIds, ['track:0', 'track:1', 'track:2'])
  assert.deepEqual(payload.shuffledSongIds, ['track:2', 'track:1', 'track:0'])
  assert.equal(checkpoint.structureRevision, 7)
})

test('scalar checkpoints omit queue arrays without changing the state signature', () => {
  const full = buildPlaylistCheckpoint(state(), true)
  const scalar = buildPlaylistCheckpoint(state(), false)
  const payload = JSON.parse(scalar.payload)

  assert.equal(payload.songIds, undefined)
  assert.equal(payload.shuffledSongIds, undefined)
  assert.equal(scalar.structureRevision, null)
  assert.equal(scalar.signature, full.signature)
})

test('a 10,000-track progress checkpoint stays below 256 bytes', () => {
  const checkpoint = buildPlaylistCheckpoint(state(10_000), false)

  assert.ok(Buffer.byteLength(checkpoint.payload) < 256)
})
