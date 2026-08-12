import assert from 'node:assert/strict'
import test from 'node:test'

import {
  queueSelection,
  shuffledTracks,
  trackEndAction,
  wrappedIndex,
} from '../src/utils/player/playbackOrder.mjs'

const songs = [{ id: 'a' }, { id: 'b' }, { id: 'c' }]

test('previous and next navigation wrap around the active queue', () => {
  assert.equal(wrappedIndex(-1, 3), 2)
  assert.equal(wrappedIndex(3, 3), 0)
  assert.equal(wrappedIndex(0, 0), -1)
})

test('shuffle selection keeps source and shuffled indices distinct', () => {
  const shuffled = [songs[2], songs[0], songs[1]]
  const selection = queueSelection(3, 0, songs, shuffled)

  assert.equal(selection.track.id, 'c')
  assert.equal(selection.currentIndex, 2)
  assert.equal(selection.shuffleIndex, 0)
  assert.equal(queueSelection(1, 3, songs, shuffled).track.id, 'a')
})

test('enabling shuffle keeps the current track first unless play-all starts a new queue', () => {
  const deterministicRandom = () => 0

  assert.deepEqual(
    shuffledTracks(songs, 'a', false, deterministicRandom).map((track) => track.id),
    ['a', 'b', 'c'],
  )
  assert.deepEqual(
    shuffledTracks(songs, 'a', true, deterministicRandom).map((track) => track.id),
    ['b', 'c', 'a'],
  )
})

test('track-end behavior preserves sequential, list-loop, single-loop and shuffle modes', () => {
  assert.equal(trackEndAction(0, 2, 3), 'stop')
  assert.equal(trackEndAction(0, 1, 3), 'next')
  assert.equal(trackEndAction(1, 2, 3), 'next')
  assert.equal(trackEndAction(2, 1, 3), 'repeat')
  assert.equal(trackEndAction(3, 2, 3), 'next')
})
