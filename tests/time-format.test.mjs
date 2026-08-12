import assert from 'node:assert/strict'
import test from 'node:test'

import { songTime, songTime2 } from '../src/utils/player/time.js'

test('metadata millisecond formatting preserves the existing minute component contract', () => {
  assert.equal(songTime(0), 0)
  assert.equal(songTime('--'), '--')
  assert.equal(songTime(null), undefined)
  assert.equal(songTime(65_900), '1:05')
  assert.equal(songTime(3_665_900), '1:05')
})

test('player second formatting remains zero-padded', () => {
  assert.equal(songTime2(0), '00:00')
  assert.equal(songTime2(65.9), '01:05')
  assert.equal(songTime2(3_665.9), '61:05')
})
