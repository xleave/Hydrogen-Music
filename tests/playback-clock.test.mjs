import assert from 'node:assert/strict'
import test from 'node:test'

import { PlaybackClock } from '../src/utils/player/playbackClock.mjs'

test('playing position advances locally between native reconciliations', () => {
  const clock = new PlaybackClock({ position: 12, duration: 240, playing: true, ended: false }, 1_000)

  assert.equal(clock.estimate(1_200), 12.2)
  assert.equal(clock.estimate(2_000), 13)
  assert.equal(clock.shouldReconcile(1_999, 1_000), false)
  assert.equal(clock.shouldReconcile(2_000, 1_000), true)
})

test('paused and ended clocks stay fixed', () => {
  const paused = new PlaybackClock({ position: 23, duration: 240, playing: false, ended: false }, 500)
  const ended = new PlaybackClock({ position: 240, duration: 240, playing: false, ended: true }, 500)

  assert.equal(paused.estimate(50_000), 23)
  assert.equal(ended.estimate(50_000), 240)
})

test('end confirmation begins within one visual progress tick', () => {
  const clock = new PlaybackClock({ position: 99, duration: 100, playing: true, ended: false }, 0)

  assert.equal(clock.needsEndConfirmation(799, 0.2), false)
  assert.equal(clock.needsEndConfirmation(800, 0.2), true)
  assert.equal(clock.estimate(2_000), 100)
})
